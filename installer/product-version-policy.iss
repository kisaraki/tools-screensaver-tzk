// Shared by the real installer and a non-elevated, no-wizard policy test.
type
  TProductInstallAction = (
    piaProceed,
    piaRemovePrevious,
    piaCancel,
    piaBlockSilent
  );

function ProductVersionsEqual(const InstalledVersion,
  CurrentVersion: String): Boolean;
var
  InstalledPacked, CurrentPacked: Int64;
begin
  Result := SameText(Trim(InstalledVersion), Trim(CurrentVersion));
  if Result then
    Exit;
  if StrToVersion(Trim(InstalledVersion), InstalledPacked) and
     StrToVersion(Trim(CurrentVersion), CurrentPacked) then
    Result := ComparePackedVersion(InstalledPacked, CurrentPacked) = 0;
end;

function ProductInstallAction(Installed, SameVersion, SilentMode,
  RemovalAccepted: Boolean): TProductInstallAction;
begin
  Result := piaProceed;
  if (not Installed) or SameVersion then
    Exit;
  if SilentMode then
    Result := piaBlockSilent
  else if RemovalAccepted then
    Result := piaRemovePrevious
  else
    Result := piaCancel;
end;

function UninstallExecutableFromCommand(const CommandLine: String): String;
var
  ClosingQuote, FirstSpace: Integer;
  Value: String;
begin
  Result := '';
  Value := Trim(CommandLine);
  if Value = '' then
    Exit;
  if Value[1] = '"' then
  begin
    Delete(Value, 1, 1);
    ClosingQuote := Pos('"', Value);
    if ClosingQuote > 0 then
      Result := Copy(Value, 1, ClosingQuote - 1);
  end
  else
  begin
    FirstSpace := Pos(' ', Value);
    if FirstSpace > 0 then
      Result := Copy(Value, 1, FirstSpace - 1)
    else
      Result := Value;
  end;
end;

function DetectInstalledProduct(const UninstallKey: String;
  var Version, Uninstaller: String): Boolean;
var
  CommandLine: String;
begin
  Version := '';
  Uninstaller := '';
  Result := RegKeyExists(HKLM64, UninstallKey);
  if Result then
  begin
    RegQueryStringValue(HKLM64, UninstallKey, 'DisplayVersion', Version);
    if RegQueryStringValue(HKLM64, UninstallKey, 'UninstallString', CommandLine) then
      Uninstaller := UninstallExecutableFromCommand(CommandLine);
  end
  else
  begin
    Result := RegKeyExists(HKLM32, UninstallKey);
    if Result then
    begin
      RegQueryStringValue(HKLM32, UninstallKey, 'DisplayVersion', Version);
      if RegQueryStringValue(HKLM32, UninstallKey, 'UninstallString', CommandLine) then
        Uninstaller := UninstallExecutableFromCommand(CommandLine);
    end;
  end;
  if Result and (Trim(Version) = '') then
    Version := '未知版本';
end;
