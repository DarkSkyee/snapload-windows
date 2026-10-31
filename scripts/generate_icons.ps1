Add-Type -AssemblyName System.Drawing

$assetsDir = ".\extension\assets"
if (!(Test-Path $assetsDir)) {
    New-Item -ItemType Directory -Path $assetsDir -Force | Out-Null
}

$sizes = @(16, 32, 48, 128)

foreach ($size in $sizes) {
    $bmp = New-Object System.Drawing.Bitmap $size, $size
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
    $g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic

    # Background squircle
    $rect = New-Object System.Drawing.Rectangle 0, 0, $size, $size
    $bgBrush = New-Object System.Drawing.Drawing2D.LinearGradientBrush(
        $rect,
        [System.Drawing.Color]::FromArgb(255, 7, 9, 19),
        [System.Drawing.Color]::FromArgb(255, 30, 20, 59),
        45.0
    )
    $radius = [int]($size * 0.22)
    $path = New-Object System.Drawing.Drawing2D.GraphicsPath
    $diameter = $radius * 2
    $path.AddArc(0, 0, $diameter, $diameter, 180, 90)
    $path.AddArc($size - $diameter, 0, $diameter, $diameter, 270, 90)
    $path.AddArc($size - $diameter, $size - $diameter, $diameter, $diameter, 0, 90)
    $path.AddArc(0, $size - $diameter, $diameter, $diameter, 90, 90)
    $path.CloseFigure()
    $g.FillPath($bgBrush, $path)

    # Accent Glow Circle
    $penCyan = New-Object System.Drawing.Pen([System.Drawing.Color]::FromArgb(60, 0, 242, 254), [float]([Math]::Max(1.0, $size * 0.04)))
    $margin = [float]($size * 0.12)
    $g.DrawEllipse($penCyan, $margin, $margin, [float]($size - $margin * 2), [float]($size - $margin * 2))

    # Center arrow
    $penWhite = New-Object System.Drawing.Pen([System.Drawing.Color]::FromArgb(255, 255, 255, 255), [float]([Math]::Max(1.5, $size * 0.09)))
    $penWhite.StartCap = [System.Drawing.Drawing2D.LineCap]::Round
    $penWhite.EndCap = [System.Drawing.Drawing2D.LineCap]::Round

    $cx = [float]($size / 2.0)
    $topY = [float]($size * 0.35)
    $botY = [float]($size * 0.65)
    $wingW = [float]($size * 0.16)
    $wingY = [float]($size * 0.52)

    # Arrow stem
    $g.DrawLine($penWhite, $cx, $topY, $cx, $botY)
    # Arrow wings
    $g.DrawLine($penWhite, $cx - $wingW, $wingY, $cx, $botY)
    $g.DrawLine($penWhite, $cx + $wingW, $wingY, $cx, $botY)

    $outPath = Join-Path $assetsDir "icon-$size.png"
    $bmp.Save($outPath, [System.Drawing.Imaging.ImageFormat]::Png)

    $g.Dispose()
    $bmp.Dispose()
    Write-Host "Generated icon: $outPath ($size x $size)"
}
