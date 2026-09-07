# Phase 11：日本旅行多螢幕播放修正

日期：2026-09-08<br>
產品：tools-screensaver-tzk v0.7.0 開發候選版<br>
規格：v1.8；原始碼與成品以 Git tag `v0.7.0` 追溯<br>
環境：Windows 10 Education 22H2 x64，build 19045.6456

## 問題與修正

舊版只為排序第一的主螢幕建立 WebView2 player；第二螢幕一直使用 GDI 靜態伴隨畫面，而該畫面的文字固定寫成「連線中或影像來源暫時無法使用」。因此第二螢幕的提示不能用來判斷網路是否真的失敗。

v0.7.0 改為每個螢幕各自建立一個 `TravelHost`，保存自己的 player、來源、地點、generation、播放 token、60 秒輪換與重試狀態。背景來源與 WebView2 通知帶入所屬 HWND，由共用 coordinator 路由。第二螢幕的來源失敗不會改寫第一螢幕的播放狀態或計時。各螢幕獨立隨機挑選，允許選到相同地點。

本機 HTML shell 在播放器啟動前只寫入一次。非同步 COM startup 輪詢不跨呼叫持有 `RefCell` 借用，避免訊息重入時借用衝突。每個 host 輪換時重用 controller；共用鍵鼠退出與顯示拓撲變化處理會關閉所有 host，已關閉的路由與晚到結果不再使用。

GDI 預覽明確顯示「靜態預覽」；全螢幕等待畫面使用該螢幕的狀態，播放器無法啟動或停止時給出 WebView2 提示，不再把所有情況寫成來源連線失敗。旅行框仍位於完整 player 矩形外。每個 screen／本機頁面只有一個 autoplay player，對照 [YouTube Required Minimum Functionality](https://developers.google.com/youtube/terms/required-minimum-functionality) 的每頁或每螢幕限制；網路、記憶體與 GPU 用量會隨播放螢幕數增加。

## 實際驗證

| 項目 | 狀態 | 證據 |
| --- | --- | --- |
| fmt／Clippy／Release／封裝 | PASS | [package.txt](evidence/phase11/package.txt)；Rust 1.97.1、Inno Setup 6.7.3 |
| 預設 Rust 測試 | PASS | 49 passed：lib 29、CLI 8、native noninteractive 2、layout 10；9 個測試預設 ignored |
| 新增回歸測試 | PASS | 兩個 message-only HWND 的 host 路由、注入第二來源失敗後第一螢幕狀態／計時不變、各自地點、清理全部 host、弱參照回收；前兩模式不建立 host；預覽／播放／播放器失敗文字區分 |
| Installer policy | PASS | 同一份 Pascal policy 的 19 checks；最低權限 harness 在精靈建立前結束；沒有執行 Bootstrapper 或改寫 registry |
| Runtime 與 Bootstrapper | PASS | 唯讀偵測 Runtime `152.0.4191.66`；內附官方 Bootstrapper `1.3.265.7` 雜湊一致且 Microsoft 簽章有效 |
| SCR smoke | PASS | 版本、PE／resources／manifest／imports、靜態 CRT、無 UI 錯誤路徑、helper 拒絕與 registry 不變；[JSON](evidence/phase11/smoke/smoke-report.json) |
| GDI fixture | PASS | 40 張重新匯出：[fixtures.txt](evidence/phase11/fixtures.txt)；新增檢查來源、播放文字與播放器失敗三種固定狀態；[視覺證據](visual-reference.md) |
| 實際雙螢幕 WebView2 播放 | NOT TESTED | 遠端工作階段不啟動正式播放器或全螢幕；狀態模型／GDI fixture 不等於實際影片播放 |

建置紀錄的 Source revision 為基底 `7d26681`，包含本階段當時尚未提交的修改；最終原始碼由 `v0.7.0` tag 鎖定。來源 HTTP 證據沿用 Phase 8 的具體時間紀錄，本輪未重新探測來源。

## 成品

| 檔案 | Bytes | SHA-256 |
| --- | ---: | --- |
| `tools-screensaver-tzk.scr` | 812,544 | `e9aa5d8be9de35480fa8db26951540affe0f5fab83d79e07b3543171e40d7a43` |
| `tools-screensaver-tzk-Setup.exe` | 3,982,059 | `eafdf0deaf9cbbf60db66a595a01780e01f4dc4c520b599e0cc6cd3341ba851d` |

產品成品仍為 `NotSigned`。GitHub Release 與 [公開下載頁](https://kisaraki.github.io/tools-screensaver-tzk/#download) 提供相同成品；Pages 直連不需要登入。

## 未測範圍

實際每螢幕進入 `PLAYING`、各自至少五次輪換、斷線、混合 DPI／直向／4K、30 分鐘含 WebView2 子程序的資源觀察仍需本機人工驗收。共用 browser process 本身崩潰可能影響多個 controller；本次測試證明的是來源與事件狀態隔離，不是底層 Runtime 程序故障隔離。

本輪沒有啟動正式 Setup、Microsoft Bootstrapper、全螢幕或 WebView2 player，沒有觸發 UAC、寫 System32 或變更系統螢幕保護設定。完整安裝／升級／解除安裝矩陣仍未測；Windows 11 延期。完整逐項狀態見 [驗收報告](acceptance-report.md)。
