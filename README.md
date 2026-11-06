# SnapLoad (Windows)

<p align="center">
  <a href="https://github.com/DarkSkyee/snapload-windows/releases">
    <img src="https://img.shields.io/github/downloads/DarkSkyee/snapload-windows/total?style=for-the-badge&logo=github&color=10B981" alt="Total Downloads" />
  </a>
  <a href="https://github.com/DarkSkyee/snapload-windows/stargazers">
    <img src="https://img.shields.io/github/stars/DarkSkyee/snapload-windows?style=for-the-badge&logo=github&color=F59E0B" alt="GitHub Stars" />
  </a>
  <a href="https://github.com/DarkSkyee/snapload-windows/releases/latest">
    <img src="https://img.shields.io/github/v/release/DarkSkyee/snapload-windows?style=for-the-badge&logo=github&color=6366F1" alt="Latest Release" />
  </a>
  <a href="LICENSE">
    <img src="https://img.shields.io/badge/License-Non--Commercial%20(Attribution)-orange.svg?style=for-the-badge" alt="License" />
  </a>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/Platform-Windows%2010%20%2F%2011%20(64--bit)-0078D6?style=for-the-badge&logo=windows&logoColor=white" alt="Windows 64-bit" />
  <img src="https://img.shields.io/badge/Engine-Rust%202021-DEA584?style=for-the-badge&logo=rust&logoColor=black" alt="Rust" />
  <img src="https://img.shields.io/badge/GUI-Tauri%202-FFC131?style=for-the-badge&logo=tauri&logoColor=black" alt="Tauri 2" />
  <img src="https://img.shields.io/badge/Extension-TypeScript%20MV3-3178C6?style=for-the-badge&logo=typescript&logoColor=white" alt="TypeScript" />
  <a href="https://github.com/yt-dlp/yt-dlp">
    <img src="https://img.shields.io/badge/Core-yt--dlp%20Engine-FF0000?style=for-the-badge&logo=youtube&logoColor=white" alt="yt-dlp" />
  </a>
</p>

<br/>

<p align="center">
  <img src="docs/assets/logo.svg" width="160" height="160" alt="SnapLoad Windows Logo" />
</p>

<p align="center">
  <a href="https://github.com/DarkSkyee/snapload-windows/releases/latest">
    <img src="https://img.shields.io/badge/%E2%AC%87%EF%B8%8F%20Download-Latest%20Release-6366F1?style=for-the-badge&logo=windows&logoColor=white" height="42" alt="Download Latest Release" />
  </a>
</p>

<br/>

**SnapLoad Windows** is a lightweight, privacy-focused Windows desktop companion for media downloading. Engineered from the ground up to complement the SnapLoad Android ecosystem, it brings instant, high-speed media downloading directly to your desktop web browser.

Unlike web converter portals with intrusive ads or heavy Electron background daemons, SnapLoad Windows executes **100% on-device** using a tiny **Rust Native Messaging Host (~1.0 MB)** and local `yt-dlp` / `FFmpeg` runtimes.

---

## 🌟 Highlights

- ⚡ **No Permanent Background Daemons**: The Desktop Engine starts on-demand when requested by the browser via Native Messaging and terminates cleanly upon task completion. Zero idle memory consumption.
- 🚀 **Zero Electron**: Ultra-lightweight native Rust core paired with modern WebExtensions and an optional Tauri 2 setup wizard.
- 🌐 **Multi-Browser Compatibility**: Dedicated Manifest V3 builds for **Chromium** (Google Chrome, Microsoft Edge, Brave, Opera) and WebExtensions for **Mozilla Firefox**.
- 🎬 **YouTube SPA Integration**: Injects a clean, native download button into YouTube's player control bar with seamless support for YouTube SPA navigation (`yt-navigate-finish`) and duplicate protection.
- 🎵 **Spotify & SoundCloud Native Integration**: Direct in-browser downloading from `open.spotify.com` and `soundcloud.com` with native player bar buttons, tracklist row actions, automated metadata embedding, and high-accuracy `TrackMatcher` audio matching.
- 🚦 **Smart Batch Queue Manager**: Built-in concurrency control (limit of 2 concurrent downloads) to prevent rate limits (HTTP 429) when downloading albums and playlists.
- 🖱️ **Context Menu**: Right-click any video, audio, or webpage link to trigger "Download with SnapLoad".
- 🎧 **Multi-Format & Quality Selection**:
  - **Video**: MP4 (H.264), MKV, WebM in Best Available (4K / Original), 1080p FHD, 720p HD, 480p SD.
  - **Audio**: MP3 (320kbps CBR / VBR Best), M4A, FLAC (Lossless), WAV, OPUS.
  - **Intelligent Audio Fallback**: Automatically falls back to high-fidelity MP3 extraction if a requested video stream is unavailable.
