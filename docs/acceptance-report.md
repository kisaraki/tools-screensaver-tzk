# tools-screensaver-tzk 驗收報告

軟體版本：0.7.0 開發候選版<br>
規格文件：v1.8（修訂版）<br>
執行日期：2026-09-07～2026-09-08（建置與續作驗證）<br>
Source revision：本報告隨 Git tag `v0.7.0` 鎖定；Phase 0～5 歷史結果由 tag `v0.1.1` 追溯<br>
環境：Windows 10 Education 22H2 x64，build 19045.6456；Intel Core i5-8259U，4 cores／8 logical processors，約 24 GiB RAM；Intel Iris Plus Graphics 655；雙 3840×2160、兩者 144 DPI／150%，左側螢幕為負 X<br>
工具：Rust／Cargo 1.97.1、MSVC x64 toolset 14.51.36231（link.exe 14.51.36256.0）、Windows SDK RC 10.0.26100.0、Inno Setup 6.7.3、WebView2 Evergreen Runtime 152.0.4191.66

狀態只使用 `PASS`、`FAIL`、`NOT TESTED`、`NOT APPLICABLE`。必要實機情境只完成一部分時，整項仍列 `NOT TESTED` 並說明局部證據。本報告沒有已知 `FAIL`，但仍有必要項目 `NOT TESTED`，所以不宣稱日本旅行模式或 Windows 10 完整驗收完成。

## v0.7.0 非互動驗證摘要

| 項目 | 狀態 | 結果／證據 |
| --- | --- | --- |
| `scripts\build.bat` | PASS | fmt、Clippy `-D warnings`、locked tests、locked Release build 均 exit 0 |
| 自動測試 | PASS | 預設 49 passed：lib 29、CLI 8、native noninteractive 2、layout 10；9 個測試預設 ignored；40 張 GDI 圖片重新匯出 |
| 置中版面／GDI fixture | PASS | 64%×60% 中央內容區、小型 preview、直向、4K 與旅行場景；40 張圖片重新匯出，包含旅行狀態文字；[Phase 11 report](phase11-report.md) |
| Runtime registry 偵測 | PASS | 2026-09-08：使用 Setup 同一個唯讀函式偵測到電腦層級 Runtime 152.0.4191.66；沒有啟動 Runtime |
| 產品識別掃描 | PASS | 目前工作樹的文字與路徑以大小寫無關檢查，沒有改名前的英文識別；[Phase 8 report](phase8-report.md) |
| 來源 HTTP probe（沿用 v0.4.0 證據） | PASS | 2026-09-07T20:17:11.3035837+08:00：目錄正常、8／8 候選可解析；本次未重跑網路探測；[source-health.json](evidence/phase8/source-health.json) |
| 非互動 smoke | PASS | v0.7.0、ProductName／FileDescription、第三模式、兩種旅行場景 radio、imports、靜態 CRT、無 UI 錯誤參數與 registry 不變；[smoke-report.json](evidence/phase11/smoke/smoke-report.json) |
| Package | PASS | Inno Setup 6.7.3 封裝成功；`.scr` 與 Setup 版本一致且均 NotSigned |
| WebView2 Bootstrapper | PASS | 官方 `1.3.265.7`；有效 Microsoft Corporation 簽章；版本／大小／SHA-256 鎖定且編譯時再次核對；[部署紀錄](webview2-setup.md) |
| Installer policy | PASS | 共用 Pascal policy 的 19 個非互動 checks，最低權限 harness 在 wizard 建立前結束；[Phase 10 report](phase10-report.md) |

上述驗證沒有開啟 `/s`、WebView2 player、設定畫面或正式 Setup，沒有執行 Microsoft Bootstrapper，也沒有寫 System32、改螢幕保護設定或觸發 UAC。獨立 policy harness 在 wizard 建立前結束。HTTP 可達及 Runtime 可偵測均不能代替影片 `PLAYING`。

## AC01～AC22

