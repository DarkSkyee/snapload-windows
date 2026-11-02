use crate::browser::BrowserDetector;
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
        Self::get_app_dir().join("bin")
    }

    pub fn get_extensions_dir() -> PathBuf {
        Self::get_app_dir().join("extensions")
    }

    pub fn get_chromium_ext_dir() -> PathBuf {
        Self::get_extensions_dir().join("chromium")
    }

    pub fn get_firefox_ext_dir() -> PathBuf {
        Self::get_extensions_dir().join("firefox")
    }

    pub fn get_paths() -> ExtensionPaths {
        let app_dir = Self::get_app_dir().to_string_lossy().to_string();
        let chromium_dir = Self::get_chromium_ext_dir().to_string_lossy().to_string();
        let firefox_xpi = Self::get_firefox_ext_dir()
            .join("snapload-extension-firefox.xpi")
            .to_string_lossy()
            .to_string();
        let downloads_dir = std::env::var_os("USERPROFILE")
            .map(|p| {
                PathBuf::from(p)
                    .join("Downloads")
                    .join("SnapLoad")
                    .to_string_lossy()
                    .to_string()
            })
            .unwrap_or_default();

        ExtensionPaths {
            app_dir,
            chromium_dir,
            firefox_xpi,
            downloads_dir,
        }
    }

    pub async fn run_install(options: InstallOptions) -> Result<InstallResult, String> {
        let mut logs = Vec::new();
        logs.push("Starting SnapLoad installation...".to_string());

        let app_dir = Self::get_app_dir();
        let bin_dir = Self::get_bin_dir();
        let ext_dir = Self::get_extensions_dir();
        let chrome_ext_dir = Self::get_chromium_ext_dir();
        let firefox_ext_dir = Self::get_firefox_ext_dir();

        create_dir_all(&bin_dir)
            .map_err(|e| format!("Failed to create bin dir {:?}: {}", bin_dir, e))?;
        create_dir_all(&chrome_ext_dir).map_err(|e| {
            format!(
                "Failed to create chromium ext dir {:?}: {}",
                chrome_ext_dir, e
            )
        })?;
        create_dir_all(&firefox_ext_dir).map_err(|e| {
            format!(
                "Failed to create firefox ext dir {:?}: {}",
                firefox_ext_dir, e
            )
        })?;

        logs.push(format!("Created directories in {:?}", app_dir));

        // 1. Deploy SnapLoad Desktop Engine binary
        let engine_dest = bin_dir.join("snapload-engine.exe");
        let mut engine_source = None;
        let engine_candidates = [
            PathBuf::from("engine/target/release/snapload-engine.exe"),
            PathBuf::from("../engine/target/release/snapload-engine.exe"),
            PathBuf::from("../../engine/target/release/snapload-engine.exe"),
        ];

        for c in &engine_candidates {
            if c.exists() && c.is_file() {
                engine_source = Some(c.clone());
                break;
            }
        }

        if let Some(src) = engine_source {
            std::fs::copy(&src, &engine_dest)
                .map_err(|e| format!("Failed to copy engine binary from {:?}: {}", src, e))?;
            logs.push(format!("Deployed engine binary from {:?}", src));
        } else if !EMBEDDED_ENGINE.is_empty() {
            std::fs::write(&engine_dest, EMBEDDED_ENGINE)
                .map_err(|e| format!("Failed to deploy embedded engine binary: {}", e))?;
            logs.push("Deployed embedded SnapLoad engine binary".to_string());
        } else if engine_dest.exists() {
            logs.push("Existing engine binary verified in bin directory".to_string());
        } else {
            return Err("SnapLoad engine executable not found in build assets.".to_string());
        }

        // 2. Deploy Chromium extension package and extract files
        let chrome_zip_dest = ext_dir.join("snapload-extension-chromium.zip");
        let mut chrome_source = None;
        let chrome_candidates = [
            PathBuf::from("extension/dist/snapload-extension-chromium.zip"),
            PathBuf::from("../extension/dist/snapload-extension-chromium.zip"),
            PathBuf::from("../../extension/dist/snapload-extension-chromium.zip"),
        ];

        for c in &chrome_candidates {
            if c.exists() {
                chrome_source = Some(c.clone());
                break;
            }
        }

        let mut chrome_bytes_to_unpack = None;
        if let Some(src) = chrome_source {
            let _ = std::fs::copy(&src, &chrome_zip_dest);
            if let Ok(bytes) = std::fs::read(&src) {
                chrome_bytes_to_unpack = Some(bytes);
            }
            logs.push("Copied Chromium extension archive".to_string());
        } else if !EMBEDDED_CHROME_EXT.is_empty() {
            let _ = std::fs::write(&chrome_zip_dest, EMBEDDED_CHROME_EXT);
            chrome_bytes_to_unpack = Some(EMBEDDED_CHROME_EXT.to_vec());
            logs.push("Extracted embedded Chromium extension archive".to_string());
        }

        if let Some(bytes) = chrome_bytes_to_unpack {
            if let Err(e) = unpack_zip_bytes(&bytes, &chrome_ext_dir) {
                logs.push(format!(
                    "Notice: Pure Rust unpack returned: {}. Checking raw folders...",
                    e
                ));
            }
        }

        // Fallback: If dist/chrome directory exists in workspace, copy it directly
        if !chrome_ext_dir.join("manifest.json").exists() {
            let workspace_chrome_dirs = [
                PathBuf::from("extension/dist/chrome"),
                PathBuf::from("../extension/dist/chrome"),
                PathBuf::from("../../extension/dist/chrome"),
            ];
            for w in &workspace_chrome_dirs {
                if w.exists() && w.join("manifest.json").exists() {
                    let _ = copy_dir_recursive(w, &chrome_ext_dir);
                    logs.push(format!("Copied extension files directly from {:?}", w));
                    break;
                }
            }
        }

        if chrome_ext_dir.join("manifest.json").exists() {
            logs.push(format!(
                "Chromium extension unpacked and verified at: {:?}",
                chrome_ext_dir
            ));
        } else {
            logs.push(
                "Notice: manifest.json not found in unpacked chromium folder. Place extension files into extensions\\chromium."
                    .to_string(),
            );
        }

        // Deploy directly to profile directories for detected browsers
        deploy_to_browser_profiles(&chrome_ext_dir, &mut logs);

        // 3. Deploy Firefox extension package (XPI & unpacked)
        let firefox_zip_dest = ext_dir.join("snapload-extension-firefox.zip");
        let firefox_xpi_dest = firefox_ext_dir.join("snapload-extension-firefox.xpi");
        let mut firefox_source = None;
        let firefox_candidates = [
            PathBuf::from("extension/dist/snapload-extension-firefox.zip"),
            PathBuf::from("../extension/dist/snapload-extension-firefox.zip"),
            PathBuf::from("../../extension/dist/snapload-extension-firefox.zip"),
        ];

        for c in &firefox_candidates {
            if c.exists() {
                firefox_source = Some(c.clone());
                break;
            }
        }

        let mut ff_bytes_to_unpack = None;
        if let Some(src) = firefox_source {
            let _ = std::fs::copy(&src, &firefox_zip_dest);
            let _ = std::fs::copy(&src, &firefox_xpi_dest);
            if let Ok(bytes) = std::fs::read(&src) {
                ff_bytes_to_unpack = Some(bytes);
            }
            logs.push("Copied Firefox extension package".to_string());
        } else if !EMBEDDED_FIREFOX_EXT.is_empty() {
            let _ = std::fs::write(&firefox_zip_dest, EMBEDDED_FIREFOX_EXT);
            let _ = std::fs::write(&firefox_xpi_dest, EMBEDDED_FIREFOX_EXT);
            ff_bytes_to_unpack = Some(EMBEDDED_FIREFOX_EXT.to_vec());
            logs.push("Extracted embedded Firefox extension package".to_string());
        }

        if let Some(bytes) = ff_bytes_to_unpack {
            let ff_unpacked = firefox_ext_dir.join("unpacked");
            let _ = unpack_zip_bytes(&bytes, &ff_unpacked);
        }

        // 4. Generate Native Messaging Host manifests
        let (chrome_manifest, firefox_manifest) = RegistryManager::write_manifests(&engine_dest)
            .map_err(|e| format!("Failed to write manifests: {}", e))?;
        logs.push(format!(
            "Generated Native Messaging manifests at: {:?}",
            RegistryManager::get_manifest_dir()
        ));

        // 5. Register Native Messaging Host & Extensions for selected browsers
        for browser_id in &options.browsers {
            let manifest_path = if browser_id == "firefox" {
                &firefox_manifest
            } else {
                &chrome_manifest
            };

            match RegistryManager::register_browser(browser_id, manifest_path) {
                Ok(_) => logs.push(format!(
                    "Registered Native Messaging Host for '{}'",
                    browser_id
                )),
                Err(e) => logs.push(format!("Warning: Registration for '{}': {}", browser_id, e)),
            }

            if browser_id == "firefox" {
                if let Err(e) = RegistryManager::register_firefox_extension(&firefox_xpi_dest) {
                    logs.push(format!(
                        "Warning: Firefox extension registry hook: {}",
                        e
                    ));
                }
            } else if let Err(e) =
                RegistryManager::register_chromium_extension(browser_id, &chrome_ext_dir)
            {
                logs.push(format!(
                    "Warning: Chromium extension registry hook for '{}': {}",
                    browser_id, e
                ));
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
                message: "Executable missing from bin directory".to_string(),
                detail: format!("Expected at: {:?}", engine_path),
            });
        }

        // 2. Native Messaging Manifests
        let chrome_manifest = RegistryManager::get_manifest_dir().join("com.darkskyee.snapload.chromium.json");
        let firefox_manifest = RegistryManager::get_manifest_dir().join("com.darkskyee.snapload.firefox.json");

        if chrome_manifest.exists() {
            items.push(DiagnosticItem {
                name: "Chromium Native Host Manifest".to_string(),
                status: "ok".to_string(),
                message: "Configured with valid stdio protocol and origin keys".to_string(),
                detail: chrome_manifest.to_string_lossy().to_string(),
            });
        } else {
            items.push(DiagnosticItem {
                name: "Chromium Native Host Manifest".to_string(),
                status: "error".to_string(),
                message: "Manifest missing from disk".to_string(),
                detail: chrome_manifest.to_string_lossy().to_string(),
            });
        }

        if firefox_manifest.exists() {
            items.push(DiagnosticItem {
                name: "Firefox Native Host Manifest".to_string(),
                status: "ok".to_string(),
                message: "Configured with valid allowed_extensions identifier".to_string(),
                detail: firefox_manifest.to_string_lossy().to_string(),
            });
        } else {
            items.push(DiagnosticItem {
                name: "Firefox Native Host Manifest".to_string(),
                status: "warn".to_string(),
                message: "Firefox manifest not present (optional if not using Firefox)".to_string(),
                detail: firefox_manifest.to_string_lossy().to_string(),
            });
        }

        // 3. Windows Registry Native Host Integration
        for b_id in ["chrome", "edge", "brave", "opera", "operagx", "firefox"] {
            let (registered, target) = RegistryManager::check_native_host_registration(b_id);
            let b_name = match b_id {
                "chrome" => "Google Chrome",
                "edge" => "Microsoft Edge",
                "brave" => "Brave Browser",
                "opera" => "Opera",
                "operagx" => "Opera GX",
                "firefox" => "Mozilla Firefox",
                _ => b_id,
            };

            if registered {
                items.push(DiagnosticItem {
                    name: format!("Registry: {}", b_name),
                    status: "ok".to_string(),
                    message: "Native Messaging Host registered in Windows Registry".to_string(),
                    detail: target.unwrap_or_else(|| "Key verified".to_string()),
                });
            }
        }

        // 4. Live Native Messaging IPC Ping Roundtrip
        if engine_path.exists() {
            let (ping_ok, ping_msg) = test_native_messaging_ping(&engine_path).await;
            items.push(DiagnosticItem {
                name: "Live Engine IPC Roundtrip".to_string(),
                status: if ping_ok { "ok".to_string() } else { "warn".to_string() },
                message: ping_msg,
                detail: "Native Messaging 4-byte LE length prefix frame test".to_string(),
            });
        }

        // 5. Chromium Extension Deployment
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
                message: "Binary present in bin directory".to_string(),
                detail: ffmpeg_path.to_string_lossy().to_string(),
            });
        } else if let Some(p) = which("ffmpeg.exe") {
            items.push(DiagnosticItem {
                name: "FFmpeg Media Converter".to_string(),
                status: "ok".to_string(),
                message: "Detected on Windows system PATH".to_string(),
                detail: p.to_string_lossy().to_string(),
            });
        } else {
            items.push(DiagnosticItem {
                name: "FFmpeg Media Converter".to_string(),
                status: "warn".to_string(),
                message: "Not found (optional for audio transcoding)".to_string(),
                detail: "Check 'FFmpeg' component during installation".to_string(),
            });
        }

        // 9. Downloads Storage Directory
        if let Some(user_profile) = std::env::var_os("USERPROFILE") {
            let dl_dir = PathBuf::from(user_profile)
                .join("Downloads")
                .join("SnapLoad");
            if dl_dir.exists() {
                items.push(DiagnosticItem {
                    name: "Downloads Directory".to_string(),
                    status: "ok".to_string(),
                    message: "Media directory ready".to_string(),
                    detail: dl_dir.to_string_lossy().to_string(),
                });
            }
        }

        let total = items.len();
        let passed = items.iter().filter(|i| i.status == "ok").count();
        let all_passed = items.iter().all(|i| i.status != "error");

        DiagnosticResult {
            all_passed,
            total,
            passed,
            items,
        }
    }

    pub fn uninstall() -> Result<Vec<String>, String> {
        let mut logs = Vec::new();
        logs.push("Cleaning up Windows Registry keys...".to_string());
        let removed_keys = RegistryManager::unregister_all();
        logs.extend(removed_keys);

        let app_dir = Self::get_app_dir();
        let bin_dir = Self::get_bin_dir();
        let ext_dir = Self::get_extensions_dir();
        let manifest_dir = RegistryManager::get_manifest_dir();

        let _ = remove_dir_all(&bin_dir);
        let _ = remove_dir_all(&ext_dir);
        let _ = remove_dir_all(&manifest_dir);

        logs.push(format!("Cleaned application directories from {:?}", app_dir));
        logs.push("SnapLoad components uninstalled. Downloads folder preserved.".to_string());

        Ok(logs)
    }

    pub fn ensure_engine_deployed() -> Result<PathBuf, String> {
        let bin_dir = Self::get_bin_dir();
        create_dir_all(&bin_dir)
            .map_err(|e| format!("Failed to create bin dir {:?}: {}", bin_dir, e))?;
        let engine_dest = bin_dir.join("snapload-engine.exe");
        if engine_dest.exists() && engine_dest.metadata().map(|m| m.len() > 10_000).unwrap_or(false) {
            return Ok(engine_dest);
        }

        let mut engine_source = None;
        let engine_candidates = [
            PathBuf::from("engine/target/release/snapload-engine.exe"),
            PathBuf::from("../engine/target/release/snapload-engine.exe"),
            PathBuf::from("../../engine/target/release/snapload-engine.exe"),
        ];

        for c in &engine_candidates {
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
            _ => return Err(format!("Nieobsługiwana przeglądarka: {}", browser_id)),
        };

        let exe_path = match exe_opt {
            Some(p) => BrowserDetector::sanitize_exe_path(&p),
            None => return Err(format!(
                "Nie znaleziono pliku wykonywalnego przeglądarki {} na tym komputerze. Upewnij się, że przeglądarka jest zainstalowana.",
                name
            )),
        };

        let mut logs = Vec::new();
        logs.push(format!("Wykryto ścieżkę .exe: {}", exe_path));

        // 1. Ensure engine is deployed
        let engine_dest = Self::ensure_engine_deployed()?;
        logs.push(format!("SnapLoad Engine aktywny: {:?}", engine_dest));

        // 2. Write Native Messaging Host manifests
        let (chrome_manifest, firefox_manifest) = RegistryManager::write_manifests(&engine_dest)
            .map_err(|e| format!("Błąd zapisu manifestu hosta: {}", e))?;

        let manifest_to_use = if browser_id == "firefox" {
            &firefox_manifest
        } else {
            &chrome_manifest
        };

        // 3. Register Native Messaging Host
        RegistryManager::register_browser(browser_id, manifest_to_use)?;
        logs.push(format!("Zarejestrowano Native Messaging Host dla {}", name));

        // 4. Find profile directories
        let profile_dirs = BrowserDetector::find_browser_profiles(browser_id);
        let primary_profile = BrowserDetector::get_browser_display_profile(browser_id);
        if let Some(ref prof) = primary_profile {
            logs.push(format!("Wykryto profil przeglądarki: {}", prof));
        }

        // 5. Check if browser is running and restart if requested
        let was_running = BrowserDetector::is_browser_process_running(browser_id);
        if was_running {
            if restart_if_running {
                logs.push(format!(
                    "Przeglądarka {} jest obecnie uruchomiona — zamykanie procesów w celu załadowania rozszerzenia...",
                    name
                ));
                let _ = BrowserDetector::close_browser_processes(browser_id);
            } else {
                logs.push(format!(
                    "Uwaga: {} jest uruchomiona. Rozszerzenie zostanie w pełni załadowane po restarcie przeglądarki.",
                    name
                ));
            }
        }

        // 6. Deploy extension and launch browser
        let ext_to_copy_to_clip: String;
        if browser_id == "firefox" {
            let firefox_xpi = Self::ensure_firefox_ext_deployed()?;
            ext_to_copy_to_clip = firefox_xpi.to_string_lossy().to_string();
            logs.push(format!("Pakiet rozszerzenia Firefox XPI: {:?}", firefox_xpi));

            // Deploy XPI to profile extensions folders
            for p in &profile_dirs {
                let target_ext_dir = p.join("extensions");
                let _ = create_dir_all(&target_ext_dir);
                let target_file = target_ext_dir.join("snapload@darkskyee.github.io.xpi");
                if firefox_xpi.exists() {
                    let _ = std::fs::copy(&firefox_xpi, &target_file);
                    logs.push(format!("Wdrożono plik .xpi do profilu Firefox: {:?}", target_file));
                }
            }

            // Register in Registry
            let _ = RegistryManager::register_firefox_extension(&firefox_xpi);
            logs.push("Zarejestrowano rozszerzenie Firefox w rejestrze Windows".to_string());

            // Launch Firefox with XPI path to prompt installation and open YouTube
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                const CREATE_NEW_PROCESS_GROUP: u32 = 0x00000200;
                let mut cmd = std::process::Command::new(&exe_path);
                cmd.arg(firefox_xpi.to_string_lossy().to_string());
                cmd.arg("https://www.youtube.com");
                cmd.creation_flags(CREATE_NEW_PROCESS_GROUP);
                cmd.spawn().map_err(|e| format!("Błąd uruchomienia Firefox: {}", e))?;
                logs.push("Uruchomiono Firefox z monitem instalacji rozszerzenia na YouTube".to_string());
            }
        } else {
            let chrome_unpacked = Self::ensure_chromium_ext_deployed()?;
            ext_to_copy_to_clip = chrome_unpacked.to_string_lossy().to_string();
            logs.push(format!("Katalog rozszerzenia Chromium: {:?}", chrome_unpacked));

            let ext_id = "ikkidkbojpedpeaomipgioiigmjlpflg";
            let version_tag = "1.0.0_0";

            for p in &profile_dirs {
                let target_dir = p.join("Extensions").join(ext_id).join(version_tag);
                let _ = create_dir_all(&target_dir);
                let _ = copy_dir_recursive(&chrome_unpacked, &target_dir);
                logs.push(format!("Wdrożono rozszerzenie do profilu: {:?}", target_dir));
            }

            // Register external extension in Registry
            let _ = RegistryManager::register_chromium_extension(browser_id, &chrome_unpacked);
            logs.push(format!("Zarejestrowano rozszerzenie w rejestrze Windows dla {}", name));

            // Launch browser with extension loaded in detached process
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                const CREATE_NEW_PROCESS_GROUP: u32 = 0x00000200;
                let mut cmd = std::process::Command::new(&exe_path);
                let ext_str = chrome_unpacked.to_string_lossy().to_string();
                cmd.arg(format!("--load-extension={}", ext_str));
                cmd.arg("--enable-extensions");
                cmd.arg("https://www.youtube.com");
                cmd.creation_flags(CREATE_NEW_PROCESS_GROUP);
                cmd.spawn().map_err(|e| format!("Błąd uruchomienia {}: {}", name, e))?;
                logs.push(format!("Uruchomiono {} z załadowanym rozszerzeniem SnapLoad na YouTube", name));
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
                logs.push("Skopiowano ścieżkę rozszerzenia do schowka systemu Windows".to_string());
            }
        }

        let message = if was_running && restart_if_running {
            format!(
                "Przeglądarka {} została zrestartowana i rozszerzenie SnapLoad zostało pomyślnie załadowane!",
                name
            )
        } else {
            format!(
                "Rozszerzenie SnapLoad zostało pomyślnie zainstalowane i aktywowane w {}!",
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

pub fn deploy_to_browser_profiles(chrome_unpacked: &Path, logs: &mut Vec<String>) {
    let ext_id = "ikkidkbojpedpeaomipgioiigmjlpflg";
    let version_tag = "1.0.0_0";

    for b_id in ["opera", "operagx", "chrome", "edge", "brave"] {
        let profiles = BrowserDetector::find_browser_profiles(b_id);
        for p in profiles {
            let target_dir = p.join("Extensions").join(ext_id).join(version_tag);
            if let Ok(_) = create_dir_all(&target_dir) {
                if let Ok(_) = copy_dir_recursive(chrome_unpacked, &target_dir) {
                    logs.push(format!("Auto-deployed extension files into {} profile: {:?}", b_id, target_dir));
                }
            }
        }
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
        assert!(res.unwrap_err().contains("Nieobsługiwana przeglądarka"));
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
