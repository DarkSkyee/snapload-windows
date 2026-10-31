mod browser;
mod registry;
mod setup;
mod updater;

use browser::{BrowserDetector, BrowserInfo};
use setup::{InstallOptions, InstallResult, SetupManager};
use updater::{UpdateChecker, UpdateInfo};

#[tauri::command]
fn detect_browsers() -> Vec<BrowserInfo> {
    BrowserDetector::detect_all()
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
fn verify_installation() -> Vec<String> {
    SetupManager::verify_installation()
}

#[tauri::command]
fn uninstall_snapload() -> Result<Vec<String>, String> {
    SetupManager::uninstall()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            detect_browsers,
            install_snapload,
            check_updates,
            verify_installation,
            uninstall_snapload
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
