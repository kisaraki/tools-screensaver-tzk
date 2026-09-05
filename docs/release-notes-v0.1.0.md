# MyDateTimeScreensaver v0.1.0

第一個公開開發候選版，提供 Windows x64 的時間日期與倒數計時兩種螢幕保護畫面。

## 內容

- 原生 Win32/GDI `.scr`，支援 `/s`、`/p HWND` 與 `/c`。
- 圓角方形指針鐘、六列 Gregorian 月曆與今天標示。
- 六位七段倒數、沙漏、進度線、最後十秒警示與歸零閃爍。
- 四種主色、三種內建字型選項與自訂系統字型。
- 多螢幕、負座標、每螢幕 DPI、橫直版與小型預覽適配。
- Inno Setup x64 安裝程式與獨立 `.scr`。

## 下載與驗證

建議下載 `MyDateTimeScreensaver-Setup.exe` 與 `SHA256SUMS.txt`。安裝程式需要系統管理員權限；「將它設為目前的螢幕保護程式」預設不勾。

| 成品 | SHA-256 |
| --- | --- |
| `MyDateTimeScreensaver.scr` | `30e49516ed210d1f7b2e506f1841a0929ee8863cbbb63485f7e7bbc11b907941` |
| `MyDateTimeScreensaver-Setup.exe` | `2aa1fda583a94ea249a6265d8357afffd943507d836038b16e5bfc6a43a40d78` |

## 已知限制

- `.scr` 與 Setup 均未含 Authenticode 簽章，Windows 可能顯示未驗證發行者或 SmartScreen 提示。
- Windows 10 x64 的建置、原生功能與非互動 smoke test 已通過。
- 遠端測試刻意不顯示 UAC、不安裝到 System32；完整安裝、升級與解除安裝矩陣仍為 `NOT TESTED`。
- Windows 11 尚無測試環境，狀態為 `NOT TESTED`。

完整狀態見 [驗收報告](https://github.com/kisaraki/tools-screensaver-tzk/blob/v0.1.0/docs/acceptance-report.md)。
