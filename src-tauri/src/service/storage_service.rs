use crate::domain::{ConfigFile, EnvDef, GroupDef, HooksDef};
use rusqlite::{params, Connection};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// Owns local config discovery, SQLite persistence, and legacy YAML bootstrapping.
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
            .map(|p| p
                .join("config")
                .join("environments.yaml")
                .display()
                .to_string())
            .unwrap_or_else(|| "<no-parent>".to_string())
    ))
}

fn clicky_data_dir() -> Result<PathBuf, String> {
    // Store app-owned data under the user profile so the app stays portable across working directories.
    let home = std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .ok_or_else(|| "failed to resolve user home directory".to_string())?;
    Ok(PathBuf::from(home).join(".clicky"))
}

/// Returns the location of the SQLite database used by the desktop app.
pub fn db_path() -> Result<PathBuf, String> {
    Ok(clicky_data_dir()?.join("environments.db"))
}

/// Normalizes old YAML layouts into the current grouped config shape.
pub fn normalize_config(mut cfg: ConfigFile) -> ConfigFile {
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

pub fn load_config_from_yaml() -> Result<ConfigFile, String> {
    let path = config_path()?;
    let content = fs::read_to_string(&path)
        .map_err(|e| format!("failed to read {}: {}", path.display(), e))?;
    let cfg = serde_yaml::from_str::<ConfigFile>(&content)
        .map_err(|e| format!("failed to parse {}: {}", path.display(), e))?;
    Ok(normalize_config(cfg))
}

pub fn open_db() -> Result<Connection, String> {
    let path = db_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            format!(
                "failed to create data directory {}: {}",
                parent.display(),
                e
            )
        })?;
    }
    Connection::open(path).map_err(|e| format!("failed to open sqlite db: {}", e))
}

/// Creates the SQLite tables when the app starts or when the database is new.
pub fn ensure_db_schema(conn: &Connection) -> Result<(), String> {
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

pub fn load_config_from_db(conn: &Connection) -> Result<ConfigFile, String> {
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
                Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?))
            })
            .map_err(|e| format!("failed to map groups: {}", e))?;

        for row in rows {
            let (name, description) =
                row.map_err(|e| format!("failed to read group row: {}", e))?;
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

        // Read variables per environment so the SQL stays simple and the data shape stays explicit.
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

/// Writes the full in-memory config back to SQLite in one transaction.
pub fn save_config_to_db(conn: &mut Connection, cfg: &ConfigFile) -> Result<(), String> {
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
                .map_err(|e| {
                    format!(
                        "failed to serialize hooks for {}/{}: {}",
                        group_name, env_name, e
                    )
                })?;

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

/// Loads the current config from the persistent database, creating the schema first if needed.
pub fn load_config() -> Result<ConfigFile, String> {
    let conn = open_db()?;
    ensure_db_schema(&conn)?;
    load_config_from_db(&conn)
}

/// Saves the current config to the persistent database.
pub fn save_config(cfg: &ConfigFile) -> Result<(), String> {
    let mut conn = open_db()?;
    ensure_db_schema(&conn)?;
    save_config_to_db(&mut conn, cfg)
}
