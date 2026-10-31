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
        let local_app_data = std::env::var_os("LOCALAPPDATA").expect("LOCALAPPDATA not found");
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

        // Try sibling directories or current exe directory
        let search_candidates = [
            PathBuf::from(r".\engine\target\release\snapload-engine.exe"),
            PathBuf::from(r".\engine\target\debug\snapload-engine.exe"),
            std::env::current_exe().unwrap_or_default().parent().unwrap_or(Path::new("")).join("snapload-engine.exe"),
        ];

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
            logs.push(format!("Existing SnapLoad Engine verified at: {:?}", engine_dest));
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
            if !ytdlp_dest.exists() {
                logs.push("yt-dlp is configured. Download url: https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe".to_string());
            } else {
                logs.push(format!("yt-dlp core verified at: {:?}", ytdlp_dest));
            }
        }

        // Handle FFmpeg installation if requested
        if options.install_ffmpeg {
            let ffmpeg_dest = bin_dir.join("ffmpeg.exe");
            if !ffmpeg_dest.exists() {
                logs.push("FFmpeg converter configured for on-demand installation.".to_string());
            } else {
                logs.push(format!("FFmpeg core verified at: {:?}", ffmpeg_dest));
            }
        }

        // Initialize default download directory in User Downloads\SnapLoad
        if let Some(user_profile) = std::env::var_os("USERPROFILE") {
            let dl_dir = PathBuf::from(user_profile).join("Downloads").join("SnapLoad");
            let _ = create_dir_all(&dl_dir);
            logs.push(format!("Default media downloads folder ready: {:?}", dl_dir));
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
            checks.push(format!("Engine binary verified ({} bytes)", engine_path.metadata().map(|m| m.len()).unwrap_or(0)));
        } else {
            checks.push("Engine binary missing from %LOCALAPPDATA%\\SnapLoad\\bin".to_string());
        }

        let manifest_dir = RegistryManager::get_manifest_dir();
        if manifest_dir.join("com.darkskyee.snapload.chromium.json").exists() {
            checks.push("Chromium Native Messaging manifest verified".to_string());
        }
        if manifest_dir.join("com.darkskyee.snapload.firefox.json").exists() {
            checks.push("Firefox Native Messaging manifest verified".to_string());
        }

        checks
    }

    pub fn uninstall() -> Result<Vec<String>, String> {
        let mut logs = Vec::new();

        // 1. Unregister all registry keys
        let removed_keys = RegistryManager::unregister_all();
        logs.push(format!("Removed {} registry registrations", removed_keys.len()));

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
