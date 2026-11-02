pub mod browser;
pub mod registry;
pub mod setup;
pub mod updater;

use browser::{BrowserDetector, BrowserInfo};
use setup::{
    AutoInstallResult, DiagnosticResult, ExtensionPaths, InstallOptions, InstallResult,
    SetupManager,
};
use updater::{UpdateChecker, UpdateInfo};

#[tauri::command]
fn detect_browsers() -> Vec<BrowserInfo> {
    BrowserDetector::detect_all()
}

#[tauri::command]
fn auto_install_browser_extension(
    browser_id: String,
    restart_if_running: Option<bool>,
) -> Result<AutoInstallResult, String> {
    SetupManager::auto_install_for_browser(&browser_id, restart_if_running.unwrap_or(true))
}

#[tauri::command]
fn is_browser_running(browser_id: String) -> bool {
    BrowserDetector::is_browser_process_running(&browser_id)
}

#[tauri::command]
fn copy_to_clipboard(text: String) -> Result<(), String> {
    #[cfg(windows)]
    {
        use std::io::Write;
        use std::os::windows::process::CommandExt;
        let mut child = std::process::Command::new("clip.exe")
            .stdin(std::process::Stdio::piped())
            .creation_flags(0x08000000)
            .spawn()
            .map_err(|e| format!("Failed to spawn clip: {}", e))?;
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(text.as_bytes());
        }
        let _ = child.wait();
        Ok(())
    }
    #[cfg(not(windows))]
    {
        Err("Only windows supported".to_string())
    }
}

#[tauri::command]
async fn install_snapload(options: InstallOptions) -> Result<InstallResult, String> {
    SetupManager::run_install(options).await
}

#[tauri::command]
async fn check_updates() -> UpdateInfo {
    UpdateChecker::check_all().await
}

#[tauri::command]
async fn verify_installation() -> DiagnosticResult {
    SetupManager::verify_installation().await
}

#[tauri::command]
fn uninstall_snapload() -> Result<Vec<String>, String> {
    SetupManager::uninstall()
}

#[tauri::command]
fn get_paths() -> ExtensionPaths {
    SetupManager::get_paths()
}

#[tauri::command]
fn open_url(url: String) -> Result<(), String> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let mut cmd = std::process::Command::new("explorer.exe");
        cmd.arg(&url);
        cmd.creation_flags(0x08000000);
        cmd.spawn().map_err(|e| format!("Failed to open URL: {}", e))?;
        Ok(())
    }
    #[cfg(not(windows))]
    {
        Err("Only windows is supported".to_string())
    }
}

#[tauri::command]
fn open_folder(path: String) -> Result<(), String> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let target = if path.is_empty() {
            SetupManager::get_app_dir().to_string_lossy().to_string()
        } else {
            path
        };
        let mut cmd = std::process::Command::new("explorer.exe");
        cmd.arg(&target);
        cmd.creation_flags(0x08000000);
        cmd.spawn().map_err(|e| format!("Failed to open folder: {}", e))?;
        Ok(())
    }
    #[cfg(not(windows))]
    {
        Err("Only windows is supported".to_string())
    }
}

#[tauri::command]
fn launch_browser_with_extension(browser_id: String) -> Result<String, String> {
    let browser_exe = match browser_id.as_str() {
        "chrome" => BrowserDetector::find_chrome(),
        "edge" => BrowserDetector::find_edge(),
        "brave" => BrowserDetector::find_brave(),
        "opera" => BrowserDetector::find_opera(),
        "operagx" => BrowserDetector::find_opera_gx(),
        "firefox" => BrowserDetector::find_firefox(),
        _ => None,
    };

    if browser_id == "firefox" {
        let xpi_path = SetupManager::get_firefox_ext_dir().join("snapload-extension-firefox.xpi");
        if let Some(exe) = browser_exe {
            let clean_exe = BrowserDetector::sanitize_exe_path(&exe);
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                let mut cmd = std::process::Command::new(&clean_exe);
                cmd.arg(xpi_path.to_string_lossy().to_string());
                cmd.creation_flags(0x08000000);
                cmd.spawn().map_err(|e| format!("Failed to launch Firefox: {}", e))?;
                return Ok("Opened extension in Firefox — click 'Add' to install!".to_string());
            }
        } else {
            return Err("Firefox executable not found.".to_string());
        }
    }

    let ext_path = SetupManager::get_chromium_ext_dir();
    let ext_path_str = ext_path.to_string_lossy().to_string();

    if let Some(exe) = browser_exe {
        let clean_exe = BrowserDetector::sanitize_exe_path(&exe);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            let mut cmd = std::process::Command::new(&clean_exe);
            cmd.arg(format!("--load-extension={}", ext_path_str));
            cmd.arg("https://www.youtube.com");
            cmd.creation_flags(0x08000000);
            cmd.spawn().map_err(|e| format!("Failed to launch {}: {}", browser_id, e))?;
            Ok(format!("Launched {} with SnapLoad loaded!", browser_id))
        }
        #[cfg(not(windows))]
        {
            Err("Only windows supported".to_string())
        }
    } else {
        Err(format!("Browser executable for '{}' not found.", browser_id))
    }
}

#[tauri::command]
fn open_browser_extensions_page(browser_id: String) -> Result<String, String> {
    let url = match browser_id.as_str() {
        "chrome" => "chrome://extensions",
        "edge" => "edge://extensions",
        "brave" => "brave://extensions",
        "opera" | "operagx" => "opera://extensions",
        "firefox" => "about:addons",
        _ => "chrome://extensions",
    };

    let browser_exe = match browser_id.as_str() {
        "chrome" => BrowserDetector::find_chrome(),
        "edge" => BrowserDetector::find_edge(),
        "brave" => BrowserDetector::find_brave(),
        "opera" => BrowserDetector::find_opera(),
        "operagx" => BrowserDetector::find_opera_gx(),
        "firefox" => BrowserDetector::find_firefox(),
        _ => None,
    };

    if let Some(exe) = browser_exe {
        let clean_exe = BrowserDetector::sanitize_exe_path(&exe);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            let mut cmd = std::process::Command::new(&clean_exe);
            cmd.arg(url);
            cmd.creation_flags(0x08000000);
            cmd.spawn().map_err(|e| format!("Failed to open extensions page: {}", e))?;
            Ok(format!("Opened {}", url))
        }
        #[cfg(not(windows))]
        {
            Err("Only windows supported".to_string())
        }
    } else {
        open_url(url.to_string())?;
        Ok(format!("Opened {}", url))
    }
}

#[tauri::command]
fn open_folder_and_select(path: String) -> Result<(), String> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let mut cmd = std::process::Command::new("explorer.exe");
        cmd.arg(format!("/select,{}", path));
        cmd.creation_flags(0x08000000);
        cmd.spawn().map_err(|e| format!("Failed to open Explorer: {}", e))?;
        Ok(())
    }
    #[cfg(not(windows))]
    {
        Err("Only windows is supported".to_string())
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            detect_browsers,
            auto_install_browser_extension,
            is_browser_running,
            copy_to_clipboard,
            install_snapload,
            check_updates,
            verify_installation,
            uninstall_snapload,
            get_paths,
            open_url,
            open_folder,
            open_folder_and_select,
            launch_browser_with_extension,
            open_browser_extensions_page,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
