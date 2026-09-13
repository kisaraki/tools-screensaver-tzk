# tools-screensaver-tzk 驗收報告

軟體版本：0.14.1 開發候選版<br>
規格文件：v2.9（修訂版）<br>
執行日期：2026-09-07～2026-09-13（建置與續作驗證）<br>
Source revision：本報告隨 Git tag `v0.14.1` 鎖定；Phase 0～5 歷史結果由 tag `v0.1.1` 追溯<br>
環境：Windows 10 Education 22H2 x64，build 19045.6456；Intel Core i5-8259U，4 cores／8 logical processors，約 24 GiB RAM；Intel Iris Plus Graphics 655；雙 3840×2160、兩者 144 DPI／150%，左側螢幕為負 X<br>
工具：Rust／Cargo 1.97.1、MSVC x64 toolset 14.51.36231（link.exe 14.51.36256.0）、Windows SDK RC 10.0.26100.0、Inno Setup 6.7.3、WebView2 Evergreen Runtime 152.0.4191.66

狀態只使用 `PASS`、`FAIL`、`NOT TESTED`、`NOT APPLICABLE`。必要實機情境只完成一部分時，整項仍列 `NOT TESTED` 並說明局部證據。本報告沒有已知 `FAIL`，但仍有必要項目 `NOT TESTED`，所以不宣稱日本旅行模式或 Windows 10 完整驗收完成。

## v0.14.1 非互動驗證摘要

| 項目 | 狀態 | 結果／證據 |
| --- | --- | --- |
| `scripts\build.bat` | PASS | fmt、Clippy `-D warnings`、locked tests、locked Release build 均 exit 0 |
| 自動測試 | PASS | 預設 69 passed：lib 49、CLI 8、native noninteractive 2、layout 10；9 個測試預設 ignored |
| 桌曆三種顯示方式 | PASS | 中式預設、完整英文月名／星期簡寫、日式月名／七曜日；所有月份文字與 weekday 欄寬測試，9 張離線 GDI fixtures；[Phase 21 report](phase21-report.md) |
| 每場景自訂 YouTube 來源 | PASS | 10 個上限、URL 正規化／去重／非 YouTube 拒絕、預設來源保留、排除目前入口；schema 9 的五場景 round-trip、清空、損壞回退、遷移及 rollback；實際 editor 操作與播放仍 NOT TESTED |
| 自訂來源重新啟動恢復 | PASS | 實際專用 HKCU test key 寫入五場景來源，關閉 store 後由全新 adapter 完整讀回；正式設定 key 未變更；[Phase 22 report](phase22-report.md) |
| 每次來源隨機起播 | PASS | 每次原生來源啟用皆重新產生並傳入 181～539 秒；direct、playlist、shuffle、prefetch 路徑共用該次值，片尾重播另抽新值；實際 YouTube seek 仍 NOT TESTED |
| 啟動設定與游標 | PASS | helper 只設定三個目標值並保留安全值；全螢幕在顯示前把游標停至主要螢幕外角、記錄新 baseline 後隱藏，退出路徑還原游標形狀；[Phase 20 report](phase20-report.md) |
| 配色與番茄鐘面板 | PASS | schema 8 鐵灰 round-trip、七色自動循環通過；番茄鐘使用真正 GDI 平滑漸層形成近黑褐核心、上下光衰減與左右瓶壁折射，移除分格式實色帶；[Phase 19 report](phase19-report.md) |
| 旅行框景與眨眼 | PASS | 御運轉士 player 以 16:9 cover 填滿素材窗孔；地方散策使用曲線模糊眼瞼與 420 ms 閉合／520 ms 展開兩段；10 張離線 HTML fixture 與 source test 通過；[Phase 20 report](phase20-report.md) |
| 播放流程與字幕 | PASS | 所有 load／replay 使用 3:01～8:59 隨機起點；ready、load、playing 與 `onApiChange` 重申關閉字幕；下一候選預備與最小 player UI 保留；[Phase 20 report](phase20-report.md) |
| Runtime registry 偵測 | PASS | 2026-09-08：使用 Setup 同一個唯讀函式偵測到電腦層級 Runtime 152.0.4191.66；沒有啟動 Runtime |
| 產品識別掃描 | PASS | 目前工作樹的文字與路徑以大小寫無關檢查，沒有改名前的英文識別；[Phase 8 report](phase8-report.md) |
| 來源 HTTP probe | PASS | 2026-09-09：tw.live 目錄正常、8／8 候選可解析，4／4 指定 playlist embed endpoint 回傳 HTTP 200；[source-health.json](evidence/phase20/source-health.json) |
| 非互動 smoke | PASS | v0.14.1、五個旅行場景／八個色彩選項、來源切換欄位、含 `msimg32.dll` 的預期 imports、靜態 CRT、無 UI 錯誤參數與正式 registry 不變；[smoke-report.json](evidence/phase22/smoke/smoke-report.json) |
| Package | PASS | Inno Setup 6.7.3 封裝成功；`.scr` 與 Setup 版本一致且均 NotSigned |
| WebView2 Bootstrapper | PASS | 官方 `1.3.265.7`；有效 Microsoft Corporation 簽章；版本／大小／SHA-256 鎖定且編譯時再次核對；[部署紀錄](webview2-setup.md) |
| Installer policy | PASS | 19 個 WebView2 與 15 個產品版本 checks；兩個最低權限 harness 都在 wizard 建立前結束，未顯示提示、執行程序或寫 registry；[Phase 18 report](phase18-report.md) |

