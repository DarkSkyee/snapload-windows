use snapload_engine::downloader::DownloadManager;
use snapload_engine::protocol::{
    EngineRequest, EngineResponse, MediaType, NativeMessageIo,
    RequestAction, ResponseType, VideoFormat, VideoQuality,
    MAX_MESSAGE_SIZE,
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
    let err_resp = EngineResponse::err("err-101", "INVALID_URL", "Only HTTP and HTTPS URLs are allowed");
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
