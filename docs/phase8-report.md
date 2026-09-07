# Phase 8 產品識別統一與驗證報告

日期：2026-09-07<br>
目標軟體版本：0.4.0<br>
規格：`tools-screensaver-tzk_Codex_Spec.md` v1.5<br>
必要平台：Windows 10 x64；Windows 11 依使用者指示延期

## 結論

目前工作樹的產品識別已統一為 `tools-screensaver-tzk`。Cargo package 與 binary target、Release EXE／SCR／Setup、Win32 VERSIONINFO 與 manifest、window class／title、Registry key、WebView2 資料目錄、User-Agent、Inno Setup、建置與測試腳本、CI artifact、README、規格、GitHub Pages 與下載路徑均已同步。Rust 原始碼中的 crate 識別依 Rust 語法使用 `tools_screensaver_tzk`。

大小寫無關的 repository 文字掃描與路徑掃描都沒有找到改名前的英文識別。Git commit 與 tag 歷史保留不改寫。

## 產品路徑與升級邊界

- System32 安裝檔：`tools-screensaver-tzk.scr`
- 個人偏好：`HKCU\\Software\\tools-screensaver-tzk`
- WebView2 profile／本機 shell：`%LOCALAPPDATA%\\KOMSMOS\\tools-screensaver-tzk\\`
- 建置與封裝：`dist\\tools-screensaver-tzk.scr` 與 `dist\\tools-screensaver-tzk-Setup.exe`
- 公開下載：`/tools-screensaver-tzk/downloads/v0.4.0/`

Installer 繼續使用固定 AppId，以保留安裝系統的升級識別。新 Registry key 不會自動匯入改名前版本的個人偏好；改名前版本的實際覆蓋升級、舊 System32 檔清理與解除安裝仍為 `NOT TESTED`，因為遠端工作階段不啟動 installer 或 UAC。

## 非互動驗證

| 項目 | 狀態 | 結果／證據 |
| --- | --- | --- |
| `scripts\\package.bat` | PASS | fmt、Clippy `-D warnings`、45 passed／9 ignored、locked Release build 及 Inno Setup 6.7.3 均 exit 0 |
| Resource／PE smoke | PASS | x64 PE32+ GUI、v0.4.0、`tools-screensaver-tzk` ProductName／FileDescription、dialog labels、manifest、imports、靜態 CRT、非互動錯誤路徑及 registry 前後相同；[smoke-report.json](evidence/phase8/smoke/smoke-report.json) |
| WebView2 Runtime probe | PASS | 偵測到 Evergreen Runtime 152.0.4191.66；沒有建立視窗、controller 或 player；[runtime-probe.txt](evidence/phase8/runtime-probe.txt) |
| 產品 WinHTTP／parser probe | PASS | 8 個內建候選均解析成受限 YouTube ID；[live-parser-probe.txt](evidence/phase8/live-parser-probe.txt) |
| 獨立來源健康檢查 | PASS | tw.live 目錄 HTTP 200，8／8 候選的 detail、oEmbed 與 thumbnail 可達；[source-health.json](evidence/phase8/source-health.json) |
| 文字與路徑掃描 | PASS | 排除編譯快取目錄後，大小寫無關掃描為零結果 |

所有驗證均由非互動 shell 執行，沒有開啟 `/s`、設定 dialog、WebView2 player 或安裝程式，沒有寫入 System32，也沒有觸發 UAC。來源可達不代表影片已進入 `PLAYING`。

## v0.4.0 成品

| 成品 | Bytes | SHA-256 | 狀態 |
| --- | ---: | --- | --- |
| `tools-screensaver-tzk.scr` | 808,448 | `870d9e4c6110f5a6a4132cf96dc96c11aebbf4b9ce6d331ef9d374c42985fa37` | Build／smoke PASS；NotSigned |
| `tools-screensaver-tzk-Setup.exe` | 2,321,598 | `006cd1fb5352106a393fc6b527f286312cda24a877503cfcc978dc9e6d9679fb` | Package／版本 PASS；實際安裝 NOT TESTED；NotSigned |

## 仍未實機驗證

- 改名前版本至 v0.4.0 的覆蓋升級、舊檔清理、Windows 列舉、解除安裝及跨帳號 UAC。
- 日本旅行的實際 player `PLAYING`、60 秒輪換、多螢幕、斷網與 30 分鐘資源觀察。
- 無開發工具的乾淨 Windows 10 與 Windows 11 相容性。
