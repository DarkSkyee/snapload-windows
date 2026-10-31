use crate::config::EngineConfig;
use crate::dependency::DependencyManager;
use crate::logger::Logger;
use crate::protocol::{
    AudioFormat, AudioQuality, EngineResponse, MediaType, ResponseType, StartDownloadPayload,
    VideoFormat, VideoQuality,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::{mpsc, Mutex};
use url::Url;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressUpdate {
    pub download_id: String,
    pub percent: f32,
    pub speed: String,
    pub eta: String,
    pub total_bytes: u64,
    pub downloaded_bytes: u64,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletedPayload {
    pub download_id: String,
    pub file_path: String,
    pub file_name: String,
    pub file_size_bytes: u64,
    pub extension: String,
    pub fallback_to_audio: bool,
}

struct ActiveDownload {
    child_id: u32,
    cancel_tx: Option<tokio::sync::oneshot::Sender<()>>,
}

#[derive(Clone)]
pub struct DownloadManager {
    active_downloads: Arc<Mutex<HashMap<String, ActiveDownload>>>,
}

impl Default for DownloadManager {
    fn default() -> Self {
        Self::new()
    }
}

impl DownloadManager {
    pub fn new() -> Self {
        Self {
            active_downloads: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn validate_url(raw_url: &str) -> Result<String, &'static str> {
        if raw_url.contains('\0') || raw_url.contains('\r') || raw_url.contains('\n') {
            return Err("URL contains forbidden control characters");
        }
        let trimmed = raw_url.trim();
        if trimmed.is_empty() {
            return Err("URL is empty");
        }
        if trimmed.starts_with('-') {
            return Err("URL cannot start with a hyphen or flag prefix");
        }
        let parsed = Url::parse(trimmed).map_err(|_| "Invalid URL format")?;
        let scheme = parsed.scheme().to_lowercase();
        if scheme != "http" && scheme != "https" {
            return Err("Only HTTP and HTTPS URLs are allowed");
        }
        if parsed.host_str().is_none() {
            return Err("URL must specify a host");
        }
        Ok(trimmed.to_string())
    }

    pub fn validate_output_dir(path_str: &str) -> Result<PathBuf, &'static str> {
        if path_str.contains('\0') || path_str.contains('\r') || path_str.contains('\n') {
            return Err("Custom output directory contains forbidden control characters");
        }
        let trimmed = path_str.trim();
        if trimmed.is_empty() {
            return Err("Custom output directory cannot be empty");
        }
        if trimmed.starts_with(r"\\") || trimmed.starts_with("//") {
            return Err("UNC network paths are not permitted");
        }
        if trimmed.contains("..") {
            return Err(
                "Custom output directory must not contain path traversal components ('..')",
            );
        }
        let p = Path::new(trimmed);
        if !p.is_absolute() {
            return Err("Custom output directory must be an absolute path");
        }
        let s = trimmed.as_bytes();
        if !(s.len() >= 3
            && s[0].is_ascii_alphabetic()
            && s[1] == b':'
            && (s[2] == b'\\' || s[2] == b'/'))
        {
            return Err("Custom output directory must be a local drive path (e.g. C:\\...)");
        }
        Ok(PathBuf::from(trimmed))
    }

    pub fn validate_cookies_file(file_str: &str) -> Result<PathBuf, &'static str> {
        if file_str.contains('\0') || file_str.contains('\r') || file_str.contains('\n') {
            return Err("Cookies file contains forbidden control characters");
        }
        let trimmed = file_str.trim();
        if trimmed.is_empty() {
            return Err("Cookies file path cannot be empty");
        }
        if trimmed.starts_with('-') {
            return Err("Cookies file cannot start with a hyphen or flag prefix");
        }
        if trimmed.starts_with(r"\\") || trimmed.starts_with("//") {
            return Err("UNC network paths are not permitted for cookies");
        }
        if trimmed.contains("..") {
            return Err("Cookies file must not contain path traversal ('..')");
        }
        let p = Path::new(trimmed);
        if !p.is_absolute() {
            return Err("Cookies file path must be an absolute path");
        }
        let s = trimmed.as_bytes();
        if !(s.len() >= 3
            && s[0].is_ascii_alphabetic()
            && s[1] == b':'
            && (s[2] == b'\\' || s[2] == b'/'))
        {
            return Err("Cookies file must be on a local drive (e.g. C:\\...)");
        }
        if !p.is_file() {
            return Err("Cookies file does not exist or is not a regular file");
        }
        Ok(p.to_path_buf())
    }

    pub async fn cancel_download(&self, download_id: &str) -> bool {
        let mut map = self.active_downloads.lock().await;
        if let Some(mut active) = map.remove(download_id) {
            Logger::info(&format!(
                "Cancelling download {}, killing PID {}",
                download_id, active.child_id
            ));
            if let Some(tx) = active.cancel_tx.take() {
                let _ = tx.send(());
            }
            #[cfg(windows)]
            {
                // Force kill process tree on Windows using taskkill
                let _ = Command::new("taskkill")
                    .args(["/F", "/T", "/PID", &active.child_id.to_string()])
                    .output()
                    .await;
            }
            return true;
        }
        false
    }

    pub async fn start_download(
        &self,
        request_id: String,
        payload: StartDownloadPayload,
        config: EngineConfig,
        response_tx: mpsc::Sender<EngineResponse>,
    ) {
        let download_id = request_id.clone();
        if !download_id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
            || download_id.is_empty()
        {
            let _ = response_tx
                .send(EngineResponse::err(
                    &download_id,
                    "INVALID_ID",
                    "Download ID must contain only alphanumeric characters, hyphens, and underscores",
                ))
                .await;
            return;
        }

        let valid_url = match Self::validate_url(&payload.url) {
            Ok(u) => u,
            Err(e) => {
                let _ = response_tx
                    .send(EngineResponse::err(&download_id, "INVALID_URL", e))
                    .await;
                return;
            }
        };

        let output_dir = if let Some(ref custom) = payload.custom_output_dir {
            match Self::validate_output_dir(custom) {
                Ok(p) => p,
                Err(e) => {
                    let _ = response_tx
                        .send(EngineResponse::err(&download_id, "INVALID_PATH", e))
                        .await;
                    return;
                }
            }
        } else {
            config.download_directory.clone()
        };

        let validated_cookies = if let Some(ref cf) = payload.cookies_file {
            match Self::validate_cookies_file(cf) {
                Ok(p) => Some(p),
                Err(e) => {
                    let _ = response_tx
                        .send(EngineResponse::err(&download_id, "INVALID_COOKIES", e))
                        .await;
                    return;
                }
            }
        } else {
            None
        };

        let ytdlp_path = match DependencyManager::find_ytdlp(&config) {
            Some(p) => p,
            None => {
                let _ = response_tx.send(EngineResponse::err(
                    &download_id,
                    "DEPENDENCY_MISSING",
                    "yt-dlp is not installed or could not be found. Please run SnapLoad Setup to install dependencies.",
                )).await;
                return;
            }
        };

        let ffmpeg_path = DependencyManager::find_ffmpeg(&config);

        if let Err(e) = fs::create_dir_all(&output_dir) {
            let _ = response_tx
                .send(EngineResponse::err(
                    &download_id,
                    "IO_ERROR",
                    &format!("Failed to create download directory: {}", e),
                ))
                .await;
            return;
        }

        // Notify client download has started
        let _ = response_tx
            .send(EngineResponse::ok(
                &download_id,
                ResponseType::DownloadStarted,
                serde_json::json!({
                    "download_id": &download_id,
                    "url": &valid_url,
                    "output_dir": output_dir.to_string_lossy(),
                }),
            ))
            .await;

        let manager = self.clone();
        tokio::spawn(async move {
            manager
                .execute_download(
                    download_id,
                    valid_url,
                    payload,
                    validated_cookies,
                    ytdlp_path,
                    ffmpeg_path,
                    output_dir,
                    response_tx,
                )
                .await;
        });
    }

    #[allow(clippy::too_many_arguments)]
    async fn execute_download(
        &self,
        download_id: String,
        url: String,
        payload: StartDownloadPayload,
        validated_cookies: Option<PathBuf>,
        ytdlp_path: PathBuf,
        ffmpeg_path: Option<PathBuf>,
        output_dir: PathBuf,
        response_tx: mpsc::Sender<EngineResponse>,
    ) {
        let media_type = payload.media_type.unwrap_or(MediaType::Video);
        let video_fmt = payload.video_format.unwrap_or(VideoFormat::Mp4);
        let video_qual = payload.video_quality.unwrap_or(VideoQuality::Best);
        let audio_fmt = payload.audio_format.unwrap_or(AudioFormat::Mp3);
        let audio_qual = payload.audio_quality.unwrap_or(AudioQuality::Best);

        let unique_prefix = format!("snapload_{}", download_id);
        let output_template = output_dir.join(format!("{}_%(title).100s.%(ext)s", unique_prefix));

        let mut args: Vec<String> = vec![
            "-o".to_string(),
            output_template.to_string_lossy().to_string(),
            "--no-mtime".to_string(),
            "--no-playlist".to_string(),
            "--trim-filenames".to_string(),
            "100".to_string(),
            "--restrict-filenames".to_string(),
            "--newline".to_string(),
            "--no-colors".to_string(),
            "--progress-template".to_string(),
            "SNAPLOAD_PROG|%(progress._percent_str)s|%(progress._eta_str)s|%(progress._speed_str)s|%(progress.total_bytes)s|%(progress.downloaded_bytes)s".to_string(),
        ];

        // Custom cookies support (pre-validated against flags, traversal, and UNC paths)
        if let Some(ref cp) = validated_cookies {
            args.push("--cookies".to_string());
            args.push(cp.to_string_lossy().to_string());
        }

        // FFmpeg integration
        if let Some(ref ffmpeg) = ffmpeg_path {
            if let Some(dir) = ffmpeg.parent() {
                args.push("--ffmpeg-location".to_string());
                args.push(dir.to_string_lossy().to_string());
            }
        }

        if media_type == MediaType::Video {
            args.push("-f".to_string());
            args.push(video_qual.ytdl_format_selector().to_string());
            args.push("--merge-output-format".to_string());
            args.push(video_fmt.extension().to_string());
            args.push("--remux-video".to_string());
            args.push(video_fmt.extension().to_string());
        } else {
            args.push("-x".to_string());
            args.push("--audio-format".to_string());
            args.push(audio_fmt.extension().to_string());
            args.push("--audio-quality".to_string());
            args.push(audio_qual.ytdl_quality().to_string());
        }

        // Positional argument delimiter prevents option parsing confusion
        args.push("--".to_string());
        args.push(url.clone());

        Logger::info(&format!("Spawning yt-dlp for {}: {:?}", download_id, args));

        let mut cmd = Command::new(&ytdlp_path);
        cmd.args(&args);
        cmd.stdin(std::process::Stdio::null());
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());

        let mut child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => {
                Logger::error(&format!("Failed to spawn yt-dlp: {}", e));
                let _ = response_tx
                    .send(EngineResponse::err(
                        &download_id,
                        "SPAWN_FAILED",
                        &e.to_string(),
                    ))
                    .await;
                return;
            }
        };

        let child_pid = child.id().unwrap_or(0);
        let (cancel_tx, mut cancel_rx) = tokio::sync::oneshot::channel::<()>();

        {
            let mut map = self.active_downloads.lock().await;
            map.insert(
                download_id.clone(),
                ActiveDownload {
                    child_id: child_pid,
                    cancel_tx: Some(cancel_tx),
                },
            );
        }

        let stdout = child.stdout.take();
        let stderr = child.stderr.take();

        let tx_progress = response_tx.clone();
        let id_prog = download_id.clone();

        // Task reading stdout for progress updates
        let stdout_handle = tokio::spawn(async move {
            if let Some(pipe) = stdout {
                let mut reader = BufReader::new(pipe).lines();
                while let Ok(Some(line)) = reader.next_line().await {
                    if let Some(prog) = Self::parse_progress_line(&id_prog, &line) {
                        let _ = tx_progress
                            .send(EngineResponse::ok(
                                &id_prog,
                                ResponseType::DownloadProgress,
                                serde_json::to_value(prog).unwrap_or_default(),
                            ))
                            .await;
                    }
                }
            }
        });

        // Task capturing stderr for diagnostics
        let stderr_handle = tokio::spawn(async move {
            let mut err_log = Vec::new();
            if let Some(pipe) = stderr {
                let mut reader = BufReader::new(pipe).lines();
                while let Ok(Some(line)) = reader.next_line().await {
                    Logger::debug(&format!("[yt-dlp stderr] {}", line));
                    err_log.push(line);
                }
            }
            err_log
        });

        let wait_result = tokio::select! {
            res = child.wait() => Ok(res),
            _ = &mut cancel_rx => {
                let _ = child.kill().await;
                Err("cancelled")
            }
        };

        let _ = stdout_handle.await;
        let stderr_lines = stderr_handle.await.unwrap_or_default();

        // Remove from active downloads
        {
            let mut map = self.active_downloads.lock().await;
            map.remove(&download_id);
        }

        match wait_result {
            Ok(Ok(status)) if status.success() => {
                Logger::info(&format!(
                    "Download {} completed with exit status success",
                    download_id
                ));
                // Locate the finished file
                if let Some(file_info) = Self::find_output_file(&output_dir, &unique_prefix) {
                    let _ = response_tx
                        .send(EngineResponse::ok(
                            &download_id,
                            ResponseType::DownloadCompleted,
                            serde_json::to_value(CompletedPayload {
                                download_id: download_id.clone(),
                                file_path: file_info.path.to_string_lossy().to_string(),
                                file_name: file_info.name,
                                file_size_bytes: file_info.size,
                                extension: file_info.extension,
                                fallback_to_audio: false,
                            })
                            .unwrap_or_default(),
                        ))
                        .await;
                } else {
                    let _ = response_tx
                        .send(EngineResponse::err(
                            &download_id,
                            "FILE_NOT_FOUND",
                            "yt-dlp finished but the output file could not be verified on disk.",
                        ))
                        .await;
                }
            }
            Ok(Ok(status)) => {
                let err_msg = stderr_lines.join("\n");
                Logger::error(&format!("yt-dlp failed (status {}): {}", status, err_msg));

                // Check for fallback condition (e.g. video requested but source is audio-only)
                let is_format_error = err_msg.contains("Requested format")
                    || err_msg.contains("format not available")
                    || err_msg.contains("no video formats")
                    || err_msg.contains("audio only");

                if media_type == MediaType::Video && is_format_error {
                    Logger::warn(&format!("Attempting audio fallback for {}", download_id));
                    self.execute_audio_fallback(
                        download_id,
                        url,
                        validated_cookies,
                        ytdlp_path,
                        ffmpeg_path,
                        output_dir,
                        unique_prefix,
                        response_tx,
                    )
                    .await;
                } else {
                    let _ = response_tx
                        .send(EngineResponse::err(
                            &download_id,
                            "DOWNLOAD_FAILED",
                            if err_msg.is_empty() {
                                "Download process failed without error details"
                            } else {
                                &err_msg
                            },
                        ))
                        .await;
                }
            }
            Ok(Err(e)) => {
                Logger::error(&format!("Error waiting for child process: {}", e));
                let _ = response_tx
                    .send(EngineResponse::err(
                        &download_id,
                        "PROCESS_ERROR",
                        &e.to_string(),
                    ))
                    .await;
            }
            Err(_) => {
                Logger::info(&format!(
                    "Download {} was successfully cancelled",
                    download_id
                ));
                Self::cleanup_incomplete_files(&output_dir, &unique_prefix);
                let _ = response_tx
                    .send(EngineResponse::ok(
                        &download_id,
                        ResponseType::DownloadCancelled,
                        serde_json::json!({ "download_id": download_id }),
                    ))
                    .await;
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    async fn execute_audio_fallback(
        &self,
        download_id: String,
        url: String,
        validated_cookies: Option<PathBuf>,
        ytdlp_path: PathBuf,
        ffmpeg_path: Option<PathBuf>,
        output_dir: PathBuf,
        unique_prefix: String,
        response_tx: mpsc::Sender<EngineResponse>,
    ) {
        let output_template = output_dir.join(format!("{}.%(ext)s", unique_prefix));
        let mut args: Vec<String> = vec![
            "-o".to_string(),
            output_template.to_string_lossy().to_string(),
            "--no-mtime".to_string(),
            "--no-playlist".to_string(),
            "--trim-filenames".to_string(),
            "100".to_string(),
            "--restrict-filenames".to_string(),
            "--no-colors".to_string(),
            "-x".to_string(),
            "--audio-format".to_string(),
            "mp3".to_string(),
            "--audio-quality".to_string(),
            "0".to_string(),
        ];

        if let Some(ref cp) = validated_cookies {
            args.push("--cookies".to_string());
            args.push(cp.to_string_lossy().to_string());
        }

        if let Some(ffmpeg) = ffmpeg_path {
            if let Some(dir) = ffmpeg.parent() {
                args.push("--ffmpeg-location".to_string());
                args.push(dir.to_string_lossy().to_string());
            }
        }

        // Positional argument delimiter prevents option parsing confusion
        args.push("--".to_string());
        args.push(url);

        let mut cmd = Command::new(&ytdlp_path);
        cmd.args(&args);
        cmd.stdin(std::process::Stdio::null());
        cmd.stdout(std::process::Stdio::null());
        cmd.stderr(std::process::Stdio::null());

        let mut child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => {
                let _ = response_tx
                    .send(EngineResponse::err(
                        &download_id,
                        "SPAWN_FAILED",
                        &e.to_string(),
                    ))
                    .await;
                return;
            }
        };

        let child_pid = child.id().unwrap_or(0);
        let (cancel_tx, mut cancel_rx) = tokio::sync::oneshot::channel::<()>();

        {
            let mut map = self.active_downloads.lock().await;
            map.insert(
                download_id.clone(),
                ActiveDownload {
                    child_id: child_pid,
                    cancel_tx: Some(cancel_tx),
                },
            );
        }

        let wait_result = tokio::select! {
            res = child.wait() => Ok(res),
            _ = &mut cancel_rx => {
                let _ = child.kill().await;
                Err("cancelled")
            }
        };

        {
            let mut map = self.active_downloads.lock().await;
            map.remove(&download_id);
        }

        match wait_result {
            Ok(Ok(status)) if status.success() => {
                if let Some(file_info) = Self::find_output_file(&output_dir, &unique_prefix) {
                    let _ = response_tx
                        .send(EngineResponse::ok(
                            &download_id,
                            ResponseType::DownloadCompleted,
                            serde_json::to_value(CompletedPayload {
                                download_id: download_id.clone(),
                                file_path: file_info.path.to_string_lossy().to_string(),
                                file_name: file_info.name,
                                file_size_bytes: file_info.size,
                                extension: file_info.extension,
                                fallback_to_audio: true,
                            })
                            .unwrap_or_default(),
                        ))
                        .await;
                    return;
                }
            }
            Ok(_) => {}
            Err(_) => {
                Logger::info(&format!(
                    "Fallback audio download {} was cancelled",
                    download_id
                ));
                Self::cleanup_incomplete_files(&output_dir, &unique_prefix);
                let _ = response_tx
                    .send(EngineResponse::ok(
                        &download_id,
                        ResponseType::DownloadCancelled,
                        serde_json::json!({ "download_id": download_id }),
                    ))
                    .await;
                return;
            }
        }

        let _ = response_tx
            .send(EngineResponse::err(
                &download_id,
                "DOWNLOAD_FAILED",
                "Video stream was unavailable and fallback to audio also failed.",
            ))
            .await;
    }

    fn parse_progress_line(download_id: &str, line: &str) -> Option<ProgressUpdate> {
        if line.starts_with("SNAPLOAD_PROG|") {
            let parts: Vec<&str> = line.split('|').collect();
            if parts.len() >= 6 {
                let percent_raw = parts[1].trim().trim_end_matches('%');
                let percent = percent_raw.parse::<f32>().unwrap_or(0.0);
                let eta = parts[2].trim().to_string();
                let speed = parts[3].trim().to_string();
                let total_bytes = parts[4].trim().parse::<u64>().unwrap_or(0);
                let downloaded_bytes = parts[5].trim().parse::<u64>().unwrap_or(0);

                return Some(ProgressUpdate {
                    download_id: download_id.to_string(),
                    percent,
                    speed,
                    eta,
                    total_bytes,
                    downloaded_bytes,
                    status: "Downloading...".to_string(),
                });
            }
        }
        // Fallback parser for standard [download] lines
        if line.starts_with("[download]") && line.contains('%') {
            let tokens: Vec<&str> = line.split_whitespace().collect();
            for (idx, token) in tokens.iter().enumerate() {
                if token.ends_with('%') {
                    if let Ok(pct) = token.trim_end_matches('%').parse::<f32>() {
                        let speed = tokens.get(idx + 4).cloned().unwrap_or("").to_string();
                        let eta = tokens.get(idx + 6).cloned().unwrap_or("").to_string();
                        return Some(ProgressUpdate {
                            download_id: download_id.to_string(),
                            percent: pct,
                            speed,
                            eta,
                            total_bytes: 0,
                            downloaded_bytes: 0,
                            status: "Downloading...".to_string(),
                        });
                    }
                }
            }
        }
        None
    }

    fn find_output_file(dir: &Path, prefix: &str) -> Option<DiscoveredFile> {
        let entries = fs::read_dir(dir).ok()?;
        let mut candidates = Vec::new();

        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(file_name) = path.file_name().and_then(|s| s.to_str()) {
                    if file_name.starts_with(prefix) {
                        let ext = path
                            .extension()
                            .and_then(|s| s.to_str())
                            .unwrap_or("")
                            .to_lowercase();
                        if ext != "part" && ext != "ytdl" && ext != "temp" && ext != "aria2" {
                            if let Ok(meta) = entry.metadata() {
                                if meta.len() > 0 {
                                    candidates.push(DiscoveredFile {
                                        path: path.clone(),
                                        name: file_name.to_string(),
                                        size: meta.len(),
                                        extension: ext,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        candidates.into_iter().max_by_key(|f| f.size)
    }

    fn cleanup_incomplete_files(dir: &Path, prefix: &str) {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                    if name.starts_with(prefix) {
                        let _ = fs::remove_file(path);
                    }
                }
            }
        }
    }
}

struct DiscoveredFile {
    path: PathBuf,
    name: String,
    size: u64,
    extension: String,
}
