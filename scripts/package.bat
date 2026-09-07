@echo off
setlocal EnableExtensions DisableDelayedExpansion

set "EXPECTED_ISCC_VERSION=6.7.3"
for %%I in ("%~dp0..") do set "ROOT=%%~fI"
set "ISS=%ROOT%\installer\MyDateTimeScreensaver.iss"
set "DIST=%ROOT%\dist"
set "SCR=%DIST%\tools-screensaver-tzk.scr"
set "SETUP=%DIST%\tools-screensaver-tzk-Setup.exe"
set "SETUP_TEMP=%DIST%\tools-screensaver-tzk-Setup.exe.%RANDOM%.%RANDOM%.tmp"
set "STAGE=%ROOT%\target\package-stage-%RANDOM%-%RANDOM%"
set "STAGED_SETUP=%STAGE%\tools-screensaver-tzk-Setup.exe"

pushd "%ROOT%" || goto :fail
call "%~dp0build.bat"
if errorlevel 1 goto :fail

set "ISCC=%LOCALAPPDATA%\Programs\Inno Setup 6\ISCC.exe"
if exist "%ISCC%" goto :verify_iscc
set "ISCC=%ProgramFiles(x86)%\Inno Setup 6\ISCC.exe"
if exist "%ISCC%" goto :verify_iscc
set "ISCC=%ProgramFiles%\Inno Setup 6\ISCC.exe"
if exist "%ISCC%" goto :verify_iscc
echo [FAIL] Inno Setup %EXPECTED_ISCC_VERSION% ISCC.exe was not found.
echo Install the exact version with: winget install --id JRSoftware.InnoSetup --exact --version %EXPECTED_ISCC_VERSION%
goto :fail

:verify_iscc
set "ISCC_EXPECTED=%EXPECTED_ISCC_VERSION%"
powershell -NoProfile -Command "$locations=@('HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*','HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*','HKLM:\Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\*'); $match=Get-ItemProperty $locations -ErrorAction SilentlyContinue | Where-Object { $_.DisplayName -like 'Inno Setup*' -and $_.DisplayVersion -eq $env:ISCC_EXPECTED }; if(-not $match){ Write-Error ('Required Inno Setup version is not registered: '+$env:ISCC_EXPECTED); exit 1 }"
if errorlevel 1 goto :fail
if not exist "%ISS%" (
    echo [FAIL] Inno source is missing: %ISS%
    goto :fail
)
mkdir "%STAGE%"
if errorlevel 1 goto :fail

echo [package] Inno Setup %EXPECTED_ISCC_VERSION%
"%ISCC%" /Qp /O"%STAGE%" "%ISS%"
if errorlevel 1 goto :fail
if not exist "%STAGED_SETUP%" (
    echo [FAIL] Inno did not produce the expected staged Setup: %STAGED_SETUP%
    goto :fail
)

set "STAGED_SETUP_PATH=%STAGED_SETUP%"
set "SCR_PATH=%SCR%"
powershell -NoProfile -Command "$scr=[Diagnostics.FileVersionInfo]::GetVersionInfo($env:SCR_PATH); $setup=[Diagnostics.FileVersionInfo]::GetVersionInfo($env:STAGED_SETUP_PATH); if(-not $scr.FileVersion -or -not $setup.FileVersion){ Write-Error 'Missing embedded file version'; exit 1 }; $a=([version]$scr.FileVersion.Trim()).ToString(3); $b=([version]$setup.FileVersion.Trim()).ToString(3); if($a -ne $b){ Write-Error ('Version mismatch: scr='+$a+', setup='+$b); exit 1 }; if($setup.ProductName.Trim() -ne 'MyDateTimeScreensaver'){ Write-Error ('Unexpected Setup ProductName: '+$setup.ProductName); exit 1 }"
if errorlevel 1 goto :fail

move /Y "%STAGED_SETUP%" "%SETUP_TEMP%" >nul
if errorlevel 1 goto :fail
move /Y "%SETUP_TEMP%" "%SETUP%" >nul
if errorlevel 1 goto :fail
rmdir "%STAGE%" >nul 2>nul

set "SETUP_PATH=%SETUP%"
set "SUMS=%DIST%\SHA256SUMS.txt"
powershell -NoProfile -Command "function Get-Sha256([string]$path){ $sha=[Security.Cryptography.SHA256]::Create(); $stream=[IO.File]::OpenRead($path); try { ([BitConverter]::ToString($sha.ComputeHash($stream))).Replace('-','').ToLowerInvariant() } finally { $stream.Dispose(); $sha.Dispose() } }; function Get-SignatureState([string]$path){ try { $cert=[Security.Cryptography.X509Certificates.X509Certificate]::CreateFromSignedFile($path); if($cert){ 'SignaturePresent (certificate chain not validated by package.bat)' } else { 'NotSigned' } } catch [Security.Cryptography.CryptographicException] { 'NotSigned' } }; $items=@($env:SCR_PATH,$env:SETUP_PATH); $lines=foreach($item in $items){ (Get-Sha256 $item)+'  '+[IO.Path]::GetFileName($item) }; [IO.File]::WriteAllLines($env:SUMS,$lines,[Text.Encoding]::ASCII); foreach($item in $items){ $info=[Diagnostics.FileVersionInfo]::GetVersionInfo($item); Write-Output ('Artifact: '+$item); Write-Output ('Version: '+$info.FileVersion); Write-Output ('Bytes: '+([IO.FileInfo]::new($item)).Length); Write-Output ('SHA-256: '+(Get-Sha256 $item)); Write-Output ('Signature: '+(Get-SignatureState $item)) }"
if errorlevel 1 goto :fail

echo SHA-256 list: %SUMS%
echo [PASS] Phase 5 package completed with Inno Setup %EXPECTED_ISCC_VERSION%.
popd
endlocal & exit /b 0

:fail
set "RESULT=%errorlevel%"
if "%RESULT%"=="0" set "RESULT=1"
if exist "%SETUP_TEMP%" del /F /Q "%SETUP_TEMP%" >nul 2>nul
echo.
echo [FAIL] The current Phase 5 package did not complete. Any existing Setup is from an earlier successful run.
if exist "%STAGE%" echo Staging retained for diagnosis: %STAGE%
popd >nul 2>nul
endlocal & exit /b %RESULT%
