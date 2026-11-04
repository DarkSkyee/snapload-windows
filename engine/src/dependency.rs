use crate::config::EngineConfig;
use crate::logger::Logger;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyInfo {
    pub name: String,
    pub installed: bool,
    pub path: Option<String>,
    pub version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyStatus {
    pub ytdlp: DependencyInfo,
    pub ffmpeg: DependencyInfo,
    pub ready: bool,
}

pub struct DependencyManager;

impl DependencyManager {
    pub fn find_ytdlp(config: &EngineConfig) -> Option<PathBuf> {
        // 1. Config custom path
        if let Some(custom) = &config.ytdlp_custom_path {
            if custom.exists() && custom.is_file() {
                return Some(custom.clone());
            }
        }

        // 2. LocalAppData\SnapLoad\bin\yt-dlp.exe
        if let Some(local_app_data) = std::env::var_os("LOCALAPPDATA") {
            let mut p = PathBuf::from(local_app_data);
            p.push("SnapLoad");
            p.push("bin");
            p.push("yt-dlp.exe");
            if p.exists() {
                return Some(p);
            }
        }

        // 3. Exe directory or Exe\bin
        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(parent) = exe_path.parent() {
                let candidate1 = parent.join("yt-dlp.exe");
                if candidate1.exists() {
                    return Some(candidate1);
                }
                let candidate2 = parent.join("bin").join("yt-dlp.exe");
                if candidate2.exists() {
                    return Some(candidate2);
                }
            }
        }

        // 4. System PATH
        if let Ok(path) = which("yt-dlp.exe") {
            return Some(path);
        }
        if let Ok(path) = which("yt-dlp") {
            return Some(path);
        }

        None
    }

    pub fn find_ffmpeg(config: &EngineConfig) -> Option<PathBuf> {
        // 1. Config custom path
        if let Some(custom) = &config.ffmpeg_custom_path {
            if custom.exists() && custom.is_file() {
                return Some(custom.clone());
            }
        }

        // 2. LocalAppData\SnapLoad\bin\ffmpeg.exe
        if let Some(local_app_data) = std::env::var_os("LOCALAPPDATA") {
            let mut p = PathBuf::from(local_app_data);
            p.push("SnapLoad");
            p.push("bin");
            p.push("ffmpeg.exe");
            if p.exists() {
                return Some(p);
            }
        }

        // 3. Exe directory or Exe\bin
        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(parent) = exe_path.parent() {
                let candidate1 = parent.join("ffmpeg.exe");
                if candidate1.exists() {
                    return Some(candidate1);
                }
                let candidate2 = parent.join("bin").join("ffmpeg.exe");
                if candidate2.exists() {
                    return Some(candidate2);
                }
            }
        }

        // 4. System PATH
        if let Ok(path) = which("ffmpeg.exe") {
            return Some(path);
        }
        if let Ok(path) = which("ffmpeg") {
            return Some(path);
        }

        None
    }

    pub async fn check_status(config: &EngineConfig) -> DependencyStatus {
        let ytdlp_path = Self::find_ytdlp(config);
        let ffmpeg_path = Self::find_ffmpeg(config);

        let mut ytdlp_version = None;
        let ytdlp_installed = if let Some(path) = &ytdlp_path {
            match Self::probe_version(path, &["--version"]).await {
                Ok(v) => {
                    ytdlp_version = Some(v.trim().to_string());
                    true
                }
                Err(e) => {
                    Logger::warn(&format!(
                        "Found yt-dlp at {:?} but failed to probe: {}",
                        path, e
                    ));
                    false
                }
            }
        } else {
            false
        };

        let mut ffmpeg_version = None;
        let ffmpeg_installed = if let Some(path) = &ffmpeg_path {
            match Self::probe_version(path, &["-version"]).await {
                Ok(v) => {
                    let first_line = v.lines().next().unwrap_or("ffmpeg").trim().to_string();
                    ffmpeg_version = Some(first_line);
                    true
                }
                Err(e) => {
                    Logger::warn(&format!(
                        "Found ffmpeg at {:?} but failed to probe: {}",
                        path, e
                    ));
                    false
                }
            }
        } else {
            false
        };

        let ready = ytdlp_installed; // yt-dlp is required; ffmpeg is highly recommended for audio conversion

        DependencyStatus {
            ytdlp: DependencyInfo {
                name: "yt-dlp".to_string(),
                installed: ytdlp_installed,
                path: ytdlp_path.map(|p| p.to_string_lossy().to_string()),
                version: ytdlp_version,
            },
            ffmpeg: DependencyInfo {
                name: "FFmpeg".to_string(),
                installed: ffmpeg_installed,
                path: ffmpeg_path.map(|p| p.to_string_lossy().to_string()),
                version: ffmpeg_version,
            },
            ready,
        }
    }

    async fn probe_version(exe: &Path, args: &[&str]) -> Result<String, std::io::Error> {
        let mut cmd = Command::new(exe);
        cmd.args(args);
        #[cfg(windows)]
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
        // Do not inherit standard input/output to avoid blocking
        cmd.stdin(std::process::Stdio::null());
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());

        let child = cmd.spawn()?;
        let output = timeout(Duration::from_secs(4), child.wait_with_output())
            .await
            .map_err(|_| {
                std::io::Error::new(std::io::ErrorKind::TimedOut, "Command timed out")
            })??;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            Err(std::io::Error::other(format!(
                "Process exited with status {}",
                output.status
            )))
        }
    }

    pub async fn auto_download_ytdlp() -> Option<PathBuf> {
        let local_app_data = std::env::var_os("LOCALAPPDATA")?;
        let mut dest_dir = PathBuf::from(local_app_data);
        dest_dir.push("SnapLoad");
        dest_dir.push("bin");
        let _ = std::fs::create_dir_all(&dest_dir);
        let dest_file = dest_dir.join("yt-dlp.exe");

        let mut cmd = Command::new("curl.exe");
        #[cfg(windows)]
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
        cmd.args([
            "-s",
            "-L",
            "-f",
            "-o",
            dest_file.to_str().unwrap_or_default(),
            "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe",
        ]);

        if let Ok(status) = cmd.status().await {
            if status.success()
                && dest_file.exists()
                && dest_file.metadata().map(|m| m.len() > 1_000_000).unwrap_or(false)
            {
                Logger::info(&format!("Auto-downloaded yt-dlp to {:?}", dest_file));
                return Some(dest_file);
            }
        }
        None
    }
}

/// Helper to check PATH for executable
fn which(name: &str) -> Result<PathBuf, ()> {
    if let Some(paths) = std::env::var_os("PATH") {
        for path in std::env::split_paths(&paths) {
            let full_path = path.join(name);
            if full_path.is_file() {
                return Ok(full_path);
            }
        }
    }
    Err(())
}
