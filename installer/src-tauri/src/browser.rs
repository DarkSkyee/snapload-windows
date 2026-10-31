use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use winreg::enums::*;
use winreg::RegKey;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserInfo {
    pub id: String,
    pub name: String,
    pub icon: String,
    pub detected: bool,
    pub path: Option<String>,
    pub selected: bool,
}

pub struct BrowserDetector;

impl BrowserDetector {
    pub fn detect_all() -> Vec<BrowserInfo> {
        let mut list = Vec::new();

        // 1. Google Chrome
        let chrome_path = Self::find_chrome();
        list.push(BrowserInfo {
            id: "chrome".to_string(),
            name: "Google Chrome".to_string(),
            icon: "🌐".to_string(),
            detected: chrome_path.is_some(),
            selected: chrome_path.is_some(),
            path: chrome_path,
        });

        // 2. Microsoft Edge
        let edge_path = Self::find_edge();
        list.push(BrowserInfo {
            id: "edge".to_string(),
            name: "Microsoft Edge".to_string(),
            icon: "🌊".to_string(),
            detected: edge_path.is_some(),
            selected: edge_path.is_some(),
            path: edge_path,
        });

        // 3. Brave Browser
        let brave_path = Self::find_brave();
        list.push(BrowserInfo {
            id: "brave".to_string(),
            name: "Brave Browser".to_string(),
            icon: "🦁".to_string(),
            detected: brave_path.is_some(),
            selected: brave_path.is_some(),
            path: brave_path,
        });

        // 4. Opera
        let opera_path = Self::find_opera();
        list.push(BrowserInfo {
            id: "opera".to_string(),
            name: "Opera".to_string(),
            icon: "⭕".to_string(),
            detected: opera_path.is_some(),
            selected: opera_path.is_some(),
            path: opera_path,
        });

        // 5. Mozilla Firefox
        let firefox_path = Self::find_firefox();
        list.push(BrowserInfo {
            id: "firefox".to_string(),
            name: "Mozilla Firefox".to_string(),
            icon: "🦊".to_string(),
            detected: firefox_path.is_some(),
            selected: firefox_path.is_some(),
            path: firefox_path,
        });

        list
    }

    fn check_reg_or_file(reg_path: &str, file_candidates: &[PathBuf]) -> Option<String> {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);

        if let Ok(key) = hkcu.open_subkey(reg_path) {
            if let Ok(val) = key.get_value::<String, _>("") {
                return Some(val);
            }
        }
        if let Ok(key) = hklm.open_subkey(reg_path) {
            if let Ok(val) = key.get_value::<String, _>("") {
                return Some(val);
            }
        }

        for p in file_candidates {
            if p.exists() {
                return Some(p.to_string_lossy().to_string());
            }
        }
        None
    }

    fn find_chrome() -> Option<String> {
        let mut candidates = Vec::new();
        if let Some(prog) = std::env::var_os("ProgramFiles") {
            candidates.push(PathBuf::from(prog).join("Google\\Chrome\\Application\\chrome.exe"));
        }
        if let Some(prog86) = std::env::var_os("ProgramFiles(x86)") {
            candidates.push(PathBuf::from(prog86).join("Google\\Chrome\\Application\\chrome.exe"));
        }
        if let Some(local) = std::env::var_os("LOCALAPPDATA") {
            candidates.push(PathBuf::from(local).join("Google\\Chrome\\Application\\chrome.exe"));
        }
        Self::check_reg_or_file(
            r"Software\Microsoft\Windows\CurrentVersion\App Paths\chrome.exe",
            &candidates,
        )
    }

    fn find_edge() -> Option<String> {
        let mut candidates = Vec::new();
        if let Some(prog86) = std::env::var_os("ProgramFiles(x86)") {
            candidates.push(PathBuf::from(prog86).join("Microsoft\\Edge\\Application\\msedge.exe"));
        }
        if let Some(prog) = std::env::var_os("ProgramFiles") {
            candidates.push(PathBuf::from(prog).join("Microsoft\\Edge\\Application\\msedge.exe"));
        }
        Self::check_reg_or_file(
            r"Software\Microsoft\Windows\CurrentVersion\App Paths\msedge.exe",
            &candidates,
        )
    }

    fn find_brave() -> Option<String> {
        let mut candidates = Vec::new();
        if let Some(prog) = std::env::var_os("ProgramFiles") {
            candidates.push(
                PathBuf::from(prog).join("BraveSoftware\\Brave-Browser\\Application\\brave.exe"),
            );
        }
        if let Some(local) = std::env::var_os("LOCALAPPDATA") {
            candidates.push(
                PathBuf::from(local).join("BraveSoftware\\Brave-Browser\\Application\\brave.exe"),
            );
        }
        Self::check_reg_or_file(
            r"Software\Microsoft\Windows\CurrentVersion\App Paths\brave.exe",
            &candidates,
        )
    }

    fn find_opera() -> Option<String> {
        let mut candidates = Vec::new();
        if let Some(local) = std::env::var_os("LOCALAPPDATA") {
            candidates.push(PathBuf::from(local).join("Programs\\Opera\\launcher.exe"));
        }
        if let Some(prog) = std::env::var_os("ProgramFiles") {
            candidates.push(PathBuf::from(prog).join("Opera\\launcher.exe"));
        }
        Self::check_reg_or_file(
            r"Software\Microsoft\Windows\CurrentVersion\App Paths\opera.exe",
            &candidates,
        )
    }

    fn find_firefox() -> Option<String> {
        let mut candidates = Vec::new();
        if let Some(prog) = std::env::var_os("ProgramFiles") {
            candidates.push(PathBuf::from(prog).join("Mozilla Firefox\\firefox.exe"));
        }
        if let Some(prog86) = std::env::var_os("ProgramFiles(x86)") {
            candidates.push(PathBuf::from(prog86).join("Mozilla Firefox\\firefox.exe"));
        }
        Self::check_reg_or_file(
            r"Software\Microsoft\Windows\CurrentVersion\App Paths\firefox.exe",
            &candidates,
        )
    }
}
