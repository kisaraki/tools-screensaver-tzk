# Phase 27：接近全透的氣象玻璃與公開 MIT 授權

產品 v0.15.4；設定 schema 10；2026-09-16；Windows 10 x64；原始碼以 tag `v0.15.4` 固定。

資訊玻璃本體透光係數由 0.85 提高至 0.98，遮色不透明度由 15% 降至 2%；BGR 色調偏移 [1.34, 1.14, 1.0]。Gaussian 近似連續模糊半徑從 round(面板寬 * 0.035)、2～48px 改為 round(面板寬 * 0.006)、1～10px。廣域反射從 0.065 降至 0.018，陰影從 0.22 降至 0.14。背景細節清楚透出，保留曲面折射、細微色散、拋光唇邊、高光及小型面板尺寸；文字排版不變。

根目錄既有 `LICENSE` 是完整標準 MIT 授權，Copyright (c) 2026 kisaraki；Setup 已使用它作為授權頁。新增 Pages 的 `LICENSE.txt` 公開全文與下載連結，內容與根目錄一致。

完整 package 包含 fmt、Clippy、79 個預設 Rust tests、locked Release、19＋15 個安裝政策 checks 及 Inno Setup 編譯，見 [package.txt](evidence/phase27/package.txt)。八種氣象、三種尺寸的 24 張離線 GDI 匯出見 [畫面](evidence/phase27/weather-fixtures/)；代表晴、曇、嵐畫面人工檢查。成品非互動檢查見 [smoke-report.json](evidence/phase27/smoke/smoke-report.json)。

本次未修改網路來源、更新生命週期、設定 schema 或安裝行為。遠端不啟動正式設定、全螢幕、播放器、安裝或 UAC；實際安裝及 Windows 11 仍未驗證。成品未簽章，大小與 SHA-256 見 [README](../README.md)。
