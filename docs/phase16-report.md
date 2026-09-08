# Phase 16：安裝版本衝突處理

日期：2026-09-09<br>
產品：tools-screensaver-tzk 0.10.2<br>
規格：v2.3

## 完成內容

- Setup 以既有固定 AppId 的 64-bit uninstall registry key 偵測已安裝產品與 `DisplayVersion`。
- 未安裝時直接繼續；同版允許修復安裝，不建立第二個解除安裝項目。
- 不同版本的一般安裝會顯示已安裝版本與本版版本，詢問是否先移除舊版。拒絕時中止。
- 同意後，Setup 在正式安裝前以既有 uninstaller 的 `/VERYSILENT /SUPPRESSMSGBOXES /NORESTART` 執行移除，等待結束並確認 uninstall key 消失後才繼續。
- 找不到 uninstaller、移除失敗或 uninstall key 仍存在時停止安裝；靜默安裝遇到版本衝突也停止。個人 HKCU 偏好與目前螢幕保護設定仍由既有解除安裝政策保留。

## 非互動驗證

新增共用 Pascal policy 與最低權限 harness，涵蓋版本正規化、較舊／較新／異常版本、未安裝、同版修復、同意／拒絕移除、靜默衝突與 uninstall command 解析共 15 項。harness 在 `InitializeSetup` 結束，不建立 wizard、不顯示提示、不執行 uninstaller、不寫 registry，也不觸發 UAC。[測試輸出](evidence/phase16/product-version-policy.txt)

`build.bat` 的 58 個預設測試、封裝後 smoke、19 個 WebView2 policy checks、15 個產品版本 policy checks 及 Inno Setup 6.7.3 package 均為 `PASS`。

| 成品 | Bytes | SHA-256 |
| --- | ---: | --- |
| `tools-screensaver-tzk.scr` | 9,542,656 | `e1f414cc830d976771b503aa5c1f52b5d91ffaed1cfc8adc67109dbfd0f9cc2a` |
| `tools-screensaver-tzk-Setup.exe` | 12,604,353 | `a33e1a8269242091abb810f3af19bd71a59a30e2c8ace7ab52b3632c22ae6305` |

本機唯讀偵測到既有 `0.10.0.0` uninstall metadata，未執行移除。實際從舊版升級、拒絕提示、同版修復、解除安裝失敗、需重新啟動及 UAC 流程仍為 `NOT TESTED`。
