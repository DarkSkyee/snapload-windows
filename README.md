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
  <a href="https://github.com/DarkSkyee/snapload-android">
    <img src="https://img.shields.io/badge/%F0%9F%93%B1%20Also%20Available-SnapLoad%20for%20Android-3DDC84?style=for-the-badge&logo=android&logoColor=white" height="42" alt="SnapLoad for Android" />
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
2. Launch the installer. It will automatically detect your installed browsers (Chrome, Edge, Brave, Opera, Opera GX, Firefox).
3. Select your desired browsers and click **Install & Configure SnapLoad**.
4. The installer automatically deploys the Desktop Engine, registers Windows Native Messaging registry keys, extracts the browser extensions, and guides you through loading them.

### Method 2: Portable / Manual Installation
For full manual headless setup without the graphical wizard, see the [Installation Guide](docs/INSTALLATION.md).

---

## 🧩 Detailed Browser Extension Installation Guide

SnapLoad integrates directly into your browser via a lightweight WebExtension. Because SnapLoad is distributed directly from GitHub (outside the Chrome Web Store / Firefox Add-ons store), it is installed using standard Developer Mode.

### 🌐 1. Chromium Browsers (Google Chrome, Microsoft Edge, Brave, Opera, Opera GX)

All Chromium-based browsers use the same packed or unpacked extension.

#### Step 1: Obtain the Extension Files
- **If you used `snapload-setup.exe`**: The extension is already unpacked and ready at:
  ```text
  %LOCALAPPDATA%\SnapLoad\extensions\chromium
  ```
  *(Press `Win + R`, paste the path above, and press Enter to view it).*
- **If installing manually / standalone**: Download `snapload-extension-chromium.zip` from [Releases](https://github.com/DarkSkyee/snapload-windows/releases) and extract it to a permanent folder (e.g. `C:\Users\<YourUser>\Documents\SnapLoadExtension`).

#### Step 2: Open Extensions Page in Your Browser
Open your browser and navigate to the extensions page by typing the following URL in the address bar:
| Browser | Address URL |
| :--- | :--- |
| **Google Chrome** | `chrome://extensions` |
| **Microsoft Edge** | `edge://extensions` |
| **Brave Browser** | `brave://extensions` |
| **Opera** | `opera://extensions` |
| **Opera GX** | `opera://extensions` |

#### Step 3: Enable Developer Mode
Look for the **Developer mode** toggle (**Tryb programisty**):
- **Chrome / Brave**: Switch the toggle located in the **top-right corner** of the page.
- **Edge**: Turn on the toggle in the **left sidebar** or **top-right**.
- **Opera / Opera GX**: Switch the toggle located in the **top-right corner**.

#### Step 4: Load the Unpacked Extension
1. Click the button labeled **Load unpacked** (**Załaduj rozpakowane**) in the top toolbar.
2. A folder picker dialog will appear. Select the `chromium` folder:
   - For automatic setup: `%LOCALAPPDATA%\SnapLoad\extensions\chromium`
   - For manual setup: The unzipped folder containing `manifest.json`.
3. Click **Select Folder**.

#### Step 5: Pin and Verify
1. Click the puzzle-piece icon (🧩) in your browser toolbar.
2. Find **SnapLoad** and click the **Pin** icon to keep it visible in your toolbar.
3. Open [YouTube](https://www.youtube.com), [Spotify Web Player](https://open.spotify.com), or [SoundCloud](https://soundcloud.com). You will see native **Download** buttons automatically appear inside the players and tracklists!

---

### 🦊 2. Mozilla Firefox

#### Option A: Load via `about:debugging` (Fastest for testing)
1. Open Firefox and enter `about:debugging#/runtime/this-firefox` in the address bar.
2. Click **Load Temporary Add-on...** (**Załaduj dodatek tymczasowy...**).
3. Select `manifest.json` from:
   ```text
   %LOCALAPPDATA%\SnapLoad\extensions\firefox\manifest.json
   ```
   *(or the extracted `manifest.json` from `snapload-extension-firefox.zip`).*
4. SnapLoad is now active in Firefox!

#### Option B: Install Signed XPI Package
1. In Firefox, open `about:addons`.
2. Click the gear icon (⚙️) next to "Manage Your Extensions" in the top-right corner.
3. Select **Install Add-on From File...** (**Zainstaluj dodatek z pliku...**).
4. Choose the `snapload-extension-firefox.xpi` file (located in `%LOCALAPPDATA%\SnapLoad\extensions\firefox\` or downloaded from Releases).
5. Confirm the installation prompts.

---

### 🔍 3. Verifying Native Messaging Connection

1. Click the **SnapLoad** extension icon in your browser toolbar to open the popup.
2. The popup communicates directly with `snapload-engine.exe` via standard Windows stdin/stdout:
   - A **green dot** or **Connected** status confirms the native host is registered and responsive.
   - If it reports *Disconnected*, ensure you have run `snapload-setup.exe` so the Windows Registry keys (`HKCU\Software\Google\Chrome\NativeMessagingHosts\com.darkskyee.snapload`) are initialized.

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

## 🌐 SnapLoad Ecosystem

Looking for SnapLoad on mobile? Check out the Android companion app:

| Platform | Project | Features | Tech Stack |
| :--- | :--- | :--- | :--- |
| 💻 **Windows** | **SnapLoad Windows** *(Current)* | Instant in-browser downloads, YouTube / Spotify / SoundCloud buttons, zero Electron | Rust, Tauri 2, Chromium & Firefox Extension |
| 🤖 **Android** | [**SnapLoad Android**](https://github.com/DarkSkyee/snapload-android) | On-device offline conversion, share sheet integration, Material 3 Expressive UI | Kotlin, Jetpack Compose, Python, yt-dlp, FFmpeg |

---

## 📄 License & Attribution

This project is licensed under the **MIT Non-Commercial License (MIT-NC) with Attribution**:
- ✅ **Personal & Educational**: Completely free to use, inspect, fork, and develop for non-commercial purposes.
- ❌ **Commercial Prohibition**: May NOT be sold, resold, or bundled into monetized distributions.
- ❌ **Attribution Required**: The original author's name (**DarkSkyee**) must be prominently retained in all copies, derivative works, and user-facing dialogs.

See the full [LICENSE](LICENSE) file for complete legal terms.
