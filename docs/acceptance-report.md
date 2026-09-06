# MyDateTimeScreensaver 驗收報告

軟體版本：0.3.0 開發候選版<br>
規格文件：v1.4（修訂版）<br>
執行日期：2026-09-07<br>
Source revision：本報告隨 Git tag `v0.3.0` 鎖定；Phase 0～5 歷史結果由 tag `v0.1.1` 追溯<br>
環境：Windows 10 Education 22H2 x64，build 19045.6456；Intel Core i5-8259U，4 cores／8 logical processors，約 24 GiB RAM；Intel Iris Plus Graphics 655；雙 3840×2160、兩者 144 DPI／150%，左側螢幕為負 X<br>
工具：Rust／Cargo 1.97.1、MSVC x64 toolset 14.51.36231（link.exe 14.51.36256.0）、Windows SDK RC 10.0.26100.0、Inno Setup 6.7.3、WebView2 Evergreen Runtime 152.0.4191.66

狀態只使用 `PASS`、`FAIL`、`NOT TESTED`、`NOT APPLICABLE`。必要實機情境只完成一部分時，整項仍列 `NOT TESTED` 並說明局部證據。本報告沒有已知 `FAIL`，但仍有必要項目 `NOT TESTED`，所以不宣稱日本旅行模式或 Windows 10 完整驗收完成。

## v0.3.0 非互動驗證摘要

| 項目 | 狀態 | 結果／證據 |
| --- | --- | --- |
| `scripts\build.bat` | PASS | fmt、Clippy `-D warnings`、locked tests、locked Release build 均 exit 0 |
| 自動測試 | PASS | 預設 45 passed：lib 26、CLI 8、native noninteractive 2、layout 9；另有 9 個互動、長時間或環境測試 ignored，其中 Runtime 與產品解析器即時來源測試另行明確執行並通過 |
| Runtime 無視窗 probe | PASS | 偵測到 WebView2 Evergreen Runtime 152.0.4191.66；未建立 controller／player／視窗 |
| 來源 HTTP probe | PASS | 2026-09-07T06:36:23.3366718+08:00：目錄正常、8／8 候選可解析；[source-health.json](evidence/phase7/source-health.json) |
| 非互動 smoke | PASS | v0.3.0、第三模式、兩種旅行場景 radio、imports、靜態 CRT、無 UI 錯誤參數與 registry 不變；[smoke-report.json](evidence/phase7/smoke/smoke-report.json) |
| Package | PASS | Inno Setup 6.7.3 封裝成功；`.scr` 與 Setup 版本一致且均 NotSigned |

上述驗證沒有開啟 `/s`、WebView2 player、設定畫面或安裝程式，也沒有寫 System32、改螢幕保護設定或觸發 UAC。HTTP 可達及 Runtime 可偵測均不能代替影片 `PLAYING`。

## AC01～AC18

| ID | 狀態 | 環境／實際結果 | 證據 | 失敗原因或缺少條件 |
| --- | --- | --- | --- | --- |
| AC01 | NOT TESTED | v0.3.0 fmt、Clippy、tests、Release、PE／resources／imports 全部實跑通過 | Phase 7 build、smoke | MT15 的無開發工具乾淨 Win10 尚無環境 |
| AC02 | PASS | Debug／Release 真實命令列與原生視窗測試沿用 Phase 4 證據；v0.3.0 無 UI 錯誤參數重跑通過 | Phase 4 `native-release.txt`、UT01～03、Phase 6 smoke | — |
| AC03 | NOT TESTED | 真實跨程序 parent、三種 host DPI context、resize／退出／刷新已有基線證據 | Phase 3 native、Phase 4 native Release | v0.3.0 未重跑互動 host；缺實體混合 100%／150%／200% 桌面與 Windows 設定頁 |
| AC04 | NOT TESTED | 原生設定、preview、ChooseFont、提交／取消閉環已有 Phase 3 證據；v0.3.0 第三模式及雙場景 radio resource 通過 | Phase 3 report／截圖、Phase 7 smoke | 安裝後 Windows 設定頁調度及雙場景互動預覽未測 |
| AC05 | PASS | Gregorian／鐘角／layout tests 與 24 張 GDI fixture | Phase 2 visuals、UT04～07、17～19 | — |
| AC06 | NOT TESTED | 倒數原生輸入與取消已有證據 | Phase 3 native、UT08～09、MT07 | 真實閒置／恢復登入未測 |
| AC07 | NOT TESTED | 七段、沙漏、進度、最後十秒、四次閃爍測試與 MT07 通過 | UT10～15、Phase 2 fixtures | 實際睡眠／恢復／校時未測 |
| AC08 | NOT TESTED | 雙 4K、負 X、共用 generation 與合成 display-change 清理已有基線證據 | UT25、MT03、Phase 4 native | 旅行模式多螢幕／混合 DPI、拔除顯示器與登出未測 |
| AC09 | NOT TESTED | 四色、四字型、fallback 與極端點數有測試／fixture | UT18～22、visual-reference | 200% 實體 dialog 與混合 DPI 未測 |
| AC10 | PASS | 專用測試 key 與 fake store 驗證型別、schema 4、mode 2 與 TravelStyle 0／1 round-trip、schema 3 回退、取消、rollback、未知值 | UT20～24、UT26 | — |
| AC11 | NOT TESTED | 指定鍵鼠、4px／500ms、同程序焦點與清理已有 Phase 1／4 證據；旅行 WebView accelerator／mouse poll 已建置 | Phase 1／4 native、Phase 6 build | 旅行 player 實際輸入退出、外部 foreground 成功分支與登出未測 |
| AC12 | NOT TESTED | 前兩種 GDI 模式 Release preview 各 30 分鐘、各 59 次 cache 循環穩定 | Phase 4 resource report | 正式 Countdown 10 Hz 與旅行 WebView2 30 分鐘資源觀察未測 |
| AC13 | NOT TESTED | v0.3.0 Setup 已編譯且版本／hash 一致 | Phase 7 package、`SHA256SUMS.txt` | 沒有實際安裝、列舉、移除；遠端不觸發 UAC |
| AC14 | NOT TESTED | 非 System32 helper code 4 且四個螢幕保護 registry 值不變 | Phase 6 smoke | 原使用者成功路徑、另一管理員 UAC、直接提權 installer 未測 |
| AC15 | PASS | README、規格、來源紀錄、測試、`.scr`、Setup、hash、Phase 6～7 與逐項報告均存在 | repository 交付清單 | 成品明列 NotSigned 與未測限制 |
| AC16 | NOT TESTED | 「自在飛行」與「列車旅行」HTML／CSS shell、GDI fallback、完整 player rect／外部 caption 的 deterministic test 及 fixture 通過 | UT26、UT34、Phase 7 report | 兩種場景的實際 player、地點文字及多螢幕未觀察 |
| AC17 | NOT TESTED | 8 個 seed 隨機排序、排除上一來源、60 秒 monotonic 邊界、parser／狀態模型及 8／8 HTTP probe 通過 | UT27～UT30、Phase 6 source health | 實際 `PLAYING`、輪換、stall／error failover與 shutdown 未測；UT31～UT32 未覆蓋完整整合矩陣 |
| AC18 | NOT TESTED | 前兩模式／preview 不建立 travel session；Runtime probe、host allowlist、CSP、fallback 與揭露已實作 | UT33 局部證據、README、source record | 實際斷線／Runtime 缺失及 outbound capture 未測；8 個來源權利條款未逐一抽查 |