| ID | 狀態 | 環境／實際結果 | 證據 | 失敗原因或缺少條件 |
| --- | --- | --- | --- | --- |
| AC01 | NOT TESTED | v0.7.0 fmt、Clippy、tests、Release、PE／resources／imports 全部實跑通過 | Phase 11 report、smoke | MT15 的無開發工具乾淨 Win10 尚無環境 |
| AC02 | PASS | Debug／Release 真實命令列與原生視窗測試沿用 Phase 4 證據；v0.7.0 無 UI 錯誤參數重跑通過 | Phase 4 `native-release.txt`、UT01～03、Phase 11 smoke | — |
| AC03 | NOT TESTED | 真實跨程序 parent、三種 host DPI context、resize／退出／刷新已有基線證據 | Phase 3 native、Phase 4 native Release | v0.7.0 未重跑互動 host；缺實體混合 100%／150%／200% 桌面與 Windows 設定頁 |
| AC04 | NOT TESTED | 原生設定、preview、ChooseFont、提交／取消閉環已有 Phase 3 證據；v0.7.0 第三模式及雙場景 radio resource 通過 | Phase 3 report／截圖、Phase 11 smoke | 安裝後 Windows 設定頁調度及雙場景互動預覽未測 |
| AC05 | PASS | Gregorian／鐘角／layout tests 與置中版面 GDI fixture | Phase 9 fixtures、UT04～07、17～19 | — |
| AC06 | NOT TESTED | 倒數原生輸入與取消已有證據 | Phase 3 native、UT08～09、MT07 | 真實閒置／恢復登入未測 |
| AC07 | NOT TESTED | 七段、沙漏、進度、最後十秒、四次閃爍測試與 MT07 通過 | UT10～15、Phase 2 fixtures | 實際睡眠／恢復／校時未測 |
| AC08 | NOT TESTED | 雙 4K、負 X、共用 generation 與合成 display-change 清理已有基線證據 | UT25、MT03、Phase 4 native | 旅行模式多螢幕／混合 DPI、拔除顯示器與登出未測 |
| AC09 | NOT TESTED | 四色、四字型、fallback 與極端點數有測試／fixture | UT18～22、visual-reference | 200% 實體 dialog 與混合 DPI 未測 |
| AC10 | PASS | 專用測試 key 與 fake store 驗證型別、schema 4、mode 2 與 TravelStyle 0／1 round-trip、schema 3 回退、取消、rollback、未知值 | UT20～24、UT26 | — |
| AC11 | NOT TESTED | 指定鍵鼠、4px／500ms、同程序焦點與清理已有 Phase 1／4 證據；旅行 WebView accelerator／mouse poll 已建置 | Phase 1／4 native、Phase 6 build | 旅行 player 實際輸入退出、外部 foreground 成功分支與登出未測 |
| AC12 | NOT TESTED | 前兩種 GDI 模式 Release preview 各 30 分鐘、各 59 次 cache 循環穩定 | Phase 4 resource report | 正式 Countdown 10 Hz 與旅行 WebView2 30 分鐘資源觀察未測 |
| AC13 | NOT TESTED | v0.7.0 Setup 已編譯且版本／hash 一致 | Phase 11 report、`SHA256SUMS.txt` | 沒有實際安裝、列舉、移除或改名前版本覆蓋升級；遠端不觸發 UAC |
| AC14 | NOT TESTED | 非 System32 helper code 4 且四個螢幕保護 registry 值不變 | Phase 11 smoke | 原使用者成功路徑、另一管理員 UAC、直接提權 installer 未測 |
| AC15 | PASS | README、規格、來源紀錄、測試、`.scr`、Setup、hash、Phase 6～11 與逐項報告均存在 | repository 交付清單 | 成品明列 NotSigned 與未測限制 |
| AC16 | NOT TESTED | 「自在飛行」與「列車旅行」HTML／CSS shell、GDI fallback、完整 player rect／外部 caption 的 deterministic test 及 fixture 通過 | UT26、UT34、Phase 7 report | 兩種場景的實際 player、地點文字及多螢幕未觀察 |
| AC17 | NOT TESTED | 8 個 seed 隨機排序、排除上一來源、60 秒 monotonic 邊界、parser／狀態模型及 8／8 HTTP probe 通過 | UT27～UT30、Phase 6 source health | 實際 `PLAYING`、輪換、stall／error failover與 shutdown 未測；UT31～UT32 未覆蓋完整整合矩陣 |
| AC18 | NOT TESTED | 前兩模式／preview 不建立 travel session；Runtime probe、host allowlist、CSP、fallback 與揭露已實作 | UT33 局部證據、README、source record | 實際斷線／Runtime 缺失及 outbound capture 未測；8 個來源權利條款未逐一抽查 |
| AC19 | PASS | Cargo package／binary、Rust crate、SCR／Setup、resources、manifest、window、Registry、WebView2 path、User-Agent、腳本、文件與 Pages 均已統一 | Phase 8 report、resource smoke、repository scan | Git commit／tag 歷史依版本紀錄保留 |
| AC20 | PASS | 大型桌曆時鐘與番茄鐘置中於 64%W×60%H，小型 preview 及旅行維持 88%W×82%H；所有圖形落在指定區域 | layout tests、Phase 9 的 37 張 GDI fixtures | GDI 畫面證據；沒有開啟全螢幕或系統設定視窗 |
| AC21 | NOT TESTED | Runtime 唯讀偵測、19 個 policy checks、官方 Bootstrapper 簽章與封裝通過 | Phase 10 report、Phase 11 package.txt | 實際 Runtime 安裝、斷網／Proxy、失敗重試、重啟與多帳號 UAC 尚未驗證 |
| AC22 | NOT TESTED | 每螢幕獨立 TravelHost、事件路由、來源失敗隔離、清理與真實狀態文字的回歸測試通過 | Phase 11 report、49 個 Rust tests、40 張 GDI fixtures | 實際雙螢幕 WebView2 同時播放、各自輪換、混合 DPI 與完整資源清理尚未觀察 |