Phase 20 曾完成 10 張離線 headless Edge 靜態 HTML 版面檢查；本階段未改旅行幾何，沿用該歷史證據。[Phase 20 report](phase20-report.md)。本階段新增 9 張桌曆 GDI fixture，詳 [Phase 21 report](phase21-report.md)。

上述驗證沒有開啟 `/s`、WebView2 player、設定畫面或正式 Setup，沒有執行 Microsoft Bootstrapper，也沒有寫 System32、改螢幕保護設定或觸發 UAC。獨立 policy harness 在 wizard 建立前結束。HTTP 可達及 Runtime 可偵測均不能代替影片 `PLAYING`。

## AC01～AC34

| ID | 狀態 | 環境／實際結果 | 證據 | 失敗原因或缺少條件 |
| --- | --- | --- | --- | --- |
| AC01 | NOT TESTED | v0.14.1 fmt、Clippy、69 tests、Release、PE／resources／imports 全部實跑通過 | Phase 22 report、smoke | MT15 的無開發工具乾淨 Win10 尚無環境 |
| AC02 | PASS | Debug／Release 真實命令列與原生視窗測試沿用 Phase 4 證據；v0.9.0 無 UI 錯誤參數重跑通過 | Phase 4 `native-release.txt`、UT01～03、Phase 13 smoke | — |
| AC03 | NOT TESTED | 真實跨程序 parent、三種 host DPI context、resize／退出／刷新已有基線證據 | Phase 3 native、Phase 4 native Release | v0.9.0 未重跑互動 host；缺實體混合 100%／150%／200% 桌面與 Windows 設定頁 |
| AC04 | NOT TESTED | 原生設定、preview、ChooseFont、提交／取消閉環已有 Phase 3 證據；v0.12.0 五場景及八個色彩 radio resource 通過 | Phase 3 report／截圖、Phase 18 smoke | 安裝後 Windows 設定頁調度及新增選項互動預覽未測 |
| AC05 | PASS | Gregorian／鐘角／layout tests 與置中版面 GDI fixture | Phase 9 fixtures、UT04～07、17～19 | — |
| AC06 | NOT TESTED | 倒數原生輸入與取消已有證據 | Phase 3 native、UT08～09、MT07 | 真實閒置／恢復登入未測 |
| AC07 | NOT TESTED | 七段、沙漏、進度、最後十秒、四次閃爍測試與 MT07 通過 | UT10～15、Phase 2 fixtures | 實際睡眠／恢復／校時未測 |
| AC08 | NOT TESTED | 雙 4K、負 X、共用 generation 與合成 display-change 清理已有基線證據 | UT25、MT03、Phase 4 native | 旅行模式多螢幕／混合 DPI、拔除顯示器與登出未測 |
| AC09 | NOT TESTED | 七種固定色、自動色彩、四字型、fallback、深棕面板對比與極端點數有測試／fixture | UT18～22、UT40、UT44、visual-reference | 200% 實體 dialog、長時間自動換色與混合 DPI 未測 |
| AC10 | PASS | 專用測試 key 與 fake store 驗證型別、schema 8、TravelStyle 0～4 與 ColorPreset 0～7 round-trip、舊 schema 回退、取消、rollback、未知值 | UT20～24、UT26、UT44 | — |
| AC11 | NOT TESTED | 指定鍵鼠、4px／500ms、同程序焦點與清理已有 Phase 1／4 證據；旅行 WebView accelerator／mouse poll 已建置 | Phase 1／4 native、Phase 6 build | 旅行 player 實際輸入退出、外部 foreground 成功分支與登出未測 |
| AC12 | NOT TESTED | 前兩種 GDI 模式 Release preview 各 30 分鐘、各 59 次 cache 循環穩定 | Phase 4 resource report | 正式 Countdown 10 Hz 與旅行 WebView2 30 分鐘資源觀察未測 |
| AC13 | NOT TESTED | v0.14.1 Setup 已編譯且版本／hash 一致 | Phase 22 report、`SHA256SUMS.txt` | 沒有實際安裝、列舉、移除或覆蓋升級；遠端不觸發 UAC |
| AC14 | NOT TESTED | 非 System32 helper code 4 且四個螢幕保護 registry 值不變；合法路徑的目標值與安全值排除有單元測試 | Phase 18 smoke、UT44 | 原使用者成功路徑、實際 SPI 套用、另一管理員 UAC、直接提權 installer 未測 |
| AC15 | PASS | README、規格、來源紀錄、測試、`.scr`、Setup、hash、Phase 6～13 與逐項報告均存在 | repository 交付清單 | 成品明列 NotSigned 與未測限制 |
| AC16 | NOT TESTED | 五場景 HTML／CSS shell、GDI fallback、player rect／外部 caption 的 deterministic test 及 fixture 通過 | UT26、UT34、UT39、Phase 15 report | 五種場景的實際 player、地點文字及多螢幕未觀察 |
| AC17 | NOT TESTED | 8 個 seed 隨機排序、排除上一來源、60 秒 monotonic 邊界、parser／狀態模型及 8／8 HTTP probe 通過 | UT27～UT30、Phase 6 source health | 實際 `PLAYING`、輪換、stall／error failover與 shutdown 未測；UT31～UT32 未覆蓋完整整合矩陣 |
| AC18 | NOT TESTED | 前兩模式／preview 不建立 travel session；Runtime probe、host allowlist、CSP、fallback 與揭露已實作 | UT33 局部證據、README、source record | 實際斷線／Runtime 缺失及 outbound capture 未測；8 個來源權利條款未逐一抽查 |
| AC19 | PASS | Cargo package／binary、Rust crate、SCR／Setup、resources、manifest、window、Registry、WebView2 path、User-Agent、腳本、文件與 Pages 均已統一 | Phase 8 report、resource smoke、repository scan | Git commit／tag 歷史依版本紀錄保留 |
| AC20 | PASS | 大型桌曆時鐘與番茄鐘置中於 64%W×60%H，小型 preview 及旅行維持 88%W×82%H；所有圖形落在指定區域 | layout tests、Phase 9 的 37 張 GDI fixtures | GDI 畫面證據；沒有開啟全螢幕或系統設定視窗 |
| AC21 | NOT TESTED | Runtime 唯讀偵測、19 個 policy checks、官方 Bootstrapper 簽章與封裝通過 | Phase 10 report、Phase 11 package.txt | 實際 Runtime 安裝、斷網／Proxy、失敗重試、重啟與多帳號 UAC 尚未驗證 |
| AC22 | NOT TESTED | 每螢幕獨立 TravelHost、事件路由、來源失敗隔離、清理與真實狀態文字的回歸測試通過 | Phase 11 report、49 個 Rust tests、40 張 GDI fixtures | 實際雙螢幕 WebView2 同時播放、各自輪換、混合 DPI 與完整資源清理尚未觀察 |
| AC23 | PASS | 鐘面數字縮至 0.30R、中心 0.58R；字型量測與刻度間距、小預覽／直向／4K fixtures 通過 | Phase 12 GDI、既有 GDI 測試擴充 | 非互動離屏驗證 |
| AC24 | NOT TESTED | 兩張原創 AI 擬真 PNG 內附、WIC 解碼／快取、40 張 GDI、4 張靜態 HTML 版面通過 | Phase 12 report、travel-artwork、geometry.json | 實際 YouTube player 與窗框完整性尚未觀察 |
| AC25 | NOT TESTED | schema 5、0／1～1440 分鐘保存、舊設定／非法值、隱藏控制項、各螢幕計時與不切換復原測試通過 | UT36～38、Phase 12 package／smoke | 正式設定視窗 DPI／鍵盤互動、自訂間隔及不切換的實際長時間播放未測 |
| AC26 | NOT TESTED | 和風庭園原創 PNG 內附，WIC、GDI 與 HTML 皆使用同圖；50 張 GDI 與 6 張 HTML 幾何 fixture 通過 | Phase 13 report、travel-artwork、geometry.json | 實際 YouTube player 與庭園窗框完整性尚未觀察 |
| AC27 | NOT TESTED | schema 8 鐵灰 round-trip；雪藍／琥珀／鐵灰 fixture 與自動色彩 120 秒邊界、七色完整循環測試通過 | UT26、UT40、UT44、Phase 18 fixtures／smoke | 正式設定視窗與多螢幕長時間自動換色未測 |
| AC28 | NOT TESTED | 四份指定 playlist ID、IFrame API refresh／shuffle／隨機選片、結束重播及 5 秒清單 timeout 的 deterministic test 通過；4／4 embed HTTP 200 | UT41、Phase 14 source health | 正式 WebView2 的清單內容、地區限制、嵌入允許及實際 `PLAYING` 未測 |
| AC29 | NOT TESTED | 御運轉士窗孔內 16:9 cover 與兩側設備、地方散策徑向暗角及無血管素材通過 WIC／GDI／HTML fixture；曲線模糊眼瞼與閉合／展開兩段由測試固定 | UT34、UT39、UT41、UT43～UT46、Phase 20 report | 實際影片中的裁切／眨眼、多螢幕與 GPU 表現未觀察 |
| AC30 | NOT TESTED | 15 個共用 Pascal policy checks 驗證同版、較舊／較新／異常版本、接受／拒絕移除、靜默衝突及命令解析；本機唯讀辨識既有 0.10.0.0 | UT42、Phase 16 policy report | 遵守遠端限制，未顯示詢問、未執行舊 uninstaller、未安裝本版或觸發 UAC |
| AC31 | NOT TESTED | 新顯示名稱、3:01～8:59 隨機 load/replay、控制列／字幕 hooks／註解停用、輪換前 prepare、地方散策完整／定時／緩衝眨眼結構均通過 source test 與 smoke | UT43、UT46、Phase 20 report | 實際 WebView2 播放器 UI、字幕偏好、seek、網路節流與動畫觀感未測 |
| AC32 | NOT TESTED | Setup 預設設定 60 秒閒置啟動、保留安全值；游標生命週期、柔化眨眼、鐵灰與深棕玻璃面板均通過非互動檢查 | UT44、Phase 18 report、55 張 GDI fixtures | 實際 Setup／UAC、系統閒置準時啟動、游標與動畫觀感需在可互動 Win10 補驗 |
| AC33 | NOT TESTED | 初版實色橫帶已移除；平滑上下／左右漸層、圓角 clipping、細唇邊與局部柔光在 0×0～4K、96～288 DPI、50 次資源循環及 55 張 fixture 通過 | UT45、Phase 19 report、Phase 19 fixtures | 實際全螢幕桌面觀感需在可互動 Win10 補驗 |
| AC34 | NOT TESTED | 御運轉士填窗、兩段眨眼、隨機起點、字幕關閉 hooks、游標停放與 Setup 完成頁 `/c` 選項均已實作並通過 build／source／HTML／package 驗證 | UT46、Phase 20 report、Phase 20 HTML fixtures | 實際播放、游標、字幕偏好、動畫與 Setup 完成頁互動需在可互動 Win10 補驗 |

