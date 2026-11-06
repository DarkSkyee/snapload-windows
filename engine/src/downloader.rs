use crate::config::EngineConfig;
use crate::dependency::DependencyManager;
use crate::logger::Logger;
use crate::protocol::{
    AudioFormat, AudioQuality, EngineResponse, MediaType, ResponseType, StartDownloadPayload,
    VideoFormat, VideoQuality,
};
use crate::spotify::{SpotifyResolver, SpotifyTrackMeta};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
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
    cancelled_ids: Arc<Mutex<HashSet<String>>>,
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
            cancelled_ids: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    pub fn is_spotify_url(url: &str) -> bool {
        SpotifyResolver::is_spotify_url(url)
    }

    pub fn is_soundcloud_url(url: &str) -> bool {
        SpotifyResolver::is_soundcloud_url(url)
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
        if scheme != "http" && scheme != "https" && scheme != "spotify" {
            return Err("Only HTTP, HTTPS and Spotify URLs are allowed");
        }
        if scheme != "spotify" && parsed.host_str().is_none() {
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
        self.cancelled_ids.lock().await.insert(download_id.to_string());
        let mut map = self.active_downloads.lock().await;
        if let Some(mut active) = map.remove(download_id) {
            Logger::info(&format!(
                "Cancelling download {}, killing PID {}",
                download_id, active.child_id
            ));
            if let Some(tx) = active.cancel_tx.take() {
                let _ = tx.send(());
            }
            if active.child_id > 0 {
                #[cfg(windows)]
                {
                    // Force kill process tree on Windows using taskkill
                    let taskkill_bin = if Path::new(r"C:\Windows\System32\taskkill.exe").exists() {
                        r"C:\Windows\System32\taskkill.exe"
                    } else {
                        "taskkill.exe"
                    };
                    let mut cmd = Command::new(taskkill_bin);
                    cmd.args(["/F", "/T", "/PID", &active.child_id.to_string()]);
                    cmd.creation_flags(0x08000000);
                    let _ = cmd.output().await;
                }
            }
            return true;
        }
        true
    }

    pub async fn is_cancelled(&self, download_id: &str) -> bool {
        self.cancelled_ids.lock().await.contains(download_id)
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

        // Immediately notify client that download request has been accepted
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

        let ytdlp_path = match DependencyManager::find_ytdlp(&config) {
            Some(p) => p,
            None => {
                Logger::warn(&format!(
                    "yt-dlp not found locally for download {}. Attempting automated download...",
                    download_id
                ));
                let _ = response_tx
                    .send(EngineResponse::ok(
                        &download_id,
                        ResponseType::DownloadProgress,
                        serde_json::json!({
                            "download_id": &download_id,
                            "percent": 0.0,
                            "speed": "",
                            "eta": "",
                            "total_bytes": 0,
                            "downloaded_bytes": 0,
                            "status": "Setting up yt-dlp core extractor...",
                        }),
                    ))
                    .await;
                match DependencyManager::auto_download_ytdlp().await {
                    Some(p) => p,
                    None => {
                        let _ = response_tx.send(EngineResponse::err(
                            &download_id,
                            "DEPENDENCY_MISSING",
                            "yt-dlp is not installed and automatic download failed. Please run SnapLoad Setup to configure dependencies.",
                        )).await;
                        return;
                    }
                }
            }
        };

        let ffmpeg_path = DependencyManager::find_ffmpeg(&config);

        let manager = self.clone();
        tokio::spawn(async move {
            let active_ffmpeg = if ffmpeg_path.is_none() {
                Logger::warn(&format!(
                    "FFmpeg not found locally for download {}. Attempting automated download...",
                    download_id
                ));
                DependencyManager::auto_download_ffmpeg().await
            } else {
                ffmpeg_path
            };

            manager
                .execute_download(
                    download_id,
                    valid_url,
                    payload,
                    validated_cookies,
                    ytdlp_path,
                    active_ffmpeg,
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
        let is_spotify = Self::is_spotify_url(&url);
        let is_soundcloud = Self::is_soundcloud_url(&url);

        let media_type = if is_spotify || is_soundcloud {
            MediaType::Audio
        } else {
            payload.media_type.unwrap_or(MediaType::Video)
        };
        let video_fmt = payload.video_format.unwrap_or(VideoFormat::Mp4);
        let video_qual = payload.video_quality.unwrap_or(VideoQuality::Best);
        let audio_fmt = payload.audio_format.unwrap_or(AudioFormat::Mp3);
        let audio_qual = payload.audio_quality.unwrap_or(AudioQuality::Best);

        let mut target_url = url.clone();
        let mut spotify_meta: Option<SpotifyTrackMeta> = None;

        if is_spotify {
            let _ = response_tx
                .send(EngineResponse::ok(
                    &download_id,
                    ResponseType::DownloadProgress,
                    serde_json::json!({
                        "download_id": &download_id,
                        "percent": 0.0,
                        "speed": "",
                        "eta": "",
                        "total_bytes": 0,
                        "downloaded_bytes": 0,
                        "status": "Resolving Spotify track metadata...",
                    }),
                ))
                .await;

            let resolver = SpotifyResolver::new();
            if let Some(track) = resolver.resolve_track(&url).await {
                let _ = response_tx
                    .send(EngineResponse::ok(
                        &download_id,
                        ResponseType::DownloadProgress,
                        serde_json::json!({
                            "download_id": &download_id,
                            "percent": 0.0,
                            "speed": "",
                            "eta": "",
                            "total_bytes": 0,
                            "downloaded_bytes": 0,
                            "status": format!("Matching '{}' on YouTube...", track.title),
                        }),
                    ))
                    .await;

                if let Some(match_cand) =
                    SpotifyResolver::find_best_youtube_match(&ytdlp_path, &track).await
                {
                    Logger::info(&format!(
                        "Spotify track '{} - {}' matched to YouTube video: {} ('{}')",
                        track.artist, track.title, match_cand.id, match_cand.title
                    ));
                    target_url = format!("https://www.youtube.com/watch?v={}", match_cand.id);
                    spotify_meta = Some(track);
                } else {
                    Logger::warn(&format!(
                        "Could not match Spotify track '{} - {}' on YouTube",
                        track.artist, track.title
                    ));
                    let _ = response_tx
                        .send(EngineResponse::err(
                            &download_id,
                            "SPOTIFY_NO_MATCH",
                            &format!(
                                "Could not find a high-accuracy match on YouTube for Spotify track: '{} - {}'",
                                track.artist, track.title
                            ),
                        ))
                        .await;
                    return;
                }
            } else {
                Logger::error(&format!("Failed to retrieve Spotify track metadata for {}", url));
                let _ = response_tx
                    .send(EngineResponse::err(
                        &download_id,
                        "SPOTIFY_RESOLVE_FAILED",
                        "Failed to retrieve metadata from Spotify. Please check the track link.",
                    ))
                    .await;
                return;
            }
        }

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
            } else {
                args.push("--ffmpeg-location".to_string());
                args.push(ffmpeg.to_string_lossy().to_string());
            }
        }

        if media_type == MediaType::Video {
            if ffmpeg_path.is_none() {
                Logger::warn(&format!(
                    "FFmpeg is unavailable for download {}. Using pre-merged fallback format selector to ensure complete audio/video stream.",
                    download_id
                ));
                args.push("-f".to_string());
                args.push(video_qual.ytdl_format_selector_fallback().to_string());
            } else {
                args.push("-f".to_string());
                args.push(video_qual.ytdl_format_selector().to_string());
                if video_fmt == VideoFormat::Mp4 {
                    // Prioritize AAC audio in MP4 container so downloaded video always plays audio in Windows Media Player
                    args.push("-S".to_string());
                    args.push("ext:mp4:m4a,res".to_string());
                }
                args.push("--merge-output-format".to_string());
                args.push(video_fmt.extension().to_string());
                args.push("--remux-video".to_string());
                args.push(video_fmt.extension().to_string());
            }
        } else {
            args.push("-x".to_string());
            args.push("--audio-format".to_string());
            args.push(audio_fmt.extension().to_string());
            args.push("--audio-quality".to_string());
            args.push(audio_qual.ytdl_quality().to_string());
        }

        if is_spotify {
            args.push("--embed-metadata".to_string());
            args.push("--embed-thumbnail".to_string());
            if let Some(ref sp) = spotify_meta {
                if ffmpeg_path.is_some() {
                    let clean_title = sp.title.replace('"', "'");
                    let clean_artist = sp.artist.replace('"', "'");
                    args.push("--ppa".to_string());
                    args.push(format!(
                        "ExtractAudio:-metadata title=\"{}\" -metadata artist=\"{}\"",
                        clean_title, clean_artist
                    ));
                }
            }
        }

        // Positional argument delimiter prevents option parsing confusion
        args.push("--".to_string());
        args.push(target_url.clone());

        Logger::info(&format!("Spawning yt-dlp for {}: {:?}", download_id, args));

        let mut cmd = Command::new(&ytdlp_path);
        cmd.args(&args);
        #[cfg(windows)]
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
        cmd.stdin(std::process::Stdio::null());
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());

        if self.is_cancelled(&download_id).await {
            Logger::info(&format!("Download {} was cancelled before starting", download_id));
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
            biased;
            _ = &mut cancel_rx => {
                let _ = child.kill().await;
                stdout_handle.abort();
                stderr_handle.abort();
                let _ = child.wait().await;
                Err("cancelled")
            }
            res = child.wait() => Ok(res),
        };

        if wait_result.is_err() {
            stdout_handle.abort();
            stderr_handle.abort();
            let _ = child.wait().await;
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        } else {
            let _ = stdout_handle.await;
        }
        let stderr_lines = stderr_handle.await.unwrap_or_default();

        // Remove from active downloads
        {
            let mut map = self.active_downloads.lock().await;
            map.remove(&download_id);
        }

        let was_cancelled = wait_result.is_err() || self.is_cancelled(&download_id).await;
        if was_cancelled {
            Logger::info(&format!(
                "Download {} was successfully cancelled",
                download_id
            ));
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
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

        match wait_result {
            Ok(Ok(status)) if status.success() => {
                Logger::info(&format!(
                    "Download {} completed with exit status success",
                    download_id
                ));

                // If yt-dlp left unmerged separate stream fragments (e.g. .f137.mp4 and .f140.m4a),
                // attempt automatic remuxing if FFmpeg is available
                let active_ffmpeg = ffmpeg_path.clone().or_else(|| {
                    DependencyManager::find_ffmpeg(&crate::config::EngineConfig::default())
                });
                if let Some(merged) = Self::try_merge_unmerged_fragments(
                    &output_dir,
                    &unique_prefix,
                    &video_fmt,
                    active_ffmpeg.as_deref(),
                )
                .await
                {
                    Logger::info(&format!(
                        "Successfully merged stream fragments for download {} into {:?}",
                        download_id, merged.path
                    ));
                }

                // Locate the finished file
                if let Some(mut file_info) = Self::find_output_file(&output_dir, &unique_prefix) {
                    let prefix_with_underscore = format!("{}_", unique_prefix);
                    let clean_name = if file_info.name.starts_with(&prefix_with_underscore) {
                        file_info.name[prefix_with_underscore.len()..].to_string()
                    } else if file_info.name.starts_with(&unique_prefix) {
                        file_info.name[unique_prefix.len()..].to_string()
                    } else {
                        file_info.name.clone()
                    };

                    let clean_path = output_dir.join(&clean_name);
                    if clean_path != file_info.path && !clean_name.is_empty() && !clean_path.exists() {
                        if fs::rename(&file_info.path, &clean_path).is_ok() {
                            file_info.path = clean_path;
                            file_info.name = clean_name;
                        }
                    }

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
                    let unmerged = Self::find_unmerged_fragments(&output_dir, &unique_prefix);
                    let (err_code, err_msg) = if !unmerged.is_empty() {
                        (
                            "MERGE_FAILED",
                            "yt-dlp downloaded separate video/audio streams, but FFmpeg is required to merge them into a single playable file. Please ensure FFmpeg is installed.",
                        )
                    } else {
                        (
                            "FILE_NOT_FOUND",
                            "yt-dlp finished but the output file could not be verified on disk.",
                        )
                    };

                    let _ = response_tx
                        .send(EngineResponse::err(&download_id, err_code, err_msg))
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

        if let Some(ref ffmpeg) = ffmpeg_path {
            if let Some(dir) = ffmpeg.parent() {
                args.push("--ffmpeg-location".to_string());
                args.push(dir.to_string_lossy().to_string());
            } else {
                args.push("--ffmpeg-location".to_string());
                args.push(ffmpeg.to_string_lossy().to_string());
            }
        }

        // Positional argument delimiter prevents option parsing confusion
        args.push("--".to_string());
        args.push(url);

        let mut cmd = Command::new(&ytdlp_path);
        cmd.args(&args);
        #[cfg(windows)]
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
        cmd.stdin(std::process::Stdio::null());
        cmd.stdout(std::process::Stdio::null());
        if self.is_cancelled(&download_id).await {
            Logger::info(&format!(
                "Fallback audio download {} was cancelled before starting",
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
            biased;
            _ = &mut cancel_rx => {
                let _ = child.kill().await;
                Err("cancelled")
            }
            res = child.wait() => Ok(res),
        };

        {
            let mut map = self.active_downloads.lock().await;
            map.remove(&download_id);
        }

        let was_cancelled = wait_result.is_err() || self.is_cancelled(&download_id).await;
        if was_cancelled {
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

        match wait_result {
            Ok(Ok(status)) if status.success() => {
                if let Some(mut file_info) = Self::find_output_file(&output_dir, &unique_prefix) {
                    let prefix_with_underscore = format!("{}_", unique_prefix);
                    let clean_name = if file_info.name.starts_with(&prefix_with_underscore) {
                        file_info.name[prefix_with_underscore.len()..].to_string()
                    } else if file_info.name.starts_with(&unique_prefix) {
                        file_info.name[unique_prefix.len()..].to_string()
                    } else {
                        file_info.name.clone()
                    };

                    let clean_path = output_dir.join(&clean_name);
                    if clean_path != file_info.path && !clean_name.is_empty() && !clean_path.exists() {
                        if fs::rename(&file_info.path, &clean_path).is_ok() {
                            file_info.path = clean_path;
                            file_info.name = clean_name;
                        }
                    }

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

    pub fn is_unmerged_fragment(file_name: &str) -> bool {
        if let Some(dot_idx) = file_name.rfind('.') {
            let without_ext = &file_name[..dot_idx];
            if let Some(second_dot_idx) = without_ext.rfind('.') {
                let fmt = &without_ext[second_dot_idx + 1..];
                // Check if fmt matches genuine yt-dlp stream fragment format codes:
                // 1. YouTube numeric itags: 'f' followed strictly by digits (e.g. f137, f140, f251, f399, f22)
                // 2. Storyboards: 'fsb' followed strictly by digits (e.g. fsb0, fsb1)
                // 3. Shorthand stream tags: 'fba' (best audio), 'fbv' (best video)
                // 4. Resolution tags: 'f' followed by digits + 'p' (e.g. f1080p, f720p)
                if fmt.starts_with('f') && fmt.len() >= 2 {
                    let rest = &fmt[1..];
                    if rest.chars().all(|c| c.is_ascii_digit()) {
                        return true;
                    }
                    if rest.starts_with("sb") && rest.len() > 2 && rest[2..].chars().all(|c| c.is_ascii_digit()) {
                        return true;
                    }
                    if rest == "ba" || rest == "bv" {
                        return true;
                    }
                    if rest.ends_with('p') && rest.len() > 1 && rest[..rest.len() - 1].chars().all(|c| c.is_ascii_digit()) {
                        return true;
                    }
                }
            }
        }
        false
    }

    fn get_fragment_format_id(file_name: &str) -> Option<String> {
        if let Some(dot_idx) = file_name.rfind('.') {
            let without_ext = &file_name[..dot_idx];
            if let Some(second_dot_idx) = without_ext.rfind('.') {
                let fmt = &without_ext[second_dot_idx + 1..];
                return Some(fmt.to_lowercase());
            }
        }
        None
    }

    fn is_audio_fragment(frag: &DiscoveredFile) -> bool {
        let ext = frag.extension.to_lowercase();
        if ext == "m4a" || ext == "opus" || ext == "mp3" || ext == "aac" || ext == "flac" || ext == "wav" || ext == "ogg" {
            return true;
        }
        if let Some(fmt) = Self::get_fragment_format_id(&frag.name) {
            // Known YouTube audio itags (139, 140, 249, 250, 251, 599, 600) or shorthand fba
            if fmt == "f139" || fmt == "f140" || fmt == "f249" || fmt == "f250" || fmt == "f251" || fmt == "f599" || fmt == "f600" || fmt == "fba" {
                return true;
            }
        }
        false
    }

    fn is_video_fragment(frag: &DiscoveredFile) -> bool {
        if Self::is_audio_fragment(frag) {
            return false;
        }
        let ext = frag.extension.to_lowercase();
        if ext == "mp4" || ext == "webm" || ext == "mkv" || ext == "avi" || ext == "mov" || ext == "flv" {
            return true;
        }
        if let Some(fmt) = Self::get_fragment_format_id(&frag.name) {
            if fmt == "fbv" || (fmt.starts_with('f') && fmt.ends_with('p')) {
                return true;
            }
        }
        false
    }

    pub fn find_unmerged_fragments(dir: &Path, prefix: &str) -> Vec<DiscoveredFile> {
        let mut fragments = Vec::new();
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    if let Some(file_name) = path.file_name().and_then(|s| s.to_str()) {
                        if file_name.starts_with(prefix) && Self::is_unmerged_fragment(file_name) {
                            let ext = path
                                .extension()
                                .and_then(|s| s.to_str())
                                .unwrap_or("")
                                .to_lowercase();
                            if let Ok(meta) = entry.metadata() {
                                if meta.len() > 0 {
                                    fragments.push(DiscoveredFile {
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
        fragments
    }

    pub async fn try_merge_unmerged_fragments(
        dir: &Path,
        prefix: &str,
        video_fmt: &VideoFormat,
        ffmpeg_exe: Option<&Path>,
    ) -> Option<DiscoveredFile> {
        let ffmpeg = ffmpeg_exe?;
        let fragments = Self::find_unmerged_fragments(dir, prefix);
        if fragments.len() < 2 {
            return None;
        }

        let mut video_frag: Option<&DiscoveredFile> = None;
        let mut audio_frag: Option<&DiscoveredFile> = None;

        for frag in &fragments {
            let is_audio = Self::is_audio_fragment(frag);
            let is_video = Self::is_video_fragment(frag);

            if is_audio && (audio_frag.is_none() || frag.size > audio_frag.unwrap().size) {
                audio_frag = Some(frag);
            } else if is_video && (video_frag.is_none() || frag.size > video_frag.unwrap().size) {
                video_frag = Some(frag);
            }
        }

        if let (Some(video), Some(audio)) = (video_frag, audio_frag) {
            let base_name = if let Some(dot_idx) = video.name.rfind('.') {
                let without_ext = &video.name[..dot_idx];
                if let Some(fmt_dot_idx) = without_ext.rfind('.') {
                    without_ext[..fmt_dot_idx].to_string()
                } else {
                    without_ext.to_string()
                }
            } else {
                format!("{}_merged", prefix)
            };

            let out_file_name = format!("{}.{}", base_name, video_fmt.extension());
            let out_path = dir.join(&out_file_name);

            Logger::info(&format!(
                "Auto-merging unmerged fragments {:?} and {:?} to {:?}",
                video.path, audio.path, out_path
            ));

            // Attempt 1: Fast stream copy
            let mut cmd = Command::new(ffmpeg);
            #[cfg(windows)]
            cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
            cmd.args([
                "-y",
                "-i",
                video.path.to_str()?,
                "-i",
                audio.path.to_str()?,
                "-c",
                "copy",
                out_path.to_str()?,
            ]);

            let mut success = false;
            if let Ok(status) = cmd.status().await {
                if status.success() && out_path.exists() && fs::metadata(&out_path).map(|m| m.len() > 0).unwrap_or(false) {
                    success = true;
                }
            }

            // Attempt 2: If stream copy fails (e.g. incompatible Opus in MP4), transcode audio to AAC
            if !success && *video_fmt == VideoFormat::Mp4 {
                Logger::info(&format!(
                    "Stream copy failed; falling back to AAC audio transcode for {:?}",
                    out_path
                ));
                let mut fallback_cmd = Command::new(ffmpeg);
                #[cfg(windows)]
                fallback_cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
                fallback_cmd.args([
                    "-y",
                    "-i",
                    video.path.to_str()?,
                    "-i",
                    audio.path.to_str()?,
                    "-c:v",
                    "copy",
                    "-c:a",
                    "aac",
                    "-b:a",
                    "192k",
                    out_path.to_str()?,
                ]);

                if let Ok(status) = fallback_cmd.status().await {
                    if status.success() && out_path.exists() && fs::metadata(&out_path).map(|m| m.len() > 0).unwrap_or(false) {
                        success = true;
                    }
                }
            }

            if success {
                let _ = fs::remove_file(&video.path);
                let _ = fs::remove_file(&audio.path);
                if let Ok(meta) = fs::metadata(&out_path) {
                    return Some(DiscoveredFile {
                        path: out_path,
                        name: out_file_name,
                        size: meta.len(),
                        extension: video_fmt.extension().to_string(),
                    });
                }
            }
        }

        None
    }

    pub fn find_output_file(dir: &Path, prefix: &str) -> Option<DiscoveredFile> {
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
                            // Reject unmerged raw format fragments (e.g. .f137.mp4)
                            if !Self::is_unmerged_fragment(file_name) {
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

#[derive(Debug, Clone)]
pub struct DiscoveredFile {
    pub path: PathBuf,
    pub name: String,
    pub size: u64,
    pub extension: String,
}
