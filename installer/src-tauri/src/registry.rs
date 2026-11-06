use std::fs::{create_dir_all, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use winreg::enums::*;
use winreg::RegKey;

pub const HOST_NAME: &str = "com.darkskyee.snapload";

pub struct RegistryManager;

fn path_to_extension_id(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let hash = hasher.finalize();
    let mut id = String::with_capacity(32);
    for &b in &hash[0..16] {
        id.push((b'a' + (b >> 4)) as char);
        id.push((b'a' + (b & 0x0f)) as char);
    }
    id
}

pub fn compute_path_extension_ids(dir: &Path) -> Vec<String> {
    let mut ids = Vec::new();
    let path_str = dir.to_string_lossy().to_string();
    let variations = [
        path_str.clone(),
        path_str.to_lowercase(),
        path_str.replace('\\', "/"),
        path_str.to_lowercase().replace('\\', "/"),
    ];
    for v in variations {
        // Chromium on Windows uses UTF-16LE bytes of the file path
        let utf16_bytes: Vec<u8> = v.encode_utf16().flat_map(|u| u.to_le_bytes()).collect();
        ids.push(path_to_extension_id(&utf16_bytes));
        // Fallback UTF-8 bytes
        ids.push(path_to_extension_id(v.as_bytes()));
    }
    ids.sort();
    ids.dedup();
    ids
}

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
        create_dir_all(&dir)?;
        let engine_path_escaped = engine_exe_path.to_string_lossy().to_string();

        let local_app_data = std::env::var_os("LOCALAPPDATA")
            .or_else(|| std::env::var_os("APPDATA"))
            .unwrap_or_else(|| std::ffi::OsString::from("."));
        let chrom_dir = PathBuf::from(local_app_data)
            .join("SnapLoad")
            .join("extensions")
            .join("chromium");

        let mut allowed_origins = vec![
            "chrome-extension://ikkidkbojpedpeaomipgioiigmjlpflg/".to_string(),
            "chrome-extension://fmkfjmpdcflidoafkijcnafjmmdcjfck/".to_string(),
            "chrome-extension://ohoalmkigaipobmpanfbceocpiciajgg/".to_string(),
        ];

        // Add dynamic path-derived unpacked extension IDs (Chromium requires chrome-extension:// format)
        for id in compute_path_extension_ids(&chrom_dir) {
            allowed_origins.push(format!("chrome-extension://{}/", id));
        }

        allowed_origins.sort();
        allowed_origins.dedup();

        // 1. Chromium Manifest (Chrome, Edge, Brave, Opera, Opera GX)
        let chrome_manifest = serde_json::json!({
            "name": HOST_NAME,
            "description": "SnapLoad Desktop Downloader Native Messaging Host",
            "path": engine_path_escaped,
            "type": "stdio",
            "allowed_origins": allowed_origins
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

    pub fn get_subkeys_for_browser(browser_id: &str) -> Vec<String> {
        match browser_id {
            "chrome" => vec![
                format!(r"Software\Google\Chrome\NativeMessagingHosts\{}", HOST_NAME),
                format!(r"Software\Chromium\NativeMessagingHosts\{}", HOST_NAME),
                format!(r"Software\WOW6432Node\Google\Chrome\NativeMessagingHosts\{}", HOST_NAME),
                format!(r"Software\WOW6432Node\Chromium\NativeMessagingHosts\{}", HOST_NAME),
            ],
            "edge" => vec![
                format!(r"Software\Microsoft\Edge\NativeMessagingHosts\{}", HOST_NAME),
                format!(r"Software\Google\Chrome\NativeMessagingHosts\{}", HOST_NAME),
                format!(r"Software\WOW6432Node\Microsoft\Edge\NativeMessagingHosts\{}", HOST_NAME),
            ],
            "brave" => vec![
                format!(r"Software\BraveSoftware\Brave-Browser\NativeMessagingHosts\{}", HOST_NAME),
                format!(r"Software\Google\Chrome\NativeMessagingHosts\{}", HOST_NAME),
                format!(r"Software\WOW6432Node\BraveSoftware\Brave-Browser\NativeMessagingHosts\{}", HOST_NAME),
            ],
            "opera" => vec![
                format!(r"Software\Opera Software\NativeMessagingHosts\{}", HOST_NAME),
                format!(r"Software\Opera Software\Opera Stable\NativeMessagingHosts\{}", HOST_NAME),
                format!(r"Software\Opera Software\Opera GX Stable\NativeMessagingHosts\{}", HOST_NAME),
                format!(r"Software\Opera Software\Opera GX\NativeMessagingHosts\{}", HOST_NAME),
                format!(r"Software\Opera Software\Opera Developer\NativeMessagingHosts\{}", HOST_NAME),
                format!(r"Software\Opera Software\Opera Next\NativeMessagingHosts\{}", HOST_NAME),
                format!(r"Software\Google\Chrome\NativeMessagingHosts\{}", HOST_NAME),
                format!(r"Software\Chromium\NativeMessagingHosts\{}", HOST_NAME),
                format!(r"Software\WOW6432Node\Opera Software\NativeMessagingHosts\{}", HOST_NAME),
                format!(r"Software\WOW6432Node\Opera Software\Opera Stable\NativeMessagingHosts\{}", HOST_NAME),
                format!(r"Software\WOW6432Node\Opera Software\Opera GX Stable\NativeMessagingHosts\{}", HOST_NAME),
                format!(r"Software\WOW6432Node\Google\Chrome\NativeMessagingHosts\{}", HOST_NAME),
            ],
            "operagx" => vec![
                format!(r"Software\Opera Software\Opera GX Stable\NativeMessagingHosts\{}", HOST_NAME),
                format!(r"Software\Opera Software\Opera GX\NativeMessagingHosts\{}", HOST_NAME),
                format!(r"Software\Opera Software\NativeMessagingHosts\{}", HOST_NAME),
                format!(r"Software\Opera Software\Opera Stable\NativeMessagingHosts\{}", HOST_NAME),
                format!(r"Software\Google\Chrome\NativeMessagingHosts\{}", HOST_NAME),
                format!(r"Software\Chromium\NativeMessagingHosts\{}", HOST_NAME),
                format!(r"Software\WOW6432Node\Opera Software\Opera GX Stable\NativeMessagingHosts\{}", HOST_NAME),
                format!(r"Software\WOW6432Node\Opera Software\Opera GX\NativeMessagingHosts\{}", HOST_NAME),
                format!(r"Software\WOW6432Node\Opera Software\NativeMessagingHosts\{}", HOST_NAME),
                format!(r"Software\WOW6432Node\Google\Chrome\NativeMessagingHosts\{}", HOST_NAME),
            ],
            "firefox" => vec![
                format!(r"Software\Mozilla\NativeMessagingHosts\{}", HOST_NAME),
                format!(r"Software\WOW6432Node\Mozilla\NativeMessagingHosts\{}", HOST_NAME),
            ],
            _ => vec![],
        }
    }

    pub fn register_browser(browser_id: &str, manifest_path: &Path) -> Result<(), String> {
        let subkeys = Self::get_subkeys_for_browser(browser_id);
        if subkeys.is_empty() {
            return Err(format!("Unsupported browser id: {}", browser_id));
        }

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

    pub fn register_all_chromium_hosts(manifest_path: &Path) {
        for b in &["chrome", "edge", "brave", "opera", "operagx"] {
            let _ = Self::register_browser(b, manifest_path);
        }
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
        let path_str = ext_path.to_string_lossy().trim().to_string();
        if path_str.is_empty() || path_str.eq_ignore_ascii_case("dummy") || path_str.contains("dummy") {
            return Err("Invalid extension path: cannot be empty or 'dummy'".to_string());
        }

        if !ext_path.is_absolute() {
            return Err(format!("Extension path must be absolute: {:?}", ext_path));
        }

        let manifest = ext_path.join("manifest.json");
        if !manifest.exists() {
            return Err(format!("manifest.json not found in extension path: {:?}", ext_path));
        }

        let ext_id = "ikkidkbojpedpeaomipgioiigmjlpflg";
        let subkeys = match browser_id {
            "chrome" => vec![
                format!(r"Software\Google\Chrome\Extensions\{}", ext_id),
                format!(r"Software\WOW6432Node\Google\Chrome\Extensions\{}", ext_id),
            ],
            "edge" => vec![
                format!(r"Software\Microsoft\Edge\Extensions\{}", ext_id),
                format!(r"Software\WOW6432Node\Microsoft\Edge\Extensions\{}", ext_id),
            ],
            "brave" => vec![
                format!(r"Software\BraveSoftware\Brave-Browser\Extensions\{}", ext_id),
                format!(r"Software\WOW6432Node\BraveSoftware\Brave-Browser\Extensions\{}", ext_id),
            ],
            "opera" => vec![
                format!(r"Software\Opera Software\Opera Stable\Extensions\{}", ext_id),
                format!(r"Software\Opera Software\Extensions\{}", ext_id),
                format!(r"Software\Google\Chrome\Extensions\{}", ext_id),
                format!(r"Software\WOW6432Node\Opera Software\Opera Stable\Extensions\{}", ext_id),
                format!(r"Software\WOW6432Node\Opera Software\Extensions\{}", ext_id),
                format!(r"Software\WOW6432Node\Google\Chrome\Extensions\{}", ext_id),
            ],
            "operagx" => vec![
                format!(r"Software\Opera Software\Opera GX Stable\Extensions\{}", ext_id),
                format!(r"Software\Opera Software\Opera GX\Extensions\{}", ext_id),
                format!(r"Software\Opera Software\Extensions\{}", ext_id),
                format!(r"Software\Google\Chrome\Extensions\{}", ext_id),
                format!(r"Software\WOW6432Node\Opera Software\Opera GX Stable\Extensions\{}", ext_id),
                format!(r"Software\WOW6432Node\Opera Software\Opera GX\Extensions\{}", ext_id),
                format!(r"Software\WOW6432Node\Opera Software\Extensions\{}", ext_id),
                format!(r"Software\WOW6432Node\Google\Chrome\Extensions\{}", ext_id),
            ],
            _ => return Ok(()),
        };

        let version_str = std::fs::read_to_string(&manifest)
            .ok()
            .and_then(|c| serde_json::from_str::<serde_json::Value>(&c).ok())
            .and_then(|v| v.get("version").and_then(|s| s.as_str()).map(|s| s.to_string()))
            .unwrap_or_else(|| "1.0.0".to_string());

        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);

        for subkey in subkeys {
            if let Ok((key, _)) = hkcu.create_subkey(&subkey) {
                let _ = key.set_value("path", &path_str);
                let _ = key.set_value("version", &version_str);
            }
            if let Ok((key, _)) = hklm.create_subkey(&subkey) {
                let _ = key.set_value("path", &path_str);
                let _ = key.set_value("version", &version_str);
            }
        }
        Ok(())
    }

    pub fn register_chromium_policy_allowlist(browser_id: &str) -> Result<(), String> {
        let ext_id = "ikkidkbojpedpeaomipgioiigmjlpflg";
        let subkeys = match browser_id {
            "chrome" => vec![
                r"Software\Policies\Google\Chrome\ExtensionInstallAllowlist".to_string(),
            ],
            "edge" => vec![
                r"Software\Policies\Microsoft\Edge\ExtensionInstallAllowlist".to_string(),
            ],
            "brave" => vec![
                r"Software\Policies\BraveSoftware\Brave\ExtensionInstallAllowlist".to_string(),
            ],
            "opera" => vec![
                r"Software\Policies\Opera Software\ExtensionInstallAllowlist".to_string(),
                r"Software\Policies\Opera Software\Opera Stable\ExtensionInstallAllowlist".to_string(),
            ],
            "operagx" => vec![
                r"Software\Policies\Opera Software\ExtensionInstallAllowlist".to_string(),
                r"Software\Policies\Opera Software\Opera GX Stable\ExtensionInstallAllowlist".to_string(),
            ],
            _ => vec![],
        };

        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);

        for subkey in subkeys {
            if let Ok((key, _)) = hkcu.create_subkey(&subkey) {
                let _ = key.set_value("1", &ext_id);
            }
            if let Ok((key, _)) = hklm.create_subkey(&subkey) {
                let _ = key.set_value("1", &ext_id);
            }
        }
        Ok(())
    }

    pub fn register_chromium_policy_sources(browser_id: &str) -> Result<(), String> {
        let subkeys = match browser_id {
            "chrome" => vec![
                r"Software\Policies\Google\Chrome\ExtensionInstallSources".to_string(),
            ],
            "edge" => vec![
                r"Software\Policies\Microsoft\Edge\ExtensionInstallSources".to_string(),
            ],
            "brave" => vec![
                r"Software\Policies\BraveSoftware\Brave\ExtensionInstallSources".to_string(),
            ],
            "opera" => vec![
                r"Software\Policies\Opera Software\ExtensionInstallSources".to_string(),
                r"Software\Policies\Opera Software\Opera Stable\ExtensionInstallSources".to_string(),
            ],
            "operagx" => vec![
                r"Software\Policies\Opera Software\ExtensionInstallSources".to_string(),
                r"Software\Policies\Opera Software\Opera GX Stable\ExtensionInstallSources".to_string(),
            ],
            _ => vec![],
        };

        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);

        for subkey in subkeys {
            if let Ok((key, _)) = hkcu.create_subkey(&subkey) {
                let _ = key.set_value("1", &"<all_urls>");
                let _ = key.set_value("2", &"file:///*");
            }
            if let Ok((key, _)) = hklm.create_subkey(&subkey) {
                let _ = key.set_value("1", &"<all_urls>");
                let _ = key.set_value("2", &"file:///*");
            }
        }
        Ok(())
    }

    pub fn register_chromium_developer_mode_policy(browser_id: &str) -> Result<(), String> {
        let subkeys = match browser_id {
            "chrome" => vec![r"Software\Policies\Google\Chrome".to_string()],
            "edge" => vec![r"Software\Policies\Microsoft\Edge".to_string()],
            "brave" => vec![r"Software\Policies\BraveSoftware\Brave".to_string()],
            "opera" => vec![
                r"Software\Policies\Opera Software".to_string(),
                r"Software\Policies\Opera Software\Opera Stable".to_string(),
            ],
            "operagx" => vec![
                r"Software\Policies\Opera Software".to_string(),
                r"Software\Policies\Opera Software\Opera GX Stable".to_string(),
            ],
            _ => vec![],
        };

        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);

        for subkey in subkeys {
            if let Ok((key, _)) = hkcu.create_subkey(&subkey) {
                let _ = key.set_value("DeveloperModeAvailability", &1u32);
            }
            if let Ok((key, _)) = hklm.create_subkey(&subkey) {
                let _ = key.set_value("DeveloperModeAvailability", &1u32);
            }
        }
        Ok(())
    }

    pub fn register_chromium_policy_forcelist(browser_id: &str, items: &[&str]) -> Result<(), String> {
        if items.is_empty() {
            return Ok(());
        }

        let subkeys = match browser_id {
            "chrome" => vec![
                r"Software\Policies\Google\Chrome\ExtensionInstallForcelist".to_string(),
            ],
            "edge" => vec![
                r"Software\Policies\Microsoft\Edge\ExtensionInstallForcelist".to_string(),
            ],
            "brave" => vec![
                r"Software\Policies\BraveSoftware\Brave\ExtensionInstallForcelist".to_string(),
            ],
            "opera" => vec![
                r"Software\Policies\Opera Software\ExtensionInstallForcelist".to_string(),
                r"Software\Policies\Opera Software\Opera Stable\ExtensionInstallForcelist".to_string(),
            ],
            "operagx" => vec![
                r"Software\Policies\Opera Software\ExtensionInstallForcelist".to_string(),
                r"Software\Policies\Opera Software\Opera GX Stable\ExtensionInstallForcelist".to_string(),
            ],
            _ => vec![],
        };

        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);

        for subkey in subkeys {
            if let Ok((key, _)) = hkcu.create_subkey(&subkey) {
                for (idx, item) in items.iter().enumerate() {
                    let _ = key.set_value(&(idx + 1).to_string(), item);
                }
            }
            if let Ok((key, _)) = hklm.create_subkey(&subkey) {
                for (idx, item) in items.iter().enumerate() {
                    let _ = key.set_value(&(idx + 1).to_string(), item);
                }
            }
        }
        Ok(())
    }

    pub fn clean_obsolete_forcelist_policies(browser_id: &str) {
        let subkeys = match browser_id {
            "chrome" => vec![
                r"Software\Policies\Google\Chrome\ExtensionInstallForcelist".to_string(),
            ],
            "edge" => vec![
                r"Software\Policies\Microsoft\Edge\ExtensionInstallForcelist".to_string(),
            ],
            "brave" => vec![
                r"Software\Policies\BraveSoftware\Brave\ExtensionInstallForcelist".to_string(),
            ],
            "opera" => vec![
                r"Software\Policies\Opera Software\ExtensionInstallForcelist".to_string(),
                r"Software\Policies\Opera Software\Opera Stable\ExtensionInstallForcelist".to_string(),
            ],
            "operagx" => vec![
                r"Software\Policies\Opera Software\ExtensionInstallForcelist".to_string(),
                r"Software\Policies\Opera Software\Opera GX Stable\ExtensionInstallForcelist".to_string(),
            ],
            _ => vec![],
        };

        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
        for sub in &subkeys {
            let _ = hkcu.delete_subkey_all(sub);
            let _ = hklm.delete_subkey_all(sub);
        }
    }

    pub fn register_all_chromium_policies(browser_id: &str) {
        Self::clean_obsolete_forcelist_policies(browser_id);
        let _ = Self::register_chromium_policy_allowlist(browser_id);
        let _ = Self::register_chromium_policy_sources(browser_id);
        let _ = Self::register_chromium_developer_mode_policy(browser_id);
    }

    pub fn check_native_host_registration(browser_id: &str) -> (bool, Option<String>) {
        let subkeys = Self::get_subkeys_for_browser(browser_id);
        if subkeys.is_empty() {
            return (false, None);
        }

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
        let subkeys = vec![
            format!(r"Software\Google\Chrome\NativeMessagingHosts\{}", HOST_NAME),
            format!(r"Software\Chromium\NativeMessagingHosts\{}", HOST_NAME),
            format!(r"Software\Microsoft\Edge\NativeMessagingHosts\{}", HOST_NAME),
            format!(r"Software\BraveSoftware\Brave-Browser\NativeMessagingHosts\{}", HOST_NAME),
            format!(r"Software\Opera Software\NativeMessagingHosts\{}", HOST_NAME),
            format!(r"Software\Opera Software\Opera Stable\NativeMessagingHosts\{}", HOST_NAME),
            format!(r"Software\Opera Software\Opera GX Stable\NativeMessagingHosts\{}", HOST_NAME),
            format!(r"Software\Opera Software\Opera GX\NativeMessagingHosts\{}", HOST_NAME),
            format!(r"Software\Opera Software\Opera Developer\NativeMessagingHosts\{}", HOST_NAME),
            format!(r"Software\Opera Software\Opera Next\NativeMessagingHosts\{}", HOST_NAME),
            format!(r"Software\Mozilla\NativeMessagingHosts\{}", HOST_NAME),
            format!(r"Software\WOW6432Node\Google\Chrome\NativeMessagingHosts\{}", HOST_NAME),
            format!(r"Software\WOW6432Node\Chromium\NativeMessagingHosts\{}", HOST_NAME),
            format!(r"Software\WOW6432Node\Microsoft\Edge\NativeMessagingHosts\{}", HOST_NAME),
            format!(r"Software\WOW6432Node\BraveSoftware\Brave-Browser\NativeMessagingHosts\{}", HOST_NAME),
            format!(r"Software\WOW6432Node\Opera Software\NativeMessagingHosts\{}", HOST_NAME),
            format!(r"Software\WOW6432Node\Opera Software\Opera Stable\NativeMessagingHosts\{}", HOST_NAME),
            format!(r"Software\WOW6432Node\Opera Software\Opera GX Stable\NativeMessagingHosts\{}", HOST_NAME),
            format!(r"Software\WOW6432Node\Opera Software\Opera GX\NativeMessagingHosts\{}", HOST_NAME),
            format!(r"Software\WOW6432Node\Mozilla\NativeMessagingHosts\{}", HOST_NAME),
            format!(r"Software\Google\Chrome\Extensions\ikkidkbojpedpeaomipgioiigmjlpflg"),
            format!(r"Software\Microsoft\Edge\Extensions\ikkidkbojpedpeaomipgioiigmjlpflg"),
            format!(r"Software\BraveSoftware\Brave-Browser\Extensions\ikkidkbojpedpeaomipgioiigmjlpflg"),
            format!(r"Software\Opera Software\Opera Stable\Extensions\ikkidkbojpedpeaomipgioiigmjlpflg"),
            format!(r"Software\Opera Software\Opera GX Stable\Extensions\ikkidkbojpedpeaomipgioiigmjlpflg"),
            format!(r"Software\Opera Software\Opera GX\Extensions\ikkidkbojpedpeaomipgioiigmjlpflg"),
            format!(r"Software\Opera Software\Extensions\ikkidkbojpedpeaomipgioiigmjlpflg"),
            format!(r"Software\Policies\Google\Chrome\ExtensionInstallAllowlist"),
            format!(r"Software\Policies\Microsoft\Edge\ExtensionInstallAllowlist"),
            format!(r"Software\Policies\BraveSoftware\Brave\ExtensionInstallAllowlist"),
            format!(r"Software\Policies\Opera Software\ExtensionInstallAllowlist"),
            format!(r"Software\Policies\Opera Software\Opera Stable\ExtensionInstallAllowlist"),
            format!(r"Software\Policies\Google\Chrome\ExtensionInstallForcelist"),
            format!(r"Software\Policies\Microsoft\Edge\ExtensionInstallForcelist"),
            format!(r"Software\Policies\BraveSoftware\Brave\ExtensionInstallForcelist"),
            format!(r"Software\Policies\Opera Software\ExtensionInstallForcelist"),
            format!(r"Software\Policies\Opera Software\Opera Stable\ExtensionInstallForcelist"),
            format!(r"Software\Policies\Opera Software\Opera GX Stable\ExtensionInstallForcelist"),
            format!(r"Software\Policies\Google\Chrome\ExtensionInstallSources"),
            format!(r"Software\Policies\Microsoft\Edge\ExtensionInstallSources"),
            format!(r"Software\Policies\BraveSoftware\Brave\ExtensionInstallSources"),
            format!(r"Software\Policies\Opera Software\ExtensionInstallSources"),
            format!(r"Software\Policies\Opera Software\Opera Stable\ExtensionInstallSources"),
            format!(r"Software\Policies\Opera Software\Opera GX Stable\ExtensionInstallSources"),
            format!(r"Software\Policies\Mozilla\Firefox\ExtensionSettings\snapload@darkskyee.github.io"),
        ];

        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
        for sub in &subkeys {
            if hkcu.delete_subkey_all(sub).is_ok() {
                removed.push(format!("HKCU\\{}", sub));
            }
            if hklm.delete_subkey_all(sub).is_ok() {
                removed.push(format!("HKLM\\{}", sub));
            }
        }

        // Clean Firefox extension value in HKCU and HKLM
        if let Ok(key) = hkcu.open_subkey(r"Software\Mozilla\Firefox\Extensions") {
            let _ = key.delete_value("snapload@darkskyee.github.io");
            removed.push("HKCU Firefox Extension Registration".to_string());
        }
        if let Ok(key) = hklm.open_subkey(r"Software\Mozilla\Firefox\Extensions") {
            let _ = key.delete_value("snapload@darkskyee.github.io");
            removed.push("HKLM Firefox Extension Registration".to_string());
        }

        // Clean DeveloperModeAvailability policies if set by SnapLoad
        let dev_policies = [
            r"Software\Policies\Google\Chrome",
            r"Software\Policies\Microsoft\Edge",
            r"Software\Policies\BraveSoftware\Brave",
            r"Software\Policies\Opera Software",
            r"Software\Policies\Opera Software\Opera Stable",
            r"Software\Policies\Opera Software\Opera GX Stable",
        ];
        for pol in dev_policies {
            if let Ok(key) = hkcu.open_subkey(pol) {
                let _ = key.delete_value("DeveloperModeAvailability");
            }
            if let Ok(key) = hklm.open_subkey(pol) {
                let _ = key.delete_value("DeveloperModeAvailability");
            }
        }

        removed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_chromium_extension_rejects_dummy_and_relative() {
        let dummy = PathBuf::from("dummy");
        let res = RegistryManager::register_chromium_extension("opera", &dummy);
        assert!(res.is_err(), "Expected error for dummy path");

        let rel = PathBuf::from("relative/ext/path");
        let res = RegistryManager::register_chromium_extension("opera", &rel);
        assert!(res.is_err(), "Expected error for relative path");
    }

    #[test]
    fn test_register_chromium_extension_rejects_missing_manifest() {
        let temp_dir = std::env::temp_dir().join("snapload_test_empty_ext_dir");
        let _ = std::fs::remove_dir_all(&temp_dir);
        let _ = std::fs::create_dir_all(&temp_dir);
        let res = RegistryManager::register_chromium_extension("opera", &temp_dir);
        assert!(res.is_err(), "Expected error when manifest.json is missing");
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_policy_forcelist_registration() {
        let entry = "ikkidkbojpedpeaomipgioiigmjlpflg;https://clients2.google.com/service/update2/crx";
        for b_id in ["opera", "operagx", "chrome", "edge", "brave"] {
            let res = RegistryManager::register_chromium_policy_forcelist(b_id, &[entry]);
            assert!(res.is_ok(), "Failed registering forcelist for {}: {:?}", b_id, res);
            RegistryManager::clean_obsolete_forcelist_policies(b_id);
        }
    }
}
