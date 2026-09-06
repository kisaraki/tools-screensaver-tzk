[CmdletBinding()]
param(
    [string]$ArtifactPath,
    [string]$OutputDirectory,
    [switch]$Interactive,
    [ValidateRange(2, 60)]
    [int]$TimeoutSeconds = 10
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$scriptRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
$projectRoot = Split-Path -Parent $scriptRoot
if (-not $ArtifactPath) {
    $ArtifactPath = Join-Path $projectRoot 'dist\MyDateTimeScreensaver.scr'
}
$artifact = (Resolve-Path -LiteralPath $ArtifactPath).Path
if (-not $OutputDirectory) {
    $OutputDirectory = Join-Path $projectRoot 'docs\evidence\phase5'
}
New-Item -ItemType Directory -Force -Path $OutputDirectory | Out-Null
$output = (Resolve-Path -LiteralPath $OutputDirectory).Path

if (-not ('Phase5ResourceReader' -as [type])) {
    Add-Type -TypeDefinition @'
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;

public static class Phase5ResourceReader {
    const uint LOAD_LIBRARY_AS_DATAFILE = 0x00000002;
    const uint LOAD_LIBRARY_AS_IMAGE_RESOURCE = 0x00000020;

    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    static extern IntPtr LoadLibraryExW(string file, IntPtr reserved, uint flags);
    [DllImport("kernel32.dll", SetLastError = true)]
    static extern bool FreeLibrary(IntPtr module);
    [DllImport("kernel32.dll", SetLastError = true)]
    static extern IntPtr FindResourceW(IntPtr module, IntPtr name, IntPtr type);
    [DllImport("kernel32.dll", SetLastError = true)]
    static extern IntPtr LoadResource(IntPtr module, IntPtr resource);
    [DllImport("kernel32.dll", SetLastError = true)]
    static extern uint SizeofResource(IntPtr module, IntPtr resource);
    [DllImport("kernel32.dll", SetLastError = true)]
    static extern IntPtr LockResource(IntPtr resourceData);
    [DllImport("user32.dll", SetLastError = true)]
    static extern bool EnumWindows(EnumWindowsProc callback, IntPtr data);
    delegate bool EnumWindowsProc(IntPtr hwnd, IntPtr data);
    [DllImport("user32.dll")]
    static extern uint GetWindowThreadProcessId(IntPtr hwnd, out uint processId);
    [DllImport("user32.dll")]
    static extern bool IsWindowVisible(IntPtr hwnd);
    [DllImport("user32.dll", SetLastError = true)]
    static extern bool PostMessageW(IntPtr hwnd, uint message, IntPtr wParam, IntPtr lParam);

    public static byte[] GetResource(string path, int id, int type) {
        IntPtr module = LoadLibraryExW(path, IntPtr.Zero,
            LOAD_LIBRARY_AS_DATAFILE | LOAD_LIBRARY_AS_IMAGE_RESOURCE);
        if (module == IntPtr.Zero) throw new System.ComponentModel.Win32Exception();
        try {
            IntPtr found = FindResourceW(module, new IntPtr(id), new IntPtr(type));
            if (found == IntPtr.Zero) throw new InvalidOperationException(
                String.Format("resource type {0}, id {1} is missing", type, id));
            uint size = SizeofResource(module, found);
            IntPtr loaded = LoadResource(module, found);
            IntPtr bytes = LockResource(loaded);
            if (loaded == IntPtr.Zero || bytes == IntPtr.Zero || size == 0)
                throw new InvalidOperationException("resource cannot be loaded");
            byte[] result = new byte[size];
            Marshal.Copy(bytes, result, 0, checked((int)size));
            return result;
        } finally { FreeLibrary(module); }
    }

    public static IntPtr[] VisibleWindows(uint processId) {
        var result = new List<IntPtr>();
        EnumWindows(delegate(IntPtr hwnd, IntPtr data) {
            uint current;
            GetWindowThreadProcessId(hwnd, out current);
            if (current == processId && IsWindowVisible(hwnd)) result.Add(hwnd);
            return true;
        }, IntPtr.Zero);
        return result.ToArray();
    }

    public static void CloseVisibleWindows(uint processId) {
        foreach (IntPtr hwnd in VisibleWindows(processId))
            PostMessageW(hwnd, 0x0010, IntPtr.Zero, IntPtr.Zero);
    }
}
'@
}

function Read-U16([byte[]]$Bytes, [int]$Offset) {
    if ($Offset -lt 0 -or $Offset + 2 -gt $Bytes.Length) { throw 'PE u16 offset is out of range.' }
    [BitConverter]::ToUInt16($Bytes, $Offset)
}

function Read-I32([byte[]]$Bytes, [int]$Offset) {
    if ($Offset -lt 0 -or $Offset + 4 -gt $Bytes.Length) { throw 'PE i32 offset is out of range.' }
    [BitConverter]::ToInt32($Bytes, $Offset)
}

function Start-TrackedProcess([string]$Arguments) {
    $info = [Diagnostics.ProcessStartInfo]::new()
    $info.FileName = $script:smokeCopy
    $info.Arguments = $Arguments
    $info.UseShellExecute = $false
    $info.WorkingDirectory = $script:smokeTemp
    $process = [Diagnostics.Process]::new()
    $process.StartInfo = $info
    if (-not $process.Start()) { throw "Could not start artifact with: $Arguments" }
    $script:tracked.Add($process)
    $process
}

function Invoke-NoUiCase([string]$Arguments, [int]$ExpectedCode) {
    $process = Start-TrackedProcess $Arguments
    if (-not $process.WaitForExit($TimeoutSeconds * 1000)) {
        $process.Kill()
        $process.WaitForExit()
        throw "Timed out for noninteractive arguments: $Arguments"
    }
    if ($process.ExitCode -ne $ExpectedCode) {
        throw "Arguments '$Arguments' exited $($process.ExitCode), expected $ExpectedCode."
    }
    [ordered]@{ arguments = $Arguments; exitCode = $process.ExitCode; uiExpected = $false }
}

function Invoke-InteractiveCase([string]$Arguments) {
    $process = Start-TrackedProcess $Arguments
    $deadline = [DateTime]::UtcNow.AddSeconds($TimeoutSeconds)
    $shown = $false
    while ([DateTime]::UtcNow -lt $deadline -and -not $process.HasExited) {
        if ([Phase5ResourceReader]::VisibleWindows([uint32]$process.Id).Count -gt 0) {
            $shown = $true
            break
        }
        Start-Sleep -Milliseconds 50
    }
    if (-not $shown) {
        if (-not $process.HasExited) { $process.Kill(); $process.WaitForExit() }
        throw "No visible window appeared for interactive arguments: $Arguments"
    }
    Start-Sleep -Milliseconds 500
    [Phase5ResourceReader]::CloseVisibleWindows([uint32]$process.Id)
    if (-not $process.WaitForExit($TimeoutSeconds * 1000)) {
        $process.Kill()
        $process.WaitForExit()
        throw "Interactive cleanup timed out for: $Arguments"
    }
    if ($process.ExitCode -ne 0) {
        throw "Interactive arguments '$Arguments' exited $($process.ExitCode), expected 0."
    }
    [ordered]@{ arguments = $Arguments; exitCode = $process.ExitCode; windowObserved = $shown }
}

function Get-ScreensaverSystemSnapshot {
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

$tracked = [Collections.Generic.List[Diagnostics.Process]]::new()
$tempRoot = [IO.Path]::GetFullPath([IO.Path]::GetTempPath()).TrimEnd('\') + '\'
$smokeTemp = Join-Path $tempRoot ("MyDateTimeScreensaver-smoke-" + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $smokeTemp | Out-Null
$smokeTemp = (Resolve-Path -LiteralPath $smokeTemp).Path
$smokeCopy = Join-Path $smokeTemp 'MyDateTimeScreensaver.scr'

try {
    Copy-Item -LiteralPath $artifact -Destination $smokeCopy
    $sourceHash = (Get-FileHash -LiteralPath $artifact -Algorithm SHA256).Hash.ToLowerInvariant()
    $copyHash = (Get-FileHash -LiteralPath $smokeCopy -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($sourceHash -ne $copyHash) { throw 'Temporary .scr hash differs from the release artifact.' }

    $bytes = [IO.File]::ReadAllBytes($smokeCopy)
    if ($bytes.Length -lt 512 -or $bytes[0] -ne 0x4d -or $bytes[1] -ne 0x5a) {
        throw 'Artifact is not a valid MZ executable.'
    }
    $pe = Read-I32 $bytes 0x3c
    if ($pe -lt 0 -or $pe + 96 -gt $bytes.Length -or
        $bytes[$pe] -ne 0x50 -or $bytes[$pe + 1] -ne 0x45 -or
        $bytes[$pe + 2] -ne 0 -or $bytes[$pe + 3] -ne 0) {
        throw 'Artifact has no valid PE header.'
    }
    $machine = Read-U16 $bytes ($pe + 4)
    $optional = $pe + 24
    $magic = Read-U16 $bytes $optional
    $subsystem = Read-U16 $bytes ($optional + 68)
    if ($machine -ne 0x8664 -or $magic -ne 0x20b -or $subsystem -ne 2) {
        throw ("Expected x64 PE32+ Windows GUI; machine=0x{0:x4}, magic=0x{1:x4}, subsystem={2}" -f $machine, $magic, $subsystem)
    }

    $cargo = Get-Content -LiteralPath (Join-Path $projectRoot 'Cargo.toml') -Raw
    $versionMatch = [regex]::Match($cargo, '(?m)^version\s*=\s*"([^"]+)"')
    if (-not $versionMatch.Success) { throw 'Cargo package version is missing.' }
    $expectedVersion = $versionMatch.Groups[1].Value
    $versionInfo = [Diagnostics.FileVersionInfo]::GetVersionInfo($smokeCopy)
    if ($versionInfo.FileVersion -ne $expectedVersion -or
        $versionInfo.ProductVersion -ne $expectedVersion -or
        $versionInfo.ProductName -ne 'MyDateTimeScreensaver') {
        throw 'Embedded VERSIONINFO does not match Cargo metadata.'
    }

    $resources = @(
        [ordered]@{ type = 14; id = 101; name = 'group icon' },
        [ordered]@{ type = 5; id = 2003; name = 'configuration dialog' },
        [ordered]@{ type = 5; id = 2004; name = 'countdown dialog' },
        [ordered]@{ type = 6; id = 1; name = 'string table block' },
        [ordered]@{ type = 16; id = 1; name = 'version' },
        [ordered]@{ type = 24; id = 1; name = 'manifest' }
    )
    foreach ($resource in $resources) {
        $resource.bytes = [Phase5ResourceReader]::GetResource($smokeCopy, $resource.id, $resource.type).Length
    }
    $configDialogBytes = [Phase5ResourceReader]::GetResource($smokeCopy, 2003, 5)
    $configDialogText = [Text.Encoding]::Unicode.GetString($configDialogBytes)
    $expectedDialogLabels = @(
        '標準桌曆暨時鐘模式(&T)',
        '離機作業番茄鐘模式(&C)',
        'KOMSMOS TOOLKIT',
        '探真拓知酷'
    )
    foreach ($label in $expectedDialogLabels) {
        if (-not $configDialogText.Contains($label)) {
            throw "Configuration dialog is missing the expected label: $label"
        }
    }
    $manifestBytes = [Phase5ResourceReader]::GetResource($smokeCopy, 1, 24)
    $manifest = [Text.Encoding]::UTF8.GetString($manifestBytes).TrimStart([char]0xfeff)
    $expectedAssemblyVersion = "$expectedVersion.0"
    foreach ($token in @('level="asInvoker"', 'uiAccess="false"', 'PerMonitorV2',
            'Microsoft.Windows.Common-Controls', 'processorArchitecture="amd64"',
            "version=`"$expectedAssemblyVersion`"", '{8e0f7a12-bfb3-4fe8-b9a5-48fd50a15a9a}')) {
        if (-not $manifest.Contains($token)) { throw "Embedded manifest is missing: $token" }
    }
    [IO.File]::WriteAllText((Join-Path $output 'embedded.manifest'), $manifest, [Text.UTF8Encoding]::new($false))

    $dumpbin = (Get-Command dumpbin.exe -ErrorAction SilentlyContinue | Select-Object -First 1 -ExpandProperty Source)
    if (-not $dumpbin) {
        $vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
        if (Test-Path -LiteralPath $vswhere) {
            $dumpbin = (& $vswhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -find 'VC\Tools\MSVC\*\bin\Hostx64\x64\dumpbin.exe' | Select-Object -Last 1)
        }
    }
    if (-not $dumpbin -or -not (Test-Path -LiteralPath $dumpbin)) { throw 'x64 MSVC dumpbin.exe was not found.' }
    $dump = (& $dumpbin /headers /imports $smokeCopy 2>&1 | Out-String)
    if ($LASTEXITCODE -ne 0) { throw "dumpbin failed with exit code $LASTEXITCODE" }
    $dump | Set-Content -LiteralPath (Join-Path $output 'pe-headers-imports.txt') -Encoding utf8
    $imports = @([regex]::Matches($dump, '(?im)^\s+([A-Za-z0-9_.-]+\.dll)\s*$') |
        ForEach-Object { $_.Groups[1].Value.ToLowerInvariant() } | Sort-Object -Unique)
    $forbiddenRuntime = @($imports | Where-Object { $_ -match '^(msvcp|vcruntime|ucrtbase|api-ms-win-crt)' })
    if ($forbiddenRuntime.Count -ne 0) { throw "Dynamic VC/UCRT dependency found: $($forbiddenRuntime -join ', ')" }

    $invalidCases = @()
    $invalidCases += Invoke-NoUiCase '/p' 2
    $invalidCases += Invoke-NoUiCase '/p:0' 2
    $invalidCases += Invoke-NoUiCase '/s /c' 2
    $invalidCases += Invoke-NoUiCase '--dev-render=time-date' 2

    $helperRegistryBefore = Get-ScreensaverSystemSnapshot
    $helperRefusal = Invoke-NoUiCase '--install-set-current' 4
    $helperRegistryAfter = Get-ScreensaverSystemSnapshot
    if (($helperRegistryBefore | ConvertTo-Json -Depth 5 -Compress) -ne
        ($helperRegistryAfter | ConvertTo-Json -Depth 5 -Compress)) {
        throw 'Rejected set-current helper changed a protected screensaver registry value.'
    }

    $interactiveCases = @()
    if ($Interactive) {
        $interactiveCases += Invoke-InteractiveCase '/c'
        $interactiveCases += Invoke-InteractiveCase '/s'

        $oldTestExe = [Environment]::GetEnvironmentVariable('SCREENSAVER_TEST_EXE', 'Process')
        try {
            [Environment]::SetEnvironmentVariable('SCREENSAVER_TEST_EXE', $smokeCopy, 'Process')
            $cargoInfo = [Diagnostics.ProcessStartInfo]::new()
            $cargoInfo.FileName = 'cargo.exe'
            $cargoInfo.Arguments = 'test --release --locked --test native_modes preview_embeds_resizes_and_exits_in_three_dpi_contexts -- --ignored --exact --test-threads=1 --nocapture'
            $cargoInfo.WorkingDirectory = $projectRoot
            $cargoInfo.UseShellExecute = $false
            $cargoProcess = [Diagnostics.Process]::new()
            $cargoProcess.StartInfo = $cargoInfo
            if (-not $cargoProcess.Start()) { throw 'Could not start the bounded preview host test.' }
            $tracked.Add($cargoProcess)
            if (-not $cargoProcess.WaitForExit(120000)) {
                $cargoProcess.Kill()
                $cargoProcess.WaitForExit()
                throw 'The preview host smoke test timed out.'
            }
            if ($cargoProcess.ExitCode -ne 0) { throw "The preview host smoke test exited $($cargoProcess.ExitCode)." }
            $interactiveCases += [ordered]@{ arguments = '/p <test HWND>'; exitCode = 0; boundedHostTest = $true }
        } finally {
            [Environment]::SetEnvironmentVariable('SCREENSAVER_TEST_EXE', $oldTestExe, 'Process')
        }
    }

    $signature = Get-AuthenticodeSignature -LiteralPath $smokeCopy
    $report = [ordered]@{
        capturedAt = (Get-Date -Format o)
        artifact = $artifact
        temporaryScr = $smokeCopy
        sha256 = $copyHash
        bytes = (Get-Item -LiteralPath $smokeCopy).Length
        machine = '0x8664 (x64)'
        optionalHeader = 'PE32+'
        subsystem = 'Windows GUI'
        version = $expectedVersion
        fileDescription = $versionInfo.FileDescription
        resources = $resources
        configurationLabels = $expectedDialogLabels
        manifestChecks = 'PASS'
        imports = $imports
        staticCrtCheck = 'PASS (no dynamic VC/UCRT import)'
        signatureStatus = [string]$signature.Status
        invalidArgumentCases = $invalidCases
        installHelperRefusal = $helperRefusal
        protectedRegistryBefore = $helperRegistryBefore
        protectedRegistryAfter = $helperRegistryAfter
        interactive = [bool]$Interactive
        interactiveCases = $interactiveCases
        result = 'PASS'
    }
    $report | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath (Join-Path $output 'smoke-report.json') -Encoding utf8
    $report | ConvertTo-Json -Depth 8
} finally {
    foreach ($process in $tracked) {
        try {
            if (-not $process.HasExited) { $process.Kill(); $process.WaitForExit() }
        } catch { }
        $process.Dispose()
    }
    $resolvedTemp = [IO.Path]::GetFullPath($smokeTemp)
    if (-not $resolvedTemp.StartsWith($tempRoot, [StringComparison]::OrdinalIgnoreCase) -or
        [IO.Path]::GetFileName($resolvedTemp) -notlike 'MyDateTimeScreensaver-smoke-*') {
        throw "Refusing to clean unexpected temporary path: $resolvedTemp"
    }
    if (Test-Path -LiteralPath $resolvedTemp) {
        Remove-Item -LiteralPath $resolvedTemp -Recurse -Force
    }
}
