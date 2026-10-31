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

# 3. Build Extension
Write-Host "`n[3/4] Building Browser Extensions (Chromium & Firefox)..." -ForegroundColor Yellow
Push-Location "$root\extension"
npm run typecheck
if ($LASTEXITCODE -ne 0) {
    Write-Host "Extension typecheck failed!" -ForegroundColor Red
    Pop-Location
    exit 1
}
npm run build
if ($LASTEXITCODE -ne 0) {
    Write-Host "Extension build failed!" -ForegroundColor Red
    Pop-Location
    exit 1
}
Pop-Location

# 4. Build Installer Frontend
Write-Host "`n[4/4] Building Installer & Management App..." -ForegroundColor Yellow
Push-Location "$root\installer"
npm run build
if ($LASTEXITCODE -ne 0) {
    Write-Host "Installer frontend build failed!" -ForegroundColor Red
    Pop-Location
    exit 1
}
Pop-Location

Write-Host "`n==========================================" -ForegroundColor Green
Write-Host "  SnapLoad Windows Build Succeeded!" -ForegroundColor Green
Write-Host "==========================================" -ForegroundColor Green
