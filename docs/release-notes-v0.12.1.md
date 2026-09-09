# tools-screensaver-tzk v0.12.1

本版重新設計離機作業番茄鐘的深棕玻璃面板，修正 v0.12.0 看起來像巧克力條的問題。

## 變更

- 移除橫跨面板的實色反光帶與底部暗帶。
- 使用平滑 GDI 漸層形成近黑褐透光核心、上下光衰減與左右琥珀色瓶壁折射。
- 簡化外框，只保留深棕瓶身、細銅色唇邊、低亮度內框與短局部柔光。
- 極小 preview 與一般／高 DPI 畫面共用相同幾何與有界 clipping。

## 驗證

- 59 個預設程式測試通過。
- 19 個 WebView2 與 15 個產品版本 installer policy checks 通過。
- 55 張離屏 GDI fixture 已重新匯出；1920×1080 與 800×369 番茄鐘畫面已人工檢視。
- `.scr` 與 Setup 已完成 Release build；兩者目前均未簽章。

## SHA-256

```text
3c1f9d2dcf43c7351849dcbecd803cbd9b8e124c83173d5c408eb6df5397b500  tools-screensaver-tzk.scr
31ab561567fde1b965af7504bffe111b0f8c84f62ae2ab8cde16e925d357df5c  tools-screensaver-tzk-Setup.exe
```

遠端驗證沒有啟動 Setup、UAC 或全螢幕模式。實際桌面視覺仍可在可互動 Windows 10 環境補驗；Windows 11 尚未驗證。
