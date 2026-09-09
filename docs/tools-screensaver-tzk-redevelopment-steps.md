# tools-screensaver-tzk 從零重新開發步驟

文件版本：1.0

目標基準：重建與 v0.13.0 相容的 Windows 10 x64 版本

搭配文件：[系統開發規格書](tools-screensaver-tzk-system-development-spec.md)

本文件提供從乾淨工作目錄開始，到建置、驗證、封裝與公開發布的實作順序。每一階段都先完成可自動驗證的產物，再進入需要 Windows UI 或 UAC 的實機驗收。

## 1. 準備開發環境

### 1.1 必要工具

使用 Windows 10 x64，並以 `winget` 優先安裝：

```powershell
winget install --id Git.Git --exact
winget install --id Rustlang.Rustup --exact
winget install --id Microsoft.VisualStudio.2022.BuildTools --exact
winget install --id JRSoftware.InnoSetup --exact --version 6.7.3
```

Visual Studio Build Tools 必須包含：

- MSVC v143 x64/x86 build tools。
- Windows 10 或 Windows 11 SDK，需提供 `rc.exe`。
- x64-hosted x64 `link.exe`。

Rust 依 repository 的 `rust-toolchain.toml` 安裝：

```powershell
rustup toolchain install 1.97.1 `
  --profile minimal `
  --component rustfmt `
  --component clippy `
  --target x86_64-pc-windows-msvc `
  --no-self-update
```

Microsoft Edge 用於選用的 headless HTML fixture；Microsoft Edge WebView2 Runtime 用於實際日本旅行模式。封裝階段會下載並驗證官方 Evergreen Bootstrapper，開發機可以已有 Runtime。

### 1.2 檢查工具

```powershell
git --version
rustup toolchain list
rustc +1.97.1-x86_64-pc-windows-msvc --version
cargo +1.97.1-x86_64-pc-windows-msvc --version
where.exe rc.exe
where.exe link.exe
```

若 `rc.exe` 或 `link.exe` 不在 PATH，可直接執行 `scripts\build.bat`；腳本會透過 `vswhere.exe` 載入最新合格的 Visual Studio x64 developer environment。

## 2. 建立 repository 骨架

### 2.1 初始化

```powershell
git clone https://github.com/kisaraki/tools-screensaver-tzk.git
Set-Location tools-screensaver-tzk
git switch main
git status
```

若是完全重寫而不取用現有原始碼，仍應先建立同名 repository，加入 `README.md`、`LICENSE`、`.gitignore`、`.gitattributes`、`Cargo.toml`、`Cargo.lock`、`rust-toolchain.toml` 及 `.cargo/config.toml`。

`.cargo/config.toml` 應固定 target、靜態 CRT 與可重現 linker flag：

```toml
[build]
target = "x86_64-pc-windows-msvc"

