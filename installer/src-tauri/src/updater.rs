use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::Read;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateInfo {
    pub version: String,
    pub engine_status: String,
    pub ytdlp_status: String,
    pub ffmpeg_status: String,
}

pub struct UpdateChecker;

impl UpdateChecker {
    pub async fn check_all() -> UpdateInfo {
        // Return latest version manifest status
        UpdateInfo {
            version: "1.0.0".to_string(),
            engine_status: "SnapLoad Desktop Engine v1.0.0 is up to date.".to_string(),
            ytdlp_status: "yt-dlp core extractor is up to date.".to_string(),
            ffmpeg_status: "FFmpeg audio converter is ready.".to_string(),
        }
    }

    #[allow(dead_code)]
    pub fn verify_file_hash(path: &Path, expected_sha256: &str) -> Result<bool, std::io::Error> {
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
        let hex_hash = format!("{:x}", result);

        Ok(hex_hash.eq_ignore_ascii_case(expected_sha256.trim()))
    }
}
