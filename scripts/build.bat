@echo off
setlocal EnableExtensions DisableDelayedExpansion

set "TOOLCHAIN=1.97.1-x86_64-pc-windows-msvc"
set "TARGET=x86_64-pc-windows-msvc"
for %%I in ("%~dp0..") do set "ROOT=%%~fI"
set "RELEASE_EXE=%ROOT%\target\%TARGET%\release\tools-screensaver-tzk.exe"
set "DIST_DIR=%ROOT%\dist"
set "DIST_SCR=%DIST_DIR%\tools-screensaver-tzk.scr"
set "DIST_TEMP=%DIST_DIR%\tools-screensaver-tzk.scr.%RANDOM%.%RANDOM%.tmp"

pushd "%ROOT%" || goto :fail

for %%T in (rustup.exe cargo.exe rustc.exe) do (
    where %%T >nul 2>nul
    if errorlevel 1 (
        echo [FAIL] Required tool was not found: %%T
        goto :fail
    )
)

powershell -NoProfile -Command "$items=@(& rustup toolchain list); if($LASTEXITCODE -ne 0 -or -not ($items | Where-Object { $_ -like ($env:TOOLCHAIN+'*') })){ exit 1 }"
if errorlevel 1 (
    echo [FAIL] Locked Rust toolchain is not installed: %TOOLCHAIN%
    goto :fail
)
powershell -NoProfile -Command "$items=@(& rustup target list --installed --toolchain $env:TOOLCHAIN); if($LASTEXITCODE -ne 0 -or $items -notcontains $env:TARGET){ exit 1 }"
if errorlevel 1 (
    echo [FAIL] Locked Rust target is not installed: %TARGET%
    goto :fail
)
for %%C in (rustfmt clippy) do (
    set "REQUIRED_COMPONENT=%%C"
    powershell -NoProfile -Command "$items=@(& rustup component list --installed --toolchain $env:TOOLCHAIN); if($LASTEXITCODE -ne 0 -or -not ($items | Where-Object { $_ -like ($env:REQUIRED_COMPONENT+'-*') })){ exit 1 }"
    if errorlevel 1 (
        echo [FAIL] Rust component is not installed for %TOOLCHAIN%: %%C
        goto :fail
    )
)

where rc.exe >nul 2>nul
if errorlevel 1 goto :load_msvc
where link.exe >nul 2>nul
if errorlevel 1 goto :load_msvc
goto :verify_msvc

:load_msvc
set "VSWHERE=%ProgramFiles(x86)%\Microsoft Visual Studio\Installer\vswhere.exe"
if not exist "%VSWHERE%" (
    echo [FAIL] vswhere.exe was not found. Open an x64 Visual Studio Developer Command Prompt.
    goto :fail
)
for /f "usebackq delims=" %%I in (`"%VSWHERE%" -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath`) do set "VSROOT=%%I"
if not defined VSROOT (
    echo [FAIL] MSVC x64 build tools were not found. Install them with Visual Studio Build Tools.
    goto :fail
)
if not exist "%VSROOT%\Common7\Tools\VsDevCmd.bat" (
    echo [FAIL] Official VsDevCmd.bat was not found below: %VSROOT%
    goto :fail
)
call "%VSROOT%\Common7\Tools\VsDevCmd.bat" -arch=amd64 -host_arch=amd64 >nul
if errorlevel 1 goto :fail

:verify_msvc
for /f "delims=" %%I in ('where rc.exe 2^>nul') do if not defined RC_EXE set "RC_EXE=%%I"
for /f "delims=" %%I in ('where link.exe 2^>nul') do if not defined LINK_EXE set "LINK_EXE=%%I"
if not defined RC_EXE (
    echo [FAIL] rc.exe was not found after loading the MSVC environment.
    goto :fail
)
if not defined LINK_EXE (
    echo [FAIL] link.exe was not found after loading the MSVC environment.
    goto :fail
)
powershell -NoProfile -Command "$p=$env:LINK_EXE; if($p.IndexOf('\VC\Tools\MSVC\',[StringComparison]::OrdinalIgnoreCase) -lt 0){ exit 1 }"
if errorlevel 1 (
    echo [FAIL] The first link.exe is not the Visual C++ linker: %LINK_EXE%
    goto :fail
)
powershell -NoProfile -Command "$p=$env:LINK_EXE; if(-not $p.EndsWith('\bin\Hostx64\x64\link.exe',[StringComparison]::OrdinalIgnoreCase)){ exit 1 }"
if errorlevel 1 (
    echo [FAIL] The Visual C++ linker is not the x64-hosted x64 linker: %LINK_EXE%
    goto :fail
)

