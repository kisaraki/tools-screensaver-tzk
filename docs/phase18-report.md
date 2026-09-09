# Phase 18：啟動可靠度、游標、眨眼與新視覺

產品版本：0.12.0<br>
規格版本：v2.5<br>
驗證日期：2026-09-09<br>
環境：Windows 10 Education 22H2 x64（build 19045.6456）

## 完成內容

- Setup 的「設為目前的螢幕保護程式並啟用」改為預設勾選。安裝後 helper 以發起安裝的使用者身分設定 `HKCU\Control Panel\Desktop` 的 `SCRNSAVE.EXE`、`ScreenSaveActive=1` 與 `ScreenSaveTimeOut=60`，呼叫 `SystemParametersInfoW` 套用逾時與啟用狀態，逐項讀回核對後送出有界的設定變更通知。既有 `ScreenSaverIsSecure` 不會被改寫。
- 螢幕保護程式在第一個全螢幕表面顯示前隱藏游標；退出、錯誤及視窗全部關閉時還原進入前借用的游標 handle。
- 地方散策的上下眼瞼改成多段透明度漸層。來源切換時上下範圍仍重疊而完全閉眼；定時輕眨與緩衝加深共用柔和邊緣。
- 新增「鐵灰色」固定主色 `RGB(154,160,163)`；registry schema 升為 8，`ColorPreset=7` 只在 schema 8 以上解讀。自動模式每 120 秒循環七種固定色。
- 番茄鐘計時面板改成深棕玻璃視覺：深棕外層、較亮內層、棕銅框、上方反光帶與底部暗帶。

## 非互動驗證

| 驗證 | 結果 | 證據 |
| --- | --- | --- |
| `scripts\build.bat` | PASS | fmt、Clippy `-D warnings`、59 個預設測試與 locked Release build 均完成 |
| Rust 測試 | PASS | lib 39、CLI 8、native noninteractive 2、layout 10；另有 9 個互動／長時間測試預設 ignored |
| Smoke | PASS | [smoke-report.json](evidence/phase18/smoke/smoke-report.json)；版本、PE、manifest、imports、靜態 CRT、設定資源與非法 helper 路徑均符合 |
| WebView2 installer policy | PASS | [policy-report.txt](evidence/phase18/installer-webview2/policy-report.txt)；19 checks，沒有建立精靈、啟動程序、寫 registry 或顯示提示 |
| 產品版本 installer policy | PASS | [policy-report.txt](evidence/phase18/installer-product-version/policy-report.txt)；15 checks，沒有建立精靈、啟動程序、寫 registry 或顯示提示 |
| GDI 視覺 fixture | PASS | [55 張 fixtures](evidence/phase18/fixtures/)；含八個色彩選項、鐵灰色與深棕玻璃番茄鐘面板 |
| Inno Setup package | PASS | `.scr` 與 Setup 版本一致；兩者均為 NotSigned |

## 成品

| 檔案 | Bytes | SHA-256 |
| --- | ---: | --- |
| `tools-screensaver-tzk.scr` | 9,550,848 | `b753e9330741161e267a9f1042bbf6635433dbb5c7a9987a824fb2fe5f43715e` |
| `tools-screensaver-tzk-Setup.exe` | 12,606,717 | `2cd226c69b6eb91afe330a1c86b3462cb99932ec4432809673d65503453e0400` |

## 尚待可互動 Windows 10 驗證

依遠端測試限制，本階段沒有執行正式 Setup、UAC、Microsoft Bootstrapper、`/s` 全螢幕或 Windows 螢幕保護程式設定頁，也沒有改寫本機實際螢幕保護 registry。仍需在可互動 Windows 10 補驗 60 秒閒置準時啟動、群組原則環境、游標隱藏與退出還原，以及地方散策實際影片上的眨眼觀感。Windows 11 依使用者指示未驗證。