[target.x86_64-pc-windows-msvc]
rustflags = ["-C", "target-feature=+crt-static", "-C", "link-arg=/Brepro"]
```

### 2.2 建議目錄

```text
tools-screensaver-tzk/
├─ .cargo/                    # target 與 rustflags
├─ .github/workflows/         # Windows CI、GitHub Pages
├─ assets/                    # icon 與五張旅行框景
├─ docs/                      # 規格、報告、驗證證據
├─ installer/                 # Inno Setup 與 policy include/tests
├─ resources/                 # RC、manifest、dialog、VERSIONINFO
├─ scripts/                   # 建置、封裝、smoke、fixture、實機測試
├─ site/                      # GitHub Pages 靜態網站與公開成品
├─ src/                       # Rust 原始碼
├─ build.rs
├─ Cargo.toml
├─ Cargo.lock
├─ README.md
└─ LICENSE
```

## 3. 階段 A：先建立可啟動的原生程式

1. 建立 `src/main.rs`，使用 Windows subsystem，將執行轉交 `app::run()`。
2. 建立嚴格 CLI parser，支援無參數、`/s`、`/p HWND`、`/c [HWND]`，並拒絕無效輸入。
3. 建立 `resources/resource.h`、`resources/resources.rc`、`resources/app.manifest` 與 `assets/app.ico`。
4. 在 `build.rs` 驗證 Windows x64 MSVC host/target，產生版本 header 與 Rust resource ID，呼叫 `rc.exe` 並連結 `.res`。
5. manifest 設為 `asInvoker`、PerMonitorV2、Windows 10 compatibility 與 Common Controls 6。
6. 先建立空白但可關閉的 fullscreen、preview 與 configure entry point。

完成條件：

- Release binary 無 console 視窗。
- `/s`、`/p`、`/c` dispatch 正確。
- icon、manifest、VERSIONINFO 與 Traditional Chinese dialog resource 可從 PE 讀取。
- 錯誤參數快速失敗，不建立殘留視窗。

## 4. 階段 B：建立純邏輯與設定 schema

1. 在 `model.rs` 實作 Gregorian 月曆、連續指針角度、倒數 deadline/frame、`HH:MM:SS` 及有界 PRNG。
2. 在 `config.rs` 建立 `DisplayMode`、`TravelStyle`、`ColorPreset`、`FontMode`、`AppConfig` 與 schema 8。
3. 建立 `SettingsStore` trait，先以 memory store 測試，再於 `registry.rs` 實作 HKCU adapter。
4. 實作局部回退、舊 schema gate、未來 schema 拒絕寫入與多值 rollback transaction。
5. 對所有 enum 登錄值、預設值、錯誤型別、極端數字、rollback 及 future schema 寫 unit tests。

完成條件：設定資料完全不依賴 UI 即可測試，且符合規格書第 6 節的 registry contract。

## 5. 階段 C：完成 GDI renderer

1. 在 `gdi.rs` 用 RAII 包裝 HDC、bitmap、font、pen、brush、clip 與 selection。
2. 在 `layout.rs` 以 client width/height 計算所有 `Rect`；不得在 renderer 中散落固定螢幕解析度。
3. 建立黑色背景與雙緩衝。
4. 完成日期時鐘：刻度、縮小並內移的 12/3/6/9、三支指針、月曆、今日圓形反白。
5. 完成番茄鐘：倒數、沙漏、最後十秒、完成動畫，以及有透明層次、高光、暗部與圓角的深棕藥劑瓶玻璃進度面板。
6. 完成色彩 preset、自動每 120 秒輪換、七段/Consolas/細明體/自訂字型。
7. 實作 64%×60% 中央舞台、橫直向布局、Full/Compact/Tiny 與有界防烙印位移。

完成條件：離線 fixture 覆蓋至少 1920×1080、3840×2160、1080×1920、320×180、120×80、多 DPI、全部色彩與代表字型；鐘面數字與刻度無重疊。

## 6. 階段 D：完成設定與倒數 dialog

1. 以 RC dialog template 建立三種主模式、五種旅行場景、八種色彩、字型與旅行切換欄位。
2. 加入 `KOMSMOS TOOLKIT／探真拓知酷` 圖示標示。
3. 設定 dialog 載入 draft，控制項變更只刷新離線預覽；按確定才交易保存。
4. 日本旅行未選中時停用場景與切換控制；選擇不切換時停用分鐘 edit。
5. 自訂字型使用標準系統 font picker，保存正規化 LOGFONT 與 point size。
6. 番茄鐘全螢幕啟動前顯示時間輸入；校驗 99:59:59 上限、禁止 0，取消即不啟動 saver。

完成條件：取消不寫 registry、無效輸入有中文錯誤、tab order 與 access key 可用、預覽不連網。

## 7. 階段 E：完成視窗、多螢幕與退出

1. `monitor.rs` 使用 `EnumDisplayMonitors`，主要螢幕先排序並保留負座標；失敗才回退 virtual screen metrics。
2. `window.rs` 建立一個 coordinator 與每螢幕 surface，使用共享、不可變的 frame snapshot 同步時間。
3. 實作 timer：番茄鐘動畫期間 100 ms，其餘 1000 ms。
4. 顯示前把游標移至主要螢幕左上外角非 player 區，隱藏後再建立輸入 baseline。
5. 實作 500 ms grace、4 px threshold、按鍵/滑鼠/滾輪退出、失去前景、display change、power/time change。
6. 所有 window、timer、GDI、cursor 與 message loop 清理必須冪等；最後 surface 關閉後才退出。

完成條件：單元測試覆蓋負座標、baseline、重複 shutdown、later-window create failure 與每螢幕狀態隔離。

## 8. 階段 F：建立五種旅行框景

1. 準備五張 1586:992 PNG：`free-flight.png`、`train-journey.png`、`japanese-inn.png`、`train-cab.png`、`walking.png`。
2. 素材必須是自有、合法取得或可追溯的生成圖；不要將使用者參考照片直接當成可散布素材。
3. `travel_art.rs` 使用 WIC 解碼 embedded PNG，保留比例繪入 GDI fallback。
4. 在 `travel.rs` 的本機 HTML/CSS template 定義各窗孔。
5. 御運轉士採 16:9 center-cover 並由窗孔 clip；地方散策採 0.4% 外緣、中央約 65% 視域的強烈 vignette。
6. caption 一律在 artwork 下方獨立顯示地點與狀態。

完成條件：執行離線 headless fixture，在 1600×1000 與 900×1600 驗證五種場景；player 維持 16:9、御運轉士無左右黑帶、caption 不覆蓋框景。

```powershell
powershell -NoProfile -NonInteractive -File .\scripts\export-travel-shell-fixtures.ps1 `
  -OutputDirectory docs/evidence/current/html
```

