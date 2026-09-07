# MyDateTimeScreensaver v0.3.0

v0.3.0 擴充日本旅行模式：原 A380 客艙窗框更名為「自在飛行」，並新增依暖色木質觀光列車氛圍自行繪製的「列車旅行」。

## 新增與變更

- 設定畫面新增「日本旅行場景」群組，可選「自在飛行」或「列車旅行」。
- 列車旅行在 WebView2 shell 與 GDI fallback 使用暖木色框、頂棚肋條、燈飾、窗景、餐桌及座椅語彙。
- schema 升為 4，新增 `TravelStyle=0/1`；v0.2.0 的 schema 3 設定會安全沿用「自在飛行」。
- 兩種場景共用既有的 8 個 tw.live 候選、靜音 player、60 秒輪換、預抓、來源健康檢查與有界 failover。
- 玩家矩形維持完整 16:9；旅行框、地名及狀態仍位於 player 外。
- 使用者提供的列車圖片只作設計參考，沒有納入 repository、網站或成品。
- 公開下載與建置成品統一使用 `tools-screensaver-tzk` 檔名前綴；安裝後沿用既有 System32 檔名以維持升級相容。

## 驗證

- Windows 10 x64：fmt、Clippy、locked Release build 與 45 個預設非互動測試。
- 雙場景 HTML shell 結構、GDI 小尺寸及 800×450 列車 fixture。
- WebView2 Runtime 無視窗 probe、產品 WinHTTP／parser 即時來源測試及獨立來源健康 probe。
- 非互動 smoke：v0.3.0 resources、兩個場景 radio、manifest、imports、靜態 CRT、無 UI 錯誤路徑與 registry 不變。

| 成品 | Bytes | SHA-256 |
| --- | ---: | --- |
| `tools-screensaver-tzk.scr` | 808,448 | `2211fd40f847d6d3b2b01c95e318caca2da09993bb5b8d25146b2c88630ddb8a` |
| `tools-screensaver-tzk-Setup.exe` | 2,321,785 | `69879de0c40c840a9aabdd757d30af77c0c16828eeb6d99488ca23553a1f6042` |

兩個成品都沒有 Authenticode 簽章。

## 尚未完成的實機驗收

- 兩種旅行場景的實際 WebView2 `PLAYING`、靜音、地名與完整 player 顯示。
- 至少五次完整 60 秒輪換、斷網、全部來源失效與 Runtime 缺失 fallback。
- 旅行模式多螢幕／混合 DPI／拓撲變更及 30 分鐘資源觀察。
- v0.2.0 升級、安裝、解除安裝與需要 UAC 的帳號矩陣。
- Windows 11 相容性；依目前開發環境延期。

完整限制與證據見 [Phase 7 報告](phase7-report.md)、[驗收報告](acceptance-report.md)及[來源、網路與授權紀錄](japan-travel-sources.md)。
