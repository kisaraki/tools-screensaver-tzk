# Phase 4 FFI 與資源所有權稽核

日期：2026-09-05  
範圍：`src/` 正式程式路徑；`#[cfg(test)]` 測試輔助另由測試門檻約束  
結論：未發現尚未處理的 FFI lifetime、重入或 owned handle 洩漏

## 稽核方法

- 逐一檢查 166 個 `unsafe {}` 呼叫區塊及 4 個 Win32 callback，對照 API 的輸入、輸出、錯誤值、同步性與清理契約。
- 原始碼共有 146 個 `// SAFETY:` 說明；一個說明可涵蓋同一區塊內數個緊鄰 API，因此不以註解數量冒充一對一證明。
- `unsafe_op_in_unsafe_fn = "deny"`、Release `panic = "abort"`、Clippy `-D warnings` 持續作為機械門檻。
- 搜尋確認沒有 `static mut`。正式路徑沒有 `unwrap()`、`expect()`、`panic!`、`todo!`、`unimplemented!` 或 `unreachable!`；搜尋到的斷言與 unwrap 均位於 `#[cfg(test)]`。

## 所有權與清理對照

| 區域 | 資源／指標 | 所有權與失敗路徑 |
| --- | --- | --- |
| `cli.rs` | `CommandLineToArgvW` | `OwnedArgs::drop` 以 `LocalFree` 配對；先驗證 count 與各指標，轉成 owned `OsString` 後才釋放。 |
| `registry.rs` | `HKEY` | `Key` 只包裝成功的 `RegOpenKeyExW`／`RegCreateKeyExW` 結果，`Drop` 使用 `RegCloseKey`；讀取有 4,096-byte 上限及三次 `ERROR_MORE_DATA` 重試。 |
| `monitor.rs` | `HMONITOR` | 視為 OS borrowed handle，不關閉；callback 僅在同步 `EnumDisplayMonitors` 期間借用 stack `Vec`。 |
| `window.rs` | window class | `WindowClass` 成功註冊後才建立 guard；local drop 順序先由 `SessionGuard` 銷毀所有 HWND，再 `UnregisterClassW`。 |
| `window.rs` | `WindowState` raw pointer | caller 在 `CreateWindowExW` 期間持有 `PendingWindow`；只在 `WM_NCCREATE` 成功後將一個 `Box` 交給 `GWLP_USERDATA`，`WM_NCDESTROY` 先清空 pointer 再唯一回收。`WM_NCCREATE`／`WM_CREATE` 失敗測試覆蓋 transfer 前後。 |
| `window.rs` | 多視窗 group | `SessionGuard` 處理 coordinator、部分建立的 surfaces、timer 與 cursor；`closing`／`Shutdown` 讓重複 close request 冪等，最後一個 HWND 才 `PostQuitMessage`。 |
| `window.rs` | timer | 保存 `SetTimer` 的非零 ID；cadence 更新替換同一 ID；所有結束路徑由 `close_all` 配對 `KillTimer`。 |
| `window.rs` | cursor | `SetCursor(NULL)` 不改全域 ShowCursor count，只保存 borrowed previous cursor；group 結束時恢復，不刪除。 |
| `dialog.rs` | dialog state | `DialogBoxParamW` 是同步 modal call；heap state 在整段呼叫期間由 caller 持有，`DWLP_USER` 只借用，`WM_NCDESTROY` 清空。owner 用 HWND／PID／TID identity 每秒重驗。 |
| `dialog.rs` | owner-draw HDC | `DRAWITEMSTRUCT` 僅在同步 `WM_DRAWITEM` 使用；renderer 以 `SaveDC`／`RestoreDC` 保護 viewport、clip、brush、pen 與 font 狀態。 |
| `dialog.rs` | `ChooseFontW` | `CHOOSEFONTW`／`LOGFONTW` stack storage 覆蓋完整 modal call；成功後重新驗證 face、UTF-16、weight、boolean 與 point size，取消立即讀 `CommDlgExtendedError`。 |
| `gdi.rs` | `HGDIOBJ` | `Object` 僅包 owned font／pen／bitmap，`Drop` 使用 `DeleteObject`；stock objects 不進 wrapper。`Selection` 在 owned object 刪除前恢復原 object。 |
| `gdi.rs` | memory DC／bitmap | `Buffer::new` 的每個中途失敗分支都清除已建立資源；`Drop` 明確先選回舊 bitmap，再刪 bitmap 與 DC。 |
| `gdi.rs` | saved DC state | `SavedDc` 成功後在所有 `?`／return 路徑以 `RestoreDC` 配對。 |
| `render.rs` | paint DC | `Paint` 將 `BeginPaint`／`EndPaint` 包成 RAII；初始化用的 `GetDC` 由局部 `WindowDc` 配對 `ReleaseDC`。 |
| `render.rs` | cache replacement | fonts、pens 沒有跨 frame 保持 selected；key 改變時清空 cache。50 次真實 GDI 建立／釋放測試由 1 回到 1 個 process GDI object。 |

## 重入與錯誤路徑

- `DestroyWindow`、`SetWindowPos`、`BeginPaint`、`DialogBoxParamW` 等可能送出同步訊息前，不保留 `RefCell` borrow 或 `&WindowState`。視窗 callback 先複製 `Rc`／role；renderer 由 `Option` 暫時移出，重入 paint 使用黑色 fallback，返回後再放回。
- `close_all` 先複製 HWND list 再釋放 borrow，才逐一 `DestroyWindow`。coordinator 最後銷毀；同步 `WM_NCDESTROY` 再進入 cleanup 時由 `closing` 阻止重複銷毀。
- `SetWindowLongPtrW` 在呼叫前 `SetLastError(0)`，只在回傳 0 且新 error 非 0 時判失敗。
- 有 `GetLastError` 契約的失敗立即轉為 `AppError`；Registry 使用 `LSTATUS`，ChooseFont 使用 `CommDlgExtendedError`。
- 全螢幕 surface、preview child、timer、message loop 或繪圖失敗都匯入同一 group shutdown；`GetMessageW == -1` 會保留原始 error 並由 guard 清理。

## 本階段修正

| ID | 發現 | 修正 |
| --- | --- | --- |
| FFI-01 | `REG_DWORD` 在長度檢查後仍以 `expect` 轉換。 | 改為 `try_into().ok()?`，損壞資料只觸發欄位 fallback。 |
| FFI-02 | 靜態預設字型透過驗證函式後使用 `expect`。 | 直接建構已知安全的私有 `FontSpec` 欄位，正式路徑不含 panic。 |
| BUILD-01 | 不同 target 目錄的 PE timestamp／CodeView GUID 造成 hash 不同。 | x64 MSVC linker 加入 `/Brepro`；一般與 fresh offline build 的最終 hash 相同。 |

## 實測佐證

- `window::tests::real_creation_failures_and_partial_group_cleanup_drop_each_state_once`：覆蓋 `WM_NCCREATE` 前失敗、`WM_CREATE` 後失敗與多 surface 部分建立清理。
- `render::tests::gdi_small_sizes_fit_and_fifty_resource_cycles_release_objects`：實際 GDI 小尺寸、font／buffer cache 50 次循環，process GDI objects 回到穩定基線。
- Release 原生測試：兩個 4K／144 DPI surface、十一種關閉訊息、preview 三種 DPI context 與反覆 resize。
- Release 30 分鐘觀察：標準桌曆暨時鐘模式（`TimeDate`）與離機作業番茄鐘模式（`Countdown`）各 59 次 size／font／color cache 重建，GDI、USER 與 private bytes 無持續成長；詳見 [Phase 4 報告](phase4-report.md)。
