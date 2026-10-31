Write-Host "Generating SnapLoad Windows Release Manifest..." -ForegroundColor Cyan

$root = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)

function Get-Sha256($path) {
    if (Test-Path $path) {
        return (Get-FileHash -Path $path -Algorithm SHA256).Hash.ToLower()
    }
    return ""
}

$engineExe = "$root\engine\target\release\snapload-engine.exe"
$setupExe = "$root\installer\src-tauri\target\release\snapload-setup.exe"
$chromeZip = "$root\extension\dist\snapload-extension-chromium.zip"
$firefoxZip = "$root\extension\dist\snapload-extension-firefox.zip"

$manifest = @{
    version = "1.0.0"
    release_date = (Get-Date).ToString("yyyy-MM-dd")
    author = "DarkSkyee"
    repository = "https://github.com/DarkSkyee/snapload-windows"
    artifacts = @{
        engine = @{
            file = "snapload-engine.exe"
            sha256 = (Get-Sha256 $engineExe)
            url = "https://github.com/DarkSkyee/snapload-windows/releases/download/v1.0.0/snapload-engine.exe"
        }
        setup = @{
            file = "snapload-setup.exe"
            sha256 = (Get-Sha256 $setupExe)
            url = "https://github.com/DarkSkyee/snapload-windows/releases/download/v1.0.0/snapload-setup.exe"
        }
        extension_chromium = @{
            file = "snapload-extension-chromium.zip"
            sha256 = (Get-Sha256 $chromeZip)
            url = "https://github.com/DarkSkyee/snapload-windows/releases/download/v1.0.0/snapload-extension-chromium.zip"
        }
        extension_firefox = @{
            file = "snapload-extension-firefox.zip"
            sha256 = (Get-Sha256 $firefoxZip)
            url = "https://github.com/DarkSkyee/snapload-windows/releases/download/v1.0.0/snapload-extension-firefox.zip"
        }
    }
    dependencies = @{
        ytdlp = @{
            source = "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe"
        }
        ffmpeg = @{
            source = "https://github.com/GyanD/codexffmpeg/releases/latest"
        }
    }
}

$json = $manifest | ConvertTo-Json -Depth 5
$outPath = "$root\release_manifest.json"
$json | Set-Content -Path $outPath -Encoding UTF8

Write-Host "Generated release manifest: $outPath" -ForegroundColor Green
