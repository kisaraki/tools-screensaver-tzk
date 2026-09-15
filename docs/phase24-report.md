# Phase 24：氣象背景置中留白

產品 v0.15.1；設定 schema 10；日期：2026-09-15；平台 Windows 10 x64。

氣象背景由全螢幕 center-cover 改為保持 1983:793 完整比例，縮放至中央 64% 寬、60% 高區域內，四周純黑。資訊卡依背景高度同步縮小，完整置於背景中央；來源及資料時間移到背景下方。天氣來源、更新排程與設定格式沿用 v0.15.0。

## 驗證與成品

完整 package 的 fmt、Clippy、79 個預設 Rust tests、Release build、19＋15 個安裝政策 checks 及 Inno Setup 封裝通過；[建置證據](evidence/phase24/package.txt)。非互動 smoke 見 [smoke-report.json](evidence/phase24/smoke/smoke-report.json)。

八種背景各產生 1600×900、900×1600、320×180，共 [24 張離線 GDI 畫面](evidence/phase24/weather-fixtures/)。代表橫向、直向及小型畫面已人工檢查，背景完整置中、黑色留白及資訊卡均可見。畫面資料為示意；未重新探測未修改的氣象或更新服務，沿用 [Phase 23](phase23-report.md) 的來源證據。

成品版本及 SHA-256 見 [README](../README.md)。遠端未開啟正式設定、全螢幕、安裝或 UAC；真實閒置／長時間／混合 DPI 與安裝維持 NOT TESTED。Windows 11 尚未驗證；成品未簽章。
