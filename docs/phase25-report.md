# Phase 25：縮小氣象面板與擬真玻璃

產品 v0.15.2；設定 schema 10；日期：2026-09-15；平台 Windows 10 x64。原始碼由 tag `v0.15.2` 鎖定。

資訊面板邊長由背景高度的 84% 改為 70%，另限制螢幕寬度的 26%；一般尺寸的邊長約減少 17%、面積約減少 31%。背景繼續保持完整比例，置中於 64% 寬、60% 高區域內，四周純黑。

## 擬真玻璃

原本四個離散取樣點會保留背景方塊；改為三次水平／垂直連續模糊，近似 Gaussian 柔化。資訊面板內採平滑雙線性取樣，不帶像素方塊質感。

圓角曲面法線驅動邊緣取樣位移及細微 RGB 色散，形成折射；另合成依光源方向變化的反射、拋光唇邊、內緣亮光、柔和投影與半像素邊界混色。這是原生像素合成的動態玻璃視覺效果，不是內附靜態貼圖。玻璃及暫存緩衝合計上限 16 MiB，只在天氣或畫面尺寸變化時重建。文字移除四向描邊，採 Consolas／Microsoft JhengHei fallback。

## 驗證

fmt、Clippy、79 個預設 Rust tests、locked Release、19＋15 個 installer policy checks 與 Inno Setup package 通過，見 [package.txt](evidence/phase25/package.txt)。八種氣象 × 橫向、直向、小型共 [24 張離線 GDI 畫面](evidence/phase25/weather-fixtures/)；代表晴、曇、嵐、直向與小型畫面已人工檢查。renderer 的極小尺寸與 50 次資源循環沿用預設測試。非互動成品 smoke 見 [smoke-report.json](evidence/phase25/smoke/smoke-report.json)。

資料來源、城市保存、自動及手動更新沒有修改，沿用 Phase 23 的服務探測。遠端未啟動正式設定、全螢幕、播放器、安裝或 UAC。真實混合 DPI、長時間閒置與安裝仍 NOT TESTED；Windows 11 尚未驗證。成品未簽章，hash 與大小見 [README](../README.md)。
