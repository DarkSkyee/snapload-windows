# SnapLoad Windows — Installation Guide

## 1. System Requirements

- **Operating System**: Windows 10 (64-bit) or Windows 11 (64-bit).
- **Architecture**: x86_64.
- **Supported Browsers**:
  - Google Chrome (v100+)
  - Microsoft Edge (v100+)
  - Brave Browser
  - Opera / Opera GX
  - Mozilla Firefox (v109+)
- **Storage**: ~80 MB for binaries and dependencies (`snapload-engine`, `yt-dlp`, `FFmpeg`).
- **Permissions**: Standard per-user permissions. **No administrative privileges required**.

---

## 2. Graphical Installation (Recommended)

1. Download the latest `snapload-setup.exe` package from the **[Releases](https://github.com/DarkSkyee/snapload-windows/releases)** page.
2. Launch `snapload-setup.exe`.
3. The setup wizard automatically scans your registry and filesystem for installed browsers.
4. Verify or toggle your target browsers (e.g. Chrome, Edge, Firefox).
5. Ensure `yt-dlp` and `FFmpeg` are checked.
6. Click **Install & Configure SnapLoad**.
7. The installer will:
   - Deploy `snapload-engine.exe` into `%LOCALAPPDATA%\SnapLoad\bin`.
   - Create Native Messaging host manifests.
   - Register registry keys under `HKEY_CURRENT_USER\Software\[Browser]\NativeMessagingHosts`.
   - Initialize your default download destination at `Downloads\SnapLoad`.
8. Once installation finishes, click the provided button to load the browser extension.

---

## 3. Browser Extension Setup

### A. Chromium Browsers (Chrome, Edge, Brave, Opera)
1. In your browser, open the extensions management page:
   - Chrome / Brave: `chrome://extensions`
   - Edge: `edge://extensions`
   - Opera: `opera://extensions`
2. Enable **Developer mode** (toggle in top-right corner).
3. Click **Load unpacked** and select the folder:
   `SnapLoad-Windows\extension\dist\chrome`
4. The SnapLoad extension icon will appear in your browser toolbar!

### B. Mozilla Firefox
1. Open Firefox and navigate to `about:debugging#/runtime/this-firefox`.
2. Click **Load Temporary Add-on...**
3. Select `manifest.json` inside:
   `SnapLoad-Windows\extension\dist\firefox\manifest.json`
4. Or drag and drop the packaged `snapload-extension-firefox.zip` into `about:addons`.

---

## 4. Manual / Headless Setup (For Power Users)

If you prefer installing without the GUI installer:

```powershell
# 1. Create target directories
$binDir = "$env:LOCALAPPDATA\SnapLoad\bin"
$manifestDir = "$env:LOCALAPPDATA\SnapLoad\manifests"
New-Item -ItemType Directory -Path $binDir, $manifestDir -Force

# 2. Copy the compiled engine binary
Copy-Item "engine\target\release\snapload-engine.exe" "$binDir\snapload-engine.exe"

# 3. Register Native Messaging Host for Chrome
$chromeReg = "HKCU:\Software\Google\Chrome\NativeMessagingHosts\com.darkskyee.snapload"
New-Item -Path $chromeReg -Force | Out-Null
Set-ItemProperty -Path $chromeReg -Name "(default)" -Value "$manifestDir\com.darkskyee.snapload.chromium.json"

# 4. Register Native Messaging Host for Firefox
$firefoxReg = "HKCU:\Software\Mozilla\NativeMessagingHosts\com.darkskyee.snapload"
New-Item -Path $firefoxReg -Force | Out-Null
Set-ItemProperty -Path $firefoxReg -Name "(default)" -Value "$manifestDir\com.darkskyee.snapload.firefox.json"
```

---

## 5. Clean Uninstallation

SnapLoad guarantees that user data is never destroyed:
1. Open `snapload-setup.exe` and select the **Maintenance** tab.
2. Click **Clean Uninstall SnapLoad**.
3. All registered registry keys and engine binaries in `%LOCALAPPDATA%\SnapLoad` are purged.
4. **All your downloaded videos and songs in `Downloads\SnapLoad` remain completely untouched.**
