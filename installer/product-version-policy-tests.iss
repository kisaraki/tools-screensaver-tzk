[Setup]
AppName=tools-screensaver-tzk product version policy tests
AppVersion=1.0
PrivilegesRequired=lowest
CreateAppDir=no
Uninstallable=no
OutputBaseFilename=product-version-policy-tests
SetupLogging=no

[Code]
#include "product-version-policy.iss"

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
  Report, ResultText, InstalledVersion, Uninstaller: String;
  MachinePresent: Boolean;
begin
  // Runs before the wizard exists. No payload, process launch, prompt or writes.
  Result := False;
  Report := ExpandConstant('{param:REPORT|}');
  if Report = '' then
    Exit;
  try
    Check(ProductVersionsEqual('0.12.1.0', '0.12.1.0'), 'exact version');
    Check(ProductVersionsEqual('0.12.1', '0.12.1.0'), 'equivalent version');
    Check(ProductVersionsEqual(' 0.12.1.0 ', '0.12.1'), 'trim version');
    Check(not ProductVersionsEqual('0.12.0.0', '0.12.1.0'), 'older version');
    Check(not ProductVersionsEqual('0.13.0.0', '0.12.1.0'), 'newer version');
    Check(ProductVersionsEqual('unknown', ' unknown '), 'same malformed label');
    Check(not ProductVersionsEqual('unknown', '0.12.1.0'), 'malformed mismatch');
    Check(ProductInstallAction(False, False, False, False) = piaProceed,
      'not installed');
    Check(ProductInstallAction(True, True, False, False) = piaProceed,
      'same version repair');
    Check(ProductInstallAction(True, False, False, True) = piaRemovePrevious,
      'different version accepted');
    Check(ProductInstallAction(True, False, False, False) = piaCancel,
      'different version rejected');
    Check(ProductInstallAction(True, False, True, True) = piaBlockSilent,
      'silent version conflict blocked');
    Check(UninstallExecutableFromCommand(
      '"C:\Program Files\tools-screensaver-tzk\unins000.exe"') =
      'C:\Program Files\tools-screensaver-tzk\unins000.exe', 'quoted command');
    Check(UninstallExecutableFromCommand('C:\unins000.exe /SILENT') =
      'C:\unins000.exe', 'unquoted command');
    Check(UninstallExecutableFromCommand('') = '', 'missing command');
    MachinePresent := DetectInstalledProduct(
      'SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\' +
      '{E4D6978B-A2A2-4D9A-8FD8-8F0C3A4E94E1}_is1',
      InstalledVersion, Uninstaller);
    ResultText := 'PASS: ' + IntToStr(Passed) + ' product version policy checks' + #13#10 +
      'machineProductPresent=' + IntToStr(Ord(MachinePresent)) + #13#10 +
      'machineProductVersion=' + InstalledVersion + #13#10 +
      'interactive=false; processExecuted=false; prompts=false; registryWrites=false' + #13#10;
  except
    ResultText := GetExceptionMessage;
  end;
  SaveStringToFile(Report, ResultText, False);
end;