## 9. 階段 G：建立旅行來源與 player shell

### 9.1 原生來源層

1. 定義四個 playlist ID 與八個 tw.live camera seed。
2. 使用 WinHTTP worker；UI thread 不得做網路 I/O。
3. 限制 scheme、host、redirect、response bytes、字串長度與重試次數。
4. 每個結果帶 monotonic token；過期 worker 結果不得覆蓋新狀態。
5. 建立 `TravelRotation`，切換從 PLAYING 起算，0 表示不排程，切換前 1 分鐘 prefetch。

### 9.2 HTML/JavaScript shell

1. 以固定 local HTTPS virtual host 載入 shell，建立 YouTube IFrame Player。
2. playlist 啟動時 `loadPlaylist`、shuffle、讀回 ID、隨機選片。
3. 每次 load/replay 使用新的 181～539 秒起點。
4. 固定 mute、隱藏 controls、停用 keyboard/fullscreen/annotations，並在 lifecycle events 重申字幕關閉。
5. 下一候選只預選 ID 與預熱 thumbnail/CDN，不建立第二播放器。
6. 事件只送 `shell-ready` 或 `ready|playing|error|stalled:<token>` 類型的有界訊息。
7. 地方散策 source switch：先 420 ms close，呼叫 load，再 520 ms open；另排 20～30 秒 gentle blink 與 buffering blink。

### 9.3 WebView2 host

1. 每個螢幕建立獨立 controller，但共用已落地的 shell/assets。
2. COM 使用 STA；async creation completion 必須能在視窗先關閉時安全丟棄。
3. 套用規格書第 9 節的 navigation、permission、download、new-window、DevTools、autofill 與 message hardening。
4. controller 必須先隱藏，完成 shell navigation、套用 source 後才顯示。
5. runtime 缺失或 renderer failure 時顯示原生 fallback，且仍可用輸入退出。

完成條件：純 Rust/JS source tests、離線 shell geometry 與 WebView2 ignored environment tests 均具備；實際影片另在互動驗收執行。

## 10. 階段 H：建立安裝器

