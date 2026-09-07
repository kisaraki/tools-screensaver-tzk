# Phase 4 驗證報告

日期：2026-09-05  
規格：`tools-screensaver-tzk_Codex_Spec.md` v1.2（修訂版）<br>
範圍：品質、FFI／資源稽核、smoke、效能與目前可用 Win10 相容性；未進入 Phase 5

## 結果

Phase 4 程式與目前環境可執行的驗證已完成，沒有已知 GDI／USER／kernel handle 洩漏。正式產品路徑已移除顯式 `unwrap`／`expect`，新增有界 `smoke-test.ps1`、Release 長時間資源觀察工具與 MSVC `/Brepro` 可重現連結。

本次必要平台是 Windows 10 x64。Windows 11 依使用者指示延期。單螢幕實體 1920×1080／100%、混合 DPI／直向、睡眠／恢復／校時／跨午夜、安全桌面、實際拔除顯示器、登出與沒有開發工具的乾淨 Windows 主機缺少環境，均明列 `NOT TESTED` 或 `PARTIAL`，因此不宣稱完整平台驗收。

## FFI 與 ownership

逐項結果見 [Phase 4 FFI 與資源所有權稽核](phase4-ffi-audit.md)。稽核涵蓋 166 個 unsafe block、4 個 Win32 callback、視窗 state transfer、CreateWindow 兩個失敗階段、group shutdown 重入、timer、cursor、dialog borrowed state、Registry key、GDI object selection、DC／bitmap／font／pen 與錯誤碼保存。

兩個正式路徑 panic 點已消除：DWORD 轉換改為失敗即 fallback，預設 `FontSpec` 改為直接建構私有合法欄位。`static mut` 為 0；其餘 `unwrap`／`expect` 只在 `#[cfg(test)]`。

## 有界 smoke 與成品

[smoke-test.ps1](../scripts/smoke-test.ps1) 從 repository 外的工作目錄，以唯一暫存目錄複製本次 Release 為 `.scr`，只追蹤自己建立的 Process，並在 `finally` 驗證暫存路徑後清除。

| 項目 | 結果 |
| --- | --- |
| SHA-256 copy integrity | PASS |
| PE | x64 `0x8664`、PE32+、Windows GUI |
| Resources | group icon 101、dialogs 2003／2004、string table、VERSIONINFO、RT_MANIFEST #1 均可載入 |
| Manifest | `asInvoker`、`uiAccess=false`、PerMonitorV2、Common Controls 6、amd64、Windows 10 GUID |
| Version | Cargo／FileVersion／ProductVersion 均為 `0.1.0` |
| Imports／CRT | 僅 Windows DLL／API-set；無 `msvcp*`、`vcruntime*`、`ucrtbase`、`api-ms-win-crt*` |
| Release Debug 入口 | `--dev-render=time-date` 無 UI、code 2 |
| 錯誤參數 | `/p`、`/p:0`、`/s /c` 均無 UI、code 2 |
| 顯式互動 smoke | `/c`、`/s`、`/p <test HWND>` 均建立視窗並在有界時間正常 code 0 |
| Authenticode | `NotSigned`；符合開發階段，公開發布簽章屬 Phase 5 |

加入 `/Brepro` 後，正常 Release 與 fresh target 的 `--offline --locked` Release build 得到相同 SHA-256：`88749e4c22e6b77a4a56c63989a4a609c3686aab6d7f6bad47f21c6535235829`。第一次離線比對曾因 PE timestamp／CodeView GUID 不同而不一致；該結果保留，沒有覆寫成成功。離線測試亦通過，證明快取依賴後不需網路。

## 30 分鐘資源與效能

[observe-phase4.ps1](../scripts/observe-phase4.ps1) 只在正式產品設定鍵不存在時執行；建立唯一 lease，結束時確認沒有未知 value／subkey 且 lease 未被替換後才刪除。標準桌曆暨時鐘模式與離機作業番茄鐘模式各用本次 Release `/p` 在 96-DPI host 執行 1,800 秒；每 30 秒輪替 6 種尺寸、3 種字型與 4 種色彩。每個模式完成 59 次實際 cache 重建。

| 模式 | 時間 | 全機 CPU 平均 | private 1 min → 30 min | 最大 working set／預算 | GDI range | USER range | 循環 | 結果 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| 標準桌曆暨時鐘模式（`TimeDate`） | 1,800.133 s | 0.0131% | 1,458,176 → 1,536,000 bytes | 7,880,704／37,607,424 bytes | 4 | 0 | 59 | PASS |
| 離機作業番茄鐘模式（`Countdown`） | 1,800.004 s | 0.0157% | 1,503,232 → 1,376,256 bytes | 7,757,824／37,607,424 bytes | 1 | 0 | 59 | PASS |

