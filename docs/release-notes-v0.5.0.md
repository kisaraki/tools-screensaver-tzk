# tools-screensaver-tzk v0.5.0

日期時鐘與番茄鐘改為較精簡的置中版面，四周保留更多空間，讓桌面觀看更舒適。

- 一般桌面：內容收進中央 64% 寬、60% 高的區域，初始左右各留 18%、上下各留 20%。版面容納面積較前版縮小約 47%。
- 小型預覽：寬小於 640 px 或高小於 360 px 時保持原有比例，確保文字可讀。
- 日期時鐘、月曆、倒數數字、沙漏與進度線一起縮放；日本旅行模式延續既有「自在飛行」與「列車旅行」構圖。

## 公開下載

下列連結由 GitHub Pages 提供，不需要 GitHub 帳號、登入或身分驗證。

- [tools-screensaver-tzk-Setup.exe：Windows x64 安裝程式](https://kisaraki.github.io/tools-screensaver-tzk/downloads/v0.5.0/tools-screensaver-tzk-Setup.exe)
- [tools-screensaver-tzk.scr：獨立螢幕保護程式](https://kisaraki.github.io/tools-screensaver-tzk/downloads/v0.5.0/tools-screensaver-tzk.scr)
- [SHA256SUMS.txt：成品雜湊](https://kisaraki.github.io/tools-screensaver-tzk/downloads/v0.5.0/SHA256SUMS.txt)

| 檔案 | Bytes | SHA-256 |
| --- | ---: | --- |
| `tools-screensaver-tzk.scr` | 808,448 | `08bc4c39579606124abfc76afc86c74d0bdac2dfd61910eb6baf74b9a3fc4e5b` |
| `tools-screensaver-tzk-Setup.exe` | 2,321,648 | `60f773c1d645b2347b82a805398055e65154d01229f3ffd05237fc53701b5eb7` |

## 驗證狀態

Windows 10 x64 的 fmt、Clippy、46 個預設測試、Release 建置、37 張 GDI 圖片匯出、非互動 smoke 與 Inno Setup 封裝均通過。新版圖片涵蓋 1080p、4K、直向與小型預覽。

此版本為未簽章的開發候選版，Windows 可能顯示 SmartScreen 或未驗證發行者提示。遠端驗證沒有觸發 UAC；實際安裝／升級／解除安裝、旅行影片播放與輪換、長時間資源觀察及 Windows 11 仍未完成驗證。來源與 Runtime 探測沿用 v0.4.0 的具日期紀錄。

詳見 [Phase 9 報告](https://github.com/kisaraki/tools-screensaver-tzk/blob/v0.5.0/docs/phase9-report.md) 與 [驗收報告](https://github.com/kisaraki/tools-screensaver-tzk/blob/v0.5.0/docs/acceptance-report.md)。
