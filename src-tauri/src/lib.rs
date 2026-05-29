use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use rusqlite::{params, Connection};

#[derive(Debug, Deserialize, Serialize, Clone)]
struct ConfigFile {
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    groups: HashMap<String, GroupDef>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    environments: HashMap<String, EnvDef>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct GroupDef {
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    environments: HashMap<String, EnvDef>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct EnvDef {
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    variables: HashMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    hooks: Option<HooksDef>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct HooksDef {
    #[serde(skip_serializing_if = "Option::is_none")]
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

#[derive(Debug, Deserialize)]
struct ExportRequest {
    output_path: String,
    #[serde(default)]
    group_names: Vec<String>,
}

#[derive(Debug, Serialize)]
struct ExportResult {
    output_path: String,
    groups: usize,
    environments: usize,
    variables: usize,
}

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
enum ImportTargetMode {
    KeepGroups,
    IntoGroup,
}

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
enum ImportConflictStrategy {
    SkipExisting,
    OverwriteExisting,
    OnlyAddNew,
}

#[derive(Debug, Deserialize)]
struct ImportRequest {
    input_path: String,
    target_mode: ImportTargetMode,
    target_group: Option<String>,
    conflict_strategy: ImportConflictStrategy,
    #[serde(default)]
    dry_run: bool,
}

#[derive(Debug, Serialize, Default)]
struct ImportSummary {
    groups_added: usize,
    groups_skipped: usize,
    envs_added: usize,
    envs_skipped: usize,
    vars_added: usize,
    vars_overwritten: usize,
    vars_skipped: usize,
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

fn clicky_data_dir() -> Result<PathBuf, String> {
    let home = std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .ok_or_else(|| "failed to resolve user home directory".to_string())?;
    Ok(PathBuf::from(home).join(".clicky"))
}

fn db_path() -> Result<PathBuf, String> {
    Ok(clicky_data_dir()?.join("environments.db"))
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

fn load_config_from_yaml() -> Result<ConfigFile, String> {
    let path = config_path()?;
    let content =
        fs::read_to_string(&path).map_err(|e| format!("failed to read {}: {}", path.display(), e))?;
    let cfg = serde_yaml::from_str::<ConfigFile>(&content)
        .map_err(|e| format!("failed to parse {}: {}", path.display(), e))?;
    Ok(normalize_config(cfg))
}

fn open_db() -> Result<Connection, String> {
    let path = db_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create data directory {}: {}", parent.display(), e))?;
    }
    Connection::open(path).map_err(|e| format!("failed to open sqlite db: {}", e))
}

fn ensure_db_schema(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "
        PRAGMA foreign_keys = ON;
        CREATE TABLE IF NOT EXISTS groups (
            name TEXT PRIMARY KEY,
            description TEXT
        );
        CREATE TABLE IF NOT EXISTS environments (
            group_name TEXT NOT NULL,
            name TEXT NOT NULL,
            description TEXT,
            hooks_post_json TEXT,
            PRIMARY KEY (group_name, name),
            FOREIGN KEY(group_name) REFERENCES groups(name) ON DELETE CASCADE
        );
        CREATE TABLE IF NOT EXISTS variables (
            group_name TEXT NOT NULL,
            env_name TEXT NOT NULL,
            key TEXT NOT NULL,
            value TEXT NOT NULL,
            PRIMARY KEY (group_name, env_name, key),
            FOREIGN KEY(group_name, env_name) REFERENCES environments(group_name, name) ON DELETE CASCADE
        );
    ",
    )
    .map_err(|e| format!("failed to initialize sqlite schema: {}", e))
}

fn load_config_from_db(conn: &Connection) -> Result<ConfigFile, String> {
    let mut cfg = ConfigFile {
        groups: HashMap::new(),
        environments: HashMap::new(),
    };

    {
        let mut stmt = conn
            .prepare("SELECT name, description FROM groups ORDER BY name")
            .map_err(|e| format!("failed to query groups: {}", e))?;
        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<String>>(1)?,
                ))
            })
            .map_err(|e| format!("failed to map groups: {}", e))?;

        for row in rows {
            let (name, description) = row.map_err(|e| format!("failed to read group row: {}", e))?;
            cfg.groups.insert(
                name,
                GroupDef {
                    description,
                    environments: HashMap::new(),
                },
            );
        }
    }

