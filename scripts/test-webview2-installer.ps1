param([string]$IsccPath, [string]$OutputDirectory)
$ErrorActionPreference = 'Stop'
$webviewTestRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
if (-not $IsccPath) {
    $IsccPath = Join-Path $env:LOCALAPPDATA 'Programs/Inno Setup 6/ISCC.exe'
}
if (-not $OutputDirectory) {
    $OutputDirectory = Join-Path $webviewTestRoot ('target/webview2-tests-' + [Guid]::NewGuid().ToString('N'))
}
$webviewTestOutput = [IO.Path]::GetFullPath($OutputDirectory)
New-Item -ItemType Directory -Path $webviewTestOutput -Force | Out-Null
& $IsccPath /Qp "/O$webviewTestOutput" (Join-Path $webviewTestRoot 'installer/webview2-policy-tests.iss')
if ($LASTEXITCODE -ne 0) { throw 'WebView2 policy test compilation failed.' }
$reportPath = Join-Path $webviewTestOutput 'policy-report.txt'
[IO.File]::WriteAllText($reportPath, '', [Text.Encoding]::ASCII)
$process = Start-Process -FilePath (Join-Path $webviewTestOutput 'webview2-policy-tests.exe') `
    -ArgumentList @('/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART', ('/REPORT="' + $reportPath + '"')) `
    -WindowStyle Hidden -Wait -PassThru
# InitializeSetup deliberately returns False, before wizard creation or install.
if ($process.ExitCode -ne 1 -or -not (Test-Path -LiteralPath $reportPath)) {
    throw "Unexpected policy harness exit: $($process.ExitCode)"
}
$reportText = Get-Content -LiteralPath $reportPath -Raw
if ($reportText -notmatch '^PASS: 19 policy checks') { throw $reportText }
Write-Output $reportText
