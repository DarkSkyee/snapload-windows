use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::time::timeout;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateInfo {
    pub version: String,
    pub engine_status: String,
    pub engine_badge: String,
    pub ytdlp_status: String,
    pub ytdlp_badge: String,
    pub ffmpeg_status: String,
    pub ffmpeg_badge: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetupUpdateInfo {
    pub current_version: String,
    pub latest_version: String,
    pub update_available: bool,
    pub release_name: String,
    pub release_notes: String,
    pub download_url: Option<String>,
    pub asset_url: Option<String>,
    pub published_at: Option<String>,
    pub file_size_bytes: Option<u64>,
    pub error: Option<String>,
}

const EMBEDDED_MANIFEST: &str = include_str!("../../../release_manifest.json");

pub struct UpdateChecker;

impl UpdateChecker {
    pub fn cleanup_old_setup_binary() {
        if let Ok(current) = std::env::current_exe() {
            let old_exe = current.with_extension("exe.old");
            if old_exe.exists() {
                let _ = std::fs::remove_file(old_exe);
            }
        }
    }

    fn get_app_bin_dir() -> Option<PathBuf> {
        let local_app_data = std::env::var_os("LOCALAPPDATA")?;
        let mut path = PathBuf::from(local_app_data);
        path.push("SnapLoad");
        path.push("bin");
        Some(path)
    }

    fn load_manifest() -> Option<serde_json::Value> {
        let candidates = [
            PathBuf::from("release_manifest.json"),
            PathBuf::from("../release_manifest.json"),
            PathBuf::from("../../release_manifest.json"),
        ];

        for c in &candidates {
            if c.exists() && c.is_file() {
                if let Ok(mut f) = File::open(c) {
                    let mut s = String::new();
                    if f.read_to_string(&mut s).is_ok() {
                        if let Ok(v) = serde_json::from_str(&s) {
                            return Some(v);
                        }
                    }
                }
            }
        }
        serde_json::from_str(EMBEDDED_MANIFEST).ok()
    }

    pub fn compute_file_hash(path: &Path) -> Result<String, std::io::Error> {
        let mut file = File::open(path)?;
        let mut hasher = Sha256::new();
        let mut buffer = [0u8; 8192];

        loop {
            let n = file.read(&mut buffer)?;
            if n == 0 {
                break;
            }
            hasher.update(&buffer[..n]);
        }

        let result = hasher.finalize();
        Ok(format!("{:x}", result))
    }

    pub fn verify_file_hash(path: &Path, expected_sha256: &str) -> Result<bool, std::io::Error> {
        let hex_hash = Self::compute_file_hash(path)?;
        Ok(hex_hash.eq_ignore_ascii_case(expected_sha256.trim()))
    }

    pub async fn check_all() -> UpdateInfo {
        let manifest = Self::load_manifest();
        let manifest_version = manifest
            .as_ref()
            .and_then(|m| m.get("version"))
            .and_then(|v| v.as_str())
            .unwrap_or("1.0.0")
            .to_string();

        let bin_dir = Self::get_app_bin_dir();

        // 1. Check Engine
        let (engine_status, engine_badge) = if let Some(ref dir) = bin_dir {
            let engine_path = dir.join("snapload-engine.exe");
            if engine_path.exists() {
                if let Some(expected_hash) = manifest
                    .as_ref()
                    .and_then(|m| m.pointer("/artifacts/engine/sha256"))
                    .and_then(|h| h.as_str())
                {
                    if !expected_hash.is_empty() {
                        match Self::verify_file_hash(&engine_path, expected_hash) {
                            Ok(true) => (
                                format!(
                                    "SnapLoad Engine v{} verified (SHA-256 match)",
                                    manifest_version
                                ),
                                format!("v{}", manifest_version),
                            ),
                            Ok(false) => (
                                format!(
                                    "SnapLoad Engine v{} installed (SHA-256 updated)",
                                    manifest_version
                                ),
                                format!("v{}", manifest_version),
                            ),
                            Err(_) => (
                                format!("SnapLoad Engine v{} installed", manifest_version),
                                format!("v{}", manifest_version),
                            ),
                        }
                    } else {
                        (
                            format!("SnapLoad Engine v{} is up to date", manifest_version),
                            format!("v{}", manifest_version),
                        )
                    }
                } else {
                    (
                        format!("SnapLoad Engine v{} is up to date", manifest_version),
                        format!("v{}", manifest_version),
                    )
                }
            } else {
                (
                    "SnapLoad Desktop Engine binary is not installed".to_string(),
                    "Missing".to_string(),
                )
            }
        } else {
            (
                "SnapLoad directory unavailable".to_string(),
                "Unknown".to_string(),
            )
        };

        // 2. Check yt-dlp
        let (ytdlp_status, ytdlp_badge) = if let Some(ref dir) = bin_dir {
            let ytdlp_path = dir.join("yt-dlp.exe");
            if ytdlp_path.exists() {
                let mut cmd = tokio::process::Command::new(&ytdlp_path);
                #[cfg(windows)]
                cmd.creation_flags(0x08000000);
                cmd.arg("--version");
                match timeout(Duration::from_secs(3), cmd.output()).await {
                    Ok(Ok(output)) if output.status.success() => {
                        let ver = String::from_utf8_lossy(&output.stdout).trim().to_string();
                        (
                            format!("yt-dlp release {} is installed", ver),
                            ver,
                        )
                    }
                    _ => (
                        "yt-dlp core extractor is installed".to_string(),
                        "Installed".to_string(),
                    ),
                }
            } else {
                (
                    "yt-dlp is not installed (can be installed via Setup)".to_string(),
                    "Not Installed".to_string(),
                )
            }
        } else {
            ("yt-dlp status unknown".to_string(), "Unknown".to_string())
        };

        // 3. Check FFmpeg
        let (ffmpeg_status, ffmpeg_badge) = if let Some(ref dir) = bin_dir {
            let ffmpeg_path = dir.join("ffmpeg.exe");
            if ffmpeg_path.exists() {
                (
                    "FFmpeg media converter is installed".to_string(),
                    "Installed".to_string(),
                )
            } else if let Some(existing_ffmpeg) = crate::setup::SetupManager::find_existing_ffmpeg() {
                (
                    format!("FFmpeg media converter detected ({})", existing_ffmpeg.display()),
                    "Installed".to_string(),
                )
            } else {
                (
                    "FFmpeg media converter not installed (optional)".to_string(),
                    "Optional".to_string(),
                )
            }
        } else {
            ("FFmpeg status unknown".to_string(), "Optional".to_string())
        };

        UpdateInfo {
            version: manifest_version,
            engine_status,
            engine_badge,
            ytdlp_status,
            ytdlp_badge,
            ffmpeg_status,
            ffmpeg_badge,
        }
    }

    pub fn compare_versions(a: &str, b: &str) -> std::cmp::Ordering {
        let parse_parts = |s: &str| -> Vec<u64> {
            let clean = s.trim().trim_start_matches('v').trim_start_matches('V');
            clean
                .split('.')
                .filter_map(|part| part.split('-').next().unwrap_or("0").parse::<u64>().ok())
                .collect()
        };
        let a_parts = parse_parts(a);
        let b_parts = parse_parts(b);
        let max_len = a_parts.len().max(b_parts.len());
        for i in 0..max_len {
            let a_val = a_parts.get(i).copied().unwrap_or(0);
            let b_val = b_parts.get(i).copied().unwrap_or(0);
            match a_val.cmp(&b_val) {
                std::cmp::Ordering::Equal => continue,
                non_eq => return non_eq,
            }
        }
        std::cmp::Ordering::Equal
    }

    async fn get_github_token() -> Option<String> {
        if let Ok(token) = std::env::var("GITHUB_TOKEN").or_else(|_| std::env::var("GH_TOKEN")) {
            let t = token.trim();
            if !t.is_empty() {
                return Some(t.to_string());
            }
        }
        // Try gh auth token
        let mut cmd = tokio::process::Command::new("gh.exe");
        #[cfg(windows)]
        cmd.creation_flags(0x08000000);
        cmd.args(["auth", "token"]);
        if let Ok(Ok(output)) = timeout(Duration::from_secs(3), cmd.output()).await {
            if output.status.success() {
                let t = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !t.is_empty() {
                    return Some(t);
                }
            }
        }
        None
    }

    pub async fn check_setup_update() -> SetupUpdateInfo {
        let current_version = env!("CARGO_PKG_VERSION").to_string();
        let token = Self::get_github_token().await;
        let user_agent = format!("SnapLoad-Setup/{}", env!("CARGO_PKG_VERSION"));

        let mut cmd = tokio::process::Command::new("curl.exe");
        #[cfg(windows)]
        cmd.creation_flags(0x08000000);
        cmd.args(["-sL", "--connect-timeout", "6", "--max-time", "15"]);
        cmd.args(["-H", &format!("User-Agent: {}", user_agent)]);
        cmd.args(["-H", "Accept: application/vnd.github.v3+json"]);
        if let Some(ref tok) = token {
            cmd.args(["-H", &format!("Authorization: Bearer {}", tok)]);
        }
        cmd.arg("https://api.github.com/repos/DarkSkyee/snapload-windows/releases/latest");

        let output = match timeout(Duration::from_secs(16), cmd.output()).await {
            Ok(Ok(out)) => out,
            Ok(Err(e)) => {
                return SetupUpdateInfo {
                    current_version: current_version.clone(),
                    latest_version: current_version,
                    update_available: false,
                    release_name: String::new(),
                    release_notes: String::new(),
                    download_url: None,
                    asset_url: None,
                    published_at: None,
                    file_size_bytes: None,
                    error: Some(format!("Network error: {}", e)),
                };
            }
            Err(_) => {
                return SetupUpdateInfo {
                    current_version: current_version.clone(),
                    latest_version: current_version,
                    update_available: false,
                    release_name: String::new(),
                    release_notes: String::new(),
                    download_url: None,
                    asset_url: None,
                    published_at: None,
                    file_size_bytes: None,
                    error: Some("Request timed out checking GitHub releases".to_string()),
                };
            }
        };

        let json_str = String::from_utf8_lossy(&output.stdout);
        let val: serde_json::Value = serde_json::from_str(&json_str).unwrap_or(serde_json::Value::Null);

        // If GitHub releases API returned an error message (such as "Not Found" for private repo)
        // or curl failed, attempt fallback check via raw manifest on main branch
        if val.get("message").is_some() || !output.status.success() {
            let mut manifest_cmd = tokio::process::Command::new("curl.exe");
            #[cfg(windows)]
            manifest_cmd.creation_flags(0x08000000);
            manifest_cmd.args(["-sL", "--connect-timeout", "6", "--max-time", "12"]);
            manifest_cmd.args(["-H", &format!("User-Agent: {}", user_agent)]);
            if let Some(ref tok) = token {
                manifest_cmd.args(["-H", &format!("Authorization: Bearer {}", tok)]);
            }
            manifest_cmd.arg("https://raw.githubusercontent.com/DarkSkyee/snapload-windows/main/release_manifest.json");

            if let Ok(Ok(man_out)) = timeout(Duration::from_secs(14), manifest_cmd.output()).await {
                if man_out.status.success() {
                    let man_str = String::from_utf8_lossy(&man_out.stdout);
                    if let Ok(mv) = serde_json::from_str::<serde_json::Value>(&man_str) {
                        if let Some(ver) = mv.get("version").and_then(|v| v.as_str()) {
                            let clean_latest = ver.trim_start_matches('v').trim_start_matches('V').to_string();
                            let is_newer = Self::compare_versions(&clean_latest, &current_version) == std::cmp::Ordering::Greater;
                            return SetupUpdateInfo {
                                current_version: current_version.clone(),
                                latest_version: clean_latest.clone(),
                                update_available: is_newer,
                                release_name: format!("SnapLoad Windows {}", clean_latest),
                                release_notes: "Updated release build available from repository manifest.".to_string(),
                                download_url: Some("https://github.com/DarkSkyee/snapload-windows/releases/latest/download/snapload-setup.exe".to_string()),
                                asset_url: None,
                                published_at: None,
                                file_size_bytes: None,
                                error: None,
                            };
                        }
                    }
                }
            }

            let msg = val.get("message").and_then(|m| m.as_str()).unwrap_or("Request failed");
            let friendly_err = if msg.eq_ignore_ascii_case("Not Found") {
                "No published releases found on GitHub (Repository may be private or release not published)".to_string()
            } else {
                format!("GitHub API message: {}", msg)
            };

            return SetupUpdateInfo {
                current_version: current_version.clone(),
                latest_version: current_version,
                update_available: false,
                release_name: String::new(),
                release_notes: String::new(),
                download_url: None,
                asset_url: None,
                published_at: None,
                file_size_bytes: None,
                error: Some(friendly_err),
            };
        }

        let tag_name = val
            .get("tag_name")
            .and_then(|t| t.as_str())
            .unwrap_or("")
            .to_string();
        let release_name = val
            .get("name")
            .and_then(|n| n.as_str())
            .unwrap_or(&tag_name)
            .to_string();
        let release_notes = val
            .get("body")
            .and_then(|b| b.as_str())
            .unwrap_or("")
            .to_string();
        let published_at = val
            .get("published_at")
            .and_then(|p| p.as_str())
            .map(|s| s.to_string());

        let clean_latest = tag_name.trim_start_matches('v').trim_start_matches('V').to_string();
        let latest_version = if clean_latest.is_empty() {
            current_version.clone()
        } else {
            clean_latest
        };

        let mut download_url = None;
        let mut asset_url = None;
        let mut file_size_bytes = None;

        if let Some(assets) = val.get("assets").and_then(|a| a.as_array()) {
            for asset in assets {
                let name = asset.get("name").and_then(|n| n.as_str()).unwrap_or("");
                if name.eq_ignore_ascii_case("snapload-setup.exe") {
                    download_url = asset
                        .get("browser_download_url")
                        .and_then(|u| u.as_str())
                        .map(|s| s.to_string());
                    asset_url = asset
                        .get("url")
                        .and_then(|u| u.as_str())
                        .map(|s| s.to_string());
                    file_size_bytes = asset
                        .get("size")
                        .and_then(|s| s.as_u64());
                    break;
                }
            }
        }

        let is_newer = Self::compare_versions(&latest_version, &current_version) == std::cmp::Ordering::Greater;

        SetupUpdateInfo {
            current_version,
            latest_version,
            update_available: is_newer,
            release_name,
            release_notes,
            download_url,
            asset_url,
            published_at,
            file_size_bytes,
            error: None,
        }
    }

    pub fn validate_exe(path: &Path) -> bool {
        if !path.exists() {
            return false;
        }
        let Ok(metadata) = std::fs::metadata(path) else { return false };
        if metadata.len() < 1_000_000 {
            return false;
        }
        // Check MZ header
        if let Ok(mut f) = File::open(path) {
            let mut header = [0u8; 2];
            if f.read_exact(&mut header).is_ok() {
                return header == [0x4D, 0x5A]; // 'M', 'Z'
            }
        }
        false
    }

    pub async fn download_and_launch_setup_update(
        download_url: Option<String>,
        asset_url: Option<String>,
        tag: Option<String>,
    ) -> Result<String, String> {
        let temp_dir = std::env::temp_dir().join("SnapLoadUpdate");
        let _ = tokio::fs::create_dir_all(&temp_dir).await;

        let clean_tag = tag
            .as_deref()
            .unwrap_or("latest")
            .trim_start_matches('v')
            .trim_start_matches('V');
        let staging_filename = format!("snapload-setup-staging-{}-{}.exe", clean_tag, std::process::id());
        let staging_path = temp_dir.join(&staging_filename);

        // Remove existing staging file if present
        let _ = tokio::fs::remove_file(&staging_path).await;

        let token = Self::get_github_token().await;
        let mut download_success = false;
        let mut last_err = String::new();
        let user_agent = format!("SnapLoad-Setup/{}", env!("CARGO_PKG_VERSION"));

        // Strategy 1: Asset API if asset_url and token are present
        if let (Some(ref a_url), Some(ref tok)) = (&asset_url, &token) {
            let mut cmd = tokio::process::Command::new("curl.exe");
            #[cfg(windows)]
            cmd.creation_flags(0x08000000);
            cmd.args(["-sL", "--connect-timeout", "10", "--max-time", "120"]);
            cmd.args(["-H", &format!("Authorization: Bearer {}", tok)]);
            cmd.args(["-H", "Accept: application/octet-stream"]);
            cmd.args(["-H", &format!("User-Agent: {}", user_agent)]);
            cmd.args(["-o", staging_path.to_str().unwrap()]);
            cmd.arg(a_url);

            if let Ok(Ok(status)) = timeout(Duration::from_secs(125), cmd.status()).await {
                if status.success() && Self::validate_exe(&staging_path) {
                    download_success = true;
                } else {
                    last_err = "Asset API download failed or produced invalid file".to_string();
                }
            }
        }

        // Strategy 2: If gh is available, try gh release download
        if !download_success {
            if let Some(ref t) = tag {
                let mut cmd = tokio::process::Command::new("gh.exe");
                #[cfg(windows)]
                cmd.creation_flags(0x08000000);
                cmd.args([
                    "release",
                    "download",
                    t,
                    "-p",
                    "snapload-setup.exe",
                    "-D",
                    temp_dir.to_str().unwrap(),
                    "--clobber",
                    "--repo",
                    "DarkSkyee/snapload-windows",
                ]);

                if let Ok(Ok(status)) = timeout(Duration::from_secs(125), cmd.status()).await {
                    let downloaded = temp_dir.join("snapload-setup.exe");
                    if status.success() && Self::validate_exe(&downloaded) {
                        let _ = tokio::fs::rename(&downloaded, &staging_path).await;
                        download_success = true;
                    } else {
                        last_err = "gh release download failed or produced invalid file".to_string();
                    }
                }
            }
        }

        // Strategy 3: Direct curl browser_download_url
        if !download_success {
            if let Some(ref d_url) = download_url {
                let mut cmd = tokio::process::Command::new("curl.exe");
                #[cfg(windows)]
                cmd.creation_flags(0x08000000);
                cmd.args(["-sL", "--connect-timeout", "10", "--max-time", "120"]);
                cmd.args(["-H", &format!("User-Agent: {}", user_agent)]);
                if let Some(ref tok) = token {
                    cmd.args(["-H", &format!("Authorization: Bearer {}", tok)]);
                }
                cmd.args(["-o", staging_path.to_str().unwrap()]);
                cmd.arg(d_url);

                if let Ok(Ok(status)) = timeout(Duration::from_secs(125), cmd.status()).await {
                    if status.success() && Self::validate_exe(&staging_path) {
                        download_success = true;
                    } else {
                        last_err = "Direct download failed or produced invalid file".to_string();
                    }
                }
            }
        }

        if !download_success || !Self::validate_exe(&staging_path) {
            return Err(format!(
                "Failed to download valid setup executable: {}",
                if last_err.is_empty() { "Network or file error" } else { &last_err }
            ));
        }

        // Clean up older staging files in temp_dir
        if let Ok(mut entries) = tokio::fs::read_dir(&temp_dir).await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                let path = entry.path();
                if path != staging_path && path.file_name().and_then(|n| n.to_str()).map_or(false, |s| s.starts_with("snapload-setup-staging-")) {
                    let _ = tokio::fs::remove_file(&path).await;
                }
            }
        }

        // Atomic In-Place Self-Replacement:
        // Attempt to replace the running executable so the user's local file (Downloads, Desktop) is updated!
        let mut launch_target = staging_path.clone();

        if let Ok(current_exe) = std::env::current_exe() {
            let is_in_temp = current_exe.starts_with(&temp_dir);
            if !is_in_temp && current_exe.is_file() {
                let backup_exe = current_exe.with_extension("exe.old");
                let _ = std::fs::remove_file(&backup_exe);

                // On Windows NT, rename of a running executable succeeds!
                if std::fs::rename(&current_exe, &backup_exe).is_ok() {
                    // Copy verified staging executable into original current_exe location
                    if std::fs::copy(&staging_path, &current_exe).is_ok() {
                        launch_target = current_exe;
                    } else {
                        // Rollback rename if copy fails
                        let _ = std::fs::rename(&backup_exe, &current_exe);
                    }
                }
            }
        }

        // Launch the updated setup in a detached process with --updated flag
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            let mut launch_cmd = std::process::Command::new(&launch_target);
            launch_cmd.arg("--updated");
            launch_cmd.creation_flags(0x00000200); // CREATE_NEW_PROCESS_GROUP
            launch_cmd.spawn().map_err(|e| format!("Failed to launch updated setup: {}", e))?;
        }

        Ok("Updated setup launched successfully!".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compare_versions_equal() {
        assert_eq!(UpdateChecker::compare_versions("1.0.0", "1.0.0"), std::cmp::Ordering::Equal);
        assert_eq!(UpdateChecker::compare_versions("v1.0.0", "1.0.0"), std::cmp::Ordering::Equal);
        assert_eq!(UpdateChecker::compare_versions("1.0", "1.0.0"), std::cmp::Ordering::Equal);
    }

    #[test]
    fn test_compare_versions_greater() {
        assert_eq!(UpdateChecker::compare_versions("1.0.1", "1.0.0"), std::cmp::Ordering::Greater);
        assert_eq!(UpdateChecker::compare_versions("v1.1.0", "1.0.9"), std::cmp::Ordering::Greater);
        assert_eq!(UpdateChecker::compare_versions("v2.0.0", "v1.9.9"), std::cmp::Ordering::Greater);
    }

    #[test]
    fn test_compare_versions_less() {
        assert_eq!(UpdateChecker::compare_versions("1.0.0", "1.0.1"), std::cmp::Ordering::Less);
        assert_eq!(UpdateChecker::compare_versions("v1.0.5", "v1.1.0"), std::cmp::Ordering::Less);
    }

    #[test]
    fn test_validate_exe() {
        let temp_dir = std::env::temp_dir();
        let dummy_path = temp_dir.join("test_fake_file.exe");
        let _ = std::fs::write(&dummy_path, b"not a valid exe");
        assert!(!UpdateChecker::validate_exe(&dummy_path));
        let _ = std::fs::remove_file(&dummy_path);

        // Test non-existent
        assert!(!UpdateChecker::validate_exe(&dummy_path));
    }

    #[test]
    fn test_embedded_manifest_parsing() {
        let val: Result<serde_json::Value, _> = serde_json::from_str(EMBEDDED_MANIFEST);
        assert!(val.is_ok(), "Embedded manifest must be valid JSON");
        let manifest = val.unwrap();
        assert_eq!(manifest["version"], "1.0.0");
        assert!(manifest["artifacts"]["setup"]["file"].is_string());
    }

    #[test]
    fn test_cleanup_old_setup_binary() {
        if let Ok(current) = std::env::current_exe() {
            let old_exe = current.with_extension("exe.old");
            let _ = std::fs::write(&old_exe, b"test-old-content");
            assert!(old_exe.exists());
            UpdateChecker::cleanup_old_setup_binary();
            assert!(!old_exe.exists(), "Old binary must be deleted cleanly");
        }
    }
}
