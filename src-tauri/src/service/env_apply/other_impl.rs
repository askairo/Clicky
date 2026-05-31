use crate::service::env_apply::EnvApplier;
use crate::domain::RuntimeCapabilities;

#[derive(Copy, Clone)]
pub struct OtherEnvApplier;

impl EnvApplier for OtherEnvApplier {
    fn apply_var_persistent(&self, _key: &str, _value: &str) -> Result<(), String> {
        Err("persistent apply is not implemented for this OS yet".to_string())
    }

    fn persist_shell_env_snapshot(&self, _items: &[(String, String)]) -> Result<Option<String>, String> {
        Ok(None)
    }

    fn runtime_capabilities(&self) -> RuntimeCapabilities {
        RuntimeCapabilities {
            platform: std::env::consts::OS.to_string(),
            apply_scope_hint: "当前平台仅支持配置管理，环境变量持久化应用尚未实现。".to_string(),
            shell_integration_file: None,
        }
    }

    fn read_persistent_var(&self, key: &str) -> Result<Option<String>, String> {
        Ok(std::env::var(key).ok())
    }
}
