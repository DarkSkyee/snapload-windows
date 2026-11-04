use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use winreg::enums::*;
use winreg::RegKey;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserInfo {
    pub id: String,
    pub name: String,
    pub icon: String,
    pub detected: bool,
    pub path: Option<String>,
    pub profile_path: Option<String>,
    pub is_running: bool,
    pub selected: bool,
}

pub struct BrowserDetector;

impl BrowserDetector {
    /// Cleans and extracts the exact executable path from registry values or shortcut strings.
    /// Handles quoted paths with flags (e.g. `"C:\Program Files\Opera\opera.exe" -noautoupdate -- "%1"`),
    /// unquoted paths, single quotes, and leading/trailing whitespace.
    pub fn sanitize_exe_path(val: &str) -> String {
        let trimmed = val.trim();
        if trimmed.is_empty() {
            return String::new();
        }

        // 1. Double-quoted executable: "C:\path\to\app.exe" ...
        if trimmed.starts_with('"') {
            if let Some(end_idx) = trimmed[1..].find('"') {
                return trimmed[1..1 + end_idx].trim().to_string();
            }
        }

        // 2. Single-quoted executable: 'C:\path\to\app.exe' ...
        if trimmed.starts_with('\'') {
            if let Some(end_idx) = trimmed[1..].find('\'') {
                return trimmed[1..1 + end_idx].trim().to_string();
            }
        }

        // 3. Unquoted executable path followed by command-line arguments (find .exe)
        if let Some(exe_idx) = trimmed.to_lowercase().find(".exe") {
            let potential = &trimmed[..exe_idx + 4];
            return potential
                .trim_matches('"')
                .trim_matches('\'')
                .trim()
                .to_string();
        }

        // 4. Fallback trim
        trimmed.trim_matches('"').trim_matches('\'').trim().to_string()
    }

    pub fn detect_all() -> Vec<BrowserInfo> {
        let mut list = Vec::new();

        // 1. Google Chrome
        let chrome_path = Self::find_chrome();
        let chrome_profile = Self::get_browser_display_profile("chrome");
        let chrome_running = Self::is_browser_process_running("chrome");
        list.push(BrowserInfo {
            id: "chrome".to_string(),
            name: "Google Chrome".to_string(),
            icon: "chrome".to_string(),
            detected: chrome_path.is_some(),
            selected: chrome_path.is_some(),
            path: chrome_path,
            profile_path: chrome_profile,
            is_running: chrome_running,
        });

        // 2. Microsoft Edge
        let edge_path = Self::find_edge();
        let edge_profile = Self::get_browser_display_profile("edge");
        let edge_running = Self::is_browser_process_running("edge");
        list.push(BrowserInfo {
            id: "edge".to_string(),
            name: "Microsoft Edge".to_string(),
            icon: "edge".to_string(),
            detected: edge_path.is_some(),
            selected: edge_path.is_some(),
            path: edge_path,
            profile_path: edge_profile,
            is_running: edge_running,
        });

        // 3. Brave Browser
        let brave_path = Self::find_brave();
        let brave_profile = Self::get_browser_display_profile("brave");
        let brave_running = Self::is_browser_process_running("brave");
        list.push(BrowserInfo {
            id: "brave".to_string(),
            name: "Brave Browser".to_string(),
            icon: "brave".to_string(),
            detected: brave_path.is_some(),
            selected: brave_path.is_some(),
            path: brave_path,
            profile_path: brave_profile,
            is_running: brave_running,
        });

        // 4. Opera Stable
        let opera_path = Self::find_opera();
        let opera_profile = Self::get_browser_display_profile("opera");
        let opera_running = Self::is_browser_process_running("opera");
        list.push(BrowserInfo {
            id: "opera".to_string(),
            name: "Opera".to_string(),
            icon: "opera".to_string(),
            detected: opera_path.is_some(),
            selected: opera_path.is_some(),
            path: opera_path,
            profile_path: opera_profile,
            is_running: opera_running,
        });

        // 5. Opera GX
        let operagx_path = Self::find_opera_gx();
        let operagx_profile = Self::get_browser_display_profile("operagx");
        let operagx_running = Self::is_browser_process_running("operagx");
        list.push(BrowserInfo {
            id: "operagx".to_string(),
            name: "Opera GX".to_string(),
            icon: "operagx".to_string(),
            detected: operagx_path.is_some(),
            selected: operagx_path.is_some(),
            path: operagx_path,
            profile_path: operagx_profile,
            is_running: operagx_running,
        });

        // 6. Mozilla Firefox
        let firefox_path = Self::find_firefox();
        let firefox_profile = Self::get_browser_display_profile("firefox");
        let firefox_running = Self::is_browser_process_running("firefox");
        list.push(BrowserInfo {
            id: "firefox".to_string(),
            name: "Mozilla Firefox".to_string(),
            icon: "firefox".to_string(),
            detected: firefox_path.is_some(),
            selected: firefox_path.is_some(),
            path: firefox_path,
            profile_path: firefox_profile,
            is_running: firefox_running,
        });

        list
    }

