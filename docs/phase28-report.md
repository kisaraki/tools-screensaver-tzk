# Phase 28：半面積資訊區與氣象背景動畫

產品 v0.15.5；設定 schema 10；日期：2026-09-16；Windows 10 x64；原始碼以 tag `v0.15.5` 固定。

中央資訊面板邊長由 min(螢幕寬 26%, 背景高 70%) 改為 min(螢幕寬 18.4%, 背景高 49.5%)，約為 v0.15.4 的 70.7%，面積約 50%。連續模糊半徑由 round(面板寬 * 0.006)、1～10px 降為 round(面板寬 * 0.0025)、1～4px。98% 玻璃本體透光、曲面折射、RGB 色散、方向高光與反射保留。

八種背景加入每 500ms 更新的簡單循環動畫：晴天光芒旋轉；曇與強風有水平流線；雨、土砂降り及嵐有不同密度的斜雨；雪與吹雪有不同速度及密度的雪粒。位置由 monotonic tick 與固定雜湊計算，所有圖元保持在背景矩形內。靜態背景與玻璃仍使用既有快取，不因動畫每幀重新合成；文字最後繪製。

完整 package 包含 fmt、Clippy、80 個預設 Rust tests、locked Release、19＋15 個安裝政策 checks 及 Inno Setup 編譯，見 [package.txt](evidence/phase28/package.txt)。八種氣象、三種尺寸的 24 張離線 GDI 畫面見 [fixtures](evidence/phase28/weather-fixtures/)；晴、曇、強風、雨、嵐及雪代表畫面已人工檢查。非互動成品檢查見 [smoke-report.json](evidence/phase28/smoke/smoke-report.json)。

遠端沒有啟動正式設定、全螢幕、安裝或 UAC。動畫長時間運作、真實多螢幕與混合 DPI 仍待互動環境驗證；Windows 11 尚未驗證。成品未簽章，大小及 SHA-256 見 [README](../README.md)。
