# Phase 10：Setup WebView2 Runtime 階段

日期：2026-09-08<br>
產品：tools-screensaver-tzk v0.6.0 開發候選版<br>
規格：v1.7；原始碼與成品以 Git tag `v0.6.0` 追溯<br>
環境：Windows 10 Education 22H2 x64，build 19045.6456

## 完成內容

Setup 會檢查電腦層級 Microsoft Edge WebView2 Runtime；已安裝時顯示版本並略過。缺少時預設提供補裝 task，在產品檔案安裝前執行官方 Evergreen Bootstrapper，連網下載並靜默安裝，再驗證 Runtime 是否可偵測。失敗時停止並允許重試或返回取消該選項；只使用離線時鐘者可略過補裝。

Bootstrapper 以固定來源、版本、大小及雜湊鎖定，封裝時驗證 Microsoft Authenticode 簽章。其下載與 Runtime 安裝只屬於 Setup；`.scr` 的網路、提權及缺失 fallback 邊界保持原樣。解除安裝保留共用 Runtime。

詳細流程、來源與維護方式見 [WebView2 部署紀錄](webview2-setup.md)。

## 實際驗證

| 項目 | 狀態 | 證據 |
| --- | --- | --- |
| fmt／Clippy／Release | PASS | `scripts/package.bat` 呼叫既有 build；[package.txt](evidence/phase10/package.txt) |
| 預設 Rust 測試 | PASS | 46 passed；9 個測試預設 ignored |
| Installer policy | PASS | 19 checks；共用 Pascal policy，獨立最低權限 harness，在精靈建立前結束；[installer-policy.txt](evidence/phase10/installer-policy.txt) |
| Runtime registry | PASS | 使用 Setup 共用唯讀函式偵測到電腦層級 `152.0.4191.66` |
| 官方 Bootstrapper | PASS | `1.3.265.7`、1,783,000 bytes、有效 Microsoft Corporation 簽章、鎖定 SHA-256 一致；全新目錄下載與核對通過；[bootstrapper.json](evidence/phase10/bootstrapper.json)；沒有執行 Bootstrapper |
| Inno Setup 封裝 | PASS | 6.7.3；內附引導程式；編譯時雜湊比對通過 |
| SCR 非互動 smoke | PASS | 版本、PE／resources／manifest／imports、靜態 CRT、無 UI 錯誤路徑、helper 拒絕、registry 不變；[JSON](evidence/phase10/smoke/smoke-report.json) |
| GDI 圖片 | PASS（沿用） | v0.5.0 的 [Phase 9](phase9-report.md) 37 張圖片；本次沒有修改 renderer 或重新匯出 |

19 個 policy 案例覆蓋空白／零／格式錯誤／負數／溢位／有效版本，既有／缺少／選用／取消組合，以及成功、啟動失敗、非零結果、code 0 但 Runtime 缺失與 3010／1641 重啟結果。這是版本與結果判斷測試，不是對 Microsoft installer 的端到端測試。

建置紀錄中的 Source revision 是開始建置時的基底 `65b1788`，包含本階段尚未提交的工作樹修改；最後交付版本由 `v0.6.0` tag 鎖定。

## 成品

| 檔案 | Bytes | SHA-256 |
| --- | ---: | --- |
| `tools-screensaver-tzk.scr` | 808,448 | `3f4e1f1ea7e4ef86a8438467483814989356e075d6605f71f541beafdfaf9cf8` |
| `tools-screensaver-tzk-Setup.exe` | 3,980,373 | `f5c2cc3843aa9d35153b4b20ab496d9978e164a944b30813afe0376bf73a6c97` |

兩個產品成品仍為 `NotSigned`；內附的 Microsoft Bootstrapper 有有效 Microsoft 簽章，兩者不可混淆。[公開下載頁](https://kisaraki.github.io/tools-screensaver-tzk/#download) 提供無需登入的成品直連。

## 未測範圍

本輪沒有啟動正式 Setup、Microsoft Bootstrapper、全螢幕或 WebView2 player，沒有觸發 UAC、安裝 Runtime、寫入 System32 或變更螢幕保護設定。實際安裝／升級／解除安裝、Runtime 安裝與重試、斷網／Proxy／企業原則、需重啟及多帳號 UAC 情境仍為 `NOT TESTED`。Windows 11 依使用者指示延期。來源 HTTP 紀錄沿用 Phase 8；完整矩陣見 [驗收報告](acceptance-report.md)。
