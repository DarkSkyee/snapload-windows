use std::fs::{create_dir_all, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

static LOG_MUTEX: Mutex<()> = Mutex::new(());

pub struct Logger;

fn civil_from_days(days: i64) -> (i32, u32, u32) {
    let z = days + 719468;
    let era = (if z >= 0 { z } else { z - 146096 }) / 146097;
    let doe = (z - era * 146097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = (yoe as i64) + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y as i32, m, d)
}

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

    pub fn timestamp_utc() -> String {
        let duration = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        let secs = duration.as_secs();
        let millis = duration.subsec_millis();

        let days = (secs / 86400) as i64;
        let rem_secs = (secs % 86400) as u32;
        let hours = rem_secs / 3600;
        let minutes = (rem_secs % 3600) / 60;
        let seconds = rem_secs % 60;

        let (y, m, d) = civil_from_days(days);
        format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z",
            y, m, d, hours, minutes, seconds, millis
        )
    }

    pub fn timestamp_log() -> String {
        let duration = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        let secs = duration.as_secs();
        let millis = duration.subsec_millis();

        let days = (secs / 86400) as i64;
        let rem_secs = (secs % 86400) as u32;
        let hours = rem_secs / 3600;
        let minutes = (rem_secs % 3600) / 60;
        let seconds = rem_secs % 60;

        let (y, m, d) = civil_from_days(days);
        format!(
            "{:04}-{:02}-{:02} {:02}:{:02}:{:02}.{:03}",
            y, m, d, hours, minutes, seconds, millis
        )
    }

    pub fn log(level: &str, message: &str) {
        let _guard = LOG_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(path) = Self::log_file_path() {
            if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
                let now = Self::timestamp_log();
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