1. 使用固定 Inno AppId，限制 x64、64-bit mode、Windows build 15063 以上及 admin 權限。
2. 將 `.scr` 安裝至 `{sys}`，不要建立第二份不同檔名的產品。
3. `prepare-webview2.ps1` 依 `installer/webview2-bootstrapper.json` 下載並驗證官方 bootstrapper；不要在 lock 未審核時自動更新 hash。
4. 建立 machine-level Runtime 探測、選用安裝 task、安裝後再次確認與 restart/failure 訊息。
5. 建立同版 repair、異版互動移除詢問、異版 silent block，以及 uninstaller 缺失/失敗後中止。
6. 建立 `--install-set-current` helper 的路徑與未提升 token 防護，設定 `SCRNSAVE.EXE`、active 與 60 秒 timeout，保留 secure login。
7. 完成頁加入預設勾選的 `/c` 設定面板選項，設定 `skipifsilent` 與 `runasoriginaluser`。
8. uninstall 保留 per-user settings、cache、Runtime 與目前 saver 選擇，互動模式顯示後續操作提示。

完成條件：兩組 installer policy harness 可在不建立 wizard、不執行安裝、不寫 registry、不觸發 UAC 的情況下通過。

## 11. 日常非互動驗證

遠端開發與 CI 的標準入口：

```powershell
cmd /c scripts\build.bat

powershell -NoProfile -NonInteractive -File .\scripts\smoke-test.ps1 `
  -ArtifactPath .\dist\tools-screensaver-tzk.scr `
  -OutputDirectory .\docs\evidence\current\smoke
```

選用但應在旅行功能變更時執行：

```powershell
powershell -NoProfile -NonInteractive -File .\scripts\export-phase2-fixtures.ps1 `
  -OutputDirectory .\docs\evidence\current\fixtures

powershell -NoProfile -NonInteractive -File .\scripts\export-travel-shell-fixtures.ps1 `
  -OutputDirectory .\docs\evidence\current\html

powershell -NoProfile -NonInteractive -File .\scripts\check-japan-sources.ps1 `
  -OutputPath .\docs\evidence\current\source-health.json
```

`check-japan-sources.ps1` 會連網，其餘標準驗證不得顯示產品 UI、建立 player、安裝程式、觸發 UAC 或更動目前 saver registry。

提交前至少執行：

```powershell
git diff --check
rg -n -i "MyDateTimeScreenSaver|MyDateTimeScreensaver|MyDateTime" `
  --glob '!target/**' --glob '!docs/evidence/**' .
git status --short
```

舊產品名稱搜尋結果應為空。

## 12. 互動 Windows 10 驗收

此階段必須在可直接操作桌面的測試機執行，不要在無人遠端 session 啟動 `/s` 或 UAC。

1. 建立 registry 與檔案快照。
2. 執行 `scripts\test-installation.ps1`；確認腳本最終還原原始 saver registry。
3. 測試全新安裝、相同版本 repair、舊版升級、較新版衝突拒絕、取消移除、uninstaller 缺失、解除安裝。
4. 測試 WebView2 已存在、缺少後成功安裝、離線失敗、取消 task、要求 restart。
5. 確認完成頁勾選時開啟設定，取消時不開啟；`/SILENT` 與 `/VERYSILENT` 不顯示設定。
6. 在 Windows 螢幕保護程式控制台測 `/p`；確認 parent resize、DPI 與關閉。
7. 逐一執行三種 `/s` 模式，測鍵盤、滑鼠、滾輪、失焦與游標恢復。
8. 以單螢幕、延伸雙螢幕、主要螢幕非左上、負座標與不同 DPI 重複。
9. 日本旅行五場景各測啟動、正常播放、字幕偏好、3 分鐘後 seek、切換、下一來源預備、buffering、斷網與恢復。
10. 日期時鐘、番茄鐘、日本旅行各執行 30 分鐘，記錄 GDI/USER objects、private bytes、handles、CPU 與 crash/hang。

