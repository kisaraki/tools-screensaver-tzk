param([string]$OutputDirectory)
$ErrorActionPreference = 'Stop'
# VsDevCmd may inherit PowerShell 7's module search path. Resolve the Windows
# PowerShell inbox modules explicitly when package.bat invokes powershell.exe.
Import-Module (Join-Path $PSHOME 'Modules/Microsoft.PowerShell.Utility/Microsoft.PowerShell.Utility.psd1')
Import-Module (Join-Path $PSHOME 'Modules/Microsoft.PowerShell.Security/Microsoft.PowerShell.Security.psd1')
$webviewProjectRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
if (-not $OutputDirectory) { $OutputDirectory = Join-Path $webviewProjectRoot 'target/webview2' }
$webviewOutputRoot = [IO.Path]::GetFullPath($OutputDirectory)
$lock = Get-Content -LiteralPath (Join-Path $webviewProjectRoot 'installer/webview2-bootstrapper.json') -Raw | ConvertFrom-Json
if ($lock.fileName -ne 'MicrosoftEdgeWebview2Setup.exe' -or $lock.sha256 -notmatch '^[a-f0-9]{64}$') {
    throw 'Invalid WebView2 bootstrapper lock file.'
}
$uri = [Uri]$lock.url
if ($uri.Scheme -ne 'https' -or $uri.Host -ne 'msedge.sf.dl.delivery.mp.microsoft.com') {
    throw 'The locked bootstrapper must use the official Microsoft HTTPS distribution host.'
}
New-Item -ItemType Directory -Path $webviewOutputRoot -Force | Out-Null
$bootstrapper = Join-Path $webviewOutputRoot $lock.fileName
if (-not (Test-Path -LiteralPath $bootstrapper)) {
    [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
    Invoke-WebRequest -UseBasicParsing -Uri $uri -OutFile $bootstrapper -TimeoutSec 120
}
if ((Get-Item -LiteralPath $bootstrapper).Length -ne $lock.bytes -or
    (Get-FileHash -LiteralPath $bootstrapper -Algorithm SHA256).Hash.ToLowerInvariant() -ne $lock.sha256) {
    throw 'WebView2 bootstrapper differs from the lock. Do not package it; review the lock and cached file.'
}
$signature = Get-AuthenticodeSignature -LiteralPath $bootstrapper
if ($signature.Status -ne 'Valid' -or $signature.SignerCertificate.Subject -notmatch '(^|, )O=Microsoft Corporation(,|$)') {
    throw 'WebView2 bootstrapper must have a valid Microsoft Corporation Authenticode signature.'
}
$version = [Diagnostics.FileVersionInfo]::GetVersionInfo($bootstrapper).FileVersion.Trim()
if ($version -ne $lock.version) { throw 'WebView2 bootstrapper file version differs from the lock.' }
$include = '#define WebView2BootstrapperSHA256 "' + $lock.sha256 + '"' + [Environment]::NewLine
[IO.File]::WriteAllText((Join-Path $webviewOutputRoot 'verified-bootstrapper.iss'), $include, [Text.Encoding]::ASCII)
[pscustomobject]@{
    version = $version
    bytes = $lock.bytes
    sha256 = $lock.sha256
    signature = $signature.Status.ToString()
    publisher = $lock.publisher
    executed = $false
} | ConvertTo-Json
