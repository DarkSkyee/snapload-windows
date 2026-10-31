use chrono::Local;
use std::fs::{create_dir_all, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

static LOG_MUTEX: Mutex<()> = Mutex::new(());

pub struct Logger;

impl Logger {
    fn log_file_path() -> Option<PathBuf> {
        let local_app_data = std::env::var_os("LOCALAPPDATA")?;
        let mut path = PathBuf::from(local_app_data);
        path.push("SnapLoad");
        path.push("logs");
        let _ = create_dir_all(&path);
        path.push("engine.log");
        Some(path)
    }

    pub fn log(level: &str, message: &str) {
        let _guard = LOG_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(path) = Self::log_file_path() {
            if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
                let now = Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
                let _ = writeln!(file, "[{}] [{}] {}", now, level, message);
            }
        }
    }

    pub fn info(message: &str) {
        Self::log("INFO", message);
    }

    pub fn warn(message: &str) {
        Self::log("WARN", message);
    }

    pub fn error(message: &str) {
        Self::log("ERROR", message);
    }

    pub fn debug(message: &str) {
        Self::log("DEBUG", message);
    }
}
