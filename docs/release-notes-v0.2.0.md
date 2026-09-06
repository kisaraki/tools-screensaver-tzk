# MyDateTimeScreensaver v0.2.0

v0.2.0 新增第三個「日本旅行模式」。主螢幕會在自製 A380 客艙風格窗框中播放 tw.live 所整理的 YouTube 日本即時影像，並在 player 外顯示目前城市／地區；其他螢幕顯示靜態伴隨畫面。

## 新增與變更

- 新增「日本旅行模式」設定選項；既有「標準桌曆暨時鐘模式」與「離機作業番茄鐘模式」保持相容及離線可用。
- 內建 8 個 tw.live camera ID 候選，由背景 WinHTTP worker 檢查日本目錄與 camera detail。
- YouTube player 實際進入 `PLAYING` 後立即在背景預抓不同來源，滿 60 秒時切換；來源 error／stall 時優先使用已預抓來源立即 failover，每輪最多嘗試 3 個候選。
- 主螢幕只建立一個靜音 WebView2 player；其他螢幕不建立額外直播 player。
- `/p`、`/c` 與錯誤狀態使用自製 GDI 靜態旅行畫面，不連公開網站。
- registry schema 升為 3，保留 v0.1.x 的 mode 0／1 與個人顯示偏好。
- 設定畫面加入第三模式網路說明，並保留「KOMSMOS TOOLKIT 探真拓知酷」標示。

## 執行條件與資料邊界

日本旅行模式需要目標電腦已安裝 Microsoft Edge WebView2 Evergreen Runtime。程式與 Setup 不會下載或安裝 Runtime，也不會為此觸發 UAC；Runtime 或來源不可用時保留可退出的靜態 fallback。

只有正式 `/s` 日本旅行模式會連線至 tw.live、YouTube／Google 與來源 CDN。影片保持靜音；程式不下載、錄製、轉碼、代理、保存或重新託管影片。MIT License 只涵蓋本專案程式碼與自製圖形，不涵蓋第三方網站、攝影機或影片內容。

## 驗證

- Windows 10 x64：fmt、Clippy、locked Release build 與 45 個預設非互動測試 PASS；WebView2 Runtime 與產品解析器即時來源測試另行明確執行並通過。
- WebView2 無視窗 Runtime probe：PASS，偵測版本 152.0.4191.66。
- 非互動 smoke：PASS；已驗證 x64 PE、v0.2.0 resources／manifest、第三 radio、網路揭露、imports、靜態 CRT、無 UI 錯誤路徑與 registry 不變。
- 2026-09-06T22:34:09.5322337+08:00 來源 probe：tw.live 日本目錄正常，8／8 內建候選可解析。
- Inno Setup 6.7.3 封裝與版本一致性：PASS。

| 成品 | Bytes | SHA-256 |
| --- | ---: | --- |
| `MyDateTimeScreensaver.scr` | 802,816 | `73e971462502b98e06531fac92d356cc01e60095d8f7fe2d34f861c91756b21d` |
| `MyDateTimeScreensaver-Setup.exe` | 2,319,263 | `ddc3a86d0d9d8aeb2ac353b1cc1a97b1e0bd3f53807bf2cc931fbd89e3935372` |

兩個成品都沒有 Authenticode 簽章。

## 尚未完成的實機驗收

以下項目因遠端測試不可開啟互動 UI 或 UAC，維持 `NOT TESTED`：

- 實際 YouTube player `PLAYING`、至少 5 次完整 60 秒輪換與地點核對。
- 實際斷網、全部來源失效與 WebView2 Runtime 缺失 fallback。
- 日本旅行模式的多螢幕／混合 DPI／拓撲變更及 30 分鐘 WebView2 資源觀察。
- v0.2.0 安裝、v0.1.1 升級、解除安裝與需要 UAC 的帳號矩陣。
- 8 個候選的原始提供者與個別使用條款逐一抽查。
- Windows 11 相容性；依目前開發環境延期。

完整限制與證據請見 [Phase 6 報告](phase6-report.md)、[驗收報告](acceptance-report.md)及[來源、網路與授權紀錄](japan-travel-sources.md)。
