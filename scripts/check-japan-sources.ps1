[CmdletBinding()]
param(
    [string]$OutputPath,
    [ValidateRange(3, 30)]
    [int]$TimeoutSeconds = 10
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'

if ([string]::IsNullOrWhiteSpace($OutputPath)) {
    $OutputPath = Join-Path $PSScriptRoot '..\docs\evidence\phase6\source-health.json'
}

$cameras = @(
    [ordered]@{ id = 'sapporostationhbc'; place = '北海道・札幌' },
    [ordered]@{ id = 'okutamastationview'; place = '東京・奧多摩' },
    [ordered]@{ id = 'jpkyotokarasumadorinakagyo'; place = '京都・中京區' },
    [ordered]@{ id = 'osakahanatencam'; place = '大阪・JR 放出車站' },
    [ordered]@{ id = 'miyajimacamera'; place = '廣島・嚴島宮島' },
    [ordered]@{ id = 'kariyushibeachnago'; place = '沖繩・名護嘉利吉海灘' },
    [ordered]@{ id = 'sakurajimatarumizu'; place = '鹿兒島・垂水櫻島' },
    [ordered]@{ id = 'jpkamikochitaishoilakealps'; place = '長野・上高地大正池' }
)

$playlists = @(
    [ordered]@{ style = 'FreeFlight'; label = '自在飛行'; id = 'PLdsqwBj2O1Nw' },
    [ordered]@{ style = 'TrainJourney'; label = '列車旅行'; id = 'PLBH60D9AGfu0' },
    [ordered]@{ style = 'TrainCab'; label = '列車駕駛前方'; id = 'PLB-Fmt68BNm4' },
    [ordered]@{ style = 'Walking'; label = '散步模式'; id = 'PLbYZr39owNGo' }
)

function Invoke-BoundedGet {
    param([Parameter(Mandatory)][string]$Uri)

    $response = Invoke-WebRequest -UseBasicParsing -Uri $Uri -TimeoutSec $TimeoutSeconds -MaximumRedirection 3
    $baseResponse = $response.BaseResponse
    $finalUri = if ($baseResponse.PSObject.Properties['RequestMessage'] -and
        $baseResponse.RequestMessage.RequestUri) {
        $response.BaseResponse.RequestMessage.RequestUri.AbsoluteUri
    }
    elseif ($baseResponse.PSObject.Properties['ResponseUri'] -and $baseResponse.ResponseUri) {
        $baseResponse.ResponseUri.AbsoluteUri
    }
    else {
        $Uri
    }
    [pscustomobject]@{
        StatusCode = [int]$response.StatusCode
        Content = [string]$response.Content
        FinalUri = $finalUri
    }
}

$checkedAt = [DateTimeOffset]::Now
$catalog = [ordered]@{ url = 'https://tw.live/japan/'; ok = $false; status = 0; elapsedMs = 0; error = $null }
$catalogTimer = [Diagnostics.Stopwatch]::StartNew()
try {
    $response = Invoke-BoundedGet -Uri $catalog.url
    $catalog.status = $response.StatusCode
    $catalog.ok = $response.StatusCode -eq 200 -and
        $response.FinalUri.StartsWith('https://tw.live/', [StringComparison]::OrdinalIgnoreCase) -and
        $response.Content.Contains('href="/japan/') -and
        $response.Content.Contains('YouTube')
}
catch {
    $catalog.error = $_.Exception.Message
}
finally {
    $catalogTimer.Stop()
    $catalog.elapsedMs = $catalogTimer.ElapsedMilliseconds
}

$results = foreach ($camera in $cameras) {
    $timer = [Diagnostics.Stopwatch]::StartNew()
    $item = [ordered]@{
        cameraId = $camera.id
        place = $camera.place
        twLiveUrl = "https://tw.live/cam/?id=$($camera.id)"
        twLiveStatus = 0
        youtubeId = $null
        oEmbedStatus = 0
        thumbnailStatus = 0
        reachable = $false
        elapsedMs = 0
        error = $null
    }
    try {
        $detail = Invoke-BoundedGet -Uri $item.twLiveUrl
        $item.twLiveStatus = $detail.StatusCode
        if (-not $detail.FinalUri.StartsWith('https://tw.live/', [StringComparison]::OrdinalIgnoreCase)) {
            throw "Unexpected redirect target: $($detail.FinalUri)"
        }
        $match = [regex]::Match(
            $detail.Content,
            'https://(?:www\.)?youtube(?:-nocookie)?\.com/embed/([A-Za-z0-9_-]{11})',
            [Text.RegularExpressions.RegexOptions]::IgnoreCase
        )
        if (-not $match.Success) {
            throw 'No valid YouTube embed ID was found in the tw.live camera page.'
        }
        $item.youtubeId = $match.Groups[1].Value
        $watchUrl = "https://www.youtube.com/watch?v=$($item.youtubeId)"
        $oEmbedUrl = 'https://www.youtube.com/oembed?format=json&url=' + [Uri]::EscapeDataString($watchUrl)
        $oEmbed = Invoke-BoundedGet -Uri $oEmbedUrl
        $item.oEmbedStatus = $oEmbed.StatusCode
        $thumbnail = Invoke-BoundedGet -Uri "https://i.ytimg.com/vi/$($item.youtubeId)/hqdefault.jpg"
        $item.thumbnailStatus = $thumbnail.StatusCode
        $item.reachable = $item.twLiveStatus -eq 200 -and
            $item.oEmbedStatus -eq 200 -and $item.thumbnailStatus -eq 200
    }
    catch {
        $item.error = $_.Exception.Message
    }
    finally {
        $timer.Stop()
        $item.elapsedMs = $timer.ElapsedMilliseconds
    }
    [pscustomobject]$item
}

$playlistResults = foreach ($playlist in $playlists) {
    $timer = [Diagnostics.Stopwatch]::StartNew()
    $item = [ordered]@{
        style = $playlist.style
        label = $playlist.label
        playlistId = $playlist.id
        embedUrl = "https://www.youtube.com/embed/videoseries?list=$($playlist.id)"
        status = 0
        reachable = $false
        elapsedMs = 0
        error = $null
    }
    try {
        $response = Invoke-BoundedGet -Uri $item.embedUrl
        $item.status = $response.StatusCode
        $item.reachable = $response.StatusCode -eq 200 -and
            $response.FinalUri.StartsWith('https://www.youtube.com/', [StringComparison]::OrdinalIgnoreCase)
    }
    catch {
        $item.error = $_.Exception.Message
    }
    finally {
        $timer.Stop()
        $item.elapsedMs = $timer.ElapsedMilliseconds
    }
    [pscustomobject]$item
}

$healthyCount = @($results | Where-Object reachable).Count
$healthyPlaylistCount = @($playlistResults | Where-Object reachable).Count
$report = [ordered]@{
    schemaVersion = 2
    checkedAt = $checkedAt.ToString('o')
    checkType = 'noninteractive HTTP reachability; does not claim video PLAYING'
    timeoutSecondsPerRequest = $TimeoutSeconds
    catalog = [pscustomobject]$catalog
    summary = [ordered]@{
        cameraTotal = $results.Count
        cameraReachable = $healthyCount
        playlistTotal = $playlistResults.Count
        playlistReachable = $healthyPlaylistCount
        unavailable = ($results.Count - $healthyCount) + ($playlistResults.Count - $healthyPlaylistCount)
        networkHealthy = $catalog.ok -and $healthyCount -ge 3 -and $healthyPlaylistCount -eq $playlistResults.Count
    }
    cameras = @($results)
    playlists = @($playlistResults)
}

$destination = [IO.Path]::GetFullPath($OutputPath)
$directory = [IO.Path]::GetDirectoryName($destination)
[IO.Directory]::CreateDirectory($directory) | Out-Null
$json = $report | ConvertTo-Json -Depth 6
[IO.File]::WriteAllText($destination, $json, [Text.UTF8Encoding]::new($false))

$report.summary | Format-List
Write-Output "Evidence: $destination"
if (-not $report.summary.networkHealthy) {
    exit 1
}