## UT01～UT35

| ID | 狀態 | 證據／限制 |
| --- | --- | --- |
| UT01～UT03 | PASS | `tests/cli_tests.rs`、`tests/native_modes.rs`；含真實 Release command line |
| UT04～UT07 | PASS | `tests/time_layout.rs` Gregorian 與連續鐘角 |
| UT08～UT09 | PASS | duration 欄位拒絕、邊界與 round trip |
| UT10～UT16 | PASS | deadline、ceil、ratio、最後十秒、四次暗半週期、segment、xorshift |
| UT17～UT19 | PASS | 五種 aspect、1.349／1.350、0×0～4K、96～288 DPI、安全位移 bounds |
| UT20～UT24 | PASS | config／registry fallback、schema、font、取消與注入失敗 rollback |
| UT25 | PASS | 共用 generation、idempotent shutdown、最後視窗 quit |
| UT26 | PASS | schema 3 的 `JapanTravel=2` 相容並預設自在飛行；schema 4 的 `TravelStyle=0/1` round-trip；future schema 保護 |
| UT27～UT28 | PASS | 8 個候選唯一、固定 seed、排除上一來源、59,999／60,000 ms 與時間跳躍 |
| UT29～UT30 | PASS | detail HTML、entity、host／ID 拒絕、固定 embed URL 與 script escaping |
| UT31 | NOT TESTED | player state transition test 與 live HTTP probe 已通過；尚無完整 mock transport 的 3xx／404／TLS／取消與 mock player navigation 矩陣 |
| UT32 | NOT TESTED | generation/channel/controller teardown 已實作並通過 Clippy／建置；尚無直接 late completion／shutdown 重入整合測試 |
| UT33 | NOT TESTED | `/p`、`/c` 與前兩模式不建立 `TravelSession` 的程式路徑已審查；尚無 fake transport request counter 或 outbound capture |
| UT34 | PASS | 雙場景 GDI 小尺寸／800×450 fixture、每種 HTML shell 單一 player、正確場景 class／aria label 及 caption 位於 player 外 |
| UT35 | PASS | 兩個 message-only HWND 的事件路由、第二來源失敗不改第一螢幕播放計時、各自地點、關閉所有 host、弱參照回收；前兩模式不建立 host；未建立 WebView2 |

Phase 5 的 helper 精確旗標與非 System32 真實 binary 拒絕測試在 v0.7.0 也重跑通過。來源 probe 沿用 Phase 8；本次 Runtime 只以 Setup 共用函式讀取 registry，沒有建立 WebView2 controller 或執行 Runtime installer。

## MT01～MT20

