# Phase 29：設定品牌、產品版本與作者標示

產品 v0.15.6；設定 schema 10；日期：2026-09-16；Windows 10 x64；原始碼以 tag `v0.15.6` 固定。

設定對話框底部第一列由舊有 `KOMSMOS TOOLKIT` 更新為 `KOSMOS TOOLKIT tools-screensaver-tzk v0.15.6`；第二列由 `探真拓知酷` 更新為 `探真拓知酷 作者：水清見底謂之湜`。兩列寬度擴至 202 dialog units，右側距按鈕 6 units。

版本沒有硬編碼在 RC：`build.rs` 從 Cargo version 產生完整 `APP_BRAND_STR`，資源編譯器再寫入 `RT_DIALOG`。後續升版會自動顯示同一版號。非互動 smoke 從成品內嵌資源讀回並核對完整品牌、版本與作者字串。

完整 package 包含 fmt、Clippy、80 個預設 Rust tests、locked Release、19＋15 個安裝政策 checks 與 Inno Setup 編譯，見 [package.txt](evidence/phase29/package.txt)。成品非互動檢查見 [smoke-report.json](evidence/phase29/smoke/smoke-report.json)。本階段未修改畫面模式、網路或安裝行為。

依遠端測試限制，未開啟設定視窗或進行 UAC／正式安裝；文字截斷尚未以互動截圖驗證。Windows 11 尚未驗證，成品未簽章，大小及 SHA-256 見 [README](../README.md)。