如只需先檢查 harness，可使用較短時間；正式資源驗收仍應每模式 1800 秒：

```powershell
powershell -NoProfile -File .\scripts\observe-phase4.ps1 `
  -DurationSeconds 1800 `
  -OutputDirectory .\docs\evidence\current\observation
```

## 13. 封裝

### 13.1 版本更新

更新：

1. `Cargo.toml` package version。
2. `Cargo.lock` 本套件 version，可執行 `cargo +1.97.1-x86_64-pc-windows-msvc check --locked` 前先以正常 Cargo 更新 lock。
3. `resources/app.manifest` 的 `major.minor.patch.0`。
4. README、網站、release notes、驗收報告與系統規格的目前版本。
5. 新增 `site/downloads/vX.Y.Z/`，不要覆寫舊版公開下載。

### 13.2 產生 Setup

```powershell
cmd /c scripts\package.bat
```

預期產物：

```text
dist/tools-screensaver-tzk.scr
dist/tools-screensaver-tzk-Setup.exe
dist/SHA256SUMS.txt
```

核對：

```powershell
Get-FileHash -Algorithm SHA256 .\dist\tools-screensaver-tzk.scr
Get-FileHash -Algorithm SHA256 .\dist\tools-screensaver-tzk-Setup.exe
Get-Content .\dist\SHA256SUMS.txt
```

將三個檔案複製到對應的 `site/downloads/vX.Y.Z/`，再次核對二進位 hash 完全相同。若有正式 Authenticode 憑證，簽章應安排在最終 package 後，並重新產生 SHA-256；不可沿用簽章前 hash。

## 14. 文件與網站同步

每次 release 更新：

- `README.md`：下載、安裝、解除安裝、功能、限制、測試數量、目前 hash。
- `docs/acceptance-report.md`：目前驗證證據與 `NOT TESTED`。
- `docs/japan-travel-sources.md`：來源 ID、最近健康檢查與外部內容邊界。
- `docs/release-notes-vX.Y.Z.md`：本版差異、hash、限制。
- `site/index.html`：版本、功能說明、公開下載、hash、安裝與 uninstall。
- 本系統規格與重建步驟：若架構、schema、工具版本或流程有變更，必須同步。

網站下載應使用 GitHub Pages 直連，讓未登入 GitHub 的使用者也能下載。

## 15. Git、Release 與 GitHub Pages

先確認工作目錄與差異：

```powershell
git status --short
git diff --check
git add --all
git diff --cached --check
git commit -m "Release vX.Y.Z"
git tag -a vX.Y.Z -m "tools-screensaver-tzk vX.Y.Z"
git push origin main
git push origin vX.Y.Z
```

以 GitHub CLI 建立公開 release：

```powershell
gh release create vX.Y.Z `
  "dist/tools-screensaver-tzk-Setup.exe#tools-screensaver-tzk-Setup.exe" `
  "dist/tools-screensaver-tzk.scr#tools-screensaver-tzk.scr" `
  "dist/SHA256SUMS.txt#SHA256SUMS.txt" `
  --title "tools-screensaver-tzk vX.Y.Z" `
  --notes-file "docs/release-notes-vX.Y.Z.md"
