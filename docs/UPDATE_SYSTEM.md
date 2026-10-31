# SnapLoad Windows — Update System Specification

## 1. Update Philosophy

To ensure maximum download compatibility across shifting video streaming platforms (e.g. YouTube algorithm changes), SnapLoad Windows employs a **modular, multi-tier update model**:

1. **SnapLoad Desktop Engine & Setup**: Managed through version-tagged GitHub Releases with cryptographic SHA-256 validation.
2. **yt-dlp Core Extractor**: Updates independently from nightly and stable upstream releases. Users do not need to wait for a full SnapLoad app release to receive yt-dlp extractor fixes.
3. **FFmpeg Transcoder**: Upgraded independently when newer codec optimizations become available.
4. **Browser Extensions**: Deployed via extension packages or browser web stores.

---

## 2. Release Version Manifest Schema

Remote update checks query the static HTTPS release manifest hosted on GitHub Releases:
`https://raw.githubusercontent.com/DarkSkyee/snapload-windows/main/release_manifest.json`

```json
{
  "version": "1.0.0",
  "release_date": "2026-10-31",
  "author": "DarkSkyee",
  "repository": "https://github.com/DarkSkyee/snapload-windows",
  "artifacts": {
    "engine": {
      "file": "snapload-engine.exe",
      "sha256": "3d5f8e02d6b3c9...",
      "url": "https://github.com/DarkSkyee/snapload-windows/releases/download/v1.0.0/snapload-engine.exe"
    },
    "setup": {
      "file": "snapload-setup.exe",
      "sha256": "a1b2c3d4e5f6...",
      "url": "https://github.com/DarkSkyee/snapload-windows/releases/download/v1.0.0/snapload-setup.exe"
    },
    "extension_chromium": {
      "file": "snapload-extension-chromium.zip",
      "sha256": "e4f5a6b7...",
      "url": "https://github.com/DarkSkyee/snapload-windows/releases/download/v1.0.0/snapload-extension-chromium.zip"
    },
    "extension_firefox": {
      "file": "snapload-extension-firefox.zip",
      "sha256": "f6a7b8c9...",
      "url": "https://github.com/DarkSkyee/snapload-windows/releases/download/v1.0.0/snapload-extension-firefox.zip"
    }
  },
  "dependencies": {
    "ytdlp": {
      "source": "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe"
    },
    "ffmpeg": {
      "source": "https://github.com/GyanD/codexffmpeg/releases/latest"
    }
  }
}
```

---

## 3. Atomic Replacement & Verification Protocol

When an update artifact is downloaded:

1. **Staging Stream**: The file is downloaded to a temporary location (`%LOCALAPPDATA%\SnapLoad\temp\<filename>.tmp`).
2. **Cryptographic Validation**: The SHA-256 hash of the downloaded stream is computed and strictly matched against the manifest's `sha256` value.
   - If the hash mismatches or stream is truncated, the temporary file is deleted immediately and the update fails.
3. **Atomic Replacement**:
   - If the target binary (`snapload-engine.exe`) is running, the updater moves the current binary to `snapload-engine.exe.old` and places the verified new binary into place.
   - The `.old` binary is deleted upon next system restart.
4. **Configuration Safety**:
   - User settings (`config.json`) and downloaded files (`Downloads\SnapLoad`) are strictly preserved and never modified during an update.
