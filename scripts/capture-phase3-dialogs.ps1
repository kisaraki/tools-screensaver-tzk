param(
    [string]$Executable = (Join-Path $PSScriptRoot '..\target\x86_64-pc-windows-msvc\debug\tools-screensaver-tzk.exe'),
    [string]$OutputDirectory = (Join-Path $PSScriptRoot '..\docs\evidence\phase3')
)

$ErrorActionPreference = 'Stop'

Add-Type -AssemblyName System.Drawing
Add-Type @'
using System;
using System.Runtime.InteropServices;

public static class Phase3CaptureNative
{
    [StructLayout(LayoutKind.Sequential)]
    public struct RECT { public int Left, Top, Right, Bottom; }

    [DllImport("user32.dll")]
    public static extern bool GetWindowRect(IntPtr hwnd, out RECT rect);

    [DllImport("user32.dll")]
    public static extern IntPtr SetThreadDpiAwarenessContext(IntPtr value);

    [DllImport("user32.dll")]
    public static extern bool PrintWindow(IntPtr hwnd, IntPtr hdc, uint flags);
}
'@

[void][Phase3CaptureNative]::SetThreadDpiAwarenessContext([IntPtr](-4))

$testKey = 'Software\tools-screensaver-tzk\Tests\phase3-capture'
$testEnvironment = 'MYDATETIME_SCREENSAVER_TEST_KEY'
$registry = [Microsoft.Win32.Registry]::CurrentUser

function Remove-TestKey {
    try {
        $registry.DeleteSubKeyTree($testKey, $false)
    }
    catch [System.ArgumentException] {
    }
    $software = $registry.OpenSubKey('Software', $true)
    try {
        $product = $software.OpenSubKey('tools-screensaver-tzk', $true)
        if ($null -eq $product) {
            return
        }
        try {
            $tests = $product.OpenSubKey('Tests', $false)
            if ($null -ne $tests) {
                try {
                    $testValues = @($tests.GetValueNames())
                    $testChildren = @($tests.GetSubKeyNames())
                }
                finally {
                    $tests.Dispose()
                }
                if ($testValues.Count -eq 0 -and $testChildren.Count -eq 0) {
                    $product.DeleteSubKey('Tests', $false)
                }
            }
            $productValues = @($product.GetValueNames())
            $productChildren = @($product.GetSubKeyNames())
        }
        finally {
            $product.Dispose()
        }
        if ($productValues.Count -eq 0 -and $productChildren.Count -eq 0) {
            $software.DeleteSubKey('tools-screensaver-tzk', $false)
        }
    }
    finally {
        $software.Dispose()
    }
}

function Start-Dialog([string]$argument) {
    $start = [Diagnostics.ProcessStartInfo]::new()
    $start.FileName = (Resolve-Path -LiteralPath $Executable).Path
    $start.Arguments = $argument
    $start.UseShellExecute = $false
    $start.EnvironmentVariables[$testEnvironment] = $testKey
    $process = [Diagnostics.Process]::Start($start)
    $limit = [Diagnostics.Stopwatch]::StartNew()
    while ($limit.Elapsed -lt [TimeSpan]::FromSeconds(8)) {
        $process.Refresh()
        if ($process.MainWindowHandle -ne [IntPtr]::Zero) {
            return $process
        }
        Start-Sleep -Milliseconds 40
    }
    throw "Dialog did not appear for $argument"
}

function Save-Window([Diagnostics.Process]$process, [string]$name) {
    Start-Sleep -Milliseconds 500
    $process.Refresh()
    $rect = [Phase3CaptureNative+RECT]::new()
    if (-not [Phase3CaptureNative]::GetWindowRect($process.MainWindowHandle, [ref]$rect)) {
        throw "GetWindowRect failed for $name"
    }
    $width = $rect.Right - $rect.Left
    $height = $rect.Bottom - $rect.Top
    $bitmap = [Drawing.Bitmap]::new($width, $height, [Drawing.Imaging.PixelFormat]::Format32bppArgb)
    try {
        $graphics = [Drawing.Graphics]::FromImage($bitmap)
        try {
            $graphics.Clear([Drawing.Color]::Magenta)
            $dc = $graphics.GetHdc()
            try {
                if (-not [Phase3CaptureNative]::PrintWindow($process.MainWindowHandle, $dc, 2)) {
                    throw "PrintWindow failed for $name"
                }
            }
            finally {
                $graphics.ReleaseHdc($dc)
            }
        }
        finally {
            $graphics.Dispose()
        }
        $target = Join-Path $OutputDirectory $name
        $bitmap.Save($target, [Drawing.Imaging.ImageFormat]::Png)
        return (Resolve-Path -LiteralPath $target).Path
    }
    finally {
        $bitmap.Dispose()
    }
}

function Stop-Dialog([Diagnostics.Process]$process) {
    if (-not $process.HasExited) {
        [void]$process.CloseMainWindow()
        if (-not $process.WaitForExit(3000)) {
            $process.Kill()
            $process.WaitForExit()
        }
    }
    $process.Dispose()
}

New-Item -ItemType Directory -Force -Path $OutputDirectory | Out-Null
Remove-TestKey

try {
    $configuration = Start-Dialog '/c'
    try {
        $configurationPath = Save-Window $configuration 'config-dialog-win10-150.png'
    }
    finally {
        Stop-Dialog $configuration
    }

    $key = $registry.CreateSubKey($testKey)
    try {
        $kind = [Microsoft.Win32.RegistryValueKind]::DWord
        $key.SetValue('SchemaVersion', 2, $kind)
        $key.SetValue('DisplayMode', 1, $kind)
        $key.SetValue('ColorPreset', 2, $kind)
        $key.SetValue('FontMode', 0, $kind)
        $key.SetValue('LastCountdownDurationSeconds', 300, $kind)
    }
    finally {
        $key.Dispose()
    }

    $countdown = Start-Dialog '/s'
    try {
        $countdownPath = Save-Window $countdown 'countdown-dialog-win10-150.png'
    }
    finally {
        Stop-Dialog $countdown
    }

    [pscustomobject]@{
        Configuration = $configurationPath
        Countdown = $countdownPath
    } | ConvertTo-Json
}
finally {
    Remove-TestKey
}
