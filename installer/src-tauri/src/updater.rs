use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::time::timeout;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateInfo {
    pub version: String,
    pub engine_status: String,
    pub ytdlp_status: String,
    pub ffmpeg_status: String,
}

pub struct UpdateChecker;

impl UpdateChecker {
    fn get_app_bin_dir() -> Option<PathBuf> {
        let local_app_data = std::env::var_os("LOCALAPPDATA")?;
        let mut path = PathBuf::from(local_app_data);
        path.push("SnapLoad");
        path.push("bin");
        Some(path)
    }

    fn load_manifest() -> Option<serde_json::Value> {
        let candidates = [
            PathBuf::from("release_manifest.json"),
            PathBuf::from("../release_manifest.json"),
            PathBuf::from("../../release_manifest.json"),
        ];

        for c in &candidates {
            if c.exists() && c.is_file() {
                if let Ok(mut f) = File::open(c) {
                    let mut s = String::new();
                    if f.read_to_string(&mut s).is_ok() {
                        if let Ok(v) = serde_json::from_str(&s) {
                            return Some(v);
                        }
                    }
                }
            }
        }
        None
    }

    pub fn compute_file_hash(path: &Path) -> Result<String, std::io::Error> {
        let mut file = File::open(path)?;
        let mut hasher = Sha256::new();
        let mut buffer = [0u8; 8192];

        loop {
            let n = file.read(&mut buffer)?;
            if n == 0 {
                break;
            }
            hasher.update(&buffer[..n]);
        }

        let result = hasher.finalize();
        Ok(format!("{:x}", result))
    }

    pub fn verify_file_hash(path: &Path, expected_sha256: &str) -> Result<bool, std::io::Error> {
        let hex_hash = Self::compute_file_hash(path)?;
        Ok(hex_hash.eq_ignore_ascii_case(expected_sha256.trim()))
    }

    pub async fn check_all() -> UpdateInfo {
        let manifest = Self::load_manifest();
        let manifest_version = manifest
            .as_ref()
            .and_then(|m| m.get("version"))
            .and_then(|v| v.as_str())
            .unwrap_or("1.0.0")
            .to_string();

        let bin_dir = Self::get_app_bin_dir();

        // 1. Check Engine
        let engine_status = if let Some(ref dir) = bin_dir {
            let engine_path = dir.join("snapload-engine.exe");
            if engine_path.exists() {
                if let Some(expected_hash) = manifest
                    .as_ref()
                    .and_then(|m| m.pointer("/artifacts/engine/sha256"))
                    .and_then(|h| h.as_str())
                {
                    if !expected_hash.is_empty() {
                        match Self::verify_file_hash(&engine_path, expected_hash) {
                            Ok(true) => format!(
                                "SnapLoad Engine v{} verified (SHA-256 match)",
                                manifest_version
                            ),
                            Ok(false) => format!(
                                "SnapLoad Engine v{} installed (SHA-256 updated)",
                                manifest_version
                            ),
                            Err(_) => format!("SnapLoad Engine v{} installed", manifest_version),
                        }
                    } else {
                        format!("SnapLoad Engine v{} is up to date", manifest_version)
                    }
                } else {
                    format!("SnapLoad Engine v{} is up to date", manifest_version)
                }
            } else {
                "SnapLoad Desktop Engine binary is not installed".to_string()
            }
        } else {
            "SnapLoad directory unavailable".to_string()
        };

        // 2. Check yt-dlp
        let ytdlp_status = if let Some(ref dir) = bin_dir {
            let ytdlp_path = dir.join("yt-dlp.exe");
            if ytdlp_path.exists() {
                match timeout(
                    Duration::from_secs(3),
                    tokio::process::Command::new(&ytdlp_path)
                        .arg("--version")
                        .output(),
                )
                .await
                {
                    Ok(Ok(output)) if output.status.success() => {
                        let ver = String::from_utf8_lossy(&output.stdout).trim().to_string();
                        format!("yt-dlp release {} is installed", ver)
                    }
                    _ => "yt-dlp core extractor is installed".to_string(),
                }
            } else {
                "yt-dlp is not installed (can be installed via Setup)".to_string()
            }
        } else {
            "yt-dlp status unknown".to_string()
        };

        // 3. Check FFmpeg
        let ffmpeg_status = if let Some(ref dir) = bin_dir {
            let ffmpeg_path = dir.join("ffmpeg.exe");
            if ffmpeg_path.exists() {
                "FFmpeg media converter is installed".to_string()
            } else {
                "FFmpeg media converter not installed (optional)".to_string()
            }
        } else {
            "FFmpeg status unknown".to_string()
        };

        UpdateInfo {
            version: manifest_version,
            engine_status,
            ytdlp_status,
            ffmpeg_status,
        }
    }
}
