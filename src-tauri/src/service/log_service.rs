use crate::service::storage_service;
use chrono::{Local, NaiveDate};
use log::{Level, LevelFilter, Log, Metadata, Record};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

const LOG_RETENTION_DAYS: i64 = 30;

static LOGGER: ClickyLogger = ClickyLogger;
static INIT: OnceLock<()> = OnceLock::new();

pub fn init() -> Result<(), String> {
    if INIT.get().is_some() {
        return Ok(());
    }

    cleanup_old_logs(LOG_RETENTION_DAYS)?;
    touch_today_log_files()?;
    log::set_logger(&LOGGER).map_err(|e| format!("{:?}", e))?;
    log::set_max_level(LevelFilter::Info);
    let _ = INIT.set(());
    Ok(())
}

struct ClickyLogger;

impl Log for ClickyLogger {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        matches!(metadata.level(), Level::Info | Level::Warn | Level::Error)
    }

    fn log(&self, record: &Record<'_>) {
        if !self.enabled(record.metadata()) {
            return;
        }

        if let Ok((path, line)) = format_log_line(record) {
            let _ = write_line(&path, &line);
        }
    }

    fn flush(&self) {}
}

fn format_log_line(record: &Record<'_>) -> Result<(PathBuf, String), String> {
    let base = log_dir()?;
    let date = Local::now().format("%Y-%m-%d").to_string();
    let level_name = match record.level() {
        Level::Info => "info",
        Level::Warn => "warn",
        Level::Error => "error",
        _ => return Err("unsupported log level".to_string()),
    };
    let path = base.join(format!("{}-{}.log", date, level_name));
    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
    let target = record.target();
    let line = format!(
        "[{}][{}][{}] {}\n",
        timestamp,
        record.level(),
        target,
        record.args()
    );
    Ok((path, line))
}

fn log_dir() -> Result<PathBuf, String> {
    Ok(storage_service::db_path()?
        .parent()
        .ok_or_else(|| "failed to resolve clicky data directory".to_string())?
        .join("log"))
}

fn touch_today_log_files() -> Result<(), String> {
    let base = log_dir()?;
    let date = Local::now().format("%Y-%m-%d").to_string();
    for level in ["info", "warn", "error"] {
        let path = base.join(format!("{}-{}.log", date, level));
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                format!("failed to create log directory {}: {}", parent.display(), e)
            })?;
        }
        let _ = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|e| format!("failed to create log file {}: {}", path.display(), e))?;
    }
    Ok(())
}

pub(crate) fn cleanup_old_logs(retention_days: i64) -> Result<(), String> {
    if retention_days < 0 {
        return Err("log retention days cannot be negative".to_string());
    }

    let dir = log_dir()?;
    if !dir.exists() {
        return Ok(());
    }

    let today = Local::now().date_naive();
    let cutoff = today
        .checked_sub_signed(chrono::Duration::days(retention_days))
        .unwrap_or(today);

    for entry in fs::read_dir(&dir)
        .map_err(|e| format!("failed to read log directory {}: {}", dir.display(), e))?
    {
        let entry = entry.map_err(|e| format!("failed to read log entry: {}", e))?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let Some((date, _level)) = parse_log_file_name(&path) else {
            continue;
        };

        if date < cutoff {
            let _ = fs::remove_file(&path);
        }
    }

    Ok(())
}

fn parse_log_file_name(path: &Path) -> Option<(NaiveDate, &str)> {
    let file_name = path.file_name()?.to_str()?;
    let (date_part, level_part) = file_name.rsplit_once('-')?;
    let level = level_part.strip_suffix(".log")?;
    if !matches!(level, "info" | "warn" | "error") {
        return None;
    }
    let date = NaiveDate::parse_from_str(date_part, "%Y-%m-%d").ok()?;
    Some((date, level))
}

fn write_line(path: &Path, line: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create log directory {}: {}", parent.display(), e))?;
    }

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| format!("failed to open log file {}: {}", path.display(), e))?;
    file.write_all(line.as_bytes())
        .map_err(|e| format!("failed to write log file {}: {}", path.display(), e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
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
        std::env::temp_dir().join(format!("clicky-{}-{}", prefix, stamp))
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

    #[test]
    fn cleanup_old_logs_removes_expired_files() {
        let _lock = test_lock();
        let _guard = DataDirGuard::new("log-cleanup");

        let dir = log_dir().expect("resolve log dir");
        fs::create_dir_all(&dir).expect("create log dir");

        let today = Local::now().date_naive();
        let old_date = today
            .checked_sub_signed(chrono::Duration::days(LOG_RETENTION_DAYS + 1))
            .expect("old date");
        let recent_date = today
            .checked_sub_signed(chrono::Duration::days(1))
            .expect("recent date");

        let old_file = dir.join(format!("{}-info.log", old_date.format("%Y-%m-%d")));
        let recent_file = dir.join(format!("{}-warn.log", recent_date.format("%Y-%m-%d")));
        fs::write(&old_file, "old").expect("write old file");
        fs::write(&recent_file, "recent").expect("write recent file");

        cleanup_old_logs(LOG_RETENTION_DAYS).expect("cleanup logs");

        assert!(!old_file.exists());
        assert!(recent_file.exists());
    }
}
