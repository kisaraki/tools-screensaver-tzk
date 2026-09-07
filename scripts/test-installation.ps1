[CmdletBinding()]
param(
    [string]$SetupPath,
    [string]$OutputDirectory
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$scriptRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
$projectRoot = Split-Path -Parent $scriptRoot
if (-not $SetupPath) {
    $SetupPath = Join-Path $projectRoot 'dist\tools-screensaver-tzk-Setup.exe'
}
if (-not $OutputDirectory) {
    $OutputDirectory = Join-Path $projectRoot 'docs\evidence\phase5\installation'
}
$setup = (Resolve-Path -LiteralPath $SetupPath).Path
New-Item -ItemType Directory -Force -Path $OutputDirectory | Out-Null
$output = (Resolve-Path -LiteralPath $OutputDirectory).Path
$installedScr = Join-Path $env:SystemRoot 'System32\tools-screensaver-tzk.scr'
$productDir = Join-Path $env:ProgramFiles 'tools-screensaver-tzk'
$results = [Collections.Generic.List[object]]::new()
$trackedSaver = $null
$installedByTest = $false

function Get-DesktopSnapshot {
    $key = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey('Control Panel\Desktop', $false)
    if (-not $key) { throw 'HKCU\Control Panel\Desktop is unavailable.' }
    try {
        $snapshot = [ordered]@{}
        foreach ($name in @('SCRNSAVE.EXE', 'ScreenSaveTimeOut', 'ScreenSaverIsSecure', 'ScreenSaveActive')) {
            $exists = $key.GetValueNames() -contains $name
            $snapshot[$name] = if ($exists) {
                [ordered]@{
                    exists = $true
                    kind = [string]$key.GetValueKind($name)
                    value = $key.GetValue(
                        $name,
                        $null,
                        [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames)
                }
            } else {
                [ordered]@{ exists = $false; kind = $null; value = $null }
            }
        }
        $snapshot
    } finally {
        $key.Dispose()
    }
}

function Restore-CurrentSaver($Snapshot) {
    $key = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey('Control Panel\Desktop', $true)
    if (-not $key) { throw 'HKCU\Control Panel\Desktop cannot be opened for final restoration.' }
    try {
        $entry = $Snapshot['SCRNSAVE.EXE']
        if ($entry.exists) {
            $kind = [Enum]::Parse([Microsoft.Win32.RegistryValueKind], [string]$entry.kind)
            $key.SetValue('SCRNSAVE.EXE', $entry.value, $kind)
        } else {
            $key.DeleteValue('SCRNSAVE.EXE', $false)
        }
    } finally {
        $key.Dispose()
    }
}

function Assert-ProtectedValuesUnchanged($Expected, $Actual) {
    foreach ($name in @('ScreenSaveTimeOut', 'ScreenSaverIsSecure', 'ScreenSaveActive')) {
        $a = $Expected[$name] | ConvertTo-Json -Compress
        $b = $Actual[$name] | ConvertTo-Json -Compress
        if ($a -ne $b) { throw "Protected desktop setting changed: $name" }
    }
}

function Invoke-Elevated([string]$FilePath, [string[]]$Arguments, [string]$Label) {
    Write-Host "UAC required: $Label"
    $process = Start-Process -FilePath $FilePath -ArgumentList $Arguments -Verb RunAs -Wait -PassThru
    [ordered]@{ process = $process; exitCode = $process.ExitCode }
}

function Invoke-Setup([string[]]$Extra, [string]$LogName, [string]$Label) {
    $log = Join-Path $output $LogName
    $arguments = @(
        '/VERYSILENT',
        '/SUPPRESSMSGBOXES',
        '/NORESTART',
        '/RESTARTEXITCODE=3010',
        ('/LOG="' + $log + '"')
    ) + $Extra
    $run = Invoke-Elevated $setup $arguments $Label
    if ($run.exitCode -ne 0) { throw "$Label exited $($run.exitCode); expected 0 without a reboot." }
}

function Get-ProductEntries {
    @(
        Get-ItemProperty 'HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*' -ErrorAction SilentlyContinue |
            Where-Object { $_.DisplayName -eq 'tools-screensaver-tzk' }
    )
}

$baseline = Get-DesktopSnapshot
$baselineJson = $baseline | ConvertTo-Json -Depth 5
[IO.File]::WriteAllText(
    (Join-Path $output 'desktop-before.json'),
    $baselineJson,
    [Text.UTF8Encoding]::new($false))

try {
    if ((Get-ProductEntries).Count -ne 0 -or
        (Test-Path -LiteralPath $installedScr) -or
        (Test-Path -LiteralPath $productDir)) {
        throw 'A prior installation exists. This clean-install harness refuses to overwrite an unowned installation.'
    }

    Invoke-Setup @() '01-clean-default.log' 'clean install with setcurrent unchecked'
    $installedByTest = $true
    if (-not (Test-Path -LiteralPath $installedScr)) { throw 'System32 .scr is missing after install.' }
    $entries = Get-ProductEntries
    if ($entries.Count -ne 1) { throw "Expected one uninstall entry; found $($entries.Count)." }
    $installedInfo = [Diagnostics.FileVersionInfo]::GetVersionInfo($installedScr)
    $setupInfo = [Diagnostics.FileVersionInfo]::GetVersionInfo($setup)
    if (([version]$installedInfo.FileVersion).ToString(3) -ne
        ([version]$setupInfo.FileVersion.Trim()).ToString(3)) {
        throw 'Installed .scr and Setup versions differ.'
    }
    $afterDefault = Get-DesktopSnapshot
    if (($baseline | ConvertTo-Json -Depth 5 -Compress) -ne
        ($afterDefault | ConvertTo-Json -Depth 5 -Compress)) {
        throw 'Default unchecked install changed a screensaver desktop value.'
    }
    $results.Add([ordered]@{ case = 'clean install / task unchecked'; status = 'PASS' })

    Invoke-Setup @('/TASKS="setcurrent"') '02-setcurrent.log' 'same-version overlay with setcurrent selected'
    $afterSelected = Get-DesktopSnapshot
    Assert-ProtectedValuesUnchanged $baseline $afterSelected
    if (-not $afterSelected['SCRNSAVE.EXE'].exists -or
        -not ([string]$afterSelected['SCRNSAVE.EXE'].value).Equals(
            $installedScr, [StringComparison]::OrdinalIgnoreCase)) {
        throw 'Selected setcurrent task did not update the original user HKCU to the installed path.'
    }
    if ((Get-ProductEntries).Count -ne 1) { throw 'Same-version overlay duplicated the uninstall entry.' }
    $results.Add([ordered]@{ case = 'task checked / original-user helper / same-version overlay'; status = 'PASS' })

    $trackedSaver = Start-Process -FilePath $installedScr -ArgumentList '/c' -PassThru
    Start-Sleep -Milliseconds 750
    if ($trackedSaver.HasExited) { throw 'The controlled file-in-use process exited before overlay setup.' }
    Invoke-Setup @() '03-file-in-use-overlay.log' 'same-version overlay while the installed .scr is running'
    if (-not (Test-Path -LiteralPath $installedScr)) { throw 'Installed .scr is missing after file-in-use overlay.' }
    $results.Add([ordered]@{
        case = 'file in use / Inno standard close-or-replace mechanism'
        status = 'PASS'
        saverExitedByInstaller = $trackedSaver.HasExited
    })

    Restore-CurrentSaver $baseline
    $elevatedHelper = Invoke-Elevated $installedScr @('--install-set-current') 'direct elevated helper refusal'
    if ($elevatedHelper.exitCode -ne 4) {
        throw "Elevated helper exited $($elevatedHelper.exitCode); expected refusal code 4."
    }
    $afterElevatedHelper = Get-DesktopSnapshot
    if (($baseline | ConvertTo-Json -Depth 5 -Compress) -ne
        ($afterElevatedHelper | ConvertTo-Json -Depth 5 -Compress)) {
        throw 'Direct elevated helper changed a screensaver desktop value.'
    }
    $results.Add([ordered]@{ case = 'direct elevated helper'; status = 'PASS'; exitCode = 4 })

    $uninstaller = Join-Path $productDir 'unins000.exe'
    if (-not (Test-Path -LiteralPath $uninstaller)) { throw 'Standard Inno uninstaller is missing.' }
    $uninstallLog = Join-Path $output '04-uninstall.log'
    $uninstall = Invoke-Elevated $uninstaller @(
        '/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART', ('/LOG="' + $uninstallLog + '"')) 'uninstall'
    if ($uninstall.exitCode -ne 0) { throw "Uninstall exited $($uninstall.exitCode)." }
    $installedByTest = $false
    if (Test-Path -LiteralPath $installedScr) { throw 'System32 .scr remains after uninstall.' }
    if ((Get-ProductEntries).Count -ne 0) { throw 'Uninstall entry remains after uninstall.' }
    $afterUninstall = Get-DesktopSnapshot
    if (($baseline | ConvertTo-Json -Depth 5 -Compress) -ne
        ($afterUninstall | ConvertTo-Json -Depth 5 -Compress)) {
        throw 'Uninstall changed a screensaver desktop value.'
    }
    $results.Add([ordered]@{ case = 'uninstall / preserve HKCU'; status = 'PASS' })
} catch {
    $results.Add([ordered]@{ case = 'harness'; status = 'NOT TESTED'; reason = $_.Exception.Message })
    throw
} finally {
    if ($trackedSaver) {
        try {
            if (-not $trackedSaver.HasExited) {
                $trackedSaver.CloseMainWindow() | Out-Null
                if (-not $trackedSaver.WaitForExit(3000)) { $trackedSaver.Kill(); $trackedSaver.WaitForExit() }
            }
        } catch { }
        $trackedSaver.Dispose()
    }
    if ($installedByTest) {
        $cleanupUninstaller = Join-Path $productDir 'unins000.exe'
        if (Test-Path -LiteralPath $cleanupUninstaller) {
            try {
                $cleanupLog = Join-Path $output '99-cleanup-uninstall.log'
                $cleanup = Invoke-Elevated $cleanupUninstaller @(
                    '/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART', ('/LOG="' + $cleanupLog + '"')) 'cleanup after a failed installation test'
                if ($cleanup.exitCode -eq 0) {
                    $installedByTest = $false
                    $results.Add([ordered]@{ case = 'cleanup after failure'; status = 'PASS' })
                }
            } catch {
                $results.Add([ordered]@{ case = 'cleanup after failure'; status = 'ACTION REQUIRED'; reason = $_.Exception.Message })
            }
        }
    }
    try { Restore-CurrentSaver $baseline } catch {
        $results.Add([ordered]@{ case = 'restore original SCRNSAVE.EXE'; status = 'FAIL'; reason = $_.Exception.Message })
    }
    if ($installedByTest) {
        $results.Add([ordered]@{
            case = 'cleanup'
            status = 'ACTION REQUIRED'
            reason = "A test installation may remain in $productDir because elevated cleanup was not completed."
        })
    }
    $report = [ordered]@{
        capturedAt = Get-Date -Format o
        os = (Get-CimInstance Win32_OperatingSystem).Caption
        build = [Environment]::OSVersion.Version.ToString()
        setup = $setup
        installedPath = $installedScr
        results = $results
        standardUserDifferentAdministrator = 'NOT TESTED: requires a separate standard account and administrator credential'
        versionUpgrade = 'NOT TESTED: no older signed-off product build is available'
    }
    $report | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath (Join-Path $output 'installation-report.json') -Encoding utf8
}
