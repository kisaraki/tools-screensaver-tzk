# Setup 的 Microsoft Edge WebView2 Runtime 部署

更新日期：2026-09-08；適用 tools-screensaver-tzk v0.6.0 起。

Setup 為所有使用者安裝螢幕保護程式，因此以電腦層級 Evergreen Runtime 為先決條件。它讀取 32 位元 HKLM registry view 下 `SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}` 的 `pv` 字串。值須可解析為大於 `0.0.0.0` 的版本；Microsoft Edge 瀏覽器本身或提權帳號的個人 Runtime 均不代替此檢查。

## 安裝流程

1. 已有電腦層級 Runtime：在準備安裝摘要顯示版本，略過補裝。
2. 缺少 Runtime：預設勾選 `webview2` task；可取消，只使用兩種離線模式。
3. 使用內附的官方 Evergreen Bootstrapper，以 `/silent /install` 連網下載並安裝適合電腦架構的 Runtime，沿用 Setup 管理員權限，不另外呼叫 `runas`。
4. 程序結束後重新偵測 Runtime。只有 code 0 且偵測成功才繼續；非零失敗、code 0 但仍未偵測到 Runtime，均停止產品安裝並顯示錯誤。3010／1641 要求重啟後重跑 Setup。

若因離線、Proxy 或企業原則失敗，可修復連線後重試，或返回取消 WebView2 task。安裝引導程式不是完整離線 Runtime；完整 Runtime 仍需從 Microsoft 下載。只下載 `.scr` 的使用者需自行準備 Runtime，`.scr` 不安裝任何元件。

解除安裝本程式保留共用 Runtime，也保留使用者偏好及旅行模式本機資料；清理方式見 [README](../README.md#uninstall)。

## Bootstrapper 來源與封裝

| 項目 | 值 |
| --- | --- |
| 檔名 | `MicrosoftEdgeWebview2Setup.exe` |
| 引導程式版本 | `1.3.265.7`（不是下載後 Runtime 的版本） |
| Bytes | 1,783,000 |
| SHA-256 | `17debf797a6c737959bc588236e897936ffac1af5f7e515e674ab32f9edfe719` |
| Authenticode | `Valid`；Microsoft Corporation |
| 來源鎖定 | [webview2-bootstrapper.json](../installer/webview2-bootstrapper.json) |

`scripts/package.bat` 呼叫 `scripts/prepare-webview2.ps1`，從 JSON 中固定的 Microsoft HTTPS URL 取得引導程式，核對大小、版本、SHA-256 與有效 Microsoft 簽章後才允許封裝。快取位於 `target/webview2`；不符合鎖定資料時停止，不自動換成未審查的新檔案。若官方固定 URL 不再可用，維護者需重新核對官方版本並更新鎖定資料。

Inno 編譯時及 Setup 執行引導程式前都再次比對 SHA-256。引導程式只於需要安裝 Runtime 時解壓到 Setup 的暫存目錄，不作為獨立程式安裝進 System32。

Bootstrapper 與 Runtime 由 Microsoft 提供，依 Microsoft 相關條款使用，不屬於本專案 MIT License。Runtime 的下載、更新與本機共用行為依 Microsoft 官方部署機制。

## 非互動驗證邊界

`scripts/test-webview2-installer.ps1` 編譯並執行獨立的最低權限測試程式，與正式 Setup 共用 `webview2-policy.iss`。它在 `InitializeSetup`、wizard 建立之前寫入測試結果並結束，沒有安裝 payload、外部程序、網路連線或 registry 寫入。19 個案例涵蓋版本解析、是否應安裝與安裝結果判斷，另外唯讀探測本機 Runtime。

這些測試不代替完整安裝：實際 Microsoft Bootstrapper 執行、斷網／Proxy、失敗後重試、重新啟動、不同 UAC 帳號與精靈畫面仍為 `NOT TESTED`；遠端不執行正式 Setup 或 Runtime installer。

## 官方依據

- [Microsoft：Runtime 偵測、Bootstrapper 與 silent／per-machine 部署](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/distribution)
- [Microsoft：WebView2 下載](https://developer.microsoft.com/en-us/microsoft-edge/webview2/)
- [Inno Setup：PrepareToInstall 與初始化事件](https://jrsoftware.org/ishelp/topic_scriptevents.htm)
- [Inno Setup：Exec 與等待子程序結束](https://jrsoftware.org/ishelp/topic_isxfunc_exec.htm)
