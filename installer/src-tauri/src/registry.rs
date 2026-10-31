use std::fs::{create_dir_all, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use winreg::enums::*;
use winreg::RegKey;

pub const HOST_NAME: &str = "com.darkskyee.snapload";

pub struct RegistryManager;

impl RegistryManager {
    pub fn get_manifest_dir() -> PathBuf {
        let local_app_data = std::env::var_os("LOCALAPPDATA").expect("LOCALAPPDATA not found");
        let mut path = PathBuf::from(local_app_data);
        path.push("SnapLoad");
        path.push("manifests");
        let _ = create_dir_all(&path);
        path
    }

    pub fn write_manifests(engine_exe_path: &Path) -> Result<(PathBuf, PathBuf), std::io::Error> {
        let dir = Self::get_manifest_dir();
        let engine_path_escaped = engine_exe_path.to_string_lossy().to_string();

        // 1. Chromium Manifest (Chrome, Edge, Brave, Opera)
        let chrome_manifest = serde_json::json!({
            "name": HOST_NAME,
            "description": "SnapLoad Desktop Downloader Native Messaging Host",
            "path": engine_path_escaped,
            "type": "stdio",
            "allowed_origins": [
                "chrome-extension://*/"
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
        let subkey = match browser_id {
            "chrome" => format!(r"Software\Google\Chrome\NativeMessagingHosts\{}", HOST_NAME),
            "edge" => format!(r"Software\Microsoft\Edge\NativeMessagingHosts\{}", HOST_NAME),
            "brave" => format!(r"Software\BraveSoftware\Brave-Browser\NativeMessagingHosts\{}", HOST_NAME),
            "opera" => format!(r"Software\Opera Software\Opera Stable\NativeMessagingHosts\{}", HOST_NAME),
            "firefox" => format!(r"Software\Mozilla\NativeMessagingHosts\{}", HOST_NAME),
            _ => return Err(format!("Unsupported browser id: {}", browser_id)),
        };

        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let (key, _) = hkcu.create_subkey(&subkey)
            .map_err(|e| format!("Failed to create registry subkey {}: {}", subkey, e))?;

        let manifest_str = manifest_path.to_string_lossy().to_string();
        key.set_value("", &manifest_str)
            .map_err(|e| format!("Failed to set default registry value for {}: {}", subkey, e))?;

        Ok(())
    }

    pub fn unregister_all() -> Vec<String> {
        let mut removed = Vec::new();
        let subkeys = [
            format!(r"Software\Google\Chrome\NativeMessagingHosts\{}", HOST_NAME),
            format!(r"Software\Microsoft\Edge\NativeMessagingHosts\{}", HOST_NAME),
            format!(r"Software\BraveSoftware\Brave-Browser\NativeMessagingHosts\{}", HOST_NAME),
            format!(r"Software\Opera Software\Opera Stable\NativeMessagingHosts\{}", HOST_NAME),
            format!(r"Software\Mozilla\NativeMessagingHosts\{}", HOST_NAME),
        ];

        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        for sub in &subkeys {
            if hkcu.delete_subkey_all(sub).is_ok() {
                removed.push(sub.clone());
            }
        }
        removed
    }
}
