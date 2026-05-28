use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Deserialize)]
struct ConfigFile {
    environments: HashMap<String, EnvDef>,
}

#[derive(Debug, Deserialize)]
struct EnvDef {
    description: Option<String>,
    variables: HashMap<String, String>,
    hooks: Option<HooksDef>,
}

#[derive(Debug, Deserialize)]
struct HooksDef {
    post: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
struct EnvSummary {
    name: String,
    description: Option<String>,
    var_count: usize,
}

#[derive(Debug, Serialize)]
struct VariableApplyResult {
    key: String,
    before: Option<String>,
    after: String,
    applied: bool,
    message: String,
}

#[derive(Debug, Serialize)]
struct HookResult {
    command: String,
    success: bool,
    code: Option<i32>,
    message: String,
}

#[derive(Debug, Serialize)]
struct ApplyResult {
    environment: String,
    mode: String,
    variable_results: Vec<VariableApplyResult>,
    hook_results: Vec<HookResult>,
}

fn config_path() -> Result<PathBuf, String> {
    let cwd = std::env::current_dir().map_err(|e| e.to_string())?;
    let candidates = [
        cwd.join("config").join("environments.yaml"),
        cwd.parent()
            .map(|p| p.join("config").join("environments.yaml"))
            .unwrap_or_else(|| cwd.join("__missing__")),
    ];

    for path in candidates {
        if path.exists() {
            return Ok(path);
        }
    }

    Err(format!(
        "failed to locate environments.yaml, checked: {} and {}",
        cwd.join("config").join("environments.yaml").display(),
        cwd.parent()
            .map(|p| p.join("config").join("environments.yaml").display().to_string())
            .unwrap_or_else(|| "<no-parent>".to_string())
    ))
}

fn load_config() -> Result<ConfigFile, String> {
    let path = config_path()?;
    let content = fs::read_to_string(&path)
        .map_err(|e| format!("failed to read {}: {}", path.display(), e))?;
    serde_yaml::from_str::<ConfigFile>(&content)
        .map_err(|e| format!("failed to parse {}: {}", path.display(), e))
}

#[tauri::command]
fn list_environments() -> Result<Vec<EnvSummary>, String> {
    let cfg = load_config()?;
    let mut list = cfg
        .environments
        .into_iter()
        .map(|(name, env)| EnvSummary {
            name,
            description: env.description,
            var_count: env.variables.len(),
        })
        .collect::<Vec<_>>();
    list.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(list)
}

#[tauri::command]
fn get_environment_variables(env_name: String) -> Result<HashMap<String, String>, String> {
    let cfg = load_config()?;
    cfg.environments
        .get(&env_name)
        .map(|e| e.variables.clone())
        .ok_or_else(|| format!("environment '{}' not found", env_name))
}

#[tauri::command]
fn detect_active_environments() -> Result<Vec<String>, String> {
    let cfg = load_config()?;
    let mut matches = Vec::new();

    for (env_name, env_def) in cfg.environments {
        let all_match = env_def.variables.iter().all(|(k, v)| {
            std::env::var(k)
                .map(|current| current == *v)
                .unwrap_or(false)
        });
        if all_match {
            matches.push(env_name);
        }
    }

    matches.sort();
    Ok(matches)
}

#[cfg(target_os = "windows")]
fn apply_var_persistent_windows(key: &str, value: &str) -> Result<(), String> {
    let status = Command::new("setx")
        .arg(key)
        .arg(value)
        .status()
        .map_err(|e| format!("setx {} failed: {}", key, e))?;
    if !status.success() {
        return Err(format!("setx {} failed with status {}", key, status));
    }
    std::env::set_var(key, value);
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn apply_var_persistent_windows(_key: &str, _value: &str) -> Result<(), String> {
    Err("persistent apply is not implemented for this OS yet".to_string())
}

fn run_post_hooks(hooks: Option<&HooksDef>) -> Vec<HookResult> {
    let mut results = Vec::new();
    let Some(post_hooks) = hooks.and_then(|h| h.post.as_ref()) else {
        return results;
    };

    for cmd in post_hooks {
        #[cfg(target_os = "windows")]
        let output = Command::new("cmd").args(["/C", cmd]).output();

        #[cfg(not(target_os = "windows"))]
        let output = Command::new("sh").args(["-c", cmd]).output();

        match output {
            Ok(out) => {
                let success = out.status.success();
                let mut msg = String::new();
                if !out.stdout.is_empty() {
                    msg.push_str(&String::from_utf8_lossy(&out.stdout));
                }
                if !out.stderr.is_empty() {
                    if !msg.is_empty() {
                        msg.push_str("\n");
                    }
                    msg.push_str(&String::from_utf8_lossy(&out.stderr));
                }
                results.push(HookResult {
                    command: cmd.clone(),
                    success,
                    code: out.status.code(),
                    message: msg.trim().to_string(),
                });
            }
            Err(e) => results.push(HookResult {
                command: cmd.clone(),
                success: false,
                code: None,
                message: e.to_string(),
            }),
        }
    }
    results
}

#[tauri::command]
fn apply_environment(env_name: String, mode: String) -> Result<ApplyResult, String> {
    let cfg = load_config()?;
    let env = cfg
        .environments
        .get(&env_name)
        .ok_or_else(|| format!("environment '{}' not found", env_name))?;

    if mode != "persistent" {
        return Err("envflow only supports persistent mode; reopen target processes after applying".to_string());
    }

    let mut variable_results = Vec::new();

    let mut entries = env.variables.iter().collect::<Vec<_>>();
    entries.sort_by(|a, b| a.0.cmp(b.0));

    for (k, v) in entries {
        let before = std::env::var(k).ok();
        let result = match apply_var_persistent_windows(k, v) {
            Ok(_) => VariableApplyResult {
                key: k.clone(),
                before,
                after: v.clone(),
                applied: true,
                message: "persisted for new processes".to_string(),
            },
            Err(e) => VariableApplyResult {
                key: k.clone(),
                before,
                after: v.clone(),
                applied: false,
                message: e,
            },
        };
        variable_results.push(result);
    }

    let hook_results = run_post_hooks(env.hooks.as_ref());

    Ok(ApplyResult {
        environment: env_name,
        mode,
        variable_results,
        hook_results,
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            list_environments,
            get_environment_variables,
            detect_active_environments,
            apply_environment
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
