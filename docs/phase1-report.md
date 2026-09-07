# Phase 1 實作與 Windows 10 驗證報告

- 依據：開發規格 v1.2，Phase 1；軟體版本維持 0.1.0。
- 日期：2026-09-04（Asia/Taipei）。
- 結論：**Phase 1 三條啟動路徑及生命週期已實作，現有環境可執行的建置、負向測試與主要 Win10 整合驗證通過。** 前景切換與部分實機情境仍列未驗證，詳見下表；不宣稱整份產品規格已驗收。
- 平台：Windows 10 Education 22H2 x64，19045.6456；Rust／Cargo 1.97.1、MSVC 2022、Windows SDK，沿用 Phase 0 工具。
- 實機：兩台 3840×2160，皆 144 DPI（150%）；左側矩形 `(-3840,0,0,2160)`，右側 `(0,0,3840,2160)`。
- Windows 11 依使用者指示延期，不列目前完成的阻擋條件。未安裝新元件，未修改系統螢幕保護設定或顯示配置。

## 已實作內容

| 範圍 | 實作與結果 |
| --- | --- |
| 命令列 | `GetCommandLineW`／`CommandLineToArgvW`／`LocalFree`；獨立純 parser；`/`、`-`、大小寫、空白／冒號 HWND；保留完整 64-bit 數值 |
| 錯誤 | 正常／取消 0；參數／預覽 parent 錯誤 2；Win32 初始化或執行錯誤 3；錯誤模式靜默退出，Debug 向 debugger 診斷 |
| `/s` | 列舉有效顯示器並去除重複矩形；先建立隱藏 popup，全部就緒後顯示；topmost／toolwindow、黑色背景；列舉失敗才用檢查過的 virtual screen bounds |
| 關閉 | 共用 session 與 message-only coordinator；單一 UI thread／1 秒維護 timer；關閉請求冪等；停止 timer、清除 surfaces、最後銷毀 coordinator |
| 輸入 | 500 ms 滑鼠／初始化啟用寬限；固定螢幕座標基準，任一軸嚴格大於 4 px 退出；鍵盤、系統鍵、各按鈕及雙向滾輪立即退出 |
| 前景／游標 | 以程序身分區分內部與外部前景；延後檢查並由 timer 補查；`SetCursor(NULL)` 隱藏並恢復借用的游標，不改 `ShowCursor` 計數 |
| 顯示／session | `WM_DPICHANGED` 重查 monitor bounds；`WM_DISPLAYCHANGE` 關閉全組；允許 `WM_QUERYENDSESSION`，`WM_ENDSESSION` 正常清理 |
| `/p` | 直接建立真正 `WS_CHILD | WS_VISIBLE`；記錄 parent HWND／PID／TID；每秒檢查存活及 client size；不搶焦點、不啟用全螢幕輸入退出 |
| 預覽 DPI | 建立與 resize 時匹配 parent DPI context，scope 結束恢復；支援 0×0、1×1 後恢復；空 paint rectangle 不進行填色或 buffer 配置 |
| `/c` | Win32 resource dialog 明示 Phase 1 暫時入口；有效 owner 採 modal，失效／零 owner 採無 owner；在 owner monitor work area 置中，owner 消失取消 |

主要程式：`src/cli.rs`、`error.rs`、`lifecycle.rs`、`monitor.rs`、`native.rs`、`window.rs`、`dialog.rs`。入口與資源已同步更新；只新增 `windows-sys` 的 KeyboardAndMouse API feature，沒有新增 crate 或 dev dependency。

`/c` 的確定與取消目前都不保存設定。Renderer、registry、倒數輸入、`.scr` 封裝及安裝尚未實作，分別留待後續階段。

## 所有權與失敗清理

