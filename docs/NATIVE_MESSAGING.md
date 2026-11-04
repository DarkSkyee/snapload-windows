# SnapLoad Windows — Native Messaging Protocol Specification

## 1. Protocol Wire Format

Communication between the browser extension and the SnapLoad Desktop Engine follows the standard **Native Messaging specification**:

1. Each message is serialized as a UTF-8 JSON string.
2. The message is preceded by a **32-bit unsigned integer (4 bytes) in native/little-endian byte order** specifying the length of the JSON string in bytes.
3. Total message size is strictly capped at **1,048,576 bytes (1 MB)** to protect against memory exhaustion.

```
+------------------------------+---------------------------------------+
|  Length Prefix (4 bytes)     |  JSON Payload (UTF-8 encoded string)  |
|  e.g. 0x2A 0x00 0x00 0x00    |  {"id":"req_1","action":"ping"}       |
+------------------------------+---------------------------------------+
```

---

## 2. Host Manifests & Registration

### Host Identifier
`com.darkskyee.snapload`

### Chromium Manifest (`com.darkskyee.snapload.chromium.json`)
Placed at `%LOCALAPPDATA%\SnapLoad\manifests\com.darkskyee.snapload.chromium.json`:
```json
{
  "name": "com.darkskyee.snapload",
  "description": "SnapLoad Native Messaging Downloader Host",
  "path": "C:\\Users\\<Username>\\AppData\\Local\\SnapLoad\\bin\\snapload-engine.exe",
  "type": "stdio",
  "allowed_origins": [
    "chrome-extension://ikkidkbojpedpeaomipgioiigmjlpflg/",
    "extension://ikkidkbojpedpeaomipgioiigmjlpflg/"
  ]
}
```

### Firefox Manifest (`com.darkskyee.snapload.firefox.json`)
Placed at `%LOCALAPPDATA%\SnapLoad\manifests\com.darkskyee.snapload.firefox.json`:
```json
{
  "name": "com.darkskyee.snapload",
  "description": "SnapLoad Native Messaging Downloader Host",
  "path": "C:\\Users\\<Username>\\AppData\\Local\\SnapLoad\\bin\\snapload-engine.exe",
  "type": "stdio",
  "allowed_extensions": [
    "snapload@darkskyee.github.io"
  ]
}
```

### Windows Registry Keys (`HKEY_CURRENT_USER`)
Registration does not require administrative elevation:
- **Chrome**: `HKCU\Software\Google\Chrome\NativeMessagingHosts\com.darkskyee.snapload`
- **Edge**: `HKCU\Software\Microsoft\Edge\NativeMessagingHosts\com.darkskyee.snapload`
- **Brave**: `HKCU\Software\BraveSoftware\Brave-Browser\NativeMessagingHosts\com.darkskyee.snapload`
- **Opera**: `HKCU\Software\Opera Software\Opera Stable\NativeMessagingHosts\com.darkskyee.snapload`
- **Firefox**: `HKCU\Software\Mozilla\NativeMessagingHosts\com.darkskyee.snapload`

Default value: absolute file path to the respective manifest JSON file.

---

## 3. Request Actions & Schemas

### Common Request Structure
```json
{
  "id": "req_uuid_or_random",
  "version": "1.0",
  "action": "<action_name>",
  "payload": { ... }
}
```

### 1. `ping`
Tests engine connectivity and returns server timestamp.
```json
{ "id": "req-1", "action": "ping" }
```

### 2. `get_version`
Returns engine build version and metadata.
```json
{ "id": "req-2", "action": "get_version" }
```

### 3. `check_dependencies`
Inspects installed status and versions for `yt-dlp` and `FFmpeg`.
```json
{ "id": "req-3", "action": "check_dependencies" }
```

### 4. `start_download`
Initiates a media download with custom format options.
```json
{
  "id": "req-4",
  "action": "start_download",
  "payload": {
    "url": "https://www.youtube.com/watch?v=dQw4w9WgXcQ",
    "media_type": "video",
    "video_format": "mp4",
    "video_quality": "best",
    "audio_format": "mp3",
    "audio_quality": "best"
  }
}
```

### 5. `cancel_download`
Cancels an active download and kills child process tree.
```json
{
  "id": "req-5",
  "action": "cancel_download",
  "payload": {
    "download_id": "req-4"
  }
}
```

---

## 4. Response Messages & Streaming Events

### Common Response Structure
```json
{
  "id": "req-4",
  "version": "1.0",
  "type": "<response_type>",
  "success": true,
  "data": { ... },
  "error": null
}
```

### `download_progress` (Streamed periodically)
```json
{
  "id": "req-4",
  "version": "1.0",
  "type": "download_progress",
  "success": true,
  "data": {
    "download_id": "req-4",
    "percent": 45.2,
    "speed": "6.8MiB/s",
    "eta": "00:14",
    "total_bytes": 104857600,
    "downloaded_bytes": 47395635,
    "status": "Downloading..."
  }
}
```

### `download_completed`
```json
{
  "id": "req-4",
  "version": "1.0",
  "type": "download_completed",
  "success": true,
  "data": {
    "download_id": "req-4",
    "file_path": "C:\\Users\\Name\\Downloads\\snapload_req-4.mp4",
    "file_name": "snapload_req-4.mp4",
    "file_size_bytes": 104857600,
    "extension": "mp4",
    "fallback_to_audio": false
  }
}
```

### Error Responses
```json
{
  "id": "req-4",
  "version": "1.0",
  "type": "error",
  "success": false,
  "error": {
    "code": "INVALID_URL",
    "message": "Only HTTP and HTTPS URLs are allowed"
  }
}
```

---

## 5. Standard Error Codes

| Error Code | Description |
|---|---|
| `MALFORMED_JSON` | Incoming packet failed JSON deserialization. |
| `INVALID_URL` | URL is empty, starts with command flags, or uses an unsupported scheme. |
| `DEPENDENCY_MISSING` | `yt-dlp` executable not found. |
| `SPAWN_FAILED` | Failed to spawn `yt-dlp` child process. |
| `PROCESS_ERROR` | An error occurred while monitoring the child process. |
| `DOWNLOAD_FAILED` | `yt-dlp` returned non-zero exit code. |
| `FILE_NOT_FOUND` | Download completed but output file verification failed. |
| `IO_ERROR` | Unable to create or write to the target directory. |
