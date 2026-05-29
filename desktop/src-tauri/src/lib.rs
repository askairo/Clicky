use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Deserialize, Serialize, Clone)]
struct ConfigFile {
    #[serde(default)]
    groups: HashMap<String, GroupDef>,
    #[serde(default)]
    environments: HashMap<String, EnvDef>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct GroupDef {
    description: Option<String>,
    #[serde(default)]
    environments: HashMap<String, EnvDef>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct EnvDef {
    description: Option<String>,
    #[serde(default)]
    variables: HashMap<String, String>,
    hooks: Option<HooksDef>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct HooksDef {
    post: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
struct GroupSummary {
    name: String,
    description: Option<String>,
    env_count: usize,
}

#[derive(Debug, Serialize)]
struct EnvSummary {
    group: String,
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
    group: String,
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

fn normalize_config(mut cfg: ConfigFile) -> ConfigFile {
    if !cfg.environments.is_empty() {
        let mut default_group = cfg.groups.remove("default").unwrap_or(GroupDef {
            description: Some("Default group".to_string()),
            environments: HashMap::new(),
        });
        for (name, env) in cfg.environments.drain() {
            default_group.environments.insert(name, env);
        }
        cfg.groups.insert("default".to_string(), default_group);
    }
    cfg
}

fn load_config() -> Result<ConfigFile, String> {
    let path = config_path()?;
    let content = fs::read_to_string(&path)
        .map_err(|e| format!("failed to read {}: {}", path.display(), e))?;
    let cfg = serde_yaml::from_str::<ConfigFile>(&content)
        .map_err(|e| format!("failed to parse {}: {}", path.display(), e))?;
    Ok(normalize_config(cfg))
}

fn save_config(cfg: &ConfigFile) -> Result<(), String> {
    let path = config_path()?;
    let content = serde_yaml::to_string(cfg)
        .map_err(|e| format!("failed to serialize config: {}", e))?;
    fs::write(&path, content).map_err(|e| format!("failed to write {}: {}", path.display(), e))
}

fn validate_group_variable_uniqueness(cfg: &ConfigFile) -> Result<(), String> {
    let mut key_to_group: HashMap<String, String> = HashMap::new();

    for (group_name, group) in &cfg.groups {
        let mut group_keys: HashSet<String> = HashSet::new();
        for env in group.environments.values() {
            for key in env.variables.keys() {
                group_keys.insert(key.clone());
            }
        }

        for key in group_keys {
            if let Some(existing_group) = key_to_group.get(&key) {
                if existing_group != group_name {
                    return Err(format!(
                        "variable '{}' is already used in group '{}', cannot reuse in group '{}'",
                        key, existing_group, group_name
                    ));
                }
            } else {
                key_to_group.insert(key, group_name.clone());
            }
        }
    }

    Ok(())
}

#[tauri::command]
fn list_groups() -> Result<Vec<GroupSummary>, String> {
    let cfg = load_config()?;
    let mut list = cfg
        .groups
        .into_iter()
        .map(|(name, group)| GroupSummary {
            env_count: group.environments.len(),
            name,
            description: group.description,
        })
        .collect::<Vec<_>>();
    list.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(list)
}

#[tauri::command]
fn create_group(group_name: String) -> Result<(), String> {
    let mut cfg = load_config()?;
    let name = group_name.trim();
    if name.is_empty() {
        return Err("group name is required".to_string());
    }
    if cfg.groups.contains_key(name) {
        return Err(format!("group '{}' already exists", name));
    }

    cfg.groups.insert(
        name.to_string(),
        GroupDef {
            description: Some(format!("{} group", name)),
            environments: HashMap::new(),
        },
    );

    save_config(&cfg)
}

#[tauri::command]
fn list_environments(group_name: String) -> Result<Vec<EnvSummary>, String> {
    let cfg = load_config()?;
    let group = cfg
        .groups
        .get(&group_name)
        .ok_or_else(|| format!("group '{}' not found", group_name))?;

    let mut list = group
        .environments
        .iter()
        .map(|(name, env)| EnvSummary {
            group: group_name.clone(),
            name: name.clone(),
            description: env.description.clone(),
            var_count: env.variables.len(),
        })
        .collect::<Vec<_>>();
    list.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(list)
}

#[tauri::command]
fn get_environment_variables(group_name: String, env_name: String) -> Result<HashMap<String, String>, String> {
    let cfg = load_config()?;
    let group = cfg
        .groups
        .get(&group_name)
        .ok_or_else(|| format!("group '{}' not found", group_name))?;
    group
        .environments
        .get(&env_name)
        .map(|e| e.variables.clone())
        .ok_or_else(|| format!("environment '{}' not found in group '{}'", env_name, group_name))
}

#[tauri::command]
fn detect_active_environments(group_name: String) -> Result<Vec<String>, String> {
    let cfg = load_config()?;
    let group = cfg
        .groups
        .get(&group_name)
        .ok_or_else(|| format!("group '{}' not found", group_name))?;

    let mut matches = Vec::new();
    for (env_name, env_def) in &group.environments {
        let all_match = env_def
            .variables
            .iter()
            .all(|(k, v)| std::env::var(k).map(|current| current == *v).unwrap_or(false));
        if all_match {
            matches.push(env_name.clone());
        }
    }

    matches.sort();
    Ok(matches)
}

#[tauri::command]
fn save_environment_variables(
    group_name: String,
    env_name: String,
    variables: HashMap<String, String>,
) -> Result<(), String> {
    let mut cfg = load_config()?;

    let existing_group = cfg
        .groups
        .entry(group_name.clone())
        .or_insert(GroupDef {
            description: Some(format!("{} group", group_name)),
            environments: HashMap::new(),
        });

    let existing_env = existing_group.environments.get(&env_name).cloned();
    let updated_env = match existing_env {
        Some(mut env) => {
            env.variables = variables;
            env
        }
        None => EnvDef {
            description: Some(format!("{} environment", env_name)),
            variables,
            hooks: Some(HooksDef { post: None }),
        },
    };

    existing_group.environments.insert(env_name, updated_env);
    validate_group_variable_uniqueness(&cfg)?;
    save_config(&cfg)
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
fn apply_environment(group_name: String, env_name: String, mode: String) -> Result<ApplyResult, String> {
    let cfg = load_config()?;
    let group = cfg
        .groups
        .get(&group_name)
        .ok_or_else(|| format!("group '{}' not found", group_name))?;
    let env = group
        .environments
        .get(&env_name)
        .ok_or_else(|| format!("environment '{}' not found in group '{}'", env_name, group_name))?;

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
        group: group_name,
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
            list_groups,
            create_group,
            list_environments,
            get_environment_variables,
            detect_active_environments,
            save_environment_variables,
            apply_environment
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
