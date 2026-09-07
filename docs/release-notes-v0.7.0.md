# tools-screensaver-tzk v0.7.0

修正日本旅行模式第二螢幕一直顯示連線中或失敗的問題。舊版只在主螢幕建立播放器，第二螢幕的靜態畫面卻固定顯示連線提示。

- 每個螢幕各自播放旅行影片、隨機選擇來源，並在播放滿 60 秒後輪換。
- 每個螢幕顯示自己的城市與狀態；單一來源失敗不會覆蓋其他螢幕的狀態或計時。
- 預覽明確標示為靜態預覽，播放器不可用時顯示 WebView2 提示。
- 「自在飛行」與「列車旅行」皆適用；播放螢幕數增加時，網路與系統資源用量也會增加。

[公開下載頁](https://kisaraki.github.io/tools-screensaver-tzk/#download) · [安裝程式（免登入）](https://kisaraki.github.io/tools-screensaver-tzk/downloads/v0.7.0/tools-screensaver-tzk-Setup.exe) · [解除安裝方式](https://kisaraki.github.io/tools-screensaver-tzk/#uninstall)

Windows 10 x64 的 fmt、Clippy、49 個程式測試、19 個安裝判斷測試、40 張離屏 GDI 圖片、非互動 smoke 與封裝通過。遠端驗證沒有啟動全螢幕、實際影片或 UAC；實際雙螢幕播放、輪換與長時間資源觀察仍為 `NOT TESTED`。Windows 11 與完整安裝矩陣尚未驗證。

這是未簽章的開發候選版。下載後可核對：

| 成品 | SHA-256 |
| --- | --- |
| `tools-screensaver-tzk.scr` | `e9aa5d8be9de35480fa8db26951540affe0f5fab83d79e07b3543171e40d7a43` |
| `tools-screensaver-tzk-Setup.exe` | `eafdf0deaf9cbbf60db66a595a01780e01f4dc4c520b599e0cc6cd3341ba851d` |

[Phase 11 修正與驗證報告](https://github.com/kisaraki/tools-screensaver-tzk/blob/v0.7.0/docs/phase11-report.md) · [逐項驗收報告](https://github.com/kisaraki/tools-screensaver-tzk/blob/v0.7.0/docs/acceptance-report.md)
