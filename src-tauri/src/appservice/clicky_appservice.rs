use crate::domain::*;
use crate::service::{storage_service, system_service};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;

/// Business orchestration for groups, environments, import/export, and apply flows.
pub fn list_groups() -> Result<Vec<GroupSummary>, String> {
    let cfg = storage_service::load_config()?;
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

pub fn create_group(group_name: String) -> Result<(), String> {
    let mut cfg = storage_service::load_config()?;
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
    storage_service::save_config(&cfg)
}

pub fn rename_group(old_name: String, new_name: String) -> Result<(), String> {
    let mut cfg = storage_service::load_config()?;
    let old_name = old_name.trim();
    let new_name = new_name.trim();
    if old_name.is_empty() || new_name.is_empty() {
        return Err("group name is required".to_string());
    }
    if old_name == new_name {
        return Ok(());
    }
    if !cfg.groups.contains_key(old_name) {
        return Err(format!("group '{}' not found", old_name));
    }
    if cfg.groups.contains_key(new_name) {
        return Err(format!("group '{}' already exists", new_name));
    }
    let Some(group) = cfg.groups.remove(old_name) else {
        return Err(format!("group '{}' not found", old_name));
    };
    cfg.groups.insert(new_name.to_string(), group);
    storage_service::save_config(&cfg)
}

pub fn delete_group(group_name: String) -> Result<(), String> {
    let mut cfg = storage_service::load_config()?;
    let name = group_name.trim();
    if name.is_empty() {
        return Err("group name is required".to_string());
    }
    if cfg.groups.remove(name).is_none() {
        return Err(format!("group '{}' not found", name));
    }
    storage_service::save_config(&cfg)
}

pub fn rename_environment(
    group_name: String,
    old_name: String,
    new_name: String,
) -> Result<(), String> {
    let mut cfg = storage_service::load_config()?;
    let group = cfg
        .groups
        .get_mut(group_name.trim())
        .ok_or_else(|| format!("group '{}' not found", group_name))?;
    let old_name = old_name.trim();
    let new_name = new_name.trim();
    if old_name.is_empty() || new_name.is_empty() {
        return Err("environment name is required".to_string());
    }
    if old_name == new_name {
        return Ok(());
    }
    if !group.environments.contains_key(old_name) {
        return Err(format!(
            "environment '{}' not found in group '{}'",
            old_name, group_name
        ));
    }
    if group.environments.contains_key(new_name) {
        return Err(format!(
            "environment '{}' already exists in group '{}'",
            new_name, group_name
        ));
    }
    let Some(env) = group.environments.remove(old_name) else {
        return Err(format!(
            "environment '{}' not found in group '{}'",
            old_name, group_name
        ));
    };
    group.environments.insert(new_name.to_string(), env);
    storage_service::save_config(&cfg)
}

pub fn delete_environment(group_name: String, env_name: String) -> Result<(), String> {
    let mut cfg = storage_service::load_config()?;
    let group = cfg
        .groups
        .get_mut(group_name.trim())
        .ok_or_else(|| format!("group '{}' not found", group_name))?;
    let name = env_name.trim();
    if name.is_empty() {
        return Err("environment name is required".to_string());
    }
    if group.environments.remove(name).is_none() {
        return Err(format!(
            "environment '{}' not found in group '{}'",
            name, group_name
        ));
    }
    storage_service::save_config(&cfg)
}

pub fn list_environments(group_name: String) -> Result<Vec<EnvSummary>, String> {
    let cfg = storage_service::load_config()?;
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

pub fn get_environment_variables(
    group_name: String,
    env_name: String,
) -> Result<HashMap<String, String>, String> {
    let cfg = storage_service::load_config()?;
    let group = cfg
        .groups
        .get(&group_name)
        .ok_or_else(|| format!("group '{}' not found", group_name))?;
    group
        .environments
        .get(&env_name)
        .map(|e| e.variables.clone())
        .ok_or_else(|| {
            format!(
                "environment '{}' not found in group '{}'",
                env_name, group_name
            )
        })
}

pub fn detect_active_environments(group_name: String) -> Result<Vec<String>, String> {
    let cfg = storage_service::load_config()?;
    let group = cfg
        .groups
        .get(&group_name)
        .ok_or_else(|| format!("group '{}' not found", group_name))?;

    let mut matches = Vec::new();
    for (env_name, env_def) in &group.environments {
        let all_match = env_def.variables.iter().all(|(k, v)| {
            system_service::read_persistent_var(k)
                .ok()
                .flatten()
                .map(|current| current == *v)
                .unwrap_or(false)
        });
        if all_match {
            matches.push(env_name.clone());
        }
    }

    matches.sort();
    Ok(matches)
}

pub fn save_environment_variables(
    group_name: String,
    env_name: String,
    variables: HashMap<String, String>,
) -> Result<(), String> {
    let mut cfg = storage_service::load_config()?;

    let existing_group = cfg.groups.entry(group_name.clone()).or_insert(GroupDef {
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
    normalize_group_variable_keys(existing_group);
    validate_group_variable_uniqueness(&cfg)?;
    storage_service::save_config(&cfg)
}

fn normalize_group_variable_keys(group: &mut GroupDef) {
    // Keep every environment in a group keyed by the same set of names so the editor can stay column-aligned.
    let mut key_set: HashSet<String> = HashSet::new();
    for env in group.environments.values() {
        for key in env.variables.keys() {
            key_set.insert(key.clone());
        }
    }

    if key_set.is_empty() {
        return;
    }

    for env in group.environments.values_mut() {
        for key in &key_set {
            env.variables.entry(key.clone()).or_insert_with(String::new);
        }
    }
}

pub fn export_config_flow(req: ExportRequest) -> Result<ExportResult, String> {
    let cfg = storage_service::load_config()?;
    let subset = export_subset(&cfg, &req.group_names)?;
    let content = serde_yaml::to_string(&subset)
        .map_err(|e| format!("failed to serialize export yaml: {}", e))?;

    let path = PathBuf::from(req.output_path.clone());
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            format!(
                "failed to create export directory {}: {}",
                parent.display(),
                e
            )
        })?;
    }
    fs::write(&path, content)
        .map_err(|e| format!("failed to write export file {}: {}", path.display(), e))?;

    let (groups, environments, variables) = config_counts(&subset);
    Ok(ExportResult {
        output_path: path.display().to_string(),
        groups,
        environments,
        variables,
    })
}

pub fn preview_import_config_flow(req: ImportRequest) -> Result<ImportSummary, String> {
    let content = fs::read_to_string(&req.input_path)
        .map_err(|e| format!("failed to read import file {}: {}", req.input_path, e))?;
    let imported = storage_service::normalize_config(
        serde_yaml::from_str::<ConfigFile>(&content)
            .map_err(|e| format!("failed to parse import yaml {}: {}", req.input_path, e))?,
    );

    let mut base = storage_service::load_config()?;
    let summary = merge_import(
        &mut base,
        &imported,
        req.target_mode,
        req.target_group.as_deref(),
        req.conflict_strategy,
    )?;
    validate_group_variable_uniqueness(&base)?;
    Ok(summary)
}

pub fn import_config_flow(req: ImportRequest) -> Result<ImportSummary, String> {
    let content = fs::read_to_string(&req.input_path)
        .map_err(|e| format!("failed to read import file {}: {}", req.input_path, e))?;
    let imported = storage_service::normalize_config(
        serde_yaml::from_str::<ConfigFile>(&content)
            .map_err(|e| format!("failed to parse import yaml {}: {}", req.input_path, e))?,
    );

    let mut base = storage_service::load_config()?;
    let summary = merge_import(
        &mut base,
        &imported,
        req.target_mode,
        req.target_group.as_deref(),
        req.conflict_strategy,
    )?;
    validate_group_variable_uniqueness(&base)?;
    if !req.dry_run {
        storage_service::save_config(&base)?;
    }
    Ok(summary)
}

pub fn apply_environment_flow(
    group_name: String,
    env_name: String,
    mode: String,
) -> Result<ApplyResult, String> {
    let cfg = storage_service::load_config()?;
    let group = cfg
        .groups
        .get(&group_name)
        .ok_or_else(|| format!("group '{}' not found", group_name))?;
    let env = group.environments.get(&env_name).ok_or_else(|| {
        format!(
            "environment '{}' not found in group '{}'",
            env_name, group_name
        )
    })?;

    if mode != "persistent" {
        return Err(
            "Clicky only supports persistent mode; reopen target processes after applying"
                .to_string(),
        );
    }

    let mut variable_results = Vec::new();
    let mut entries = env.variables.iter().collect::<Vec<_>>();
    entries.sort_by(|a, b| a.0.cmp(b.0));
    // Snapshot the variables once so shell integration can use the exact same sorted payload.
    let shell_items = entries
        .iter()
        .map(|(k, v)| ((*k).clone(), (*v).clone()))
        .collect::<Vec<_>>();

    for (k, v) in entries {
        let before = system_service::read_persistent_var(k).ok().flatten();
        let result = match system_service::apply_var_persistent(k, v) {
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

    // Notify once after the full batch so Windows doesn't pay the broadcast cost for every variable.
    let _ = system_service::notify_environment_change();

    if let Ok(Some(path)) = system_service::persist_shell_env_snapshot(&shell_items) {
        eprintln!("shell integration file updated: {}", path);
    }

    if let Ok(Some(path)) = system_service::persist_idea_env_snapshot(&shell_items) {
        eprintln!("IDEA env file updated: {}", path);
    }

    let hook_results = system_service::run_post_hooks(env.hooks.as_ref());
    let summary = build_apply_summary(&variable_results);

    Ok(ApplyResult {
        group: group_name,
        environment: env_name,
        mode,
        summary,
        variable_results,
        hook_results,
    })
}

pub fn get_runtime_capabilities() -> RuntimeCapabilities {
    system_service::runtime_capabilities()
}

fn build_apply_summary(results: &[VariableApplyResult]) -> ApplySummary {
    // Keep the summary derived from the detailed per-variable results.
    let total = results.len();
    let success = results.iter().filter(|item| item.applied).count();
    let changed = results
        .iter()
        .filter(|item| item.before.as_deref().unwrap_or("") != item.after.as_str())
        .count();
    ApplySummary {
        total,
        success,
        failed: total.saturating_sub(success),
        changed,
    }
}

fn validate_group_variable_uniqueness(cfg: &ConfigFile) -> Result<(), String> {
    // A variable name may exist in multiple environments of the same group, but not across groups.
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

fn config_counts(cfg: &ConfigFile) -> (usize, usize, usize) {
    let groups = cfg.groups.len();
    let mut envs = 0usize;
    let mut vars = 0usize;
    for group in cfg.groups.values() {
        envs += group.environments.len();
        for env in group.environments.values() {
            vars += env.variables.len();
        }
    }
    (groups, envs, vars)
}

fn export_subset(cfg: &ConfigFile, group_names: &[String]) -> Result<ConfigFile, String> {
    if group_names.is_empty() {
        return Ok(cfg.clone());
    }

    let mut subset = ConfigFile {
        groups: HashMap::new(),
        environments: HashMap::new(),
    };
    for name in group_names {
        let group = cfg
            .groups
            .get(name)
            .ok_or_else(|| format!("group '{}' not found", name))?;
        subset.groups.insert(name.clone(), group.clone());
    }
    Ok(subset)
}

fn merge_import(
    base: &mut ConfigFile,
    imported: &ConfigFile,
    target_mode: ImportTargetMode,
    target_group: Option<&str>,
    strategy: ImportConflictStrategy,
) -> Result<ImportSummary, String> {
    let mut summary = ImportSummary::default();
    let mut key_owner: HashMap<String, String> = HashMap::new();
    // Track which group currently owns each variable name so imports can respect the cross-group uniqueness rule.
    for (group_name, group) in &base.groups {
        for env in group.environments.values() {
            for key in env.variables.keys() {
                key_owner
                    .entry(key.clone())
                    .or_insert_with(|| group_name.clone());
            }
        }
    }

    match target_mode {
        ImportTargetMode::KeepGroups => {
            for (src_group_name, src_group) in &imported.groups {
                let dst_group_name = src_group_name.clone();
                let group_exists = base.groups.contains_key(&dst_group_name);

                if !group_exists {
                    base.groups.insert(
                        dst_group_name.clone(),
                        GroupDef {
                            description: src_group.description.clone(),
                            environments: HashMap::new(),
                        },
                    );
                    summary.groups_added += 1;
                } else {
                    summary.groups_skipped += 1;
                }

                if let Some(dst_group) = base.groups.get_mut(&dst_group_name) {
                    for (src_env_name, src_env) in &src_group.environments {
                        let env_exists = dst_group.environments.contains_key(src_env_name);
                        if !env_exists {
                            dst_group
                                .environments
                                .insert(src_env_name.clone(), src_env.clone());
                            summary.envs_added += 1;
                            summary.vars_added += src_env.variables.len();
                            for key in src_env.variables.keys() {
                                key_owner.insert(key.clone(), dst_group_name.clone());
                            }
                            continue;
                        }

                        summary.envs_skipped += 1;
                        let Some(dst_env) = dst_group.environments.get_mut(src_env_name) else {
                            continue;
                        };

                        for (key, value) in &src_env.variables {
                            if let Some(owner) = key_owner.get(key) {
                                if owner != &dst_group_name {
                                    summary.vars_skipped += 1;
                                    continue;
                                }
                            }
                            match dst_env.variables.get(key) {
                                None => {
                                    dst_env.variables.insert(key.clone(), value.clone());
                                    key_owner.insert(key.clone(), dst_group_name.clone());
                                    summary.vars_added += 1;
                                }
                                Some(_) => match strategy {
                                    ImportConflictStrategy::SkipExisting
                                    | ImportConflictStrategy::OnlyAddNew => {
                                        summary.vars_skipped += 1;
                                    }
                                    ImportConflictStrategy::OverwriteExisting => {
                                        dst_env.variables.insert(key.clone(), value.clone());
                                        summary.vars_overwritten += 1;
                                    }
                                },
                            }
                        }
                    }
                }
            }
        }
        ImportTargetMode::IntoGroup => {
            let target = target_group
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty())
                .ok_or_else(|| "target_group is required for into_group mode".to_string())?;

            if !base.groups.contains_key(&target) {
                base.groups.insert(
                    target.clone(),
                    GroupDef {
                        description: Some(format!("{} group", target)),
                        environments: HashMap::new(),
                    },
                );
                summary.groups_added += 1;
            } else {
                summary.groups_skipped += 1;
            }

            let dst_group = base
                .groups
                .get_mut(&target)
                .ok_or_else(|| format!("target group '{}' not found", target))?;

            for src_group in imported.groups.values() {
                for (src_env_name, src_env) in &src_group.environments {
                    let env_exists = dst_group.environments.contains_key(src_env_name);
                    if !env_exists {
                        dst_group
                            .environments
                            .insert(src_env_name.clone(), src_env.clone());
                        summary.envs_added += 1;
                        summary.vars_added += src_env.variables.len();
                        for key in src_env.variables.keys() {
                            key_owner.insert(key.clone(), target.clone());
                        }
                        continue;
                    }

                    summary.envs_skipped += 1;
                    let Some(dst_env) = dst_group.environments.get_mut(src_env_name) else {
                        continue;
                    };

                    for (key, value) in &src_env.variables {
                        if let Some(owner) = key_owner.get(key) {
                            if owner != &target {
                                summary.vars_skipped += 1;
                                continue;
                            }
                        }
                        match dst_env.variables.get(key) {
                            None => {
                                dst_env.variables.insert(key.clone(), value.clone());
                                key_owner.insert(key.clone(), target.clone());
                                summary.vars_added += 1;
                            }
                            Some(_) => match strategy {
                                ImportConflictStrategy::SkipExisting
                                | ImportConflictStrategy::OnlyAddNew => {
                                    summary.vars_skipped += 1;
                                }
                                ImportConflictStrategy::OverwriteExisting => {
                                    dst_env.variables.insert(key.clone(), value.clone());
                                    summary.vars_overwritten += 1;
                                }
                            },
                        }
                    }
                }
            }
        }
    }

    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::{Mutex, OnceLock};
    use std::time::{SystemTime, UNIX_EPOCH};

    static TEST_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();

    fn test_lock() -> std::sync::MutexGuard<'static, ()> {
        TEST_MUTEX
            .get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn unique_temp_dir(prefix: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock before UNIX_EPOCH")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "clicky-{}-{}-{}",
            prefix,
            std::process::id(),
            stamp
        ))
    }

    struct DataDirGuard {
        path: PathBuf,
        old_home: Option<std::ffi::OsString>,
        old_userprofile: Option<std::ffi::OsString>,
    }

    impl DataDirGuard {
        fn new(prefix: &str) -> Self {
            let path = unique_temp_dir(prefix);
            fs::create_dir_all(&path).expect("create temp data dir");
            let old_home = std::env::var_os("HOME");
            let old_userprofile = std::env::var_os("USERPROFILE");
            std::env::set_var("HOME", &path);
            std::env::set_var("USERPROFILE", &path);
            Self {
                path,
                old_home,
                old_userprofile,
            }
        }
    }

    impl Drop for DataDirGuard {
        fn drop(&mut self) {
            match &self.old_home {
                Some(value) => std::env::set_var("HOME", value),
                None => std::env::remove_var("HOME"),
            }
            match &self.old_userprofile {
                Some(value) => std::env::set_var("USERPROFILE", value),
                None => std::env::remove_var("USERPROFILE"),
            }
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    fn sample_config() -> ConfigFile {
        let mut variables = HashMap::new();
        variables.insert(
            "CLICKY_TEST_API_URL".to_string(),
            "https://dev.example".to_string(),
        );
        variables.insert("CLICKY_TEST_TOKEN".to_string(), "token-123".to_string());

        let mut environments = HashMap::new();
        environments.insert(
            "dev".to_string(),
            EnvDef {
                description: Some("development".to_string()),
                variables,
                hooks: Some(HooksDef {
                    post: Some(vec!["echo Clicky hook".to_string()]),
                }),
            },
        );

        let mut groups = HashMap::new();
        groups.insert(
            "default".to_string(),
            GroupDef {
                description: Some("default group".to_string()),
                environments,
            },
        );

        ConfigFile {
            groups,
            environments: HashMap::new(),
        }
    }

    #[cfg(target_os = "windows")]
    fn remove_registry_env_value(key: &str) {
        use winreg::enums::{HKEY_CURRENT_USER, KEY_WRITE};
        use winreg::RegKey;

        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        if let Ok(env_key) = hkcu.open_subkey_with_flags("Environment", KEY_WRITE) {
            let _ = env_key.delete_value(key);
        }
    }

    #[cfg(target_os = "windows")]
    fn cleanup_registry_values(keys: &[String]) {
        for key in keys {
            remove_registry_env_value(key);
        }
    }

    #[test]
    fn acceptance_export_import_roundtrip() {
        let _lock = test_lock();
        let _guard = DataDirGuard::new("export-import");

        let config = sample_config();
        storage_service::save_config(&config).expect("save initial config");

        let export_path = unique_temp_dir("export-file").with_extension("yaml");
        let export_result = export_config_flow(ExportRequest {
            output_path: export_path.display().to_string(),
            group_names: vec!["default".to_string()],
        })
        .expect("export config");
        assert_eq!(export_result.groups, 1);
        assert_eq!(export_result.environments, 1);
        assert_eq!(export_result.variables, 2);

        let exported = fs::read_to_string(&export_path).expect("read exported yaml");
        assert!(exported.contains("default"));
        assert!(exported.contains("CLICKY_TEST_API_URL"));

        storage_service::save_config(&ConfigFile {
            groups: HashMap::new(),
            environments: HashMap::new(),
        })
        .expect("reset db");

        let preview = preview_import_config_flow(ImportRequest {
            input_path: export_path.display().to_string(),
            target_mode: ImportTargetMode::KeepGroups,
            target_group: None,
            conflict_strategy: ImportConflictStrategy::SkipExisting,
            dry_run: true,
        })
        .expect("preview import");
        assert_eq!(preview.groups_added, 1);
        assert_eq!(preview.envs_added, 1);
        assert_eq!(preview.vars_added, 2);

        let imported = import_config_flow(ImportRequest {
            input_path: export_path.display().to_string(),
            target_mode: ImportTargetMode::KeepGroups,
            target_group: None,
            conflict_strategy: ImportConflictStrategy::SkipExisting,
            dry_run: false,
        })
        .expect("import config");
        assert_eq!(imported.groups_added, 1);
        assert_eq!(imported.envs_added, 1);
        assert_eq!(imported.vars_added, 2);

        let groups = list_groups().expect("list groups");
        assert_eq!(groups.len(), 1);
        let envs = list_environments("default".to_string()).expect("list envs");
        assert_eq!(envs.len(), 1);
        let vars =
            get_environment_variables("default".to_string(), "dev".to_string()).expect("get vars");
        assert_eq!(
            vars.get("CLICKY_TEST_API_URL"),
            Some(&"https://dev.example".to_string())
        );
        assert_eq!(
            vars.get("CLICKY_TEST_TOKEN"),
            Some(&"token-123".to_string())
        );

        let _ = fs::remove_file(&export_path);
    }

    #[test]
    fn acceptance_hooks_run_and_report() {
        let _lock = test_lock();
        let _guard = DataDirGuard::new("hooks");

        let config = sample_config();
        storage_service::save_config(&config).expect("save config");

        let result = apply_environment_flow(
            "default".to_string(),
            "dev".to_string(),
            "persistent".to_string(),
        )
        .expect("apply config");

        assert_eq!(result.summary.total, 2);
        assert_eq!(result.summary.success, 2);
        assert_eq!(result.summary.failed, 0);
        assert_eq!(result.hook_results.len(), 1);
        assert!(result.hook_results[0].success);
        assert!(result.hook_results[0].message.contains("Clicky hook"));
    }

    #[test]
    fn acceptance_idea_env_snapshot_written_and_updated() {
        let _lock = test_lock();
        let _guard = DataDirGuard::new("idea-env");

        let mut dev_vars = HashMap::new();
        dev_vars.insert(
            "CLICKY_TEST_API_URL".to_string(),
            "https://dev.example".to_string(),
        );
        dev_vars.insert("CLICKY_TEST_TOKEN".to_string(), "token-123".to_string());

        let mut sit_vars = HashMap::new();
        sit_vars.insert(
            "CLICKY_TEST_API_URL".to_string(),
            "https://sit.example".to_string(),
        );
        sit_vars.insert("CLICKY_TEST_TOKEN".to_string(), "token-456".to_string());

        let mut environments = HashMap::new();
        environments.insert(
            "dev".to_string(),
            EnvDef {
                description: Some("development".to_string()),
                variables: dev_vars,
                hooks: None,
            },
        );
        environments.insert(
            "sit".to_string(),
            EnvDef {
                description: Some("system integration test".to_string()),
                variables: sit_vars,
                hooks: None,
            },
        );

        let mut groups = HashMap::new();
        groups.insert(
            "default".to_string(),
            GroupDef {
                description: Some("default group".to_string()),
                environments,
            },
        );

        storage_service::save_config(&ConfigFile {
            groups,
            environments: HashMap::new(),
        })
        .expect("save config");

        let result = apply_environment_flow(
            "default".to_string(),
            "dev".to_string(),
            "persistent".to_string(),
        )
        .expect("apply dev");
        assert_eq!(result.summary.success, 2);

        let idea_env_file = storage_service::db_path()
            .expect("resolve db path")
            .parent()
            .expect("resolve clicky data dir")
            .join("idea")
            .join("current.env");

        let first = fs::read_to_string(&idea_env_file).expect("read idea env file");
        assert!(first.contains("CLICKY_TEST_API_URL=https://dev.example"));
        assert!(first.contains("CLICKY_TEST_TOKEN=token-123"));

        let result = apply_environment_flow(
            "default".to_string(),
            "sit".to_string(),
            "persistent".to_string(),
        )
        .expect("apply sit");
        assert_eq!(result.summary.success, 2);

        let second = fs::read_to_string(&idea_env_file).expect("read updated idea env file");
        assert!(second.contains("CLICKY_TEST_API_URL=https://sit.example"));
        assert!(second.contains("CLICKY_TEST_TOKEN=token-456"));
        assert!(!second.contains("https://dev.example"));
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn acceptance_windows_apply_and_detect() {
        let _lock = test_lock();
        let _guard = DataDirGuard::new("windows-apply");

        let key = format!("CLICKY_TEST_ENV_{}_A", std::process::id());
        let value = format!("value-{}", unique_temp_dir("windows-key").display());
        cleanup_registry_values(std::slice::from_ref(&key));
        std::env::remove_var(&key);

        let mut variables = HashMap::new();
        variables.insert(key.clone(), value.clone());

        let mut environments = HashMap::new();
        environments.insert(
            "sit".to_string(),
            EnvDef {
                description: Some("system integration test".to_string()),
                variables,
                hooks: None,
            },
        );

        let mut groups = HashMap::new();
        groups.insert(
            "default".to_string(),
            GroupDef {
                description: Some("default group".to_string()),
                environments,
            },
        );

        storage_service::save_config(&ConfigFile {
            groups,
            environments: HashMap::new(),
        })
        .expect("save config");

        let before =
            detect_active_environments("default".to_string()).expect("detect before apply");
        assert!(before.is_empty());

        let result = apply_environment_flow(
            "default".to_string(),
            "sit".to_string(),
            "persistent".to_string(),
        )
        .expect("apply config");
        assert_eq!(result.summary.total, 1);
        assert_eq!(result.summary.success, 1);

        let active = detect_active_environments("default".to_string()).expect("detect after apply");
        assert_eq!(active, vec!["sit".to_string()]);

        let capabilities = get_runtime_capabilities();
        assert_eq!(capabilities.platform, "windows");
        assert!(capabilities.apply_scope_hint.contains("新开进程"));

        cleanup_registry_values(std::slice::from_ref(&key));
        std::env::remove_var(&key);
    }
}