    let mut env_stmt = conn
        .prepare(
            "
            SELECT e.group_name, e.name, e.description, e.hooks_post_json
            FROM environments e
            ORDER BY e.group_name, e.name
        ",
        )
        .map_err(|e| format!("failed to query environments: {}", e))?;
    let env_rows = env_stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<String>>(3)?,
            ))
        })
        .map_err(|e| format!("failed to map environments: {}", e))?;

    let mut var_stmt = conn
        .prepare(
            "
            SELECT key, value
            FROM variables
            WHERE group_name = ?1 AND env_name = ?2
            ORDER BY key
        ",
        )
        .map_err(|e| format!("failed to prepare variable query: {}", e))?;

    for env_row in env_rows {
        let (group_name, env_name, env_description, hooks_json) =
            env_row.map_err(|e| format!("failed to read environment row: {}", e))?;

        if !cfg.groups.contains_key(&group_name) {
            cfg.groups.insert(
                group_name.clone(),
                GroupDef {
                    description: Some(format!("{} group", group_name)),
                    environments: HashMap::new(),
                },
            );
        }

        let vars_iter = var_stmt
            .query_map(params![&group_name, &env_name], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(|e| format!("failed to query variables: {}", e))?;

        let mut variables = HashMap::new();
        for item in vars_iter {
            let (k, v) = item.map_err(|e| format!("failed to read variable row: {}", e))?;
            variables.insert(k, v);
        }

        let hooks = hooks_json
            .as_ref()
            .and_then(|json| serde_json::from_str::<Vec<String>>(json).ok())
            .map(|post| HooksDef { post: Some(post) });

        let env_def = EnvDef {
            description: env_description,
            variables,
            hooks,
        };

        if let Some(group) = cfg.groups.get_mut(&group_name) {
            group.environments.insert(env_name, env_def);
        }
    }

    Ok(cfg)
}

fn save_config_to_db(conn: &mut Connection, cfg: &ConfigFile) -> Result<(), String> {
    let tx = conn
        .transaction()
        .map_err(|e| format!("failed to start sqlite transaction: {}", e))?;

    tx.execute("DELETE FROM variables", [])
        .map_err(|e| format!("failed to clear variables: {}", e))?;
    tx.execute("DELETE FROM environments", [])
        .map_err(|e| format!("failed to clear environments: {}", e))?;
    tx.execute("DELETE FROM groups", [])
        .map_err(|e| format!("failed to clear groups: {}", e))?;

    for (group_name, group) in &cfg.groups {
        tx.execute(
            "INSERT INTO groups(name, description) VALUES (?1, ?2)",
            params![group_name, group.description],
        )
        .map_err(|e| format!("failed to insert group '{}': {}", group_name, e))?;

        for (env_name, env) in &group.environments {
            let hooks_json = env
                .hooks
                .as_ref()
                .and_then(|h| h.post.as_ref())
                .map(|post| serde_json::to_string(post))
                .transpose()
                .map_err(|e| format!("failed to serialize hooks for {}/{}: {}", group_name, env_name, e))?;

            tx.execute(
                "INSERT INTO environments(group_name, name, description, hooks_post_json) VALUES (?1, ?2, ?3, ?4)",
                params![group_name, env_name, env.description, hooks_json],
            )
            .map_err(|e| format!("failed to insert environment '{}/{}': {}", group_name, env_name, e))?;

            for (key, value) in &env.variables {
                tx.execute(
                    "INSERT INTO variables(group_name, env_name, key, value) VALUES (?1, ?2, ?3, ?4)",
                    params![group_name, env_name, key, value],
                )
                .map_err(|e| format!("failed to insert variable '{}/{}:{}': {}", group_name, env_name, key, e))?;
            }
        }
    }

    tx.commit()
        .map_err(|e| format!("failed to commit sqlite transaction: {}", e))
}

fn load_config() -> Result<ConfigFile, String> {
    let conn = open_db()?;
    ensure_db_schema(&conn)?;
    load_config_from_db(&conn)
}

fn save_config(cfg: &ConfigFile) -> Result<(), String> {
    let mut conn = open_db()?;
    ensure_db_schema(&conn)?;
    save_config_to_db(&mut conn, cfg)
}

