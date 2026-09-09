# Phase 19：暗色藥劑瓶玻璃面板

產品版本：0.12.1<br>
規格版本：v2.6<br>
驗證日期：2026-09-09<br>
環境：Windows 10 Education 22H2 x64（build 19045.6456）

## 改良內容

- 移除橫跨面板的上方反光帶與底部暗帶，避免產生巧克力分格感。
- 面板中央改為接近黑色的透明褐色，以上下連續漸層模擬光線穿過暗色玻璃的衰減。
- 左右邊緣加入方向相反的琥珀褐水平漸層，模擬厚瓶壁的折射與集光。
- 外圈簡化為一層深棕瓶身與細銅色唇邊，內圈保留低亮度玻璃輪廓。
- 只在左上保留短而柔和的局部高光，不再使用硬線或大面積實色條。
- 在共用 GDI Canvas 新增有界 `GradientFill` 與圓角 clipping；零尺寸及極小 preview 會略過無效區域。

## 非互動驗證

| 驗證 | 結果 | 證據 |
| --- | --- | --- |
| `scripts\build.bat` | PASS | fmt、Clippy `-D warnings`、59 個預設測試與 locked Release build |
| 極小尺寸與資源循環 | PASS | 0×0～4K、96～288 DPI 與 50 次資源循環測試，包含新 gradient／clip 路徑 |
| GDI 視覺 fixture | PASS | [55 張 fixtures](evidence/phase19/fixtures/)；人工檢視 1920×1080 與 800×369 番茄鐘畫面 |
| Smoke | PASS | [smoke-report.json](evidence/phase19/smoke/smoke-report.json)；v0.12.1、PE、resources、manifest、imports 與靜態 CRT 均符合 |
| Installer policy | PASS | WebView2 19 checks、產品版本 15 checks；未建立精靈、提示或程序，未寫 registry |
| Package | PASS | Inno Setup 6.7.3；SCR 與 Setup 版本一致且均為 NotSigned |

## 成品

| 檔案 | Bytes | SHA-256 |
| --- | ---: | --- |
| `tools-screensaver-tzk.scr` | 9,553,408 | `3c1f9d2dcf43c7351849dcbecd803cbd9b8e124c83173d5c408eb6df5397b500` |
| `tools-screensaver-tzk-Setup.exe` | 12,607,444 | `31ab561567fde1b965af7504bffe111b0f8c84f62ae2ab8cde16e925d357df5c` |

依遠端限制，本階段沒有啟動 Setup、UAC、設定視窗或 `/s` 全螢幕模式。視覺判斷以離屏 GDI 成品為準；實際桌面觀感仍可在可互動 Windows 10 環境補驗。Windows 11 尚未驗證。
