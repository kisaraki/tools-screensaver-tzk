// Shared by the real installer and a non-elevated, no-wizard policy test.
type
  TWebView2Outcome = (wvReady, wvFailed, wvRestartRequired);

function WebView2VersionPresent(const Value: String): Boolean;
var
  Version: Int64;
begin
  Result := False;
  if StrToVersion(Trim(Value), Version) then
    Result := ComparePackedVersion(Version, PackVersionComponents(0, 0, 0, 0)) > 0;
end;

function DetectMachineWebView2(var Version: String): Boolean;
begin
  Version := '';
  // Setup installs into System32 for all users. Do not mistake the elevated
  // administrator's HKCU Runtime for one available to the original user.
  Result := RegQueryStringValue(HKLM32,
    'SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}',
    'pv', Version);
  if Result then
    Result := WebView2VersionPresent(Version);
end;

function WebView2ShouldInstall(Installed, Requested: Boolean): Boolean;
begin
  Result := (not Installed) and Requested;
end;

function WebView2InstallOutcome(Started: Boolean; ExitCode: Integer;
  InstalledAfter: Boolean): TWebView2Outcome;
begin
  Result := wvFailed;
  if not Started then
    Exit;
  if (ExitCode = 3010) or (ExitCode = 1641) then
    Result := wvRestartRequired
  else if (ExitCode = 0) and InstalledAfter then
    Result := wvReady;
end;