1. 呼叫端先以 `PendingWindow` 持有 `Box<WindowState>`。`WM_NCCREATE` 成功把指標存入 `GWLP_USERDATA` 才轉移所有權；拒絕建立或安裝指標失敗時仍由呼叫端釋放。
2. 轉移後僅 `WM_NCDESTROY` 清空指標並回收 Box，因此 `WM_CREATE` 回傳 -1 的系統清理也使用同一條路徑。
3. Callback 先複製 role、clone `Rc<Session>`，才呼叫可能同步重入的 Win32 API。視窗清單先建立快照、結束 `RefCell` 借用，再呼叫 `DestroyWindow`。
4. `SessionGuard` 涵蓋中途 `?` 返回與訊息迴圈錯誤。`GetMessageW` 分別處理正值、0 與 -1；正常關閉時最後一個 window state 回收後才送 `WM_QUIT`。
5. 暫時設定 dialog 的狀態由 `DialogBoxParamW` 呼叫端持有至 modal call 返回，`DWLP_USER` 只借用，不重複釋放。Timer 在銷毀時停止。

Win32 單元測試實際觸發 `WM_NCCREATE` 拒絕、`WM_CREATE` 失敗、重複 close，以及 coordinator／既有兩個 surface 建好後下一個 surface 失敗。以 Drop 計數確認共 9 個 state 各回收一次、live 計數歸零，並以 `IsWindow` 檢查先前 handle 已失效。故障注入只存在 `#[cfg(test)]`，Release 不含測試命令或開關。

## 指令與證據

先依 [README](../README.md) 載入 MSVC Developer Shell。

| 操作 | 結果 | 證據 |
| --- | --- | --- |
| `cargo fmt --check` | PASS | [fmt.txt](evidence/phase1/fmt.txt) |
| `cargo check --locked --all-targets` | PASS | [check.txt](evidence/phase1/check.txt) |
| `cargo clippy --locked --all-targets -- -D warnings` | PASS，0 warning | [clippy.txt](evidence/phase1/clippy.txt) |
| `cargo test --locked` | PASS，14 項非互動測試；4 項互動測試預設忽略 | [unit-tests.txt](evidence/phase1/unit-tests.txt) |
| `cargo build --release --locked --offline` | PASS | [release-build.txt](evidence/phase1/release-build.txt) |
| Release 真實程序整合測試 | 4 項完整通過；1 項有未驗證分支 | [native-release-tests.txt](evidence/phase1/native-release-tests.txt) |
| PE／版本／hash／DLL imports | AMD64、Windows GUI、0.1.0；系統 DLL imports | [verification.json](evidence/phase1/verification.json)、[dependencies.txt](evidence/phase1/dependencies.txt) |

14 項非互動測試包含 7 項 library 測試、6 項純 parser 測試，以及 1 項實際啟動程序的非法參數測試（內含 10 組輸入）。包含 UT01～UT03 的 parser 範圍、UTF-16 邊界、負座標與溢位、滑鼠門檻及失敗清理。

Release 驗證明確設定 `SCREENSAVER_TEST_EXE` 指向 Release 執行檔，再執行：

```powershell
cargo test --locked --test native_modes -- --include-ignored --test-threads=1 --nocapture
```

測試 runner 本身是 Debug harness；它啟動的被測程序是 **Release binary**。每個子程序都有有界等待與失敗清理，沒有按程序名稱廣泛終止其他工作。互動測試操作測試 host 與本次 saver 子程序，沒有變更 registry、系統時鐘、DPI 設定或顯示拓撲。

### 真實 Windows 整合結果

