Add-Type -AssemblyName System.Drawing

$rootDir = Split-Path -Parent $PSScriptRoot
$src = Join-Path $rootDir "installer\src-tauri\icons\128x128.png"
$dst = Join-Path $rootDir "installer\src-tauri\icons\icon.ico"

$bmp = [System.Drawing.Bitmap]::FromFile($src)
$hIcon = $bmp.GetHicon()
$icon = [System.Drawing.Icon]::FromHandle($hIcon)
$fs = New-Object System.IO.FileStream($dst, [System.IO.FileMode]::Create)
$icon.Save($fs)
$fs.Close()
$icon.Dispose()
$bmp.Dispose()

Write-Host "ICO successfully generated at $dst"
