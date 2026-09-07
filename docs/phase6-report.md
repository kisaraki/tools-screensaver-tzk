# Phase 6 日本旅行模式實作與驗證報告

日期：2026-09-06<br>
目標軟體版本：0.2.0<br>
規格：`tools-screensaver-tzk_Codex_Spec.md` v1.3<br>
必要平台：Windows 10 x64；Windows 11 依使用者指示延期

## 結論

Phase 6 的第三模式實作、Windows 10 非互動建置、45 個預設自動測試、WebView2 Runtime 無視窗探測、產品解析器即時來源測試、來源健康檢查、非互動 smoke 與候選成品封裝均已完成。v0.2.0 可作為明列限制的未簽章開發候選版。

實際 YouTube player `PLAYING`、連續五次 60 秒輪換、斷網／全部來源失效、多螢幕旅行畫面與 30 分鐘 WebView2 資源觀察沒有在遠端工作階段啟動，仍為 `NOT TESTED`。因此本報告不宣稱日本旅行模式或 Windows 10 完整驗收已通過。

Phase 0～5 報告、v0.1.0／v0.1.1 Release、hash 與 evidence 保留為功能加入前的歷史結果；它們沒有被改寫成旅行模式證據。

## 最終產品行為

- 設定畫面第三個選項為「日本旅行模式」，內部值為 `JapanTravel=2`；`TimeDate=0` 與 `Countdown=1` 維持相容，registry schema 由 2 升為 3。
- 正式 `/s` 只在主螢幕建立一個 WebView2 player。自製本機 HTML／CSS shell 畫出 A380 客艙風格窗框及 player 外的地點／狀態列；其他螢幕顯示自製 GDI 靜態伴隨畫面。
- 程式內建 8 個 tw.live camera ID 候選。背景 WinHTTP worker 先檢查 `https://tw.live/japan/`，再從隨機排序且不含上一個成功來源的清單解析 detail；單輪最多嘗試 3 個候選。
- YouTube player 實際回報 `PLAYING` 後，以 monotonic tick 開始 60,000 ms deadline，並在背景預抓不同 camera；到期時立即載入已準備的來源，或在尚未完成時保留目前畫面直至成功。player error／stall 優先使用已預抓來源立即 failover，其他來源失敗以 30 秒間隔重試。
- player 使用官方 YouTube IFrame API、保持控制項可見且影片靜音。窗框、地名與狀態列不覆蓋 player rect。
- `/p` 與 `/c` 的旅行預覽、其他兩種模式都不建立 WebView2，也不發公開網路 request。
- `webview2-com` 使用靜態 loader，不另帶 `WebView2Loader.dll`。Microsoft Edge WebView2 Evergreen Runtime 是目標機外部先決條件；缺少 Runtime 時只顯示 GDI fallback，程式與 Setup 不下載或安裝 Runtime，也不為此觸發 UAC。
- 程式不嵌 tw.live 整頁，不下載、錄製、轉碼、代理或保存第三方影片。

## 遠端無互動邊界

依使用者要求，本階段的自動驗證沒有開啟 `/c`、`/s`、設定 dialog、全螢幕視窗或 WebView2 player，也沒有安裝 `.scr`、Setup 或 Runtime、寫入 System32、改目前螢幕保護設定或觸發 UAC。

公開來源檢查是另行顯式執行的有界非互動 HTTPS probe。它不等於 player 實際播放驗收。

## 來源網路檢查

2026-09-06T22:34:09.5322337+08:00 執行 `scripts/check-japan-sources.ps1`。日本目錄回傳 HTTP 200，8 個內建候選的 tw.live detail、YouTube oEmbed 與縮圖檢查全部成功；另以產品實際使用的 WinHTTP 與 HTML parser 明確執行 ignored 即時來源測試並通過。

| 層級 | 狀態 | 實際證據 | 限制 |
| --- | --- | --- | --- |
| `CatalogReachable` | PASS | 日本 index HTTP 200；預期目錄標記存在 | 只代表檢查當下可達 |
| `CandidateResolved` | PASS | 8／8 detail 200，均解析出合法 11 字元 video ID；oEmbed 與縮圖 200 | 不代表直播在線或目標地區可播 |
| `PlaybackHealthy` | NOT TESTED | 沒有建立 WebView2 player | HTTP 200 不能代替 player `PLAYING` |

