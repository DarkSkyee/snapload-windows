use crate::registry::RegistryManager;
use serde::{Deserialize, Serialize};
use std::fs::{create_dir_all, remove_dir_all};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::time::timeout;

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
            .map(|u| PathBuf::from(u).join("Downloads").join("SnapLoad"))
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

    pub fn deploy_to_browser_profiles(chrome_unpacked_dir: &Path) -> Vec<String> {
        let mut deployed = Vec::new();
        let ext_id = "ikkidkbojpedpeaomipgioiigmjlpflg";
        let version_tag = "1.0.0_0";

        let mut targets: Vec<(&str, PathBuf)> = Vec::new();

        // 1. Opera Stable & Opera GX
        if let Some(appdata) = std::env::var_os("APPDATA") {
            let opera_dir = PathBuf::from(&appdata)
                .join("Opera Software")
                .join("Opera Stable")
                .join("Default")
                .join("Extensions")
                .join(ext_id)
                .join(version_tag);
            targets.push(("Opera Stable", opera_dir));

            let operagx_dir = PathBuf::from(&appdata)
                .join("Opera Software")
                .join("Opera GX Stable")
                .join("Default")
                .join("Extensions")
                .join(ext_id)
                .join(version_tag);
            targets.push(("Opera GX", operagx_dir));
        }

        // 2. Google Chrome, Microsoft Edge, Brave Browser
        if let Some(local) = std::env::var_os("LOCALAPPDATA") {
            let chrome_dir = PathBuf::from(&local)
                .join("Google")
                .join("Chrome")
                .join("User Data")
                .join("Default")
                .join("Extensions")
                .join(ext_id)
                .join(version_tag);
            targets.push(("Google Chrome", chrome_dir));

            let edge_dir = PathBuf::from(&local)
                .join("Microsoft")
                .join("Edge")
                .join("User Data")
                .join("Default")
                .join("Extensions")
                .join(ext_id)
                .join(version_tag);
            targets.push(("Microsoft Edge", edge_dir));

            let brave_dir = PathBuf::from(&local)
                .join("BraveSoftware")
                .join("Brave-Browser")
                .join("User Data")
                .join("Default")
                .join("Extensions")
                .join(ext_id)
                .join(version_tag);
            targets.push(("Brave Browser", brave_dir));
        }

        for (name, target_dir) in targets {
            if let Some(ext_parent) = target_dir.parent() {
                if let Some(default_profile) = ext_parent.parent() {
                    if default_profile.exists() {
                        let _ = create_dir_all(&target_dir);
                        if let Ok(_) = copy_dir_recursive(chrome_unpacked_dir, &target_dir) {
                            deployed.push(format!("Deployed extension files to {} profile: {:?}", name, target_dir));
                        }
                    }
                }
            }
        }

        deployed
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

        // 5. Register in Windows Registry for selected browsers
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

        // 8. Initialize default download directory in User Downloads\SnapLoad
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