- 🛡️ **Safe & Secure**: Process arguments are passed via strict operating system arrays without shell string evaluation, preventing command injection.

---

## 🏗️ Architecture & Ecosystem

```
┌────────────────────────────────────────────────────────┐
│         Browser Extension (Chromium / Firefox)         │
│  • Context Menu Handler      • YouTube SPA Injection   │
│  • Extension Popup UI        • Real-Time Progress Bar  │
└──────────────────────────┬─────────────────────────────┘
                           │ Native Messaging Protocol (stdin/stdout)
                           ▼
┌────────────────────────────────────────────────────────┐
│            SnapLoad Desktop Engine (Rust)              │
│  • URL Validation            • Process Tree Manager    │
│  • Streamed Progress Parser  • Cancellation Trigger    │
│  • File-Based Diagnostics    • Audio Fallback Logic    │
└──────────────┬─────────────────────────┬───────────────┘
               │                         │
               ▼                         ▼
      yt-dlp Core Extractor       FFmpeg Media Converter
               │
               ▼
      Downloads\\ (User Scoped Storage)
```

---

## 📲 How to Install

### Method 1: Graphical Setup Wizard (Recommended)
1. Download `snapload-setup.exe` from the latest **[Releases](https://github.com/DarkSkyee/snapload-windows/releases)**.
2. Launch the installer. It will automatically detect your installed browsers (Chrome, Edge, Brave, Opera, Firefox).
3. Select your desired browsers and click **Install & Configure SnapLoad**.
4. Activate your browser extension using the on-screen guide.

### Method 2: Portable / Manual Installation
For manual setup, see the complete [Installation Guide](docs/INSTALLATION.md).

---

## 🛠️ Building From Source

```powershell
# 1. Clone repository
git clone https://github.com/DarkSkyee/snapload-windows.git
cd snapload-windows

# 2. Build the Rust Desktop Engine
cd engine
cargo test
cargo build --release
cd ..

# 3. Build Browser Extensions (Chromium & Firefox)
cd extension
npm install
npm run typecheck
npm run build
cd ..

# 4. Build Installer & Management Desktop Application
cd installer
npm install
npm run build
cd ..
```

Or execute the automated ecosystem build script:
```powershell
powershell -ExecutionPolicy Bypass -File scripts\\build\\build_all.ps1
```

---

## 📚 Technical Documentation

- **[Architecture Deep-Dive](docs/ARCHITECTURE.md)**: Process model, cancellation flow, and design principles.
- **[Native Messaging Protocol](docs/NATIVE_MESSAGING.md)**: Binary length-prefixed JSON protocol specification and registry schema.
- **[Browser Compatibility](docs/BROWSER_COMPATIBILITY.md)**: Chromium MV3 vs Firefox WebExtensions integration details.
- **[Update System Specification](docs/UPDATE_SYSTEM.md)**: Cryptographic SHA-256 validation and atomic replacement workflow.
- **[Development Guide](docs/DEVELOPMENT.md)**: Testing guidelines and project workflows.

---

## 📄 License & Attribution

This project is licensed under the **MIT Non-Commercial License (MIT-NC) with Attribution**:
- ✅ **Personal & Educational**: Completely free to use, inspect, fork, and develop for non-commercial purposes.
- ❌ **Commercial Prohibition**: May NOT be sold, resold, or bundled into monetized distributions.
- ❌ **Attribution Required**: The original author's name (**DarkSkyee**) must be prominently retained in all copies, derivative works, and user-facing dialogs.

See the full [LICENSE](LICENSE) file for complete legal terms.
