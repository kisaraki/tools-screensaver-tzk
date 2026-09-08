# Phase 12：鐘面留白、擬真旅行窗框與切換設定

日期：2026-09-08<br>
產品：tools-screensaver-tzk v0.8.0 開發候選版<br>
規格：v1.9；原始碼與成品以 Git tag `v0.8.0` 追溯<br>
環境：Windows 10 Education 22H2 x64，build 19045.6456

## 完成內容

鐘面的 12／3／6／9 原本字高為 `0.40R`、中心半徑為 `0.62R`，數字上緣會貼到主刻度內緣 `0.82R`。現在將字高縮至 `0.30R`（縮小 25%）、中心半徑內收到 `0.58R`；一般與自訂字型都依相同數字框量測縮放。整體置中內容區與月曆、番茄鐘面積保持原有設定。

「自在飛行」與「列車旅行」換成專案原創 AI 擬真 PNG，呈現客機艙壁、金屬窗框與暖木質列車。它們是虛構場景，並非實際 A380 或特定列車照片。內建 `image_gen` 工具生成的原檔與完整提示見 [素材紀錄](travel-artwork.md)。PNG 內附於單一 `.scr`；GDI 以 Windows Imaging Component 在記憶體解碼，每張圖片只快取純像素，沒有增加第三方圖片套件或預覽網路請求。WIC 介面在解碼結束前釋放，支援已存在的 STA／MTA。[Microsoft WIC 文件](https://learn.microsoft.com/en-us/windows/win32/api/_wic/)

正式旅行模式將相同圖片寫入本機 player shell 目錄。完整 16:9 player 位於窗孔內，黑色留邊補足不同窗孔比例；caption 位於圖像下方，沒有裁切 player 或覆蓋控制項。離線／設定預覽使用同一圖片的靜態風景，不能當作直播播放證據。

設定新增「來源切換」，可選「不切換」或「每隔」1～1440 整數分鐘，預設 1 分鐘。Registry schema 升為 5，`TravelSwitchMinutes=0` 表示停用定時輪換及預抓；來源故障仍會自動復原。每個螢幕從本次影片首次 `PLAYING` 開始各自計時；長間隔只在最後 1 分鐘預抓下一個來源。舊設定預設 1 分鐘、無效資料回退，超出範圍的 UI 輸入不能保存；按取消不寫入偏好。

## 驗證結果

| 項目 | 狀態 | 證據 |
| --- | --- | --- |
| fmt／Clippy／Release／封裝 | PASS | [package.txt](evidence/phase12/package.txt)；Rust 1.97.1、Inno Setup 6.7.3 |
| 預設 Rust 測試 | PASS | 56 passed：lib 36、CLI 8、native noninteractive 2、layout 10；9 個預設 ignored |
| 新增測試 | PASS | 6 項切換設定／舊 schema／UI 隱藏控制項／不切換故障復原測試，以及 1 項兩張圖片解碼、快取、MTA 生命週期測試 |
| 鐘面間距 | PASS | 擴充既有 GDI 測試，涵蓋小預覽、1080p、4K、直向、細明體與 240pt Arial Black 粗斜體，量測數字框與刻度間距 |
| GDI fixture | PASS | 40 張重新匯出；[fixtures.txt](evidence/phase12/fixtures.txt)、[視覺證據](visual-reference.md) |
| 靜態 HTML 版面 | PASS | 兩場景 × 1600×1000／900×1600 共 4 張 headless Edge 圖片；檢查 player 16:9、位於窗孔內、caption 在圖像外且不超出 viewport；[geometry.json](evidence/phase12/html/geometry.json) |
| Installer policy | PASS | 19 checks；最低權限 harness 在精靈建立前結束；Runtime 唯讀偵測 `152.0.4191.66`，官方 Bootstrapper 雜湊與 Microsoft 簽章有效 |
| SCR smoke | PASS | v0.8.0、PE／resources／manifest／imports、切換設定標籤、靜態 CRT、無 UI 錯誤路徑、helper 拒絕與 registry 不變；[JSON](evidence/phase12/smoke/smoke-report.json) |

HTML fixture 移除全部產品 player script，以靜態色塊表示影片區，使用獨立暫存 Edge profile 並阻擋 host 解析；沒有載入 YouTube、WebView2 player 或可見瀏覽器。左上 2 px 綠色標記是測試注入的幾何檢查結果，產品不含此標記。執行方式：`pwsh -NoProfile -NonInteractive -File scripts/export-travel-shell-fixtures.ps1`。

建置紀錄的 Source revision 為基底 `4c3a7d6`，包含本階段當時尚未提交的修改；最終原始碼由 `v0.8.0` tag 鎖定。來源 HTTP 紀錄沿用 Phase 8 的具體日期，本輪沒有重新探測來源。

## 成品

| 檔案 | Bytes | SHA-256 |
| --- | ---: | --- |
| `tools-screensaver-tzk.scr` | 4,484,096 | `827e0c9b45e4820cb00dc337cac7321bdcb0a3e06ff06f1c2e35c3028f45b281` |
| `tools-screensaver-tzk-Setup.exe` | 7,619,663 | `f92160cb1334db2a707014181916d82810cd876ee45b5d6ee505b334741a3e95` |

產品成品仍為 `NotSigned`，圖片內附使檔案增大。GitHub Release 與 [公開下載頁](https://kisaraki.github.io/tools-screensaver-tzk/#download) 提供相同成品；Pages 直連不需要登入。

## 未測範圍

實際設定視窗的高 DPI 互動、每個螢幕直播 `PLAYING`、自訂間隔／不切換的長時間實機觀察、斷線與 WebView2 資源清理仍為 `NOT TESTED`。靜態照片、原生隱藏控制項和 headless HTML 檢查不能代替這些情境。

本輪沒有啟動全螢幕、正式 Setup、Microsoft Bootstrapper 或 UAC，沒有寫入 System32 或改變系統螢幕保護設定。安裝／升級／解除安裝矩陣與 Windows 11 仍未驗證，詳見 [逐項驗收報告](acceptance-report.md)。
