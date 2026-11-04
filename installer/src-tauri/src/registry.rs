use std::fs::{create_dir_all, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use winreg::enums::*;
use winreg::RegKey;

pub const HOST_NAME: &str = "com.darkskyee.snapload";

pub struct RegistryManager;

impl RegistryManager {
    pub fn get_manifest_dir() -> PathBuf {
        let local_app_data = std::env::var_os("LOCALAPPDATA")
            .or_else(|| std::env::var_os("APPDATA"))
            .unwrap_or_else(|| std::ffi::OsString::from("."));
        let mut path = PathBuf::from(local_app_data);
        path.push("SnapLoad");
        path.push("manifests");
        let _ = create_dir_all(&path);
        path
    }

    pub fn write_manifests(engine_exe_path: &Path) -> Result<(PathBuf, PathBuf), std::io::Error> {
        let dir = Self::get_manifest_dir();
        let engine_path_escaped = engine_exe_path.to_string_lossy().to_string();

        // 1. Chromium Manifest (Chrome, Edge, Brave, Opera, Opera GX)
        let chrome_manifest = serde_json::json!({
            "name": HOST_NAME,
            "description": "SnapLoad Desktop Downloader Native Messaging Host",
            "path": engine_path_escaped,
            "type": "stdio",
            "allowed_origins": [
                "chrome-extension://ikkidkbojpedpeaomipgioiigmjlpflg/",
                "extension://ikkidkbojpedpeaomipgioiigmjlpflg/",
                "opera-extension://ikkidkbojpedpeaomipgioiigmjlpflg/",
                "chrome-extension://fmkfjmpdcflidoafkijcnafjmmdcjfck/",
                "chrome-extension://ohoalmkigaipobmpanfbceocpiciajgg/"
            ]
        });

        let chrome_path = dir.join("com.darkskyee.snapload.chromium.json");
        let mut f1 = File::create(&chrome_path)?;
        f1.write_all(serde_json::to_string_pretty(&chrome_manifest)?.as_bytes())?;

        // 2. Firefox Manifest
        let firefox_manifest = serde_json::json!({
            "name": HOST_NAME,
            "description": "SnapLoad Desktop Downloader Native Messaging Host",
            "path": engine_path_escaped,
            "type": "stdio",
            "allowed_extensions": [
                "snapload@darkskyee.github.io"
            ]
        });

        let firefox_path = dir.join("com.darkskyee.snapload.firefox.json");
        let mut f2 = File::create(&firefox_path)?;
        f2.write_all(serde_json::to_string_pretty(&firefox_manifest)?.as_bytes())?;

        Ok((chrome_path, firefox_path))
    }

    pub fn register_browser(browser_id: &str, manifest_path: &Path) -> Result<(), String> {
        let subkeys = match browser_id {
            "chrome" => vec![format!(r"Software\Google\Chrome\NativeMessagingHosts\{}", HOST_NAME)],
            "edge" => vec![
                format!(r"Software\Microsoft\Edge\NativeMessagingHosts\{}", HOST_NAME),
                format!(r"Software\Google\Chrome\NativeMessagingHosts\{}", HOST_NAME),
            ],
            "brave" => vec![
                format!(r"Software\BraveSoftware\Brave-Browser\NativeMessagingHosts\{}", HOST_NAME),
                format!(r"Software\Google\Chrome\NativeMessagingHosts\{}", HOST_NAME),
            ],
            "opera" => vec![
                format!(r"Software\Opera Software\Opera Stable\NativeMessagingHosts\{}", HOST_NAME),
                format!(r"Software\Opera Software\NativeMessagingHosts\{}", HOST_NAME),
                format!(r"Software\Google\Chrome\NativeMessagingHosts\{}", HOST_NAME),
            ],
            "operagx" => vec![
                format!(r"Software\Opera Software\Opera GX Stable\NativeMessagingHosts\{}", HOST_NAME),
                format!(r"Software\Opera Software\NativeMessagingHosts\{}", HOST_NAME),
                format!(r"Software\Google\Chrome\NativeMessagingHosts\{}", HOST_NAME),
            ],
            "firefox" => vec![format!(r"Software\Mozilla\NativeMessagingHosts\{}", HOST_NAME)],
            _ => return Err(format!("Unsupported browser id: {}", browser_id)),
        };

        let manifest_str = manifest_path.to_string_lossy().to_string();
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);

        for subkey in subkeys {
            // Register in HKCU (user level)
            if let Ok((key, _)) = hkcu.create_subkey(&subkey) {
                let _ = key.set_value("", &manifest_str);
            }
            // Also attempt HKLM (machine level if elevated)
            if let Ok((key, _)) = hklm.create_subkey(&subkey) {
                let _ = key.set_value("", &manifest_str);
            }
        }

        Ok(())
    }

    pub fn register_firefox_extension(xpi_path: &Path) -> Result<(), String> {
        let subkey = r"Software\Mozilla\Firefox\Extensions";
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let (key, _) = hkcu
            .create_subkey(subkey)
            .map_err(|e| format!("Failed to create Firefox Extensions registry key: {}", e))?;
        let xpi_str = xpi_path.to_string_lossy().to_string();
        key.set_value("snapload@darkskyee.github.io", &xpi_str)
            .map_err(|e| format!("Failed to set Firefox Extension value: {}", e))?;

        // Also register in Enterprise Policies for silent auto-installation
        let policy_subkey = r"Software\Policies\Mozilla\Firefox\ExtensionSettings\snapload@darkskyee.github.io";
        if let Ok((pol_key, _)) = hkcu.create_subkey(policy_subkey) {
            let file_url = format!("file:///{}", xpi_str.replace('\\', "/"));
            let _ = pol_key.set_value("installation_mode", &"normal_installed");
            let _ = pol_key.set_value("install_url", &file_url);
        }

        Ok(())
    }

    pub fn register_chromium_extension(browser_id: &str, ext_path: &Path) -> Result<(), String> {
        let ext_id = "ikkidkbojpedpeaomipgioiigmjlpflg";
        let subkeys = match browser_id {
            "chrome" => vec![format!(r"Software\Google\Chrome\Extensions\{}", ext_id)],
            "edge" => vec![format!(r"Software\Microsoft\Edge\Extensions\{}", ext_id)],
            "brave" => vec![format!(r"Software\BraveSoftware\Brave-Browser\Extensions\{}", ext_id)],
            "opera" => vec![
                format!(r"Software\Opera Software\Opera Stable\Extensions\{}", ext_id),
                format!(r"Software\Opera Software\Extensions\{}", ext_id),
            ],
            "operagx" => vec![
                format!(r"Software\Opera Software\Opera GX Stable\Extensions\{}", ext_id),
            ],
            _ => return Ok(()),
        };

        let path_str = ext_path.to_string_lossy().to_string();
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);

        for subkey in subkeys {
            if let Ok((key, _)) = hkcu.create_subkey(&subkey) {
                let _ = key.set_value("path", &path_str);
                let _ = key.set_value("version", &"1.0.0");
            }
            if let Ok((key, _)) = hklm.create_subkey(&subkey) {
                let _ = key.set_value("path", &path_str);
                let _ = key.set_value("version", &"1.0.0");
            }
        }
        Ok(())
    }

    pub fn check_native_host_registration(browser_id: &str) -> (bool, Option<String>) {
        let subkeys = match browser_id {
            "chrome" => vec![format!(r"Software\Google\Chrome\NativeMessagingHosts\{}", HOST_NAME)],
            "edge" => vec![format!(r"Software\Microsoft\Edge\NativeMessagingHosts\{}", HOST_NAME)],
            "brave" => vec![format!(r"Software\BraveSoftware\Brave-Browser\NativeMessagingHosts\{}", HOST_NAME)],
            "opera" => vec![
                format!(r"Software\Opera Software\Opera Stable\NativeMessagingHosts\{}", HOST_NAME),
                format!(r"Software\Opera Software\NativeMessagingHosts\{}", HOST_NAME),
            ],
            "operagx" => vec![
                format!(r"Software\Opera Software\Opera GX Stable\NativeMessagingHosts\{}", HOST_NAME),
            ],
            "firefox" => vec![format!(r"Software\Mozilla\NativeMessagingHosts\{}", HOST_NAME)],
            _ => return (false, None),
        };

        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);

        for subkey in subkeys {
            if let Ok(key) = hkcu.open_subkey(&subkey) {
                if let Ok(val) = key.get_value::<String, _>("") {
                    return (true, Some(val));
                }
                return (true, None);
            }
            if let Ok(key) = hklm.open_subkey(&subkey) {
                if let Ok(val) = key.get_value::<String, _>("") {
                    return (true, Some(val));
                }
                return (true, None);
            }
        }
        (false, None)
    }

    pub fn check_firefox_extension_registered() -> bool {
        let subkey = r"Software\Mozilla\Firefox\Extensions";
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        if let Ok(key) = hkcu.open_subkey(subkey) {
            return key.get_value::<String, _>("snapload@darkskyee.github.io").is_ok();
        }
        false
    }

    pub fn unregister_all() -> Vec<String> {
        let mut removed = Vec::new();
        let subkeys = [
            format!(r"Software\Google\Chrome\NativeMessagingHosts\{}", HOST_NAME),
            format!(r"Software\Microsoft\Edge\NativeMessagingHosts\{}", HOST_NAME),
            format!(r"Software\BraveSoftware\Brave-Browser\NativeMessagingHosts\{}", HOST_NAME),
            format!(r"Software\Opera Software\Opera Stable\NativeMessagingHosts\{}", HOST_NAME),
            format!(r"Software\Opera Software\NativeMessagingHosts\{}", HOST_NAME),
            format!(r"Software\Opera Software\Opera GX Stable\NativeMessagingHosts\{}", HOST_NAME),
            format!(r"Software\Mozilla\NativeMessagingHosts\{}", HOST_NAME),
            format!(r"Software\Google\Chrome\Extensions\ikkidkbojpedpeaomipgioiigmjlpflg"),
            format!(r"Software\Microsoft\Edge\Extensions\ikkidkbojpedpeaomipgioiigmjlpflg"),
            format!(r"Software\BraveSoftware\Brave-Browser\Extensions\ikkidkbojpedpeaomipgioiigmjlpflg"),
            format!(r"Software\Opera Software\Opera Stable\Extensions\ikkidkbojpedpeaomipgioiigmjlpflg"),
            format!(r"Software\Opera Software\Extensions\ikkidkbojpedpeaomipgioiigmjlpflg"),
            format!(r"Software\Opera Software\Opera GX Stable\Extensions\ikkidkbojpedpeaomipgioiigmjlpflg"),
        ];

        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        for sub in &subkeys {
            if hkcu.delete_subkey_all(sub).is_ok() {
                removed.push(sub.clone());
            }
        }

        // Clean Firefox extension value
        if let Ok(key) = hkcu.open_subkey(r"Software\Mozilla\Firefox\Extensions") {
            let _ = key.delete_value("snapload@darkskyee.github.io");
            removed.push("Firefox Extension Registration".to_string());
        }

        removed
    }
}