```

`.github/workflows/ci.yml` 應在 `main` push、pull request 與手動觸發時於 `windows-2022` 執行 build 與非互動 smoke。`.github/workflows/pages.yml` 應在 `site/**` 變更時部署完整 `site/`。

等待兩個 workflow 完成：

```powershell
gh run list --branch main --limit 6
gh run watch RUN_ID --exit-status
```

## 16. 匿名下載驗證

驗證時不要讓 `curl` 帶 GitHub token、browser cookie 或已登入 session：

```powershell
$version = 'vX.Y.Z'
$base = "https://kisaraki.github.io/tools-screensaver-tzk/downloads/$version"
$verify = Join-Path $env:TEMP "tools-screensaver-tzk-$version-public"
New-Item -ItemType Directory -Force $verify | Out-Null

curl.exe --fail --location --output "$verify\tools-screensaver-tzk-Setup.exe" `
  "$base/tools-screensaver-tzk-Setup.exe"
curl.exe --fail --location --output "$verify\tools-screensaver-tzk.scr" `
  "$base/tools-screensaver-tzk.scr"
curl.exe --fail --location --output "$verify\SHA256SUMS.txt" `
  "$base/SHA256SUMS.txt"

Get-FileHash -Algorithm SHA256 "$verify\tools-screensaver-tzk-Setup.exe"
Get-FileHash -Algorithm SHA256 "$verify\tools-screensaver-tzk.scr"
Get-Content "$verify\SHA256SUMS.txt"
```

HTTP 必須為 200，兩個 binary hash 必須與 release manifest 一致。Git checkout 可能將文字 manifest 正規化為 LF，因此應比較 manifest 內容所列的 binary hash，不應要求不同下載通道的 `SHA256SUMS.txt` 本身 byte hash 相同。

## 17. 失敗處理

| 症狀 | 檢查 |
| --- | --- |
| 找不到 `rc.exe` | 安裝 Windows SDK，確認 VS Build Tools workload，或從 x64 Developer Prompt 執行 |
| 找到錯誤 `link.exe` | `where link.exe`；第一個必須是 `Hostx64\x64\link.exe` |
| manifest 版本不一致 | 同步 Cargo `X.Y.Z` 與 manifest `X.Y.Z.0` |
| WebView2 bootstrapper 驗證失敗 | 不要直接改 hash；重新取得官方檔案並人工審核 lock 的 URL、版本、bytes、簽章 |
| Setup 拒絕 package | 確認 Inno Setup 恰為 6.7.3，並先單獨跑兩個 policy test |
| 日本旅行只有 fallback | 檢查 machine-level Runtime、profile 目錄權限、來源健康與 WebView2 process failure |
| 第二螢幕持續連線失敗 | 確認每螢幕 token、WebView controller、navigation complete 與 player event 沒有共用錯誤狀態 |
| 御運轉士有黑帶或壓框 | player 需 16:9、寬度填滿 aperture、垂直置中，由 aperture `overflow:hidden` 裁切 |
| 地方散策只閉不開 | source transition 必須是 close timer → load → 獨立 open animation，不可用單一 class 同時處理 |
| 安裝後未準時啟動 | 驗證 System32 helper 路徑、未提升 token、三個 HKCU Desktop 值、SPI calls 與 `WM_SETTINGCHANGE`；再查群組原則 |

## 18. 每次重新開發完成檢查表

- [ ] 產品名稱與所有路徑均為 `tools-screensaver-tzk`。
- [ ] Cargo、manifest、SCR、Setup、網站與 release 版本一致。
- [ ] 三種主模式與五種旅行場景完整。
- [ ] schema 8 讀寫與舊資料相容，future schema 不被覆寫。
- [ ] 日期時鐘與番茄鐘集中於中央友善面積。
- [ ] 八種色彩與四種字型來源可用。
- [ ] 多螢幕、負座標、PerMonitorV2、輸入退出與游標恢復完成。
- [ ] WebView2 hardening、來源限制、字幕關閉、隨機起點、prefetch 與 fallback 完成。
- [ ] 御運轉士 cover 與地方散策 close/open blink 通過離線幾何／source tests。
- [ ] 異版不能並存，WebView2 prerequisite、set-current 與完成頁設定選項完成。
- [ ] build、smoke、installer policy、fixtures 與來源健康通過。
- [ ] 需要 UAC 或 GUI 的項目只在可互動 Windows 10 執行；未執行者清楚標記 `NOT TESTED`。
- [ ] README、規格、步驟、release notes、網站、公開檔案與 SHA-256 同步。
- [ ] GitHub repository 為 PUBLIC，Release 與 Pages 匿名下載實測 HTTP 200。
