#define MyAppName "tools-screensaver-tzk"
#define MyAppDisplayName "tools-screensaver-tzk"
#define MyAppPublisher "kisaraki"
#define MyAppScr AddBackslash(SourcePath) + "..\dist\tools-screensaver-tzk.scr"
#define MyAppVersion GetFileVersion(MyAppScr)
#define MyAppUninstallKey "SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\{E4D6978B-A2A2-4D9A-8FD8-8F0C3A4E94E1}_is1"
#define WebView2Bootstrapper AddBackslash(SourcePath) + "..\target\webview2\MicrosoftEdgeWebview2Setup.exe"
#include "..\target\webview2\verified-bootstrapper.iss"
#if GetSHA256OfFile(WebView2Bootstrapper) != WebView2BootstrapperSHA256
  #error WebView2 bootstrapper changed after verification. Run scripts/prepare-webview2.ps1.
#endif

[Setup]
AppId={{E4D6978B-A2A2-4D9A-8FD8-8F0C3A4E94E1}
AppName={#MyAppDisplayName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
VersionInfoVersion={#MyAppVersion}
VersionInfoProductVersion={#MyAppVersion}
VersionInfoProductName={#MyAppName}
VersionInfoDescription={#MyAppDisplayName} 安裝程式
VersionInfoCompany={#MyAppPublisher}
VersionInfoCopyright=Copyright (c) 2026 kisaraki
UninstallDisplayName={#MyAppDisplayName}
UninstallDisplayIcon={sys}\tools-screensaver-tzk.scr
DefaultDirName={autopf}\tools-screensaver-tzk
OutputDir=..\dist
OutputBaseFilename=tools-screensaver-tzk-Setup
ArchitecturesAllowed=x64os
ArchitecturesInstallIn64BitMode=x64os
PrivilegesRequired=admin
MinVersion=10.0.15063
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
DisableProgramGroupPage=yes
CloseApplications=yes
RestartApplications=no
SetupLogging=yes
UsePreviousTasks=no
LicenseFile=..\LICENSE

[Files]
Source: "{#MyAppScr}"; DestDir: "{sys}"; DestName: "tools-screensaver-tzk.scr"; Flags: ignoreversion restartreplace uninsrestartdelete
Source: "{#WebView2Bootstrapper}"; Flags: dontcopy

[Tasks]
Name: "webview2"; Description: "檢查並安裝 Microsoft Edge WebView2 Runtime（日本旅行模式需要；缺少時連網下載）"; Check: NeedsWebView2
Name: "setcurrent"; Description: "將它設為目前的螢幕保護程式"; Flags: unchecked

[Code]
#include "webview2-policy.iss"
#include "product-version-policy.iss"

var
  PreviousVersionNeedsRemoval: Boolean;
  PreviousVersion: String;
  PreviousUninstaller: String;

function InitializeSetup(): Boolean;
var
  Installed, SameVersion, RemovalAccepted: Boolean;
  Action: TProductInstallAction;
begin
  Result := True;
  Installed := DetectInstalledProduct('{#MyAppUninstallKey}',
    PreviousVersion, PreviousUninstaller);
  if not Installed then
  begin
    Log('No installed tools-screensaver-tzk version detected.');
    Exit;
  end;

  SameVersion := ProductVersionsEqual(PreviousVersion, '{#MyAppVersion}');
  if SameVersion then
  begin
    Log('Installed version matches Setup; repair installation may continue: ' +
      PreviousVersion);
    Exit;
  end;

  if WizardSilent then
  begin
    Log('Silent installation blocked by a different installed version: ' +
      PreviousVersion + '; Setup=' + '{#MyAppVersion}');
    Result := False;
    Exit;
  end;

  if (PreviousUninstaller = '') or (not FileExists(PreviousUninstaller)) then
  begin
    MsgBox(
      '偵測到不同版本的 tools-screensaver-tzk，但找不到其解除安裝程式。' + #13#10 +
      '已安裝版本：' + PreviousVersion + #13#10 +
      '準備安裝版本：{#MyAppVersion}' + #13#10#13#10 +
      '請先從 Windows「應用程式與功能」移除舊版，再重新執行本安裝程式。',
      mbError, MB_OK);
    Result := False;
    Exit;
  end;

  RemovalAccepted := MsgBox(
    '系統中已有不同版本的 tools-screensaver-tzk。' + #13#10 +
    '已安裝版本：' + PreviousVersion + #13#10 +
    '準備安裝版本：{#MyAppVersion}' + #13#10#13#10 +
    '要先移除已安裝版本，再安裝本版嗎？' + #13#10 +
    '個人模式與外觀設定將予以保留。',
    mbConfirmation, MB_YESNO) = IDYES;
  Action := ProductInstallAction(True, False, False, RemovalAccepted);
  case Action of
    piaRemovePrevious: PreviousVersionNeedsRemoval := True;
    piaCancel: Result := False;
  end;
end;

function RemovePreviousProduct(var NeedsRestart: Boolean): String;
var
  Started, StillInstalled: Boolean;
  ExitCode: Integer;
  RemainingVersion, RemainingUninstaller: String;
begin
  Result := '';
  Log('Removing installed tools-screensaver-tzk version ' + PreviousVersion +
    ' before installing {#MyAppVersion}.');
  Started := Exec(PreviousUninstaller,
    '/VERYSILENT /SUPPRESSMSGBOXES /NORESTART', '', SW_HIDE,
    ewWaitUntilTerminated, ExitCode);
  if not Started then
  begin
    Result := '無法啟動舊版解除安裝程式（Win32 錯誤 ' +
      IntToStr(ExitCode) + '）。安裝已停止，請手動移除舊版後重試。';
    Exit;
  end;
  if (ExitCode = 3010) or (ExitCode = 1641) then
    NeedsRestart := True
  else if ExitCode <> 0 then
  begin
    Result := '舊版解除安裝失敗（代碼 ' + IntToStr(ExitCode) +
      '）。安裝已停止，系統不會同時保留兩個版本。';
    Exit;
  end;

  StillInstalled := DetectInstalledProduct('{#MyAppUninstallKey}',
    RemainingVersion, RemainingUninstaller);
  if StillInstalled then
  begin
    Result := '解除安裝後仍偵測到 tools-screensaver-tzk ' +
      RemainingVersion + '。安裝已停止，請重新啟動 Windows 或手動移除舊版後重試。';
    Exit;
  end;
  PreviousVersionNeedsRemoval := False;
  Log('Previous tools-screensaver-tzk version removed successfully.');
end;

function NeedsWebView2(): Boolean;
var
  Version: String;
begin
  Result := not DetectMachineWebView2(Version);
end;

function UpdateReadyMemo(Space, NewLine, MemoUserInfoInfo, MemoDirInfo,
  MemoTypeInfo, MemoComponentsInfo, MemoGroupInfo, MemoTasksInfo: String): String;
var
  Version, RuntimeStatus: String;
begin
  if DetectMachineWebView2(Version) then
    RuntimeStatus := '已偵測到電腦層級 WebView2 Runtime ' + Version + '，略過安裝。'
  else if WizardIsTaskSelected('webview2') then
    RuntimeStatus := '未偵測到電腦層級 WebView2 Runtime。將使用 Microsoft 官方安裝引導程式連網下載並靜默安裝，供所有使用者共用。'
  else
    RuntimeStatus := '已略過 WebView2 安裝；日期時鐘與番茄鐘可正常使用。日本旅行模式需要另行安裝 Runtime。';
  Result := MemoDirInfo + NewLine + NewLine + MemoTasksInfo + NewLine +
    NewLine + 'Microsoft Edge WebView2 Runtime:' + NewLine + Space + RuntimeStatus;
end;

function PrepareToInstall(var NeedsRestart: Boolean): String;
var
  Version, BootstrapperPath: String;
  ExitCode: Integer;
  Installed, Started: Boolean;
begin
  Result := '';
  if PreviousVersionNeedsRemoval then
  begin
    Result := RemovePreviousProduct(NeedsRestart);
    if Result <> '' then
      Exit;
  end;
  Installed := DetectMachineWebView2(Version);
  if not WebView2ShouldInstall(Installed, WizardIsTaskSelected('webview2')) then
  begin
    Log('WebView2 prerequisite skipped: installed=' + IntToStr(Ord(Installed)) +
      ', version=' + Version);
    Exit;
  end;

  WizardForm.PreparingLabel.Caption :=
    '正在下載並安裝 Microsoft Edge WebView2 Runtime，請保持網路連線。';
  try
    ExtractTemporaryFile('MicrosoftEdgeWebview2Setup.exe');
    BootstrapperPath := ExpandConstant('{tmp}\MicrosoftEdgeWebview2Setup.exe');
    if not SameText(GetSHA256OfFile(BootstrapperPath), '{#WebView2BootstrapperSHA256}') then
      RaiseException('Microsoft WebView2 安裝引導程式完整性檢查失敗。');
    // Inherits the existing Setup elevation. No second runas/UAC request.
    Started := Exec(BootstrapperPath, '/silent /install', '', SW_HIDE,
      ewWaitUntilTerminated, ExitCode);
    Installed := DetectMachineWebView2(Version);
    Log('WebView2 bootstrapper exit=' + IntToStr(ExitCode) + ', version=' + Version);
    case WebView2InstallOutcome(Started, ExitCode, Installed) of
      wvReady: Log('WebView2 machine Runtime verified after installation.');
      wvRestartRequired:
        begin
          NeedsRestart := True;
          Result := 'WebView2 Runtime 要求重新啟動 Windows。請重新啟動後再執行本安裝程式。';
        end;
      wvFailed:
        Result := 'WebView2 Runtime 尚未安裝成功（代碼 ' + IntToStr(ExitCode) +
          '）。請檢查網路或系統管理原則後重試；也可返回取消 WebView2 選項，先安裝離線時鐘模式。';
    end;
  except
    Result := GetExceptionMessage + #13#10 +
      '請重試；或返回取消 WebView2 選項，先安裝離線時鐘模式。';
    Log('WebView2 prerequisite failed: ' + Result);
  end;
end;

procedure RunSetCurrentHelper();
var
  ResultCode: Integer;
  Started: Boolean;
begin
  Log('setcurrent task selected; starting installed helper as original user');
  Started := ExecAsOriginalUser(
    ExpandConstant('{sys}\tools-screensaver-tzk.scr'),
    '--install-set-current', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
  if Started and (ResultCode = 0) then
  begin
    Log('setcurrent helper completed successfully');
  end
  else
  begin
    if Started then
      Log(Format('setcurrent helper refused or failed with exit code %d', [ResultCode]))
    else
      Log(Format('setcurrent helper could not be started; Win32 error %d', [ResultCode]));
    if not WizardSilent then
      MsgBox(
        '安裝已完成，但尚未設為目前的螢幕保護程式。' + #13#10 +
        '請登入自己的帳號後，在 Windows 的螢幕保護程式設定中選取「tools-screensaver-tzk」。',
        mbInformation, MB_OK);
  end;
end;

procedure CurStepChanged(CurStep: TSetupStep);
begin
  if (CurStep = ssPostInstall) and WizardIsTaskSelected('setcurrent') then
    RunSetCurrentHelper();
end;

function InitializeUninstall(): Boolean;
begin
  Result := True;
  Log('Uninstall preserves per-user preferences and does not change HKCU SCRNSAVE.EXE.');
  if not UninstallSilent then
    MsgBox(
      '解除安裝不會自動變更任何帳號目前選用的螢幕保護程式。' + #13#10 +
      '若目前仍選用本程式，解除安裝後請到 Windows 螢幕保護程式設定改選其他項目或「無」。',
      mbInformation, MB_OK);
end;
