use crate::domain::RuntimeCapabilities;
use crate::domain::{HookResult, HooksDef};
use crate::service::env_apply;
use std::process::Command;

/// Thin wrapper around platform-specific env handling and hook execution.
pub fn apply_var_persistent(key: &str, value: &str) -> Result<(), String> {
    env_apply::apply_var_persistent(key, value)
}

/// Persists a shell snapshot when the current platform supports it.
pub fn persist_shell_env_snapshot(items: &[(String, String)]) -> Result<Option<String>, String> {
    env_apply::persist_shell_env_snapshot(items)
}

/// Returns the platform capability summary shown in the frontend.
pub fn runtime_capabilities() -> RuntimeCapabilities {
    env_apply::runtime_capabilities()
}

/// Reads a single variable from the persistent scope.
pub fn read_persistent_var(key: &str) -> Result<Option<String>, String> {
    env_apply::read_persistent_var(key)
}

/// Runs post-apply hooks in a shell appropriate for the host OS.
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
