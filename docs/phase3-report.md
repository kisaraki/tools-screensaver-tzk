# Phase 3 驗證報告

日期：2026-09-05  
規格：`MyDateTimeScreensaver_Codex_Spec.md` v1.2（修訂版）  
範圍：設定、系統字型、registry、即時預覽與倒數輸入；未進入 Phase 4／5

## 結果

Phase 3 產品程式與自動測試已完成。`/c` 使用正式 Win32 RC 對話框；`/p` 每秒重讀設定並保持合法完整的 `AppConfig`；`/s` 使用啟動快照，離機作業番茄鐘模式在建立任何全螢幕 surface 前先取得並保存本次時間。正式使用者設定鍵未被測試留下。

本次必要平台為 Windows 10 x64。Windows 11 依使用者指示延期，未宣稱通過。Windows 系統閒置啟動／安全桌面 MT08 因尚未安裝 `.scr` 且沒有隔離的系統閒置測試環境，記為 `NOT TESTED`；直接執行 `/s` 的結果沒有冒充 MT08。

## 實作對照

- [config.rs](../src/config.rs)：`AppConfig`、`ConfigDraft`、`ColorPreset`、enum 明確整數轉換、逐欄 fallback、schema 2、schema-last 提交與 rollback。
- [registry.rs](../src/registry.rs)：HKCU adapter、read-only open、寫入時才建立 key、型別／大小兩階段讀取、`ERROR_MORE_DATA` 有界重試、owned `HKEY` RAII。
- [font.rs](../src/font.rs)：92-byte `LOGFONTW`、face／UTF-16／weight／boolean／point size 驗證；未知 SDK 欄位正規化；依實際像素高度重建水平字型。
- [dialog.rs](../src/dialog.rs) 與 [resources.rc](../resources/resources.rc)：繁體中文 `DIALOGEX`、tab／radio group、owner 生命週期、`SS_OWNERDRAW` 共用 renderer、標準桌曆暨時鐘模式（`TimeDate`）每秒更新、固定 `00:05:00`／ratio 0.5 的離機作業番茄鐘模式預覽、`ChooseFontW`、倒數欄位驗證與保存失敗提示。
- [window.rs](../src/window.rs)：`/p` 一秒 poll、離機作業番茄鐘模式 preview 上次時間／滿量沙漏、`/s` 不動態換設定、接受倒數後才建立隱藏 surface、共用 deadline 建立後才顯示。
- [phase3_native.rs](../tests/phase3_native.rs)：專用 Debug HKCU 子鍵的真實跨程序設定／preview／倒數／全螢幕閉環；Release 不接受測試鍵覆寫。

設定提交只處理模式、顏色、字型及 schema，不覆蓋另一程序更新的倒數秒數；倒數提交只處理 `LastCountdownDurationSeconds` 及 schema。中途失敗會倒序回復已改欄位，不刪除未知 value。rollback 本身失敗時會提示「部分設定可能已變更」、立即重讀實際設定並保留草稿。

## Win10 實機與視覺結果

驗證主機：Windows 10 Education 22H2 x64，build 19045.6456；兩台 3840×2160 顯示器，均為 144 DPI／150%，左側螢幕使用負 X 座標。

- [設定對話框 150%](evidence/phase3/config-dialog-win10-150.png)：繁中標籤、四個顏色 radio、固定四項字型 combo、系統字型按鈕、owner-draw 預覽及確定／取消均完整，未見重疊或裁切。
- [離機作業番茄鐘模式輸入 150%](evidence/phase3/countdown-dialog-win10-150.png)：`00:05:00` 預填、小時欄全選、開始／取消與錯誤列空間完整。

100% 與 200% 的實體 dialog 顯示器本次無法提供，記為 `NOT TESTED`；RC 使用 `DS_SHELLFONT`，layout 的 96／192／288 DPI 與極小尺寸仍由既有幾何及實際 GDI 測試覆蓋。這不取代之後的混合 DPI 實機矩陣。

## UT20～UT24

| ID | 結果 | 證據 |
| --- | --- | --- |
| UT20 | PASS | 缺值、錯誤型別、非 4-byte DWORD、91-byte LOGFONT 逐欄 fallback；registry 讀取有 4096-byte 上限。 |
| UT21 | PASS | schema 缺失／1／2、損壞／0、future 3；future 可讀已知欄位但設定與倒數寫入均被拒絕。 |
| UT22 | PASS | 179／180／2400／2401、未終止 face、非法 UTF-16、`i32::MIN`，以及 SDK 欄位安全正規化。 |
| UT23 | PASS | invalid draft 與 ChooseFont cancel 保存呼叫為 0；原生 `/c` cancel 不建立 value。 |
| UT24 | PASS | 注入第 N 次寫入失敗，驗證 rollback 成功／失敗的不同狀態及未知 value 保留。 |

## MT05～MT08

| ID | 結果 | 證據與限制 |
| --- | --- | --- |
| MT05 | PARTIAL | 真實 `/p` host 保持開啟，另一 `/c` 保存後 2 秒內刷新；取消不寫入。Windows「螢幕保護程式設定」頁因尚未封裝／安裝 `.scr`，實際頁面調度為 `NOT TESTED`。 |
| MT06 | PASS | Release `/p` 在 unaware／system-aware／PMv2 host 跟隨 480×270、0×0、1×1、320×180 resize；parent 關閉後程序退出。 |
| MT07 | PASS | 原生流程驗證 1 秒、5 秒、30 分鐘、99:59:59，保存、下次預填、只改所屬欄位、取消不啟動；ceil／最後十秒／四次閃爍由既有純時間測試覆蓋。 |
| MT08 | NOT TESTED | 缺已安裝 `.scr` 及可控制的 Windows 閒置／安全桌面環境。手動 `/s` 已測，但不能代替本項。 |

## 執行的 gate

以下命令均使用 MSVC x64 Developer Shell：

```powershell
cargo fmt --check
cargo check --locked --all-targets
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked -- --nocapture
cargo test --locked --test phase3_native -- --ignored --test-threads=1 --nocapture
cargo build --release --locked
cargo test --release --locked --test native_modes -- --ignored --test-threads=1 --nocapture
```

非互動測試共 33 項通過；Phase 3 原生閉環 1 項通過；Release 原生 ignored 集合 4 項通過，其中 Windows 拒絕外部 foreground activation 的分支由測試明列 `NOT TESTED`。全螢幕測試確認兩個 3840×2160／144 DPI surface 覆蓋 `(0,0)` 與 `(-3840,0)`，並對鍵盤、滑鼠、display change、session end 及 close 清理。

沒有新增 Cargo 相依性、外部字型、網路功能或安裝元件。Phase 3 截圖由 [capture-phase3-dialogs.ps1](../scripts/capture-phase3-dialogs.ps1) 在專用 Debug 測試鍵產生並清理；未更改系統時間、顯示設定、目前螢幕保護程式或正式偏好。

## 後續範圍

Phase 4 才執行完整 FFI／資源稽核、30 分鐘 Release 資源與效能觀察、混合 DPI／更多顯示拓撲、離線乾淨機相容性與 smoke script。Phase 5 才建立 `.scr`、Setup、安裝／解除安裝與 Windows 設定頁完整整合驗收。
