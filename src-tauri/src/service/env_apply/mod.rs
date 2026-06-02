use crate::domain::RuntimeCapabilities;
use crate::service::storage_service;
use std::path::PathBuf;

/// Platform adapter boundary for persistent environment-variable writes and reads.
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
    /// Writes one variable into the platform's persistent scope.
    fn apply_var_persistent(&self, key: &str, value: &str) -> Result<(), String>;
    /// Writes a shell snapshot when the platform needs an extra startup helper.
    fn persist_shell_env_snapshot(
        &self,
        items: &[(String, String)],
    ) -> Result<Option<String>, String>;
    /// Exposes the current platform's runtime hints to the frontend.
    fn runtime_capabilities(&self) -> RuntimeCapabilities;
    /// Reads back one variable from the persistent scope.
    fn read_persistent_var(&self, key: &str) -> Result<Option<String>, String>;
    /// Notifies the OS that environment values changed, if the platform needs it.
    fn notify_environment_change(&self) -> Result<(), String>;
}

/// Writes one variable through the current platform adapter.
pub fn apply_var_persistent(key: &str, value: &str) -> Result<(), String> {
    let applier = platform::CurrentEnvApplier;
    applier.apply_var_persistent(key, value)
}

/// Persists a shell snapshot when the current platform supports it.
pub fn persist_shell_env_snapshot(items: &[(String, String)]) -> Result<Option<String>, String> {
    let applier = platform::CurrentEnvApplier;
    applier.persist_shell_env_snapshot(items)
}

/// Returns the current platform capability summary.
pub fn runtime_capabilities() -> RuntimeCapabilities {
    let applier = platform::CurrentEnvApplier;
    applier.runtime_capabilities()
}

/// Reads one variable from the current platform adapter.
pub fn read_persistent_var(key: &str) -> Result<Option<String>, String> {
    let applier = platform::CurrentEnvApplier;
    applier.read_persistent_var(key)
}

/// Signals to the host OS that the environment has changed.
pub fn notify_environment_change() -> Result<(), String> {
    let applier = platform::CurrentEnvApplier;
    applier.notify_environment_change()
}

/// Returns the shell integration file path under the app data directory.
pub fn shell_integration_file_path() -> Result<PathBuf, String> {
    let db_path = storage_service::db_path()?;
    let base = db_path
        .parent()
        .ok_or_else(|| "failed to resolve clicky data directory".to_string())?;
    Ok(base.join("env.sh"))
}
