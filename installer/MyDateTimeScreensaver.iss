#define MyAppName "MyDateTimeScreensaver"
#define MyAppDisplayName "日期時間螢幕保護程式"
#define MyAppPublisher "kisaraki"
#define MyAppScr AddBackslash(SourcePath) + "..\dist\MyDateTimeScreensaver.scr"
#define MyAppVersion GetFileVersion(MyAppScr)

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
UninstallDisplayIcon={sys}\MyDateTimeScreensaver.scr
DefaultDirName={autopf}\MyDateTimeScreensaver
OutputDir=..\dist
OutputBaseFilename=MyDateTimeScreensaver-Setup
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
Source: "{#MyAppScr}"; DestDir: "{sys}"; DestName: "MyDateTimeScreensaver.scr"; Flags: ignoreversion restartreplace uninsrestartdelete

[Tasks]
Name: "setcurrent"; Description: "將它設為目前的螢幕保護程式"; Flags: unchecked

[Code]
procedure RunSetCurrentHelper();
var
  ResultCode: Integer;
  Started: Boolean;
begin
  Log('setcurrent task selected; starting installed helper as original user');
  Started := ExecAsOriginalUser(
    ExpandConstant('{sys}\MyDateTimeScreensaver.scr'),
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
        '請登入自己的帳號後，在 Windows 的螢幕保護程式設定中選取「MyDateTimeScreensaver」。',
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
