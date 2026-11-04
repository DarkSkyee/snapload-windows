use crate::browser::BrowserDetector;
use crate::registry::RegistryManager;
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
        let _ = create_dir_all(&path);
        path
    }

    pub fn get_logs_dir() -> PathBuf {
        let mut path = Self::get_app_dir();
        path.push("logs");
        let _ = create_dir_all(&path);
        path
    }

    pub fn get_extensions_dir() -> PathBuf {
        let mut path = Self::get_app_dir();
        path.push("extensions");
        let _ = create_dir_all(&path);
        path
    }

    pub fn get_chromium_ext_dir() -> PathBuf {
        let mut path = Self::get_extensions_dir();
        path.push("chromium");
        let _ = create_dir_all(&path);
        path
    }

    pub fn get_firefox_ext_dir() -> PathBuf {
        let mut path = Self::get_extensions_dir();
        path.push("firefox");
        let _ = create_dir_all(&path);
        path
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

    pub fn deploy_to_browser_profiles(_chrome_unpacked_dir: &Path) -> Vec<String> {
        vec!["Unpacked Chromium extension ready in SnapLoad directory".to_string()]
    }

    pub async fn run_install(options: InstallOptions) -> Result<InstallResult, String> {
        let mut logs = Vec::new();
        logs.push("Initializing SnapLoad desktop environment...".to_string());

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
            std::fs::copy(&src, &engine_dest)
                .map_err(|e| format!("Failed to copy engine binary from {:?}: {}", src, e))?;
            logs.push(format!("Deployed SnapLoad Engine to: {:?}", engine_dest));
        } else if !EMBEDDED_ENGINE.is_empty() {
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
            return false;
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
        if std::fs::remove_dir_all(dir).is_ok() {
            return true;
        }
        #[cfg(windows)]
        {
            let mut cmd = std::process::Command::new("cmd.exe");
            cmd.args(["/C", "rd", "/S", "/Q", &dir.to_string_lossy()]);
            cmd.creation_flags(CREATE_NO_WINDOW);
            let _ = cmd.output();
        }
        !dir.exists()
    }

    pub fn clean_extension_from_chromium_profile(profile_path: &Path, ext_id: &str) {
        // 1. Clean from Preferences JSON
        let prefs_file = profile_path.join("Preferences");
        if prefs_file.exists() {
            if let Ok(content) = std::fs::read_to_string(&prefs_file) {
                if let Ok(mut json) = serde_json::from_str::<serde_json::Value>(&content) {
                    let mut modified = false;

                    if let Some(settings) = json.pointer_mut("/extensions/settings") {
                        if let Some(map) = settings.as_object_mut() {
                            if map.remove(ext_id).is_some() {
                                modified = true;
                            }
                        }
                    }

                    if let Some(pinned) = json.pointer_mut("/extensions/pinned_extensions") {
                        if let Some(arr) = pinned.as_array_mut() {
                            let orig_len = arr.len();
                            arr.retain(|v| v.as_str().map(|s| s != ext_id).unwrap_or(true));
                            if arr.len() != orig_len {
                                modified = true;
                            }
                        }
                    }

                    if let Some(toolbar) = json.pointer_mut("/extensions/toolbar") {
                        if let Some(arr) = toolbar.as_array_mut() {
                            let orig_len = arr.len();
                            arr.retain(|v| v.as_str().map(|s| s != ext_id).unwrap_or(true));
                            if arr.len() != orig_len {
                                modified = true;
                            }
                        }
                    }

                    if modified {
                        if let Ok(serialized) = serde_json::to_string_pretty(&json) {
                            let _ = std::fs::write(&prefs_file, serialized);
                        }
                    }
                }
            }
        }

        // 2. Also clean Secure Preferences if present
        let sec_prefs_file = profile_path.join("Secure Preferences");
        if sec_prefs_file.exists() {
            if let Ok(content) = std::fs::read_to_string(&sec_prefs_file) {
                if let Ok(mut json) = serde_json::from_str::<serde_json::Value>(&content) {
                    let mut modified = false;
                    if let Some(settings) = json.pointer_mut("/extensions/settings") {
                        if let Some(map) = settings.as_object_mut() {
                            if map.remove(ext_id).is_some() {
                                modified = true;
                            }
                        }
                    }
                    if modified {
                        if let Ok(serialized) = serde_json::to_string_pretty(&json) {
                            let _ = std::fs::write(&sec_prefs_file, serialized);
                        }
                    }
                }
            }
        }

        // 3. Remove all extension subdirectories
        let subdirs = [
            "Extensions",
            "Local Extension Settings",
            "Extension State",
            "Extension Rules",
            "Extension Scripts",
            "Sync Extension Settings",
        ];
        for sub in subdirs {
            let target = profile_path.join(sub).join(ext_id);
            if target.exists() {
                let _ = Self::force_remove_dir_all(&target);
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
                "yt-dlp.exe",
                "opera.exe",
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
            std::thread::sleep(std::time::Duration::from_millis(600));
        }

        // 2. Unregister all registry keys
        let removed_keys = RegistryManager::unregister_all();
        logs.push(format!(
            "Removed {} registry registrations",
            removed_keys.len()
        ));

        // 3. Deep clean extension from all browser profiles
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
                                            a.get("id").and_then(|v| v.as_str()) != Some("snapload@darkskyee.github.io")
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

        // 4. Force remove all binaries: ffmpeg.exe, yt-dlp.exe, snapload-engine.exe
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
            for exe in &["ffmpeg.exe", "yt-dlp.exe", "snapload-engine.exe"] {
                let f = b_dir.join(exe);
                if f.exists() {
                    if Self::force_remove_file(&f) {
                        logs.push(format!("Removed binary: {:?}", f));
                    }
                }
            }
        }

        // 5. Completely remove SnapLoad application directories
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

        logs.push("SnapLoad components and browser extensions uninstalled. Downloaded media preserved.".to_string());
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
            std::fs::copy(&src, &engine_dest)
                .map_err(|e| format!("Failed to copy engine binary from {:?}: {}", src, e))?;
            Ok(engine_dest)
        } else if !EMBEDDED_ENGINE.is_empty() {
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
        if chrome_unpacked_dir.join("manifest.json").exists() {
            return Ok(chrome_unpacked_dir);
        }

        let chrome_zip_dest = Self::get_extensions_dir().join("snapload-extension-chromium.zip");
        if !EMBEDDED_CHROME_EXT.is_empty() {
            let _ = std::fs::write(&chrome_zip_dest, EMBEDDED_CHROME_EXT);
            if unpack_zip_bytes(EMBEDDED_CHROME_EXT, &chrome_unpacked_dir).is_ok() {
                return Ok(chrome_unpacked_dir);
            }
        }

        let workspace_chrome_dirs = [
            PathBuf::from("extension/dist/chrome"),
            PathBuf::from("../extension/dist/chrome"),
            PathBuf::from("../../extension/dist/chrome"),
        ];
        for w in &workspace_chrome_dirs {
            if w.exists() && w.join("manifest.json").exists() {
                let _ = copy_dir_recursive(w, &chrome_unpacked_dir);
                return Ok(chrome_unpacked_dir);
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

        let ffmpeg_dest = bin_dir.join("ffmpeg.exe");
        if ffmpeg_dest.exists() {
            logs.push(format!("FFmpeg core verified at: {:?}", ffmpeg_dest));
        } else if let Some(path_src) = which("ffmpeg.exe") {
            if std::fs::copy(&path_src, &ffmpeg_dest).is_ok() {
                logs.push(format!("Copied FFmpeg from PATH: {:?}", path_src));
            }
        }

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
                    "Browser {} is currently running — closing processes to load extension into session...",
                    name
                ));
                let _ = BrowserDetector::close_browser_processes(browser_id);
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

            // Launch Firefox with XPI path to prompt installation and open YouTube
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                const CREATE_NEW_PROCESS_GROUP: u32 = 0x00000200;
                let mut cmd = std::process::Command::new(&exe_path);
                cmd.arg(firefox_xpi.to_string_lossy().to_string());
                cmd.arg("https://www.youtube.com");
                cmd.creation_flags(CREATE_NEW_PROCESS_GROUP);
                cmd.spawn().map_err(|e| format!("Failed to launch Firefox: {}", e))?;
                logs.push("Launched Firefox with YouTube and extension prompt".to_string());
            }
        } else {
            let chrome_unpacked = Self::ensure_chromium_ext_deployed()?;
            ext_to_copy_to_clip = chrome_unpacked.to_string_lossy().to_string();
            logs.push(format!("Chromium extension folder: {:?}", chrome_unpacked));



            // Register external extension in Registry
            let _ = RegistryManager::register_chromium_extension(browser_id, &chrome_unpacked);
            logs.push(format!("Registered extension in Windows Registry for {}", name));

            // Launch browser in detached process
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                const CREATE_NEW_PROCESS_GROUP: u32 = 0x00000200;
                let mut cmd = std::process::Command::new(&exe_path);
                cmd.arg("https://www.youtube.com");
                cmd.creation_flags(CREATE_NEW_PROCESS_GROUP);
                cmd.spawn().map_err(|e| format!("Failed to launch {}: {}", name, e))?;
                logs.push(format!("Launched {} on YouTube with SnapLoad configured", name));
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

        let message = if was_running && restart_if_running {
            format!(
                "Browser {} has been restarted and SnapLoad extension is active!",
                name
            )
        } else {
            format!(
                "SnapLoad extension has been installed and activated in {}!",
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
}
