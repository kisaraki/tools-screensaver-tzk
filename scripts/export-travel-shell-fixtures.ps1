param([string]$OutputDirectory = 'docs/evidence/phase14/html')
$ErrorActionPreference = 'Stop'
# Render only the product's local HTML/CSS with its player scripts removed.
# A new disposable browser profile and blocked host resolution avoid using the
# person's browser session. No WebView2, YouTube player or product UI is opened.
$repoRoot = Split-Path $PSScriptRoot
$output = [IO.Path]::GetFullPath((Join-Path $repoRoot $OutputDirectory))
$edge = Join-Path ${env:ProgramFiles(x86)} 'Microsoft/Edge/Application/msedge.exe'
if (!(Test-Path -LiteralPath $edge)) { throw 'Microsoft Edge is required for the optional headless HTML fixtures.' }
$null = New-Item -ItemType Directory -Path $output -Force
$work = Join-Path $repoRoot ('target/travel-shell-fixtures-' + [Guid]::NewGuid().ToString('N'))
$null = New-Item -ItemType Directory -Path $work
Copy-Item -LiteralPath (Join-Path $repoRoot 'assets/travel/free-flight.png'),(Join-Path $repoRoot 'assets/travel/train-journey.png'),(Join-Path $repoRoot 'assets/travel/japanese-inn.png'),(Join-Path $repoRoot 'assets/travel/train-cab.png'),(Join-Path $repoRoot 'assets/travel/walking.png') -Destination $work
$source = [IO.File]::ReadAllText((Join-Path $repoRoot 'src/travel.rs'))
$template = [regex]::Match($source,'(?s)const TRAVEL_HTML_TEMPLATE: &str = r#"(.*?)"#;').Groups[1].Value
if (!$template) { throw 'Travel HTML template not found' }
$template = [regex]::Replace($template,'(?s)<script>.*?</script>','')
$validation = @'
<script>
window.addEventListener('load', () => {
  const rect = selector => {
    const r = document.querySelector(selector).getBoundingClientRect();
    return {x:r.x,y:r.y,w:r.width,h:r.height,right:r.right,bottom:r.bottom};
  };
  const art=rect('.scene'), windowRect=rect('.window'), player=rect('.screen'), caption=rect('.caption');
  const contains=(a,b)=>b.x>=a.x-1&&b.y>=a.y-1&&b.right<=a.right+1&&b.bottom<=a.bottom+1;
  const walking=document.querySelector('.cabin').classList.contains('walking');
  const walkingMargins=!walking||(windowRect.w/art.w<=.61&&windowRect.h/art.h<=.55);
  const pass=contains(art,windowRect)&&contains(windowRect,player)&&caption.y>=art.bottom-.1
    &&Math.abs(player.w/player.h-16/9)<.005&&caption.bottom<=innerHeight+1&&walkingMargins;
  document.body.dataset.geometry=JSON.stringify({pass,viewport:[innerWidth,innerHeight],art,window:windowRect,player,caption});
  const marker=document.createElement('div');
  marker.style.cssText='position:fixed;left:0;top:0;width:2px;height:2px;z-index:99999;background:'+(pass?'rgb(0,255,0)':'rgb(255,0,0)');
  document.body.appendChild(marker);
});
</script>
'@
Add-Type -AssemblyName System.Drawing
$reports = @()
foreach ($style in @('free-flight','train-journey','japanese-inn','train-cab','walking')) {
    $label = switch ($style) {
        'free-flight' { 'Free flight realistic frame' }
        'train-journey' { 'Train journey realistic frame' }
        'japanese-inn' { 'Japanese inn realistic frame' }
        'train-cab' { 'Train driver forward realistic frame' }
        default { 'First person human eye walking frame' }
    }
    $html = $template.Replace('__TRAVEL_SCENE_CLASS__',$style).Replace('__TRAVEL_SCENE_LABEL__',$label)
    $html = [regex]::Replace($html,'<span id="place">.*?</span>',"<span id=`"place`">$label</span>")
    $html = [regex]::Replace($html,'<span id="status">.*?</span>','<span id="status">Offline layout validation</span>')
    $html = $html.Replace('<div id="player"></div>','<div id="player" style="background:#172d3d;display:grid;place-items:center;font-size:clamp(12px,1.3vw,22px)">Full 16:9 player area</div>')
    $html = $html.Replace('</body>',$validation + '</body>')
    $page = Join-Path $work "$style.html"
    [IO.File]::WriteAllText($page,$html,[Text.UTF8Encoding]::new($false))
    foreach ($size in @('1600,1000','900,1600')) {
        $labelSize = $size.Replace(',','x')
        $screenshot = Join-Path $work "$style-$labelSize.png"
        $dom = Join-Path $work "$style-$labelSize-dom.txt"
        $profile = Join-Path $work "profile-$style-$labelSize"
        $arguments = @('--headless=new','--disable-gpu','--no-first-run','--no-default-browser-check',
            '--disable-background-networking','--disable-component-update','--disable-sync','--hide-scrollbars',
            '--host-resolver-rules="MAP * ~NOTFOUND"',"--user-data-dir=`"$profile`"",'--force-device-scale-factor=1',
            "--window-size=$size",'--virtual-time-budget=1500',"--screenshot=`"$screenshot`"",([Uri]$page).AbsoluteUri)
        $process = Start-Process -FilePath $edge -ArgumentList $arguments -WindowStyle Hidden -PassThru -RedirectStandardOutput $dom -RedirectStandardError (Join-Path $work "$style-$labelSize-stderr.txt")
        if (!$process.WaitForExit(45000)) { throw 'Headless HTML fixture timed out; inspect the isolated profile process.' }
        # Edge's launcher may return before its isolated headless child finishes.
        $deadline = [DateTime]::UtcNow.AddSeconds(30)
        while ([DateTime]::UtcNow -lt $deadline) {
            if ((Test-Path -LiteralPath $screenshot) -and (Get-Item -LiteralPath $screenshot).Length -gt 1000) { break }
            Start-Sleep -Milliseconds 250
        }
        if (!(Test-Path -LiteralPath $screenshot)) { throw "Screenshot missing: $style-$labelSize" }
        $bitmap = [Drawing.Bitmap]::FromFile($screenshot)
        try { $marker = $bitmap.GetPixel(0,0) } finally { $bitmap.Dispose() }
        if ($marker.R -ne 0 -or $marker.G -ne 255 -or $marker.B -ne 0) { throw "HTML layout bounds failed or validation script missing: $style-$labelSize" }
        Copy-Item -LiteralPath $screenshot -Destination (Join-Path $output "$style-$labelSize.png")
        $reports += [pscustomobject]@{style=$style;size=$labelSize;geometry='PASS: player 16:9 inside window; window inside artwork; caption below artwork and inside viewport';validationMarker='2px green at top left';livePlayer=$false;networkHostsBlocked=$true}
    }
}
$report = [pscustomobject]@{capturedAt=[DateTimeOffset]::Now.ToString('o');browserVersion=(Get-Item -LiteralPath $edge).VersionInfo.FileVersion;interactive=$false;livePlayer=$false;result='PASS';fixtures=$reports}
$report | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath (Join-Path $output 'geometry.json') -Encoding utf8
'PASS: ten offline headless HTML screenshots; player rectangles remain 16:9, walking view stays central, and captions stay outside the artwork.'
