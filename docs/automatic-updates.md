# 自動及手動版本更新

實作版本：v0.15.3；設定 schema 10；日期：2026-09-16。

## 控制面板與生命週期

- 「自動更新」預設不勾選；外層確定後才保存 `AutoUpdate=1`，取消不保存。
- 「手動更新」立即開始背景檢查，不受自動選項與每日限制影響；不自動保存其他尚未提交的設定草稿。
- 自動更新僅在正式 `/s`、一般且未鎖定的 `Default` 桌面啟動。`/p`、設定預覽、Debug、Setup helper 均不自動檢查；安全／鎖定桌面不顯示更新或啟動安裝。
- 每位 Windows 使用者以本機日期 YYYYMMDD 記錄 `LastUpdateCheckDay`。命名 mutex 與 HKCU 日期寫入保證同日第一次啟動才發出檢查；後續啟動不再連線。檢查失敗亦不在同日重試，使用者可按手動更新。
- 檢查不阻擋螢幕保護啟動。發現新版時，先關閉全部 surface／WebView2、還原游標，再詢問是否下載安裝。若使用者在結果回來前離開 saver，所有 saver 視窗仍立即關閉，程序僅在背景最多再等 50 秒完成該次檢查；不因此漏掉當日結果。
- 拒絕下載即保留原版本；同日不重複詢問。點選是後進入可取消的下載視窗，核對檔案後透過 Windows 啟動普通互動 Setup。Setup 延用既有的異版移除詢問與 UAC。成功啟動 Setup 時關閉設定面板，讓原 `.scr` 程序退出，以免佔用安裝檔。
- 不會靜默覆蓋程式、不執行無人升級，也不改寫登入安全設定。

## 來源與完整性

版本來源：GitHub 公開 REST endpoint `https://api.github.com/repos/kisaraki/tools-screensaver-tzk/releases/latest`。只接受非 draft、非 prerelease 的 `vMAJOR.MINOR.PATCH`，以三個整數比較；相同／較舊版本不更新。不執行 release body，也不採信任意 download URL。

下載固定為 GitHub Pages：

```text
https://kisaraki.github.io/tools-screensaver-tzk/downloads/vX.Y.Z/SHA256SUMS.txt
https://kisaraki.github.io/tools-screensaver-tzk/downloads/vX.Y.Z/tools-screensaver-tzk-Setup.exe
```

不需帳號或 token。雜湊表上限 16 KiB，必須只有一個正確檔名的 64 位 hexadecimal SHA-256；Setup 上限 128 MiB，必須符合 SHA-256 與 MZ 標頭。TLS、固定 host、禁止 redirect 及 SHA-256 任一失敗都不執行安裝。SHA-256 驗證的是同一官方 HTTPS 發布來源的完整性，不等同 Authenticode 身分簽章；成品目前仍未簽章。

更新暫存位置：`%LOCALAPPDATA%\KOMSMOS\tools-screensaver-tzk\Updates\{version}-{PID}-{nonce}\tools-screensaver-tzk-Setup.exe`。採唯一目錄及 create-new 寫入，不覆寫既有下載。使用者取消下載視窗後不會啟動其工作結果；暫存檔可自行刪除。

## 發布約束

新版應先將 SCR、Setup 與 SHA256SUMS 放到版本化 Pages 下載目錄，確認匿名下載與 hash，再建立／公開 GitHub Release，以免更新檢查發現已公布但尚不可下載的版本。舊版下載目錄不可覆寫。手動與自動更新的實際 UAC／異版移除／重新安裝需在可互動 Windows 10 補測；遠端僅執行資料解析、日期 gate、公開下載及雜湊驗證，不啟動 Setup。
