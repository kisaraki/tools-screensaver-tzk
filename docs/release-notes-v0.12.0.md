# tools-screensaver-tzk v0.12.0

本版改善安裝後的螢幕保護程式啟動可靠度，並完成游標、地方散策眨眼、鐵灰色與番茄鐘深棕玻璃面板。

## 變更

- Setup 預設啟用目前螢幕保護程式，設定 `SCRNSAVE.EXE`、`ScreenSaveActive=1` 與 60 秒 `ScreenSaveTimeOut`，並透過 Windows API 套用及通知設定變更。既有登入安全選項維持原值。
- 全螢幕顯示前隱藏滑鼠游標，退出時還原。
- 地方散策的上下眨眼加入漸層柔邊；切換影片時仍會完整閉眼。
- 新增「鐵灰色」主色，自動模式改為每 2 分鐘循環七色。
- 番茄鐘計時面板改成深棕玻璃質感。

## 驗證

- 59 個預設 Rust／原生非互動測試通過。
- 19 個 WebView2 與 15 個產品版本 installer policy checks 通過。
- 55 張 GDI fixture 已匯出並檢查。
- `.scr` 與 Setup 已完成 Release build；兩者目前均未簽章。

## SHA-256

```text
b753e9330741161e267a9f1042bbf6635433dbb5c7a9987a824fb2fe5f43715e  tools-screensaver-tzk.scr
2cd226c69b6eb91afe330a1c86b3462cb99932ec4432809673d65503453e0400  tools-screensaver-tzk-Setup.exe
```

遠端驗證沒有啟動 Setup、UAC、Microsoft Bootstrapper 或全螢幕模式。60 秒閒置啟動、實際游標生命週期與眨眼觀感仍需在可互動 Windows 10 環境確認；Windows 11 尚未驗證。
