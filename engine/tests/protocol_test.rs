use snapload_engine::downloader::DownloadManager;
use snapload_engine::protocol::{
    EngineRequest, EngineResponse, MediaType, NativeMessageIo, RequestAction, ResponseType,
    VideoFormat, VideoQuality, MAX_MESSAGE_SIZE,
};
use std::io::Cursor;

#[tokio::test]
async fn test_native_messaging_roundtrip() {
    let mut buffer = Vec::new();
    let sample_json = r#"{"id":"req-1","version":"1.0","action":"ping"}"#;

    // Write message
    NativeMessageIo::write_message(&mut buffer, sample_json)
        .await
        .expect("write_message failed");

    // Must have 4-byte header
    assert!(buffer.len() > 4);
    let len = u32::from_ne_bytes(buffer[0..4].try_into().unwrap());
    assert_eq!(len as usize, sample_json.len());

    // Read message
    let mut cursor = Cursor::new(buffer);
    let read_result = NativeMessageIo::read_message(&mut cursor)
        .await
        .expect("read_message failed");

    assert_eq!(read_result, Some(sample_json.to_string()));
}

#[tokio::test]
async fn test_oversized_message_rejection() {
    // Construct length prefix of (MAX_MESSAGE_SIZE + 1)
    let bad_len = (MAX_MESSAGE_SIZE + 1).to_ne_bytes();
    let mut cursor = Cursor::new(bad_len);

    let result = NativeMessageIo::read_message(&mut cursor).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
}

#[tokio::test]
async fn test_empty_payload_handling() {
    let zero_len = 0u32.to_ne_bytes();
    let mut cursor = Cursor::new(zero_len);

    let result = NativeMessageIo::read_message(&mut cursor)
        .await
        .expect("read failed");
    assert_eq!(result, Some("".to_string()));
}

#[tokio::test]
async fn test_eof_handling() {
    let empty_bytes: Vec<u8> = vec![];
    let mut cursor = Cursor::new(empty_bytes);

    let result = NativeMessageIo::read_message(&mut cursor)
        .await
        .expect("read failed");
    assert_eq!(result, None);
}

#[tokio::test]
async fn test_request_serialization_deserialization() {
    let json_req = r#"{
        "id": "req-99",
        "version": "1.0",
        "action": "start_download",
        "payload": {
            "url": "https://www.youtube.com/watch?v=dQw4w9WgXcQ",
            "media_type": "video",
            "video_format": "mp4",
            "video_quality": "best"
        }
    }"#;

    let req: EngineRequest = serde_json::from_str(json_req).expect("Failed to deserialize request");
    assert_eq!(req.id, "req-99");
    assert_eq!(req.version, "1.0");

    match req.action {
        RequestAction::StartDownload(payload) => {
            assert_eq!(payload.url, "https://www.youtube.com/watch?v=dQw4w9WgXcQ");
            assert_eq!(payload.media_type, Some(MediaType::Video));
            assert_eq!(payload.video_format, Some(VideoFormat::Mp4));
            assert_eq!(payload.video_quality, Some(VideoQuality::Best));
        }
        _ => panic!("Expected StartDownload action"),
    }
}

