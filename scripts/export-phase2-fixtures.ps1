param([string]$OutputDirectory)
$ErrorActionPreference = 'Stop'
$phase2ProjectRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
if (-not $OutputDirectory) { $OutputDirectory = Join-Path $phase2ProjectRoot 'docs/evidence/phase2/fixtures' }
$phase2OutputRoot = [System.IO.Path]::GetFullPath($OutputDirectory)
$phase2RawRoot = Join-Path $phase2ProjectRoot 'target/phase2-fixtures-bmp'
New-Item -ItemType Directory -Path $phase2RawRoot,$phase2OutputRoot -Force | Out-Null
$phase2OldFixturePath = $env:PHASE2_FIXTURES
Push-Location $phase2ProjectRoot
try {
    $env:PHASE2_FIXTURES = $phase2RawRoot
    cargo test --locked --lib export_visual_fixtures -- --ignored --nocapture
    if ($LASTEXITCODE -ne 0) { throw 'GDI fixture rendering failed.' }
    Add-Type -AssemblyName System.Drawing
    foreach ($phase2File in Get-ChildItem -LiteralPath $phase2RawRoot -Filter '*.bmp' -File) {
        $phase2Bitmap = [System.Drawing.Bitmap]::FromFile($phase2File.FullName)
        try {
            $phase2Bitmap.Save((Join-Path $phase2OutputRoot ($phase2File.BaseName + '.png')), [System.Drawing.Imaging.ImageFormat]::Png)
        } finally { $phase2Bitmap.Dispose() }
    }
    Write-Output "PNG fixtures: $phase2OutputRoot"
    Write-Output "Uncompressed GDI output: $phase2RawRoot"
} finally {
    Pop-Location
    if ($null -eq $phase2OldFixturePath) { Remove-Item Env:\PHASE2_FIXTURES -ErrorAction SilentlyContinue }
    else { $env:PHASE2_FIXTURES = $phase2OldFixturePath }
}
