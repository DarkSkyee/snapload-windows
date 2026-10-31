# SnapLoad Windows — System Architecture

## 1. Overview

**SnapLoad Windows** is a lightweight, privacy-focused media downloading ecosystem engineered specifically for Windows desktop environments. It is designed as an independent companion to the SnapLoad Android application, sharing identical multimedia philosophy, format configurations, and core dependencies (`yt-dlp` and `FFmpeg`), but architected natively for Windows desktop integration.

```
┌──────────────────────────────────────────────────────────────────┐
│                 Web Browsers (Chrome, Firefox, etc.)             │
│                                                                  │
│  ┌───────────────────────┐            ┌───────────────────────┐  │
│  │  Browser Popup UI     │            │ YouTube Content Script│  │
│  │ (Format/Quality/Prog) │            │ (SPA Action Button)   │  │
│  └───────────┬───────────┘            └───────────┬───────────┘  │
│              └───────────────┬────────────────────┘              │
│                              │ Internal Messaging                │
│                              ▼                                   │
│             ┌──────────────────────────────────┐                 │
│             │   Extension Background Worker    │                 │
│             └────────────────┬─────────────────┘                 │
└──────────────────────────────┼───────────────────────────────────┘
                               │ Native Messaging Protocol (stdin/stdout)
                               ▼
┌──────────────────────────────────────────────────────────────────┐
│              SnapLoad Desktop Engine (Rust CLI)                  │
│                                                                  │
│  • JSON Protocol Router      • Security & URL Validator          │
│  • Process Manager           • File-based Diagnostics Logger     │
│  • Concurrency Controller    • Process Tree Cancellation         │
│  • Progress Parser           • Output File Verifier              │
└───────────────┬───────────────────────────────┬──────────────────┘
                │ Safe Args                     │ Location Flag
                ▼                               ▼
       ┌─────────────────┐             ┌─────────────────┐
       │     yt-dlp      │◄────────────┤     FFmpeg      │
       │ (Media Engine)  │             │  (Transcoder)   │
       └────────┬────────┘             └─────────────────┘
                │
                ▼
       Downloads/SnapLoad/ (Local User Storage)
```

---

## 2. Architectural Principles

1. **Zero Permanent Background Services**:
   - There are no permanent daemon processes, HTTP servers, or tray services idling in the background.
   - The Desktop Engine is invoked on-demand by the browser via standard **Native Messaging**.
   - When the browser closes or the download concludes, the engine exits cleanly.

2. **No Electron**:
   - Electron introduces 200MB+ memory footprints and bloated installer packages.
   - SnapLoad Engine is implemented in native **Rust** (~1.0 MB binary), and the setup companion is built with **Tauri 2**, leveraging native Windows WebView2.

3. **100% On-Device & Privacy-First**:
   - No user accounts, telemetries, or remote APIs.
   - All link resolving, metadata extraction, and media transcoding happen locally on your PC.

4. **Process Isolation & Strict Security**:
   - The engine never invokes system shells (`cmd.exe /c` or PowerShell).
   - Process arguments are passed directly via `std::process::Command` argument arrays, eliminating shell injection vulnerabilities.
   - URLs are strictly validated to require HTTP or HTTPS schemes and reject command flag injections.

---

## 3. Subsystem Breakdown

### A. Desktop Engine (`engine/`)
- **Technology**: Rust 2021 + Tokio async runtime.
- **Role**: Native Messaging host listening on `stdin`, executing `yt-dlp` child processes, capturing line-delimited progress, and responding on `stdout`.
- **Wire Protocol**: 32-bit unsigned little-endian length prefix followed by UTF-8 encoded JSON.
- **Safety Limits**: Max packet size capped at 1 MB to prevent memory exhaustion attacks.
- **Logging**: Isolated file logging to `%LOCALAPPDATA%\SnapLoad\logs\engine.log` to preserve `stdout` purely for native messaging wire packets.

### B. Browser Extensions (`extension/`)
- **Technology**: TypeScript + ESBuild multi-target compilation.
- **Targets**:
  - **Chromium**: Google Chrome, Microsoft Edge, Brave, Opera (Manifest V3 service worker).
  - **Firefox**: Mozilla Firefox (WebExtension background script).
- **Features**:
  - Context menu item: *"Download with SnapLoad"*.
  - YouTube player integration: Injecting a reactive download button with SPA `yt-navigate-finish` event listeners and duplicate prevention.
  - Extension popup: Format selector (Video: MP4, MKV, WebM; Audio: MP3, M4A, FLAC, WAV, OPUS), quality selector, live progress bar, speed, and ETA.

### C. Setup & Management GUI (`installer/`)
- **Technology**: Tauri 2 + Rust backend + Vite/TypeScript frontend.
- **Visual Design**: Dark slate UI (`#070913`), cards (`#0F172A`), neon cyan (`#00F2FE`), and indigo/violet gradients consistent with SnapLoad Android.
- **Functions**:
  - Browser detection in Windows Registry (`HKCU` / `HKLM`) and standard paths.
  - Deployment of `snapload-engine.exe` to `%LOCALAPPDATA%\SnapLoad\bin`.
  - Registration of Native Messaging host keys in `HKCU\Software\[Browser]\NativeMessagingHosts`.
  - Dependency verification for `yt-dlp` and `FFmpeg`.
  - Clean uninstallation without touching user media downloads.

---

## 4. Fallback Architecture

Matching SnapLoad Android's proven fallback mechanism:
- When a user requests **Video (MP4)** from an audio-only stream (e.g. YouTube Music track or Spotify track), or when `yt-dlp` reports that video streams are unavailable:
  1. The engine automatically catches the condition.
  2. The download seamlessly retries using high-quality **MP3 (320 kbps)** audio extraction.
  3. The completion response flags `fallback_to_audio: true` so the extension popup notifies the user of the audio conversion.
