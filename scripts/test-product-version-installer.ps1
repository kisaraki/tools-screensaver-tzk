param([string]$IsccPath, [string]$OutputDirectory)
$ErrorActionPreference = 'Stop'
$testRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
if (-not $IsccPath) {
    $IsccPath = Join-Path $env:LOCALAPPDATA 'Programs/Inno Setup 6/ISCC.exe'
}
if (-not $OutputDirectory) {
    $OutputDirectory = Join-Path $testRoot ('target/product-version-tests-' + [Guid]::NewGuid().ToString('N'))
}
$testOutput = [IO.Path]::GetFullPath($OutputDirectory)
New-Item -ItemType Directory -Path $testOutput -Force | Out-Null
& $IsccPath /Qp "/O$testOutput" (Join-Path $testRoot 'installer/product-version-policy-tests.iss')
if ($LASTEXITCODE -ne 0) { throw 'Product version policy test compilation failed.' }
$reportPath = Join-Path $testOutput 'policy-report.txt'
[IO.File]::WriteAllText($reportPath, '', [Text.Encoding]::ASCII)
$process = Start-Process -FilePath (Join-Path $testOutput 'product-version-policy-tests.exe') `
    -ArgumentList @('/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART', ('/REPORT="' + $reportPath + '"')) `
    -WindowStyle Hidden -Wait -PassThru
# InitializeSetup deliberately returns False before wizard creation or installation.
if ($process.ExitCode -ne 1 -or -not (Test-Path -LiteralPath $reportPath)) {
    throw "Unexpected product version policy harness exit: $($process.ExitCode)"
}
$reportText = Get-Content -LiteralPath $reportPath -Raw
if ($reportText -notmatch '^PASS: 15 product version policy checks') { throw $reportText }
Write-Output $reportText
