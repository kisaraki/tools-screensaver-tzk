[Setup]
AppName=tools-screensaver-tzk WebView2 policy tests
AppVersion=1.0
PrivilegesRequired=lowest
CreateAppDir=no
Uninstallable=no
OutputBaseFilename=webview2-policy-tests
SetupLogging=no

[Code]
#include "webview2-policy.iss"

var
  Passed: Integer;

procedure Check(Value: Boolean; const Name: String);
begin
  if not Value then
    RaiseException('FAIL: ' + Name);
  Passed := Passed + 1;
end;

function InitializeSetup(): Boolean;
var
  Report, Version, ResultText: String;
  MachinePresent: Boolean;
begin
  // Runs before the wizard exists. No payload, Exec, network or registry writes.
  Result := False;
  Report := ExpandConstant('{param:REPORT|}');
  if Report = '' then
    Exit;
  try
    Check(not WebView2VersionPresent(''), 'missing pv');
    Check(not WebView2VersionPresent('0.0.0.0'), 'uninstalled pv');
    Check(not WebView2VersionPresent('invalid'), 'malformed pv');
    Check(not WebView2VersionPresent('-1.2.3.4'), 'negative version');
    Check(not WebView2VersionPresent('999999.0.0.0'), 'overflow version');
    Check(WebView2VersionPresent('152.0.4191.66'), 'current version');
    Check(WebView2VersionPresent('1.0.0.0'), 'positive version');
    Check(WebView2VersionPresent(' 152.0.4191.66 '), 'trim version');
    Check(not WebView2ShouldInstall(True, True), 'installed skip');
    Check(not WebView2ShouldInstall(True, False), 'installed unselected');
    Check(WebView2ShouldInstall(False, True), 'missing selected install');
    Check(not WebView2ShouldInstall(False, False), 'offline opt out');
    Check(WebView2InstallOutcome(True, 0, True) = wvReady, 'verified success');
    Check(WebView2InstallOutcome(True, 0, False) = wvFailed, 'zero without Runtime rejected');
    Check(WebView2InstallOutcome(False, 5, False) = wvFailed, 'launch failure');
    Check(WebView2InstallOutcome(True, 1603, False) = wvFailed, 'installer failure');
    Check(WebView2InstallOutcome(True, 1603, True) = wvFailed, 'nonzero still rejected');
    Check(WebView2InstallOutcome(True, 3010, True) = wvRestartRequired, 'restart required');
    Check(WebView2InstallOutcome(True, 1641, False) = wvRestartRequired, 'restart initiated');
    MachinePresent := DetectMachineWebView2(Version);
    ResultText := 'PASS: ' + IntToStr(Passed) + ' policy checks' + #13#10 +
      'machineRuntimePresent=' + IntToStr(Ord(MachinePresent)) + #13#10 +
      'machineRuntimeVersion=' + Version + #13#10 +
      'interactive=false; runtimeExecuted=false; registryWrites=false' + #13#10;
  except
    ResultText := GetExceptionMessage;
  end;
  SaveStringToFile(Report, ResultText, False);
end;
