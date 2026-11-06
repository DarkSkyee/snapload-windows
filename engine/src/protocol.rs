use serde::{Deserialize, Serialize};
use std::io;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub const MAX_MESSAGE_SIZE: u32 = 1024 * 1024; // 1 MB limit (Chrome standard)

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MediaType {
    Video,
    Audio,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum VideoFormat {
    Mp4,
    Mkv,
    Webm,
}

impl VideoFormat {
    pub fn extension(&self) -> &'static str {
        match self {
            VideoFormat::Mp4 => "mp4",
            VideoFormat::Mkv => "mkv",
            VideoFormat::Webm => "webm",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum VideoQuality {
    Best,
    P1080,
    P720,
    P480,
}

impl VideoQuality {
    pub fn ytdl_format_selector(&self) -> &'static str {
        match self {
            VideoQuality::Best => "bestvideo+bestaudio[ext=m4a]/bestvideo+bestaudio/best",
            VideoQuality::P1080 => "bestvideo[height<=1080]+bestaudio[ext=m4a]/bestvideo[height<=1080]+bestaudio/best[height<=1080]/best",
            VideoQuality::P720 => "bestvideo[height<=720]+bestaudio[ext=m4a]/bestvideo[height<=720]+bestaudio/best[height<=720]/best",
            VideoQuality::P480 => "bestvideo[height<=480]+bestaudio[ext=m4a]/bestvideo[height<=480]+bestaudio/best[height<=480]/best",
        }
    }

    /// Single-stream pre-merged format selector used when FFmpeg is not installed/available.
    /// This ensures yt-dlp never downloads separate video-only / audio-only streams that cannot be merged.
    pub fn ytdl_format_selector_fallback(&self) -> &'static str {
        match self {
            VideoQuality::Best => "best[ext=mp4][vcodec!=none][acodec!=none]/best[vcodec!=none][acodec!=none]/best",
            VideoQuality::P1080 => "best[height<=1080][ext=mp4][vcodec!=none][acodec!=none]/best[height<=1080][vcodec!=none][acodec!=none]/best",
            VideoQuality::P720 => "best[height<=720][ext=mp4][vcodec!=none][acodec!=none]/best[height<=720][vcodec!=none][acodec!=none]/best",
            VideoQuality::P480 => "best[height<=480][ext=mp4][vcodec!=none][acodec!=none]/best[height<=480][vcodec!=none][acodec!=none]/best",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AudioFormat {
    Mp3,
    M4a,
    Flac,
    Wav,
    Opus,
}

impl AudioFormat {
    pub fn extension(&self) -> &'static str {
        match self {
            AudioFormat::Mp3 => "mp3",
            AudioFormat::M4a => "m4a",
            AudioFormat::Flac => "flac",
            AudioFormat::Wav => "wav",
            AudioFormat::Opus => "opus",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AudioQuality {
    Best,
    High,
    Medium,
    Low,
}

impl AudioQuality {
    pub fn ytdl_quality(&self) -> &'static str {
        match self {
            AudioQuality::Best => "0",
            AudioQuality::High => "2",
            AudioQuality::Medium => "4",
            AudioQuality::Low => "6",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartDownloadPayload {
    pub url: String,
    #[serde(default)]
    pub media_type: Option<MediaType>,
    #[serde(default)]
    pub video_format: Option<VideoFormat>,
    #[serde(default)]
    pub video_quality: Option<VideoQuality>,
    #[serde(default)]
    pub audio_format: Option<AudioFormat>,
    #[serde(default)]
    pub audio_quality: Option<AudioQuality>,
    #[serde(default)]
    pub custom_output_dir: Option<String>,
    #[serde(default)]
    pub cookies_file: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelDownloadPayload {
    pub download_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", content = "payload", rename_all = "snake_case")]
pub enum RequestAction {
    #[serde(alias = "PING")]
    Ping,
    #[serde(alias = "GET_VERSION")]
    GetVersion,
    #[serde(alias = "CHECK_DEPENDENCIES")]
    CheckDependencies,
    #[serde(alias = "GET_CONFIG")]
    GetConfig,
    #[serde(alias = "START_DOWNLOAD")]
    StartDownload(StartDownloadPayload),
    #[serde(alias = "CANCEL_DOWNLOAD")]
    CancelDownload(CancelDownloadPayload),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineRequest {
    pub id: String,
    #[serde(default = "default_version")]
    pub version: String,
    #[serde(flatten)]
    pub action: RequestAction,
}

fn default_version() -> String {
    "1.0".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResponseType {
    Pong,
    VersionInfo,
    DependencyStatus,
    Config,
    DownloadStarted,
    DownloadProgress,
    DownloadCompleted,
    DownloadFailed,
    DownloadCancelled,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorInfo {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineResponse {
    pub id: String,
    pub version: String,
    #[serde(rename = "type")]
    pub response_type: ResponseType,
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorInfo>,
}

impl EngineResponse {
    pub fn ok(id: &str, response_type: ResponseType, data: serde_json::Value) -> Self {
        Self {
            id: id.to_string(),
            version: "1.0".to_string(),
            response_type,
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn err(id: &str, code: &str, message: &str) -> Self {
        Self {
            id: id.to_string(),
            version: "1.0".to_string(),
            response_type: ResponseType::Error,
            success: false,
            data: None,
            error: Some(ErrorInfo {
                code: code.to_string(),
                message: message.to_string(),
            }),
        }
    }
}

pub struct NativeMessageIo;

impl NativeMessageIo {
    /// Read a Native Messaging packet: 4-byte little-endian length followed by UTF-8 JSON.
    /// Returns Ok(None) upon EOF.
    pub async fn read_message<R: AsyncReadExt + Unpin>(
        reader: &mut R,
    ) -> io::Result<Option<String>> {
        let mut length_buf = [0u8; 4];
        match reader.read_exact(&mut length_buf).await {
            Ok(_) => {}
            Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => return Ok(None),
            Err(e) => return Err(e),
        }

        let message_len = u32::from_ne_bytes(length_buf);
        if message_len == 0 {
            return Ok(Some(String::new()));
        }
        if message_len > MAX_MESSAGE_SIZE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "Message length {} exceeds max allowed limit of {}",
                    message_len, MAX_MESSAGE_SIZE
                ),
            ));
        }

        let mut body = vec![0u8; message_len as usize];
        reader.read_exact(&mut body).await?;

        let text = String::from_utf8(body).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Invalid UTF-8 payload: {}", e),
            )
        })?;

        Ok(Some(text))
    }

    /// Write a Native Messaging packet: 4-byte little-endian length followed by UTF-8 bytes.
    pub async fn write_message<W: AsyncWriteExt + Unpin>(
        writer: &mut W,
        json_str: &str,
    ) -> io::Result<()> {
        let bytes = json_str.as_bytes();
        let len = bytes.len() as u32;
        if len > MAX_MESSAGE_SIZE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "Outgoing message length {} exceeds max allowed limit of {}",
                    len, MAX_MESSAGE_SIZE
                ),
            ));
        }
        let len_buf = len.to_ne_bytes();

        writer.write_all(&len_buf).await?;
        writer.write_all(bytes).await?;
        writer.flush().await?;
        Ok(())
    }
}
