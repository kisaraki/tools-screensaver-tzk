# Phase 0 實作與驗證報告

- 依據：tools-screensaver-tzk 開發規格 v1.2，Phase 0。
- 軟體版本：0.1.0；驗證日期：2026-09-04（Asia/Taipei）。
- 結論：**Phase 0 工程基礎完成**。這不是整個螢幕保護程式的產品驗收完成。
- 驗收範圍補充（2026-09-04）：依使用者指示，目前只要求 Windows 10 驗證；Windows 11 延期，不影響 Phase 0 已完成的結論，也不阻擋後續 Windows 10 開發。
- Windows 10 Education 22H2／x64／build 19045；確切工具路徑與 OS 修訂版見 [environment.json](evidence/phase0/environment.json)。
- Rust 1.97.1、Cargo 1.97.1、Visual Studio Build Tools 2022、Windows SDK；沒有為此階段安裝完整 IDE 或額外 crate。

## 已完成內容

| 範圍 | 檔案 | 結果 |
| --- | --- | --- |
| Cargo binary＋library | `Cargo.toml`、`Cargo.lock`、`.cargo/config.toml`、`rust-toolchain.toml` | 固定版本／MSVC x64／靜態 CRT／Release profile |
| 最小 Win32 入口 | `src/main.rs`、`src/lib.rs`、`src/app.rs` | 取得 module、檢查名稱／圖示／manifest，成功退出；初始化錯誤 code 3 |
| UTF-16 邊界 | `src/utf16.rs` | 保留 CJK／surrogate pair、附 NUL、拒絕內嵌 NUL |
| 資源建置 | `build.rs`、`resources/*` | std 呼叫 RC；只在 OUT_DIR 產生版本 header／Rust resource IDs／RES |
| 原生資源 | `assets/app.ico` | 四尺寸自製圖示、繁體中文名稱、版本、單一 manifest |
| 可維護的圖示來源 | `assets/generate-icon.ps1`、`assets/README.md` | 無外部素材、無字型依賴，可重建 ICO |
| 工程文件 | `README.md`、`LICENSE`、本報告與 evidence | 建置方式、Phase 邊界、授權未定狀態及實測證據 |
| 版本管理準備 | `.gitignore`、Git repository、Cargo.lock | 原始碼與 lockfile 納入追蹤；target／dist 排除，未建立 Git commit |

本機已有 Rustup／Build Tools／SDK；只使用官方 `rustup toolchain install 1.97.1 ...` 安裝可明確指定名稱的同版 toolchain，未更動全域預設 stable。所有 MSVC 環境變更只作用於本次 shell。

## 指令與結果

| ID | 實際操作 | 狀態 | 證據 |
| --- | --- | --- | --- |
| P0-01 | `cargo generate-lockfile --offline` | PASS | `Cargo.lock`：windows-sys 0.61.2、windows-link 0.2.1；無 build／dev dependency |
| P0-02 | `cargo fmt --check` | PASS | [fmt.txt](evidence/phase0/fmt.txt) |
| P0-03 | `cargo check --locked --all-targets` | PASS | [check.txt](evidence/phase0/check.txt) |
| P0-04 | `cargo clippy --locked --all-targets -- -D warnings` | PASS | [clippy.txt](evidence/phase0/clippy.txt)，無 warning |
| P0-05 | `cargo test --locked` | PASS | [tests.txt](evidence/phase0/tests.txt)，3 passed／0 failed；測試只覆蓋目前已存在的 UTF-16 邊界 |
| P0-06 | `cargo build --locked` | PASS | [debug-build.txt](evidence/phase0/debug-build.txt) |
| P0-07 | `cargo build --release --locked --offline` | PASS | [release-build.txt](evidence/phase0/release-build.txt) |
| P0-08 | 啟動 Debug／Release，5 秒內等待退出 | PASS | [verification.json](evidence/phase0/verification.json)，兩者 exit code 0 |
| P0-09 | `dumpbin /headers`、直接讀取 PE header | PASS | [pe-headers.txt](evidence/phase0/pe-headers.txt)，AMD64 0x8664、Windows GUI subsystem 2 |
| P0-10 | `dumpbin /dependents` | PASS | [dependencies.txt](evidence/phase0/dependencies.txt)，只有 Windows 系統 imports |
| P0-11 | `mt.exe -inputresource:<exe>;#1 -out:<manifest>`，列舉實際資源 | PASS | [embedded.manifest](evidence/phase0/embedded.manifest)、[resource-ids.txt](evidence/phase0/resource-ids.txt) |
| P0-12 | 更新 RC 修改時間後 `cargo build --release --locked --offline -v` | PASS | [rc-rebuild.txt](evidence/phase0/rc-rebuild.txt) 明確顯示 RC changed；[結果](evidence/phase0/rc-rebuild-result.json) 證實 app.res 重新產生 |
| P0-13 | 將 RC 暫設為不存在路徑後 build，再恢復正常環境 | PASS | [missing-rc.txt](evidence/phase0/missing-rc.txt)，預期 cargo exit 101／build script exit 1，診斷指出 rc.exe 路徑；恢復後 build 成功 |
| P0-14 | 複製必要原始碼到 `target/Phase 0 中文路徑/source`，無既有 target cache，執行 locked offline Release build 並啟動 | PASS | [unicode-space-build.txt](evidence/phase0/unicode-space-build.txt)、[結果](evidence/phase0/unicode-space-result.txt) |
| P0-15 | 讀取 FileVersionInfo、載入 ICO 並檢視 | PASS | FileVersion／ProductVersion=0.1.0，繁體中文正常；[圖示預覽](evidence/phase0/app-icon.png) |