## UT01～UT46

| ID | 狀態 | 證據／限制 |
| --- | --- | --- |
| UT01～UT03 | PASS | `tests/cli_tests.rs`、`tests/native_modes.rs`；含真實 Release command line |
| UT04～UT07 | PASS | `tests/time_layout.rs` Gregorian 與連續鐘角 |
| UT08～UT09 | PASS | duration 欄位拒絕、邊界與 round trip |
| UT10～UT16 | PASS | deadline、ceil、ratio、最後十秒、四次暗半週期、segment、xorshift |
| UT17～UT19 | PASS | 五種 aspect、1.349／1.350、0×0～4K、96～288 DPI、安全位移 bounds |
| UT20～UT24 | PASS | config／registry fallback、schema、font、取消與注入失敗 rollback |
| UT25 | PASS | 共用 generation、idempotent shutdown、最後視窗 quit |
| UT26 | PASS | schema 3 的 `JapanTravel=2` 相容；schema 6～8 的 `TravelStyle=0～4` 與 `ColorPreset=0～7` round-trip；舊 schema 不誤解新版值，future schema 保護 |
| UT27～UT28 | PASS | 8 個候選唯一、固定 seed、排除上一來源、59,999／60,000 ms 與時間跳躍 |
| UT29～UT30 | PASS | detail HTML、entity、host／ID 拒絕、固定 embed URL 與 script escaping |
| UT31 | NOT TESTED | player state transition test 與 live HTTP probe 已通過；尚無完整 mock transport 的 3xx／404／TLS／取消與 mock player navigation 矩陣 |
| UT32 | NOT TESTED | generation/channel/controller teardown 已實作並通過 Clippy／建置；尚無直接 late completion／shutdown 重入整合測試 |
| UT33 | NOT TESTED | `/p`、`/c` 與前兩模式不建立 `TravelSession` 的程式路徑已審查；尚無 fake transport request counter 或 outbound capture |
| UT34 | PASS | 五場景 GDI 小尺寸／800×450 fixture、每種 HTML shell 單一 player、正確場景 class／aria label；Phase 20 的御運轉士 cover 與地方散策外緣幾何通過 |
| UT35 | PASS | 兩個 message-only HWND 的事件路由、第二來源失敗不改第一螢幕播放計時、各自地點、關閉所有 host、弱參照回收；前兩模式不建立 host；未建立 WebView2 |
| UT36 | PASS | schema 1～4 預設 1 分鐘、schema 5 合法值 round-trip、型別／長度／範圍錯誤回退、future schema 保護；countdown-only save 遷移也保留舊版 1 分鐘行為 |
| UT37 | PASS | 1～1440 整數解析、空白／負數／小數／全形／超長文字拒絕；message-only parent 下不可見控制項的讀取、啟用與停用；未開正式設定視窗 |
| UT38 | PASS | 0 無排程／預抓、1／2／1440 分鐘邊界與最後 1 分鐘預抓；不切換時來源故障仍走重試；不等於實際串流長測 |
| UT39 | PASS | 五張 1586×992 PNG 解碼成 32-bpp、快取重用、無效 PNG 拒絕、既有 MTA 生命週期保留與離屏 GDI 繪製 |
| UT40 | PASS | 自動色彩 119,999／120,000 ms 邊界、720,000 ms 鐵灰及 840,000 ms 回到第一色的七色循環 |
| UT41 | PASS | 四份指定 playlist 映射、官方 player 的 load／shuffle／getPlaylist／隨機 loadVideoById、同片重播、散步眨眼 keyframes 與延遲載入結構 |
| UT42 | PASS | 15 個 Inno Pascal 版本 policy checks；最低權限、無 wizard／prompt／Exec／registry write；唯讀產品偵測命中 0.10.0.0 |
| UT43 | PASS | source assertions 固定五個新 labels、`controls=0`、`cc_load_policy=0`、`iv_load_policy=3`、180 秒 load/replay、20～30 秒輕眨、BUFFERING 深眨、780 ms 完全閉眼與 `window.travel.prepare` 注入 |
| UT44 | PASS | helper 只規劃 path／active／timeout 且排除安全值；schema 8 鐵灰 round-trip；七色自動循環；柔化眼瞼漸層與完整閉合；55 張 GDI fixture 含鐵灰及深棕玻璃面板 |
| UT45 | PASS | `GradientFill` 與圓角 clipping 納入既有極小尺寸、DPI、50 次 GDI 資源循環及 fixture export；新面板不使用貫穿全寬的實色亮／暗帶 |
| UT46 | PASS | source assertions 固定御運轉士 46.4%×35.2% 窗孔與 16:9 cover、地方散策 0.4% 外緣及曲線模糊閉／睜兩段、181～539 秒隨機函式、字幕 hooks；游標停放純函式測試與 Setup `/c` Run entry 編譯成功 |