機器可讀結果位於 [source-health.json](evidence/phase6/source-health.json)；候選與權利／隱私邊界記錄於 [日本旅行模式來源、網路與授權紀錄](japan-travel-sources.md)。

## 自動驗證結果

| 項目 | 狀態 | 實際結果／證據 |
| --- | --- | --- |
| `scripts\build.bat` | PASS | fmt、Clippy `-D warnings`、locked tests、locked Release build 均 exit 0 |
| 單元與非互動整合測試 | PASS | 預設 45 passed：lib 26、CLI 8、native noninteractive 2、layout 9；另有 9 個互動、長時間或環境測試 ignored，其中 2 個非互動環境測試另行明確執行並通過 |
| DisplayMode／schema migration | PASS | mode 0／1 相容、schema 3 的 mode 2 round-trip、未來 schema 保護均通過 |
| 隨機候選／60 秒 deadline | PASS | 8 個 seed 唯一、排除上一來源、邊界與時間跳躍測試通過 |
| detail parser／輸入界線 | PASS | host、camera ID、video ID、HTML entity、JSON escaping、錯誤輸入測試通過 |
| player shell／狀態模型 | PASS | 單一 player、caption 位於 player 後、ready／playing／error／stall 與狀態轉換測試通過；實際播放另列 `NOT TESTED` |
| WebView2 Runtime 無視窗探測 | PASS | 找到 Evergreen Runtime `152.0.4191.66`；沒有建立 controller 或可見視窗 |
| 公開來源 HTTP probe | PASS | 8／8 候選可解析；見 `source-health.json` |
| Windows 非互動 smoke | PASS | x64 PE32+ GUI、v0.2.0 resources／manifest、第三 radio／網路揭露、imports、靜態 CRT、無 UI 錯誤參數、helper 拒絕與 registry 不變均通過；見 [smoke-report.json](evidence/phase6/smoke/smoke-report.json) |
| Package／hash | PASS | Inno Setup 6.7.3 封裝成功，`.scr` 與 Setup 版本一致；兩者 NotSigned |

Runtime probe 只證明本次 Windows 10 主機有可用 Runtime，不代表 WebView2 controller 或 YouTube player 已建立。

## v0.2.0 候選成品

| 成品 | Bytes | SHA-256 | 狀態 |
| --- | ---: | --- | --- |
| `tools-screensaver-tzk.scr` | 802,816 | `73e971462502b98e06531fac92d356cc01e60095d8f7fe2d34f861c91756b21d` | Build／smoke PASS；NotSigned |
| `tools-screensaver-tzk-Setup.exe` | 2,319,263 | `ddc3a86d0d9d8aeb2ac353b1cc1a97b1e0bd3f53807bf2cc931fbd89e3935372` | Package／版本檢查 PASS；實際安裝 NOT TESTED；NotSigned |

## Win10 實機項目

| ID | 狀態 | 已取得的局部證據 | 缺少條件 |
| --- | --- | --- | --- |
| MT16 正常播放與至少 5 次輪換 | NOT TESTED | player shell、事件與 60 秒 deadline 的 deterministic tests PASS | 遠端限制下沒有開啟 `/s`／WebView2 player |
| MT17 來源失敗與 Runtime 缺失 fallback | NOT TESTED | 錯誤 state、Runtime probe、GDI fallback code 已建置 | 需可控網路及可互動 Win10；不得為測試解除安裝 Runtime |
| MT18 多螢幕／DPI／拓撲 | NOT TESTED | 程式限制主螢幕一個 player，其他螢幕走 GDI | 需實際 WebView2 player 及互動顯示環境 |
| MT19 30 分鐘／至少 29 次輪換資源觀察 | NOT TESTED | controller 重用及 shutdown code 通過建置／靜態審查 | 需量測 parent 與 WebView2 descendant process tree |
| MT20 outbound isolation capture | NOT TESTED | host allowlist 與 CSP 已實作，preview 不建 travel session | 需可控 proxy／capture 環境，且不得破壞使用者網路設定 |

8 個候選的原始提供者與個別使用條款抽查：`NOT TESTED`。Windows 11：`NOT TESTED（依使用者指示延期）`。

## 發布判定

v0.2.0 已符合非互動候選版的建置、測試、來源探測、smoke 與封裝條件。Release、README、Pages 與 acceptance report 必須繼續明列 MT16～MT20、第三方權利抽查、互動安裝矩陣與簽章狀態；完成這些必要實機項目前，不得把候選版描述為日本旅行模式或 Windows 10 的完整驗收版本。
