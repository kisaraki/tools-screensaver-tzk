# 視覺參考與實作證據

產品版本：0.1.1<br>
規格文件：v1.2  
更新日期：2026-09-06

## 參考範圍

使用者提供的私有圖片只作為標準桌曆暨時鐘模式的構圖參考：黑底、左側類比鐘、右側月曆、紅色前景及今天圓形標示。圖片沒有公開再授權依據，因此不納入 Git repository 或 Setup。產品沒有複製附件像素；鐘面、六列 Gregorian 月曆、字型縮放、配色和所有 GDI 圖形都由本專案原始碼產生。

Classroom Timer 網址是倒數畫面的需求來源；開發期間沒有取得網頁內容，因此沒有宣稱與網站目前畫面逐像素一致。實作依規格固定為六位七段數字、沙漏、進度線、最後十秒外框和零點四次閃爍。

## 固定 fixture

[Phase 2 視覺檢查](phase2-visuals.md) 記錄各圖的固定日期、時間、倒數值、模式、色票、字型、畫布和 DPI。下列 PNG 由產品使用的同一套 GDI renderer 輸出，並非參考圖：

| 情境 | 證據 |
| --- | --- |
| 參考比例 800×369，標準桌曆暨時鐘模式，四色 | `docs/evidence/phase2/fixtures/00`～`03` |
| 1920×1080／96 DPI，標準桌曆暨時鐘模式 | [04-TimeDate-1920x1080](evidence/phase2/fixtures/04-TimeDate-1920x1080-dpi96-p2-SevenSegment-size.png) |
| 3840×2160／144 DPI，標準桌曆暨時鐘模式 | [05-TimeDate-3840x2160](evidence/phase2/fixtures/05-TimeDate-3840x2160-dpi144-p2-SevenSegment-size.png) |
| 1080×1920／192 DPI，標準桌曆暨時鐘模式 | [06-TimeDate-1080x1920](evidence/phase2/fixtures/06-TimeDate-1080x1920-dpi192-p2-SevenSegment-size.png) |
| 極小 120×80／288 DPI，標準桌曆暨時鐘模式 | [08-TimeDate-120x80](evidence/phase2/fixtures/08-TimeDate-120x80-dpi288-p2-SevenSegment-size.png) |
| 參考比例 800×369，離機作業番茄鐘模式，四色 | `docs/evidence/phase2/fixtures/11`～`14` |
| 1920×1080／96 DPI，離機作業番茄鐘模式 | [15-Countdown-1920x1080](evidence/phase2/fixtures/15-Countdown-1920x1080-dpi96-p2-SevenSegment-size.png) |
| 3840×2160／144 DPI，離機作業番茄鐘模式 | [16-Countdown-3840x2160](evidence/phase2/fixtures/16-Countdown-3840x2160-dpi144-p2-SevenSegment-size.png) |
| 1080×1920／192 DPI，離機作業番茄鐘模式 | [17-Countdown-1080x1920](evidence/phase2/fixtures/17-Countdown-1080x1920-dpi192-p2-SevenSegment-size.png) |
| 極小 120×80／288 DPI，離機作業番茄鐘模式 | [19-Countdown-120x80](evidence/phase2/fixtures/19-Countdown-120x80-dpi288-p2-SevenSegment-size.png) |
| 最後十秒、暗半週期、完成狀態 | `docs/evidence/phase2/fixtures/22`～`24` |

標準桌曆暨時鐘模式的固定 fixture 使用 2023-12-31 12:15:40；正常產品模式仍讀取真實系統時間。fixture 同時驗證六列月曆、星期一為首欄、今天標示、指針連續角度、左右／上下切換及極小空間退化。

## Win10 原生對話框

本機為 Windows 10 Education 22H2 x64、build 19045.6456，雙 3840×2160 顯示器，兩者 144 DPI／150%。

- [設定對話框（150%）](evidence/phase3/config-dialog-win10-150.png)：繁體中文標籤、四種色票、四種字型入口、owner-draw 即時預覽及確定／取消。
- [倒數輸入（150%）](evidence/phase3/countdown-dialog-win10-150.png)：時／分／秒欄位、預填、錯誤列及開始／取消。

上述截圖是 Phase 3 歷史證據。v0.1.1 設定畫面已加入「KOMSMOS TOOLKIT 探真拓知酷」識別，目前由非互動 smoke test 檢查內嵌資源字串；尚未以新的互動截圖補證。

100%／200% 實體對話框、混合 DPI 實體桌面及 Windows 11 沒有可用環境，狀態為 `NOT TESTED`。既有 96／144／192／288 DPI fixture 與三種 preview host DPI context 不能代替這些實機畫面。

## Phase 5 安裝畫面

Setup 已由 Inno Setup 6.7.3 編譯，task 文案和解除安裝提示存在於 `installer/MyDateTimeScreensaver.iss`。本輪 UAC 被取消，未完成安裝精靈、Windows 螢幕保護設定頁列舉或解除安裝提示的實機截圖，因此這些畫面為 `NOT TESTED`，沒有以編譯成功冒充視覺驗證。
