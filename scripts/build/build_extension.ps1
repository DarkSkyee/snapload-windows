Write-Host "Building and packaging SnapLoad Browser Extensions..." -ForegroundColor Cyan

$root = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
Push-Location "$root\extension"

npm run typecheck
npm run build

$dist = "$root\extension\dist"
$chromeDir = "$dist\chrome"
$firefoxDir = "$dist\firefox"

$chromeZip = "$dist\snapload-extension-chromium.zip"
$firefoxZip = "$dist\snapload-extension-firefox.zip"

if (Test-Path $chromeZip) { Remove-Item $chromeZip -Force }
if (Test-Path $firefoxZip) { Remove-Item $firefoxZip -Force }

Compress-Archive -Path "$chromeDir\*" -DestinationPath $chromeZip -CompressionLevel Optimal
Compress-Archive -Path "$firefoxDir\*" -DestinationPath $firefoxZip -CompressionLevel Optimal

Write-Host "Packaged Chromium extension: $chromeZip" -ForegroundColor Green
Write-Host "Packaged Firefox extension: $firefoxZip" -ForegroundColor Green

Pop-Location
