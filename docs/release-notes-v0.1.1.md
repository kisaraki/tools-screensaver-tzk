# tools-screensaver-tzk v0.1.1

此版本更新使用者可見的模式名稱與設定畫面產品識別，功能與原有設定相容。

## 變更

- 「時間日期」更名為「標準桌曆暨時鐘模式」。
- 「倒數計時」更名為「離機作業番茄鐘模式」。
- 設定畫面加入「KOMSMOS TOOLKIT 探真拓知酷」識別，非互動 smoke test 會檢查內嵌資源字串。
- 內部 enum、registry 值與測試介面仍使用 `TimeDate` 與 `Countdown`，既有偏好可繼續使用。

## 下載與驗證

建議下載 `tools-screensaver-tzk-Setup.exe` 與 `SHA256SUMS.txt`。安裝程式需要系統管理員權限；「將它設為目前的螢幕保護程式」預設不勾。

| 成品 | SHA-256 |
| --- | --- |
| `tools-screensaver-tzk.scr` | `02f34b45a2ca65069721aae3fd5401d9fa0becf3498645bdc0e234ad38387d2a` |
| `tools-screensaver-tzk-Setup.exe` | `6e3cd998e5d220aa07ead08686528544dc4f24fecba67c634c0bf2a292e82712` |

## 驗證狀態與限制

- Windows 10 x64 的建置、自動測試、原生功能與 smoke test 使用無互動流程執行。
- 遠端驗證不觸發 UAC、不寫入 System32；完整安裝、覆蓋升級與解除安裝矩陣仍為 `NOT TESTED`。
- `.scr` 與 Setup 均未含 Authenticode 簽章，Windows 可能顯示未驗證發行者或 SmartScreen 提示。
- Windows 11 尚無測試環境，狀態為 `NOT TESTED`。

完整狀態見 [驗收報告](https://github.com/kisaraki/tools-screensaver-tzk/blob/v0.1.1/docs/acceptance-report.md)。
