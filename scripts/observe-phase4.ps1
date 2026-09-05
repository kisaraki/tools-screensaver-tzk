[CmdletBinding()]
param(
    [string]$ArtifactPath,
    [string]$OutputDirectory,
    [ValidateRange(5, 1800)]
    [int]$DurationSeconds = 1800,
    [switch]$AllowShortRun
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$scriptRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
$projectRoot = Split-Path -Parent $scriptRoot
if (-not $ArtifactPath) {
    $ArtifactPath = Join-Path $projectRoot 'target\x86_64-pc-windows-msvc\release\my_datetime_screensaver.exe'
}
$artifact = (Resolve-Path -LiteralPath $ArtifactPath).Path
if (-not $OutputDirectory) {
    $OutputDirectory = Join-Path $projectRoot 'docs\evidence\phase4'
}
New-Item -ItemType Directory -Force -Path $OutputDirectory | Out-Null
$output = (Resolve-Path -LiteralPath $OutputDirectory).Path
if ($DurationSeconds -lt 1800 -and -not $AllowShortRun) {
    throw 'A qualifying Phase 4 run requires 1800 seconds per mode. Use -AllowShortRun only to test the harness.'
}

if (-not ('Phase4PreviewHost' -as [type])) {
    Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
using System.Threading;

public sealed class Phase4PreviewHost : IDisposable {
    const uint WS_POPUP = 0x80000000;
    const uint WS_VISIBLE = 0x10000000;
    const uint WS_EX_TOOLWINDOW = 0x00000080;
    const uint WS_EX_NOACTIVATE = 0x08000000;
    const uint SWP_NOACTIVATE = 0x0010;
    const uint SWP_NOZORDER = 0x0004;
    const uint WM_CLOSE = 0x0010;
    const uint WM_DESTROY = 0x0002;
    const uint GR_GDIOBJECTS = 0;
    const uint GR_USEROBJECTS = 1;

    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
    struct WNDCLASSW {
        public uint style;
        public IntPtr lpfnWndProc;
        public int cbClsExtra;
        public int cbWndExtra;
        public IntPtr hInstance;
        public IntPtr hIcon;
        public IntPtr hCursor;
        public IntPtr hbrBackground;
        public string lpszMenuName;
        public string lpszClassName;
    }
    [StructLayout(LayoutKind.Sequential)]
    struct MSG {
        public IntPtr hwnd;
        public uint message;
        public UIntPtr wParam;
        public IntPtr lParam;
        public uint time;
        public int x;
        public int y;
        public uint privateValue;
    }
    delegate IntPtr WindowProc(IntPtr hwnd, uint message, UIntPtr wParam, IntPtr lParam);

    [DllImport("kernel32.dll", CharSet = CharSet.Unicode)]
    static extern IntPtr GetModuleHandleW(string name);
    [DllImport("user32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    static extern ushort RegisterClassW(ref WNDCLASSW windowClass);
    [DllImport("user32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    static extern IntPtr CreateWindowExW(uint exStyle, string className, string title,
        uint style, int x, int y, int width, int height, IntPtr parent, IntPtr menu,
        IntPtr instance, IntPtr parameter);
    [DllImport("user32.dll")]
    static extern IntPtr DefWindowProcW(IntPtr hwnd, uint message, UIntPtr wParam, IntPtr lParam);
    [DllImport("user32.dll")]
    static extern bool DestroyWindow(IntPtr hwnd);
    [DllImport("user32.dll")]
    static extern void PostQuitMessage(int code);
    [DllImport("user32.dll")]
    static extern int GetMessageW(out MSG message, IntPtr hwnd, uint min, uint max);
    [DllImport("user32.dll")]
    static extern bool TranslateMessage(ref MSG message);
    [DllImport("user32.dll")]
    static extern IntPtr DispatchMessageW(ref MSG message);
    [DllImport("user32.dll")]
    static extern bool PostMessageW(IntPtr hwnd, uint message, UIntPtr wParam, IntPtr lParam);
    [DllImport("user32.dll", SetLastError = true)]
    static extern bool SetWindowPos(IntPtr hwnd, IntPtr after, int x, int y, int width,
        int height, uint flags);
    [DllImport("user32.dll")]
    static extern IntPtr SetThreadDpiAwarenessContext(IntPtr context);
    [DllImport("user32.dll")]
    static extern uint GetGuiResources(IntPtr process, uint flags);

    static readonly WindowProc Callback = WndProc;
    readonly ManualResetEventSlim ready = new ManualResetEventSlim(false);
    readonly Thread thread;
    IntPtr window;
    Exception failure;

    public Phase4PreviewHost(int x, int y, int width, int height) {
        thread = new Thread(delegate() { Run(x, y, width, height); });
        thread.IsBackground = true;
        thread.Name = "Phase4 preview host";
        thread.Start();
        if (!ready.Wait(TimeSpan.FromSeconds(10)))
            throw new TimeoutException("preview host creation timed out");
        if (failure != null) throw new InvalidOperationException("preview host failed", failure);
        if (window == IntPtr.Zero) throw new InvalidOperationException("preview host is null");
    }

    void Run(int x, int y, int width, int height) {
        try {
            SetThreadDpiAwarenessContext(new IntPtr(-1)); // DPI unaware: 96-DPI baseline.
            string name = "MyDateTimeScreensaver.Phase4Host." + Guid.NewGuid().ToString("N");
            var wc = new WNDCLASSW();
            wc.lpfnWndProc = Marshal.GetFunctionPointerForDelegate(Callback);
            wc.hInstance = GetModuleHandleW(null);
            wc.lpszClassName = name;
            if (RegisterClassW(ref wc) == 0)
                throw new System.ComponentModel.Win32Exception();
            window = CreateWindowExW(WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE, name,
                "Phase 4 resource observation", WS_POPUP | WS_VISIBLE,
                x, y, width, height, IntPtr.Zero, IntPtr.Zero, wc.hInstance, IntPtr.Zero);
            if (window == IntPtr.Zero) throw new System.ComponentModel.Win32Exception();
        } catch (Exception error) { failure = error; }
        finally { ready.Set(); }
        if (window == IntPtr.Zero) return;
        MSG message;
        while (GetMessageW(out message, IntPtr.Zero, 0, 0) > 0) {
            TranslateMessage(ref message);
            DispatchMessageW(ref message);
        }
        window = IntPtr.Zero;
    }

    static IntPtr WndProc(IntPtr hwnd, uint message, UIntPtr wParam, IntPtr lParam) {
        if (message == WM_CLOSE) { DestroyWindow(hwnd); return IntPtr.Zero; }
        if (message == WM_DESTROY) { PostQuitMessage(0); return IntPtr.Zero; }
        return DefWindowProcW(hwnd, message, wParam, lParam);
    }

    public IntPtr Handle { get { return window; } }
    public void Resize(int x, int y, int width, int height) {
        if (window == IntPtr.Zero || !SetWindowPos(window, IntPtr.Zero, x, y, width, height,
            SWP_NOACTIVATE | SWP_NOZORDER))
            throw new System.ComponentModel.Win32Exception();
    }
    public void Dispose() {
        if (window != IntPtr.Zero) PostMessageW(window, WM_CLOSE, UIntPtr.Zero, IntPtr.Zero);
        if (!thread.Join(TimeSpan.FromSeconds(10)))
            throw new TimeoutException("preview host cleanup timed out");
        ready.Dispose();
    }
    public static uint GdiObjects(IntPtr process) { return GetGuiResources(process, GR_GDIOBJECTS); }
    public static uint UserObjects(IntPtr process) { return GetGuiResources(process, GR_USEROBJECTS); }
}
'@
}

function Set-ObservationConfig([int]$Mode, [int]$Cycle) {
    $font = $Cycle % 3
    $color = $Cycle % 4
    New-ItemProperty -LiteralPath $script:registryPath -Name 'DisplayMode' -PropertyType DWord -Value $Mode -Force | Out-Null
    New-ItemProperty -LiteralPath $script:registryPath -Name 'ColorPreset' -PropertyType DWord -Value $color -Force | Out-Null
    New-ItemProperty -LiteralPath $script:registryPath -Name 'FontMode' -PropertyType DWord -Value $font -Force | Out-Null
    New-ItemProperty -LiteralPath $script:registryPath -Name 'LastCountdownDurationSeconds' -PropertyType DWord -Value 300 -Force | Out-Null
    New-ItemProperty -LiteralPath $script:registryPath -Name 'SchemaVersion' -PropertyType DWord -Value 2 -Force | Out-Null
}

function Get-Sample([Diagnostics.Process]$Process, [Diagnostics.Process]$Controller,
        [string]$Mode, [double]$Elapsed, [int]$Cycle) {
    $Process.Refresh()
    [ordered]@{
        elapsedSeconds = [Math]::Round($Elapsed, 3)
        mode = $Mode
        pid = $Process.Id
        cpuSeconds = [Math]::Round($Process.TotalProcessorTime.TotalSeconds, 6)
        workingSetBytes = $Process.WorkingSet64
        privateBytes = $Process.PrivateMemorySize64
        gdiObjects = [Phase4PreviewHost]::GdiObjects($Process.Handle)
        userObjects = [Phase4PreviewHost]::UserObjects($Process.Handle)
        handleCount = $Process.HandleCount
        controllerWorkingSetBytes = $Controller.WorkingSet64
        cycle = $Cycle
    }
}

function Observe-Mode([int]$ModeValue, [string]$ModeName) {
    Set-ObservationConfig $ModeValue 0
    $hostWindow = [Phase4PreviewHost]::new(-3800, 20, 1920, 1080)
    $process = $null
    try {
        $info = [Diagnostics.ProcessStartInfo]::new()
        $info.FileName = $artifact
        $info.Arguments = "/p $([int64]$hostWindow.Handle)"
        $info.UseShellExecute = $false
        $info.WorkingDirectory = $projectRoot
        $process = [Diagnostics.Process]::new()
        $process.StartInfo = $info
        if (-not $process.Start()) { throw "Could not start $ModeName preview." }
        $script:children.Add($process)
        Start-Sleep -Seconds 2
        if ($process.HasExited) { throw "$ModeName preview exited during startup." }

        $watch = [Diagnostics.Stopwatch]::StartNew()
        $cycleInterval = if ($DurationSeconds -eq 1800) { 30.0 } else { 2.0 }
        $nextCycle = $cycleInterval
        $cycle = 0
        $targets = if ($DurationSeconds -eq 1800) {
            @(0, 60, 300, 600, 900, 1200, 1500, 1800)
        } else {
            @(0, $DurationSeconds)
        }
        $samples = @()
        foreach ($target in $targets) {
            while ($watch.Elapsed.TotalSeconds -lt $target) {
                if ($process.HasExited) { throw "$ModeName preview exited before $DurationSeconds seconds." }
                if ($watch.Elapsed.TotalSeconds -ge $nextCycle) {
                    $cycle++
                    Set-ObservationConfig $ModeValue $cycle
                    $sizes = @(
                        @(320, 180), @(120, 80), @(960, 540), @(1920, 1080),
                        @(1080, 720), @(1600, 900)
                    )
                    $size = $sizes[$cycle % $sizes.Count]
                    $hostWindow.Resize(-3800, 20, $size[0], $size[1])
                    $nextCycle += $cycleInterval
                }
                Start-Sleep -Milliseconds 200
            }
            $sample = Get-Sample $process $script:controller $ModeName $watch.Elapsed.TotalSeconds $cycle
            $samples += $sample
            $line = @(
                $sample.elapsedSeconds, $sample.mode, $sample.pid, $sample.cpuSeconds,
                $sample.workingSetBytes, $sample.privateBytes, $sample.gdiObjects,
                $sample.userObjects, $sample.handleCount, $sample.cycle
            ) -join ','
            Add-Content -LiteralPath $script:csv -Value $line -Encoding utf8
            Write-Host ("{0} {1,7:n1}s: CPU {2:n3}s, private {3:n1} MiB, GDI {4}, USER {5}, cycles {6}" -f
                $ModeName, $sample.elapsedSeconds, $sample.cpuSeconds,
                ($sample.privateBytes / 1MB), $sample.gdiObjects, $sample.userObjects, $sample.cycle)
        }
        $watch.Stop()

        $first = $samples[0]
        $baseline = if ($DurationSeconds -eq 1800) { $samples[1] } else { $samples[0] }
        $final = $samples[-1]
        $wall = [Math]::Max(0.001, $final.elapsedSeconds - $baseline.elapsedSeconds)
        $cpuPercent = 100.0 * ($final.cpuSeconds - $baseline.cpuSeconds) /
            ($wall * [Environment]::ProcessorCount)
        $allowedPrivateGrowth = [Math]::Max(4MB, [double]$baseline.privateBytes * 0.10)
        $late = if ($DurationSeconds -eq 1800) { @($samples | Where-Object { $_.elapsedSeconds -ge 600 }) } else { $samples }
        $gdiValues = @($late | ForEach-Object { [int64]$_.gdiObjects })
        $userValues = @($late | ForEach-Object { [int64]$_.userObjects })
        $gdiRange = (($gdiValues | Measure-Object -Maximum).Maximum - ($gdiValues | Measure-Object -Minimum).Minimum)
        $userRange = (($userValues | Measure-Object -Maximum).Maximum - ($userValues | Measure-Object -Minimum).Minimum)
        $maxWorkingSet = ($samples | ForEach-Object { [int64]$_.workingSetBytes } | Measure-Object -Maximum).Maximum
        $memoryBudget = [int64](24MB + 1.5 * (4 * 1920 * 1080))
        $qualifying = $DurationSeconds -eq 1800
        $pass = (-not $qualifying) -or ($cpuPercent -lt 1.0 -and $gdiRange -le 20 -and
            $userRange -le 20 -and
            $final.privateBytes -le [double]$baseline.privateBytes + $allowedPrivateGrowth -and
            $maxWorkingSet -le $memoryBudget -and $final.cycle -ge 50)
        [ordered]@{
            mode = $ModeName
            durationSeconds = $final.elapsedSeconds
            releaseProcess = $true
            previewDpi = 96
            qualifying30MinuteRun = $qualifying
            resizeAndFontCycles = $final.cycle
            logicalProcessors = [Environment]::ProcessorCount
            wholeMachineCpuPercentAfterWarmup = [Math]::Round($cpuPercent, 4)
            baselinePrivateBytes = $baseline.privateBytes
            finalPrivateBytes = $final.privateBytes
            allowedPrivateGrowthBytes = [int64]$allowedPrivateGrowth
            maxWorkingSetBytes = [int64]$maxWorkingSet
            engineeringMemoryBudgetBytes = $memoryBudget
            lateGdiRange = $gdiRange
            lateUserRange = $userRange
            samples = $samples
            result = if ($pass) { if ($qualifying) { 'PASS' } else { 'HARNESS PASS (short run)' } } else { 'FAIL' }
        }
    } finally {
        $hostWindow.Dispose()
        if ($null -ne $process) {
            if (-not $process.WaitForExit(5000)) { $process.Kill(); $process.WaitForExit() }
        }
    }
}

$registryPath = 'HKCU:\Software\MyDateTimeScreensaver'
$leaseName = 'Phase4ObservationLease'
$lease = [Guid]::NewGuid().ToString('N')
if (Test-Path -LiteralPath $registryPath) {
    throw 'Refusing to run: HKCU\Software\MyDateTimeScreensaver already exists. Preserve the user setting and use a clean test account.'
}

$children = [Collections.Generic.List[Diagnostics.Process]]::new()
$controller = [Diagnostics.Process]::GetCurrentProcess()
$csv = Join-Path $output 'resource-performance.csv'
'elapsed_seconds,mode,pid,cpu_seconds,working_set_bytes,private_bytes,gdi_objects,user_objects,handle_count,resize_font_cycle' |
    Set-Content -LiteralPath $csv -Encoding utf8
$results = @()
$cleanup = 'pending'
try {
    New-Item -Path $registryPath -Force | Out-Null
    New-ItemProperty -LiteralPath $registryPath -Name $leaseName -PropertyType String -Value $lease -Force | Out-Null
    $results += Observe-Mode 0 'TimeDate'
    $results += Observe-Mode 1 'Countdown'
    if (@($results | Where-Object { $_.result -eq 'FAIL' }).Count -ne 0) {
        throw 'At least one resource/performance observation failed its threshold.'
    }
} finally {
    foreach ($process in $children) {
        try {
            if (-not $process.HasExited) { $process.Kill(); $process.WaitForExit() }
        } catch { }
        $process.Dispose()
    }
    if (Test-Path -LiteralPath $registryPath) {
        $key = Get-Item -LiteralPath $registryPath
        $names = @($key.GetValueNames())
        $subkeys = @($key.GetSubKeyNames())
        $allowed = @($leaseName, 'DisplayMode', 'ColorPreset', 'FontMode',
            'LastCountdownDurationSeconds', 'SchemaVersion')
        $unexpected = @($names | Where-Object { $_ -notin $allowed })
        $actualLease = Get-ItemPropertyValue -LiteralPath $registryPath -Name $leaseName -ErrorAction SilentlyContinue
        if ($subkeys.Count -eq 0 -and $unexpected.Count -eq 0 -and $actualLease -eq $lease) {
            Remove-Item -LiteralPath $registryPath -Recurse -Force
            $cleanup = 'PASS: removed only the leased observation key'
        } else {
            $cleanup = 'PRESERVED: key changed outside the observation harness'
        }
    } else {
        $cleanup = 'PASS: observation key already absent'
    }
    $summary = [ordered]@{
        capturedAt = (Get-Date -Format o)
        artifact = $artifact
        artifactSha256 = (Get-FileHash -LiteralPath $artifact -Algorithm SHA256).Hash.ToLowerInvariant()
        tool = '.NET Process counters and Win32 GetGuiResources'
        controllerPid = $controller.Id
        requestedSecondsPerMode = $DurationSeconds
        results = $results
        registryCleanup = $cleanup
        scope = 'Release /p at a 96-DPI host; full-screen 10-Hz countdown performance is reported separately as not tested'
    }
    $summary | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath (Join-Path $output 'resource-performance.json') -Encoding utf8
}

$summary | ConvertTo-Json -Depth 10
