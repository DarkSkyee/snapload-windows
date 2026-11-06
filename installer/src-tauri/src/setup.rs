use crate::browser::BrowserDetector;
use crate::registry::{compute_path_extension_ids, RegistryManager};
use serde::{Deserialize, Serialize};
use std::fs::create_dir_all;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::time::timeout;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoInstallResult {
    pub success: bool,
    pub browser_id: String,
    pub browser_name: String,
    pub exe_path: String,
    pub profile_path: Option<String>,
    pub message: String,
    pub logs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticItem {
    pub name: String,
    pub status: String, // "ok", "warn", "error"
    pub message: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticResult {
    pub all_passed: bool,
    pub total: usize,
    pub passed: usize,
    pub items: Vec<DiagnosticItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionPaths {
    pub app_dir: String,
    pub chromium_dir: String,
    pub firefox_xpi: String,
    pub downloads_dir: String,
}

pub struct SetupManager;

const EMBEDDED_ENGINE: &[u8] = include_bytes!("../../../engine/target/release/snapload-engine.exe");
const EMBEDDED_CHROME_EXT: &[u8] =
    include_bytes!("../../../extension/dist/snapload-extension-chromium.zip");
const EMBEDDED_FIREFOX_EXT: &[u8] =
    include_bytes!("../../../extension/dist/snapload-extension-firefox.zip");

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
        path
    }

    pub fn get_logs_dir() -> PathBuf {
        let mut path = Self::get_app_dir();
        path.push("logs");
        path
    }

    pub fn get_extensions_dir() -> PathBuf {
        let mut path = Self::get_app_dir();
        path.push("extensions");
        path
    }

    pub fn get_chromium_ext_dir() -> PathBuf {
        let mut path = Self::get_extensions_dir();
        path.push("chromium");
        path
    }

    pub fn get_firefox_ext_dir() -> PathBuf {
        let mut path = Self::get_extensions_dir();
        path.push("firefox");
        path
    }

    pub fn get_manifest_dir() -> PathBuf {
        let mut path = Self::get_app_dir();
        path.push("manifests");
        path
    }

    pub fn ensure_directories_at(base_dir: &Path) -> Result<Vec<PathBuf>, std::io::Error> {
        let bin = base_dir.join("bin");
        let logs = base_dir.join("logs");
        let ext = base_dir.join("extensions");
        let chrom = ext.join("chromium");
        let ff = ext.join("firefox");
        let manifests = base_dir.join("manifests");

        let dirs = vec![
            base_dir.to_path_buf(),
            bin,
            logs,
            ext,
            chrom,
            ff,
            manifests,
        ];

        for d in &dirs {
            create_dir_all(d)?;
        }

        Ok(dirs)
    }

    pub fn ensure_app_directories() -> Result<Vec<PathBuf>, std::io::Error> {
        Self::ensure_directories_at(&Self::get_app_dir())
    }

    pub fn get_paths() -> ExtensionPaths {
        let app_dir = Self::get_app_dir().to_string_lossy().to_string();
        let chromium_dir = Self::get_chromium_ext_dir().to_string_lossy().to_string();
        let firefox_xpi = Self::get_firefox_ext_dir()
            .join("snapload-extension-firefox.xpi")
            .to_string_lossy()
            .to_string();
        let downloads_dir = std::env::var_os("USERPROFILE")
            .map(|u| PathBuf::from(u).join("Downloads"))
            .unwrap_or_else(|| PathBuf::from("."))
            .to_string_lossy()
            .to_string();

        ExtensionPaths {
            app_dir,
            chromium_dir,
            firefox_xpi,
            downloads_dir,
        }
    }

    pub fn cleanup_desktop_shortcuts() -> Vec<String> {
        Self::clean_all_browser_shortcuts()
    }

    pub fn build_isolated_launch_args(
        browser_id: &str,
        ext_path: &Path,
    ) -> Result<(PathBuf, Vec<String>), String> {
        let path_str = ext_path.to_string_lossy().trim().to_string();
        if path_str.is_empty() || path_str.eq_ignore_ascii_case("dummy") || path_str.contains("dummy") {
            return Err("Invalid extension path: cannot be empty or 'dummy'".to_string());
        }

        if !ext_path.is_absolute() {
            return Err(format!("Extension path must be absolute: {:?}", ext_path));
        }

        let manifest_path = ext_path.join("manifest.json");
        if !manifest_path.exists() {
            return Err(format!(
                "manifest.json does not exist in extension path: {:?}",
                ext_path
            ));
        }

        let temp_profile = std::env::temp_dir().join(format!("snapload_isolated_profile_{}", browser_id));
        let _ = std::fs::create_dir_all(&temp_profile);

        let clean_profile_str = temp_profile.to_string_lossy();
        let clean_ext_str = ext_path.to_string_lossy();

        let args = vec![
            format!("--user-data-dir=\"{}\"", clean_profile_str),
            format!("--load-extension=\"{}\"", clean_ext_str),
            "--disable-features=DisableLoadExtensionCommandLineSwitch".to_string(),
            "--no-first-run".to_string(),
            "--no-default-browser-check".to_string(),
        ];

        Ok((temp_profile, args))
    }

    pub fn launch_browser_isolated(
        browser_id: &str,
        exe_path: &str,
        ext_path: &Path,
    ) -> Result<String, String> {
        let clean_exe = BrowserDetector::sanitize_exe_path(exe_path);
        if clean_exe.is_empty() {
            return Err(format!("Invalid executable path for browser: {}", browser_id));
        }

        if browser_id == "firefox" {
            let xpi_path = if ext_path.join("snapload-extension-firefox.xpi").exists() {
                ext_path.join("snapload-extension-firefox.xpi")
            } else if ext_path.is_file() && ext_path.extension().map_or(false, |e| e == "xpi") {
                ext_path.to_path_buf()
            } else {
                Self::get_firefox_ext_dir().join("snapload-extension-firefox.xpi")
            };

            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                const CREATE_NEW_PROCESS_GROUP: u32 = 0x00000200;
                let mut cmd = std::process::Command::new(&clean_exe);
                cmd.arg(xpi_path.to_string_lossy().to_string());
                cmd.creation_flags(CREATE_NEW_PROCESS_GROUP);
                cmd.spawn().map_err(|e| format!("Failed to launch Firefox: {}", e))?;
                return Ok("Opened extension in Firefox".to_string());
            }
            #[cfg(not(windows))]
            {
                return Err("Only windows is supported".to_string());
            }
        }

        let (temp_profile, args) = Self::build_isolated_launch_args(browser_id, ext_path)?;

        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NEW_PROCESS_GROUP: u32 = 0x00000200;
            let mut cmd = std::process::Command::new(&clean_exe);
            for arg in &args {
                cmd.raw_arg(arg);
            }
            cmd.creation_flags(CREATE_NEW_PROCESS_GROUP);
            cmd.spawn().map_err(|e| format!("Failed to launch {}: {}", browser_id, e))?;
            Ok(format!(
                "Launched {} in isolated profile ({:?}) with extension",
                browser_id, temp_profile
            ))
        }
        #[cfg(not(windows))]
        {
            Err("Only windows is supported".to_string())
        }
    }

    pub fn deploy_to_browser_profiles(chrome_unpacked_dir: &Path) -> Vec<String> {
        let mut logs = Vec::new();
        if !chrome_unpacked_dir.join("manifest.json").exists() {
            if let Ok(dir) = Self::ensure_chromium_ext_deployed() {
                logs.push(format!("Deployed Chromium extension with manifest.json to: {:?}", dir));
            } else {
                logs.push("Warning: could not deploy unpacked Chromium extension".to_string());
            }
        } else {
            logs.push(format!("Verified unpacked Chromium extension at: {:?}", chrome_unpacked_dir));
        }

        // Configure enterprise policies, external extensions, and profile preferences for all detected Chromium browsers
        for b_id in ["chrome", "opera", "operagx", "edge", "brave"] {
            if let Some(exe) = BrowserDetector::find_browser_exe(b_id) {
                let clean_exe = BrowserDetector::sanitize_exe_path(&exe);
                if !clean_exe.is_empty() {
                    let _ = RegistryManager::register_chromium_extension(b_id, chrome_unpacked_dir);
                    RegistryManager::register_all_chromium_policies(b_id);
                    let profiles = BrowserDetector::find_browser_profiles(b_id);
                    for prof in &profiles {
                        Self::inject_extension_to_chromium_profile(prof, "ikkidkbojpedpeaomipgioiigmjlpflg", chrome_unpacked_dir);
                    }
                }
            }
        }

        // Purge any corrupted shortcuts and remove orphaned snapload shortcuts across system
        let cleaned_sc = Self::clean_all_browser_shortcuts();
        logs.extend(cleaned_sc);
        logs.push("Configured Chromium native registry entries, policies, and sanitized shortcuts".to_string());

        logs
    }

    pub async fn run_install(options: InstallOptions) -> Result<InstallResult, String> {
        let mut logs = Vec::new();
        logs.push("Initializing SnapLoad desktop environment...".to_string());

        let cleaned_sc = Self::cleanup_desktop_shortcuts();
        logs.extend(cleaned_sc);

        // Ensure all application target directories (bin, extensions, logs, manifest) exist before deploying any components
        Self::ensure_app_directories()
            .map_err(|e| format!("Failed to create application directories: {}", e))?;

        let bin_dir = Self::get_bin_dir();
        let _logs_dir = Self::get_logs_dir();
        logs.push(format!("Created application directories: {:?}", bin_dir));

        // 1. Locate snapload-engine.exe
        let engine_dest = bin_dir.join("snapload-engine.exe");
        let mut engine_source = None;

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
            create_dir_all(&bin_dir)
                .map_err(|e| format!("Failed to create bin directory {:?}: {}", bin_dir, e))?;
            std::fs::copy(&src, &engine_dest)
                .map_err(|e| format!("Failed to copy engine binary from {:?}: {}", src, e))?;
            logs.push(format!("Deployed SnapLoad Engine to: {:?}", engine_dest));
        } else if !EMBEDDED_ENGINE.is_empty() {
            create_dir_all(&bin_dir)
                .map_err(|e| format!("Failed to create bin directory {:?}: {}", bin_dir, e))?;
            std::fs::write(&engine_dest, EMBEDDED_ENGINE)
                .map_err(|e| format!("Failed to deploy embedded engine: {}", e))?;
            logs.push(format!(
                "Extracted embedded SnapLoad Engine to: {:?}",
                engine_dest
            ));
        } else if engine_dest.exists() {
            logs.push(format!(
                "Existing SnapLoad Engine verified at: {:?}",
                engine_dest
            ));
        } else {
            return Err("SnapLoad engine executable not found.".to_string());
        }

        // 2. Deploy Chromium extension (unpacked directory + zip archive)
        let chrome_unpacked_dir = Self::get_chromium_ext_dir();
        let chrome_zip_dest = Self::get_extensions_dir().join("snapload-extension-chromium.zip");
        create_dir_all(Self::get_extensions_dir())
            .map_err(|e| format!("Failed to create extensions directory: {}", e))?;
        create_dir_all(&chrome_unpacked_dir)
            .map_err(|e| format!("Failed to create chromium extension directory: {}", e))?;

        let mut chrome_extracted = false;
        if !EMBEDDED_CHROME_EXT.is_empty() {
            let _ = std::fs::write(&chrome_zip_dest, EMBEDDED_CHROME_EXT);
            if let Ok(_) = unpack_zip_bytes(EMBEDDED_CHROME_EXT, &chrome_unpacked_dir) {
                chrome_extracted = true;
                logs.push(format!(
                    "Unpacked Chromium extension directly to: {:?}",
                    chrome_unpacked_dir
                ));
            }
        }

        if !chrome_extracted {
            // Check workspace fallback
            let workspace_chrome_dirs = [
                PathBuf::from("extension/dist/chrome"),
                PathBuf::from("../extension/dist/chrome"),
                PathBuf::from("../../extension/dist/chrome"),
            ];
            for w in &workspace_chrome_dirs {
                if w.exists() && w.join("manifest.json").exists() {
                    let _ = copy_dir_recursive(w, &chrome_unpacked_dir);
                    logs.push(format!("Copied extension files to: {:?}", chrome_unpacked_dir));
                    break;
                }
            }
        }

        // 2b. Automatically deploy extension directly to browser profile directories
        let profile_deploy_logs = Self::deploy_to_browser_profiles(&chrome_unpacked_dir);
        for l in profile_deploy_logs {
            logs.push(l);
        }

        // 3. Deploy Firefox extension (XPI package + unpacked)
        let firefox_dir = Self::get_firefox_ext_dir();
        let firefox_xpi_dest = firefox_dir.join("snapload-extension-firefox.xpi");
        let firefox_zip_dest = Self::get_extensions_dir().join("snapload-extension-firefox.zip");
        create_dir_all(Self::get_extensions_dir())
            .map_err(|e| format!("Failed to create extensions directory: {}", e))?;
        create_dir_all(&firefox_dir)
            .map_err(|e| format!("Failed to create firefox extension directory: {}", e))?;

        if !EMBEDDED_FIREFOX_EXT.is_empty() {
            let _ = std::fs::write(&firefox_zip_dest, EMBEDDED_FIREFOX_EXT);
            let _ = std::fs::write(&firefox_xpi_dest, EMBEDDED_FIREFOX_EXT);
            let _ = unpack_zip_bytes(EMBEDDED_FIREFOX_EXT, &firefox_dir.join("unpacked"));
            logs.push(format!("Deployed Firefox extension XPI to: {:?}", firefox_xpi_dest));
        } else {
            let workspace_ff_dirs = [
                PathBuf::from("extension/dist/firefox"),
                PathBuf::from("../extension/dist/firefox"),
                PathBuf::from("../../extension/dist/firefox"),
            ];
            for w in &workspace_ff_dirs {
                if w.exists() && w.join("manifest.json").exists() {
                    let _ = copy_dir_recursive(w, &firefox_dir.join("unpacked"));
                    break;
                }
            }
        }

        // 4. Generate and write Native Messaging host manifests
        logs.push("Generating Native Messaging host manifests...".to_string());
        create_dir_all(Self::get_manifest_dir())
            .map_err(|e| format!("Failed to create manifests directory: {}", e))?;
        let (chrome_manifest, firefox_manifest) = RegistryManager::write_manifests(&engine_dest)
            .map_err(|e| format!("Failed to write host manifests: {}", e))?;
        logs.push(format!("Chromium Host Manifest: {:?}", chrome_manifest));
        logs.push(format!("Firefox Host Manifest: {:?}", firefox_manifest));

        // 5. Register in Windows Registry for selected browsers and all Chromium browsers
        RegistryManager::register_all_chromium_hosts(&chrome_manifest);
        logs.push("Configured Native Messaging Host for all Chromium browsers (Chrome, Edge, Brave, Opera, Opera GX)".to_string());

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

            // Register browser extension hooks
            if b_id == "firefox" {
                if let Err(e) = RegistryManager::register_firefox_extension(&firefox_xpi_dest) {
                    logs.push(format!("Firefox extension registry note: {}", e));
                } else {
                    logs.push("Registered Firefox extension in Windows Registry".to_string());
                }
            } else {
                if let Err(e) = RegistryManager::register_chromium_extension(b_id, &chrome_unpacked_dir) {
                    logs.push(format!("Chromium extension registry note: {}", e));
                } else {
                    logs.push(format!("Configured external extension entry for {}", b_id));
                }
            }
        }

        // 6. Handle yt-dlp installation if requested
        if options.install_ytdlp {
            create_dir_all(&bin_dir)
                .map_err(|e| format!("Failed to create bin directory {:?}: {}", bin_dir, e))?;
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
                let mut cmd = tokio::process::Command::new("curl.exe");
                #[cfg(windows)]
                cmd.creation_flags(CREATE_NO_WINDOW);
                cmd.args([
                    "-s",
                    "-L",
                    "-f",
                    "-o",
                    ytdlp_dest.to_str().unwrap_or_default(),
                    "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe",
                ]);

                let download_res = cmd.status().await;

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

        // 7. Handle FFmpeg installation if requested
        if options.install_ffmpeg {
            Self::install_ffmpeg_binary(&bin_dir, &mut logs);
        }

        // 8. Initialize default download directory in User Downloads
        if let Some(user_profile) = std::env::var_os("USERPROFILE") {
            let dl_dir = PathBuf::from(user_profile)
                .join("Downloads");
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

    pub fn find_existing_ffmpeg() -> Option<PathBuf> {
        // 1. Check existing PATH
        if let Some(path_src) = which("ffmpeg.exe") {
            return Some(path_src);
        }

        // 2. Check WinGet Links or Packages
        if let Some(local_app_data) = std::env::var_os("LOCALAPPDATA") {
            let winget_ffmpeg = PathBuf::from(&local_app_data)
                .join("Microsoft")
                .join("WinGet")
                .join("Links")
                .join("ffmpeg.exe");
            if winget_ffmpeg.exists() {
                return Some(winget_ffmpeg);
            }

            let pkg_dir = PathBuf::from(&local_app_data)
                .join("Microsoft")
                .join("WinGet")
                .join("Packages");
            if let Ok(entries) = std::fs::read_dir(&pkg_dir) {
                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if name.contains("FFmpeg") || name.contains("ffmpeg") || name.contains("Gyan") {
                        let candidate1 = entry.path().join("ffmpeg-9.0.1-essentials_build").join("bin").join("ffmpeg.exe");
                        if candidate1.exists() {
                            return Some(candidate1);
                        }
                        if let Ok(sub_entries) = std::fs::read_dir(entry.path()) {
                            for sub in sub_entries.flatten() {
                                let sub_bin = sub.path().join("bin").join("ffmpeg.exe");
                                if sub_bin.exists() {
                                    return Some(sub_bin);
                                }
                            }
                        }
                    }
                }
            }
        }
        None
    }

    pub fn install_ffmpeg_binary(bin_dir: &Path, logs: &mut Vec<String>) {
        let _ = create_dir_all(bin_dir);
        let ffmpeg_dest = bin_dir.join("ffmpeg.exe");
        if ffmpeg_dest.exists() && ffmpeg_dest.metadata().map(|m| m.len() > 1_000_000).unwrap_or(false) {
            logs.push(format!("FFmpeg core verified at: {:?}", ffmpeg_dest));
            return;
        }

        if let Some(existing) = Self::find_existing_ffmpeg() {
            if std::fs::copy(&existing, &ffmpeg_dest).is_ok() {
                logs.push(format!("Copied existing FFmpeg to {:?}: {:?}", ffmpeg_dest, existing));
                return;
            } else {
                logs.push(format!("FFmpeg detected at: {:?}", existing));
                return;
            }
        }

        // Try WinGet package installation
        logs.push("Attempting FFmpeg installation via WinGet...".to_string());
        let mut winget_cmd = std::process::Command::new("winget.exe");
        #[cfg(windows)]
        winget_cmd.creation_flags(CREATE_NO_WINDOW);
        winget_cmd.args([
            "install",
            "--id",
            "Gyan.FFmpeg.Essentials",
            "-e",
            "--accept-source-agreements",
            "--accept-package-agreements",
            "--silent",
        ]);

        if let Ok(status) = winget_cmd.status() {
            if status.success() {
                if let Some(existing) = Self::find_existing_ffmpeg() {
                    let _ = std::fs::copy(&existing, &ffmpeg_dest);
                    if ffmpeg_dest.exists() {
                        logs.push("FFmpeg installed and verified via WinGet!".to_string());
                        return;
                    }
                }
            }
        }

        // 4. Direct download fallback
        logs.push("Downloading official FFmpeg release archive...".to_string());
        let temp_zip = bin_dir.join("ffmpeg_temp.zip");
        let mut curl_cmd = std::process::Command::new("curl.exe");
        #[cfg(windows)]
        curl_cmd.creation_flags(CREATE_NO_WINDOW);
        curl_cmd.args([
            "-s",
            "-L",
            "-f",
            "-o",
            temp_zip.to_str().unwrap_or_default(),
            "https://github.com/GyanD/codexffmpeg/releases/download/9.0.1/ffmpeg-9.0.1-essentials_build.zip",
        ]);

        if let Ok(status) = curl_cmd.status() {
            if status.success() && temp_zip.exists() {
                let mut tar_cmd = std::process::Command::new("tar.exe");
                #[cfg(windows)]
                tar_cmd.creation_flags(CREATE_NO_WINDOW);
                tar_cmd.args([
                    "-xf",
                    temp_zip.to_str().unwrap_or_default(),
                    "-C",
                    bin_dir.to_str().unwrap_or_default(),
                    "--strip-components",
                    "2",
                    "*ffmpeg.exe",
                ]);
                let _ = tar_cmd.status();
                let _ = std::fs::remove_file(&temp_zip);

                if ffmpeg_dest.exists() && ffmpeg_dest.metadata().map(|m| m.len() > 1_000_000).unwrap_or(false) {
                    logs.push("FFmpeg downloaded and installed successfully!".to_string());
                    return;
                }
            }
        }
        let _ = std::fs::remove_file(&temp_zip);
        logs.push("Notice: FFmpeg automated setup did not complete. SnapLoad will automatically fallback to single-stream formats or configure FFmpeg on first run.".to_string());
    }

    pub async fn verify_installation() -> DiagnosticResult {
        let mut items = Vec::new();
        let bin_dir = Self::get_bin_dir();
        let engine_path = bin_dir.join("snapload-engine.exe");

        // 1. Engine binary check
        if engine_path.exists() {
            let size = engine_path.metadata().map(|m| m.len()).unwrap_or(0);
            if size > 10_000 {
                items.push(DiagnosticItem {
                    name: "SnapLoad Desktop Engine".to_string(),
                    status: "ok".to_string(),
                    message: format!("Binary active ({:.2} MB)", size as f64 / 1_048_576.0),
                    detail: engine_path.to_string_lossy().to_string(),
                });
            } else {
                items.push(DiagnosticItem {
                    name: "SnapLoad Desktop Engine".to_string(),
                    status: "error".to_string(),
                    message: "Binary file exists but is corrupted or empty".to_string(),
                    detail: engine_path.to_string_lossy().to_string(),
                });
            }
        } else {
            items.push(DiagnosticItem {
                name: "SnapLoad Desktop Engine".to_string(),
                status: "error".to_string(),
                message: "snapload-engine.exe missing from bin directory".to_string(),
                detail: format!("Expected at: {:?}", engine_path),
            });
        }

        // 2. Chromium Native Messaging Manifest
        let manifest_dir = RegistryManager::get_manifest_dir();
        let chrome_manifest = manifest_dir.join("com.darkskyee.snapload.chromium.json");
        if chrome_manifest.exists() {
            items.push(DiagnosticItem {
                name: "Chromium Native Manifest".to_string(),
                status: "ok".to_string(),
                message: "Registered (Chrome, Edge, Brave, Opera, Opera GX)".to_string(),
                detail: chrome_manifest.to_string_lossy().to_string(),
            });
        } else {
            items.push(DiagnosticItem {
                name: "Chromium Native Manifest".to_string(),
                status: "error".to_string(),
                message: "Host manifest missing".to_string(),
                detail: "com.darkskyee.snapload.chromium.json not found".to_string(),
            });
        }

        // 3. Firefox Native Messaging Manifest
        let firefox_manifest = manifest_dir.join("com.darkskyee.snapload.firefox.json");
        if firefox_manifest.exists() {
            items.push(DiagnosticItem {
                name: "Firefox Native Manifest".to_string(),
                status: "ok".to_string(),
                message: "Registered (Mozilla Firefox)".to_string(),
                detail: firefox_manifest.to_string_lossy().to_string(),
            });
        } else {
            items.push(DiagnosticItem {
                name: "Firefox Native Manifest".to_string(),
                status: "warn".to_string(),
                message: "Firefox host manifest not installed".to_string(),
                detail: "Optional if only using Chromium browsers".to_string(),
            });
        }

        // 4. Windows Registry Registrations
        let browsers = ["chrome", "edge", "brave", "opera", "operagx", "firefox"];
        let mut registered = Vec::new();
        for b in browsers {
            let (is_reg, _) = RegistryManager::check_native_host_registration(b);
            if is_reg {
                registered.push(b);
            }
        }

        if registered.is_empty() && engine_path.exists() {
            // Auto-repair missing host manifests and registry keys if engine is deployed
            if let Ok((chrome_man, firefox_man)) = RegistryManager::write_manifests(&engine_path) {
                for b in browsers {
                    let man = if b == "firefox" { &firefox_man } else { &chrome_man };
                    if RegistryManager::register_browser(b, man).is_ok() {
                        registered.push(b);
                    }
                }
            }
        }

        if !registered.is_empty() {
            items.push(DiagnosticItem {
                name: "Windows Registry Host Keys".to_string(),
                status: "ok".to_string(),
                message: format!("Configured for: {}", registered.join(", ")),
                detail: "HKCU\\Software\\...\\NativeMessagingHosts\\com.darkskyee.snapload".to_string(),
            });
        } else {
            items.push(DiagnosticItem {
                name: "Windows Registry Host Keys".to_string(),
                status: "warn".to_string(),
                message: "No active browser NativeMessagingHosts entries".to_string(),
                detail: "Run installation to register registry keys".to_string(),
            });
        }

        // 5. Unpacked Chromium Extension
        let chrom_ext_dir = Self::get_chromium_ext_dir();
        if chrom_ext_dir.join("manifest.json").exists() {
            items.push(DiagnosticItem {
                name: "Chromium Extension Deployment".to_string(),
                status: "ok".to_string(),
                message: "Unpacked extension folder ready to load (manifest verified)".to_string(),
                detail: chrom_ext_dir.to_string_lossy().to_string(),
            });
        } else {
            items.push(DiagnosticItem {
                name: "Chromium Extension Deployment".to_string(),
                status: "warn".to_string(),
                message: "Unpacked extension not found in SnapLoad directory".to_string(),
                detail: format!("Expected at: {:?}", chrom_ext_dir),
            });
        }

        // 6. Firefox Extension Package
        let ff_xpi = Self::get_firefox_ext_dir().join("snapload-extension-firefox.xpi");
        let ff_reg = RegistryManager::check_firefox_extension_registered();
        if ff_xpi.exists() {
            items.push(DiagnosticItem {
                name: "Firefox Extension Package".to_string(),
                status: "ok".to_string(),
                message: format!(
                    "XPI package verified (Registry hook: {})",
                    if ff_reg { "Active" } else { "Pending restart" }
                ),
                detail: ff_xpi.to_string_lossy().to_string(),
            });
        } else {
            items.push(DiagnosticItem {
                name: "Firefox Extension Package".to_string(),
                status: "warn".to_string(),
                message: "Firefox extension package not deployed".to_string(),
                detail: "Optional if using Chrome/Edge".to_string(),
            });
        }

        // 7. yt-dlp Core Extractor
        let ytdlp_path = bin_dir.join("yt-dlp.exe");
        if ytdlp_path.exists() {
            let mut cmd = tokio::process::Command::new(&ytdlp_path);
            #[cfg(windows)]
            cmd.creation_flags(CREATE_NO_WINDOW);
            cmd.arg("--version");
            let ver_out = timeout(Duration::from_secs(2), cmd.output()).await;
            if let Ok(Ok(out)) = ver_out {
                if out.status.success() {
                    let v = String::from_utf8_lossy(&out.stdout).trim().to_string();
                    items.push(DiagnosticItem {
                        name: "yt-dlp Core Extractor".to_string(),
                        status: "ok".to_string(),
                        message: format!("Operational (release {})", v),
                        detail: ytdlp_path.to_string_lossy().to_string(),
                    });
                } else {
                    items.push(DiagnosticItem {
                        name: "yt-dlp Core Extractor".to_string(),
                        status: "ok".to_string(),
                        message: "Executable present in bin".to_string(),
                        detail: ytdlp_path.to_string_lossy().to_string(),
                    });
                }
            } else {
                items.push(DiagnosticItem {
                    name: "yt-dlp Core Extractor".to_string(),
                    status: "ok".to_string(),
                    message: "Binary verified".to_string(),
                    detail: ytdlp_path.to_string_lossy().to_string(),
                });
            }
        } else if let Some(p) = which("yt-dlp.exe") {
            items.push(DiagnosticItem {
                name: "yt-dlp Core Extractor".to_string(),
                status: "ok".to_string(),
                message: "Detected on Windows system PATH".to_string(),
                detail: p.to_string_lossy().to_string(),
            });
        } else {
            items.push(DiagnosticItem {
                name: "yt-dlp Core Extractor".to_string(),
                status: "warn".to_string(),
                message: "Not found (required for video extractions)".to_string(),
                detail: "Check 'yt-dlp' component during installation".to_string(),
            });
        }

        // 8. FFmpeg Media Converter
        let ffmpeg_path = bin_dir.join("ffmpeg.exe");
        if ffmpeg_path.exists() {
            items.push(DiagnosticItem {
                name: "FFmpeg Media Converter".to_string(),
                status: "ok".to_string(),
                message: "Converter engine ready for audio remuxing".to_string(),
                detail: ffmpeg_path.to_string_lossy().to_string(),
            });
        } else if let Some(p) = which("ffmpeg.exe") {
            items.push(DiagnosticItem {
                name: "FFmpeg Media Converter".to_string(),
                status: "ok".to_string(),
                message: "Found on system PATH".to_string(),
                detail: p.to_string_lossy().to_string(),
            });
        } else {
            items.push(DiagnosticItem {
                name: "FFmpeg Media Converter".to_string(),
                status: "warn".to_string(),
                message: "Optional (recommended for MP3/FLAC conversions)".to_string(),
                detail: "Place ffmpeg.exe in SnapLoad\\bin to enable".to_string(),
            });
        }

        // 9. Live Native Messaging IPC Ping Test
        let (ping_ok, ping_msg) = test_native_messaging_ping(&engine_path).await;
        if ping_ok {
            items.push(DiagnosticItem {
                name: "Live Native Messaging IPC".to_string(),
                status: "ok".to_string(),
                message: "Engine ping test passed (JSON stdio active)".to_string(),
                detail: ping_msg,
            });
        } else {
            items.push(DiagnosticItem {
                name: "Live Native Messaging IPC".to_string(),
                status: "warn".to_string(),
                message: "Live ping check warning".to_string(),
                detail: ping_msg,
            });
        }

        let total = items.len();
        let passed = items.iter().filter(|i| i.status == "ok").count();
        let any_err = items.iter().any(|i| i.status == "error");

        DiagnosticResult {
            all_passed: !any_err,
            total,
            passed,
            items,
        }
    }

    pub fn force_remove_file(path: &Path) -> bool {
        if !path.exists() {
            return false;
        }
        if let Ok(meta) = std::fs::metadata(path) {
            let mut perms = meta.permissions();
            if perms.readonly() {
                perms.set_readonly(false);
                let _ = std::fs::set_permissions(path, perms);
            }
        }
        std::fs::remove_file(path).is_ok()
    }

    pub fn force_remove_dir_all(dir: &Path) -> bool {
        if !dir.exists() {
            return true;
        }
        if let Ok(meta) = std::fs::metadata(dir) {
            let mut perms = meta.permissions();
            if perms.readonly() {
                perms.set_readonly(false);
                let _ = std::fs::set_permissions(dir, perms);
            }
        }
        // Recursively reset readonly flag on everything and delete children
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_dir() {
                    let _ = Self::force_remove_dir_all(&p);
                } else {
                    let _ = Self::force_remove_file(&p);
                }
            }
        }
        let _ = std::fs::remove_dir_all(dir);
        if !dir.exists() {
            return true;
        }
        #[cfg(windows)]
        {
            let mut cmd = std::process::Command::new("cmd.exe");
            cmd.args(["/C", "rd", "/S", "/Q", &format!("\"{}\"", dir.to_string_lossy())]);
            cmd.creation_flags(CREATE_NO_WINDOW);
            let _ = cmd.output();
        }
        !dir.exists()
    }

    pub fn inject_extension_to_chromium_profile(
        profile_path: &Path,
        ext_id: &str,
        unpacked_path: &Path,
    ) {
        let mut prefs_files = vec![
            profile_path.join("Preferences"),
            profile_path.join("Default").join("Preferences"),
        ];
        if let Some(parent) = profile_path.parent() {
            prefs_files.push(parent.join("Preferences"));
        }

        let path_str = unpacked_path.to_string_lossy().to_string();

        for prefs_file in &prefs_files {
            if !prefs_file.exists() {
                continue;
            }
            if let Ok(content) = std::fs::read_to_string(prefs_file) {
                if let Ok(mut json) = serde_json::from_str::<serde_json::Value>(&content) {
                    let mut modified = false;

                    if json.get("extensions").is_none() {
                        json["extensions"] = serde_json::json!({});
                    }

                    if let Some(extensions) = json.get_mut("extensions").and_then(|e| e.as_object_mut()) {
                        let is_opera = profile_path.to_string_lossy().to_lowercase().contains("opera") || extensions.contains_key("opsettings");

                        let ext_entry = serde_json::json!({
                            "active_permissions": {
                                "api": ["contextMenus", "nativeMessaging", "notifications", "storage", "tabs"],
                                "explicit_host": ["*://*.youtu.be/*", "*://*.youtube.com/*"],
                                "manifest_permissions": [],
                                "scriptable_host": ["*://*.youtube.com/*", "https://*.youtube.com/*"]
                            },
                            "commands": {},
                            "content_settings": [],
                            "creation_flags": 38,
                            "events": [],
                            "from_bookmark": false,
                            "from_webstore": false,
                            "granted_permissions": {
                                "api": ["contextMenus", "nativeMessaging", "notifications", "storage", "tabs"],
                                "explicit_host": ["*://*.youtu.be/*", "*://*.youtube.com/*"],
                                "manifest_permissions": [],
                                "scriptable_host": ["*://*.youtube.com/*", "https://*.youtube.com/*"]
                            },
                            "incognito_content_settings": [],
                            "incognito_preferences": {},
                            "initial_keybindings": [],
                            "location": 4,
                            "new_allow_file_access": true,
                            "path": path_str,
                            "preferences": {},
                            "regular_only_preferences": {},
                            "service_worker_registration_info": {},
                            "state": 1,
                            "was_installed_by_default": false,
                            "was_installed_by_oem": false
                        });

                        // Standard settings (Chrome, Edge, Brave)
                        if !extensions.contains_key("settings") {
                            extensions.insert("settings".to_string(), serde_json::json!({}));
                        }
                        if let Some(settings) = extensions.get_mut("settings").and_then(|s| s.as_object_mut()) {
                            settings.insert(ext_id.to_string(), ext_entry.clone());
                            modified = true;
                        }

                        // Opera and Opera GX use opsettings
                        if is_opera {
                            if !extensions.contains_key("opsettings") {
                                extensions.insert("opsettings".to_string(), serde_json::json!({}));
                            }
                            if let Some(opsettings) = extensions.get_mut("opsettings").and_then(|s| s.as_object_mut()) {
                                opsettings.insert(ext_id.to_string(), ext_entry);
                                modified = true;
                            }
                        }

                        // Ensure developer mode is active in ui
                        if !extensions.contains_key("ui") {
                            extensions.insert("ui".to_string(), serde_json::json!({}));
                        }
                        if let Some(ui) = extensions.get_mut("ui").and_then(|u| u.as_object_mut()) {
                            ui.insert("developer_mode".to_string(), serde_json::json!(true));
                            modified = true;
                        }

                        if !extensions.contains_key("pinned_extensions") {
                            extensions.insert("pinned_extensions".to_string(), serde_json::json!([]));
                        }
                        if let Some(pinned) = extensions.get_mut("pinned_extensions").and_then(|p| p.as_array_mut()) {
                            if !pinned.iter().any(|v| v.as_str() == Some(ext_id)) {
                                pinned.push(serde_json::Value::String(ext_id.to_string()));
                                modified = true;
                            }
                        }

                        if let Some(toolbar) = extensions.get_mut("toolbar").and_then(|t| t.as_array_mut()) {
                            if !toolbar.iter().any(|v| v.as_str() == Some(ext_id)) {
                                toolbar.push(serde_json::Value::String(ext_id.to_string()));
                                modified = true;
                            }
                        }
                    }

                    if modified {
                        if let Ok(serialized) = serde_json::to_string_pretty(&json) {
                            let _ = std::fs::write(prefs_file, serialized);
                        }
                    }
                }
            }
        }
    }

    pub fn clean_all_browser_shortcuts() -> Vec<String> {
        let mut logs = Vec::new();
        #[cfg(windows)]
        {
            let ps_script = r#"
$ws = New-Object -ComObject WScript.Shell;
$dirs = @(
    "$env:USERPROFILE\Desktop",
    "$env:USERPROFILE\Pulpit",
    "C:\Users\Public\Desktop",
    "C:\Users\Public\Pulpit",
    [Environment]::GetFolderPath("Desktop"),
    [Environment]::GetFolderPath("CommonDesktopDirectory"),
    [Environment]::GetFolderPath("Programs"),
    [Environment]::GetFolderPath("CommonPrograms"),
    [Environment]::GetFolderPath("StartMenu"),
    [Environment]::GetFolderPath("CommonStartMenu"),
    "$env:USERPROFILE\OneDrive\Desktop",
    "$env:USERPROFILE\OneDrive - Personal\Desktop",
    "$env:USERPROFILE\OneDrive\Pulpit",
    "$env:USERPROFILE\OneDrive - Personal\Pulpit",
    "$env:APPDATA\Microsoft\Windows\Start Menu",
    "$env:APPDATA\Microsoft\Windows\Start Menu\Programs",
    "$env:ProgramData\Microsoft\Windows\Start Menu",
    "$env:ProgramData\Microsoft\Windows\Start Menu\Programs",
    "$env:APPDATA\Microsoft\Internet Explorer\Quick Launch",
    "$env:APPDATA\Microsoft\Internet Explorer\Quick Launch\User Pinned\StartMenu",
    "$env:APPDATA\Microsoft\Internet Explorer\Quick Launch\User Pinned\TaskBar"
);
try {
    $usf = Get-ItemProperty -Path "HKCU:\Software\Microsoft\Windows\CurrentVersion\Explorer\User Shell Folders" -ErrorAction SilentlyContinue;
    if ($usf) {
        foreach ($prop in @('Desktop', 'Programs', 'Start Menu', 'Common Desktop', 'Common Programs', 'Common Start Menu')) {
            $val = $usf.$prop;
            if ($val) {
                $exp = [Environment]::ExpandEnvironmentVariables($val);
                if ($exp -and (Test-Path $exp)) { $dirs += $exp; }
            }
        }
    }
} catch {}
$dirs = $dirs | Where-Object { $_ -and (Test-Path $_) } | Select-Object -Unique;
foreach ($d in $dirs) {
    Get-ChildItem -Path $d -Recurse -Filter "*.lnk" -Force -ErrorAction SilentlyContinue | ForEach-Object {
        try {
            $lnkPath = $_.FullName;
            $name = $_.Name.ToLower();
            if ($name -like "*snapload*") {
                Remove-Item -Force -LiteralPath $lnkPath -ErrorAction SilentlyContinue;
                Write-Output "DELETED:$lnkPath";
            } else {
                $sc = $ws.CreateShortcut($lnkPath);
                $targetPath = ('' + $sc.TargetPath).ToLower();
                if ($targetPath -like "*snapload*" -or $targetPath -like "*non_existent*" -or $targetPath -like "*\dummy*" -or $targetPath -like "*/dummy*" -or $targetPath -eq "dummy") {
                    Remove-Item -Force -LiteralPath $lnkPath -ErrorAction SilentlyContinue;
                    Write-Output "DELETED:$lnkPath";
                } else {
                    $curArgs = '' + $sc.Arguments;
                    $desc = '' + $sc.Description;
                    $needsSave = $false;
                    if ($curArgs -match "(?i)load-extension|DisableLoadExtensionCommandLineSwitch|snapload|youtube|dummy") {
                        $cleanArgs = $curArgs -replace '(?i)--load-extension\s*=\s*(?:\\?"[^"]*\\?"|\\?''[^'']*\\?''|\S+)', '' `
                                              -replace '(?i)--disable-features=DisableLoadExtensionCommandLineSwitch', '' `
                                              -replace '(?i)https?://[^\s]*youtube[^\s]*', '' `
                                              -replace '(?i)(?:https?://)?(?:www\.)?youtube\.com[^\s]*', '' `
                                              -replace '(?i)(?:https?://)?youtu\.be[^\s]*', '' `
                                              -replace '(?i)(?:\\?"|\b)dummy(?:\\?"|\b)', '';
                        $cleanArgs = [regex]::Replace($cleanArgs, '\s{2,}', ' ').Trim();
                        if ($cleanArgs -ne $curArgs) {
                            if ($_.IsReadOnly) { $_.IsReadOnly = $false; }
                            $sc.Arguments = $cleanArgs;
                            $needsSave = $true;
                        }
                    }
                    if ($desc -like "*snapload*") {
                        if ($_.IsReadOnly) { $_.IsReadOnly = $false; }
                        $sc.Description = "";
                        $needsSave = $true;
                    }
                    if ($needsSave) {
                        try {
                            $sc.Save();
                            Write-Output "CLEANED:$lnkPath";
                        } catch {}
                    }
                }
            }
        } catch {}
    }
}
"#;
            let mut cmd = std::process::Command::new("powershell.exe");
            cmd.creation_flags(CREATE_NO_WINDOW);
            cmd.args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-Command", ps_script]);
            if let Ok(output) = cmd.output() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                for line in stdout.lines() {
                    let line = line.trim();
                    if !line.is_empty() {
                        logs.push(format!("Shortcut: {}", line));
                    }
                }
            }
        }
        logs
    }

    pub fn clean_extension_from_chromium_profile(profile_path: &Path, ext_id: &str) {
        let mut target_ids = vec![ext_id.to_string()];

        // Include all path-derived unpacked extension IDs
        if let Some(local_app_data) = std::env::var_os("LOCALAPPDATA").or_else(|| std::env::var_os("APPDATA")) {
            let chrom_dir = PathBuf::from(local_app_data).join("SnapLoad").join("extensions").join("chromium");
            target_ids.extend(compute_path_extension_ids(&chrom_dir));
        }

        let mut prefs_files = vec![
            profile_path.join("Preferences"),
            profile_path.join("Secure Preferences"),
            profile_path.join("Preferences.backup"),
            profile_path.join("Secure Preferences.backup"),
            profile_path.join("Default").join("Preferences"),
            profile_path.join("Default").join("Secure Preferences"),
            profile_path.join("Default").join("Preferences.backup"),
            profile_path.join("Default").join("Secure Preferences.backup"),
        ];
        if let Some(parent) = profile_path.parent() {
            prefs_files.push(parent.join("Preferences"));
            prefs_files.push(parent.join("Secure Preferences"));
            prefs_files.push(parent.join("Preferences.backup"));
            prefs_files.push(parent.join("Secure Preferences.backup"));
        }
        for prefs_file in &prefs_files {
            if prefs_file.exists() {
                if let Ok(content) = std::fs::read_to_string(prefs_file) {
                    if let Ok(mut json) = serde_json::from_str::<serde_json::Value>(&content) {
                        let mut modified = false;

                        // Identify all SnapLoad extension IDs from settings and opsettings
                        let mut snapload_ids = target_ids.clone();
                        for key in &["/extensions/settings", "/extensions/opsettings"] {
                            if let Some(settings) = json.pointer(key).and_then(|s| s.as_object()) {
                                for (k, v) in settings {
                                    let path_match = v.get("path")
                                        .and_then(|p| p.as_str())
                                        .map(|p| p.to_lowercase().contains("snapload"))
                                        .unwrap_or(false);
                                    let name_match = v.pointer("/manifest/name")
                                        .and_then(|n| n.as_str())
                                        .map(|n| n.to_lowercase().contains("snapload"))
                                        .unwrap_or(false);
                                    if path_match || name_match {
                                        snapload_ids.push(k.clone());
                                    }
                                }
                            }
                        }
                        snapload_ids.sort();
                        snapload_ids.dedup();

                        // Remove from standard settings (Chrome, Edge, Brave)
                        if let Some(settings) = json.pointer_mut("/extensions/settings").and_then(|s| s.as_object_mut()) {
                            for id in &snapload_ids {
                                if settings.remove(id).is_some() {
                                    modified = true;
                                }
                            }
                        }

                        // Remove from opsettings (Opera / Opera GX)
                        if let Some(opsettings) = json.pointer_mut("/extensions/opsettings").and_then(|s| s.as_object_mut()) {
                            for id in &snapload_ids {
                                if opsettings.remove(id).is_some() {
                                    modified = true;
                                }
                            }
                        }

                        // Remove from protection.macs (Chromium integrity validation)
                        let mac_pointers = [
                            "/protection/macs/extensions/settings",
                            "/protection/macs/extensions/settings_encrypted_hash",
                            "/protection/macs/extensions/opsettings",
                            "/protection/macs/extensions/opsettings_encrypted_hash",
                        ];
                        for ptr in &mac_pointers {
                            if let Some(mac_obj) = json.pointer_mut(ptr).and_then(|m| m.as_object_mut()) {
                                for id in &snapload_ids {
                                    if mac_obj.remove(id).is_some() {
                                        modified = true;
                                    }
                                }
                            }
                        }

                        // Remove from pinned_extensions
                        if let Some(pinned) = json.pointer_mut("/extensions/pinned_extensions").and_then(|p| p.as_array_mut()) {
                            let orig_len = pinned.len();
                            pinned.retain(|v| {
                                v.as_str().map(|s| !snapload_ids.contains(&s.to_string())).unwrap_or(true)
                            });
                            if pinned.len() != orig_len {
                                modified = true;
                            }
                        }

                        // Remove from toolbar
                        if let Some(toolbar) = json.pointer_mut("/extensions/toolbar").and_then(|t| t.as_array_mut()) {
                            let orig_len = toolbar.len();
                            toolbar.retain(|v| {
                                v.as_str().map(|s| !snapload_ids.contains(&s.to_string())).unwrap_or(true)
                            });
                            if toolbar.len() != orig_len {
                                modified = true;
                            }
                        }

                        if modified {
                            if let Ok(meta) = std::fs::metadata(prefs_file) {
                                let mut perms = meta.permissions();
                                if perms.readonly() {
                                    perms.set_readonly(false);
                                    let _ = std::fs::set_permissions(prefs_file, perms);
                                }
                            }
                            if let Ok(serialized) = serde_json::to_string_pretty(&json) {
                                for attempt in 0..5 {
                                    if std::fs::write(prefs_file, &serialized).is_ok() {
                                        break;
                                    }
                                    std::thread::sleep(std::time::Duration::from_millis(50 * (attempt + 1)));
                                }
                            }
                        }

                        // Remove all extension subdirectories for all identified snapload_ids
                        let subdirs = [
                            "Extensions",
                            "Local Extension Settings",
                            "Extension State",
                            "Extension Rules",
                            "Extension Scripts",
                            "Sync Extension Settings",
                            "DNR Extension Rules",
                            "Managed Extension Settings",
                        ];
                        for sub in subdirs {
                            for id in &snapload_ids {
                                let target = profile_path.join(sub).join(id);
                                if target.exists() {
                                    let _ = Self::force_remove_dir_all(&target);
                                }
                            }
                        }

                        // Also clean IndexedDB directories matching snapload or extension id
                        let idb_dir = profile_path.join("IndexedDB");
                        if idb_dir.exists() {
                            if let Ok(entries) = std::fs::read_dir(&idb_dir) {
                                for e in entries.flatten() {
                                    let name = e.file_name().to_string_lossy().to_string().to_lowercase();
                                    for id in &snapload_ids {
                                        if name.contains(&id.to_lowercase()) || name.contains("snapload") {
                                            let _ = Self::force_remove_dir_all(&e.path());
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn uninstall() -> Result<Vec<String>, String> {
        let mut logs = Vec::new();

        // 1. Terminate running SnapLoad engine, utilities, and browser processes to unlock files & profiles
        #[cfg(windows)]
        {
            let procs = [
                "snapload-engine.exe",
                "ffmpeg.exe",
                "ffprobe.exe",
                "ffplay.exe",
                "yt-dlp.exe",
                "opera.exe",
                "opera_crashreporter.exe",
                "opera_autoupdate.exe",
                "chrome.exe",
                "msedge.exe",
                "brave.exe",
                "firefox.exe",
            ];
            for p in procs {
                let mut cmd = std::process::Command::new("taskkill.exe");
                cmd.args(["/F", "/IM", p]);
                cmd.creation_flags(CREATE_NO_WINDOW);
                let _ = cmd.output();
            }
            // Wait for browser & engine processes to fully terminate and release disk locks
            for _ in 0..10 {
                let mut any_running = false;
                for b in &["chrome", "edge", "brave", "opera", "operagx", "firefox"] {
                    if BrowserDetector::is_browser_process_running(b) {
                        any_running = true;
                        break;
                    }
                }
                if !any_running {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(150));
            }
        }

        // 2. Clean all browser shortcuts (strip --load-extension flags, delete SnapLoad shortcuts)
        let cleaned_shortcuts = Self::clean_all_browser_shortcuts();
        logs.extend(cleaned_shortcuts);

        // 3. Unregister all registry keys
        let removed_keys = RegistryManager::unregister_all();
        logs.push(format!(
            "Removed {} registry registrations",
            removed_keys.len()
        ));

        // 4. Deep clean extension from all browser profiles
        let ext_id = "ikkidkbojpedpeaomipgioiigmjlpflg";
        let browsers = ["chrome", "edge", "brave", "opera", "operagx", "firefox"];
        for b_id in browsers {
            let profs = BrowserDetector::find_browser_profiles(b_id);
            for p in profs {
                if b_id == "firefox" {
                    let ff_ext = p.join("extensions").join("snapload@darkskyee.github.io.xpi");
                    if ff_ext.exists() {
                        let _ = Self::force_remove_file(&ff_ext);
                        logs.push(format!("Removed Firefox extension XPI: {:?}", ff_ext));
                    }
                    let ext_json = p.join("extensions.json");
                    if ext_json.exists() {
                        if let Ok(c) = std::fs::read_to_string(&ext_json) {
                            if let Ok(mut j) = serde_json::from_str::<serde_json::Value>(&c) {
                                if let Some(addons) = j.pointer_mut("/addons") {
                                    if let Some(arr) = addons.as_array_mut() {
                                        let orig_len = arr.len();
                                        arr.retain(|a| {
                                            let id_match = a.get("id").and_then(|v| v.as_str()) == Some("snapload@darkskyee.github.io");
                                            let path_match = a.get("path").and_then(|v| v.as_str()).map(|p| p.to_lowercase().contains("snapload")).unwrap_or(false);
                                            !id_match && !path_match
                                        });
                                        if arr.len() != orig_len {
                                            if let Ok(serialized) = serde_json::to_string_pretty(&j) {
                                                let _ = std::fs::write(&ext_json, serialized);
                                                logs.push("Cleaned Firefox extensions.json".to_string());
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                } else {
                    Self::clean_extension_from_chromium_profile(&p, ext_id);
                    logs.push(format!("Cleaned {} profile extension settings: {:?}", b_id, p));
                }
            }
        }

        // Explicitly check Opera & Opera GX profile directories (both with & without Default)
        if let Some(appdata) = std::env::var_os("APPDATA") {
            let opera_base = PathBuf::from(&appdata).join("Opera Software");
            for sub in &["Opera Stable", "Opera GX Stable", "Opera Developer", "Opera Next"] {
                let op = opera_base.join(sub);
                if op.exists() {
                    Self::clean_extension_from_chromium_profile(&op, ext_id);
                    let def = op.join("Default");
                    if def.exists() {
                        Self::clean_extension_from_chromium_profile(&def, ext_id);
                    }
                }
            }
        }

        // 5. Uninstall FFmpeg (WinGet package + residual links and packages)
        #[cfg(windows)]
        {
            let winget_path = which("winget.exe")
                .or_else(|| {
                    std::env::var_os("LOCALAPPDATA")
                        .map(|l| PathBuf::from(l).join("Microsoft").join("WindowsApps").join("winget.exe"))
                })
                .unwrap_or_else(|| PathBuf::from("winget.exe"));

            for pkg_id in &["Gyan.FFmpeg", "Gyan.FFmpeg.Shared"] {
                let mut winget_cmd = std::process::Command::new(&winget_path);
                winget_cmd.args(["uninstall", "--id", pkg_id, "--silent", "--accept-source-agreements"]);
                winget_cmd.creation_flags(CREATE_NO_WINDOW);
                let _ = winget_cmd.output();
            }
        }

        if let Some(local) = std::env::var_os("LOCALAPPDATA") {
            let winget_links = PathBuf::from(&local).join("Microsoft").join("WinGet").join("Links");
            for exe in &["ffmpeg.exe", "ffprobe.exe", "ffplay.exe"] {
                let f = winget_links.join(exe);
                if f.exists() {
                    let _ = Self::force_remove_file(&f);
                    logs.push(format!("Removed WinGet FFmpeg link: {:?}", f));
                }
            }

            let winget_pkgs = PathBuf::from(&local).join("Microsoft").join("WinGet").join("Packages");
            if winget_pkgs.exists() {
                if let Ok(entries) = std::fs::read_dir(&winget_pkgs) {
                    for entry in entries.flatten() {
                        let p = entry.path();
                        if let Some(name) = p.file_name().and_then(|n| n.to_str()) {
                            if name.to_lowercase().contains("ffmpeg") || name.to_lowercase().contains("gyan") {
                                let _ = Self::force_remove_dir_all(&p);
                                logs.push(format!("Removed WinGet FFmpeg package: {:?}", p));
                            }
                        }
                    }
                }
            }
        }

        // 6. Force remove all binaries: ffmpeg.exe, ffprobe.exe, yt-dlp.exe, snapload-engine.exe
        let mut candidate_bin_dirs = Vec::new();
        if let Some(local) = std::env::var_os("LOCALAPPDATA") {
            candidate_bin_dirs.push(PathBuf::from(&local).join("SnapLoad").join("bin"));
            candidate_bin_dirs.push(PathBuf::from(&local).join("SnapLoad"));
        }
        if let Some(appdata) = std::env::var_os("APPDATA") {
            candidate_bin_dirs.push(PathBuf::from(&appdata).join("SnapLoad").join("bin"));
            candidate_bin_dirs.push(PathBuf::from(&appdata).join("SnapLoad"));
        }

        for b_dir in &candidate_bin_dirs {
            for exe in &["ffmpeg.exe", "ffprobe.exe", "yt-dlp.exe", "snapload-engine.exe"] {
                let f = b_dir.join(exe);
                if f.exists() {
                    if Self::force_remove_file(&f) {
                        logs.push(format!("Removed binary: {:?}", f));
                    }
                }
            }
        }

        // 7. Completely remove SnapLoad application directories
        let mut app_dirs_to_clean = Vec::new();
        if let Some(local) = std::env::var_os("LOCALAPPDATA") {
            app_dirs_to_clean.push(PathBuf::from(&local).join("SnapLoad"));
        }
        if let Some(appdata) = std::env::var_os("APPDATA") {
            app_dirs_to_clean.push(PathBuf::from(&appdata).join("SnapLoad"));
        }

        for d in app_dirs_to_clean {
            if d.exists() {
                if Self::force_remove_dir_all(&d) {
                    logs.push(format!("Cleaned application directory: {:?}", d));
                } else {
                    logs.push(format!("Notice: Partial removal of {:?}", d));
                }
            }
        }

        // 8. Clean up any leftover shortcuts from Desktop and Start Menu
        let cleaned_sc = Self::cleanup_desktop_shortcuts();
        logs.extend(cleaned_sc);

        let mut sm_dirs = Vec::new();
        if let Some(sm) = std::env::var_os("APPDATA").map(|u| PathBuf::from(u).join(r"Microsoft\Windows\Start Menu\Programs")) {
            sm_dirs.push(sm);
        }
        if let Some(pd) = std::env::var_os("ProgramData").map(|u| PathBuf::from(u).join(r"Microsoft\Windows\Start Menu\Programs")) {
            sm_dirs.push(pd);
        }

        for sm in sm_dirs {
            if let Ok(entries) = std::fs::read_dir(&sm) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        let lower = name.to_lowercase();
                        if lower.ends_with(".lnk") && (lower.starts_with("snapload") || lower.contains("snapload")) {
                            let _ = Self::force_remove_file(&path);
                            logs.push(format!("Removed Start Menu shortcut: {:?}", path));
                        }
                    }
                }
            }
        }

        logs.push("SnapLoad components, browser extensions, and dependencies uninstalled. Downloaded media preserved.".to_string());
        Ok(logs)
    }

    pub fn ensure_engine_deployed() -> Result<PathBuf, String> {
        let bin_dir = Self::get_bin_dir();
        let engine_dest = bin_dir.join("snapload-engine.exe");

        if engine_dest.exists() && engine_dest.metadata().map(|m| m.len() > 1000).unwrap_or(false) {
            return Ok(engine_dest);
        }

        let mut engine_source = None;
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
            create_dir_all(&bin_dir)
                .map_err(|e| format!("Failed to create bin directory {:?}: {}", bin_dir, e))?;
            std::fs::copy(&src, &engine_dest)
                .map_err(|e| format!("Failed to copy engine binary from {:?}: {}", src, e))?;
            Ok(engine_dest)
        } else if !EMBEDDED_ENGINE.is_empty() {
            create_dir_all(&bin_dir)
                .map_err(|e| format!("Failed to create bin directory {:?}: {}", bin_dir, e))?;
            std::fs::write(&engine_dest, EMBEDDED_ENGINE)
                .map_err(|e| format!("Failed to deploy embedded engine: {}", e))?;
            Ok(engine_dest)
        } else if engine_dest.exists() {
            Ok(engine_dest)
        } else {
            Err("SnapLoad engine executable not found.".to_string())
        }
    }

    pub fn ensure_chromium_ext_deployed() -> Result<PathBuf, String> {
        let chrome_unpacked_dir = Self::get_chromium_ext_dir();
        create_dir_all(Self::get_extensions_dir())
            .map_err(|e| format!("Failed to create extensions directory: {}", e))?;
        create_dir_all(&chrome_unpacked_dir)
            .map_err(|e| format!("Failed to create chromium extension directory: {}", e))?;
        if chrome_unpacked_dir.join("manifest.json").exists() {
            return Ok(chrome_unpacked_dir);
        }

        let chrome_zip_dest = Self::get_extensions_dir().join("snapload-extension-chromium.zip");
        if !EMBEDDED_CHROME_EXT.is_empty() {
            let _ = std::fs::write(&chrome_zip_dest, EMBEDDED_CHROME_EXT);
            if unpack_zip_bytes(EMBEDDED_CHROME_EXT, &chrome_unpacked_dir).is_ok()
                && chrome_unpacked_dir.join("manifest.json").exists()
            {
                return Ok(chrome_unpacked_dir);
            }
        }

        // Try workspace directories
        let mut workspace_chrome_dirs = vec![
            PathBuf::from("extension/dist/chrome"),
            PathBuf::from("../extension/dist/chrome"),
            PathBuf::from("../../extension/dist/chrome"),
        ];

        if let Ok(cwd) = std::env::current_dir() {
            workspace_chrome_dirs.push(cwd.join("extension/dist/chrome"));
            workspace_chrome_dirs.push(cwd.join("../extension/dist/chrome"));
        }

        if let Ok(exe) = std::env::current_exe() {
            if let Some(parent) = exe.parent() {
                workspace_chrome_dirs.push(parent.join("extension/dist/chrome"));
                workspace_chrome_dirs.push(parent.join("../../extension/dist/chrome"));
            }
        }

        for w in &workspace_chrome_dirs {
            if w.exists() && w.join("manifest.json").exists() {
                let _ = copy_dir_recursive(w, &chrome_unpacked_dir);
                if chrome_unpacked_dir.join("manifest.json").exists() {
                    return Ok(chrome_unpacked_dir);
                }
            }
        }

        // Try workspace zip files
        let mut workspace_zips = vec![
            PathBuf::from("extension/dist/snapload-extension-chromium.zip"),
            PathBuf::from("../extension/dist/snapload-extension-chromium.zip"),
            PathBuf::from("../../extension/dist/snapload-extension-chromium.zip"),
        ];
        if let Ok(cwd) = std::env::current_dir() {
            workspace_zips.push(cwd.join("extension/dist/snapload-extension-chromium.zip"));
            workspace_zips.push(cwd.join("../extension/dist/snapload-extension-chromium.zip"));
        }

        for z in &workspace_zips {
            if z.exists() {
                if let Ok(bytes) = std::fs::read(z) {
                    let _ = create_dir_all(Self::get_extensions_dir());
                    let _ = std::fs::write(&chrome_zip_dest, &bytes);
                    if unpack_zip_bytes(&bytes, &chrome_unpacked_dir).is_ok()
                        && chrome_unpacked_dir.join("manifest.json").exists()
                    {
                        return Ok(chrome_unpacked_dir);
                    }
                }
            }
        }

        if chrome_unpacked_dir.join("manifest.json").exists() {
            Ok(chrome_unpacked_dir)
        } else {
            Err("Chromium extension files could not be located or unpacked.".to_string())
        }
    }

    pub fn ensure_firefox_ext_deployed() -> Result<PathBuf, String> {
        let firefox_dir = Self::get_firefox_ext_dir();
        create_dir_all(Self::get_extensions_dir())
            .map_err(|e| format!("Failed to create extensions directory: {}", e))?;
        create_dir_all(&firefox_dir)
            .map_err(|e| format!("Failed to create firefox extension directory: {}", e))?;
        let firefox_xpi_dest = firefox_dir.join("snapload-extension-firefox.xpi");
        if firefox_xpi_dest.exists() && firefox_xpi_dest.metadata().map(|m| m.len() > 100).unwrap_or(false) {
            return Ok(firefox_xpi_dest);
        }

        let firefox_zip_dest = Self::get_extensions_dir().join("snapload-extension-firefox.zip");
        if !EMBEDDED_FIREFOX_EXT.is_empty() {
            let _ = std::fs::write(&firefox_zip_dest, EMBEDDED_FIREFOX_EXT);
            let _ = std::fs::write(&firefox_xpi_dest, EMBEDDED_FIREFOX_EXT);
            let _ = unpack_zip_bytes(EMBEDDED_FIREFOX_EXT, &firefox_dir.join("unpacked"));
            return Ok(firefox_xpi_dest);
        }

        let workspace_ff_zips = [
            PathBuf::from("extension/dist/snapload-extension-firefox.zip"),
            PathBuf::from("../extension/dist/snapload-extension-firefox.zip"),
            PathBuf::from("../../extension/dist/snapload-extension-firefox.zip"),
        ];
        for w in &workspace_ff_zips {
            if w.exists() {
                let _ = std::fs::copy(w, &firefox_xpi_dest);
                return Ok(firefox_xpi_dest);
            }
        }

        if firefox_xpi_dest.exists() {
            Ok(firefox_xpi_dest)
        } else {
            Err("Firefox extension package could not be located.".to_string())
        }
    }

    pub fn auto_install_for_browser(
        browser_id: &str,
        restart_if_running: bool,
    ) -> Result<AutoInstallResult, String> {
        let (name, exe_opt) = match browser_id {
            "chrome" => ("Google Chrome", BrowserDetector::find_chrome()),
            "edge" => ("Microsoft Edge", BrowserDetector::find_edge()),
            "brave" => ("Brave Browser", BrowserDetector::find_brave()),
            "opera" => ("Opera", BrowserDetector::find_opera()),
            "operagx" => ("Opera GX", BrowserDetector::find_opera_gx()),
            "firefox" => ("Mozilla Firefox", BrowserDetector::find_firefox()),
            _ => return Err(format!("Unsupported browser: {}", browser_id)),
        };

        let exe_path = match exe_opt {
            Some(p) => BrowserDetector::sanitize_exe_path(&p),
            None => return Err(format!(
                "Executable for {} was not found on this computer. Please ensure the browser is installed.",
                name
            )),
        };

        let mut logs = Vec::new();
        logs.push(format!("Detected browser executable: {}", exe_path));

        // Ensure all application target directories exist before auto install
        Self::ensure_app_directories()
            .map_err(|e| format!("Failed to create application directories: {}", e))?;

        // 1. Ensure engine is deployed
        let engine_dest = Self::ensure_engine_deployed()?;
        logs.push(format!("SnapLoad Engine verified: {:?}", engine_dest));

        // 1b. Ensure yt-dlp and FFmpeg are deployed in SnapLoad\bin
        let bin_dir = Self::get_bin_dir();
        let ytdlp_dest = bin_dir.join("yt-dlp.exe");
        if ytdlp_dest.exists() {
            logs.push(format!("yt-dlp core verified at: {:?}", ytdlp_dest));
        } else if let Some(path_src) = which("yt-dlp.exe") {
            if std::fs::copy(&path_src, &ytdlp_dest).is_ok() {
                logs.push(format!("Copied yt-dlp from PATH: {:?}", path_src));
            }
        } else {
            logs.push("Downloading latest official yt-dlp release...".to_string());
            let mut cmd = std::process::Command::new("curl.exe");
            #[cfg(windows)]
            cmd.creation_flags(CREATE_NO_WINDOW);
            cmd.args([
                "-s",
                "-L",
                "-f",
                "-o",
                ytdlp_dest.to_str().unwrap_or_default(),
                "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe",
            ]);
            let _ = cmd.status();
            if ytdlp_dest.exists() && ytdlp_dest.metadata().map(|m| m.len() > 1_000_000).unwrap_or(false) {
                logs.push("yt-dlp core extractor deployed successfully!".to_string());
            } else {
                logs.push("Notice: yt-dlp will be downloaded automatically on first run.".to_string());
            }
        }

        Self::install_ffmpeg_binary(&bin_dir, &mut logs);

        // Initialize User Downloads directory
        if let Some(user_profile) = std::env::var_os("USERPROFILE") {
            let dl_dir = PathBuf::from(user_profile).join("Downloads");
            let _ = create_dir_all(&dl_dir);
            logs.push(format!("Downloads folder verified: {:?}", dl_dir));
        }

        // 2. Write Native Messaging Host manifests
        let (chrome_manifest, firefox_manifest) = RegistryManager::write_manifests(&engine_dest)
            .map_err(|e| format!("Failed to write host manifests: {}", e))?;

        let manifest_to_use = if browser_id == "firefox" {
            &firefox_manifest
        } else {
            &chrome_manifest
        };

        // 3. Register Native Messaging Host
        RegistryManager::register_browser(browser_id, manifest_to_use)?;
        logs.push(format!("Registered Native Messaging Host for {}", name));

        // Also register for other Chromium browsers to guarantee interoperability
        if browser_id != "firefox" {
            for other in &["chrome", "edge", "brave", "opera", "operagx"] {
                if *other != browser_id {
                    let _ = RegistryManager::register_browser(other, &chrome_manifest);
                }
            }
        }

        // 4. Find profile directories
        let profile_dirs = BrowserDetector::find_browser_profiles(browser_id);
        let primary_profile = BrowserDetector::get_browser_display_profile(browser_id);
        if let Some(ref prof) = primary_profile {
            logs.push(format!("Detected browser profile: {}", prof));
        }

        // 5. Check if browser is running and restart if requested
        let was_running = BrowserDetector::is_browser_process_running(browser_id);
        if was_running {
            if restart_if_running {
                logs.push(format!(
                    "Browser {} is currently running — closing processes to load extension cleanly into session...",
                    name
                ));
                let _ = BrowserDetector::close_browser_processes(browser_id);
                // Poll until all browser processes have terminated (up to 2.5 seconds)
                for _ in 0..12 {
                    if !BrowserDetector::is_browser_process_running(browser_id) {
                        break;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(200));
                }
            } else {
                logs.push(format!(
                    "Notice: {} is running. Extension will take full effect after restarting the browser.",
                    name
                ));
            }
        }

        // 6. Deploy extension and launch browser
        let ext_to_copy_to_clip: String;
        if browser_id == "firefox" {
            let firefox_xpi = Self::ensure_firefox_ext_deployed()?;
            ext_to_copy_to_clip = firefox_xpi.to_string_lossy().to_string();
            logs.push(format!("Firefox extension XPI package: {:?}", firefox_xpi));

            // Deploy XPI to profile extensions folders
            for p in &profile_dirs {
                let target_ext_dir = p.join("extensions");
                let _ = create_dir_all(&target_ext_dir);
                let target_file = target_ext_dir.join("snapload@darkskyee.github.io.xpi");
                if firefox_xpi.exists() {
                    let _ = std::fs::copy(&firefox_xpi, &target_file);
                    logs.push(format!("Deployed .xpi file to Firefox profile: {:?}", target_file));
                }
            }

            // Register in Registry
            let _ = RegistryManager::register_firefox_extension(&firefox_xpi);
            logs.push("Registered Firefox extension in Windows Registry".to_string());

            // Launch Firefox with XPI path to prompt installation
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                const CREATE_NEW_PROCESS_GROUP: u32 = 0x00000200;
                let mut cmd = std::process::Command::new(&exe_path);
                cmd.arg(firefox_xpi.to_string_lossy().to_string());
                cmd.creation_flags(CREATE_NEW_PROCESS_GROUP);
                cmd.spawn().map_err(|e| format!("Failed to launch Firefox: {}", e))?;
                logs.push("Launched Firefox with extension prompt".to_string());
            }
        } else {
            let chrome_unpacked = Self::ensure_chromium_ext_deployed()?;
            ext_to_copy_to_clip = chrome_unpacked.to_string_lossy().to_string();
            logs.push(format!("Chromium extension folder: {:?}", chrome_unpacked));

            // Register external extension in Registry
            let _ = RegistryManager::register_chromium_extension(browser_id, &chrome_unpacked);
            logs.push(format!("Registered extension in Windows Registry for {}", name));

            // Configure Enterprise Policies (Sources, DevMode & Allowlist)
            RegistryManager::register_all_chromium_policies(browser_id);
            logs.push(format!("Configured Extension Policies for {}", name));

            // Inject into all detected profile Preferences directly
            for prof in &profile_dirs {
                Self::inject_extension_to_chromium_profile(prof, "ikkidkbojpedpeaomipgioiigmjlpflg", &chrome_unpacked);
                logs.push(format!("Configured permanent extension profile preferences for: {:?}", prof));
            }

            // Clean up any legacy or corrupted shortcuts (NEVER modify or inject shortcuts)
            let cleaned_sc = Self::clean_all_browser_shortcuts();
            logs.extend(cleaned_sc);

            // Launch browser cleanly to its extensions page (never append --load-extension to user profile shortcuts)
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                const CREATE_NEW_PROCESS_GROUP: u32 = 0x00000200;
                let mut cmd = std::process::Command::new(&exe_path);
                let ext_url = match browser_id {
                    "opera" | "operagx" => "opera://extensions",
                    "edge" => "edge://extensions",
                    "brave" => "brave://extensions",
                    _ => "chrome://extensions",
                };
                cmd.arg(ext_url);
                cmd.creation_flags(CREATE_NEW_PROCESS_GROUP);
                cmd.spawn().map_err(|e| format!("Failed to launch {}: {}", name, e))?;
                logs.push(format!("Launched {} opening {}", name, ext_url));
            }
        }

        // Copy path to clipboard
        #[cfg(windows)]
        {
            use std::io::Write;
            use std::os::windows::process::CommandExt;
            let mut clip_cmd = std::process::Command::new("clip.exe");
            clip_cmd.stdin(std::process::Stdio::piped());
            clip_cmd.creation_flags(CREATE_NO_WINDOW);
            if let Ok(mut child) = clip_cmd.spawn() {
                if let Some(mut stdin) = child.stdin.take() {
                    let _ = stdin.write_all(ext_to_copy_to_clip.as_bytes());
                }
                let _ = child.wait();
                logs.push("Copied extension path to Windows clipboard".to_string());
            }
        }

        let message = if browser_id == "firefox" {
            format!("SnapLoad extension prompt opened in {}! Click 'Add' to activate.", name)
        } else {
            format!(
                "Browser {} configured with native registry & profile preferences! Extensions page opened.",
                name
            )
        };

        Ok(AutoInstallResult {
            success: true,
            browser_id: browser_id.to_string(),
            browser_name: name.to_string(),
            exe_path,
            profile_path: primary_profile,
            message,
            logs,
        })
    }
}

async fn test_native_messaging_ping(engine_path: &Path) -> (bool, String) {
    if !engine_path.exists() {
        return (false, "Engine binary missing".to_string());
    }

    let mut cmd = tokio::process::Command::new(engine_path);
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd.stdin(std::process::Stdio::piped());
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::null());

    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => return (false, format!("Failed to spawn engine: {}", e)),
    };

    let mut stdin = match child.stdin.take() {
        Some(s) => s,
        None => return (false, "Could not open stdin".to_string()),
    };
    let mut stdout = match child.stdout.take() {
        Some(s) => s,
        None => return (false, "Could not open stdout".to_string()),
    };

    let ping_msg = serde_json::json!({
        "action": "ping",
        "id": "diag-verify-1"
    })
    .to_string();

    let len = ping_msg.len() as u32;
    let mut frame = Vec::with_capacity(4 + ping_msg.len());
    frame.extend_from_slice(&len.to_le_bytes());
    frame.extend_from_slice(ping_msg.as_bytes());

    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    if let Err(e) = stdin.write_all(&frame).await {
        let _ = child.kill().await;
        return (false, format!("Failed writing ping frame: {}", e));
    }
    let _ = stdin.flush().await;

    // Read response with 2.5s timeout
    let read_result = timeout(Duration::from_millis(2500), async {
        let mut len_buf = [0u8; 4];
        stdout.read_exact(&mut len_buf).await?;
        let resp_len = u32::from_le_bytes(len_buf) as usize;
        if resp_len > 1_000_000 {
            return Err(std::io::Error::other("Response oversized"));
        }
        let mut resp_buf = vec![0u8; resp_len];
        stdout.read_exact(&mut resp_buf).await?;
        Ok(String::from_utf8_lossy(&resp_buf).to_string())
    })
    .await;

    let _ = child.kill().await;

    match read_result {
        Ok(Ok(response_text)) => {
            if response_text.contains("PONG") || response_text.contains("pong") {
                (
                    true,
                    "Native Messaging roundtrip successful (PONG received)".to_string(),
                )
            } else {
                (false, format!("Unexpected engine response: {}", response_text))
            }
        }
        Ok(Err(e)) => (false, format!("I/O error reading engine response: {}", e)),
        Err(_) => (
            false,
            "Ping test timed out (engine did not reply in 2.5s)".to_string(),
        ),
    }
}

pub fn unpack_zip_bytes(bytes: &[u8], dest_dir: &Path) -> Result<(), String> {
    if bytes.is_empty() {
        return Ok(());
    }
    create_dir_all(dest_dir).map_err(|e| format!("Failed to create directory {:?}: {}", dest_dir, e))?;
    let cursor = std::io::Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(cursor).map_err(|e| format!("Failed to read zip archive: {}", e))?;
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|e| format!("Zip entry error: {}", e))?;
        let raw_name = file.name().to_string();
        let normalized = raw_name.replace('\\', "/");
        let trimmed = normalized.trim_start_matches("./").trim_start_matches('/');
        if trimmed.is_empty() {
            continue;
        }

        let outpath = dest_dir.join(Path::new(trimmed));
        let is_dir = file.is_dir() || raw_name.ends_with('/') || raw_name.ends_with('\\') || normalized.ends_with('/');
        if is_dir {
            create_dir_all(&outpath).map_err(|e| format!("Failed to create subfolder {:?}: {}", outpath, e))?;
        } else {
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    create_dir_all(p).map_err(|e| format!("Failed to create parent {:?}: {}", p, e))?;
                }
            }
            let mut outfile = std::fs::File::create(&outpath)
                .map_err(|e| format!("Failed to write file {:?}: {}", outpath, e))?;
            std::io::copy(&mut file, &mut outfile)
                .map_err(|e| format!("Failed to write contents to {:?}: {}", outpath, e))?;
        }
    }
    Ok(())
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

pub(crate) fn which(name: &str) -> Option<PathBuf> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_paths_returns_valid_structure() {
        let paths = SetupManager::get_paths();
        assert!(!paths.app_dir.is_empty());
        assert!(!paths.chromium_dir.is_empty());
        assert!(!paths.firefox_xpi.is_empty());
        assert!(!paths.downloads_dir.is_empty());
        assert!(paths.chromium_dir.contains("chromium"));
        assert!(paths.firefox_xpi.ends_with(".xpi"));
    }

    #[test]
    fn test_auto_install_invalid_browser_fails() {
        let res = SetupManager::auto_install_for_browser("internet_explorer_6", false);
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("Unsupported browser"));
    }

    #[test]
    fn test_ensure_chromium_ext_deployed_has_manifest() {
        let res = SetupManager::ensure_chromium_ext_deployed();
        assert!(res.is_ok(), "ensure_chromium_ext_deployed failed: {:?}", res);
        let dir = res.unwrap();
        assert!(dir.join("manifest.json").exists(), "manifest.json missing in {:?}", dir);
    }

    #[test]
    fn test_ensure_engine_deployed_locates_binary() {
        let res = SetupManager::ensure_engine_deployed();
        assert!(res.is_ok(), "ensure_engine_deployed failed: {:?}", res);
        let path = res.unwrap();
        assert!(path.exists());
    }

    #[test]
    fn test_is_browser_process_running_returns_bool() {
        let _ = BrowserDetector::is_browser_process_running("opera");
        let _ = BrowserDetector::is_browser_process_running("chrome");
        let _ = BrowserDetector::is_browser_process_running("nonexistent");
    }

    #[test]
    fn test_find_browser_profiles_valid_paths() {
        for b_id in ["chrome", "opera", "edge", "brave", "operagx", "firefox"] {
            let profs = BrowserDetector::find_browser_profiles(b_id);
            for p in profs {
                assert!(p.exists(), "Profile path for {} must exist: {:?}", b_id, p);
            }
        }
    }

    #[test]
    fn test_policy_allowlist_registration() {
        for b_id in ["opera", "operagx", "chrome", "edge", "brave"] {
            let res = RegistryManager::register_chromium_policy_allowlist(b_id);
            assert!(res.is_ok(), "Policy allowlist registration failed for {}: {:?}", b_id, res);
        }
    }

    #[test]
    fn test_policy_sources_registration() {
        for b_id in ["opera", "operagx", "chrome", "edge", "brave"] {
            let res = RegistryManager::register_chromium_policy_sources(b_id);
            assert!(res.is_ok(), "Policy sources registration failed for {}: {:?}", b_id, res);
        }
    }

    #[test]
    fn test_policy_developer_mode_registration() {
        for b_id in ["opera", "operagx", "chrome", "edge", "brave"] {
            let res = RegistryManager::register_chromium_developer_mode_policy(b_id);
            assert!(res.is_ok(), "Policy developer mode registration failed for {}: {:?}", b_id, res);
        }
    }

    #[test]
    fn test_isolated_launch_args_rejects_dummy() {
        let dummy = PathBuf::from("dummy");
        let res = SetupManager::build_isolated_launch_args("opera", &dummy);
        assert!(res.is_err(), "build_isolated_launch_args must reject 'dummy' path");

        let dummy_upper = PathBuf::from("DUMMY");
        let res2 = SetupManager::build_isolated_launch_args("opera", &dummy_upper);
        assert!(res2.is_err(), "build_isolated_launch_args must reject case-insensitive 'DUMMY'");
    }

    #[test]
    fn test_isolated_launch_args_rejects_relative_path() {
        let rel = PathBuf::from("extensions/chromium");
        let res = SetupManager::build_isolated_launch_args("opera", &rel);
        assert!(res.is_err(), "build_isolated_launch_args must reject relative path");
    }

    #[test]
    fn test_isolated_launch_args_rejects_missing_manifest() {
        let temp_dir = std::env::temp_dir().join("snapload_test_no_manifest_isolated");
        let _ = std::fs::remove_dir_all(&temp_dir);
        let _ = std::fs::create_dir_all(&temp_dir);
        let res = SetupManager::build_isolated_launch_args("opera", &temp_dir);
        assert!(res.is_err(), "build_isolated_launch_args must reject directory without manifest.json");
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_isolated_launch_args_success_with_valid_manifest() {
        let temp_dir = std::env::temp_dir().join("snapload_test_valid_manifest_isolated");
        let _ = std::fs::remove_dir_all(&temp_dir);
        let _ = std::fs::create_dir_all(&temp_dir);
        let manifest_file = temp_dir.join("manifest.json");
        std::fs::write(&manifest_file, r#"{"manifest_version": 3, "name": "SnapLoad"}"#).unwrap();

        let res = SetupManager::build_isolated_launch_args("opera", &temp_dir);
        assert!(res.is_ok(), "Expected Ok for valid absolute path with manifest: {:?}", res);
        let (profile_dir, args) = res.unwrap();

        assert!(profile_dir.to_string_lossy().contains("snapload_isolated_profile_opera"));
        assert!(args.iter().any(|a| a.starts_with("--user-data-dir=")));
        assert!(args.iter().any(|a| a.starts_with("--load-extension=") && a.contains(&temp_dir.to_string_lossy().to_string())));
        assert!(args.iter().any(|a| a == "--disable-features=DisableLoadExtensionCommandLineSwitch"));
        assert!(!args.iter().any(|a| a.contains("dummy")));

        let _ = std::fs::remove_dir_all(&temp_dir);
        let _ = std::fs::remove_dir_all(&profile_dir);
    }

    #[test]
    fn test_launch_browser_isolated_rejects_dummy_or_relative() {
        let dummy = PathBuf::from("dummy");
        let res = SetupManager::launch_browser_isolated("opera", "opera.exe", &dummy);
        assert!(res.is_err(), "launch_browser_isolated must reject dummy extension path");

        let rel = PathBuf::from("some/relative/path");
        let res2 = SetupManager::launch_browser_isolated("opera", "opera.exe", &rel);
        assert!(res2.is_err(), "launch_browser_isolated must reject relative extension path");
    }

    #[test]
    fn test_cleanup_desktop_shortcuts_does_not_panic() {
        let logs = SetupManager::cleanup_desktop_shortcuts();
        let _ = logs;
    }

    #[test]
    fn test_clean_all_browser_shortcuts_does_not_panic() {
        let logs = SetupManager::clean_all_browser_shortcuts();
        let _ = logs;
    }

    #[test]
    fn test_cleanup_desktop_shortcuts_removes_case_variations() {
        if let Some(user) = std::env::var_os("USERPROFILE") {
            let desktop = PathBuf::from(user).join("Desktop");
            if desktop.exists() {
                let dummy_file = desktop.join("snapload_test_case_variant.lnk");
                let _ = std::fs::write(&dummy_file, b"dummy lnk");
                assert!(dummy_file.exists());
                let _ = SetupManager::cleanup_desktop_shortcuts();
                assert!(!dummy_file.exists(), "Dummy case-variant shortcut must be removed by cleanup");
            }
        }
    }

    #[test]
    fn test_ensure_chromium_ext_deployed_manifest_contents() {
        let res = SetupManager::ensure_chromium_ext_deployed();
        assert!(res.is_ok(), "Failed ensuring chromium ext deployed: {:?}", res);
        let path = res.unwrap();
        let manifest = path.join("manifest.json");
        assert!(manifest.exists(), "manifest.json must exist in {:?}", path);
        let content = std::fs::read_to_string(&manifest).expect("manifest.json readable");
        assert!(content.contains("\"manifest_version\""), "manifest must have manifest_version");
        assert!(content.contains("SnapLoad"), "manifest must contain SnapLoad");
    }

    #[test]
    fn test_get_paths_pure_without_creating_missing_subdirs() {
        let dummy_nonexistent = std::env::temp_dir().join("snapload_test_nonexistent_dir_probe");
        let _ = std::fs::remove_dir_all(&dummy_nonexistent);
        assert!(!dummy_nonexistent.exists());

        // Probe path getters logic
        let p_chrom = SetupManager::get_chromium_ext_dir();
        let p_ff = SetupManager::get_firefox_ext_dir();
        let p_bin = SetupManager::get_bin_dir();
        let p_logs = SetupManager::get_logs_dir();
        let p_manifest = SetupManager::get_manifest_dir();
        let p_reg_manifest = RegistryManager::get_manifest_dir();

        // Verify that path getters return standard names without panicking
        assert!(p_chrom.ends_with("chromium"));
        assert!(p_ff.ends_with("firefox"));
        assert!(p_bin.ends_with("bin"));
        assert!(p_logs.ends_with("logs"));
        assert!(p_manifest.ends_with("manifests"));
        assert!(p_reg_manifest.ends_with("manifests"));

        // Test with simulated clean machine / nonexistent base path
        let orig_localappdata = std::env::var_os("LOCALAPPDATA");
        std::env::set_var("LOCALAPPDATA", &dummy_nonexistent);

        let clean_app = SetupManager::get_app_dir();
        let clean_bin = SetupManager::get_bin_dir();
        let clean_logs = SetupManager::get_logs_dir();
        let clean_ext = SetupManager::get_extensions_dir();
        let clean_chrom = SetupManager::get_chromium_ext_dir();
        let clean_ff = SetupManager::get_firefox_ext_dir();
        let clean_man = SetupManager::get_manifest_dir();
        let clean_reg_man = RegistryManager::get_manifest_dir();

        // Ensure getters did not create any directories on disk
        assert!(!dummy_nonexistent.exists(), "Path getters must be pure and NOT create directories on disk");
        assert!(!clean_app.exists());
        assert!(!clean_bin.exists());
        assert!(!clean_logs.exists());
        assert!(!clean_ext.exists());
        assert!(!clean_chrom.exists());
        assert!(!clean_ff.exists());
        assert!(!clean_man.exists());
        assert!(!clean_reg_man.exists());

        // Restore environment variable
        match orig_localappdata {
            Some(val) => std::env::set_var("LOCALAPPDATA", val),
            None => std::env::remove_var("LOCALAPPDATA"),
        }
    }

    #[test]
    fn test_ensure_directories_at_clean_machine_creates_all_subdirs_and_allows_writes() {
        let temp_base = std::env::temp_dir().join("snapload_test_clean_machine_probe");
        let _ = std::fs::remove_dir_all(&temp_base);
        assert!(!temp_base.exists(), "Clean test directory must not exist prior to test");

        let res = SetupManager::ensure_directories_at(&temp_base);
        assert!(res.is_ok(), "ensure_directories_at must succeed: {:?}", res);
        let created_dirs = res.unwrap();
        assert_eq!(created_dirs.len(), 7, "Must ensure 7 target directories");

        // Verify all target directories exist on disk
        let bin = temp_base.join("bin");
        let logs = temp_base.join("logs");
        let ext = temp_base.join("extensions");
        let chrom = ext.join("chromium");
        let ff = ext.join("firefox");
        let manifests = temp_base.join("manifests");

        assert!(temp_base.exists(), "Base app directory must exist");
        assert!(bin.exists(), "bin directory must exist");
        assert!(logs.exists(), "logs directory must exist");
        assert!(ext.exists(), "extensions directory must exist");
        assert!(chrom.exists(), "chromium extension directory must exist");
        assert!(ff.exists(), "firefox extension directory must exist");
        assert!(manifests.exists(), "manifests directory must exist");

        // Verify that writing files into these newly created directories succeeds (no OS error 3)
        let engine_file = bin.join("snapload-engine.exe");
        assert!(std::fs::write(&engine_file, b"dummy engine executable").is_ok(), "Writing to bin must succeed");

        let chrom_zip = ext.join("snapload-extension-chromium.zip");
        assert!(std::fs::write(&chrom_zip, b"dummy zip").is_ok(), "Writing to extensions must succeed");

        let log_file = logs.join("installer.log");
        assert!(std::fs::write(&log_file, b"dummy log").is_ok(), "Writing to logs must succeed");

        let manifest_file = manifests.join("com.darkskyee.snapload.chromium.json");
        assert!(std::fs::write(&manifest_file, b"{}").is_ok(), "Writing to manifests must succeed");

        let chrom_manifest = chrom.join("manifest.json");
        assert!(std::fs::write(&chrom_manifest, b"{}").is_ok(), "Writing to chromium must succeed");

        let ff_xpi = ff.join("snapload-extension-firefox.xpi");
        assert!(std::fs::write(&ff_xpi, b"dummy xpi").is_ok(), "Writing to firefox must succeed");

        let _ = std::fs::remove_dir_all(&temp_base);
    }

    #[test]
    fn test_ensure_directories_at_fails_gracefully_when_path_is_blocked_by_file() {
        let temp_file = std::env::temp_dir().join("snapload_test_blocked_by_file");
        let _ = std::fs::remove_file(&temp_file);
        let _ = std::fs::remove_dir_all(&temp_file);
        std::fs::write(&temp_file, b"blocking file content").expect("Create blocking file");

        // Attempting to create directory hierarchy inside a file must fail cleanly with Err
        let res = SetupManager::ensure_directories_at(&temp_file);
        assert!(res.is_err(), "ensure_directories_at must return an Err if base path is a file");

        let _ = std::fs::remove_file(&temp_file);
    }

    #[test]
    fn test_ensure_directories_at_is_idempotent() {
        let temp_base = std::env::temp_dir().join("snapload_test_idempotent_probe");
        let _ = std::fs::remove_dir_all(&temp_base);

        // First run: creates
        let res1 = SetupManager::ensure_directories_at(&temp_base);
        assert!(res1.is_ok());

        // Write marker file
        let marker = temp_base.join("bin").join("marker.txt");
        std::fs::write(&marker, b"marker content").expect("Write marker");

        // Second run: must not fail or delete existing files
        let res2 = SetupManager::ensure_directories_at(&temp_base);
        assert!(res2.is_ok());
        assert!(marker.exists(), "Existing files must remain preserved across repeated ensure calls");

        let _ = std::fs::remove_dir_all(&temp_base);
    }

    #[test]
    fn test_ensure_app_directories_succeeds_and_creates_all_target_dirs() {
        let res = SetupManager::ensure_app_directories();
        assert!(res.is_ok(), "ensure_app_directories must succeed: {:?}", res);

        assert!(SetupManager::get_app_dir().exists(), "app dir must exist");
        assert!(SetupManager::get_bin_dir().exists(), "bin dir must exist");
        assert!(SetupManager::get_logs_dir().exists(), "logs dir must exist");
        assert!(SetupManager::get_extensions_dir().exists(), "extensions dir must exist");
        assert!(SetupManager::get_chromium_ext_dir().exists(), "chromium ext dir must exist");
        assert!(SetupManager::get_firefox_ext_dir().exists(), "firefox ext dir must exist");
        assert!(SetupManager::get_manifest_dir().exists(), "manifests dir must exist");
    }

    #[test]
    fn test_run_install_on_clean_machine_succeeds_without_os_error() {
        let temp_base = std::env::temp_dir().join("snapload_test_clean_machine_install");
        let _ = std::fs::remove_dir_all(&temp_base);
        assert!(!temp_base.exists());

        let orig_localappdata = std::env::var_os("LOCALAPPDATA");
        std::env::set_var("LOCALAPPDATA", &temp_base);

        let options = InstallOptions {
            browsers: vec![],
            install_ytdlp: false,
            install_ffmpeg: false,
        };

        let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
        let install_res = rt.block_on(SetupManager::run_install(options));

        // Restore environment variable immediately
        match orig_localappdata {
            Some(val) => std::env::set_var("LOCALAPPDATA", val),
            None => std::env::remove_var("LOCALAPPDATA"),
        }

        assert!(install_res.is_ok(), "run_install must succeed on clean machine without OS error 3: {:?}", install_res);

        let app_dir = temp_base.join("SnapLoad");
        assert!(app_dir.join("bin").exists(), "bin dir must exist");
        assert!(app_dir.join("logs").exists(), "logs dir must exist");
        assert!(app_dir.join("extensions").exists(), "extensions dir must exist");
        assert!(app_dir.join("extensions").join("chromium").exists(), "chromium dir must exist");
        assert!(app_dir.join("extensions").join("firefox").exists(), "firefox dir must exist");
        assert!(app_dir.join("manifests").exists(), "manifests dir must exist");

        // Verify engine was deployed
        assert!(app_dir.join("bin").join("snapload-engine.exe").exists(), "snapload-engine.exe must exist in bin");

        let _ = std::fs::remove_dir_all(&temp_base);
    }

    #[test]
    fn test_clean_extension_from_chromium_profile_scrubs_all_records() {
        let temp_dir = std::env::temp_dir().join("snapload_profile_clean_test");
        let _ = std::fs::remove_dir_all(&temp_dir);
        let _ = std::fs::create_dir_all(&temp_dir);

        let ext_id = "ikkidkbojpedpeaomipgioiigmjlpflg";
        let prefs_json = serde_json::json!({
            "extensions": {
                "settings": {
                    "ikkidkbojpedpeaomipgioiigmjlpflg": {
                        "path": "C:\\fake\\snapload\\extensions\\chromium",
                        "state": 1
                    }
                },
                "opsettings": {
                    "ikkidkbojpedpeaomipgioiigmjlpflg": {
                        "path": "C:\\fake\\snapload\\extensions\\chromium",
                        "state": 1
                    }
                },
                "pinned_extensions": ["ikkidkbojpedpeaomipgioiigmjlpflg", "other_extension_12345"]
            },
            "protection": {
                "macs": {
                    "extensions": {
                        "opsettings": {
                            "ikkidkbojpedpeaomipgioiigmjlpflg": "HASH123",
                            "other_extension_12345": "HASH456"
                        },
                        "opsettings_encrypted_hash": {
                            "ikkidkbojpedpeaomipgioiigmjlpflg": "ENC123",
                            "other_extension_12345": "ENC456"
                        }
                    }
                }
            }
        });

        let serialized = serde_json::to_string_pretty(&prefs_json).unwrap();
        let sec_prefs = temp_dir.join("Secure Preferences");
        let sec_prefs_bak = temp_dir.join("Secure Preferences.backup");
        std::fs::write(&sec_prefs, &serialized).unwrap();
        std::fs::write(&sec_prefs_bak, &serialized).unwrap();

        // Subdirectories to be deleted
        let ext_sub = temp_dir.join("Extensions").join(ext_id);
        std::fs::create_dir_all(&ext_sub).unwrap();
        assert!(ext_sub.exists());

        let idb_sub = temp_dir.join("IndexedDB").join(format!("chrome-extension_{}_0.indexeddb.leveldb", ext_id));
        std::fs::create_dir_all(&idb_sub).unwrap();
        assert!(idb_sub.exists());

        // Run cleaning
        SetupManager::clean_extension_from_chromium_profile(&temp_dir, ext_id);

        // Verify directories were deleted
        assert!(!ext_sub.exists(), "Extension subdir must be removed");
        assert!(!idb_sub.exists(), "IndexedDB subdir must be removed");

        // Verify Secure Preferences was cleaned
        for file in &[sec_prefs, sec_prefs_bak] {
            let content = std::fs::read_to_string(file).unwrap();
            let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
            assert!(parsed.pointer("/extensions/settings/ikkidkbojpedpeaomipgioiigmjlpflg").is_none());
            assert!(parsed.pointer("/extensions/opsettings/ikkidkbojpedpeaomipgioiigmjlpflg").is_none());
            assert!(parsed.pointer("/protection/macs/extensions/opsettings/ikkidkbojpedpeaomipgioiigmjlpflg").is_none());
            assert!(parsed.pointer("/protection/macs/extensions/opsettings_encrypted_hash/ikkidkbojpedpeaomipgioiigmjlpflg").is_none());
            // Verify other extension was preserved
            assert!(parsed.pointer("/protection/macs/extensions/opsettings/other_extension_12345").is_some());
            let pinned = parsed.pointer("/extensions/pinned_extensions").and_then(|p| p.as_array()).unwrap();
            assert_eq!(pinned.len(), 1);
            assert_eq!(pinned[0].as_str(), Some("other_extension_12345"));
        }

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_isolated_launch_args_handles_path_with_spaces() {
        let temp_dir = std::env::temp_dir().join("snapload test dir with spaces");
        let _ = std::fs::remove_dir_all(&temp_dir);
        let _ = std::fs::create_dir_all(&temp_dir);
        let manifest_file = temp_dir.join("manifest.json");
        std::fs::write(&manifest_file, r#"{"manifest_version": 3, "name": "SnapLoad with spaces"}"#).unwrap();

        let res = SetupManager::build_isolated_launch_args("opera", &temp_dir);
        assert!(res.is_ok(), "Expected Ok for path with spaces: {:?}", res);
        let (profile_dir, args) = res.unwrap();

        let user_data_arg = args.iter().find(|a| a.starts_with("--user-data-dir=")).unwrap();
        assert!(user_data_arg.starts_with("--user-data-dir=\""));
        assert!(user_data_arg.ends_with("\""));

        let load_ext_arg = args.iter().find(|a| a.starts_with("--load-extension=")).unwrap();
        assert!(load_ext_arg.starts_with("--load-extension=\""));
        assert!(load_ext_arg.ends_with("\""));
        assert!(load_ext_arg.contains("snapload test dir with spaces"));

        let _ = std::fs::remove_dir_all(&temp_dir);
        let _ = std::fs::remove_dir_all(&profile_dir);
    }

    #[test]
    fn test_inject_extension_to_chromium_profile_only_modifies_preferences_not_secure_preferences() {
        let temp_dir = std::env::temp_dir().join("snapload_profile_prefs_hmac_test");
        let _ = std::fs::remove_dir_all(&temp_dir);
        let _ = std::fs::create_dir_all(&temp_dir);

        let ext_dir = temp_dir.join("ext_src");
        std::fs::create_dir_all(&ext_dir).unwrap();
        std::fs::write(ext_dir.join("manifest.json"), r#"{"manifest_version": 3, "name": "SnapLoad"}"#).unwrap();

        let ext_id = "ikkidkbojpedpeaomipgioiigmjlpflg";
        let initial_sec_prefs = r#"{"protection":{"macs":{}},"unrelated_secure_data":123}"#;
        let sec_prefs_file = temp_dir.join("Secure Preferences");
        std::fs::write(&sec_prefs_file, initial_sec_prefs).unwrap();

        let prefs_file = temp_dir.join("Preferences");
        std::fs::write(&prefs_file, r#"{"extensions":{"ui":{"developer_mode":false}}}"#).unwrap();

        SetupManager::inject_extension_to_chromium_profile(&temp_dir, ext_id, &ext_dir);

        // Verify Preferences was updated
        let prefs_content = std::fs::read_to_string(&prefs_file).unwrap();
        assert!(prefs_content.contains(ext_id), "Preferences must contain extension id");
        assert!(prefs_content.contains("\"developer_mode\": true"), "developer_mode must be set to true in Preferences");

        // Verify Secure Preferences was NOT touched (preserving HMAC integrity)
        let sec_prefs_content = std::fs::read_to_string(&sec_prefs_file).unwrap();
        assert_eq!(sec_prefs_content, initial_sec_prefs, "Secure Preferences must NOT be modified by injector");

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}

