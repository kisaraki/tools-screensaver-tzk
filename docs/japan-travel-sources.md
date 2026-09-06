# 日本旅行模式來源、網路與授權紀錄

產品版本：0.2.0<br>
規格文件：v1.3<br>
最後非互動 HTTP 檢查：2026-09-06T22:34:09.5322337+08:00<br>
主要目錄：[tw.live 日本旅行即時影像](https://tw.live/japan/)

## 文件用途

本文件記錄 v0.2.0 日本旅行模式實際使用的來源契約、當次網路探測，以及尚未完成的播放與權利驗證。第三方 camera ID、video ID、頁面結構、嵌入權限及可用性都可能改變；這份紀錄不是永久可用或重新散布的保證。

tw.live 是民間公開資料整合平台。其日本頁目前整理日本各地即時影像並標示資料來源為 YouTube；本程式使用 tw.live camera detail 解析出的 YouTube video ID，交由 YouTube 官方嵌入播放器播放。程式不嵌入 tw.live 整頁，也不執行其 script、廣告或追蹤碼。

## 健康狀態定義

| 層級 | 判定 | 不能代表 |
| --- | --- | --- |
| `CatalogReachable` | `https://tw.live/japan/` 經 HTTPS 回傳可接受的 2xx HTML，且包含預期的日本目錄與 YouTube 標記 | 任一候選存在或影片可播放 |
| `CandidateResolved` | 指定 camera detail 經 HTTPS 回傳 2xx，且可解析出允許 host 的 11 字元 YouTube video ID | player 已載入、直播在線或可在目標地區播放 |
| `PlaybackHealthy` | 產品內 WebView2 navigation 成功，YouTube player 明確回報 `PLAYING` | 未來持續可用或已取得第三方內容授權 |

只有第三層能在產品畫面標成播放中。catalog、detail、YouTube oEmbed 或縮圖回傳 HTTP 200 都不能冒充 `PlaybackHealthy`。

## 2026-09-06 非互動探測

`scripts/check-japan-sources.ps1` 以有界 HTTPS GET 檢查日本目錄與全部 8 個內建 camera seed；每個 request timeout 為 10 秒。每個候選都依序檢查 tw.live detail、解析出的 YouTube oEmbed 及縮圖。腳本沒有開啟 `/s`、`/c`、WebView2、安裝程式或 UAC。

日本目錄回傳 HTTP 200，8／8 候選在檢查當下可解析且三個 HTTP 檢查均成功；`networkHealthy=true`。完整機器可讀證據位於 [source-health.json](evidence/phase6/source-health.json)。

| 內建地點提示 | camera detail | 當次 video ID | detail／oEmbed／縮圖 |
| --- | --- | --- | --- |
| 北海道・札幌 | [sapporostationhbc](https://tw.live/cam/?id=sapporostationhbc) | `Ee27soLzJ5c` | 200／200／200 |
| 東京・奧多摩 | [okutamastationview](https://tw.live/cam/?id=okutamastationview) | `PXpYve3XhE8` | 200／200／200 |
| 京都・中京區 | [jpkyotokarasumadorinakagyo](https://tw.live/cam/?id=jpkyotokarasumadorinakagyo) | `rjMsbLzg5p0` | 200／200／200 |
| 大阪・JR 放出車站 | [osakahanatencam](https://tw.live/cam/?id=osakahanatencam) | `A1EYCaxAhMY` | 200／200／200 |
| 廣島・嚴島宮島 | [miyajimacamera](https://tw.live/cam/?id=miyajimacamera) | `s2CxZ7N25i0` | 200／200／200 |
| 沖繩・名護嘉利吉海灘 | [kariyushibeachnago](https://tw.live/cam/?id=kariyushibeachnago) | `THryehCFhUU` | 200／200／200 |
| 鹿兒島・垂水櫻島 | [sakurajimatarumizu](https://tw.live/cam/?id=sakurajimatarumizu) | `NfR1Y-mYEtg` | 200／200／200 |
| 長野・上高地大正池 | [jpkamikochitaishoilakealps](https://tw.live/cam/?id=jpkamikochitaishoilakealps) | `jdUIL3sodzU` | 200／200／200 |

`CatalogReachable`：**PASS**。<br>
8 個內建候選的 `CandidateResolved` 探測：**PASS（8／8）**。<br>
`PlaybackHealthy`：**NOT TESTED**。遠端驗證沒有建立 WebView2 或實際播放影片，因此沒有驗證動態影格、player state、地區限制、廣告、60 秒輪換或 player cleanup。

## 執行期來源契約

- 程式內建上表 8 個 tw.live camera ID 作為 seed。啟動時先檢查日本目錄，再把候選隨機排序；後續輪換排除上一個成功播放的 camera，同一輪最多嘗試 3 個 detail。video ID 每次都從 detail 重新解析，不能把本表當永久 video ID 清單。
- 只在使用者已選定日本旅行模式且 `/s` 正式啟動時建立來源 worker 與 WebView2。`/p`、`/c`、標準桌曆暨時鐘模式及離機作業番茄鐘模式不連公開網站。
- 執行期 WinHTTP 只連 `https://tw.live`，停用 redirect；每個連線階段 timeout 為 4 秒，單次讀取總時間上限 15 秒，HTML 上限 512 KiB。非 2xx、內容型別不符、非法 UTF-8、錯誤 host／path／video ID 或解析失敗都拒絕。
- 遠端 HTML 一律視為不可信資料。程式只取有界的地點文字與 YouTube video ID，對插入本機 shell 的字串作 JSON escaping，並自行載入固定的 `youtube-nocookie.com` player host。
- 來源解析與健康檢查在背景 worker 執行，不阻塞 Win32 視窗訊息。關閉時停止接受結果；已開始的 WinHTTP request 依有界 timeout 結束後，其晚到資料由關閉的 channel 回收。
- player 第一次進入 `PLAYING` 時才開始 60,000 ms monotonic 計時，並立即在背景預抓不同 camera。到期時載入已準備的來源；若預抓尚未完成，保留目前畫面並在第一個成功結果到達時切換。睡眠或訊息延遲跨過多個區間只切換一次，不補跑漏掉的分鐘。
- player error／stall 會立即換候選；HTTP／解析失敗每 30 秒重試。換來源時重用同一個 WebView2 controller，不每分鐘建立新的 player 視窗。
- 正式多螢幕 `/s` 只在主螢幕建立一個 autoplay player。其他螢幕使用內建 GDI 靜態伴隨畫面，不另建直播 player。
- Runtime、網路或全部候選不可用時保留可退出的 GDI fallback。程式不下載 Runtime、不顯示安裝 UI，也不觸發 UAC。
- WebView2 profile 與本機 player shell 儲存在 `%LOCALAPPDATA%\KOMSMOS\MyDateTimeScreensaver\`；不保存縮圖、影格、音訊或影片。

## Player、框架與第三方規則

正式主螢幕使用本專案自製的本機 HTML／CSS A380 客艙風格 shell；完整 16:9 YouTube player 位於窗框內，地名與狀態列在 player 矩形外。`/p`、`/c`、其他螢幕及錯誤 fallback 使用自製 GDI 靜態畫面。這些畫面不使用 Airbus 商標、航空公司塗裝或第三方照片。

影片固定靜音，播放器控制項保持顯示。程式不遮蔽或裁切 player、YouTube 品牌、廣告或 controls，也不下載、錄製、轉碼、代理或重新託管影片。[YouTube Required Minimum Functionality](https://developers.google.com/youtube/terms/required-minimum-functionality) 說明播放器可見性、最小尺寸、Referer 與 overlay 邊界；[YouTube Developer Policies](https://developers.google.com/youtube/terms/developer-policies-guide) 說明 autoplay、播放完整性及資料處理規則。每次發布都必須重新檢查目前政策。

## 授權與隱私邊界

[tw.live 常見問題](https://tw.live/faq/) 說明平台是內容整合入口，攝影機與串流由不同來源提供。公開可瀏覽不能推定為可重新散布或商業使用；8 個候選的原始提供者與個別授權條款尚未逐一完成驗證，因此權利抽查狀態為 **NOT TESTED**。若來源撤回嵌入、標示不完整或權利狀態不適用，正式來源清單必須移除該候選。

MIT License 只涵蓋本 repository 的程式碼與自製圖形，不涵蓋 tw.live、YouTube、攝影機提供者或影片內容。第三方影片沒有封入 `.scr`、Setup、Git repository 或 GitHub Pages。

啟動旅行模式會把 IP 位址、User-Agent、連線時間與播放器正常運作所需資料傳給 tw.live、YouTube／Google 及來源 CDN。[tw.live 隱私權政策](https://tw.live/privacy/) 表示一般瀏覽可能記錄 IP、使用時間、瀏覽器及瀏覽／點選資料；`youtube-nocookie.com` 不能描述成完全不傳資料。

## 後續人工驗收

1. 在可互動 Windows 10 桌面以產品本身完成至少五次 `PLAYING` 與 60 秒輪換，核對地區／鏡頭文字、靜音及 player 顯示完整性。
2. 驗證斷網、所有候選失效與 WebView2 Runtime 缺失 fallback；不能以解除安裝 Runtime 作遠端測試，也不能觸發 UAC。
3. 在實際多螢幕環境驗證主螢幕只有一個 player、其他螢幕為靜態伴隨畫面，以及退出後 WebView2 子程序清理。
4. 抽查 8 個候選的原始提供者、官方嵌入是否仍允許及相關使用條款；未完成時在 Release、README、Pages 與 acceptance report 維持 `NOT TESTED`。