fn init_storage() -> Result<(), String> {
    let path = db_path()?;
    let db_exists = path.exists();

    let mut conn = open_db()?;
    ensure_db_schema(&conn)?;

    if db_exists {
        return Ok(());
    }

    if let Ok(yaml_cfg) = load_config_from_yaml() {
        validate_group_variable_uniqueness(&yaml_cfg)?;
        save_config_to_db(&mut conn, &yaml_cfg)?;
    }

    Ok(())
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
    for (group_name, group) in &base.groups {
        for env in group.environments.values() {
            for key in env.variables.keys() {
                key_owner.entry(key.clone()).or_insert_with(|| group_name.clone());
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
                                Some(existing) => match strategy {
                                    ImportConflictStrategy::SkipExisting | ImportConflictStrategy::OnlyAddNew => {
                                        let _ = existing;
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
                            Some(existing) => match strategy {
                                ImportConflictStrategy::SkipExisting | ImportConflictStrategy::OnlyAddNew => {
                                    let _ = existing;
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
fn rename_group(old_name: String, new_name: String) -> Result<(), String> {
    let mut cfg = load_config()?;
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
    save_config(&cfg)
}

#[tauri::command]
fn delete_group(group_name: String) -> Result<(), String> {
    let mut cfg = load_config()?;
    let name = group_name.trim();
    if name.is_empty() {
        return Err("group name is required".to_string());
    }
    if cfg.groups.remove(name).is_none() {
        return Err(format!("group '{}' not found", name));
    }
    save_config(&cfg)
}

#[tauri::command]
fn rename_environment(group_name: String, old_name: String, new_name: String) -> Result<(), String> {
    let mut cfg = load_config()?;
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
        return Err(format!("environment '{}' not found in group '{}'", old_name, group_name));
    }
    if group.environments.contains_key(new_name) {
        return Err(format!("environment '{}' already exists in group '{}'", new_name, group_name));
    }
    let Some(env) = group.environments.remove(old_name) else {
        return Err(format!("environment '{}' not found in group '{}'", old_name, group_name));
    };
    group.environments.insert(new_name.to_string(), env);
    save_config(&cfg)
}

#[tauri::command]
fn delete_environment(group_name: String, env_name: String) -> Result<(), String> {
    let mut cfg = load_config()?;
    let group = cfg
        .groups
        .get_mut(group_name.trim())
        .ok_or_else(|| format!("group '{}' not found", group_name))?;
    let name = env_name.trim();
    if name.is_empty() {
        return Err("environment name is required".to_string());
    }
    if group.environments.remove(name).is_none() {
        return Err(format!("environment '{}' not found in group '{}'", name, group_name));
    }
    save_config(&cfg)
}

#[tauri::command]
fn export_config(req: ExportRequest) -> Result<ExportResult, String> {
    let cfg = load_config()?;
    let subset = export_subset(&cfg, &req.group_names)?;
    let content =
        serde_yaml::to_string(&subset).map_err(|e| format!("failed to serialize export yaml: {}", e))?;

    let path = PathBuf::from(req.output_path.clone());
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create export directory {}: {}", parent.display(), e))?;
    }
    fs::write(&path, content).map_err(|e| format!("failed to write export file {}: {}", path.display(), e))?;

    let (groups, environments, variables) = config_counts(&subset);
    Ok(ExportResult {
        output_path: path.display().to_string(),
        groups,
        environments,
        variables,
    })
}

#[tauri::command]
fn preview_import_config(req: ImportRequest) -> Result<ImportSummary, String> {
    let content = fs::read_to_string(&req.input_path)
        .map_err(|e| format!("failed to read import file {}: {}", req.input_path, e))?;
    let imported = normalize_config(
        serde_yaml::from_str::<ConfigFile>(&content)
            .map_err(|e| format!("failed to parse import yaml {}: {}", req.input_path, e))?,
    );

    let mut base = load_config()?;
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

#[tauri::command]
fn import_config(req: ImportRequest) -> Result<ImportSummary, String> {
    let content = fs::read_to_string(&req.input_path)
        .map_err(|e| format!("failed to read import file {}: {}", req.input_path, e))?;
    let imported = normalize_config(
        serde_yaml::from_str::<ConfigFile>(&content)
            .map_err(|e| format!("failed to parse import yaml {}: {}", req.input_path, e))?,
    );

    let mut base = load_config()?;
    let summary = merge_import(
        &mut base,
        &imported,
        req.target_mode,
        req.target_group.as_deref(),
        req.conflict_strategy,
    )?;
    validate_group_variable_uniqueness(&base)?;
    if !req.dry_run {
        save_config(&base)?;
    }
    Ok(summary)
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
        return Err("Clicky only supports persistent mode; reopen target processes after applying".to_string());
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
    if let Err(e) = init_storage() {
        eprintln!("storage initialization failed: {}", e);
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            list_groups,
            create_group,
            rename_group,
            delete_group,
            rename_environment,
            delete_environment,
            export_config,
            preview_import_config,
            import_config,
            list_environments,
            get_environment_variables,
            detect_active_environments,
            save_environment_variables,
            apply_environment
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
