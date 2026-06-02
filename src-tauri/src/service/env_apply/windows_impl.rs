use crate::domain::RuntimeCapabilities;
use crate::service::env_apply::EnvApplier;

use windows_sys::Win32::UI::WindowsAndMessaging::{
    SendMessageTimeoutW, HWND_BROADCAST, SMTO_ABORTIFHUNG, WM_SETTINGCHANGE,
};
use winreg::enums::HKEY_CURRENT_USER;
use winreg::RegKey;

/// Windows adapter: writes to HKCU\\Environment and broadcasts the change.
#[derive(Copy, Clone)]
pub struct WindowsEnvApplier;

impl EnvApplier for WindowsEnvApplier {
    fn apply_var_persistent(&self, key: &str, value: &str) -> Result<(), String> {
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

    fn persist_shell_env_snapshot(
        &self,
        _items: &[(String, String)],
    ) -> Result<Option<String>, String> {
        Ok(None)
    }

    fn runtime_capabilities(&self) -> RuntimeCapabilities {
        RuntimeCapabilities {
            platform: "windows".to_string(),
            apply_scope_hint: "新开进程生效，当前进程通常需重启。".to_string(),
            shell_integration_file: None,
        }
    }

    fn read_persistent_var(&self, key: &str) -> Result<Option<String>, String> {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let env_key = hkcu
            .open_subkey("Environment")
            .map_err(|e| format!("open HKCU\\Environment failed for '{}': {}", key, e))?;
        match env_key.get_value::<String, _>(key) {
            Ok(value) => Ok(Some(value)),
            Err(_) => Ok(None),
        }
    }

    fn notify_environment_change(&self) -> Result<(), String> {
        broadcast_environment_change()
    }
}

fn broadcast_environment_change() -> Result<(), String> {
    let environment = "Environment"
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect::<Vec<u16>>();
    let mut result = 0usize;
    let sent = unsafe {
        SendMessageTimeoutW(
            HWND_BROADCAST,
            WM_SETTINGCHANGE,
            0,
            environment.as_ptr() as isize,
            SMTO_ABORTIFHUNG,
            5000,
            &mut result,
        )
    };

    if sent == 0 {
        return Err(
            "persisted environment variable, but failed to notify Windows environment change"
                .to_string(),
        );
    }

    Ok(())
}
