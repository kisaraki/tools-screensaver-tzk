# Phase 22：來源持久化與隨機起播保證

產品版本：v0.14.1；規格版本：v2.9；設定 schema：9。

日期：2026-09-13；平台：Windows 10 x64。原始碼由 Git tag `v0.14.1` 鎖定。

## 實作內容

- 五組自訂來源繼續以 canonical HTTPS URL、UTF-8 `REG_BINARY` 儲存在 `HKCU\Software\tools-screensaver-tzk`。設定面板確定後自動保存；新程序啟動時由 `load_registry()` 重新載入，不依賴 WebView2 profile 或程序記憶體。
- 實際 registry adapter 測試寫入五組來源後關閉 store，再建立全新 store 讀回，模擬日後重新登入、重新開機或再次啟動。測試只使用 `HKCU\Software\Classes\tools-screensaver-tzk.Test.<PID>`，結束後刪除專用 key，不碰正式偏好。
- 每個螢幕的 `TravelSession` 新增獨立播放起點 PRNG。每次 `activate_source` 都推進一次，產生 181～539 秒；來源選擇 PRNG 與播放起點 PRNG 分離。
- 原生端在每個 load command 加入 `startSeconds`。本機 player shell 只接受整數 181～539；直接影片、清單、清單選片與預抓切換使用該次值。片尾同片重播仍由 player shell 重新隨機抽選。

## 驗證

完整 `scripts\package.bat` 已通過格式、Clippy、69 個預設測試（lib 49、CLI 8、native noninteractive 2、layout 10；9 ignored）、Release build、19 個 WebView2 與 15 個產品版本安裝判斷及 Inno Setup 封裝。結果與成品 hash 見 [package.txt](evidence/phase22/package.txt)。非互動 smoke 通過，見 [smoke-report.json](evidence/phase22/smoke/smoke-report.json)。

本階段沒有啟動正式 Setup、UAC、設定視窗、螢幕保護全螢幕或 WebView2 影片。真實 YouTube 對短片、直播、短於起點影片或 keyframe 的 seek 修正仍由平台決定，不能僅以離線測試宣稱實際畫面已從精確秒數播放。

## 成品

| 成品 | Bytes | SHA-256 |
| --- | ---: | --- |
| `tools-screensaver-tzk.scr` | 9,570,816 | `49a2d2a02608aaf574f09f7fabe970770c26f642fea771ade4995f4d384ef7ab` |
| `tools-screensaver-tzk-Setup.exe` | 12,614,509 | `bc58d235bf4f1f671153b906a9738125c14248e7b841688f014d8b87e26a0642` |

兩者版本一致且未簽章。
