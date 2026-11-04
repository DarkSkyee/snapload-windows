Write-Host "==========================================" -ForegroundColor Cyan
Write-Host "  Building SnapLoad Windows Ecosystem" -ForegroundColor Cyan
Write-Host "==========================================" -ForegroundColor Cyan

$root = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)

if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    $env:PATH = "$HOME\.cargo\bin;$env:PATH"
}

# 1. Generate Icons
Write-Host "`n[1/4] Generating application icons..." -ForegroundColor Yellow
& "$root\scripts\generate_icons.ps1"

# 2. Build Engine
Write-Host "`n[2/4] Building SnapLoad Desktop Engine (Rust)..." -ForegroundColor Yellow
Push-Location "$root\engine"
cargo test --release
if ($LASTEXITCODE -ne 0) {
    Write-Host "Engine tests failed!" -ForegroundColor Red
    Pop-Location
    exit 1
}
cargo build --release
if ($LASTEXITCODE -ne 0) {
    Write-Host "Engine build failed!" -ForegroundColor Red
    Pop-Location
    exit 1
}
Pop-Location

# 3. Build & Package Browser Extensions
Write-Host "`n[3/4] Building and packaging Browser Extensions (Chromium & Firefox)..." -ForegroundColor Yellow
& "$root\scripts\build\build_extension.ps1"
if ($LASTEXITCODE -ne 0) {
    Write-Host "Extension packaging failed!" -ForegroundColor Red
    exit 1
}

# 4. Build Installer & Management App
Write-Host "`n[4/4] Building Installer & Management App..." -ForegroundColor Yellow
Push-Location "$root\installer"
npm run build
if ($LASTEXITCODE -ne 0) {
    Write-Host "Installer frontend build failed!" -ForegroundColor Red
    Pop-Location
    exit 1
}
Push-Location "$root\installer\src-tauri"
cargo build --release
if ($LASTEXITCODE -ne 0) {
    Write-Host "Installer binary build failed!" -ForegroundColor Red
    Pop-Location
    Pop-Location
    exit 1
}
Pop-Location
Pop-Location

# 5. Generate Release Manifest
Write-Host "`n[5/5] Updating Release Manifest with artifact hashes..." -ForegroundColor Yellow
& "$root\scripts\release\generate_manifest.ps1"

Write-Host "`n==========================================" -ForegroundColor Green
Write-Host "  SnapLoad Windows Build Succeeded!" -ForegroundColor Green
Write-Host "==========================================" -ForegroundColor Green
