use crate::protocol::{AudioFormat, AudioQuality, MediaType, VideoFormat, VideoQuality};
use serde::{Deserialize, Serialize};
use std::fs::{create_dir_all, File};
use std::io::{Read, Write};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineConfig {
    pub download_directory: PathBuf,
    pub default_media_type: MediaType,
    pub default_video_format: VideoFormat,
    pub default_video_quality: VideoQuality,
    pub default_audio_format: AudioFormat,
    pub default_audio_quality: AudioQuality,
    pub ytdlp_custom_path: Option<PathBuf>,
    pub ffmpeg_custom_path: Option<PathBuf>,
}

impl Default for EngineConfig {
    fn default() -> Self {
        let download_dir = if let Some(user_profile) = std::env::var_os("USERPROFILE") {
            let mut p = PathBuf::from(user_profile);
            p.push("Downloads");
            p
        } else {
            PathBuf::from("downloads")
        };

        Self {
            download_directory: download_dir,
            default_media_type: MediaType::Video,
            default_video_format: VideoFormat::Mp4,
            default_video_quality: VideoQuality::Best,
            default_audio_format: AudioFormat::Mp3,
            default_audio_quality: AudioQuality::Best,
            ytdlp_custom_path: None,
            ffmpeg_custom_path: None,
        }
    }
}

impl EngineConfig {
    pub fn config_path() -> Option<PathBuf> {
        let local_app_data = std::env::var_os("LOCALAPPDATA")?;
        let mut path = PathBuf::from(local_app_data);
        path.push("SnapLoad");
        let _ = create_dir_all(&path);
        path.push("config.json");
        Some(path)
    }

    pub fn load() -> Self {
        if let Some(path) = Self::config_path() {
            if path.exists() {
                if let Ok(mut file) = File::open(&path) {
                    let mut content = String::new();
                    if file.read_to_string(&mut content).is_ok() {
                        if let Ok(mut cfg) = serde_json::from_str::<EngineConfig>(&content) {
                            if cfg.download_directory.ends_with("SnapLoad") {
                                if let Some(parent) = cfg.download_directory.parent() {
                                    if parent.ends_with("Downloads") {
                                        cfg.download_directory = parent.to_path_buf();
                                        let _ = cfg.save();
                                    }
                                }
                            }
                            return cfg;
                        }
                    }
                }
            }
        }
        let default_cfg = Self::default();
        let _ = default_cfg.save();
        default_cfg
    }

    pub fn save(&self) -> Result<(), std::io::Error> {
        if let Some(path) = Self::config_path() {
            let json = serde_json::to_string_pretty(self)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
            let mut file = File::create(path)?;
            file.write_all(json.as_bytes())?;
        }
        Ok(())
    }
}