Phase 5 的 helper 精確旗標與非 System32 真實 binary 拒絕測試在 v0.9.0 也重跑通過。來源 probe 沿用 Phase 8；本次 Runtime 只以 Setup 共用函式讀取 registry，沒有建立 WebView2 controller 或執行 Runtime installer。

## MT01～MT23

| ID | 狀態 | 已取得的證據 | 缺少條件 |
| --- | --- | --- | --- |
| MT01 | NOT TESTED | Win10 build與歷史 `/s`／`/p`／`/c` 測試 | v0.9.0 安裝、列舉、系統 preview、解除安裝未測 |
| MT02 | NOT TESTED | 1920×1080 fixture／96-DPI preview 長測 | 無實體單螢幕 100% 全螢幕環境 |
| MT03 | PASS | 雙 4K／150%、左側負 X、全覆蓋、共用 generation、內部焦點的 Phase 4 基線 | — |
| MT04 | NOT TESTED | 4K／150% 與直向 fixture | 無混合 100%／150%／200%、實體直向與 200% dialog |
| MT05 | NOT TESTED | 真實 host 保存後 2 秒刷新、取消不寫的 Phase 3 證據 | Windows 設定頁中的已安裝 v0.9.0 尚未測 |
| MT06 | PASS | unaware／system／PMv2 host，四種 resize，parent 關閉退出的 Phase 3 證據 | — |
| MT07 | PASS | 1 秒、5 秒、30 分鐘、99:59:59 原生流程與時間邏輯 | — |
| MT08 | NOT TESTED | 手動 `/s` 歷史測試不採計本項 | 真實閒置、自動啟動、三種模式、恢復登入開／關與安全桌面 |
| MT09 | NOT TESTED | 純 deadline／校時／跨午夜模型已測 | 實際睡眠／恢復、校時與跨午夜 |
| MT10 | NOT TESTED | 合成 display change、session end、close 已測 | 實際拔線、外部前景成功切換分支與登出 |
| MT11 | NOT TESTED | helper 身分檢查已實作 | 獨立標準帳號與另一管理員 UAC |
| MT12 | NOT TESTED | helper 非安裝路徑拒絕已測 | Setup 從已提權程序啟動及安裝後 elevated helper |
| MT13 | NOT TESTED | 固定 AppId、版本比較、詢問／拒絕、先移除再安裝、靜默衝突阻擋及保留偏好規則已編譯；15 個 policy checks 通過 | 實際較舊／較新版升降級、拒絕、同版修復、檔案使用中、解除安裝與 UAC |
| MT14 | NOT TESTED | 前兩種 GDI 模式 Release preview 各 30 分鐘與 59 次循環通過 | 全螢幕 10 Hz 長測與實體 DPI 變更 |
| MT15 | NOT TESTED | fresh offline hash、靜態 CRT、無 VC/UCRT／WebView2Loader 動態 import | 無未裝開發工具／VC++ Redistributable 的乾淨 Win10；缺 Runtime fallback 也未實機驗證 |
| MT16 | NOT TESTED | player shell、狀態模型、來源與 60 秒 deadline tests PASS | 實際 `PLAYING`、靜音、地點吻合、至少 5 次輪換、自訂分鐘與不切換長測 |
| MT17 | NOT TESTED | 錯誤狀態與 GDI fallback code 已建置；Runtime probe PASS | 實際 tw.live／YouTube 斷線、所有候選失效及 Runtime 缺失 |
| MT18 | NOT TESTED | 每個螢幕一個 player；兩螢幕路由、失敗隔離與清理的非互動回歸測試通過；既有雙 4K 基線 | 旅行 player 的多螢幕、150%／200%、4K、直向及拓撲變化 |
| MT19 | NOT TESTED | controller 重用與 shutdown code 已審查 | 30 分鐘、至少 29 次切換及 parent＋WebView2 descendant resource 記錄 |
| MT20 | NOT TESTED | host allowlist、CSP、new-window／download／permission deny 已實作 | 實際 outbound isolation capture |
| MT21 | NOT TESTED | installer 預設 task、三個 HKCU 目標值、SPI 呼叫、設定通知、游標隱藏／還原與漸層眨眼已編譯並由單元／source test 驗證 | 遠端限制下未執行 Setup、UAC、實際閒置啟動、全螢幕游標或動畫觀察 |
| MT22 | NOT TESTED | 1920×1080 與 800×369 離屏成品已人工檢查，面板由近黑褐核心、平滑光衰減與琥珀瓶壁組成，沒有舊版分格橫帶 | 需在可互動 Win10 全螢幕確認實際顯示器的黑階、色偏與玻璃觀感 |
| MT23 | NOT TESTED | Phase 20 source、headless HTML、Release 與 Setup 編譯均通過，且遠端流程未啟動產品 UI | 需在可互動 Win10 觀察御運轉士、閉／睜動畫、字幕偏好、游標位置，以及一般／靜默 Setup 完成流程 |

Windows 11：`NOT TESTED`，依使用者指示延期；這不阻擋目前 Windows 10 候選版交付，也不能被寫成已支援通過。

## v0.14.1 成品

| 成品 | Bytes | SHA-256 | 狀態 |
| --- | ---: | --- | --- |
| `dist/tools-screensaver-tzk.scr` | 9,570,816 | `49a2d2a02608aaf574f09f7fabe970770c26f642fea771ade4995f4d384ef7ab` | Build／smoke PASS；NotSigned |
| `dist/tools-screensaver-tzk-Setup.exe` | 12,614,509 | `bc58d235bf4f1f671153b906a9738125c14248e7b841688f014d8b87e26a0642` | Package PASS；實際安裝／升級 NOT TESTED；NotSigned |

Phase 5 的 v0.1.1 hash 保留在 Phase 5 報告與該 Release，不再列為目前成品。完成 Windows 10 完整驗收仍需在可互動本機環境補做上述必要項目；遠端工作階段不觸發 UAC。程式碼簽章憑證尚未提供，v0.14.1 成品維持 NotSigned。
