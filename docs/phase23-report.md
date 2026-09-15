# Phase 23：即時氣象與版本更新

產品版本：v0.15.0；規格 v2.10；設定 schema 10；日期：2026-09-15。

平台：Windows 10 x64；原始碼以 tag `v0.15.0` 鎖定。Windows 11 依使用者要求延期。

## 實作

- 新增第四主模式「即時氣象模式」，使用使用者提供的八張背景及中央玻璃卡；顯示城市、攝氏氣溫、天氣、日期時間、濕度、風速、來源與資料時間。
- 城市設定涵蓋九個國家／地區 43 城市，預設臺北；支援正式英文城市名及 IP 約略定位，定位失敗回退保存城市。臺灣使用 CWA 最近的已支援測站觀測，其他地區使用 Open-Meteo 目前天氣模型；自訂名稱以 GeoNames 搜尋並限制所選國家。
- 正式啟動重新抓取，請求完成後每 60 分鐘更新；背景單一工作共享多螢幕結果。首次錯誤不顯示假數值，後續錯誤明示保留上次資料。觀測超過兩小時拒收。
- 晴、曇、強風、雨、土砂降り、嵐、雪、吹雪採雨量／風速與現象映射。JMA 沒有通用全球的嵐／吹雪警報數值，本程式明確將其視為背景分類。
- 新增預設關閉的「自動更新」與即時「手動更新」。每日首次未鎖定正式啟動以 mutex＋HKCU 日期 gate 檢查 GitHub 正式 Release；新版先退出 saver、恢復游標，再詢問。確認後下載固定 Pages Setup，核對 SHA-256，啟動既有互動安裝流程。
- schema 10 保存氣象及更新偏好，舊版遷移回退預設；倒數保存保留新欄位；未知未來 schema 不覆寫。

來源與隱私見 [氣象規格](weather-mode.md)，生命週期與發布約束見 [更新規格](automatic-updates.md)。

## 非互動驗證

| 項目 | 結果 | 證據 |
| --- | --- | --- |
| fmt、Clippy、locked tests、Release、Inno Setup | PASS | [package.txt](evidence/phase23/package.txt) |
| 預設 Rust tests | PASS | 79 passed：lib 59、CLI 8、native 2、layout 10；12 ignored |
| 安裝 policy harness | PASS | 19 個 WebView2＋15 個產品版本 checks，wizard 建立前退出，無 UAC／程序執行／registry 寫入 |
| schema 10 恢復與失敗回復 | PASS | 全新 Registry adapter 讀回實際專用 HKCU test key 的新偏好及五組來源；逐欄故障 rollback；正式 key 未變更 |
| 原生 WinHTTP 氣象連線 | PASS | [weather-network.txt](evidence/phase23/weather-network.txt)、[weather-probe.json](evidence/phase23/weather-probe.json)：臺北 CWA、東京、自訂 Yokohama、IP 座標驗證 |
| 更新的公開檢查及 Setup 下載 | PASS | [update-network.txt](evidence/phase23/update-network.txt)：以舊公開 v0.14.1 驗證固定路徑下載與 SHA-256，立即刪除測試下載，未執行 |
| 八種氣象 × 三種尺寸 | PASS | [24 張 GDI fixtures](evidence/phase23/weather-fixtures/)；1600×900、900×1600、320×180；圖內明示離線示意資料 |
| 小尺寸、字型與 GDI 50 次資源循環 | PASS | 原有 renderer 稽核加入 Weather，極小畫面略過無法閱讀的資訊文字 |
| PE／版本／五個 dialog／imports／錯誤 CLI | PASS | [smoke-report.json](evidence/phase23/smoke/smoke-report.json)；Windows 螢幕保護設定前後不變 |

CWA CDN 的壓縮回應曾令 Win10 原生 WinHTTP 讀取失敗；改為明確 TLS 1.2、識別本專案的相容 User-Agent 及 identity encoding 後，上述實際氣象探測通過。未降低憑證驗證或寫入機器 TLS 設定。

## 尚未實機驗證

以下為 `NOT TESTED`：設定視窗互動及低解析度 DPI、真實閒置啟動、氣象每小時刷新／斷線復原／多螢幕、更新提示的接受與取消／UAC／異版移除／重裝、鎖定與登入切換。遠端測試未啟動 `/s`、正式 `/c`、播放器或 Setup，也未修改正式 HKCU／System32。公開來源 HTTP 可達不能代替完整 UI 驗收。

## 成品

成品版本一致且未簽章；大小與 SHA-256 以同版本的 [README](../README.md) 及 `dist/SHA256SUMS.txt` 為準。版本化下載先部署 Pages，確認匿名下載後才公開 GitHub Release，避免自動更新發現尚不能下載的新版。

2026-09-15 Pages [部署成功](https://github.com/kisaraki/tools-screensaver-tzk/actions/runs/34976356795)；以停用 curl 設定檔、沒有 cookie／帳密／Authorization 且禁止 redirect 的請求下載 SCR、Setup、SHA256SUMS，均 HTTP 200。兩個二進位的 SHA-256 與本機封裝完全一致，雜湊表只有 Git／Linux 的 CRLF→LF 換行差異；[匿名下載證據](evidence/phase23/public-downloads.json)。下載測試未執行檔案。
