# Phase 21：自訂 YouTube 來源與桌曆顯示

產品版本：v0.14.0；規格版本：v2.8；設定 schema：9。

日期：2026-09-10；平台：Windows 10 x64。原始碼由 Git tag `v0.14.0` 鎖定。

## 完成內容

- 顯示名稱「鐵灰色」改為「鐵灰」，保留 enum、RGB 與舊偏好。
- 新增中式／英文／日式桌曆選單、月名與星期對照，以及實際 GDI 文字尺寸驗證。三種方式皆維持 Gregorian 月曆，不做陰曆換算。
- 五種旅行場景分別有自訂來源編輯器，每場景最多 10 個 YouTube 影片或清單 URL；支援新增、刪除、清空、去重及有界離線驗證。
- 來源與各場景預設入口一起隨機挑選；沒有自訂來源時保留既有行為。有多入口時排除上一個入口，自訂影片／清單沿用原播放器、預抓、隨機起點、切換與錯誤復原。
- 設定 schema 9 新增 CalendarStyle 及五組 REG_BINARY URL 清單；來源 editor 只更新主 draft，主面板確定才交易保存。舊 schema 安全回退，倒數提交升版不意外啟用未知新值。
- README、Pages、開發規格、系統開發規格及從零開發步驟同步更新。

## 實際驗證

| 項目 | 結果與證據 |
| --- | --- |
| fmt、Clippy、Release build、Package | PASS；[package.txt](evidence/phase21/package.txt) |
| Rust 預設測試 | 68 passed：lib 48、CLI 8、native noninteractive 2、layout 10；9 ignored |
| 新增測試 | 8 個：月名／星期對照、URL 格式／非法 host／上限／去重、五場景選擇、schema round-trip／rollback／舊版遷移、GDI 欄寬 |
| Installer policy | 19 個 WebView2 + 15 個產品版本 checks；不建立 wizard、不提權、不執行 installer、不寫 registry |
| 無 UI smoke | PASS；[smoke-report.json](evidence/phase21/smoke/smoke-report.json)，含來源 modal 資源 2005、新控制項文字、版本、PE/imports 及受保護 registry 不變 |
| GDI fixture export | PASS；[calendar-fixtures.txt](evidence/phase21/calendar-fixtures.txt)，輸出既有 55 張加新增 9 張桌曆 |
| 新桌曆目視檢查 | 英文與日式的 1920×1080、320×180 圖確認月名／星期無超欄；三種模式各 1920×1080、1080×1920、320×180 的 9 張存於 [calendar](evidence/phase21/calendar/) |

fixture 匯出首次在一般 shell 缺少 rc.exe；載入官方 VsDevCmd x64 環境後完成。Windows PowerShell 5 的 smoke script 需要 UTF-8 BOM，已保留，重跑成功。

本階段沒有開啟設定視窗、全螢幕、實際 WebView2 播放、正式 Setup 或 UAC，沒有改動目前使用者的螢幕保護程式設定。對話框取消由 draft 路徑審查與既有保存測試支援，未宣稱實際點選測試通過。來源的公開性、playlist API 及不同機器可嵌入性仍需正式播放驗收。

## 成品

| 成品 | Bytes | SHA-256 |
| --- | ---: | --- |
| tools-screensaver-tzk.scr | 9,570,304 | `4d69ac312483376a622fe912a9d9bc2d70b6e6440fe61faced2ef38fd177551c` |
| tools-screensaver-tzk-Setup.exe | 12,614,321 | `987d0f2612d83b187bef29b5788824ce90767333c6c5786387bf1d66711350fb` |

兩者版本一致且 NotSigned。網站下載使用 `site/downloads/v0.14.0/`，保留舊版本下載。

## 待實機驗收

- 主面板三種桌曆切換／確定／取消及五場景來源 editor 的實際鍵盤、Tab、貼上、刪除、取消操作。
- 個別影片與清單的正式播放、私人／失效來源復原、五場景及雙螢幕連續輪換。
- Setup 版本更新及設定面板完成頁；遠端不觸發 UAC。
- Windows 11 依既有決策延後。