CPU 依規格使用 `100 × CPU 秒增量 /（牆鐘秒 × 8 logical processors）`。單一 1920×1080 buffer 的工程預算是 `24 MiB + 1.5 × 4 × W × H = 37,607,424 bytes`。兩者 CPU 均遠低於 1%，最大 working set 未達預算四分之一；private growth 未接近 `max(4 MiB, baseline 10%)`；GDI／USER／總 handle 也沒有持續上升。觀察使用的 Release hash 是 `/Brepro` 修正前的 `d74a3221c4fcd02061378117ea3ade6680f796b779e3ca90be7b507e583246b0`；其後唯一產品建置變更為 linker metadata 的 `/Brepro`，程式碼、資源與依賴未變。最終成品另由 smoke、Release 原生測試及位元重現比對驗證。

這次長測是 1920×1080／96-DPI Release preview host，並非實體單螢幕全螢幕，也不涵蓋正式全螢幕離機作業番茄鐘模式的 10 Hz 長時間 CPU；後者保持 `NOT TESTED`。短時間 `/s` 已在雙 4K 實機驗證 10 Hz 倒數、同步 surface 與退出清理。

## Windows 10 實機矩陣

驗證主機：Windows 10 Education 22H2 x64，build 19045.6456；Intel Core i5-8259U，8 logical processors；雙 3840×2160、兩者 144 DPI／150%，次螢幕在左側負 X。

| ID | 結果 | 實際證據與限制 |
| --- | --- | --- |
| MT01 | PARTIAL | Win10 build 已記錄；Release `/s`／`p`／`c` 通過。安裝、Windows 列舉與解除安裝屬 Phase 5，未測。 |
| MT02 | PARTIAL | 1920×1080／96-DPI preview 長測與 fixture 通過；沒有實體單螢幕 100% 全螢幕環境。 |
| MT03 | PASS | 雙 4K／150%，左側負 X；兩個 surface 全覆蓋、共用 generation、十一種退出訊息清理。次螢幕在上方未測。 |
| MT04 | PARTIAL | 4K／150% 與直向 fixture 通過；實體混合 100%／150%／200%、200% dialog 與直向螢幕未測。 |
| MT05 | PARTIAL | 真實 host 保持 `/p` 開啟，保存後 2 秒內刷新、取消不寫；Windows 設定頁待 `.scr`／Phase 5。 |
| MT06 | PASS | unaware 96、system 144、PMv2 144 host 的 480×270、0×0、1×1、320×180 resize 與 parent 關閉退出通過。 |
| MT07 | PASS | 1 秒、5 秒、30 分鐘、99:59:59 輸入／保存／預填及完成動畫測試通過。 |
| MT08 | NOT TESTED | 無已安裝 `.scr` 與可控制的閒置／安全桌面環境；手動 `/s` 不代替。 |
| MT09 | PARTIAL | deadline、校時模型、逾期跳過與跨午夜日曆由純測試通過；實際睡眠／恢復、校時與跨午夜未測。 |
| MT10 | PARTIAL | `WM_DISPLAYCHANGE`、session end、close 與外部 foreground 清理路徑已測；Windows 拒絕外部 foreground activation 的一次分支，以及實際拔線／登出未測。 |
| MT11 | NOT TESTED | 需 Phase 5 installer、標準帳號與另一管理員 UAC。 |
| MT12 | NOT TESTED | 需 Phase 5 installer／install helper。 |
| MT13 | NOT TESTED | 需 Phase 5 固定 AppId、升級與解除安裝。 |
| MT14 | PARTIAL | 標準桌曆暨時鐘模式與離機作業番茄鐘模式的 Release preview 各 30 分鐘、各 59 次 resize／font 循環通過；實體 DPI 反覆變更與全螢幕 10 Hz 長測未測。 |
| MT15 | NOT TESTED | fresh offline build、static CRT imports 通過；沒有「未裝開發工具／VC++ Redistributable」的乾淨 Windows 10 目標機。 |

Windows 11：`NOT TESTED（依使用者指示延期）`。

## 自動門檻與證據

最終 source state 執行：

```powershell
cargo fmt --check
cargo check --locked --all-targets
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked -- --nocapture
cargo build --release --locked
cargo build --release --locked --offline # fresh CARGO_TARGET_DIR
cargo test --locked --offline -- --nocapture
cargo test --locked --test phase3_native -- --ignored --test-threads=1 --nocapture
cargo test --release --locked --test native_modes -- --ignored --test-threads=1 --nocapture
powershell -NoProfile -File scripts\smoke-test.ps1
```

最終非互動測試共 33 項通過；Phase 3 原生設定閉環 1 項通過；Release 原生 ignored 集合 4 項通過。外部 foreground activation 被 Windows policy 拒絕的分支仍明列 `NOT TESTED`，不把該分支的未執行當成成功。

完整原始輸出、CSV、JSON、PE／imports、manifest、環境與 source hashes 保存在 `docs/evidence/phase4/`。smoke 預設不顯示 UI；`-Interactive` 才執行短暫 `/c`、`/s`、`/p`。

沒有新增 Cargo dependency、外部字型、網路功能或安裝元件。正式 HKCU 設定鍵與本次子程序已清理。Phase 5 才建立 `dist\tools-screensaver-tzk.scr`、Inno Setup、System32 安裝、setcurrent helper、升級／解除安裝與 Windows 設定頁完整驗收。
