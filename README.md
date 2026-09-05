# MyDateTimeScreensaver

[![Windows CI](https://github.com/kisaraki/tools-screensaver-tzk/actions/workflows/ci.yml/badge.svg)](https://github.com/kisaraki/tools-screensaver-tzk/actions/workflows/ci.yml)
[![GitHub Pages](https://github.com/kisaraki/tools-screensaver-tzk/actions/workflows/pages.yml/badge.svg)](https://github.com/kisaraki/tools-screensaver-tzk/actions/workflows/pages.yml)
[![Release](https://img.shields.io/github/v/release/kisaraki/tools-screensaver-tzk?include_prereleases&sort=semver)](https://github.com/kisaraki/tools-screensaver-tzk/releases)
[![License: MIT](https://img.shields.io/badge/license-MIT-17d98b.svg)](LICENSE)

MyDateTimeScreensaver 是以 Rust、原生 Win32 與 GDI 製作的 Windows x64 螢幕保護程式。它提供時間日期與倒數計時兩種畫面，不需要網路、外部字型檔或額外執行階段。

[專案網站與下載頁](https://kisaraki.github.io/tools-screensaver-tzk/) · [v0.1.0 發行說明](https://github.com/kisaraki/tools-screensaver-tzk/releases/tag/v0.1.0) · [完整開發規格](MyDateTimeScreensaver_Codex_Spec.md)

![時間日期模式：左側指針鐘與右側六列月曆](docs/evidence/phase2/fixtures/02-TimeDate-800x369-dpi96-p2-SevenSegment-palette.png)

> **v0.1.0 是未簽章的開發候選版。** Windows 10 x64 的建置、原生功能測試與非互動 smoke test 已通過；需要 UAC 的完整安裝／升級／解除安裝矩陣尚未在遠端工作階段執行。Windows 11 尚未驗證。下載後請先比對 SHA-256。

## 下載

| 檔案 | 用途 |
| --- | --- |
| [MyDateTimeScreensaver-Setup.exe](https://github.com/kisaraki/tools-screensaver-tzk/releases/download/v0.1.0/MyDateTimeScreensaver-Setup.exe) | 建議使用的 Windows x64 安裝程式 |
| [MyDateTimeScreensaver.scr](https://github.com/kisaraki/tools-screensaver-tzk/releases/download/v0.1.0/MyDateTimeScreensaver.scr) | 獨立螢幕保護程式檔，供進階使用者或檢查 |
| [SHA256SUMS.txt](https://github.com/kisaraki/tools-screensaver-tzk/releases/download/v0.1.0/SHA256SUMS.txt) | 兩個成品的 SHA-256 |

目前成品：

| 成品 | SHA-256 |
| --- | --- |
| `MyDateTimeScreensaver.scr` | `30e49516ed210d1f7b2e506f1841a0929ee8863cbbb63485f7e7bbc11b907941` |
| `MyDateTimeScreensaver-Setup.exe` | `2aa1fda583a94ea249a6265d8357afffd943507d836038b16e5bfc6a43a40d78` |

## 安裝與使用

1. 下載 Setup 與 `SHA256SUMS.txt`，先以 `Get-FileHash -Algorithm SHA256` 比對檔案。
2. 執行 Setup。安裝程式需要系統管理員權限，會將唯一的 `.scr` 安裝到 64 位元 Windows 的 System32。
3. 「將它設為目前的螢幕保護程式」預設不勾；需要時可在安裝時勾選，或稍後從 Windows 的螢幕保護程式設定選取。
4. 以 `/c` 開啟設定，選擇畫面、主色與字型。按「確定」才會保存個人設定。

成品沒有 Authenticode 簽章，因此 Windows 會顯示未驗證發行者或 SmartScreen 提示。這是目前發行狀態，不代表已完成簽章驗證。

解除安裝不會猜測使用者身分，也不會自動清除任何帳號目前選用的 `SCRNSAVE.EXE`。若解除安裝前仍選用本程式，請先在 Windows 設定改選其他項目或「無」。個人顯示偏好保留於 `HKCU\Software\MyDateTimeScreensaver`。

## 功能

- **時間日期**：圓角方形刻度鐘、連續移動的指針、星期一為首欄的六列 Gregorian 月曆，以及今天的圓形標示。
- **倒數計時**：六位七段數字、沙漏、剩餘比例線、最後十秒警示與歸零閃爍。
- **個人化**：深紅、深橘、亮綠、灰白四色；電子錶、Consolas、新細明體及自訂系統字型。
- **Windows 整合**：支援 `/s` 全螢幕、`/p HWND` 系統預覽與 `/c` 原生設定對話框。
- **顯示適配**：多螢幕、負座標、每螢幕 DPI、橫向／直向／極小畫面與防烙印位移。
- **離線與精簡**：純 Win32/GDI、靜態 CRT，沒有網路請求、遙測、常駐服務或額外 VC++ Runtime 需求。

![倒數計時模式：七段數字、沙漏與進度線](docs/evidence/phase2/fixtures/13-Countdown-800x369-dpi96-p2-SevenSegment-palette.png)

## 命令列模式

```text
MyDateTimeScreensaver.scr /s
MyDateTimeScreensaver.scr /p <HWND>
MyDateTimeScreensaver.scr /c
```

- `/s`：每台螢幕建立無邊框視窗；時間日期直接開始，倒數模式會先要求本次時、分、秒。
- `/p <HWND>` 或 `/p:<HWND>`：嵌入 Windows 提供的預覽父視窗。
- `/c` 或無參數：開啟原生設定對話框。

正常取消或退出回傳 code `0`；命令列／preview parent 錯誤為 `2`；Win32 初始化或執行期錯誤為 `3`；安裝專用 helper 拒絕或失敗為 `4`。

## 從原始碼建置

必要環境：

- Windows 10 x64 或相容的 x64 Windows 建置環境
- Rust `1.97.1` 與 `x86_64-pc-windows-msvc` target
- MSVC x64 C++ Build Tools 與 Windows SDK
- 封裝時另需 Inno Setup `6.7.3`

專案以 `rust-toolchain.toml` 固定 Rust 版本。Windows 缺少元件時，依專案規則優先使用 `winget` 查詢與安裝官方套件；遠端 runner 應預先配置 MSVC 與 Windows SDK，避免建置途中出現安裝 UI。

```powershell
rustup toolchain install 1.97.1 --profile minimal `
  --component rustfmt --component clippy `
  --target x86_64-pc-windows-msvc --no-self-update

.\scripts\build.bat
```

成功後產生 `dist\MyDateTimeScreensaver.scr`。已預先安裝正確 Inno Setup 版本時，可執行：

```powershell
.\scripts\package.bat
```

產物位於 `dist\`；該目錄不進 Git，正式下載檔由 GitHub Release 提供。

## 遠端與 CI 測試

預設驗證路徑全程非互動，不顯示設定視窗、不啟動全螢幕保護程式、不安裝成品、不觸發 UAC，也不變更 Windows 目前的螢幕保護程式設定：

```powershell
.\scripts\build.bat
powershell -NoProfile -NonInteractive -File .\scripts\smoke-test.ps1 `
  -OutputDirectory (Join-Path $env:TEMP 'MyDateTimeScreensaver-smoke')
```

`build.bat` 會執行格式檢查、Clippy `-D warnings`、35 個非互動測試與 locked Release build。預設 smoke test 驗證 PE 架構、resources、manifest、版本、imports、靜態 CRT、無 UI 的錯誤參數，以及安裝 helper 從非 System32 路徑拒絕時不改系統設定。GitHub Actions 只使用這條非互動路徑。

下列項目只供有本機桌面、可接受視窗／UAC 且已安排復原措施的人工驗收，不由遠端工作階段或 CI 執行：

- `smoke-test.ps1 -Interactive`
- ignored native UI tests
- `observe-phase4.ps1`
- `test-installation.ps1`（會安裝到 System32 並顯示 UAC）

缺少可互動環境時，相關驗收維持 `NOT TESTED`，不以編譯成功代替。現有 Windows 10 驗證邊界與逐項狀態見 [驗收報告](docs/acceptance-report.md)。

## 專案文件

- [Codex 開發規格 v1.2](MyDateTimeScreensaver_Codex_Spec.md)
- [Phase 5 封裝與交付報告](docs/phase5-report.md)
- [AC／UT／MT 逐項驗收報告](docs/acceptance-report.md)
- [視覺參考與自製畫面證據](docs/visual-reference.md)
- [FFI 與 GDI 資源稽核](docs/phase4-ffi-audit.md)

Phase 0～4 報告保留各階段當時的版本、hash 與限制；目前下載成品以 v0.1.0 Release 與 `SHA256SUMS.txt` 為準。文字 evidence 中的本機路徑與主機名稱已在公開前匿名化。

## 授權

原始碼以 [MIT License](LICENSE) 發布，Copyright (c) 2026 kisaraki。程式圖示由本專案自行繪製；使用者提供的私有視覺參考沒有納入公開 repository 或成品。
