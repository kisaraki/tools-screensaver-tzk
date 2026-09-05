# Original geometric artwork for this project; no external images or fonts.
# Run with Windows PowerShell: powershell -NoProfile -File assets\generate-icon.ps1
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing

$iconFrames = [System.Collections.Generic.List[byte[]]]::new()
$iconSizes = @(16, 32, 48, 256)
foreach ($iconSize in $iconSizes) {
    $iconBitmap = [System.Drawing.Bitmap]::new($iconSize, $iconSize)
    $iconGraphics = [System.Drawing.Graphics]::FromImage($iconBitmap)
    $iconPen = [System.Drawing.Pen]::new([System.Drawing.Color]::FromArgb(0, 255, 0), 3)
    $iconBrush = [System.Drawing.SolidBrush]::new([System.Drawing.Color]::FromArgb(0, 255, 0))
    $iconStream = [System.IO.MemoryStream]::new()
    $iconWriter = [System.IO.BinaryWriter]::new($iconStream)
    try {
        $iconGraphics.Clear([System.Drawing.Color]::FromArgb(8, 10, 8))
        $iconGraphics.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
        $iconGraphics.ScaleTransform($iconSize / 64.0, $iconSize / 64.0)
        $iconGraphics.DrawRectangle($iconPen, 5, 5, 54, 54)
        for ($iconTick = 0; $iconTick -lt 12; $iconTick++) {
            $iconAngle = $iconTick * [Math]::PI / 6
            $iconGraphics.DrawLine(
                $iconPen,
                [single](32 + 21 * [Math]::Sin($iconAngle)),
                [single](32 - 21 * [Math]::Cos($iconAngle)),
                [single](32 + 24 * [Math]::Sin($iconAngle)),
                [single](32 - 24 * [Math]::Cos($iconAngle)))
        }
        $iconPen.Width = 4
        $iconGraphics.DrawLine($iconPen, 32, 32, 32, 17)
        $iconPen.Width = 3
        $iconGraphics.DrawLine($iconPen, 32, 32, 49, 32)
        $iconGraphics.FillEllipse($iconBrush, 29, 29, 6, 6)
        $iconGraphics.Flush()

        # ICO DIB: 32-bit BGRA pixels bottom-up, followed by an aligned AND mask.
        $iconMaskStride = [int]([Math]::Ceiling($iconSize / 32.0) * 4)
        $iconWriter.Write([uint32]40)
        $iconWriter.Write([int32]$iconSize)
        $iconWriter.Write([int32]($iconSize * 2))
        $iconWriter.Write([uint16]1)
        $iconWriter.Write([uint16]32)
        $iconWriter.Write([uint32]0)
        $iconWriter.Write([uint32]($iconSize * $iconSize * 4))
        $iconWriter.Write([int32]0)
        $iconWriter.Write([int32]0)
        $iconWriter.Write([uint32]0)
        $iconWriter.Write([uint32]0)
        for ($iconY = $iconSize - 1; $iconY -ge 0; $iconY--) {
            for ($iconX = 0; $iconX -lt $iconSize; $iconX++) {
                $iconPixel = $iconBitmap.GetPixel($iconX, $iconY)
                $iconWriter.Write([byte]$iconPixel.B)
                $iconWriter.Write([byte]$iconPixel.G)
                $iconWriter.Write([byte]$iconPixel.R)
                $iconWriter.Write([byte]$iconPixel.A)
            }
        }
        $iconWriter.Write([byte[]]::new($iconMaskStride * $iconSize))
        $iconWriter.Flush()
        $iconFrames.Add($iconStream.ToArray())
    }
    finally {
        $iconWriter.Dispose()
        $iconStream.Dispose()
        $iconGraphics.Dispose()
        $iconPen.Dispose()
        $iconBrush.Dispose()
        $iconBitmap.Dispose()
    }
}

$iconOutput = Join-Path $PSScriptRoot 'app.ico'
$iconFile = [System.IO.File]::Create($iconOutput)
$iconDirectory = [System.IO.BinaryWriter]::new($iconFile)
try {
    $iconDirectory.Write([uint16]0)
    $iconDirectory.Write([uint16]1)
    $iconDirectory.Write([uint16]$iconSizes.Count)
    $iconOffset = 6 + 16 * $iconSizes.Count
    for ($iconIndex = 0; $iconIndex -lt $iconSizes.Count; $iconIndex++) {
        $iconDimension = if ($iconSizes[$iconIndex] -eq 256) { 0 } else { $iconSizes[$iconIndex] }
        $iconDirectory.Write([byte]$iconDimension)
        $iconDirectory.Write([byte]$iconDimension)
        $iconDirectory.Write([byte]0)
        $iconDirectory.Write([byte]0)
        $iconDirectory.Write([uint16]1)
        $iconDirectory.Write([uint16]32)
        $iconDirectory.Write([uint32]$iconFrames[$iconIndex].Length)
        $iconDirectory.Write([uint32]$iconOffset)
        $iconOffset += $iconFrames[$iconIndex].Length
    }
    foreach ($iconFrame in $iconFrames) {
        $iconDirectory.Write($iconFrame)
    }
}
finally {
    $iconDirectory.Dispose()
    $iconFile.Dispose()
}
Write-Output "Generated: $iconOutput"
