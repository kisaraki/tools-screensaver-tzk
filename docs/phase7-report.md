# Phase 7 雙旅行場景實作與驗證報告

日期：2026-09-07<br>
目標軟體版本：0.3.0<br>
規格：`tools-screensaver-tzk_Codex_Spec.md` v1.4<br>
必要平台：Windows 10 x64；Windows 11 依使用者指示延期

## 結論

日本旅行模式已提供「自在飛行」與「列車旅行」兩種可保存場景。原 A380 客艙風格只更改使用者名稱及識別文字；來源、播放器、輪換與失敗回退契約保持相容。列車旅行依使用者附件的暖色木質觀光列車氛圍重新繪製，沒有複製或打包附件。

Windows 10 非互動建置、45 個預設測試、雙場景 HTML shell 結構、雙場景 GDI fixture、schema 3 → 4 相容、resource smoke、Runtime probe、來源 probe 與封裝均已通過。實際 player 播放、完整輪換、多螢幕、斷網、長時間資源與 UAC 安裝矩陣仍維持 `NOT TESTED`。

## 產品行為

- `TravelStyle=0` 顯示「自在飛行」；`TravelStyle=1` 顯示「列車旅行」。
- schema 3、缺值、型別錯誤或範圍外值都只讓場景回退為「自在飛行」，不破壞其他合法設定。
- 設定畫面的場景 radio 只在「日本旅行模式」啟用；切換其他顯示模式時保留草稿，按「確定」才一起保存。
- 正式 `/s` 依場景產生本機 WebView2 shell；preview、其他螢幕及 Runtime／網路失敗時由同一場景的 GDI fallback 呈現。
- 「列車旅行」使用暖木色框、頂棚肋條、燈飾、窗景、餐桌與座椅抽象圖形；「自在飛行」保留原客艙窗框。
- 兩種場景的完整 16:9 player 均無 overlay；地名與連線狀態保留在 player 外。
- 公開下載與 `dist` 成品使用 `tools-screensaver-tzk` 檔名前綴；安裝後的 System32 檔名與 AppId 保持不變，避免破壞既有升級路徑。

## 遠端無互動邊界

本階段不開啟 `/s`、設定 dialog、WebView2 player 或安裝程式，不寫 System32、不改目前螢幕保護設定，也不觸發 UAC。使用者附件只供視覺理解，沒有複製到原始碼、網站、成品或 evidence。

## 自動驗證

| 項目 | 狀態 | 結果／證據 |
| --- | --- | --- |
| fmt／Clippy／locked tests／Release | PASS | `scripts\\build.bat` exit 0 |
| 預設非互動測試 | PASS | 45 passed、9 ignored；Runtime 與即時來源測試另行明確執行 |
| 雙場景 shell 結構 | PASS | 每種 shell 恰有一個 player、正確 body class／aria label、caption 位於 player 後 |
| schema migration | PASS | schema 3 固定回退自在飛行；schema 4 兩值 round-trip；future schema 禁止寫入 |
| GDI fixture | PASS | 800×450 列車旅行 fixture 產生成功，172,965 個非黑像素；未開視窗 |
| resource smoke | PASS | v0.3.0、兩個場景 radio、manifest、imports、靜態 CRT、無 UI 錯誤路徑及 registry 不變；[smoke-report.json](evidence/phase7/smoke/smoke-report.json) |
| Runtime／即時來源 probe | PASS | Runtime 152.0.4191.66；產品 WinHTTP／parser 與獨立來源 probe 8／8 通過，未建立視窗或 player；[source-health.json](evidence/phase7/source-health.json) |
| Package／hash | PASS | Inno Setup 6.7.3 封裝成功；兩個成品版本一致 |

## v0.3.0 成品

| 成品 | Bytes | SHA-256 | 狀態 |
| --- | ---: | --- | --- |
| `tools-screensaver-tzk.scr` | 808,448 | `2211fd40f847d6d3b2b01c95e318caca2da09993bb5b8d25146b2c88630ddb8a` | NotSigned |
| `tools-screensaver-tzk-Setup.exe` | 2,321,785 | `69879de0c40c840a9aabdd757d30af77c0c16828eeb6d99488ca23553a1f6042` | NotSigned；實際安裝 NOT TESTED |

## 未完成的實機驗收

- 「自在飛行」與「列車旅行」正式 WebView2 player 畫面、地名、靜音及一般輸入退出。
- 至少五次 60 秒輪換、player error／stall、斷網與全部來源失效。
- 多螢幕、混合 DPI、顯示拓撲變更及 30 分鐘 parent＋WebView2 process tree 資源觀察。
- v0.2.0 → v0.3.0 實際升級、安裝、解除安裝與跨帳號 UAC 矩陣。
- Windows 11 相容性。
