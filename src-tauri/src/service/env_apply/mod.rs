use crate::domain::RuntimeCapabilities;
use crate::service::storage_service;
use std::path::PathBuf;

#[cfg(target_os = "windows")]
mod platform {
    pub use super::windows_impl::WindowsEnvApplier as CurrentEnvApplier;
}
#[cfg(target_os = "windows")]
mod windows_impl;

#[cfg(target_os = "macos")]
mod platform {
    pub use super::macos_impl::MacosEnvApplier as CurrentEnvApplier;
}
#[cfg(target_os = "macos")]
mod macos_impl;

#[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
mod platform {
    pub use super::other_impl::OtherEnvApplier as CurrentEnvApplier;
}
#[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
mod other_impl;

pub trait EnvApplier {
    fn apply_var_persistent(&self, key: &str, value: &str) -> Result<(), String>;
    fn persist_shell_env_snapshot(&self, items: &[(String, String)]) -> Result<Option<String>, String>;
    fn runtime_capabilities(&self) -> RuntimeCapabilities;
    fn read_persistent_var(&self, key: &str) -> Result<Option<String>, String>;
}

pub fn apply_var_persistent(key: &str, value: &str) -> Result<(), String> {
    let applier = platform::CurrentEnvApplier;
    applier.apply_var_persistent(key, value)
}

pub fn persist_shell_env_snapshot(items: &[(String, String)]) -> Result<Option<String>, String> {
    let applier = platform::CurrentEnvApplier;
    applier.persist_shell_env_snapshot(items)
}

pub fn runtime_capabilities() -> RuntimeCapabilities {
    let applier = platform::CurrentEnvApplier;
    applier.runtime_capabilities()
}

pub fn read_persistent_var(key: &str) -> Result<Option<String>, String> {
    let applier = platform::CurrentEnvApplier;
    applier.read_persistent_var(key)
}

pub fn shell_integration_file_path() -> Result<PathBuf, String> {
    let db_path = storage_service::db_path()?;
    let base = db_path
        .parent()
        .ok_or_else(|| "failed to resolve clicky data directory".to_string())?;
    Ok(base.join("env.sh"))
}
