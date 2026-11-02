Write-Host "Building and packaging SnapLoad Browser Extensions..." -ForegroundColor Cyan

$root = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
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

$dist = "$root\extension\dist"
$chromeDir = "$dist\chrome"
$firefoxDir = "$dist\firefox"

$chromeZip = "$dist\snapload-extension-chromium.zip"
$firefoxZip = "$dist\snapload-extension-firefox.zip"

if (Test-Path $chromeZip) { Remove-Item $chromeZip -Force }
if (Test-Path $firefoxZip) { Remove-Item $firefoxZip -Force }

if (Get-Command "tar.exe" -ErrorAction SilentlyContinue) {
    tar.exe -a -cf "$chromeZip" -C "$chromeDir" .
    tar.exe -a -cf "$firefoxZip" -C "$firefoxDir" .
} else {
    Compress-Archive -Path "$chromeDir\*" -DestinationPath $chromeZip -CompressionLevel Optimal -Force
    Compress-Archive -Path "$firefoxDir\*" -DestinationPath $firefoxZip -CompressionLevel Optimal -Force
}

Write-Host "Packaged Chromium extension: $chromeZip" -ForegroundColor Green
Write-Host "Packaged Firefox extension: $firefoxZip" -ForegroundColor Green

Pop-Location