echo [1/4] cargo fmt --check
cargo +%TOOLCHAIN% fmt --check
if errorlevel 1 goto :fail
echo [2/4] cargo clippy --locked --all-targets -- -D warnings
cargo +%TOOLCHAIN% clippy --locked --all-targets -- -D warnings
if errorlevel 1 goto :fail
echo [3/4] cargo test --locked
cargo +%TOOLCHAIN% test --locked
if errorlevel 1 goto :fail
echo [4/4] cargo build --release --locked
cargo +%TOOLCHAIN% build --release --locked
if errorlevel 1 goto :fail

if not exist "%RELEASE_EXE%" (
    echo [FAIL] Release artifact was not produced: %RELEASE_EXE%
    goto :fail
)
if not exist "%DIST_DIR%" mkdir "%DIST_DIR%"
if errorlevel 1 goto :fail
copy /B /Y "%RELEASE_EXE%" "%DIST_TEMP%" >nul
if errorlevel 1 goto :fail
move /Y "%DIST_TEMP%" "%DIST_SCR%" >nul
if errorlevel 1 goto :fail

set "ARTIFACT=%DIST_SCR%"
for /f "usebackq delims=" %%I in (`powershell -NoProfile -Command "[Diagnostics.FileVersionInfo]::GetVersionInfo($env:ARTIFACT).FileVersion"`) do set "PRODUCT_VERSION=%%I"
for /f "usebackq delims=" %%I in (`powershell -NoProfile -Command "(Get-Item -LiteralPath $env:ARTIFACT).Length"`) do set "ARTIFACT_BYTES=%%I"
for /f "usebackq delims=" %%I in (`powershell -NoProfile -Command "$sha=[Security.Cryptography.SHA256]::Create(); $stream=[IO.File]::OpenRead($env:ARTIFACT); try { ([BitConverter]::ToString($sha.ComputeHash($stream))).Replace('-','').ToLowerInvariant() } finally { $stream.Dispose(); $sha.Dispose() }"`) do set "ARTIFACT_SHA256=%%I"
if not defined PRODUCT_VERSION goto :fail
if not defined ARTIFACT_BYTES goto :fail
if not defined ARTIFACT_SHA256 goto :fail
for /f "delims=" %%I in ('git rev-parse --verify HEAD 2^>nul') do set "SOURCE_REVISION=%%I"
if not defined SOURCE_REVISION set "SOURCE_REVISION=unavailable (repository has no commit)"

echo.
echo [PASS] Phase 5 build completed.
echo Artifact: %DIST_SCR%
echo Version: %PRODUCT_VERSION%
echo Bytes: %ARTIFACT_BYTES%
echo SHA-256: %ARTIFACT_SHA256%
echo Rust:
rustc +%TOOLCHAIN% --version
cargo +%TOOLCHAIN% --version
echo RC: %RC_EXE%
echo Linker: %LINK_EXE%
echo Source revision: %SOURCE_REVISION%
popd
endlocal & exit /b 0

:fail
set "RESULT=%errorlevel%"
if "%RESULT%"=="0" set "RESULT=1"
if exist "%DIST_TEMP%" del /F /Q "%DIST_TEMP%" >nul 2>nul
echo.
echo [FAIL] The current Phase 5 build did not complete. Any existing dist artifact is from an earlier successful run.
popd >nul 2>nul
endlocal & exit /b %RESULT%
