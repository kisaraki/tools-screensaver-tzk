# 日本旅行模式來源、網路與授權紀錄

產品版本：0.13.0<br>
規格文件：v2.7<br>
最後非互動 HTTP 檢查：2026-09-09<br>
證據：[Phase 20 source-health.json](evidence/phase20/source-health.json)

## 來源配置

| 場景 | 執行期來源 | 啟動與切換行為 |
| --- | --- | --- |
| 自在飛行 | [YouTube playlist `PLdsqwBj2O1Nw`](https://www.youtube.com/playlist?list=PLdsqwBj2O1Nw) | 全螢幕啟動時讀取清單並隨機選片；輪換前預選下一候選 |
| 列車旅行 | [YouTube playlist `PLBH60D9AGfu0`](https://www.youtube.com/playlist?list=PLBH60D9AGfu0) | 同上 |
| 和風庭園 | [tw.live 日本旅行即時影像](https://tw.live/japan/) | 每次解析目前 camera detail，從八個候選隨機選取 |
| 御運轉士 | [YouTube playlist `PLB-Fmt68BNm4`](https://www.youtube.com/playlist?list=PLB-Fmt68BNm4) | 全螢幕啟動時讀取清單並隨機選片；輪換前預選下一候選 |
| 地方散策 | [YouTube playlist `PLbYZr39owNGo`](https://www.youtube.com/playlist?list=PLbYZr39owNGo) | 全螢幕啟動時讀取清單並隨機選片；切換時完全眨眼，順暢播放每 20～30 秒輕眨，緩衝時溫和加深 |

四種影片清單模式在全螢幕啟動時使用 YouTube 官方 IFrame Player API 的 `loadPlaylist` 讀取當下清單，再呼叫 `setShuffle(true)` 與 `getPlaylist()`，從回傳的影片 ID 中隨機選取並以 `loadVideoById` 播放。這個做法不需要 API key，也不抓取或解析 YouTube 網頁 HTML。清單讀取最多等待 5 秒；player error、清單空白或 20 秒內未開始播放時會回報失敗，原生控制器於 30 秒後有界重試。

每次載入與同片重播都重新抽選 `startSeconds=181..539`，從 3:01～8:59 的隨機位置開始；直播、短於該位置或來源不支援 seek 時，YouTube 可忽略或調整起點。排程輪換前最後一分鐘，原生 worker 先解析下一個來源，WebView shell 由已讀取的清單預選不同影片並預熱其縮圖/CDN 連線。切換時直接載入該候選；若無有效候選則回退到重新讀取清單。產品不建立第二個隱藏 player，也不在背景播放影音。

來源切換時間預設一分鐘，可設 1～1440 分鐘或「不切換」。計時從 player 回報 `PLAYING` 才開始，且每個螢幕獨立計時。「不切換」時目前隨機影片播放完會重播同一支；來源失效仍會執行復原。正式多螢幕模式會為每個螢幕建立獨立 WebView2 player，可能隨機選到相同影片。

和風庭園保留八個 tw.live camera seed。程式的 WinHTTP worker 只接受 `https://tw.live`、停用 redirect、限制 HTML 為 512 KiB，並只解析有界地點文字和 11 字元 YouTube ID。每輪最多嘗試三個候選。遠端內容一律視為不可信資料，插入本機 player shell 前會作 JSON escaping。

## 健康狀態與 2026-09-09 結果

| 層級 | 定義 | 本次結果 |
| --- | --- | --- |
| `CatalogReachable` | tw.live 日本目錄回傳預期 HTTPS HTML | PASS |
| `CandidateResolved` | camera detail 可解析允許的 YouTube ID，oEmbed 與縮圖可達 | PASS，8／8 |
| `PlaylistEmbedReachable` | 指定 `youtube.com/embed/videoseries` endpoint 回傳 HTTP 200 | PASS，4／4 |
| `PlaybackHealthy` | 產品內 WebView2 player 明確回報 `PLAYING` | NOT TESTED |

`scripts/check-japan-sources.ps1` 以每個 request 10 秒 timeout 非互動檢查上述 HTTP 層級，不開啟 `/s`、WebView2、設定畫面、安裝程式或 UAC。HTTP 200 不能證明影片可在使用者地區嵌入、可長時間播放或永遠可用。

## Player、隱私與授權邊界

正式影片固定靜音，設定 `controls=0`、`cc_load_policy=0`、`iv_load_policy=3`、`disablekb=1` 與 `fs=0`，並停用 player 的滑鼠事件。shell 另在 player ready、影片開始與 `onApiChange` 時要求關閉／卸載字幕模組；此處理針對 YouTube 可切換字幕，影片畫面本身燒錄的文字無法關閉。YouTube 已停用 `showinfo` 與 `modestbranding` 參數，因此平台仍可能短暫顯示必要的標題或品牌資訊，產品不以覆蓋層遮住第三方標示。程式不下載、錄製、轉碼、代理、保存或重新託管影片。場景框、地點與狀態位於 player 外；地方散策以兩段本機動畫先完全閉眼、切換來源後再睜眼，橢圓邊界加模糊黑暈。系統 preview、設定 preview、日期時鐘與番茄鐘不建立 WebView2，也不連公開網站。

啟動日本旅行模式會向 tw.live（只限和風庭園）、YouTube／Google 與影片來源 CDN 傳送正常連線需要的 IP 位址、User-Agent、時間與播放器資料。WebView2 profile 與本機 shell 位於 `%LOCALAPPDATA%\KOMSMOS\tools-screensaver-tzk\`，程式不自行保存影片、音訊或影格。

MIT License 只涵蓋 repository 的程式碼與自製圖形，不涵蓋第三方影片清單、tw.live、YouTube 或影片內容。公開可瀏覽不等於獲得重新散布或商業使用授權。來源可能改址、下線、限制地區或撤回嵌入；發布前應重新執行健康檢查並抽查來源政策。

## 尚待可互動 Win10 驗收

1. 每個場景確認實際 `PLAYING`、靜音與至少五次來源切換。
2. 驗證自訂分鐘、不切換、斷網、空清單、影片禁止嵌入及 Runtime 缺失 fallback。
3. 驗證多螢幕各自選片、失敗隔離、混合 DPI、退出後 controller 與 WebView2 子程序清理。
4. 核對每個影片清單與 tw.live 原始提供者的最新嵌入及使用條款。

## v0.14.0 各場景自訂 YouTube 來源

五種場景各可在設定中加入最多 10 個 YouTube 影片或清單網址。預設來源保留，自訂項目與預設入口一起隨機選取；設定預覽與 URL 格式驗證不連網，正式全螢幕播放才送至既有 YouTube IFrame player。只接受 YouTube／youtu.be 明確網址並抽取合法 ID，不直接瀏覽使用者輸入網址，不增加任意站台導覽權限。

來源按場景保存在 HKCU schema 9，沒有上傳設定至本專案服務。影片、清單及其 metadata 仍受原作者與 YouTube 的提供狀態、嵌入限制及相關條款影響。私人或失效連結不因格式通過而保證可播放；本次沒有替使用者指定新的公開影片。操作與格式見 [README](../README.md#自訂-youtube-來源)，實作契約見 [系統開發規格](tools-screensaver-tzk-system-development-spec.md#56-桌曆顯示與自訂來源)。