## UT01～UT34

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

Phase 5 的 helper 精確旗標與非 System32 真實 binary 拒絕測試在 v0.3.0 也重跑通過。WebView2 Runtime probe 是 ignored host prerequisite test，已由本次非互動命令明確執行；它不開 UI。

## MT01～MT20

| ID | 狀態 | 已取得的證據 | 缺少條件 |
| --- | --- | --- | --- |
| MT01 | NOT TESTED | Win10 build與歷史 `/s`／`/p`／`/c` 測試 | v0.3.0 安裝、列舉、系統 preview、解除安裝未測 |
| MT02 | NOT TESTED | 1920×1080 fixture／96-DPI preview 長測 | 無實體單螢幕 100% 全螢幕環境 |
| MT03 | PASS | 雙 4K／150%、左側負 X、全覆蓋、共用 generation、內部焦點的 Phase 4 基線 | — |
| MT04 | NOT TESTED | 4K／150% 與直向 fixture | 無混合 100%／150%／200%、實體直向與 200% dialog |
| MT05 | NOT TESTED | 真實 host 保存後 2 秒刷新、取消不寫的 Phase 3 證據 | Windows 設定頁中的已安裝 v0.3.0 尚未測 |
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
| MT18 | NOT TESTED | 程式限制主螢幕一個 player，其他螢幕為 GDI；既有雙 4K 基線 | 旅行 player 的多螢幕、150%／200%、4K、直向及拓撲變化 |
| MT19 | NOT TESTED | controller 重用與 shutdown code 已審查 | 30 分鐘、至少 29 次切換及 parent＋WebView2 descendant resource 記錄 |
| MT20 | NOT TESTED | host allowlist、CSP、new-window／download／permission deny 已實作 | 實際 outbound isolation capture |

Windows 11：`NOT TESTED`，依使用者指示延期；這不阻擋目前 Windows 10 候選版交付，也不能被寫成已支援通過。

## v0.3.0 成品

| 成品 | Bytes | SHA-256 | 狀態 |
| --- | ---: | --- | --- |
| `dist/MyDateTimeScreensaver.scr` | 808,448 | `2211fd40f847d6d3b2b01c95e318caca2da09993bb5b8d25146b2c88630ddb8a` | Build／smoke PASS；NotSigned |
| `dist/MyDateTimeScreensaver-Setup.exe` | 2,321,785 | `03635c9567a7d626eab016e9e4385a166771639f4e4d1dc53f041d562ad3e626` | Package／版本檢查 PASS；實際安裝 NOT TESTED；NotSigned |

Phase 5 的 v0.1.1 hash 保留在 Phase 5 報告與該 Release，不再列為目前成品。完成 Windows 10 完整驗收仍需在可互動本機環境補做上述必要項目；遠端工作階段不觸發 UAC。程式碼簽章憑證尚未提供，v0.3.0 成品維持 NotSigned。
