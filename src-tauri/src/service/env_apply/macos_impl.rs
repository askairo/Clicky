use crate::service::env_apply::EnvApplier;
use crate::domain::RuntimeCapabilities;
use crate::service::env_apply::shell_integration_file_path;
use std::fs;
use std::process::Command;

#[derive(Copy, Clone)]
pub struct MacosEnvApplier;

impl EnvApplier for MacosEnvApplier {
    fn apply_var_persistent(&self, key: &str, value: &str) -> Result<(), String> {
        let output = Command::new("launchctl")
            .args(["setenv", key, value])
            .output()
            .map_err(|e| format!("run launchctl setenv for '{}' failed: {}", key, e))?;

        if !output.status.success() {
            let mut message = String::new();
            if !output.stderr.is_empty() {
                message.push_str(&String::from_utf8_lossy(&output.stderr));
            }
            if message.is_empty() && !output.stdout.is_empty() {
                message.push_str(&String::from_utf8_lossy(&output.stdout));
            }
            let detail = if message.trim().is_empty() {
                format!("exit code {:?}", output.status.code())
            } else {
                message.trim().to_string()
            };
            return Err(format!("launchctl setenv '{}' failed: {}", key, detail));
        }

        std::env::set_var(key, value);
        Ok(())
    }

    fn persist_shell_env_snapshot(&self, items: &[(String, String)]) -> Result<Option<String>, String> {
        let file_path = shell_integration_file_path()?;
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("create shell integration directory {} failed: {}", parent.display(), e))?;
        }

        let mut sorted = items.to_vec();
        sorted.sort_by(|a, b| a.0.cmp(&b.0));

        let mut content = String::from("# Managed by Clicky. Do not edit manually.\n");
        for (key, value) in sorted {
            content.push_str("export ");
            content.push_str(&key);
            content.push('=');
            content.push_str(&shell_single_quote(&value));
            content.push('\n');
        }

        fs::write(&file_path, content)
            .map_err(|e| format!("write shell integration file {} failed: {}", file_path.display(), e))?;
        Ok(Some(file_path.display().to_string()))
    }

    fn runtime_capabilities(&self) -> RuntimeCapabilities {
        let shell_file = shell_integration_file_path()
            .ok()
            .map(|path| path.display().to_string());

        RuntimeCapabilities {
            platform: "macos".to_string(),
            apply_scope_hint: "新开进程生效，当前进程通常需重启。".to_string(),
            shell_integration_file: shell_file,
        }
    }

    fn read_persistent_var(&self, key: &str) -> Result<Option<String>, String> {
        let launchctl = read_from_launchctl(key)?;
        if launchctl.is_some() {
            return Ok(launchctl);
        }
        read_from_shell_snapshot(key)
    }
}

fn shell_single_quote(value: &str) -> String {
    let escaped = value.replace('\'', r#"'\''"#);
    format!("'{}'", escaped)
}

fn read_from_launchctl(key: &str) -> Result<Option<String>, String> {
    let output = Command::new("launchctl")
        .args(["getenv", key])
        .output()
        .map_err(|e| format!("run launchctl getenv for '{}' failed: {}", key, e))?;
    if !output.status.success() {
        return Ok(None);
    }
    let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if value.is_empty() {
        Ok(None)
    } else {
        Ok(Some(value))
    }
}

fn read_from_shell_snapshot(key: &str) -> Result<Option<String>, String> {
    let file_path = shell_integration_file_path()?;
    if !file_path.exists() {
        return Ok(None);
    }
    let content = fs::read_to_string(&file_path)
        .map_err(|e| format!("read shell integration file {} failed: {}", file_path.display(), e))?;
    for line in content.lines() {
        let Some(rest) = line.strip_prefix("export ") else {
            continue;
        };
        let Some((k, raw)) = rest.split_once('=') else {
            continue;
        };
        if k.trim() != key {
            continue;
        }
        return Ok(Some(shell_unquote(raw.trim())));
    }
    Ok(None)
}

fn shell_unquote(raw: &str) -> String {
    if raw.len() >= 2 && raw.starts_with('\'') && raw.ends_with('\'') {
        let inner = &raw[1..raw.len() - 1];
        return inner.replace(r#"'\''"#, "'");
    }
    raw.to_string()
}
