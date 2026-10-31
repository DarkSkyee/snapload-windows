use crate::registry::RegistryManager;
use serde::{Deserialize, Serialize};
use std::fs::{create_dir_all, remove_dir_all};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallOptions {
    pub browsers: Vec<String>,
    pub install_ytdlp: bool,
    pub install_ffmpeg: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallResult {
    pub success: bool,
    pub logs: Vec<String>,
}

pub struct SetupManager;

impl SetupManager {
    pub fn get_app_dir() -> PathBuf {
        let local_app_data = std::env::var_os("LOCALAPPDATA")
            .or_else(|| std::env::var_os("APPDATA"))
            .unwrap_or_else(|| std::ffi::OsString::from("."));
        let mut path = PathBuf::from(local_app_data);
        path.push("SnapLoad");
        path
    }

    pub fn get_bin_dir() -> PathBuf {
        let mut path = Self::get_app_dir();
        path.push("bin");
        let _ = create_dir_all(&path);
        path
    }

    pub fn get_logs_dir() -> PathBuf {
        let mut path = Self::get_app_dir();
        path.push("logs");
        let _ = create_dir_all(&path);
        path
    }

    pub async fn run_install(options: InstallOptions) -> Result<InstallResult, String> {
        let mut logs = Vec::new();
        logs.push("Initializing SnapLoad desktop environment...".to_string());

        let bin_dir = Self::get_bin_dir();
        let _logs_dir = Self::get_logs_dir();
        logs.push(format!("Created application directories: {:?}", bin_dir));

        // Locate snapload-engine.exe
        let engine_dest = bin_dir.join("snapload-engine.exe");
        let mut engine_source = None;

        // Search relative paths, sibling directories, current exe directory, and workspace targets
        let mut search_candidates = vec![
            std::env::current_exe()
                .unwrap_or_default()
                .parent()
                .unwrap_or(Path::new(""))
                .join("snapload-engine.exe"),
            std::env::current_exe()
                .unwrap_or_default()
                .parent()
                .unwrap_or(Path::new(""))
                .join("bin")
                .join("snapload-engine.exe"),
            PathBuf::from("snapload-engine.exe"),
            PathBuf::from("bin/snapload-engine.exe"),
            PathBuf::from("../engine/target/release/snapload-engine.exe"),
            PathBuf::from("engine/target/release/snapload-engine.exe"),
            PathBuf::from("../engine/target/debug/snapload-engine.exe"),
            PathBuf::from("engine/target/debug/snapload-engine.exe"),
        ];

        if let Ok(cwd) = std::env::current_dir() {
            search_candidates.push(cwd.join("engine/target/release/snapload-engine.exe"));
            search_candidates.push(cwd.join("../engine/target/release/snapload-engine.exe"));
            search_candidates.push(cwd.join("engine/target/debug/snapload-engine.exe"));
        }

        for c in &search_candidates {
            if c.exists() && c.is_file() {
                engine_source = Some(c.clone());
                break;
            }
        }

        if let Some(src) = engine_source {
            std::fs::copy(&src, &engine_dest)
                .map_err(|e| format!("Failed to copy engine binary from {:?}: {}", src, e))?;
            logs.push(format!("Deployed SnapLoad Engine to: {:?}", engine_dest));
        } else {
            // If running standalone, verify if engine_dest is already present
            if !engine_dest.exists() {
                return Err("SnapLoad engine executable not found in build directory. Please compile engine first.".to_string());
            }
            logs.push(format!(
                "Existing SnapLoad Engine verified at: {:?}",
                engine_dest
            ));
        }

        // Generate and write Native Messaging host manifests
        logs.push("Generating Native Messaging host manifests...".to_string());
        let (chrome_manifest, firefox_manifest) = RegistryManager::write_manifests(&engine_dest)
            .map_err(|e| format!("Failed to write host manifests: {}", e))?;
        logs.push(format!("Chromium Host Manifest: {:?}", chrome_manifest));
        logs.push(format!("Firefox Host Manifest: {:?}", firefox_manifest));

        // Register in Windows Registry for selected browsers
        for b_id in &options.browsers {
            let manifest_to_use = if b_id == "firefox" {
                &firefox_manifest
            } else {
                &chrome_manifest
            };

            match RegistryManager::register_browser(b_id, manifest_to_use) {
                Ok(_) => logs.push(format!("Registered Native Messaging Host for: {}", b_id)),
                Err(err) => logs.push(format!("Warning: Could not register for {}: {}", b_id, err)),
            }
        }

        // Handle yt-dlp installation if requested
        if options.install_ytdlp {
            let ytdlp_dest = bin_dir.join("yt-dlp.exe");
            if ytdlp_dest.exists() {
                logs.push(format!("yt-dlp core verified at: {:?}", ytdlp_dest));
            } else if let Some(path_src) = which("yt-dlp.exe") {
                if std::fs::copy(&path_src, &ytdlp_dest).is_ok() {
                    logs.push(format!("Copied existing yt-dlp from PATH: {:?}", path_src));
                } else {
                    logs.push(format!("yt-dlp detected on system PATH at: {:?}", path_src));
                }
            } else {
                logs.push("Downloading latest official yt-dlp release...".to_string());
                let download_res = tokio::process::Command::new("curl.exe")
                    .args([
                        "-s",
                        "-L",
                        "-f",
                        "-o",
                        ytdlp_dest.to_str().unwrap_or_default(),
                        "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe",
                    ])
                    .status()
                    .await;

                if let Ok(status) = download_res {
                    if status.success()
                        && ytdlp_dest.exists()
                        && ytdlp_dest
                            .metadata()
                            .map(|m| m.len() > 1_000_000)
                            .unwrap_or(false)
                    {
                        logs.push("yt-dlp downloaded and installed successfully!".to_string());
                    } else {
                        logs.push("Notice: Automated yt-dlp download failed. Official source: https://github.com/yt-dlp/yt-dlp/releases".to_string());
                    }
                } else {
                    logs.push("Notice: curl is unavailable to download yt-dlp. Official source: https://github.com/yt-dlp/yt-dlp/releases".to_string());
                }
            }
        }

        // Handle FFmpeg installation if requested
        if options.install_ffmpeg {
            let ffmpeg_dest = bin_dir.join("ffmpeg.exe");
            if ffmpeg_dest.exists() {
                logs.push(format!("FFmpeg core verified at: {:?}", ffmpeg_dest));
            } else if let Some(path_src) = which("ffmpeg.exe") {
                if std::fs::copy(&path_src, &ffmpeg_dest).is_ok() {
                    logs.push(format!("Copied existing FFmpeg from PATH: {:?}", path_src));
                } else {
                    logs.push(format!("FFmpeg detected on system PATH at: {:?}", path_src));
                }
            } else {
                logs.push("FFmpeg converter configured. Place ffmpeg.exe in SnapLoad\\bin or system PATH for audio remuxing.".to_string());
            }
        }

        // Initialize default download directory in User Downloads\SnapLoad
        if let Some(user_profile) = std::env::var_os("USERPROFILE") {
            let dl_dir = PathBuf::from(user_profile)
                .join("Downloads")
                .join("SnapLoad");
            let _ = create_dir_all(&dl_dir);
            logs.push(format!(
                "Default media downloads folder ready: {:?}",
                dl_dir
            ));
        }

        logs.push("SnapLoad Setup completed successfully.".to_string());

        Ok(InstallResult {
            success: true,
            logs,
        })
    }

    pub fn verify_installation() -> Vec<String> {
        let mut checks = Vec::new();
        let bin_dir = Self::get_bin_dir();
        let engine_path = bin_dir.join("snapload-engine.exe");

        if engine_path.exists() {
            checks.push(format!(
                "Engine binary verified ({} bytes)",
                engine_path.metadata().map(|m| m.len()).unwrap_or(0)
            ));
        } else {
            checks.push("Engine binary missing from %LOCALAPPDATA%\\SnapLoad\\bin".to_string());
        }

        let manifest_dir = RegistryManager::get_manifest_dir();
        if manifest_dir
            .join("com.darkskyee.snapload.chromium.json")
            .exists()
        {
            checks.push("Chromium Native Messaging manifest verified".to_string());
        }
        if manifest_dir
            .join("com.darkskyee.snapload.firefox.json")
            .exists()
        {
            checks.push("Firefox Native Messaging manifest verified".to_string());
        }

        checks
    }

    pub fn uninstall() -> Result<Vec<String>, String> {
        let mut logs = Vec::new();

        // 1. Unregister all registry keys
        let removed_keys = RegistryManager::unregister_all();
        logs.push(format!(
            "Removed {} registry registrations",
            removed_keys.len()
        ));

        // 2. Remove application directory (excluding user downloads)
        let app_dir = Self::get_app_dir();
        if app_dir.exists() {
            let _ = remove_dir_all(app_dir);
            logs.push("Cleaned up %LOCALAPPDATA%\\SnapLoad".to_string());
        }

        logs.push("SnapLoad components uninstalled. Downloaded media preserved.".to_string());
        Ok(logs)
    }
}

fn which(name: &str) -> Option<PathBuf> {
    if let Some(paths) = std::env::var_os("PATH") {
        for p in std::env::split_paths(&paths) {
            let full = p.join(name);
            if full.is_file() {
                return Some(full);
            }
        }
    }
    None
}