#[tokio::test]
async fn test_response_serialization() {
    let resp = EngineResponse::ok(
        "req-42",
        ResponseType::VersionInfo,
        serde_json::json!({ "version": "1.0.0" }),
    );
    let serialized = serde_json::to_string(&resp).expect("serialization failed");
    assert!(serialized.contains("req-42"));
    assert!(serialized.contains("version_info"));
    assert!(serialized.contains("1.0.0"));
    assert!(serialized.contains(r#""success":true"#));
}

#[test]
fn test_url_validation_safe() {
    // Valid HTTPS YouTube URL
    assert!(DownloadManager::validate_url("https://www.youtube.com/watch?v=dQw4w9WgXcQ").is_ok());

    // Valid TikTok URL
    assert!(DownloadManager::validate_url("https://www.tiktok.com/@user/video/1234567890").is_ok());

    // Valid with spaces around it (should be trimmed)
    assert!(DownloadManager::validate_url("   https://youtu.be/dQw4w9WgXcQ  ").is_ok());
}

#[test]
fn test_url_validation_rejections() {
    // Empty
    assert!(DownloadManager::validate_url("").is_err());
    assert!(DownloadManager::validate_url("   ").is_err());

    // Command flag injection attempts
    assert!(DownloadManager::validate_url("--exec rm -rf /").is_err());
    assert!(DownloadManager::validate_url("-o output.mp4").is_err());

    // Non-HTTP/HTTPS schemes
    assert!(DownloadManager::validate_url("file:///C:/Windows/System32/calc.exe").is_err());
    assert!(DownloadManager::validate_url("javascript:alert(1)").is_err());
    assert!(DownloadManager::validate_url("data:text/html,test").is_err());

    // Missing host
    assert!(DownloadManager::validate_url("https://").is_err());
}

#[test]
fn test_error_response_serialization() {
    let err_resp = EngineResponse::err(
        "err-101",
        "INVALID_URL",
        "Only HTTP and HTTPS URLs are allowed",
    );
    let serialized = serde_json::to_string(&err_resp).expect("serialization failed");
    assert!(serialized.contains("err-101"));
    assert!(serialized.contains(r#""type":"error""#));
    assert!(serialized.contains(r#""success":false"#));
    assert!(serialized.contains("INVALID_URL"));
    assert!(serialized.contains("Only HTTP and HTTPS URLs are allowed"));
}

#[test]
fn test_audio_format_and_quality_mappings() {
    use snapload_engine::protocol::{AudioFormat, AudioQuality};
    assert_eq!(AudioFormat::Mp3.extension(), "mp3");
    assert_eq!(AudioFormat::M4a.extension(), "m4a");
    assert_eq!(AudioFormat::Flac.extension(), "flac");
    assert_eq!(AudioQuality::Best.ytdl_quality(), "0");
    assert_eq!(AudioQuality::High.ytdl_quality(), "2");
    assert_eq!(AudioQuality::Low.ytdl_quality(), "6");
}

#[test]
fn test_cancel_request_deserialization() {
    let json = r#"{"id":"req-cancel-1","version":"1.0","action":"cancel_download","payload":{"download_id":"dl_abc123"}}"#;
    let req: EngineRequest = serde_json::from_str(json).expect("deserialize cancel failed");
    match req.action {
        RequestAction::CancelDownload(payload) => {
            assert_eq!(payload.download_id, "dl_abc123");
        }
        _ => panic!("Expected CancelDownload"),
    }
}

#[test]
fn test_timestamp_utc_format() {
    use snapload_engine::logger::Logger;
    let ts = Logger::timestamp_utc();
    assert!(ts.ends_with('Z'));
    assert!(ts.contains('T'));
    assert_eq!(ts.len(), 24); // e.g. "2026-10-31T23:35:18.123Z"
}

#[test]
fn test_download_manager_default() {
    let dm = DownloadManager::default();
    assert!(DownloadManager::validate_url("https://youtube.com/watch?v=123").is_ok());
    drop(dm);
}

#[tokio::test]
async fn test_start_download_input_validation() {
    use snapload_engine::config::EngineConfig;
    use snapload_engine::protocol::StartDownloadPayload;
    use tokio::sync::mpsc;

    let dm = DownloadManager::new();
    let (tx, mut rx) = mpsc::channel(10);
    let cfg = EngineConfig::default();

    // 1. Malicious download_id containing path traversal
    dm.start_download(
        "../../evil_id".to_string(),
        StartDownloadPayload {
            url: "https://www.youtube.com/watch?v=dQw4w9WgXcQ".to_string(),
            media_type: None,
            video_format: None,
            video_quality: None,
            audio_format: None,
            audio_quality: None,
            custom_output_dir: None,
            cookies_file: None,
        },
        cfg.clone(),
        tx.clone(),
    )
    .await;

    let res = rx.recv().await.expect("Expected response");
    assert!(!res.success);
    assert_eq!(
        res.error.as_ref().map(|e| e.code.as_str()),
        Some("INVALID_ID")
    );

    // 2. Malicious custom_output_dir containing path traversal
    dm.start_download(
        "valid_id_123".to_string(),
        StartDownloadPayload {
            url: "https://www.youtube.com/watch?v=dQw4w9WgXcQ".to_string(),
            media_type: None,
            video_format: None,
            video_quality: None,
            audio_format: None,
            audio_quality: None,
            custom_output_dir: Some("../../Windows/System32".to_string()),
            cookies_file: None,
        },
        cfg.clone(),
        tx.clone(),
    )
    .await;

    let res2 = rx.recv().await.expect("Expected response");
    assert!(!res2.success);
    assert_eq!(
        res2.error.as_ref().map(|e| e.code.as_str()),
        Some("INVALID_PATH")
    );

    // 3. UNC network path rejected to prevent Windows SMB NTLM hash leaking
    dm.start_download(
        "valid_id_unc".to_string(),
        StartDownloadPayload {
            url: "https://www.youtube.com/watch?v=dQw4w9WgXcQ".to_string(),
            media_type: None,
            video_format: None,
            video_quality: None,
            audio_format: None,
            audio_quality: None,
            custom_output_dir: Some(r"\\192.168.1.100\share".to_string()),
            cookies_file: None,
        },
        cfg,
        tx,
    )
    .await;

    let res3 = rx.recv().await.expect("Expected response");
    assert!(!res3.success);
    assert_eq!(
        res3.error.as_ref().map(|e| e.code.as_str()),
        Some("INVALID_PATH")
    );
}

#[test]
fn test_output_dir_validation_logic() {
    // UNC paths must be rejected
    assert!(DownloadManager::validate_output_dir(r"\\attacker.com\share").is_err());
    assert!(DownloadManager::validate_output_dir("//attacker.com/share").is_err());
    // Path traversal must be rejected
    assert!(DownloadManager::validate_output_dir(r"C:\Users\..\Windows\System32").is_err());
    // Relative paths must be rejected
    assert!(DownloadManager::validate_output_dir("relative_dir").is_err());
    assert!(DownloadManager::validate_output_dir(r"sub\dir").is_err());
    // Control characters must be rejected
    assert!(DownloadManager::validate_output_dir("C:\\Downloads\n").is_err());
    assert!(DownloadManager::validate_output_dir("C:\\Downloads\0").is_err());
    // Valid Windows absolute drive paths accepted
    assert!(DownloadManager::validate_output_dir(r"C:\Users\Public\Downloads").is_ok());
    assert!(DownloadManager::validate_output_dir("D:/Media/Downloads").is_ok());
}

#[test]
fn test_cookies_file_validation_logic() {
    // Flag injection prefix rejected
    assert!(DownloadManager::validate_cookies_file("--config").is_err());
    assert!(DownloadManager::validate_cookies_file("-c").is_err());
    // UNC network paths rejected
    assert!(DownloadManager::validate_cookies_file(r"\\evil.com\share\cookies.txt").is_err());
    assert!(DownloadManager::validate_cookies_file("//evil.com/share/cookies.txt").is_err());
    // Traversal rejected
    assert!(DownloadManager::validate_cookies_file(r"C:\..\cookies.txt").is_err());
    // Relative path rejected
    assert!(DownloadManager::validate_cookies_file("cookies.txt").is_err());
    // Control characters rejected
    assert!(DownloadManager::validate_cookies_file("C:\\cookies.txt\r\n").is_err());
}

#[test]
fn test_url_validation_control_characters() {
    // Newline or null injection in URL rejected
    assert!(DownloadManager::validate_url("https://youtube.com/watch?v=123\n--exec=calc").is_err());
    assert!(DownloadManager::validate_url("https://youtube.com/watch?v=123\r\n").is_err());
    assert!(DownloadManager::validate_url("https://youtube.com/watch?v=123\0").is_err());
}

#[tokio::test]
async fn test_write_message_oversized_rejection() {
    let mut buffer = Vec::new();
    // Exceed 1 MB
    let huge_message = "x".repeat(MAX_MESSAGE_SIZE as usize + 1);
    let result = NativeMessageIo::write_message(&mut buffer, &huge_message).await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::InvalidData);
}

#[tokio::test]
async fn test_cancel_download_state() {
    let dm = DownloadManager::new();
    assert!(!dm.is_cancelled("dl_test_1").await);
    let res = dm.cancel_download("dl_test_1").await;
    assert!(res);
    assert!(dm.is_cancelled("dl_test_1").await);
    assert!(!dm.is_cancelled("dl_test_2").await);
}

#[test]
fn test_video_format_selector_prefers_m4a_audio() {
    assert!(VideoQuality::Best.ytdl_format_selector().contains("bestaudio[ext=m4a]"));
    assert!(VideoQuality::P1080.ytdl_format_selector().contains("bestaudio[ext=m4a]"));
    assert!(VideoQuality::P720.ytdl_format_selector().contains("bestaudio[ext=m4a]"));
    assert!(VideoQuality::P480.ytdl_format_selector().contains("bestaudio[ext=m4a]"));
}

