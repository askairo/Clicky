use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Canonical config shape stored in SQLite and used for YAML import/export.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ConfigFile {
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub groups: HashMap<String, GroupDef>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub environments: HashMap<String, EnvDef>,
}

/// A user-facing group of environments.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct GroupDef {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub environments: HashMap<String, EnvDef>,
}

/// One environment profile containing variables and optional hooks.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct EnvDef {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub variables: HashMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hooks: Option<HooksDef>,
}

/// Optional commands to run after an environment is applied.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct HooksDef {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post: Option<Vec<String>>,
}

/// Lightweight group data returned to the sidebar.
#[derive(Debug, Serialize)]
pub struct GroupSummary {
    pub name: String,
    pub description: Option<String>,
    pub env_count: usize,
}

/// Lightweight environment data returned to the sidebar.
#[derive(Debug, Serialize)]
pub struct EnvSummary {
    pub group: String,
    pub name: String,
    pub description: Option<String>,
    pub var_count: usize,
}

/// Per-variable apply result shown in the apply detail panel.
#[derive(Debug, Serialize)]
pub struct VariableApplyResult {
    pub key: String,
    pub before: Option<String>,
    pub after: String,
    pub applied: bool,
    pub message: String,
}

/// Result entry for a post-apply hook command.
#[derive(Debug, Serialize)]
pub struct HookResult {
    pub command: String,
    pub success: bool,
    pub code: Option<i32>,
    pub message: String,
}

/// Aggregate response returned after applying one environment.
#[derive(Debug, Serialize)]
pub struct ApplyResult {
    pub group: String,
    pub environment: String,
    pub mode: String,
    pub summary: ApplySummary,
    pub variable_results: Vec<VariableApplyResult>,
    pub hook_results: Vec<HookResult>,
}

/// Summary counters for the apply operation.
#[derive(Debug, Serialize)]
pub struct ApplySummary {
    pub total: usize,
    pub success: usize,
    pub failed: usize,
    pub changed: usize,
}

/// Platform-specific hints sent to the frontend.
#[derive(Debug, Serialize)]
pub struct RuntimeCapabilities {
    pub platform: String,
    pub apply_scope_hint: String,
    pub shell_integration_file: Option<String>,
}

/// Export request payload.
#[derive(Debug, Deserialize)]
pub struct ExportRequest {
    pub output_path: String,
    #[serde(default)]
    pub group_names: Vec<String>,
}

/// Export result payload.
#[derive(Debug, Serialize)]
pub struct ExportResult {
    pub output_path: String,
    pub groups: usize,
    pub environments: usize,
    pub variables: usize,
}

/// Where imported groups should land.
#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum ImportTargetMode {
    KeepGroups,
    IntoGroup,
}

/// How to handle key conflicts during import.
#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum ImportConflictStrategy {
    SkipExisting,
    OverwriteExisting,
    OnlyAddNew,
}

/// Import request payload.
#[derive(Debug, Deserialize)]
pub struct ImportRequest {
    pub input_path: String,
    pub target_mode: ImportTargetMode,
    pub target_group: Option<String>,
    pub conflict_strategy: ImportConflictStrategy,
    #[serde(default)]
    pub dry_run: bool,
}

/// Summary of what happened during import.
#[derive(Debug, Serialize, Default)]
pub struct ImportSummary {
    pub groups_added: usize,
    pub groups_skipped: usize,
    pub envs_added: usize,
    pub envs_skipped: usize,
    pub vars_added: usize,
    pub vars_overwritten: usize,
    pub vars_skipped: usize,
}
