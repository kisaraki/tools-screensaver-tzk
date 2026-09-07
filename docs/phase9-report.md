# Phase 9：日期時鐘與番茄鐘置中版面

日期：2026-09-08<br>
產品：tools-screensaver-tzk v0.5.0 開發候選版<br>
規格：v1.6；原始碼與成品以 Git tag `v0.5.0` 追溯<br>
驗證環境：Windows 10 Education 22H2 x64，build 19045.6456

## 完成內容

「標準桌曆暨時鐘模式」與「離機作業番茄鐘模式」的內容集中在螢幕中央，增加四周留白。當畫布寬至少 640 px 且高至少 360 px，內容區為畫布寬的 64%、高的 60%；初始左右各留 18%、上下各留 20%。相較先前 88% × 82% 的內容區，矩形面積縮小約 47%。此數字指版面容納範圍，並非實際發光像素數。

寬或高低於上述門檻時，預覽仍使用 88% × 82% 的內容區，維持小尺寸可讀性。日本旅行模式沿用原有構圖。防烙印位移繼續在安全範圍內運作。

- `src/layout.rs`：依模式與畫布大小選擇置中內容區，鐘面、月曆、數字、沙漏及進度線一起縮放。
- `tests/time_layout.rs`：增加大型橫向、4K、直向、小預覽及旅行模式的中央區域與尺寸檢查。
- `scripts/export-phase2-fixtures.ps1`：每次匯出使用唯一的 BMP 暫存目錄，避免舊圖混入新的證據。
- README、規格、驗收報告與 GitHub Pages：同步版本、下載、雜湊及新版示意圖。

## 非互動驗證

| 項目 | 結果 | 證據與範圍 |
| --- | --- | --- |
| Release 建置 | PASS | `scripts/build.bat`：fmt、Clippy `-D warnings`、locked tests、locked Release build 均通過 |
| 預設測試 | PASS | 46 passed：lib 26、CLI 8、native noninteractive 2、layout 10；9 個測試預設 ignored |
| GDI 匯出 | PASS | 另外明確執行 ignored fixture test，產生 37 張 PNG；每張非空，所有前景像素均落在指定內容區；[匯出紀錄](evidence/phase9/fixtures.txt) |
| 圖片檢視 | PASS | 檢視日期時鐘與番茄鐘的 1080p、直向畫面及小型預覽，確認中央留白與內容完整；4K 另有同套 renderer 匯出證據 |
| Release smoke | PASS | 版本、PE、內嵌資源、manifest、imports、靜態 CRT、錯誤參數與非安裝路徑 helper 拒絕；螢幕保護 registry 前後一致；[JSON](evidence/phase9/smoke/smoke-report.json) |
| Setup 封裝 | PASS | Inno Setup 6.7.3；版本 0.5.0.0；成品與公開下載副本雜湊一致 |

以上過程沒有開啟正式全螢幕、設定畫面、WebView2 player 或安裝程式，也沒有觸發 UAC 或寫入 System32。圖片由產品 GDI renderer 離屏繪製，不冒充實際全螢幕截圖。

## 畫面證據

| 畫布 | 日期時鐘 | 番茄鐘 |
| --- | --- | --- |
| 1920×1080／96 DPI | [圖片](evidence/phase9/fixtures/04-TimeDate-1920x1080-dpi96-p2-SevenSegment-size.png) | [圖片](evidence/phase9/fixtures/15-Countdown-1920x1080-dpi96-p2-SevenSegment-size.png) |
| 3840×2160／144 DPI | [圖片](evidence/phase9/fixtures/05-TimeDate-3840x2160-dpi144-p2-SevenSegment-size.png) | [圖片](evidence/phase9/fixtures/16-Countdown-3840x2160-dpi144-p2-SevenSegment-size.png) |
| 1080×1920／192 DPI | [圖片](evidence/phase9/fixtures/06-TimeDate-1080x1920-dpi192-p2-SevenSegment-size.png) | [圖片](evidence/phase9/fixtures/17-Countdown-1080x1920-dpi192-p2-SevenSegment-size.png) |
| 320×180／144 DPI | [圖片](evidence/phase9/fixtures/07-TimeDate-320x180-dpi144-p2-SevenSegment-size.png) | [圖片](evidence/phase9/fixtures/18-Countdown-320x180-dpi144-p2-SevenSegment-size.png) |

另外包含四色、字型、120×80、倒數最後十秒／暗半週期／完成，以及兩種旅行場景。

## 成品

| 檔案 | Bytes | SHA-256 |
| --- | ---: | --- |
| `tools-screensaver-tzk.scr` | 808,448 | `08bc4c39579606124abfc76afc86c74d0bdac2dfd61910eb6baf74b9a3fc4e5b` |
| `tools-screensaver-tzk-Setup.exe` | 2,321,648 | `60f773c1d645b2347b82a805398055e65154d01229f3ffd05237fc53701b5eb7` |

成品均為 `NotSigned`；Windows 可能顯示未驗證發行者或 SmartScreen 提示。[v0.5.0 下載頁](https://kisaraki.github.io/tools-screensaver-tzk/#download) 提供 GitHub Pages 公開直連。

旅行來源 HTTP 與 WebView2 Runtime 探測沿用 [Phase 8](phase8-report.md) 的 2026-09-07 紀錄，本次沒有重跑。實際全螢幕觀察、安裝／升級／解除安裝、旅行播放與長時間資源觀察仍依 [驗收報告](acceptance-report.md) 保留未測狀態；Windows 11 依使用者指示延期。