    pub fn find_browser_exe(browser_id: &str) -> Option<String> {
        match browser_id {
            "chrome" => Self::find_chrome(),
            "edge" => Self::find_edge(),
            "brave" => Self::find_brave(),
            "opera" => Self::find_opera(),
            "operagx" => Self::find_opera_gx(),
            "firefox" => Self::find_firefox(),
            _ => None,
        }
    }

    pub fn is_browser_process_running(browser_id: &str) -> bool {
        let proc_names: Vec<&str> = match browser_id {
            "chrome" => vec!["chrome.exe"],
            "edge" => vec!["msedge.exe"],
            "brave" => vec!["brave.exe"],
            "opera" => vec!["opera.exe"],
            "operagx" => vec!["opera.exe"],
            "firefox" => vec!["firefox.exe"],
            _ => return false,
        };

        for name in proc_names {
            let mut cmd = std::process::Command::new("tasklist.exe");
            cmd.args(["/fi", &format!("imagename eq {}", name), "/fo", "csv", "/nh"]);
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                cmd.creation_flags(0x08000000);
            }
            if let Ok(out) = cmd.output() {
                let text = String::from_utf8_lossy(&out.stdout).to_lowercase();
                if text.contains(name) {
                    return true;
                }
            }
        }
        false
    }

    pub fn close_browser_processes(browser_id: &str) -> Result<(), String> {
        let proc_names: Vec<&str> = match browser_id {
            "chrome" => vec!["chrome.exe"],
            "edge" => vec!["msedge.exe"],
            "brave" => vec!["brave.exe"],
            "opera" => vec!["opera.exe"],
            "operagx" => vec!["opera.exe"],
            "firefox" => vec!["firefox.exe"],
            _ => return Ok(()),
        };

        for name in proc_names {
            let mut cmd = std::process::Command::new("taskkill.exe");
            cmd.args(["/F", "/IM", name]);
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                cmd.creation_flags(0x08000000);
            }
            let _ = cmd.output();
        }
        std::thread::sleep(std::time::Duration::from_millis(600));
        Ok(())
    }

    fn check_reg_or_file(reg_path: &str, file_candidates: &[PathBuf]) -> Option<String> {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);

        if let Ok(key) = hkcu.open_subkey(reg_path) {
            if let Ok(val) = key.get_value::<String, _>("") {
                let clean = Self::sanitize_exe_path(&val);
                if !clean.is_empty() && Path::new(&clean).exists() {
                    return Some(clean);
                }
            }
        }
        if let Ok(key) = hklm.open_subkey(reg_path) {
            if let Ok(val) = key.get_value::<String, _>("") {
                let clean = Self::sanitize_exe_path(&val);
                if !clean.is_empty() && Path::new(&clean).exists() {
                    return Some(clean);
                }
            }
        }

        for p in file_candidates {
            if p.exists() {
                return Some(Self::sanitize_exe_path(&p.to_string_lossy()));
            }
        }
        None
    }

    pub fn find_chrome() -> Option<String> {
        let mut candidates = Vec::new();
        if let Some(prog) = std::env::var_os("ProgramFiles") {
            candidates.push(PathBuf::from(&prog).join("Google\\Chrome\\Application\\chrome.exe"));
        }
        if let Some(prog86) = std::env::var_os("ProgramFiles(x86)") {
            candidates.push(PathBuf::from(&prog86).join("Google\\Chrome\\Application\\chrome.exe"));
        }
        if let Some(local) = std::env::var_os("LOCALAPPDATA") {
            candidates.push(PathBuf::from(&local).join("Google\\Chrome\\Application\\chrome.exe"));
        }
        Self::check_reg_or_file(
            r"Software\Microsoft\Windows\CurrentVersion\App Paths\chrome.exe",
            &candidates,
        )
        .or_else(|| {
            Self::check_reg_or_file(
                r"SOFTWARE\Clients\StartMenuInternet\Google Chrome\shell\open\command",
                &candidates,
            )
        })
    }

    pub fn find_edge() -> Option<String> {
        let mut candidates = Vec::new();
        if let Some(prog86) = std::env::var_os("ProgramFiles(x86)") {
            candidates.push(PathBuf::from(&prog86).join("Microsoft\\Edge\\Application\\msedge.exe"));
        }
        if let Some(prog) = std::env::var_os("ProgramFiles") {
            candidates.push(PathBuf::from(&prog).join("Microsoft\\Edge\\Application\\msedge.exe"));
        }
        if let Some(local) = std::env::var_os("LOCALAPPDATA") {
            candidates.push(PathBuf::from(&local).join("Microsoft\\Edge\\Application\\msedge.exe"));
        }

        let is_valid_edge = |path_str: &str| -> bool {
            let p = Path::new(path_str);
            if !p.exists() || !p.is_file() {
                return false;
            }
            let lossy = path_str.to_lowercase();
            // Disallow EdgeCore, EdgeWebView, or temp stubs
            if lossy.contains("edgecore") || lossy.contains("edgewebview") {
                return false;
            }
            if let Ok(meta) = p.metadata() {
                if meta.len() < 100_000 {
                    return false;
                }
            }
            true
        };

        if let Some(found) = Self::check_reg_or_file(
            r"Software\Microsoft\Windows\CurrentVersion\App Paths\msedge.exe",
            &candidates,
        ) {
            if is_valid_edge(&found) {
                return Some(found);
            }
        }

        if let Some(found) = Self::check_reg_or_file(
            r"SOFTWARE\Clients\StartMenuInternet\Microsoft Edge\shell\open\command",
            &candidates,
        ) {
            if is_valid_edge(&found) {
                return Some(found);
            }
        }

        for c in &candidates {
            let s = c.to_string_lossy().to_string();
            if is_valid_edge(&s) {
                return Some(s);
            }
        }

        None
    }

    pub fn find_brave() -> Option<String> {
        let mut candidates = Vec::new();
        if let Some(prog) = std::env::var_os("ProgramFiles") {
            candidates.push(
                PathBuf::from(&prog).join("BraveSoftware\\Brave-Browser\\Application\\brave.exe"),
            );
        }
        if let Some(prog86) = std::env::var_os("ProgramFiles(x86)") {
            candidates.push(
                PathBuf::from(&prog86).join("BraveSoftware\\Brave-Browser\\Application\\brave.exe"),
            );
        }
        if let Some(local) = std::env::var_os("LOCALAPPDATA") {
            candidates.push(
                PathBuf::from(&local).join("BraveSoftware\\Brave-Browser\\Application\\brave.exe"),
            );
        }
        Self::check_reg_or_file(
            r"Software\Microsoft\Windows\CurrentVersion\App Paths\brave.exe",
            &candidates,
        )
        .or_else(|| {
            Self::check_reg_or_file(
                r"SOFTWARE\Clients\StartMenuInternet\Brave\shell\open\command",
                &candidates,
            )
        })
    }

    pub fn find_opera() -> Option<String> {
        let mut candidates = Vec::new();
        if let Some(local) = std::env::var_os("LOCALAPPDATA") {
            candidates.push(PathBuf::from(&local).join("Programs\\Opera\\launcher.exe"));
            candidates.push(PathBuf::from(&local).join("Programs\\Opera\\opera.exe"));
        }
        if let Some(prog) = std::env::var_os("ProgramFiles") {
            candidates.push(PathBuf::from(&prog).join("Opera\\opera.exe"));
            candidates.push(PathBuf::from(&prog).join("Opera\\launcher.exe"));
        }
        if let Some(prog86) = std::env::var_os("ProgramFiles(x86)") {
            candidates.push(PathBuf::from(&prog86).join("Opera\\opera.exe"));
            candidates.push(PathBuf::from(&prog86).join("Opera\\launcher.exe"));
        }
        if let Some(appdata) = std::env::var_os("APPDATA") {
            candidates.push(PathBuf::from(&appdata).join("Opera Software\\Opera Stable\\launcher.exe"));
        }

        Self::check_reg_or_file(
            r"Software\Microsoft\Windows\CurrentVersion\App Paths\opera.exe",
            &candidates,
        )
        .or_else(|| {
            Self::check_reg_or_file(
                r"Software\Classes\OperaStable\shell\open\command",
                &candidates,
            )
        })
        .or_else(|| {
            Self::check_reg_or_file(
                r"SOFTWARE\Clients\StartMenuInternet\OperaStable\shell\open\command",
                &candidates,
            )
        })
    }

    pub fn find_opera_gx() -> Option<String> {
        let mut candidates = Vec::new();
        if let Some(local) = std::env::var_os("LOCALAPPDATA") {
            candidates.push(PathBuf::from(&local).join("Programs\\Opera GX\\launcher.exe"));
            candidates.push(PathBuf::from(&local).join("Programs\\Opera GX\\opera.exe"));
        }
        if let Some(prog) = std::env::var_os("ProgramFiles") {
            candidates.push(PathBuf::from(&prog).join("Opera GX\\opera.exe"));
            candidates.push(PathBuf::from(&prog).join("Opera GX\\launcher.exe"));
        }
        if let Some(prog86) = std::env::var_os("ProgramFiles(x86)") {
            candidates.push(PathBuf::from(&prog86).join("Opera GX\\opera.exe"));
            candidates.push(PathBuf::from(&prog86).join("Opera GX\\launcher.exe"));
        }
        if let Some(appdata) = std::env::var_os("APPDATA") {
            candidates.push(PathBuf::from(&appdata).join("Opera Software\\Opera GX Stable\\launcher.exe"));
            candidates.push(PathBuf::from(&appdata).join("Opera Software\\Opera GX\\launcher.exe"));
        }

        Self::check_reg_or_file(
            r"Software\Microsoft\Windows\CurrentVersion\App Paths\opera_gx.exe",
            &candidates,
        )
        .or_else(|| {
            Self::check_reg_or_file(
                r"Software\Microsoft\Windows\CurrentVersion\App Paths\operagx.exe",
                &candidates,
            )
        })
        .or_else(|| {
            Self::check_reg_or_file(
                r"Software\Classes\OperaGXStable\shell\open\command",
                &candidates,
            )
        })
        .or_else(|| {
            Self::check_reg_or_file(
                r"SOFTWARE\Clients\StartMenuInternet\OperaGXStable\shell\open\command",
                &candidates,
            )
        })
    }

    pub fn find_firefox() -> Option<String> {
        let mut candidates = Vec::new();
        if let Some(prog) = std::env::var_os("ProgramFiles") {
            candidates.push(PathBuf::from(&prog).join("Mozilla Firefox\\firefox.exe"));
        }
        if let Some(prog86) = std::env::var_os("ProgramFiles(x86)") {
            candidates.push(PathBuf::from(&prog86).join("Mozilla Firefox\\firefox.exe"));
        }
        if let Some(local) = std::env::var_os("LOCALAPPDATA") {
            candidates.push(PathBuf::from(&local).join("Mozilla Firefox\\firefox.exe"));
        }

        Self::check_reg_or_file(
            r"Software\Microsoft\Windows\CurrentVersion\App Paths\firefox.exe",
            &candidates,
        )
        .or_else(|| {
            Self::check_reg_or_file(
                r"SOFTWARE\Clients\StartMenuInternet\FIREFOX.EXE\shell\open\command",
                &candidates,
            )
        })
    }

    pub fn find_browser_profiles(browser_id: &str) -> Vec<PathBuf> {
        let mut profiles = Vec::new();

        match browser_id {
            "opera" => {
                if let Some(appdata) = std::env::var_os("APPDATA") {
                    let base = PathBuf::from(&appdata).join("Opera Software").join("Opera Stable");
                    let def = base.join("Default");
                    if def.exists() {
                        profiles.push(def);
                    } else if base.exists() {
                        profiles.push(base);
                    }
                }
                if let Some(local) = std::env::var_os("LOCALAPPDATA") {
                    let base = PathBuf::from(&local).join("Opera Software").join("Opera Stable");
                    let def = base.join("Default");
                    if def.exists() && !profiles.contains(&def) {
                        profiles.push(def);
                    } else if base.exists() && !profiles.contains(&base) {
                        profiles.push(base);
                    }
                }
            }
            "operagx" => {
                if let Some(appdata) = std::env::var_os("APPDATA") {
                    let base = PathBuf::from(&appdata).join("Opera Software").join("Opera GX Stable");
                    let def = base.join("Default");
                    if def.exists() {
                        profiles.push(def);
                    } else if base.exists() {
                        profiles.push(base);
                    }
                }
                if let Some(local) = std::env::var_os("LOCALAPPDATA") {
                    let base = PathBuf::from(&local).join("Opera Software").join("Opera GX Stable");
                    let def = base.join("Default");
                    if def.exists() && !profiles.contains(&def) {
                        profiles.push(def);
                    } else if base.exists() && !profiles.contains(&base) {
                        profiles.push(base);
                    }
                }
            }
            "chrome" => {
                if let Some(local) = std::env::var_os("LOCALAPPDATA") {
                    let user_data = PathBuf::from(&local).join("Google").join("Chrome").join("User Data");
                    if user_data.exists() {
                        let def = user_data.join("Default");
                        if def.exists() {
                            profiles.push(def);
                        } else {
                            profiles.push(user_data.clone());
                        }

                        // Also detect Profile 1, Profile 2, etc.
                        if let Ok(entries) = std::fs::read_dir(&user_data) {
                            for e in entries.flatten() {
                                if let Ok(ft) = e.file_type() {
                                    if ft.is_dir() {
                                        let name = e.file_name().to_string_lossy().to_string();
                                        if name.starts_with("Profile ") {
                                            profiles.push(e.path());
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            "edge" => {
                if let Some(local) = std::env::var_os("LOCALAPPDATA") {
                    let user_data = PathBuf::from(&local).join("Microsoft").join("Edge").join("User Data");
                    if user_data.exists() {
                        let def = user_data.join("Default");
                        if def.exists() {
                            profiles.push(def);
                        } else {
                            profiles.push(user_data.clone());
                        }

                        if let Ok(entries) = std::fs::read_dir(&user_data) {
                            for e in entries.flatten() {
                                if let Ok(ft) = e.file_type() {
                                    if ft.is_dir() {
                                        let name = e.file_name().to_string_lossy().to_string();
                                        if name.starts_with("Profile ") {
                                            profiles.push(e.path());
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            "brave" => {
                if let Some(local) = std::env::var_os("LOCALAPPDATA") {
                    let user_data = PathBuf::from(&local).join("BraveSoftware").join("Brave-Browser").join("User Data");
                    if user_data.exists() {
                        let def = user_data.join("Default");
                        if def.exists() {
                            profiles.push(def);
                        } else {
                            profiles.push(user_data);
                        }
                    }
                }
            }
            "firefox" => {
                if let Some(appdata) = std::env::var_os("APPDATA") {
                    let profiles_dir = PathBuf::from(&appdata).join("Mozilla").join("Firefox").join("Profiles");
                    if profiles_dir.exists() {
                        if let Ok(entries) = std::fs::read_dir(&profiles_dir) {
                            for e in entries.flatten() {
                                if let Ok(ft) = e.file_type() {
                                    if ft.is_dir() {
                                        profiles.push(e.path());
                                    }
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }

        profiles
    }

    pub fn get_browser_display_profile(browser_id: &str) -> Option<String> {
        let profiles = Self::find_browser_profiles(browser_id);
        profiles.first().map(|p| p.to_string_lossy().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_exe_path_quoted_with_args() {
        let input = r#""C:\Program Files\Opera\opera.exe" -noautoupdate -- "%1""#;
        let expected = r"C:\Program Files\Opera\opera.exe";
        assert_eq!(BrowserDetector::sanitize_exe_path(input), expected);
    }

    #[test]
    fn test_sanitize_exe_path_plain_quoted() {
        let input = r#""C:\Program Files\Google\Chrome\Application\chrome.exe""#;
        let expected = r"C:\Program Files\Google\Chrome\Application\chrome.exe";
        assert_eq!(BrowserDetector::sanitize_exe_path(input), expected);
    }

    #[test]
    fn test_sanitize_exe_path_unquoted_with_flag() {
        let input = r"C:\Users\Konrad\AppData\Local\Programs\Opera GX\launcher.exe --flag";
        let expected = r"C:\Users\Konrad\AppData\Local\Programs\Opera GX\launcher.exe";
        assert_eq!(BrowserDetector::sanitize_exe_path(input), expected);
    }

    #[test]
    fn test_sanitize_exe_path_single_quoted() {
        let input = r"'C:\Program Files\BraveSoftware\Brave-Browser\Application\brave.exe' --start";
        let expected = r"C:\Program Files\BraveSoftware\Brave-Browser\Application\brave.exe";
        assert_eq!(BrowserDetector::sanitize_exe_path(input), expected);
    }

    #[test]
    fn test_sanitize_exe_path_empty_and_whitespace() {
        assert_eq!(BrowserDetector::sanitize_exe_path(""), "");
        assert_eq!(BrowserDetector::sanitize_exe_path("   "), "");
    }

    #[test]
    fn test_detect_all_returns_supported_browsers() {
        let browsers = BrowserDetector::detect_all();
        assert_eq!(browsers.len(), 6);
        let ids: Vec<&str> = browsers.iter().map(|b| b.id.as_str()).collect();
        assert_eq!(ids, vec!["chrome", "edge", "brave", "opera", "operagx", "firefox"]);

        for b in &browsers {
            if b.detected {
                assert!(b.path.is_some(), "Detected browser {} must have path", b.id);
                let p = b.path.as_ref().unwrap();
                assert!(Path::new(p).exists(), "Browser path must exist on disk: {}", p);
            }
        }
    }

    #[test]
    fn test_find_browser_exe_matches_detect_all() {
        for b_id in ["chrome", "edge", "brave", "opera", "operagx", "firefox"] {
            let direct = BrowserDetector::find_browser_exe(b_id);
            let in_all = BrowserDetector::detect_all().into_iter().find(|b| b.id == b_id).unwrap().path;
            assert_eq!(direct, in_all);
        }
    }
}
