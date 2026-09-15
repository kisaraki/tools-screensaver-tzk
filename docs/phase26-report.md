# Phase 26：氣象玻璃透明度調整

產品 v0.15.3；設定 schema 10；日期：2026-09-16；Windows 10 x64；原始碼以 tag `v0.15.3` 固定。

玻璃本體遮色不透明度由 30% 降至 15%，即減半；背景透光係數由 0.70 增至 0.85，色調偏移由 BGR [20, 17, 15] 減半至 [10, 8.5, 7.5]。廣域反射強度由 0.13 降至 0.065；保留 Gaussian 柔化、面板大小、曲面折射、色散及光學邊緣。文字不變。

完整 package 包含 fmt、Clippy、79 個預設 Rust tests、locked Release、19＋15 個安裝政策 checks 及 Inno Setup 編譯，見 [package.txt](evidence/phase26/package.txt)。沿用既有 renderer 尺寸與資源循環測試；八種氣象、三種尺寸的 24 張離線 GDI 畫面見 [fixtures](evidence/phase26/weather-fixtures/)，代表晴、曇、嵐畫面人工檢查。非互動成品檢查見 [smoke-report.json](evidence/phase26/smoke/smoke-report.json)。

本次只修改玻璃本體遮色及廣域反射係數；網路來源、更新生命週期、設定 schema 與安裝邏輯沒有修改。遠端不啟動正式設定、全螢幕、播放器、安裝或 UAC；實際安裝、Windows 11 仍未驗證。成品未簽章，大小與 SHA-256 見 [README](../README.md)。
