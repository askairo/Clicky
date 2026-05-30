use crate::domain::{HookResult, HooksDef};
use std::process::Command;

#[cfg(target_os = "windows")]
use winreg::enums::HKEY_CURRENT_USER;
#[cfg(target_os = "windows")]
use winreg::RegKey;

#[cfg(target_os = "windows")]
pub fn apply_var_persistent_windows(key: &str, value: &str) -> Result<(), String> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (env_key, _disp) = hkcu
        .create_subkey("Environment")
        .map_err(|e| format!("open HKCU\\Environment failed for '{}': {}", key, e))?;
    env_key
        .set_value(key, &value)
        .map_err(|e| format!("write HKCU\\Environment\\{} failed: {}", key, e))?;
    std::env::set_var(key, value);
    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub fn apply_var_persistent_windows(_key: &str, _value: &str) -> Result<(), String> {
    Err("persistent apply is not implemented for this OS yet".to_string())
}

pub fn run_post_hooks(hooks: Option<&HooksDef>) -> Vec<HookResult> {
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
