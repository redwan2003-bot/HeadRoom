# Generate Microsoft Store / MSIX Visual Assets for Headroom
param(
    [string]$OutputDir = "$PSScriptRoot\Assets"
)

Add-Type -AssemblyName System.Drawing

if (-not (Test-Path $OutputDir)) {
    New-Item -ItemType Directory -Path $OutputDir -Force | Out-Null
}

function Create-HeadroomLogo {
    param(
        [int]$Width,
        [int]$Height,
        [string]$Path,
        [bool]$IsWide = $false
    )

    $bmp = New-Object System.Drawing.Bitmap($Width, $Height)
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
    $g.TextRenderingHint = [System.Drawing.Text.TextRenderingHint]::AntiAliasGridFit

    # Background
    $bgColor = [System.Drawing.ColorTranslator]::FromHtml("#161A22")
    $bgBrush = New-Object System.Drawing.SolidBrush($bgColor)
    $g.FillRectangle($bgBrush, 0, 0, $Width, $Height)

    # Accent colors
    $cyanColor = [System.Drawing.ColorTranslator]::FromHtml("#00D2FF")
    $cyanBrush = New-Object System.Drawing.SolidBrush($cyanColor)
    $penWidth = [Math]::Max([float]2.0, [float]($Width / 25))
    $cyanPen = New-Object System.Drawing.Pen($cyanColor, $penWidth)

    if ($IsWide) {
        # Wide Tile: Icon on left, text on right
        $boxSize = [int]($Height * 0.6)
        $boxX = [int]($Height * 0.2)
        $boxY = [int]($Height * 0.2)

        # Draw chip outline
        $g.DrawRectangle($cyanPen, $boxX, $boxY, $boxSize, $boxSize)

        # Draw "H"
        $fSize = [float]($boxSize * 0.5)
        $font = New-Object System.Drawing.Font("Segoe UI", $fSize, [System.Drawing.FontStyle]::Bold)
        $sf = New-Object System.Drawing.StringFormat
        $sf.Alignment = [System.Drawing.StringAlignment]::Center
        $sf.LineAlignment = [System.Drawing.StringAlignment]::Center
        $rect = New-Object System.Drawing.RectangleF([float]$boxX, [float]$boxY, [float]$boxSize, [float]$boxSize)
        $g.DrawString("H", $font, $cyanBrush, $rect, $sf)

        # Draw Text "HEADROOM"
        $tSize = [float]($Height * 0.20)
        $titleFont = New-Object System.Drawing.Font("Segoe UI", $tSize, [System.Drawing.FontStyle]::Bold)
        $titleBrush = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::White)
        $g.DrawString("HEADROOM", $titleFont, $titleBrush, [float]($boxX + $boxSize + 15), [float]($boxY + 5))

        # Subtitle
        $sSize = [float]($Height * 0.12)
        $subFont = New-Object System.Drawing.Font("Segoe UI", $sSize, [System.Drawing.FontStyle]::Regular)
        $subBrush = New-Object System.Drawing.SolidBrush([System.Drawing.ColorTranslator]::FromHtml("#8B949E"))
        $g.DrawString("Memory Diagnostic", $subFont, $subBrush, [float]($boxX + $boxSize + 16), [float]($boxY + ($boxSize * 0.6)))
    } else {
        # Square Icon
        $padding = [int]($Width * 0.18)
        $size = $Width - ($padding * 2)

        # Draw chip boundary
        $g.DrawRectangle($cyanPen, $padding, $padding, $size, $size)

        # Letter "H"
        $fontSize = [Math]::Max([float]8.0, [float]($size * 0.52))
        $font = New-Object System.Drawing.Font("Segoe UI", $fontSize, [System.Drawing.FontStyle]::Bold)
        $sf = New-Object System.Drawing.StringFormat
        $sf.Alignment = [System.Drawing.StringAlignment]::Center
        $sf.LineAlignment = [System.Drawing.StringAlignment]::Center
        $rect = New-Object System.Drawing.RectangleF([float]$padding, [float]$padding, [float]$size, [float]$size)
        $g.DrawString("H", $font, $cyanBrush, $rect, $sf)
    }

    $bmp.Save($Path, [System.Drawing.Imaging.ImageFormat]::Png)
    $g.Dispose()
    $bmp.Dispose()
    Write-Output "Created: $Path ($Width x $Height)"
}

Create-HeadroomLogo 44 44 "$OutputDir\Square44x44Logo.png" $false
Create-HeadroomLogo 150 150 "$OutputDir\Square150x150Logo.png" $false
Create-HeadroomLogo 310 150 "$OutputDir\Wide310x150Logo.png" $true
Create-HeadroomLogo 50 50 "$OutputDir\StoreLogo.png" $false
Create-HeadroomLogo 620 300 "$OutputDir\SplashScreen.png" $true

Write-Output "All Store Visual Assets Generated Successfully in $OutputDir"