| ID | 狀態 | 已取得的證據 | 缺少條件 |
| --- | --- | --- | --- |
| MT01 | NOT TESTED | Win10 build與歷史 `/s`／`/p`／`/c` 測試 | v0.7.0 安裝、列舉、系統 preview、解除安裝未測 |
| MT02 | NOT TESTED | 1920×1080 fixture／96-DPI preview 長測 | 無實體單螢幕 100% 全螢幕環境 |
| MT03 | PASS | 雙 4K／150%、左側負 X、全覆蓋、共用 generation、內部焦點的 Phase 4 基線 | — |
| MT04 | NOT TESTED | 4K／150% 與直向 fixture | 無混合 100%／150%／200%、實體直向與 200% dialog |
| MT05 | NOT TESTED | 真實 host 保存後 2 秒刷新、取消不寫的 Phase 3 證據 | Windows 設定頁中的已安裝 v0.7.0 尚未測 |
| MT06 | PASS | unaware／system／PMv2 host，四種 resize，parent 關閉退出的 Phase 3 證據 | — |
| MT07 | PASS | 1 秒、5 秒、30 分鐘、99:59:59 原生流程與時間邏輯 | — |
| MT08 | NOT TESTED | 手動 `/s` 歷史測試不採計本項 | 真實閒置、自動啟動、三種模式、恢復登入開／關與安全桌面 |
| MT09 | NOT TESTED | 純 deadline／校時／跨午夜模型已測 | 實際睡眠／恢復、校時與跨午夜 |
| MT10 | NOT TESTED | 合成 display change、session end、close 已測 | 實際拔線、外部前景成功切換分支與登出 |
| MT11 | NOT TESTED | helper 身分檢查已實作 | 獨立標準帳號與另一管理員 UAC |
| MT12 | NOT TESTED | helper 非安裝路徑拒絕已測 | Setup 從已提權程序啟動及安裝後 elevated helper |
| MT13 | NOT TESTED | 固定 AppId／restartreplace／保留偏好規則已編譯 | 實際同版覆蓋、v0.1.1 升級、檔案使用中、解除安裝 |
| MT14 | NOT TESTED | 前兩種 GDI 模式 Release preview 各 30 分鐘與 59 次循環通過 | 全螢幕 10 Hz 長測與實體 DPI 變更 |
| MT15 | NOT TESTED | fresh offline hash、靜態 CRT、無 VC/UCRT／WebView2Loader 動態 import | 無未裝開發工具／VC++ Redistributable 的乾淨 Win10；缺 Runtime fallback 也未實機驗證 |
| MT16 | NOT TESTED | player shell、狀態模型、來源與 60 秒 deadline tests PASS | 實際 `PLAYING`、靜音、地點吻合及至少 5 次輪換 |
| MT17 | NOT TESTED | 錯誤狀態與 GDI fallback code 已建置；Runtime probe PASS | 實際 tw.live／YouTube 斷線、所有候選失效及 Runtime 缺失 |
| MT18 | NOT TESTED | 每個螢幕一個 player；兩螢幕路由、失敗隔離與清理的非互動回歸測試通過；既有雙 4K 基線 | 旅行 player 的多螢幕、150%／200%、4K、直向及拓撲變化 |
| MT19 | NOT TESTED | controller 重用與 shutdown code 已審查 | 30 分鐘、至少 29 次切換及 parent＋WebView2 descendant resource 記錄 |
| MT20 | NOT TESTED | host allowlist、CSP、new-window／download／permission deny 已實作 | 實際 outbound isolation capture |

Windows 11：`NOT TESTED`，依使用者指示延期；這不阻擋目前 Windows 10 候選版交付，也不能被寫成已支援通過。

## v0.7.0 成品

| 成品 | Bytes | SHA-256 | 狀態 |
| --- | ---: | --- | --- |
| `dist/tools-screensaver-tzk.scr` | 812,544 | `e9aa5d8be9de35480fa8db26951540affe0f5fab83d79e07b3543171e40d7a43` | Build／smoke PASS；NotSigned |
| `dist/tools-screensaver-tzk-Setup.exe` | 3,982,059 | `eafdf0deaf9cbbf60db66a595a01780e01f4dc4c520b599e0cc6cd3341ba851d` | Package／版本檢查 PASS；實際安裝 NOT TESTED；NotSigned |

Phase 5 的 v0.1.1 hash 保留在 Phase 5 報告與該 Release，不再列為目前成品。完成 Windows 10 完整驗收仍需在可互動本機環境補做上述必要項目；遠端工作階段不觸發 UAC。程式碼簽章憑證尚未提供，v0.7.0 成品維持 NotSigned。
