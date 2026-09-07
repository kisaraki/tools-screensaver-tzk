# tools-screensaver-tzk v0.4.0

v0.4.0 完成專案與產品識別統一。原始碼、Cargo package／binary、Win32 resources、System32 安裝檔、Registry、WebView2 資料目錄、建置與測試腳本、文件、GitHub repository、Pages 及下載檔都使用 `tools-screensaver-tzk`。功能仍包含「標準桌曆暨時鐘模式」、「離機作業番茄鐘模式」以及含「自在飛行」與「列車旅行」的「日本旅行模式」。

## 下載

- `tools-screensaver-tzk-Setup.exe`：Windows 10 x64 安裝程式。
- `tools-screensaver-tzk.scr`：獨立螢幕保護程式檔。
- `SHA256SUMS.txt`：上述兩個成品的 SHA-256。

GitHub Pages 提供無需 GitHub 登入或身分驗證的公開直接下載。

## 雜湊與簽章

| 成品 | Bytes | SHA-256 |
| --- | ---: | --- |
| `tools-screensaver-tzk.scr` | 808,448 | `870d9e4c6110f5a6a4132cf96dc96c11aebbf4b9ce6d331ef9d374c42985fa37` |
| `tools-screensaver-tzk-Setup.exe` | 2,321,598 | `006cd1fb5352106a393fc6b527f286312cda24a877503cfcc978dc9e6d9679fb` |

兩個成品均為 `NotSigned`。Windows 可能顯示 SmartScreen 或未驗證發行者提示。

## 驗證狀態

- Windows 10 Education 22H2 x64（build 19045.6456）上的 fmt、Clippy `-D warnings`、45 個預設測試、locked Release build、PE／resource／manifest／imports smoke 與 Inno Setup 6.7.3 封裝均通過。
- WebView2 Runtime 152.0.4191.66 無視窗探測通過。
- 2026-09-07T20:17:11.3035837+08:00 的 tw.live 目錄及 8／8 內建候選探測通過。HTTP 可達不代表實際 player 已進入 `PLAYING`。
- 目前工作樹的大小寫無關文字及路徑掃描沒有改名前的英文識別。

遠端驗證沒有開啟 UI、安裝程式或 UAC，也沒有寫入 System32。改名前版本的覆蓋升級／解除安裝、旅行 player 實際播放與輪換、斷網、長時間資源、乾淨 Win10 及 Windows 11 仍為 `NOT TESTED`。完整狀態見 [Phase 8 報告](phase8-report.md) 與 [驗收報告](acceptance-report.md)。