| 情境 | 結果與範圍 |
| --- | --- |
| 非法真實命令列 | 10 組皆 code 2，無殘留 app 視窗；包含失效 parent、溢位、全形數字、額外參數、未支援的開發／安裝命令 |
| 無參數與 `/c` | 無 owner、零 owner、失效 owner 正確；取消 code 0 |
| `/c` 有效 owner | owner 關係、modal disable／恢復、monitor work area 置中、確定與 owner 銷毀均通過 |
| 雙 4K `/s` | 兩個可見 surface、矩形精確匹配實際螢幕（含負 X）、style／ex-style 正確；讀取兩個 surface 中心像素皆為黑色 |
| 全螢幕退出 | 11 組真實程序：鍵盤／系統鍵、左中右與 X 按鈕、垂直／水平滾輪、display change、session end、close；各 code 0 並清除全部 surfaces |
| DPI 幾何政策 | 傳送帶有效 RECT 的 `WM_DPICHANGED`，確認採實際 monitor bounds；此為合成訊息，非實際改變系統 DPI |
| `/p` 跨程序嵌入 | 測試 host 與 saver 為不同程序；確認 `GetParent`、child／visible、無 topmost、原點 `(0,0)`、無前景變更、黑色中心像素 |
| 三種 parent awareness | Unaware：host／child 皆 96 DPI；System-aware 與 PMv2：皆 144 DPI；每組 `AreDpiAwarenessContextsEqual` 通過 |
| `/p` resize／退出 | 各 context 依序 320×180 → 480×270 → 0×0 → 1×1 → 320×180；parent 消失後 code 0，child handle 失效；鍵鼠訊息不當成 saver 退出 |
| 同程序／外部前景 | **NOT TESTED**：環境拒絕測試程序的 `SetForegroundWindow`；未繞過 Windows 前景規則，改以 `WM_CLOSE` 清理 |

測試 runner 對上述前景情境採「記錄未測並清理」分支，因此整體輸出為 `5 passed`，**不能解讀成五項行為都已完整驗證**。機器可讀報告另記錄 4 項完整通過、1 項含未驗證分支。

這是目前桌面的互動測試。若使用者在 `/s` 測試期間移動滑鼠或按鍵，產品會依規格提前退出，測試可能失敗；不能用停用產品輸入退出來讓測試通過。

## 成品

- 路徑：`target/x86_64-pc-windows-msvc/release/tools_screensaver_tzk.exe`
- 大小：505,344 bytes。
- SHA-256：`C30C03472586533CBC99066C4C08AB7EAA03186C2746FADCD2CAD9C498828D9D`。
- PE machine `0x8664`，subsystem `2`；FileDescription 明示 Phase 1。
- DLL imports 僅 Windows 系統 DLL；未簽章。乾淨目標機相容性仍待實測。
- 原始碼與文件指紋：[source-sha256.txt](evidence/phase1/source-sha256.txt)。Phase 0 evidence 保留其歷史結果。

## 尚未驗證與下一階段

| 項目 | 狀態／原因 |
| --- | --- |
| 同程序焦點切換不退出、外部前景退出 | 待能取得正常前景操作權限的互動測試環境；目前由程式碼檢查，未宣稱實機通過 |
| 單螢幕 1920×1080／100%、混合 DPI、直向、負 Y | 本次實機為雙 4K／150%；負 Y 只有純幾何測試。未為測試改動使用者顯示配置 |
| 真實移動滑鼠門檻、游標恢復視覺檢查 | 門檻與固定基準有單元測試；未用工具移動使用者實體游標。黑畫面測試不能替代完整人工操作驗證 |
| 顯示器拔插、實際 DPI 變更、登出 | 目前驗證對應合成 Win32 訊息的 handler／清理；真實硬體與 session 情境未測 |
| 顯示列舉 API 失敗後的 virtual-screen fallback | 已實作與檢查幾何合法性，未強制讓本機列舉 API 失敗 |
| Windows 系統螢幕保護設定頁／實際閒置啟動 | 本次使用真正外部 Win32 host；尚未安裝 `.scr`。待後續封裝與產品整合驗證 |
| 乾淨 Win10／30 分鐘資源穩定性 | 尚無乾淨 VM，且正式 renderer 尚未完成；留待產品實機矩陣 |
| Windows 11 | 使用者指定延期，非目前阻擋條件 |

下一個開發階段為 Phase 2：畫面、GDI 資源與純時間邏輯；本次未延伸實作。
