# v0.14.1：保存自訂來源與可驗證的隨機起播

- 使用者在五種日本旅行場景加入的 YouTube 影片或清單，會按場景保存至目前 Windows 使用者的非揮發性登錄設定；登出、重新開機或下次啟動程式後會自動恢復。只有主設定面板按「確定」才保存。
- 每次首次啟動來源、排程切換、失敗復原或載入預抓來源時，原生工作階段都會重新抽選 `181～539` 秒，並把該次起點明確傳給播放器。
- 直接影片、清單載入及清單隨機選片統一使用該次來源的新起點。影片播放結束後重播同一支影片，仍會再次抽選 `181～539` 秒。
- 播放器拒絕超出範圍或缺少起點的載入命令。設定保存、重新開啟與起點範圍均增加非互動測試。

此版未改變 schema 9、來源格式或既有使用方式。完整互動安裝與真實 YouTube 播放仍需在可互動 Windows 10 環境驗收。

公開下載：

- [tools-screensaver-tzk-Setup.exe](https://kisaraki.github.io/tools-screensaver-tzk/downloads/v0.14.1/tools-screensaver-tzk-Setup.exe)
- [tools-screensaver-tzk.scr](https://kisaraki.github.io/tools-screensaver-tzk/downloads/v0.14.1/tools-screensaver-tzk.scr)
- [SHA256SUMS.txt](https://kisaraki.github.io/tools-screensaver-tzk/downloads/v0.14.1/SHA256SUMS.txt)