RC 的負向測試刻意造成建置失敗，是驗證錯誤處理；最終環境已恢復，正式成品來自後續成功建置。此測試沒有移除或修改系統 SDK。

## 成品檢查

Release 執行檔：

```text
target\x86_64-pc-windows-msvc\release\tools_screensaver_tzk.exe
```

- 檔案大小：485,376 bytes（474 KiB）。
- SHA-256：`48A6AA0DD805C611726BCE55174CDCA18489560CD2A9CE905C1F85727842C65B`。
- `FileVersion=0.1.0`、`ProductVersion=0.1.0`、數值版／manifest assembly 版為 0.1.0.0。
- Phase 0 成品當時的名稱字串為「日期時間螢幕保護程式」；FileDescription 標示 Phase 0；CompanyName／LegalCopyright 留空。這是歷史驗證輸出，不是目前的使用者可見模式名稱。
- 資源類型／ID：RT_ICON 1～4、RT_STRING block 1、RT_GROUP_ICON 101、RT_VERSION 1、RT_MANIFEST 1；manifest 僅一份。
- Manifest 實際包含 `asInvoker`、`uiAccess=false`、PerMonitorV2、Windows 10／11 相容性 GUID 及 Common Controls v6。
- DLL imports：kernel32.dll／KERNEL32.dll、user32.dll、ntdll.dll、api-ms-win-core-synch-l1-2-0.dll。沒有 VC++ Redistributable DLL 或第三方 runtime DLL import。
- GUI subsystem 與程式路徑確認不要求主控台；啟動驗證沒有依賴 console input／output。未開啟 Explorer Properties 視窗，版本與圖示改以直接讀取／載入檢查。
- 未簽章；未產生 `.scr`、Setup 或 installer。

PE 仍有 MSVC 靜態函式庫引入的 CodeView 記錄，參照本地檔名 `tools_screensaver_tzk.pdb`；此檔不是執行期依賴、未納入交付或 Git。Release 已套用 `strip=symbols`；進一步發布用 debug metadata 稽核屬 Phase 4。

原始碼指紋見 [source-sha256.txt](evidence/phase0/source-sha256.txt)。這裡的可追溯／可重建指固定輸入與工具鏈可成功重建，不宣稱不同目錄／時間建置的 PE 位元組完全相同。

該指紋檔保留 Phase 0 建置當時的文件快照；後續 Windows 10 驗收範圍補充只修改規格、README 與本報告，未改程式、建置設定或執行檔。

## 範圍與未驗證項

| 項目 | 狀態 | 原因／後續階段 |
| --- | --- | --- |
| Windows 11 實際啟動 | NOT TESTED | 依使用者指示延期；非目前必要驗收項，待環境具備後補驗 |
| 未安裝 VC++ Redistributable 的乾淨 Windows 10 系統 | NOT TESTED | 已檢查 imports，尚未取得乾淨 VM；後續 MT15 |
| `/s`、`/p`、`/c` 與視窗生命週期 | NOT TESTED | Phase 1 才實作 |
| 指針／月曆／倒數畫面與多螢幕 DPI | NOT TESTED | Phase 2 及後續實機矩陣 |
| 設定、字型選擇、registry、倒數輸入 | NOT TESTED | Phase 3 才實作 |
| 安全桌面／閒置啟動、30分鐘資源測試 | NOT TESTED | Phase 3～4；目前沒有可測的正式畫面 |
| 安裝／UAC／解除安裝／簽章 | NOT TESTED | Phase 5 才實作 |

目前 Windows 10 範圍的完整 AC01 仍包含後續乾淨系統測試，因此目前只能將它的 Phase 0 建置／PE 子項記為通過，不能把整個產品的 AC01～AC15 全部記為 PASS。Windows 11 延期不列為此判定的阻擋項。

本階段完成後停止。下一階段將建立命令列路由與 Win32 視窗生命週期，保持本次可建置基礎。
