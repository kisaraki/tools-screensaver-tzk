# Explicit, bounded GUI smoke test. Requires an initialized MSVC Developer Shell.
$ErrorActionPreference = 'Stop'
$phase2Root = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$phase2Evidence = Join-Path $phase2Root 'docs/evidence/phase2'
$phase2DebugPath = Join-Path $phase2Root 'target/x86_64-pc-windows-msvc/debug/my_datetime_screensaver.exe'
$phase2ReleasePath = Join-Path $phase2Root 'target/x86_64-pc-windows-msvc/release/my_datetime_screensaver.exe'
New-Item -ItemType Directory -Path $phase2Evidence -Force | Out-Null
if (-not ('Phase2EntryWindow' -as [type])) {
    Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class Phase2EntryWindow {
    [StructLayout(LayoutKind.Sequential)] public struct Rect { public int Left, Top, Right, Bottom; }
    [DllImport("user32.dll", SetLastError=true)] public static extern bool PostMessage(IntPtr hwnd, uint message, UIntPtr wp, IntPtr lp);
    [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr hwnd, out Rect rect);
    [DllImport("user32.dll")] public static extern uint GetDpiForWindow(IntPtr hwnd);
    [DllImport("user32.dll")] public static extern IntPtr GetWindowDpiAwarenessContext(IntPtr hwnd);
    [DllImport("user32.dll")] public static extern IntPtr SetThreadDpiAwarenessContext(IntPtr context);
}
'@
}
$phase2Results = @()
foreach ($phase2Mode in @('time-date','countdown')) {
    $phase2Gui = $null
    $phase2Run = Start-Process -FilePath (Get-Command cargo).Source -ArgumentList @('run','--locked','--',"--dev-render=$phase2Mode") -WorkingDirectory $phase2Root -WindowStyle Hidden -PassThru -RedirectStandardOutput (Join-Path $phase2Evidence "cargo-run-$phase2Mode.stdout.txt") -RedirectStandardError (Join-Path $phase2Evidence "cargo-run-$phase2Mode.stderr.txt")
    try {
        $phase2Deadline = [DateTime]::UtcNow.AddSeconds(30)
        $phase2Descendants = [Collections.Generic.HashSet[int]]::new()
        [void]$phase2Descendants.Add($phase2Run.Id)
        while ($null -eq $phase2Gui) {
            foreach ($phase2Parent in @($phase2Descendants)) {
                foreach ($phase2Child in @(Get-CimInstance Win32_Process -Filter "ParentProcessId = $phase2Parent")) {
                    [void]$phase2Descendants.Add([int]$phase2Child.ProcessId)
                    if ($phase2Child.ExecutablePath -eq $phase2DebugPath) {
                        $phase2Gui = [Diagnostics.Process]::GetProcessById([int]$phase2Child.ProcessId)
                    }
                }
            }
            if ($phase2Run.HasExited -or [DateTime]::UtcNow -ge $phase2Deadline) { throw "cargo run $phase2Mode failed to create its GUI process" }
            Start-Sleep -Milliseconds 50
        }
        do {
            $phase2Gui.Refresh()
            if ($phase2Gui.HasExited -or [DateTime]::UtcNow -ge $phase2Deadline) { throw "No live $phase2Mode window" }
            Start-Sleep -Milliseconds 50
        } while ($phase2Gui.MainWindowHandle -eq [IntPtr]::Zero)
        Start-Sleep -Milliseconds 1100
        $phase2Gui.Refresh()
        if ($phase2Gui.HasExited) { throw "$phase2Mode exited during initial painting" }
        $phase2Rect = [Phase2EntryWindow+Rect]::new()
        $phase2Context = [Phase2EntryWindow]::GetWindowDpiAwarenessContext($phase2Gui.MainWindowHandle)
        $phase2OldContext = [Phase2EntryWindow]::SetThreadDpiAwarenessContext($phase2Context)
        if ($phase2OldContext -eq [IntPtr]::Zero) { throw 'Cannot match window DPI context for measurement' }
        try {
            if (-not [Phase2EntryWindow]::GetClientRect($phase2Gui.MainWindowHandle,[ref]$phase2Rect)) { throw 'Cannot read client size' }
        } finally { [void][Phase2EntryWindow]::SetThreadDpiAwarenessContext($phase2OldContext) }
        $phase2Dpi = [Phase2EntryWindow]::GetDpiForWindow($phase2Gui.MainWindowHandle)
        if (-not [Phase2EntryWindow]::PostMessage($phase2Gui.MainWindowHandle,0x10,[UIntPtr]::Zero,[IntPtr]::Zero)) { throw 'Cannot close test window' }
        if (-not $phase2Run.WaitForExit(10000)) { throw 'cargo run cleanup timed out' }
        if ($phase2Run.ExitCode -ne 0) { throw "cargo run exit $($phase2Run.ExitCode)" }
        $phase2Results += [ordered]@{profile='Debug';mode=$phase2Mode;command="cargo run --locked -- --dev-render=$phase2Mode";exitCode=$phase2Run.ExitCode;clientWidth=($phase2Rect.Right-$phase2Rect.Left);clientHeight=($phase2Rect.Bottom-$phase2Rect.Top);dpi=$phase2Dpi}
    } finally {
        if ($null -ne $phase2Gui) {
            if (-not $phase2Gui.HasExited) { $phase2Gui.Kill(); [void]$phase2Gui.WaitForExit(5000) }
            $phase2Gui.Dispose()
        }
        if (-not $phase2Run.HasExited) { $phase2Run.Kill(); [void]$phase2Run.WaitForExit(5000) }
        $phase2Run.Dispose()
    }
}
foreach ($phase2Mode in @('time-date','countdown')) {
    $phase2Rejected = Start-Process -FilePath $phase2ReleasePath -ArgumentList "--dev-render=$phase2Mode" -WindowStyle Hidden -PassThru
    try {
        if (-not $phase2Rejected.WaitForExit(5000)) { throw 'Release developer flag did not exit promptly' }
        if ($phase2Rejected.ExitCode -ne 2) { throw "Release flag returned $($phase2Rejected.ExitCode) instead of 2" }
        $phase2Results += [ordered]@{profile='Release';mode=$phase2Mode;exitCode=$phase2Rejected.ExitCode}
    } finally {
        if (-not $phase2Rejected.HasExited) { $phase2Rejected.Kill(); [void]$phase2Rejected.WaitForExit(5000) }
        $phase2Rejected.Dispose()
    }
}
$phase2Results | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $phase2Evidence 'developer-entrypoints.json') -Encoding utf8
$phase2Results | ConvertTo-Json
