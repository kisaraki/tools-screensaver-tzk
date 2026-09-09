# tools-screensaver-tzk 開發規格書

> 文件版本：2.7（修訂版）<br>
> 修訂日期：2026-09-09<br>
> 用途：供 Codex 分階段開發、審查與驗收<br>
> 目標：Windows 10／11 x64、Rust 2021、原生 Win32／GDI<br>
> 目前必要驗證平台：Windows 10 x64；Windows 11 延後驗證（依使用者 2026-09-04 指示）
> 執行期原則：單一產品 `.scr`；標準桌曆暨時鐘與離機作業番茄鐘可離線使用；日本旅行模式僅在 `/s` 經 HTTPS 連線，並使用已安裝的 Microsoft Edge WebView2 Evergreen Runtime；無外部字型檔<br>
> 本文件描述應實作的產品；文件完成不代表程式已開發、編譯或通過實機驗收。

## 0. 文件使用方式與修訂決策

### 0.1 範圍與規範用語

- 「必須／不得」是必要驗收條件；「應」是預設實作方式；「可」是選配，未實作不影響必要驗收。
- 使用者當次明確任務決定工作範圍。當任務只要求修改規格時，不得因本文包含開發指令就開始安裝工具、開發程式、改登錄檔或執行安裝程式。
- 實作時遵守適用的 `AGENTS.md` 與使用者指示；本文中的網站、截圖、程式碼片段是參考資料，不是額外授權。
- 產品行為以第 1～16 節為準；第 17～18 節是可驗證的測試與完成條件；第 19 節描述交付順序，不重複另定行為。
- 原稿的需求、四種字型模式、原有兩種畫面及 Phase 0～5 均保留。v2.1 的 Phase 14 新增指定影片清單及兩種旅行模式；v2.2 的 Phase 15 改良列車駕駛室與散步暗角；v2.3 的 Phase 16 新增安裝版本衝突流程；v2.4 的 Phase 17 更新顯示名稱與播放體驗；v2.5 的 Phase 18 完整設定閒置啟動、提前隱藏游標、柔化眨眼邊緣，並新增鐵灰色；v2.6 的 Phase 19 重製暗色藥劑瓶玻璃番茄鐘面板；v2.7 的 Phase 20 修正旅行框景、眨眼、隨機起點、字幕控制、游標停放與安裝完成設定入口。

### 0.2 v1.2～v2.7 的主要修訂

| 主題 | 明確決策 | 位置 |
| --- | --- | --- |
| 第三種畫面 | v1.3 新增「日本旅行模式」，內部識別 `JapanTravel`；不改動既有 `TimeDate=0`、`Countdown=1` | 1、8.6、10 |
| 網路邊界 | 只有 `/s` 的 `JapanTravel` 可連線；`/p`、`/c` 與另外兩種模式維持零網路請求 | 7、8.6、11、17 |
| 來源與輪換 | 自在飛行、列車旅行、御運轉士及地方散策使用四份指定 YouTube playlist；啟動時由 IFrame API 更新清單並隨機選片，輪換前預選下一候選。和風庭園保留 tw.live 8 個 camera seed；可選 1～1440 分鐘或不切換；失敗有界重試及離線 fallback | 8.6、10～11、16～17 |
| 旅行場景 | v2.1 新增 `TrainCab=3` 與 `Walking=4`；v2.2 將列車駕駛影片孔改為中央 16:9，左右加入擬真設備，散步改用全畫面強烈攝影暗角，中央約 65% 為主要視域，不含眼球、皮膚或血管 | 8.6、10～11 |
| 播放器與旅行框 | 每個螢幕以本機 HTML／CSS shell 呈現完整 WebView2 播放器、所選旅行場景及 player 外的地點／狀態；preview、等待與錯誤 fallback 使用對應 GDI 靜態畫面 | 8.6 |
| 旅行多螢幕 | 正式 `/s` 每個螢幕各建立一個 autoplay player，獨立來源、計時、狀態與錯誤路由；預覽不得誤標成連線失敗 | 5.2、8.6 |
| 旅行預覽 | `/p` 與 `/c` 只畫無網路的 GDI 靜態示意，不建立 WebView2 或探測公開網站 | 7.3、11.3 |
| WebView2 Runtime | 使用靜態 WebView2 loader；v1.7 的 Setup 偵測並補裝電腦層級 Runtime，可取消以使用離線模式；`.scr` 缺少 Runtime 時仍只顯示 fallback，不下載或提權 | 2.2、8.6、15、17、19 |
| 安裝版本衝突 | v2.3 使用固定 AppId 的 uninstall metadata 比對已安裝與 Setup 版本；不同版本須詢問，接受後先完整移除舊版才安裝，拒絕或移除失敗即停止；靜默衝突不得自動移除 | 15.4、17～19 |
| Registry schema | v1.9 schema 5 新增 `TravelSwitchMinutes`；v2.0 升為 6；v2.1 升為 7，加入 `TravelStyle=3/4`。舊設定逐欄相容，來源清單不寫入 registry | 10 |
| 桌曆時鐘色彩 | v2.0 新增的兩色於 v2.4 顯示為雪藍與琥珀；v2.5 新增鐵灰色。自動模式依共同 `GetTickCount64` 時基，每 120 秒循環七種實色，多螢幕保持一致 | 9、10～11 |
| 播放體驗 | v2.7 每次載入由 3:01～8:59 隨機起播；控制列、鍵盤與註解以播放器參數停用，並於 ready、playing、API module 變更時要求關閉字幕；輪換前預選來源並預熱；地方散策使用明確閉合、展開兩段動畫，順暢時每 20～30 秒輕眨、緩衝時溫和加深 | 8.6、11、17、19 |
| 安裝後閒置啟動 | v2.5 的 `setcurrent` 預設勾選；以原使用者身分設定程式路徑、啟用狀態與 60 秒逾時，透過系統 API 與通知立即套用，保留登入安全設定；群組原則可能覆蓋 | 15.2～15.3、17～19 |
| 游標與眨眼 | 全螢幕先把游標停在主要螢幕外角的非 player 區域，再於第一個 surface 顯示前隱藏，所有退出路徑還原游標形狀；地方散策的上下眼瞼使用橢圓曲線與模糊黑暈 | 6.3、8.6、17、19 |
| 旅行框景與安裝後設定 | 御運轉士的 16:9 player 置中覆滿寬螢幕窗孔，以容器裁掉少量上下影像，不遮蓋窗框且不留左右黑帶；一般 Setup 完成頁預設勾選開啟 `/c` 設定，使用原啟動者身分且靜默安裝不開啟 | 8.6、15、17、19 |
| 鐵灰與暗色藥劑瓶玻璃 | 新增 `IronGray=7`、schema 8，自動模式循環七種實色；v2.6 移除分格式實色橫帶，番茄鐘面板改用近黑褐核心、上下連續光衰減、左右琥珀瓶壁折射、細銅色唇邊與局部柔光 | 8.3、9～11、17、19 |
| 產品識別 | v1.5 將 repository、Cargo package／binary、Rust crate、`.scr`／Setup、VERSIONINFO、manifest、視窗、Registry、WebView2 資料目錄、腳本、文件與 Pages 全數統一為 `tools-screensaver-tzk`；Rust 程式碼中的 crate 識別依語法正規化為 `tools_screensaver_tzk` | 2、10、13～15、19 |
| 離線模式畫面密度 | v1.6 將桌曆時鐘與番茄鐘的大型畫面置中於 64%W×60%H 安全區，降低視覺壓迫；小型 Windows preview 保留較大可視面積，日本旅行 player 不縮小 | 8.1～8.3、19 |
| 鐘面數字間距 | v1.9 的 12／3／6／9 目標字高由 0.40R 降為 0.30R，中心距離由 0.62R 內收到 0.58R，避免與外刻度黏連 | 8.2、17、19 |
| 增量階段 | 已完成的 Phase 0～5 保持歷史事實；第三模式由 Phase 6 實作、測試及發布 | 19.2 |
| 倒數每次先輸入 | 保留；補上系統閒置／安全桌面實機驗證，不能僅憑直接執行 `/s` 宣稱支援 | 6.2、17.3 |
| 全螢幕顯示時機 | 先建立隱藏視窗，全部成功後才顯示；建立時不加 `WS_VISIBLE` | 6.1 |
| 多螢幕同步 | 共用時基與每次更新的不可變快照；不得在各視窗繪圖時各自取時 | 5.2、8.3 |
| 設定更新 | `/s` 使用啟動快照；`/p` 每秒重新讀取設定；`/c` 預覽直接讀取草稿 | 7、11 |
| 小型倒數預覽 | `/p` 顯示上次時間與滿量沙漏；`/c` 固定 5 分鐘、半量沙漏，總時間視為 10 分鐘 | 7、11.3 |
| 字型大小 | 保存使用者點數；實際顯示以角色比例及可用矩形適配，字體大小不能造成裁切 | 9.4 |
| 畫面與參考圖差異 | 參考圖是亮紅色；產品仍保留亮綠預設與深紅選項，不宣稱逐像素相同 | 8.2、9.1 |
| 記憶體門檻 | 以基礎開銷加各螢幕 buffer 預算計算，移除不適用於 4K 的固定 20 MB 上限 | 17.4 |
| 安裝架構 | 第一版只接受原生 x64 Windows；排除 ARM64 模擬環境 | 2.1、15.1 |
| 安裝使用者 | 系統檔案需提權；設定目前螢幕保護程式必須回到原啟動使用者身分 | 15.3 |
| 階段停點 | 單階段任務完成後停止；明確授權全部階段時，可依序持續執行 | 19.1 |
| 安裝內部入口 | 新增單用途 `--install-set-current`，供原使用者套用已勾選安裝工作，不新增畫面模式 | 15.3 |
| 解除安裝的 HKCU | 採原稿允許的「提示」方式，避免提權後清除錯誤帳號的設定 | 15.4 |
| 未驗證項目 | 必須逐項記錄；`NOT TESTED` 不能計為通過 | 18 |

### 0.3 開發前固定事項

- 文件版本與軟體版本分開；文件 v2.7 對應旅行播放與安裝完成設定修正的目標軟體版號為 `0.13.0`。
- 不虛構公司或作者。專案擁有者已於 2026-09-05 指定以 MIT License 公開發布，copyright holder 使用 GitHub 帳號 `kisaraki`；CompanyName 可留空。
- 技術預設可依本文件直接實作；若實驗證明必要條件互斥，先提交具體失敗證據與最小變更方案，不可自行刪除需求或假報通過。

### 0.4 目前平台驗收範圍

- 依使用者 2026-09-04 指示，目前以 Windows 10 x64 進行開發與必要驗證；Windows 11 環境尚未具備，延後測試。
- Windows 11 保留為未來相容性目標，記錄為 `NOT TESTED（延期，非目前必要驗收項）`，不阻擋目前各 Phase 的完成或 Windows 10 範圍的交付。
- Windows 10 上原有功能、DPI、多螢幕、系統整合與各階段品質條件仍適用。後續取得 Windows 11 環境，再補做同等相容性驗證。
- 報告須清楚區分「Windows 10 驗證通過」與「Windows 11 尚未驗證」，不得將延期寫成已通過。

## 1. 專案目標與需求追蹤

建立可由 Windows「螢幕保護程式設定」選取的 `tools-screensaver-tzk.scr`，提供「標準桌曆暨時鐘模式」、「離機作業番茄鐘模式」與「日本旅行模式」三種螢幕保護畫面。內部程式與登錄值依序使用 `TimeDate=0`、`Countdown=1`、`JapanTravel=2`。前兩種畫面由 Rust 呼叫 Win32 GDI 繪製且可離線執行；日本旅行模式可選「自在飛行」、「列車旅行」、「和風庭園」、「御運轉士」或「地方散策」，在每個螢幕以本機 HTML／CSS shell 與 WebView2 播放線上影片，preview、播放器等待及錯誤 fallback 由 GDI 繪製對應靜態場景。

### 1.1 必要功能

| ID | 需求 | 規格 | 主要階段 | 驗收 ID |
| --- | --- | --- | --- | --- |
| R01 | `/s` 全螢幕、`/p` 系統預覽、`/c` 設定 | 4、6、7、11 | 1、3 | AC02～AC04 |
| R02 | 左指針鐘、右當月月曆；窄畫面改上下排列 | 8.2 | 2 | AC05 |
| R03 | 倒數開始前輸入時、分、秒 | 6.2、11.5 | 3 | AC06 |
| R04 | 七段數字、沙漏、LCD 面板、剩餘比例線 | 8.3 | 2 | AC07 |
| R05 | 多螢幕、負座標、混合 DPI | 5～8 | 1、2、4 | AC08 |
| R06 | 七種固定顏色、自動換色、四種字型模式與系統選字型 | 9、11 | 3、13、18 | AC09、AC27、AC32 |
| R07 | HKCU 保存、取消不提交、異常資料回退 | 10、11 | 3 | AC10 |
| R08 | 全螢幕輸入退出、預覽不搶焦點 | 6.3、7 | 1 | AC03、AC11 |
| R09 | 群組位移、雙緩衝與資源穩定 | 8.4、8.5、12 | 2、4 | AC12 |
| R10 | 單一 `.scr` 與 Inno Setup 安裝 EXE | 13～15 | 0、5 | AC01、AC13 |
| R11 | 安裝／移除不擅改安全設定、不影響其他帳號 | 15 | 5 | AC14 |
| R12 | 實際測試紀錄、版本與雜湊可追溯 | 17～19、22 | 4、5 | AC15 |
| R13 | 「自在飛行」、「列車旅行」與「和風庭園」窗景、目前城市／地區及鏡頭名稱 | 7、8.6、10～11 | 6～7、13 | AC16、AC26 |
| R14 | 預設每 1 分鐘隨機換來源、來源健康檢查、有界 failover；可設定分鐘或不切換 | 5、8.6、10～11、16～17 | 6、12 | AC17、AC25 |
| R15 | 前兩模式與所有 preview 無網路；Runtime／斷線安全 fallback 與第三方揭露 | 2、7～8、15～18、22 | 6 | AC18 |
| R16 | 產品、原始碼、建置、安裝、設定路徑、文件與公開網站統一使用 `tools-screensaver-tzk` 識別 | 0、2、10、13～15、19 | 8 | AC19 |
| R17 | 桌曆時鐘與番茄鐘在一般畫面使用置中的友善可視面積，小型 preview 維持可讀 | 8.1～8.3 | 9 | AC20 |
| R18 | Setup 偵測與補裝電腦層級 WebView2 Runtime、可略過、失敗不冒充成功；遠端驗證不安裝 | 15.1.1 | 10 | AC21 |
| R19 | 每個螢幕獨立旅行播放器、來源輪換與狀態；共用退出並清理全部 host；預覽文字如實 | 5.2、8.6 | 11 | AC22 |
| R20 | 鐘面 12／3／6／9 縮小並內收，與外側刻度保持間距 | 8.2 | 12 | AC23 |
| R21 | 五種旅行場景使用內附原創 AI 擬真圖，預覽離線，完整 player 保持可見 | 7、8.6 | 12～14 | AC24、AC26、AC29 |
| R22 | 保存旅行來源切換分鐘，預設 1、0 為不切換、1～1440 為整數分鐘；舊設定相容，失效復原保持啟用 | 8.6、10～11 | 12 | AC25 |
| R23 | 和風庭園場景；雪藍、琥珀、鐵灰色與每 2 分鐘自動換色 | 8.6、9～11 | 13、17～18 | AC26～AC27、AC31～AC32 |
| R24 | 四種移動場景在啟動時更新指定 playlist 並隨機選片；和風庭園維持原來源 | 8.6、16～17 | 14、17 | AC28、AC31 |
| R25 | 御運轉士以設備包圍中央 16:9 影片；地方散策採全畫面強烈攝影暗角、中央約 65% 主要視域且無眼球／血管，並依切換與網路狀態眨眼 | 8.6、10～11 | 14～15、17 | AC29、AC31 |
| R26 | Setup 比對已安裝與本版版本；不同版本先詢問並移除舊版，拒絕、移除失敗或靜默衝突即停止，不能同時存在兩版 | 15.4 | 16 | AC30 |
| R27 | 更新五個顯示名稱；影片約從 3:00 開始、最小化播放器 UI、輪換前預備下一候選；地方散策依播放健康狀態眨眼 | 8.6、9、11 | 17 | AC31 |
| R28 | Setup 預設完整設定 60 秒閒置啟動且保留安全值；全螢幕顯示前隱藏並於退出還原游標；眨眼邊緣柔化；新增鐵灰色與深棕玻璃番茄鐘面板 | 6.3、8.3、8.6、9～11、15.2～15.3 | 18 | AC32 |
| R29 | 番茄鐘面板不得使用貫穿全寬的分格式實色帶；以平滑 GDI 漸層形成近黑褐透光核心、上下光衰減、左右琥珀瓶壁折射、細銅色唇邊與局部柔光 | 8.3、12、17～19 | 19 | AC33 |
| R30 | 御運轉士 player 填滿窗孔；地方散策分段閉眼／睜眼且邊緣曲線模糊；每次影片於第 3 分鐘後隨機起播並主動關閉字幕；游標先停至 player 外再隱藏；Setup 完成頁可開啟設定 | 6.3、8.6、15.2 | 20 | AC34 |

### 1.2 非目標

- 不使用 egui、FLTK、Qt、GTK、WinUI、WPF、遊戲引擎或外部瀏覽器。WebView2 是日本旅行模式內嵌官方影片播放器的唯一例外，不得擴張成通用瀏覽器或讓遠端頁面控制產品 UI。
- 不使用 Direct2D、DirectWrite、OpenGL、Vulkan；第一版固定 GDI。
- 不加入 `rand`、資料庫或使用者可編輯的執行期 JSON／TOML／INI 設定。旅行來源的 process-local session state 不是使用者設定，不能保存影音內容；Cargo 自身的 TOML 與測試報告不受此限制。
- 不做程式遙測、檢查更新、帳號登入、下載字型、錄影、回放、轉存、轉播或播放聲音。只有使用者已選定日本旅行模式且 `/s` 正式啟動時，才可連線至第 8.6 節規定的 HTTPS 來源。
- 不持續遮蔽或改造 YouTube 嵌入播放器；旅行框、地名與狀態放在 player 外。御運轉士只可由窗孔容器上下等量裁切 16:9 player 以填滿較寬開口；地方散策可在來源切換、20～30 秒自然間隔與緩衝事件短暫覆蓋 player。兩者都不得用覆蓋層持續遮蔽品牌或廣告。
- 不加入暫停、續跑、歸零、快捷鍵操作、百分秒、背景常駐計時或重啟後恢復倒數。
- 不自行驗證密碼、替代鎖定畫面、切換安全桌面或繞過 Windows 登入政策。
- 不產生 MSI，不支援 Windows 7／8／8.1、32 位元 Windows 或 ARM64。
- 位移只能降低固定畫面持續停留，不能宣稱保證避免 OLED 烙印。

## 2. 技術與相依性

### 2.1 平台及工具鏈

- Rust stable、Edition 2021，目標 `x86_64-pc-windows-msvc`。
- 最低 API 基線為 Windows 10 1703（build 15063）；目前主要實機驗收使用 Windows 10 22H2 x64，記錄確切 build。Windows 11 依第 0.4 節延後驗證。
- Microsoft C++ Build Tools，含 x64 MSVC linker 與 Windows SDK；SDK 本身不能取代 linker。不要求完整 Visual Studio IDE。
- Inno Setup 6.3 或以上的 6.x 穩定版為基準；若改用 7.x，必須固定實際版本並重跑安裝測試，不混用不同主版本的語法。
- 建立專案時固定實際驗證成功的 Rust 版本至 `rust-toolchain.toml`，包含 `rustfmt`、`clippy` 與目標；後續升級另行驗證。
- 元件安裝依使用者規則：Windows 優先 `winget`、其次官方工具；macOS 優先 Homebrew；Python 優先 `uv`、其次 `pip`。本專案建置不要求 Python。
- 安裝命令中的套件 ID／版本先以工具實際查核，不在腳本偷偷安裝工具、接受授權或要求提權。

### 2.2 Cargo 依賴基線

原有 Win32／GDI 路徑繼續以 `windows-sys` 實作；Phase 6 只為 WebView2 COM 加入固定版本的 `webview2-com` 與其需要的 `windows`，不得因此引入 async runtime、通用 HTTP client、瀏覽器框架或影音下載器。網路目錄讀取使用 WinHTTP。所有直接與傳遞依賴都納入 `Cargo.lock`。

```toml
[dependencies]
webview2-com = "0.39.1"
windows = { version = "0.62", features = [
    "Win32_Foundation",
    "Win32_System_Com",
    "Win32_UI_WindowsAndMessaging",
] }
windows-sys = { version = "0.61.2", features = [
    "Win32_Foundation",
    "Win32_Graphics_Gdi",
    "Win32_Security",
    "Win32_System_Diagnostics_Debug",
    "Win32_System_Environment",
    "Win32_System_LibraryLoader",
    "Win32_System_Memory",
    "Win32_Networking_WinHttp",
    "Win32_System_Registry",
    "Win32_System_SystemInformation",
    "Win32_System_Threading",
    "Win32_UI_Controls",
    "Win32_UI_Controls_Dialogs",
    "Win32_UI_HiDpi",
    "Win32_UI_Shell",
    "Win32_UI_WindowsAndMessaging",
] }

[profile.release]
opt-level = "s"
lto = true
codegen-units = 1
panic = "abort"
strip = "symbols"
```

- 版本與 feature 清單是實作起點；開發者以實際鎖定版本的 API／feature gate 編譯驗證，允許只為必要 API 作最小增減。
- `webview2-com` 的 MSVC 建置使用靜態 `WebView2LoaderStatic.lib`，交付不得新增旁載的 `WebView2Loader.dll`。這不包含 WebView2 Evergreen Runtime 本體；目標 Windows 10 仍須在執行期探測 Runtime 是否存在。
- `GetCommandLineW` 的 Environment feature、登錄檔／token 使用的 Security feature 不得遺漏；不要只憑 API 的 C header 猜測 Rust 模組位置。
- `Cargo.lock` 必須納入版控；建置、測試與 Clippy 使用 `--locked`。[windows-sys API 文件](https://docs.rs/windows-sys/0.61.2/windows_sys/)

### 2.3 執行檔與 CRT

GUI binary 使用 `#![windows_subsystem = "windows"]`，Debug／Release GUI 都不彈主控台。測試 harness 不套用 GUI subsystem。

`.cargo/config.toml` 固定目標與靜態 CRT：

```toml
[build]
target = "x86_64-pc-windows-msvc"

[target.x86_64-pc-windows-msvc]
rustflags = ["-C", "target-feature=+crt-static"]
```

因此正式來源檔案路徑統一為：

```text
target\x86_64-pc-windows-msvc\release\tools-screensaver-tzk.exe
```

靜態 CRT 是建置設定，仍須檢查成品 PE imports 並在未安裝開發工具／VC++ Redistributable 的乾淨系統測試。Windows 內建 DLL 與 API-set imports 合法，不要求完全無 DLL。[Rust：C runtime linkage](https://doc.rust-lang.org/reference/linkage.html#static-and-dynamic-c-runtimes)

### 2.4 Unicode 與指標

- 優先使用 `W` API；傳入 UTF-16 buffer 的 NUL、長度單位與生命週期必須正確。
- `HWND`、`WPARAM`、`LPARAM`、指標以指標寬度處理；不可用 `u32` 存 HWND。
- 只在 API 明訂字串長度時排除或包含 NUL，不把 bytes 與 UTF-16 code units 混用。
- 對來自登錄檔或 UI 的資料先驗證，再傳給 FFI。

## 3. 專案結構與模組責任

```text
tools-screensaver-tzk/
├─ .cargo/config.toml
├─ .gitignore
├─ rust-toolchain.toml
├─ tools-screensaver-tzk_Codex_Spec.md
├─ assets/
│  ├─ app.ico
│  └─ generate-icon.ps1
├─ docs/
│  ├─ visual-reference.md
│  └─ acceptance-report.md
├─ installer/setup.iss
├─ resources/
│  ├─ app.manifest
│  ├─ resource.h
│  └─ resources.rc
├─ scripts/
│  ├─ build.bat
│  ├─ package.bat
│  └─ smoke-test.ps1
├─ src/
│  ├─ main.rs
│  ├─ lib.rs
│  ├─ app.rs
│  ├─ cli.rs
│  ├─ config.rs
│  ├─ registry.rs
│  ├─ countdown.rs
│  ├─ calendar.rs
│  ├─ layout.rs
│  ├─ seven_segment.rs
│  ├─ render.rs
│  ├─ gdi.rs
│  ├─ font.rs
│  ├─ monitor.rs
│  ├─ window.rs
│  ├─ dialog.rs
│  └─ utf16.rs
├─ tests/
│  ├─ cli_tests.rs
│  ├─ config_tests.rs
│  ├─ calendar_tests.rs
│  ├─ countdown_tests.rs
│  └─ layout_tests.rs
├─ build.rs
├─ Cargo.toml
├─ Cargo.lock
├─ LICENSE
└─ README.md
```

- `main.rs` 只負責入口及結束碼；`lib.rs` 暴露可測試邏輯，解決 integration tests 無法匯入 binary 私有模組的問題。
- `cli`、`calendar`、`countdown`、`layout` 與設定值驗證須能以純輸入資料測試；`IsWindow`、時間讀取與 registry I/O 留在 adapter。
- `app` 持有程序狀態與更新協調；`window` 管理 HWND 生命週期；`gdi` 管理資源；`render` 只繪製快照。
- 可以依職責合併小模組；不得一次建立大量 TODO 空檔或把全部功能塞進單一巨大檔案。
- `.gitignore` 排除 `target/`、`dist/`、暫存與簽章秘密；交付 binary 不等於必須把 binary 納入 Git。

## 4. 啟動模式與 Windows 契約

### 4.1 命令列

使用 `GetCommandLineW` → `CommandLineToArgvW`，完成複製後 `LocalFree`。忽略 argv[0]，不得以空白自行切割完整命令列。

| 參數 | 行為 |
| --- | --- |
| 無參數 | 設定對話框 |
| `/s`、`-s` | 全螢幕 |
| `/p <HWND>`、`/p:<HWND>` | 指定 parent 的系統預覽 |
| `/c` | 無指定 owner 的設定 |
| `/c <HWND>`、`/c:<HWND>` | 指定 owner 的設定 |

- 上表 `/p`、`/c` 同樣接受 `-` 前綴，模式字母不分大小寫。
- HWND 只接受 ASCII 十進位非負整數；可有前導零，不接受正負號、十六進位、小數、空字串、尾隨垃圾或超出 `usize`。
- `/p 0`、不存在或已失效的 parent：安靜結束、code `2`，不得改成全螢幕。
- `/c 0` 或數值合法但 `IsWindow` 為假的 owner：退回無 owner；格式非法仍屬參數錯誤。
- 拒絕額外參數、重複模式、`/s:123`、`/s /c` 等歧義，不採「最後一個勝出」。
- Release 不接受 Debug 測試參數。第 15.3 節的安裝內部命令是唯一明列例外，不作公開使用模式。

| 結束碼 | 意義 |
| --- | --- |
| `0` | 正常完成、使用者退出或取消 |
| `2` | 命令列格式／預覽 parent 錯誤 |
| `3` | 視窗、資源、timer 或訊息迴圈初始化／執行失敗 |
| `4` | 安裝內部命令被拒絕或設定寫入失敗 |

### 4.2 系統整合邊界

- 系統設定按鈕呼叫 `/c`；小型顯示器區呼叫 `/p`；系統預覽按鈕或閒置啟動使用 `/s`。
- 不把自訂控制項注入 Windows 主設定頁；顯示自己的原生 modal dialog。
- 本專案自行實作入口及相容命令列，不連結 `Scrnsavw.lib`；因此不機械套用該 library 專屬的匯出函式／入口要求。
- 保留名稱字串 resource ID `1`、圖示與版本資訊。Windows 清單可能顯示檔名；驗收是可識別及選取，不保證所有系統版本採用同一名稱來源。
- `.scr` 更名不能代替真實系統整合驗證。參考文件包含歷史 API 與舊版範例，最終以目標 Windows 實測為準。[Microsoft：Handling Screen Savers](https://learn.microsoft.com/en-us/windows/win32/lwef/screen-saver-library)

## 5. 程序、狀態與生命週期

### 5.1 執行與所有權

- 所有 HWND、GDI 與 WebView2 COM controller 仍由單一 UI 執行緒擁有；不加入 async runtime或 busy loop。只有日本旅行模式可建立一個有 timeout 的 WinHTTP source worker，完成後透過 channel 與自訂視窗訊息把純資料交回 UI，worker 不得直接碰 HDC 或 COM controller。shutdown 不等待 worker；有界 request 結束後，晚到結果由已關閉的 receiver 回收。
- `GetMessageW(&mut msg, NULL, 0, 0)` 的結果分成 `>0` 派送、`0` 正常退出、`-1` 記錄錯誤並清理；不能只判斷非零。此處 `NULL` 表示依綁定型別傳入空 HWND。[GetMessageW](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getmessagew)
- `AppState` 由入口持有，直到所有視窗銷毀才釋放；每個 `WindowState` 只擁有自己的繪圖資源。
- `WM_NCCREATE` 從 `lpCreateParams` 接收狀態，存入 `GWLP_USERDATA`；`WM_NCDESTROY` 清空後回收一次。
- 必須明訂 `CreateWindowExW` 失敗前／後的 ownership transfer，測試 `WM_NCCREATE` 拒絕與 `WM_CREATE` 失敗，避免 double free 或未回收。
- 可能同步重入 callback 的 Win32 呼叫之前，結束 `&mut` 或 `RefCell` mutable borrow；不可跨 `DestroyWindow`、`SendMessageW`、modal dialog 等保留借用。
- 關閉集中由 `request_shutdown(reason)` 處理；狀態只能首次由 Running 轉成 ShuttingDown，重複事件無額外作用。
- 先停止更新，再關閉所有可見視窗，最後才銷毀更新 coordinator；最後一個必要視窗已回收才 `PostQuitMessage`。

### 5.2 共用快照

建議資料模型如下；命名可調整，責任不得混淆：

| 資料 | 保存內容 | 可否在 paint 修改 |
| --- | --- | --- |
| `AppConfig` | 已驗證的模式、顏色、字型及上次時間 | 否 |
| `ConfigDraft` | 設定對話框尚未提交的資料 | 由 UI 事件更新 |
| `CountdownState` | total、deadline、完成狀態 | 只能由 coordinator 更新 |
| `TravelSource` | camera ID、已清理的地點／鏡頭標題、已驗證 video ID | 否 |
| `TravelState` | generation、catalog 狀態、current source、播放狀態、切換 deadline、重試狀態 | 只能由 travel coordinator 更新 |
| `FrameSnapshot` | generation、本機年月日時分秒、remaining_ms、display_seconds、ratio、閃爍狀態 | 否 |
| `WindowState` | DPI、client size、back buffer、font cache、anchor | 尺寸／設定事件更新 |

- `/s` 只由一個 coordinator timer 取一次 `GetLocalTime`／`GetTickCount64`，更新同一份快照，再 invalidate 全部顯示器。
- 各 renderer 不取系統時間、不讀 registry、不啟動 timer。初始快照必須先於第一個可見 paint 建立。
- 畫面值按同一 generation 同步；不同螢幕的掃描／重繪不是硬體同步，不承諾同一微秒刷新，但不得出現獨立倒數累積漂移。
- 日本旅行模式由共用 coordinator 路由事件與處理退出，每個螢幕各持有一個 TravelHost，獨立保存來源、地名、generation、player channel、可選的輪換 deadline 與重試狀態。切換時間取自啟動設定快照，預設 1 分鐘；不切換時沒有排程 deadline 或預抓。每個 host 只建立一個 autoplay player 並在切換時重用 controller，不得每次切換累積 controller 或子程序。倒數模式的共用 deadline 不受旅行播放計時影響。

### 5.3 `unsafe` 與 callback

- 禁止 `static mut`；必要 `unsafe` 只用於 FFI、callback 與指標邊界，附具體 `// SAFETY:`。
- 正常輸入、失效 HWND 或 API 失敗都不能觸發 `unwrap()`、`expect()`、索引越界或 borrow panic。
- Rust panic 不得展開跨過 `extern "system"`；Release `panic=abort` 是最後保護，不是省略驗證的理由。
- 模型測試涵蓋 shutdown 重入；真正 FFI 的錯誤路徑仍須 Windows 驗證。

## 6. 全螢幕模式 `/s`

### 6.1 建立與顯示

1. 讀取並驗證一次設定；離機作業番茄鐘模式（內部識別 `Countdown`）先執行第 6.2 節輸入流程。日本旅行模式不顯示任何啟動對話框。
2. 以 `EnumDisplayMonitors(NULL, NULL, ...)` 列舉有效桌面顯示區域，讀取 `MONITORINFO.rcMonitor`。鏡像／重複矩形須避免重複覆蓋。
3. 每個有效矩形建立一個初始隱藏的 `WS_POPUP`，擴充樣式 `WS_EX_TOPMOST | WS_EX_TOOLWINDOW`。
4. 採完整 `rcMonitor`，不用 work area；支援負 X／Y，不假設主螢幕位於 `(0,0)`。
5. 全部視窗與必要資源準備完成後建立快照、顯示並開始更新；不得一開始就加 `WS_VISIBLE`。
6. 任一必要視窗建立失敗，關閉已建立視窗並以 code `3` 結束，不留部分黑畫面。

列舉失敗或無有效區域，才退回 virtual-screen metrics 建立單一視窗；fallback 仍檢查正尺寸、溢位及 allocation 上限。不得強制更換顯示解析度或獨佔顯示模式。

日本旅行模式在全部 surface 建立並顯示所選 GDI 旅行 fallback 後，為每個螢幕探測 Runtime，非同步建立各自的 WebView2 environment／controller 與本機 shell，再由各自背景 worker 檢查目錄與來源。本機 shell 檔案在所有播放器開始前寫入一次。各螢幕依自己的狀態顯示檢查中、播放中或錯誤；Runtime 失敗路徑持續顯示可退出的 GDI 旅行 fallback；不得讓使用者在等待期間看見未遮蔽桌面，也不得為等待 Runtime 或網路延後鍵鼠退出。WebView2 缺失、來源失敗或網路中斷不視為整個視窗初始化失敗。

### 6.2 離機作業番茄鐘模式啟動流程

```text
/s → 讀取模式
      ├─ 標準桌曆暨時鐘模式（TimeDate） → 準備全部視窗 → 顯示
      └─ 離機作業番茄鐘模式（Countdown） → 輸入時間
                       ├─ 取消／Esc／關閉 → 結束（0）
                       ├─ 驗證不通過 → 留在輸入框
                       └─ 開始 → 保存時間 → 準備全部視窗
                                             ├─ 失敗 → 清理結束（3）
                                             └─ 建立共用 deadline → 顯示並倒數
```

- 每次 `/s` 選用離機作業番茄鐘模式都顯示一次 `IDD_COUNTDOWN_INPUT`；多螢幕不能各問一次。
- 三個欄位範圍：小時 `0–99`、分鐘 `0–59`、秒 `0–59`；總秒數 `1–359999`。
- 預填上次按「開始」成功保存的值，首次 `00:05:00`。此值是上次接受的時間，不保證後續視窗一定建立成功。
- 只接受 1～2 個 ASCII 數字；空白、空字串、符號、全形數字與黏貼的非法內容都拒絕。`ES_NUMBER` 不取代程式驗證。
- 保存失敗：顯示明確錯誤、保留輸入、不建立全螢幕；使用者可重試或取消。
- deadline 在視窗準備完成、正式顯示前建立；對話框停留及視窗準備時間不扣除倒數。
- 輸入階段不套用一般滑鼠／鍵盤退出，不隱藏游標，也不先鋪黑全螢幕。

**保留的產品限制：** Windows 閒置自動啟動離機作業番茄鐘模式也會要求輸入。`/s` 本身不能可靠區分手動與系統啟動，不能自行推定使用者正在一般互動桌面。必須測試「繼續執行時顯示登入畫面」開／關及實際閒置啟動；若目標環境阻止互動，將該情境列 `FAIL` 或 `NOT TESTED`，不得繞過系統保護、偷偷改成自動倒數或宣稱驗收完成。此限制列入 README。

### 6.3 游標與退出

- 正式顯示後記錄啟動 tick 與 `GetCursorPos` 的螢幕座標；500 ms 寬限期只忽略滑鼠移動及初始化造成的啟用切換。
- `WM_MOUSEMOVE` 比較同一螢幕座標基準；任一軸位移嚴格大於 4 個實體像素即退出。未達門檻不更新基準，避免慢速滑動永遠不退出。
- 按鍵 `WM_KEYDOWN`／`WM_SYSKEYDOWN`、任何滑鼠按鈕按下、垂直／水平滾輪立即退出，寬限期不忽略新按鍵。
- 倒數輸入框結束後，消耗其結束事件，再啟動全螢幕輸入判定，避免「開始」那次 Enter／click 被誤當退出。
- 同程序多螢幕視窗彼此切換焦點不退出；寬限後前景轉到外部程序才退出。不要把每個 `WA_INACTIVE` 一律判成全程結束。
- 全螢幕 surfaces 建立後、顯示前，先以 `SetCursorPos` 把游標停到主要螢幕左上外角；所有旅行 player 都必須留下該角落的非 player 範圍。完成程式性移動後才記錄新的 `GetCursorPos` baseline，避免誤判成使用者退出。接著以 `SetCursor(NULL)` 隱藏游標並保存第一次取得的 borrowed cursor handle；後續 `WM_SETCURSOR` 必須再次套用空游標。正常退出、建立失敗、顯示拓撲改變與 session 結束都走同一清理路徑並還原保存的游標形狀；不得刪除該 borrowed handle。若改用 `ShowCursor`，必須對稱恢復，不用無界迴圈操縱計數。
- 不攔截系統安全快捷鍵、不反覆搶回前景。`WM_CLOSE` 與系統 session 結束採正常清理。

### 6.4 顯示配置與電源事件

- `WM_DISPLAYCHANGE`：第一版採安全退出全部全螢幕，下一次啟動重新列舉；不要求執行中無縫重建拓撲。
- `WM_DPICHANGED`：更新該螢幕 DPI 與幾何；頂層全螢幕重新查 `rcMonitor` 保持覆蓋，不機械套用一般可移動視窗的尺寸。
- `WM_TIMECHANGE`／時區變更：標準桌曆暨時鐘模式的快照立即重新取本機時間；離機作業番茄鐘的倒數與日本旅行模式的 monotonic 切換 deadline 不受影響。
- 不阻止系統睡眠、不呼叫維持螢幕常亮的 API。若程序睡眠後仍存在，恢復時以原 deadline 重算；睡眠時間計入經過時間。
- 若恢復時已逾期，顯示零；不把錯過的 timer 次數逐次補跑，不重播已過去的完整閃爍序列。
- 收到 `WM_QUERYENDSESSION` 不阻止登出；`WM_ENDSESSION` 清理退出。

## 7. 系統預覽 `/p <HWND>`

### 7.1 視窗契約

- 驗證 parent 後直接以 `WS_CHILD | WS_VISIBLE` 建立單一 child，不先建立 popup 再 `SetParent`。
- client 起點 `(0,0)`，大小取 parent 的 client rect；不設 topmost、不搶焦點、不隱藏全域游標、不因滑鼠移入而退出。
- 不注入、不 subclass 外部程序 parent。每秒檢查 `IsWindow`、parent 身分／尺寸與設定；parent 消失即清理退出。
- parent 尺寸不會保證自動傳成 child 的 `WM_SIZE`，因此尺寸變更時主動 `SetWindowPos`；child 再處理自身 `WM_SIZE`。
- 零尺寸／最小化時暫停實際繪圖，不配置 0×0 buffer；仍保留低頻率存活檢查。

### 7.2 DPI 與設定更新

- manifest 使用 Per-Monitor v2。建立 preview child 前，讀取 parent 的 DPI awareness context，必要時暫時以 `SetThreadDpiAwarenessContext` 匹配；建立後恢復原 thread context。
- 尺寸、DPI 與座標必須在一致 awareness context 下查詢／使用，不將虛擬化座標當實體像素再次放大。
- 跨程序 parenting 與 DPI 模式不同可能造成系統重設或錯誤；此流程是需實測的實作策略，不是免測保證。[SetParent DPI 注意事項](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setparent)、[SetThreadDpiAwarenessContext](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setthreaddpiawarenesscontext)
- `/p` 每 1,000 ms 讀取一份完整、驗證後的設定，只在內容變更時更新 cache。正常調度下保存成功後 2 秒內反映；Windows 若重建 preview process，新的程序立即使用最新設定。
- 這是 `/s` 不動態讀設定的明確例外，不建立 registry watcher thread。

### 7.3 預覽內容

| 模式 | 畫面來源 | 是否倒數 | 是否位移 |
| --- | --- | --- | --- |
| 標準桌曆暨時鐘模式（`TimeDate`） | 每秒更新的本機時間與當月月曆 | 不適用 | 否 |
| 離機作業番茄鐘模式（`Countdown`） | 上次保存時間，無效則 300 秒；`remaining=total`、ratio=1 | 否 | 否 |
| 日本旅行模式（`JapanTravel`） | 由 GDI 繪製內附擬真場景圖、示例地名與靜態預覽狀態 | 否 | 否 |

三種 preview 都不得發出網路請求、建立 WebView2、跳出倒數輸入框、發出聲音或觸發零點提示。小尺寸退化依第 8.1 節，不能改成獨立視窗。

## 8. 視覺、幾何與倒數行為

### 8.1 共用 layout 與繪圖輸入

- GDI renderer 接收 `RenderContext`：HDC、client rect、有效 DPI、字型／色彩、`FrameSnapshot`、preview 類型與位移。旅行 GDI layout 負責 preview、等待與錯誤 fallback，caption 取自該螢幕的狀態；每個正式螢幕的本機 HTML／CSS shell 另以結構測試證明 caption 位於完整 player rect 外。
- 幾何先用浮點或定點比例算完，再統一轉成裝置像素；所有乘法／面積／轉型檢查溢位與有限值。
- client rect 已是繪圖座標，不整張畫面再乘 `DPI/96`；DPI 主要用於筆寬、字型點數及對話框度量。
- 桌曆時鐘與番茄鐘在 client 寬至少 640 px 且高至少 360 px 時，使用置中 64% 寬、60% 高的內容區；其餘尺寸及日本旅行使用 88% 寬、82% 高。內容之外純黑，安全邊界每軸至少 2%。
- layout 回傳完整群組 bounding rect，包含描邊、圓形、沙漏及警示光暈，供位移與裁切測試使用。
- `WM_TIMER` 更新狀態及 invalidate；`WM_PAINT` 只畫快照。設定 owner-draw 的繪圖路徑見第 11.3 節。

小型預覽使用分級，不以可讀性無限縮小所有細節：

| client 大小（有效繪圖像素） | 要求 |
| --- | --- |
| 至少 320×180 | 正常細節；仍依 aspect ratio 決定排列 |
| 至少 120×80，未達上一級 | 可省略分鐘次刻度、年份及鐘面部分數字；保留鐘針、月曆格局、今天標記；倒數保留完整 6 位數與沙漏輪廓；旅行 preview 保留窗框與簡短「日本旅行」字樣 |
| 小於 120×80，但寬高皆正 | 最佳努力顯示兩區輪廓／倒數字樣／旅行窗框；不保證文字可讀，必須無 panic、負尺寸或越界，且不得建立小於播放器規範下限的 WebView2 |
| 任一邊為 0 | 不配置 buffer、不實際繪圖，待恢復 |

### 8.2 標準桌曆暨時鐘模式

#### 8.2.1 參考圖與可接受差異

- 使用者提供的原圖為 800×369，只供本機開發視覺比對；因未取得公開再授權依據，不納入 Git repository、`.scr` 或安裝程式。
- 必須保留黑底、左鐘右月曆、同色刻度與文字、今天實心圓反白數字。
- 原圖頂部的小型圓形裝置狀態圖示不屬於產品，不能繪製。
- 原圖紅色較亮、月標題沒有年份；產品色票及年月標題以第 9.1 節與下列規則為準。實機會顯示當下日期，不固定成附件日期。
- 鐘面刻度外沿應呈接近附件的圓角方形輪廓，沒有實線外框；不能只看見「指針鐘」就改成有外圈的傳統圓鐘。
- 平滑邊緣依 GDI 能力處理，不以逐像素或與瀏覽器相同反鋸齒作驗收。

#### 8.2.2 版面

- 畫面至少 640×360 時，桌曆時鐘只使用水平置中的 64%W×60%H 安全區（左右各 18%、上下各 20% 初始留白），整組視覺中心與畫面中心一致。防烙印位移仍可在安全邊界內進行。
- 小於 640×360 的 Windows preview 使用 88%W×82%H，避免預覽中的鐘面、月曆與文字過小。日本旅行模式維持 88%W×82%H 以保留影片可視面積。
- `W/H >= 1.35`：橫向排列，內容寬度分配約為鐘 48%、間距 8%、月曆 44%。
- 鐘區保持正方形，使用 `min(分配寬, 可用高)`；月曆與鐘的可見群組垂直置中，不能拉伸鐘面。
- `W/H < 1.35`：改為上鐘下月曆；在上述安全區內初始分配鐘高 48%、間距 6%、月曆高 46%，再等比適配。
- 月曆永遠保留六個日期列的空間，未用列留白，避免月底／月初造成位置跳動。
- 窄版以完整月曆及六位倒數不裁切優先；不使用固定最小字級把版面撐出畫面。

#### 8.2.3 指針鐘

- 以鐘區中心 `(cx,cy)`、半邊長 `R=0.45×鐘區邊長` 建立局部座標。
- 60 刻度每隔 6°，12 點方向為 0°、順時針為正。外沿採圓角方形比例；建議使用 superellipse 指數 6：

```text
u = sin(theta), v = -cos(theta)
r = R / (abs(u)^6 + abs(v)^6)^(1/6)
外端 = (cx + r*u, cy + r*v)
內端 = (cx + (r-L)*u, cy + (r-L)*v)
主刻度 L = 0.18R；次刻度 L = 0.075R
```

- 每 5 分鐘主刻度較長較粗；其餘短刻度亮度可為主色的 45%，主刻度及指針為 100%。
- 只顯示 `12`、`3`、`6`、`9`，目標字高為 `0.30R`（較原 `0.40R` 縮小 25%），數字中心約離鐘心 `0.58R`（原 `0.62R`），以量測後矩形置中。數字與主刻度保持可見間距；仍依字型、可用矩形及小型預覽規則適配，不能以固定字級造成黏連、重疊或裁切。
- 時針長 `0.55R`、分針 `0.82R`、秒針 `0.94R`；筆寬約 `0.055R`、`0.035R`、`0.010R`，正尺寸下至少 1 px。
- 時／分針可有短尾，秒針細長；最後繪製中心軸，主色外環、黑色內心。
- 角度以同一個快照計算：

```text
時針角度 = 30 × ((hour mod 12) + minute/60 + second/3600)
分針角度 = 6 × (minute + second/60)
秒針角度 = 6 × second
座標 = (cx + length×sin(angle), cy - length×cos(angle))
```

角度轉成 radians 才交給三角函式；秒針每秒跳動即可，不加 60 FPS 動畫。

#### 8.2.4 月曆

- 標題優先 `yyyy年 M月`，若量測後超出可用寬則改 `M月`，不是依月份字數硬編碼。
- 星期由左至右固定「一、二、三、四、五、六、日」，不受作業系統每週起始日設定影響。
- 使用 Gregorian calendar；只顯示當月數字，前後月位置留白。
- 標題區、星期列、6×7 日期格各自計算 cell rect；所有數字中心對齊。
- 今天用主色圓形與黑字。一般尺寸圓直徑至少字高 1.55 倍，且最多 `0.9×min(cell_width,cell_height)`；兩條件互斥時先縮小字型。極小預覽按第 8.1 節退化。
- 日期／月份／年份均由同一份本機時間快照取值，跨午夜不能出現「新日期舊月份」。
- 閏年：能被 400 整除，或能被 4 整除但不能被 100 整除。
- 若星期函式用 Sunday=0，第一天 Monday-based offset 為 `(weekday+6)%7`；day 的格子索引為 `offset+day-1`。

### 8.3 離機作業番茄鐘模式

#### 8.3.1 參考範圍與構圖

原稿指定 [Classroom Timer 倒數畫面](https://kisaraki.github.io/classroom-timer-tzk/?tool=countdown) 與[主頁](https://kisaraki.github.io/classroom-timer-tzk/) 作視覺參考。本次修訂未能透過網頁讀取工具取得頁面內容，因此以下數值保留／細化原稿設計，不宣稱與網站當前版本已比對一致。開發時若能開啟網站，記錄比對日期；網站變動不能自動改寫本規格。

- 純黑背景；中央極淡琥珀漸層為可選效果，失敗即退回純黑。
- 上方沙漏，下方 LCD；全群組符合第 8.1 節安全矩形。畫面至少 640×360 時與桌曆時鐘共用置中 64%W×60%H 安全區；小型 preview 維持 88%W×82%H。
- 沙漏寬約 client 短邊 10～14%、高 14～20%；若總高不足，與 LCD 一起縮小。
- 一般畫面的 LCD 寬約 client 的 64%；小型 preview 可使用 72～88%。內部留至少面板寬 4% 的左右 padding。
- 面板採暗色藥劑瓶玻璃質感。中央使用接近黑色的透明褐色，以上下連續漸層模擬透光衰減；左右使用相反方向的琥珀褐漸層模擬厚瓶壁折射。外圈只保留一層深棕瓶身、細銅色唇邊與低亮度內框，局部柔光不得橫跨整個面板。禁止用上、下兩條大面積實色帶或規則分格，避免呈現巧克力條外觀。中央顯示固定 `HH:MM:SS`，小時含前導零；所有主色仍須保持可讀。
- 不顯示工具列、按鈕、操作提示、百分秒或額外倒數文字。

#### 8.3.2 七段數字與系統字型

- 電子錶以 `Polygon` 等 GDI primitives 畫削角 segment，不使用 TTF、sprite 或 bitmap atlas。
- segment 定義：a 上、b 右上、c 右下、d 下、e 左下、f 左上、g 中；bit 0～6 對應 a～g。

| 數字 | mask（hex） | 數字 | mask（hex） |
| --- | --- | --- | --- |
| 0 | 3F | 5 | 6D |
| 1 | 06 | 6 | 7D |
| 2 | 5B | 7 | 07 |
| 3 | 4F | 8 | 7F |
| 4 | 66 | 9 | 6F |

- digit 寬約 `0.56h`、冒號寬 `0.18h`、相鄰 8 個 glyph 間有 7 個 `0.025h` 間距；`HH:MM:SS` 預估總寬 `3.895h`。
- 用可用內框求 h 的最大值，再驗證全部 glyph bounding boxes；不可只量數字而漏掉冒號／間距。
- 冒號是兩個實心圓。第一版預設省略未點亮 segment，避免灰白模式在淺面板上難辨識。
- Consolas、新細明體、自訂字型同樣繪製 `HH:MM:SS`，以文字量測適配；不能不論字型模式都強制畫七段。
- 主色仍由使用者選擇；亮綠／灰白在深棕玻璃面板上可保留既有深色描邊或陰影（`RGB(32,36,32)`），不能暗中改成另一種主色。GDI 文字可先畫小幅偏移的深色輪廓再畫主色。

#### 8.3.3 沙漏與比例線

- 沙漏包含上下端蓋、玻璃輪廓、上／下砂區及落砂細線，皆由 geometry 產生。
- 上砂高度按 `ratio` 變少，下砂按 `1-ratio` 變多；屬原稿指定的線性高度效果，不要求真實物理沙量模擬。
- 用玻璃形狀 clip 砂區，不能把矩形砂畫到外面；到零移除落砂細線。
- 金屬／玻璃輪廓使用中性灰，砂使用主色；七種固定色票下都要可辨識。
- LCD 內紅色垂直比例線由右往左，`x=inner_left+ratio×inner_width`；中心座標須縮進半個筆寬，確保線的外框也在內框內。
- 主線 `RGB(255,32,32)`，寬度約 3 個 96-DPI 邏輯像素，再依內框可用尺寸限制；可加較寬暗紅底線。
- 繪製順序：背景 → 沙漏 → LCD 底及框 → 比例線 → 數字 → 外框警示。比例線不能遮掉數字的主要筆畫。

#### 8.3.4 時間模型

全部計算以共用的毫秒數為準：

```text
total_ms = validated_total_seconds × 1000
deadline_ms = start_tick_ms.checked_add(total_ms)
remaining_ms = min(total_ms, deadline_ms.saturating_sub(now_tick_ms))
display_seconds = remaining_ms / 1000 + (remaining_ms % 1000 != 0 ? 1 : 0)
ratio = clamp(remaining_ms / total_ms, 0, 1)
HH = display_seconds / 3600
MM = (display_seconds % 3600) / 60
SS = display_seconds % 60
```

- `total_ms>0` 才可開始；deadline 溢位屬初始化錯誤，不能 wrap。
- 時基為 `GetTickCount64`；timer 只喚醒，不能當時間來源。[GetTickCount64](https://learn.microsoft.com/en-us/windows/win32/api/sysinfoapi/nf-sysinfoapi-gettickcount64)
- 不受手動校時、時區或夏令時間影響；睡眠恢復按第 6.4 節處理。
- 純邏輯函式接受注入的 `now_tick_ms`，測試不得真的等待 99 小時。
- 正常運行 100 ms 更新一次，數字到秒、比例連續採當次快照。不得提高系統 timer resolution 或忙等以追求毫秒畫面。

#### 8.3.5 最後十秒與歸零

- `0 < remaining_ms <= 10000` 才是最後十秒；LCD 外框加固定青藍色 `RGB(0,210,220)`，數字主色不改。
- 到零後 ratio=0、上砂空、下砂滿、數字保持 `00:00:00`，停止落砂。
- 定義 4 次閃爍為 8 個半週期，每個 420 ms、總長 3,360 ms：以 `elapsed=max(now-deadline,0)` 計算 `phase=floor(elapsed/420)`。
- `phase<8` 且為奇數時數字降至 35% 亮度；偶數保持正常。`elapsed>=3360` 永久正常，移除最後十秒警示。沙漏與底色不整片熄滅。
- phase 由經過時間算，不逐次累加。100 ms timer 允許最多約一個 tick 的顯示延遲；不為 420 ms 閃爍新增第二個高頻 timer。
- 完成提示結束後把 coordinator 更新降為每秒，仍執行位移／退出等必要工作。不重新開始、不自動退出、不播放聲音。

### 8.4 雙緩衝

每個正式視窗維持一個 memory DC 與一張 client 尺寸 bitmap，依下列順序：

1. `BeginPaint`；確認非零 client 尺寸。
2. 按需建立／重用 memory DC、bitmap；compatible bitmap 由畫面 DC 建立，不能由初始僅有單色 bitmap 的 memory DC 推導色深。
3. 保存 GDI 狀態；畫完整背景及快照內容。
4. `BitBlt` 到 paint DC。
5. 恢復選入物件／狀態；快取的 owned 資源保留到 resize 或 destroy。
6. 所有返回路徑均 `EndPaint`。

- `InvalidateRect(..., FALSE)`，`WM_ERASEBKGND` 回傳非零；paint 必須覆蓋整個 client，不留下前一幀殘影。
- `WM_SIZE`／DPI 改變安全重建；刪除 bitmap／font 前先從 DC 選回原物件。
- buffer 建立失敗可在有效 paint DC 直接填黑並嘗試簡化繪圖一次；不能讀取無效 handle 或無限重試。必要資源不可用時全螢幕清理退出，preview 安靜退出。
- 字型、brush、pen 依尺寸／樣式快取；不要每個 frame 建立大量短命資源。設明確上限，不以日期字串逐秒新增 cache。

### 8.5 防烙印位移

- 純 std 實作 xorshift32，非零 seed；seed 可由啟動 tick 與螢幕索引混合，若結果為零用固定非零值替代。
- 每滿 60,000 ms 計算一次新 anchor；時間跳過多個區間時只重算一次，不補跑所有歷史位置。
- X／Y 位移上限為 client 寬／高的 ±5%，再與內容 bounding rect 可容納區間取交集。
- 若某軸無位移空間，該軸用 0；不能產生負的亂數範圍。極小 client 以零位移退化。
- 鐘與月曆一起移動；沙漏、LCD、光暈一起移動；不縮放、不隨機改顏色。
- `/p`、設定預覽及視覺測試 fixture 固定 offset=(0,0)。不加每秒 2px 抖動。

### 8.6 日本旅行模式

#### 8.6.1 來源發現與可信邊界

- `https://tw.live/japan/` 是和風庭園場景的旅行來源目錄；其他四種場景使用規格指定的 YouTube playlist。所有清單、camera ID、video ID、授權及可用性均可能由第三方改變。
- v0.2.0 內建 8 個 tw.live camera seed：札幌、奧多摩、京都中京區、大阪 JR 放出車站、廣島宮島、沖繩名護、鹿兒島櫻島及長野上高地。camera ID 與最後一次探測結果記錄於 `docs/japan-travel-sources.md`；固定的是 camera ID，不是會隨直播重啟改變的 YouTube video ID。
- `/s` 啟動後，source worker 先以有界 HTTPS GET 驗證日本目錄標記，再把 8 個 seed 隨機排序，直接取得 `/cam/?id=...` detail。它不在執行期掃描地區頁或全部都道府縣；detail 只解析鏡頭標題及允許的 YouTube player URL，不執行 tw.live 的 script、廣告或追蹤碼。
- WinHTTP 只接受 `https://tw.live` 的日本目錄與 camera detail path，redirect 完全停用。connect／send／receive 各 4 秒，單次讀取總時間 15 秒，response body 上限 512 KiB；HTTP `2xx` 以外、內容型別不符、非法 UTF-8、非法 path 或結構無法解析都視為失敗。
- 遠端 HTML 一律是不可信輸入。解析器只接受 `www.youtube.com` 或 `www.youtube-nocookie.com` embed 中恰為 11 字元的 ASCII `[A-Za-z0-9_-]` video ID；本機 shell 接收的 video ID 與地點文字必須作 JSON escaping，不能把任意 `src`、query、script 或 HTML 當成程式碼。
- process 內只保留本次 `TravelSession` 的目錄健康旗標、目前來源、地點、video ID、generation 及時間狀態，不建立持久來源 cache，也不保存縮圖、影格、音訊或影片。抓取新候選失敗時可以讓目前 player 繼續顯示，再按重試狀態處理。
- 每輪最多嘗試隨機排序後的 3 個不同候選；後續輪換必須排除上一個成功播放的 camera。單輪全失敗時進入可退出的離線狀態並有界重試。

#### 8.6.2 旅行場景與地名

- 日本旅行模式提供 `FreeFlight=0`（自在飛行）、`TrainJourney=1`（列車旅行）、`JapaneseInn=2`（和風庭園）、`TrainCab=3`（御運轉士）與 `Walking=4`（地方散策）五種可保存場景。只改顯示名稱，enum 數值與 registry schema 不變。五者使用內附的原創 AI 擬真點陣圖，不宣稱是真實 A380、特定列車、旅館或業者照片，不包含附件像素、第三方照片、人物或商標。
- 原始 PNG 位於 `assets/travel/` 下對應五種場景的檔案。御運轉士的窗孔寬約 46.4%、高約 35.2%；16:9 player 以中央裁切的 cover 方式覆滿窗孔，僅由 `overflow:hidden` 裁掉少量上下影像，不得跨到窗框，也不得留下左右黑帶。地方散策 player 保留至少 0.4% 的場景外緣作為非 player 游標停放區，另用徑向攝影暗角把主要視域集中於中央約 65%；素材不得含眼球、皮膚或血管。切換來源先以約 420 ms 完全閉眼，在閉合點載入新來源，再以約 520 ms 明確睜眼；順暢播放時每 20～30 秒作一次較淺、約 380 ms 的輕眨，發生 BUFFERING 或播放時間進度異常時以不超過每 8 秒一次的較慢、較深眨眼柔化停頓。上下遮罩高度約 62%，左右超出場景約 8%，前緣使用橢圓曲線與 9～18 px 模糊黑暈；完全閉合時兩側不透明區必須重疊，動態邊界不得呈現鋒利直線。
- 每個螢幕正式 `/s` 依保存場景使用本機 HTML／CSS shell 及擬真場景圖；`/p`、`/c`、播放器等待及 Runtime／網路 fallback 由 GDI 繪製對應內附靜態圖片，不連網。場景只改變本機 frame，不改變來源清單、網路健康判定、靜音、輪換設定或 failover。
- 本機 shell 透過 WebView2 virtual host mapping 以 `https://travel.screensaver.local/index.html` 載入。shell 檔與 per-user WebView2 profile 位於 `%LOCALAPPDATA%\KOMSMOS\tools-screensaver-tzk\`，不從遠端網站取得產品 UI。
- player element 維持 16:9。自在飛行、列車旅行與和風庭園完整顯示；御運轉士只允許由窗孔容器對影片上下作等量、置中的少量裁切，以填滿較寬的實體窗孔，窗框本身不得覆蓋 player。場景外框、障子、陰影、地名及狀態區位於 player element 外；地方散策的短暫眨眼是唯一覆蓋 player 的本機動畫。
- 地名區使用 detail 解析後的標題；沒有可用標題時使用對應 camera seed 的城市／地區提示。文字必須清除控制字元並限制長度；來源尚未確認時顯示日本旅行模式與連線狀態，不能把固定縮圖冒充即時播放。
- 影片固定靜音，不播放來源音訊。播放器設定 `controls=0`、`cc_load_policy=0`、`iv_load_policy=3`、`disablekb=1`、`fs=0`，並讓 iframe 不接收滑鼠事件；在 player ready、每次 load、進入 `PLAYING` 與 `onApiChange` 時再次要求關閉字幕 track 並卸載 captions module。此處只能關閉 YouTube 可切換字幕，無法移除來源影片本身燒錄的文字。YouTube 已停用 `showinfo` 與 `modestbranding`，產品不得以無效參數宣稱能完全移除平台必要的短暫標題或品牌資訊，也不得用持續覆蓋層遮住它們。螢幕保護程式的一般鍵鼠退出規則仍優先。

#### 8.6.3 隨機輪換與多螢幕

- 第一個健康來源隨機選擇。`FreeFlight`、`TrainJourney`、`TrainCab`、`Walking` 分別使用 playlist `PLdsqwBj2O1Nw`、`PLBH60D9AGfu0`、`PLB-Fmt68BNm4`、`PLbYZr39owNGo`。每次全螢幕啟動時以官方 IFrame API 讀取並 shuffle 清單後隨機選片，不解析 YouTube HTML、不持久保存清單。`TravelSwitchMinutes` 預設 1，允許 0～1440；0 表示不定時切換。正整數設定在播放進入 `PLAYING` 後，以 `GetTickCount64` 建立 deadline。
- 只有啟用定時切換時，才在距離輪換剩餘最後 1 分鐘時，於背景預抓下一來源。和風庭園從排除目前 camera 的 7 個 seed 中隨機排序並解析下一個 video ID；影片清單場景從本次啟動已讀取的清單預選不同影片。WebView shell 可預熱該候選的 YouTube 縮圖/CDN 連線，但不得建立第二個隱藏 player、背景播放影音或持久保存縮圖。預設 1 分鐘間隔可在 `PLAYING` 後立即預抓，較長間隔則延後。到 deadline 時直接載入已準備的來源；若預抓仍在進行，保留目前畫面，並在第一個成功結果到達時切換。無有效預選時回退到 `loadPlaylist` 重新取得清單。
- 不切換時不建立輪換 deadline，不預抓下一來源；目前健康來源持續播放。初次來源尋找、來源失敗／停滯時的換候選、網路健康檢查與有界重試仍保持啟用；不得把「不切換」解釋為停止失敗復原或永久保證同一鏡頭。
- HTTP／解析 preflight 在目前影片繼續顯示時執行；候選尚未驗證完成不得先清空 player。每次取得來源、初始影片清單選片及同片重播都重新產生 `startSeconds=181～539`，從 3:01～8:59 的隨機位置開始。YouTube 可對直播、短片或 keyframe 調整此起點。只有收到新 player 的 `PLAYING` 且啟用切換時，才建立下一個設定分鐘的 deadline。
- 睡眠、暫停或訊息阻塞跨過多個 deadline 時只做一次切換，不補跑漏掉的區間。播放中斷則不等到下一個切換時間，立即進入有界 failover。
- 每個實體螢幕的 surface 都是獨立 WebView2 host，每個 screen／本機頁面最多一個 autoplay player。來源選擇、每輪最多 3 個候選的預算、首次 `PLAYING` 後的可選 deadline 與重試各自獨立，切換分鐘共用啟動設定快照；允許不同螢幕隨機選到同一地點。非同步通知帶入所屬 HWND，由 coordinator 路由回正確 host；拒絕過期 generation／playback token 與關閉後事件。某個來源失敗不得改動其他 host 的狀態或計時。共用退出流程必須關閉所有 host，記憶體與網路預算須考慮播放螢幕數。

#### 8.6.4 Runtime、健康檢查與 fallback

- 建立 player 前以 `GetAvailableCoreWebView2BrowserVersionString` 探測 Evergreen Runtime，再於 STA message loop 上非同步建立 environment／controller。等待期間保留可退出的 GDI 旅行 fallback；找不到 Runtime、非同步 completion 回報失敗、controller 失敗或版本不支援時仍保留 fallback。不得由 `.scr` 下載或啟動 Runtime installer、開啟瀏覽器、要求 UAC 或改成另一個已保存模式。
- 健康狀態分四層：`CatalogReachable` 表示 tw.live 目錄 HTTP 成功；`CandidateResolved` 表示 detail 可解析 YouTube video ID；`PlaylistEmbedReachable` 表示指定 embed endpoint 可達；`PlaybackHealthy` 表示 WebView2 player 回報 `PLAYING`。HTTP 200、oEmbed、縮圖或 Runtime probe 成功都不能單獨宣稱影片可播放。
- player source 載入 20 秒仍未進入 `PLAYING` 時視為失敗；單一候選失敗後選另一個，單輪最多嘗試 3 個。全部失敗顯示稍後重試狀態，每 30 秒再檢查；player error 或連續停滯則立即切換。
- WinHTTP worker completion 攜帶 source generation；若已切換或 shutdown，晚到 completion 只能釋放自身資料，不能覆蓋新狀態、重建視窗或重新開始播放。每次載入來源都產生非零 playback token；WebView2 player event 必須攜帶並符合目前 token，舊來源晚到的 `PLAYING`、error 或 stall 不得改變新來源狀態。
- 使用者輸入、`WM_DISPLAYCHANGE`、session end 或一般 shutdown 時，先停止 timer 與接受 completion，關閉 WebView2 controller 及 COM apartment，再按原視窗生命週期退出。已開始的 WinHTTP request 不作無界等待，最多依既定 timeout 結束；receiver 已關閉時結果直接回收。

#### 8.6.5 WebView 與第三方資料規則

- WebView2 top-level navigation 只由程式指向本專案固定的 virtual-host player shell，且 `NavigationStarting`、`NavigationCompleted`、WebMessage source 與 process failure 都必須驗證或處理；shell 導航成功前 controller 保持隱藏。shell 只載入 YouTube IFrame API 與驗證後的官方 embed。停用開發者工具、context menu、status bar、script dialog、zoom、browser accelerator、password／autofill；permission request、`NewWindowRequested` 與 download 一律拒絕，不交給系統瀏覽器。
- virtual HTTPS host 提供實際 origin／Referer，不偽裝成 tw.live，也不阻止 YouTube 為播放完整性所需的標準訊號。不得用 nested iframe、CSS、裁切或參數規避播放器政策；播放器必須可見，每個 screen／本機頁面同時最多一個 autoplay player。
- 日本旅行模式會把目標機的 IP 位址、User-Agent、連線時間及播放器所需資料傳給 tw.live、YouTube／Google 與來源使用的 CDN；README 與 Pages 必須在使用者選擇前清楚揭露。`youtube-nocookie.com` 不能被描述成「完全不傳資料」。
- tw.live 明示其為民間整合平台，不擁有所有影像授權，嵌入／轉載／商用須確認原始來源規範。公開來源清單須記錄 detail URL、原始提供者、最後檢查時間與可用性；MIT License 只涵蓋本專案程式碼及自製圖形，不授權第三方影像。
- 本程式只使用來源允許的官方嵌入播放，不下載、錄製、截取、轉碼、代理、重新託管或保存歷史影片。來源撤回嵌入、改址、區域限制或權利狀態不明時，移除該候選並以其他來源或 fallback 處理。

## 9. 顏色與字型

### 9.1 色票

| 識別值 | 名稱 | RGB | hex |
| --- | --- | --- | --- |
| 0 | 深紅 | 139, 0, 0 | `#8B0000` |
| 1 | 深橘 | 255, 140, 0 | `#FF8C00` |
| 2 | 亮綠（預設） | 0, 255, 0 | `#00FF00` |
| 3 | 灰白 | 245, 245, 245 | `#F5F5F5` |
| 4 | 雪藍 | 101, 151, 178 | `#6597B2` |
| 5 | 琥珀 | 255, 191, 0 | `#FFBF00` |
| 6 | 自動切換 | 每 120 秒循環七種固定色 | — |
| 7 | 鐵灰色 | 154, 160, 163 | `#9AA0A3` |

`COLORREF` 依 `RGB(r,g,b)`／對應位元順序建立，不能把 HTML `0xRRGGBB` 直接當 COLORREF。深紅是原稿既定低亮度色，不改成附件的純紅。自動模式使用 `GetTickCount64` 的共同快照時基，以 120,000 ms 為一期，依序循環深紅、深橘、亮綠、灰白、雪藍、琥珀及鐵灰色；不得依每個螢幕各自取時。七種固定色都須在黑底及深棕玻璃面板上驗收。

### 9.2 字型模式

| 識別值 | UI 名稱 | 實作 |
| --- | --- | --- |
| 0 | 電子錶（預設） | 七段向量主數字；月曆中文／日期用清晰 GDI 字型 |
| 1 | Consolas（打字機） | `Consolas` |
| 2 | 新細明體 | `PMingLiU` |
| 3 | 自訂字型 | `ChooseFontW` 選定的系統字型 |

- 非電子錶模式套用於鐘面數字、月曆及倒數主數字；同一字型缺中文字形時，中文部分獨立 fallback。
- 拉丁字形 fallback：指定字型 → Consolas → Microsoft JhengHei → DEFAULT_GUI_FONT。
- 中文字形 fallback：指定字型（確有 glyph）→ Microsoft JhengHei → PMingLiU → 系統 fallback。
- 字型 API 建立成功不代表所需 glyph 存在；以枚舉／glyph 檢查及實際畫面驗證，不只檢查 HFONT 非空。
- 缺字型時不改寫保存的自訂選擇；只改本次有效繪圖字型。`DEFAULT_GUI_FONT` 是 borrowed stock object，不可刪除。

### 9.3 選字型

- 使用 `ChooseFontW`；`lStructSize`、owner、`LOGFONTW`、`iPointSize` 全部初始化。
- 旗標包括 `CF_INITTOLOGFONTSTRUCT | CF_FORCEFONTEXIST | CF_SCREENFONTS | CF_LIMITSIZE`；`nSizeMin=18`、`nSizeMax=240`。
- 不使用 `CF_EFFECTS`，顏色由本程式管理；取消保持草稿不變。
- 成功保存 `LOGFONTW` 與 1/10 pt 的 `iPointSize`，自動切至 Custom。`ChooseFontW` 回傳 false 時，以 `CommDlgExtendedError` 區分取消（0）與錯誤。[CHOOSEFONTW](https://learn.microsoft.com/en-us/windows/win32/api/commdlg/ns-commdlg-choosefontw)

### 9.4 點數、適配與驗證

- 保存點數範圍 `180–2400`（18–240 pt），預設 `480`。
- 保存點數是設計基準的偏好，不是任何螢幕一律固定像素字高。先依 800×369 橫向／369×800 直向設計畫布求 layout scale，產生各角色候選字高，再縮小到實際可用矩形。
- 每個角色（倒數、鐘面、月份、星期、日期）有獨立可用矩形；以 `GetTextExtentPoint32W`／對應量測結果求一致縮小比例，不能單獨把某一個日期縮得不同。
- 全畫面採單一計算方式：`base_height_96=point_size_tenth×96/720`；`layout_scale=min(實際可用寬/設計寬,實際可用高/設計高)`；候選像素字高為 `base_height_96×layout_scale×role_scale`，再量測適配。角色初始比例可用倒數1.0、鐘面0.65、標題0.42、星期／日期0.32，依各角色可用矩形驗證。
- 上式的可用寬高已是有效像素，因此不得再乘 `dpi/96`。最終 HFONT 的 `lfHeight` 是負的適配像素字高；DPI 改變後重新取得有效幾何並重算。
- `lfHeight=-MulDiv(point_size_tenth,dpi,720)` 用於沒有畫布縮放的標準字型點數換算，例如選字型初始資料；不能把它再乘以上述實體像素 layout scale。
- 保留 face、weight、italic；第一版正規化為水平字：`lfWidth=0`、`lfEscapement=0`、`lfOrientation=0`，不支援旋轉、直書、刪除線或底線。
- 驗證完整 binary 大小、字串至少一字且 32 code units 內有 NUL、UTF-16 可解碼、weight `0–1000`、布林欄位 `0/1`、點數範圍。face name NUL 後清零。
- 不直接沿用 registry 的 `lfHeight`／width 建字型；先驗證數值可安全處理，再以已驗證點數重建，拒絕 `i32::MIN` 等不合理值。
- charset、precision、quality、pitch/family 只接受 SDK 定義的合法值；未知值以安全預設正規化，不自行把合法 CJK charset 限縮成 ASCII。
- 畫面裁切規則優先於使用者大字級；對話框文字說明：「字型大小會依顯示空間自動縮放」。

## 10. 設定資料與登錄檔

### 10.1 路徑及 schema

```text
HKEY_CURRENT_USER\Software\tools-screensaver-tzk
```

`.scr` 本體以 `asInvoker` 執行，讀寫偏好不需要管理員權限。

| 名稱 | 型別 | 有效範圍／內容 | 預設 |
| --- | --- | --- | --- |
| SchemaVersion | REG_DWORD | 目前為 8 | 8 |
| DisplayMode | REG_DWORD | 0=TimeDate、1=Countdown、2=JapanTravel | 0 |
| TravelStyle | REG_DWORD | 0=FreeFlight、1=TrainJourney、2=JapaneseInn、3=TrainCab、4=Walking | 0 |
| TravelSwitchMinutes | REG_DWORD | 0=不切換；1～1440=切換間隔整數分鐘 | 1 |
| ColorPreset | REG_DWORD | 0～7；6=每 2 分鐘自動切換、7=IronGray | 2 |
| FontMode | REG_DWORD | 0～3 | 0 |
| CustomLogFont | REG_BINARY | 完整、已驗證 LOGFONTW | 無 |
| CustomPointSizeTenth | REG_DWORD | 180～2400 | 480 |
| LastCountdownDurationSeconds | REG_DWORD | 1～359999 | 300 |

- key 不存在：全預設，讀取不建立 key。
- SchemaVersion 缺失／1／2：按該舊版的已知欄位個別驗證；`DisplayMode` 只接受舊值 0／1，值 2 在舊 schema 視為損壞而 fallback。
- SchemaVersion=3：`DisplayMode=2` 對應 `JapanTravel`；沒有 `TravelStyle`，一律使用 `FreeFlight`。
- SchemaVersion=4：正常驗證既有欄位；`TravelStyle` 非 0／1 或型別錯誤時只回退為 `FreeFlight`。schema 1～4 沒有切換時間設定，`TravelSwitchMinutes` 一律使用預設 1，不因舊 key 恰有同名資料而改為不切換。
- SchemaVersion=5：`TravelSwitchMinutes` 為 0 時表示不切換，1～1440 表示整數分鐘；缺值、型別或長度錯誤、超界只回退本欄為 1。使用者明確提交成功才寫 version 5；讀取與預覽不主動遷移。
- SchemaVersion=6：新增 `TravelStyle=2` 與 `ColorPreset=4/5/6`；舊 schema 中偶然存在這些值時必須回退，不能提前套用新版語意。明確提交成功才寫 version 6。
- SchemaVersion=7：新增 `TravelStyle=3/4`；schema 6 中偶然存在這些值時必須回退。明確提交成功才寫 version 7。
- SchemaVersion=8：新增 `ColorPreset=7`（鐵灰色）；schema 7 中偶然存在此值時必須回退。明確提交成功才寫 version 8。
- schema 型別損壞／0：視為損壞資料，使用預設，允許下一次明確提交修復已知值。
- SchemaVersion>8：未知較新版；可按本版已知欄位驗證供顯示，但禁止本版寫入。設定／倒數輸入提交時明確說明版本不相容，不自動降版、刪除 key 或啟動倒數。

### 10.2 讀取

- 使用最小權限 `KEY_QUERY_VALUE`，寫入才要求所需 write access；所有 owned HKEY 以 `RegCloseKey` 關閉。
- `REG_DWORD` 必須恰為 4 bytes；binary 必須為 `size_of::<LOGFONTW>()`（本版 Windows ABI 預期 92 bytes，以編譯期／測試 assert 驗證）。
- 先查型別與長度；第二次讀取可能遇值改變或 `ERROR_MORE_DATA`，只做有上限重試，仍失敗便 fallback。
- 將 binary 複製到已初始化、對齊正確的結構後再逐欄驗證；不把任意 `Vec<u8>` 指標直接當已對齊 LOGFONTW。
- 缺值、型別錯誤、非法 enum、超界倒數按欄位回退；不因一欄錯誤丟失所有有效顏色／模式。
- Custom 資料缺失時有效繪圖模式回退電子錶，草稿可重新選字型；讀取不寫回任何修正值。

### 10.3 寫入、取消與部分失敗

- `/c` 只在 `IDOK` 全部驗證成功後提交模式／旅行場景／切換分鐘／顏色／字型欄位，不能用載入時的舊副本覆蓋另一程序更新的 LastCountdownDurationSeconds。
- 倒數輸入只提交 LastCountdownDurationSeconds 及必要 schema 初始化，不能重寫顏色或模式。舊 schema 升為 5 時，須同時初始化 `TravelSwitchMinutes=1`，避免把舊 schema 未識別的同名值誤當成已保存偏好；已是 schema 5 時不覆寫該欄。
- 點「取消」、Esc、關閉與 `ChooseFont` 取消都不發起寫入；「取消不變」驗收以未按過失敗的提交、沒有其他程序同時寫入為前提。
- 寫入前保存本次會修改的值之原始型別／bytes／是否存在；依序寫入已驗證值，schema 最後寫。
- 多個 `RegSetValueExW` 不具有交易原子性。中途失敗須停止並嘗試還原本次已改欄位，包含刪除本次新增值；不刪除未知值。
- 若 rollback 也失敗，明確提示「部分設定可能已變更」，重讀實際值且保留草稿供重試；不能宣稱取消將還原整個對話框開啟前狀態。
- 不對普通設定保存使用 `RegFlushKey`，不把 schema-last 宣稱為跨程序原子提交。
- `/p` 讀取期間可能見到短暫中間狀態，仍須形成合法的完整 `AppConfig`；下次 poll 收斂至實際已保存值。第一版不提供跨程序交易隔離。

## 11. 設定對話框

### 11.1 RC 與控制項

使用 `.rc` 的 `DIALOGEX`，`DS_SETFONT`／`DS_SHELLFONT`、繁體中文、系統 dialog 字型；以 `DialogBoxParamW` 顯示。

```c
#define IDD_CONFIG              2003
#define IDD_COUNTDOWN_INPUT     2004
#define IDC_MODE_TIME_DATE      1001
#define IDC_MODE_COUNTDOWN      1002
#define IDC_MODE_JAPAN_TRAVEL   1003
#define IDC_TRAVEL_FREE_FLIGHT  1004
#define IDC_TRAVEL_TRAIN_JOURNEY 1005
#define IDC_TRAVEL_JAPANESE_INN 1006
#define IDC_TRAVEL_SWITCH_MODE  1010
#define IDC_TRAVEL_SWITCH_MINUTES 1011
#define IDC_TRAVEL_SWITCH_LABEL 1012
#define IDC_TRAVEL_SWITCH_UNIT  1013
#define IDC_COLOR_DARK_RED      1101
#define IDC_COLOR_DARK_ORANGE   1102
#define IDC_COLOR_BRIGHT_GREEN  1103
#define IDC_COLOR_OFF_WHITE     1104
#define IDC_COLOR_MUTED_LIGHT_BLUE 1105
#define IDC_COLOR_AMBER         1106
#define IDC_COLOR_AUTO          1107
#define IDC_COLOR_IRON_GRAY     1108
#define IDC_FONT_COMBO          1201
#define IDC_CHOOSE_FONT         1202
#define IDC_PREVIEW             1301
#define IDC_COUNTDOWN_HOURS     1401
#define IDC_COUNTDOWN_MINUTES   1402
#define IDC_COUNTDOWN_SECONDS   1403
```

- 模式群組：「標準桌曆暨時鐘模式」／「離機作業番茄鐘模式」／「日本旅行模式」三個 radio 選項。
- 日本旅行場景群組：「自在飛行」／「列車旅行」／「和風庭園」／「御運轉士」／「地方散策」五個 radio；非日本旅行模式時停用但保留草稿值。
- 日本旅行的「來源切換」使用不可自由輸入的 combo，選擇「不切換」或「每隔」；分鐘 Edit 預設 1，只接受 1～1440 的 ASCII 整數，設定 `ES_NUMBER` 與 4 字元長度上限後仍以程式驗證。空值、負數、0、超界、全形數字、小數、符號或非法黏貼一律拒絕提交，清楚標示問題欄位。
- 「不切換」對應保存值 0 並停用分鐘 Edit；切回「每隔」時保留合法草稿分鐘或恢復預設 1。非日本旅行模式時停用來源切換控制項，但保留草稿偏好；不能因停用欄位中的未使用文字阻擋其他模式保存。按「確定」才保存，取消不提交；全螢幕使用下一次啟動的設定快照。
- 日本旅行 radio 附近以非互動文字說明「需要網路；全螢幕連線至 YouTube，和風庭園另使用 tw.live；影片靜音」。選取 radio 不得立即連線、建立 WebView2、下載 Runtime 或顯示 UAC。
- 顏色群組：深紅、深橘、亮綠、灰白、雪藍、琥珀、鐵灰色與「自動切換（2 分鐘）」八個 radio；各組正確設 `WS_GROUP`，不能兩組互相取消。
- 字型 combo 使用固定四選項及不可自由輸入樣式；另有「選擇系統字型…」。
- 設定畫面固定顯示「KOMSMOS TOOLKIT 探真拓知酷」產品識別；該文字不是可互動控制項。
- 自訂大小說明、`SS_OWNERDRAW` 預覽、標準「確定」「取消」。
- Tab 順序循序可用，radio 支援方向鍵，Enter 提交、Esc 取消；標籤有明確欄位關係，不只靠顏色表意。
- 100%、150%、200% DPI 不重疊／截字；跨螢幕依 dialog DPI 機制更新，避免系統與程式各縮放一次。

### 11.2 owner 與生命週期

- 有效 owner 時設為 owned modal 並置中於 owner 所在 monitor 的 work area；沒有 owner 則置中於前景所在 monitor。
- owner 可能跨程序且隨時消失；使用期間驗證，失效時以取消結束。不得把 HWND 有效性當成永久保證，也不對外部程序做 subclass。
- `WM_INITDIALOG` 建立草稿、控制項及預覽 timer；`WM_COMMAND` 只更新草稿或處理明確提交。
- `IDOK` 保存成功才 `EndDialog`；`IDCANCEL`／`WM_CLOSE` 不提交。
- dialog procedure 未處理訊息回傳 FALSE；不能把一般 WNDPROC 的 `DefWindowProcW` 規則直接套到 DLGPROC。
- `WM_DESTROY` 停 timer 並釋放 cache；所有 pointer 狀態依 dialog 專用生命週期回收一次。

### 11.3 即時預覽

- `IDC_PREVIEW` 用 `SS_OWNERDRAW`，在 `WM_DRAWITEM` 取得 `DRAWITEMSTRUCT.hDC`／`rcItem` 後呼叫共用 renderer；此 HDC 為 borrowed，不使用 `BeginPaint`／`EndPaint`／`ReleaseDC`。
- 保存／恢復 HDC 狀態，不改變其他控制項的 clip、字型或座標原點。
- 模式／顏色變更只更新草稿、必要 cache 並 invalidate；只有字型／尺寸／DPI 變更才重建 font cache。
- 標準桌曆暨時鐘模式（`TimeDate`）每秒以目前本機時間更新；不能只在選項改變時更新時鐘。
- 離機作業番茄鐘模式（`Countdown`）靜態示範：remaining=300 秒、total=600 秒、顯示 `00:05:00`、ratio=0.5、上下各半砂量，不播放落砂動畫或警示。
- 日本旅行模式（`JapanTravel`）依草稿以 GDI 繪製五種內附擬真圖、示例來源與靜態預覽文字；PNG 由 Windows Imaging Component（WIC）在本機解碼，不下載圖片、不建立 WebView2、不發 request。
- `/c` 預覽永遠採草稿，不由 `/p` 的 registry poll 蓋掉尚未保存的修改。
- 所有預覽不位移；字型失敗時 fallback，不使對話框失去操作能力。

### 11.4 系統字型操作

- 字型選擇成功才更新草稿；取消與 error 區分處理，錯誤可顯示繁體中文訊息。
- 單純改變 ColorPreset 不丟失自訂 font；由 Custom 切回預設 font 不刪除上次自訂資料。
- 選到缺字／字形與目標尺寸不合時，按照第 9 節逐角色適配，不能讓文字超出 preview 控制項。

### 11.5 倒數輸入對話框

- 標題「設定倒數時間」；說明「請輸入本次倒數時間」。
- 依序「小時」「分鐘」「秒」，3 個 Edit 設 `ES_NUMBER` 與 `EM_SETLIMITTEXT=2`，仍完整驗證讀回文字。
- 初始小時欄選取全部內容；按開始失敗時聚焦並選取第一個錯誤欄位，以短文字說明有效範圍。
- `IDOK` 顯示為「開始」、`IDCANCEL` 為「取消」；接受／保存／取消流程依第 6.2 節。
- `/p`、`/c`、安裝內部命令都不顯示此輸入框。

## 12. GDI／Win32 資源管理

| 資源來源 | 正確釋放／恢復 |
| --- | --- |
| BeginPaint | EndPaint |
| GetDC | ReleaseDC |
| CreateCompatibleDC | DeleteDC |
| CreateFontIndirectW、CreatePen、CreateBrush、CreateCompatibleBitmap、CreateDIBSection、建立的 HRGN | 選回舊物件後 DeleteObject |
| GetStockObject／DEFAULT_GUI_FONT | borrowed，不刪除 |
| SelectObject | 保存舊 handle，使用後選回；失敗按該 API 回傳規則判斷 |
| SaveDC | 配對 RestoreDC，包含錯誤路徑 |
| SetTimer | KillTimer，使用回傳的有效 timer ID |
| RegOpenKeyExW／RegCreateKeyExW | RegCloseKey |
| CommandLineToArgvW | LocalFree |
| OpenProcessToken 等 owned kernel handle | CloseHandle；不套用 HWND／HMONITOR |
| HWND | DestroyWindow／EndDialog，依視窗種類與 UI 執行緒規則 |
| HMONITOR | borrowed，不釋放 |

- 最小 RAII wrapper 區分 owned／borrowed 與清理函式，不用一個不透明通用 handle wrapper 包全部類型。
- bitmap 選入 DC 的順序與 Drop 順序要有明確設計，不能依欄位宣告順序碰運氣。
- 重複 resize、DPI 改變、字型切換後 GDI 數量應回到穩定範圍；不只觀察正常關閉路徑。
- 正式 callback 不做同步網路、磁碟檔案操作、sleep 或長工作；短小且有界的 registry 讀寫可在已規定的 poll／提交事件執行。WinHTTP worker 與 WebView2 非同步 callback 只搬移有界結果、驗證 generation 並排程 UI 更新，不能在回呼內等待另一個 callback。

## 13. 資源與 manifest

- `resources.rc` 使用 UTF-8 與明確 code page（如 `#pragma code_page(65001)`），驗證繁體中文不亂碼。
- 包含 app icon、兩個 dialog、resource ID 1 名稱、VERSIONINFO、application manifest。
- 圖示使用有權使用的素材或自行產生的簡單原生圖形；不從參考圖抽取作業系統圖示。
- manifest 宣告 `asInvoker`、Per-Monitor v2 及必要 Common Controls v6；需要 Common Controls 時正確初始化。
- Windows 10／11 共用相應相容性宣告，不虛構 Windows 11 專用 supportedOS GUID。
- application manifest 使用正確 `RT_MANIFEST`／ID，避免 linker 另嵌預設 manifest 蓋掉資源版；以成品擷取驗證只有預期設定。
- 版本資訊包含 ProductName、FileDescription、FileVersion、ProductVersion、CompanyName、LegalCopyright；最後兩項遵守第 0.3 節。
- Cargo SemVer `a.b.c` 對應 RC 數值版 `a,b,c,0`；若含 prerelease，字串可保留標籤，數值版仍是合法四整數。

## 14. 可重現建置與封裝

### 14.1 `build.rs`

- 僅用 Rust std；驗證 target 是指定 MSVC x64，不在不支援的 host 上假裝已完成 Windows 資源編譯。
- 使用 `std::process::Command` 與分離參數呼叫 `rc.exe`，不拼接經 shell 解釋的命令。
- `.rc`、header、manifest、icon 輸出 `cargo:rerun-if-changed`；若版本由 Cargo 產生，也追蹤相關來源。
- `.res` 寫入 `OUT_DIR`；資源 include 路徑從 `CARGO_MANIFEST_DIR` 明確解析，支援空白與中文。
- `rc.exe /nologo /fo <res_path> <rc_path>` 回傳失敗即停止，保留工具診斷。
- 以指定 binary 的 Cargo linker arg 連結 `.res`，避免把 GUI resource／入口旗標誤加到測試 harness。
- 不加 `embed-resource`、`cc` 或其他 build crate。

### 14.2 `scripts/build.bat`

從任意工作目錄啟動都可用：`setlocal`、以 `%~dp0` 找 repository root、對路徑加引號，確保所有返回路徑 `popd`／正確回傳 exit code。

執行順序：

1. 檢查 rustup、cargo、rustc、鎖定工具鏈、MSVC target、rustfmt、clippy。
2. 確認 `rc.exe`、x64 MSVC `link.exe`。避免誤用其他軟體同名的 `link.exe`；找不到時提示開啟 x64 Developer Command Prompt／官方 `VsDevCmd.bat`。
3. 執行以下 gates，失敗立即停止：

```bat
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo build --release --locked
```

4. 確認 `target\x86_64-pc-windows-msvc\release\tools-screensaver-tzk.exe` 是本次成功產物。
5. 建立 `dist`，先複製至暫存名稱，成功後替換 `dist\tools-screensaver-tzk.scr`。
6. 回報完整路徑、版本、檔案 bytes、SHA-256、工具版本及原始碼 revision（若有 Git）。

- 不自動安裝缺失元件，不直接寫入 System32，不啟動 `/s` 或改使用者 screen saver 設定。
- 已有產物可保留供比對，但失敗時必須清楚標成前次產物，不能打印本次成功訊息。
- 依賴首次下載可使用網路；已快取依賴後應可 `--offline --locked` build。這項建置重現性與執行期模式分開：TimeDate／Countdown 及所有 preview 仍可離線，只有 `/s` JapanTravel 依第 8.6 節連線。

### 14.3 `scripts/package.bat`

- 以 `call` 執行 `build.bat` 並檢查 errorlevel；固定實際 ISCC 版本，不能找到任意版本就默默編譯。
- Inno Source 及 Output 路徑以腳本位置解析，避免 `installer/dist` 與根目錄 `dist` 混淆。
- 輸出先放本次 staging 目錄，只有編譯、版本及成品存在檢查通過後，才更新 `dist\tools-screensaver-tzk-Setup.exe`。
- 任何清理只限 repository 內已解析的本次 staging 路徑，不遞迴刪除使用者任意目錄。
- 記錄 `.scr` 與 Setup 的版本、大小、雜湊及未簽章／已簽章狀態；不能把舊 Setup 當成本次交付。

## 15. Inno Setup 安裝與解除安裝

### 15.1 架構、目錄與權限

第一版以原生 x64 為範圍：

```ini
[Setup]
ArchitecturesAllowed=x64os
ArchitecturesInstallIn64BitMode=x64os
PrivilegesRequired=admin
MinVersion=10.0.15063
DefaultDirName={autopf}\tools-screensaver-tzk
OutputBaseFilename=tools-screensaver-tzk-Setup
```

- 固定 AppId，版本升級不更換；uninstaller 放產品目錄，不能把解除安裝 metadata 任意散落 System32。
- 公開下載、`dist` 與 System32 安裝檔均使用 `tools-screensaver-tzk` 檔名前綴；`.scr` 安裝為 `{sys}\tools-screensaver-tzk.scr`，並明確使用 64-bit install mode 的實體 System32。Installer 繼續使用固定 AppId 以保留升級識別；改名前版本的實際升級與舊檔清理必須在可互動 Win10 環境另行驗證，未驗證前記為 `NOT TESTED`。
- `x64compatible` 也接受部分 ARM64 Windows，因此不符合本版限定範圍；這是產品支援範圍的選擇，不是聲稱 x64 程式技術上不能模擬執行。[Inno architecture identifiers](https://jrsoftware.org/ishelp/topic_archidentifiers.htm)
- 安裝成功建立標準解除安裝項目；不安裝字型或參考截圖。Setup 加入第 15.1.1 節的 WebView2 Runtime 先決條件；此為使用者要求新增的安裝功能，取代 v1.6 對 Setup 不安裝 Runtime 的限制。遠端驗證仍不得實際安裝 Runtime、開啟安裝精靈或觸發 UAC。
- `.scr` 與 installer 的權限分開：installer 提權不代表日後 `.scr` 提權。

### 15.1.1 WebView2 Runtime 先決條件

- Setup 為所有使用者安裝程式，檢查 `HKLM32\SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}` 的 `pv` 字串；版本需可解析且大於 `0.0.0.0`。不以提權帳號的 HKCU 推斷原使用者或其他帳號可用。
- 已安裝時顯示版本並略過；缺少時預設勾選 `webview2` task，於 `PrepareToInstall` 執行官方 Evergreen Bootstrapper `/silent /install`，沿用 Setup 權限。使用者可取消 task，只安裝離線時鐘模式。
- Bootstrapper 在封裝時由鎖定的 Microsoft HTTPS URL 取得；核對 SHA-256、檔案大小／版本與有效 Microsoft Corporation Authenticode 簽章後才內嵌。編譯與執行前再核對雜湊；完整 Runtime 於使用者安裝時從 Microsoft 下載。
- 必須等待程序結束、檢查 exit code 並重新偵測 Runtime。code 0 且重新偵測成功才繼續；3010／1641 視為需重啟，停止本次產品安裝；其他失敗顯示錯誤，允許重試或返回取消 task，不宣稱 Runtime 安裝成功。
- 解除安裝本程式不移除共用 Runtime。`.scr`、`/p`、`/c` 保持原有網路與提權邊界。
- 遠端可下載並驗證 Bootstrapper 供封裝，但不得執行它。測試使用 `PrivilegesRequired=lowest`、不建立 wizard、無 payload 的獨立 policy harness；只讀取 Runtime registry，不安裝、不寫系統設定。

### 15.2 使用者系統設定

預設勾選「設為目前的螢幕保護程式並啟用（閒置 1 分鐘後啟動）」；使用者可取消勾選以完全保留原有個人螢幕保護設定。只有選取時執行第 15.3 節。

```ini
[Tasks]
Name: "setcurrent"; Description: "設為目前的螢幕保護程式並啟用（閒置 1 分鐘後啟動）"
```

- 成功操作更新原使用者的 `HKCU\Control Panel\Desktop`：`SCRNSAVE.EXE` 為安裝後 `.scr` 完整路徑、`ScreenSaveActive` 為 REG_SZ `1`、`ScreenSaveTimeOut` 為 REG_SZ `60`。
- `ScreenSaverIsSecure` 必須保持原值；Setup 不選擇是否鎖定、不更動密碼或登入政策。不得寫其他使用者 hive，也不得寫 policy key 規避公司或學校的群組原則。
- 寫入後呼叫 `SystemParametersInfoW` 的 `SPI_SETSCREENSAVETIMEOUT` 與 `SPI_SETSCREENSAVEACTIVE`，使用 `SPIF_UPDATEINIFILE | SPIF_SENDCHANGE`，再讀回三個目標值並以有逾時的 `WM_SETTINGCHANGE` 通知 shell。
- 群組原則可覆蓋 HKCU 個人值；Setup 摘要、失敗訊息、README 與 Pages 必須如實說明，不能宣稱在受管理裝置必然準時啟動。
- 靜默安裝沿用預設勾選並套用上述設定；部署者可明確排除 `setcurrent` task。安裝 helper 不能顯示倒數輸入框或其他產品 UI。
- 一般安裝的完成頁提供預設勾選的「開啟 tools-screensaver-tzk『設定』面板」選項；使用者完成安裝時可取消。只有在完成頁仍勾選時，才以 `runasoriginaluser`、`/c`、`postinstall nowait` 開啟已安裝的 `.scr`。`/SILENT` 與 `/VERYSILENT` 必須以 `skipifsilent` 略過，不能在無人操作部署中開啟視窗。

### 15.3 提權後的帳號歸屬

**必要修正：** UAC 使用另一個管理員帳號時，installer 的 HKCU 不一定是發起安裝者；不得直接在提權 installer 的 `[Registry]` 寫 HKCU 來完成 setcurrent。

本版採同一 `.scr` 的單用途內部命令：

```text
tools-screensaver-tzk.scr --install-set-current
```

- 此命令只接受完整的一個旗標，不接受任意 registry key、檔案路徑或外部程式參數。
- installer 只有在 setcurrent 被選取時，以 `ExecAsOriginalUser` 執行已安裝 `.scr` 並等待結果；正常成功 code `0`，拒絕／失敗 code `4`。
- helper 先檢查自身為預期 System32 安裝路徑、token 非 elevated；不符合就拒絕，不降權猜帳號、不修改別人的 hive。
- helper 只依自身實際完整路徑寫入本 token 的 HKCU，設定程式路徑、60 秒逾時與啟用狀態，透過系統 API 套用、讀回查核，再以有逾時的系統設定變更通知更新 shell。不得修改 `ScreenSaverIsSecure` 或 policy key。
- 此路徑不載入畫面模式、不顯示設定或倒數對話框、不啟動 renderer、不需要額外 helper EXE／PowerShell runtime。
- 如果 installer 一開始就以管理員身分啟動、無法還原原使用者，或 helper 拒絕，安裝本體可成功，但必須明確顯示「未能完整套用目前程式與 60 秒閒置啟動」，引導使用者在自己帳號的 Windows 設定頁確認，並提示群組原則可能覆蓋；不得宣稱 task 成功。
- `ExecAsOriginalUser` 用於安裝階段且不支援 uninstall，不能直接把同一方案複製到解除安裝。[Inno：ExecAsOriginalUser](https://jrsoftware.org/ishelp/topic_isxfunc_execasoriginaluser.htm)、[runasoriginaluser 限制](https://jrsoftware.org/ishelp/topic_runsection.htm)

### 15.4 升級與解除安裝

- 固定 AppId 的 64-bit uninstall metadata 是版本衝突判斷依據。未安裝時直接繼續；已安裝版本與 Setup 版本等價時允許修復安裝，不產生重複解除安裝項目。
- 已安裝較舊、較新或無法解析的不同版本時，一般 Setup 必須顯示「已安裝版本」及「準備安裝版本」，詢問是否先移除既有版本再安裝本版。使用者拒絕即結束 Setup，不能寫入新檔。
- 使用者接受後，在複製新檔前以 uninstall metadata 指定的既有 uninstaller 靜默移除舊版，等待結束並再次確認 uninstall key 已消失；找不到 uninstaller、程序啟動失敗、非成功結果或 key 仍存在時停止安裝，不得留下第二套版本。
- `/SILENT`、`/VERYSILENT` 遇到不同版本時不得自行同意移除，必須停止並寫入 Setup log。同版修復仍可依既有靜默安裝契約執行。
- 舊版移除與新版安裝沿用相同 AppId 並保留 HKCU 偏好；舊 `.scr` 若正在使用，仍使用既有 close／restartreplace 規則處理。
- `.scr` 被使用中鎖住時，提示先關閉或依 installer 標準機制延後替換；不任意終止其他帳號／其他螢幕保護程序。
- 只移除本產品擁有的 `.scr`、uninstaller 與安裝檔案，禁止 System32 wildcard 清理。
- 第一版預設保留各帳號偏好，不枚舉、載入或清理其他使用者 hive。
- 原稿允許「提示或清除失效路徑」；本版採提示路徑：移除前明確說明仍選用本產品的帳號，移除後須在 Windows 螢幕保護設定改選其他項目／無。
- 提權 uninstaller 不假設自己 HKCU 是原使用者，因此不自動清除 `SCRNSAVE.EXE`。此限制列 README，靜默解除安裝列入 log。
- 使用者已選其他 screen saver 時其設定必須原封不動；不改逾時、安全、啟用值，不刪系統字型。

### 15.5 簽章與發布

- 開發交付可未簽章，須標示發行者驗證狀態；不要以「無 SmartScreen 提示」作未簽章版驗收要求。
- 公開發布：先簽 `.scr` → 封入 installer → 簽 Setup → 記錄最終已簽檔案雜湊。
- 憑證、私鑰、密碼不進 repository；公開上傳／發布不因完成 package 自動取得授權。

## 16. 錯誤處理與診斷

| 情境 | 必要行為 |
| --- | --- |
| `/s` 視窗／必要資源失敗 | 清理後 code 3；不另彈阻塞錯誤框 |
| 倒數輸入／保存失敗 | 留在已存在的輸入 dialog，指出原因，可重試／取消 |
| `/p` parent 無效 | 靜默 code 2；不退回 popup |
| `/c` 保存失敗 | 保留草稿，按第 10.3 節說明實際保存狀態 |
| 單一 registry 值損壞／缺字型 | 按已定 fallback，不 panic |
| 字型 dialog 取消 | 無錯誤提示、不修改草稿 |
| `GetMessageW=-1` | 記錄原始錯誤、清理所有視窗 |
| 安裝 helper 被拒絕 | code 4，installer 不宣稱已選為目前項目 |
| 日本目錄／候選來源失敗 | 保留目前健康影片；無健康影片時顯示 GDI 離線 fallback，按第 8.6 節有界重試 |
| WebView2 Runtime 缺失／player 建立失敗 | 顯示可退出的 GDI fallback；不下載 Runtime、不開瀏覽器、不顯示 UAC |
| 遠端 source 回傳惡意／超界資料 | 拒絕該候選、記錄不含私人資訊的階段與錯誤類別；不 navigation、不 panic |

- 有 `GetLastError` 契約的 API 失敗後立即保存 error，避免 cleanup 蓋掉。
- Registry API 使用其回傳 `LSTATUS`；ChooseFont 使用 `CommDlgExtendedError`；不對所有 API 一律讀 GetLastError。
- Debug 使用 `OutputDebugStringW`，記錄階段、API、錯誤碼；正式程式不逐幀寫 log、不記錄帳號、完整 IP、cookie、URL query 或不必要的瀏覽資訊。來源診斷最多記錄允許 host、camera ID、階段、狀態碼與 elapsed。
- `SetWindowLongPtrW` 等「0 也可能成功」API 依文件清除／檢查 last error，不能一律把零當失敗。

## 17. 測試計畫

### 17.1 自動測試案例

所有純邏輯測試可不建立視窗；Win32 部分使用同一套 Windows target 測試，不要求未安裝 SDK 的非 Windows 主機直接編譯 GUI。

| ID | 輸入／情境 | 預期 |
| --- | --- | --- |
| UT01 | 無參數、`/S`、`-s`、`/c 0` | Configure、Fullscreen、Fullscreen、Configure(None) |
| UT02 | `/p:4294967296`（只測 parser） | 數值不截斷；是否有效 HWND 由 adapter 另驗 |
| UT03 | `/p`、`/p -1`、`/p 0x20`、`/s /c`、溢位整數 | 明確 error，無意外 Fullscreen |
| UT04 | 2024-02、1900-02、2000-02 | 29、28、29 天 |
| UT05 | 2021-02-01（一）、2023-10-01（日） | offset=0／6；需要 4／6 週，layout 均預留 6 週 |
| UT06 | 2023-12-31（日） | 今日在第 5 個日期列、第 7 欄；不是固定附件圖像 |
| UT07 | 00:00:00、03:00:00、12:30:00 | 時針角度 0°、90°、15°；分針 0°、0°、180° |
| UT08 | 倒數 `00:00:00`、空白、`1a`、全形 `１`、分=60 | 拒絕；欄位錯誤可識別 |
| UT09 | 倒數 `00:00:01`、`00:59:59`、`99:59:59` | 1、3599、359999 秒；拆解重組相等 |
| UT10 | total=5000，now-start=0／1／1000／4999／5000／9000 ms | 顯示秒 5／5／4／1／0／0，ratio 限於 [0,1] |
| UT11 | remaining=10001／10000／1／0 ms | 最後十秒 false／true／true／false |
| UT12 | deadline 後 419／420／3359／3360 ms | 正常／變暗／變暗／永久正常；只 4 個暗半週期 |
| UT13 | 時間調整±1日，tick 不變 | 離機作業番茄鐘模式的倒數值不變；標準桌曆暨時鐘模式（`TimeDate`）由新快照更新 |
| UT14 | 一次跳過 10 秒／睡眠後已逾期 | 不逐 tick 補跑；依 deadline 算值，逾期=0 |
| UT15 | 0～9 segments | mask 與第 8.3.2 節一致；未點亮段不誤畫 |
| UT16 | xorshift 固定非零 seed、零 seed fallback | 可重現，無永遠輸出零的錯誤 seed |
| UT17 | 16:9、16:10、4:3、直向、超寬；比例 1.349／1.350 | 不裁切；明確切換上下／左右 |
| UT18 | 320×180、120×80、1×1、0×0；96／144／192／288 DPI | 合法退化、無除零／溢位／負尺寸 |
| UT19 | ±5% 位移、內容已滿安全區、最大字級 | bounding rect 含描邊後仍在界內，無空間軸位移=0 |
| UT20 | 缺值、錯誤型別、DWORD 非4 bytes、LOGFONT 非92 bytes | 按欄 fallback、不讀越界 |
| UT21 | schema 缺失／1／2／較新版 | 對應第 10.1 節，較新版不能被降版寫入 |
| UT22 | 點數 179／180／2400／2401、face 未終止／非法UTF-16 | 明確合法或 fallback，不直接傳不可信資料給 GDI |
| UT23 | 取消、ChooseFont 取消 | 保存呼叫次數=0 |
| UT24 | 模擬第 N 次寫入失敗，rollback 成功／失敗 | 顯示對應保存狀態，未修改未知值 |
| UT25 | 多視窗同 generation、連續 shutdown request | 共用秒數／ratio，關閉一次，最後才 quit |
| UT26 | schema 2 的 mode 0／1；schema 3 的 JapanTravel；schema 4 的 TravelStyle 0／1；schema 6 的 TravelStyle 2 與 ColorPreset 4～6；schema 7 的 TravelStyle 3／4；schema 8 的 ColorPreset 7；schema >8 | 舊值相容、五場景／色彩 round-trip、未知新版禁止降版寫入 |
| UT27 | 固定 seed；來源數 0／1／2／N；目前 index 位於頭尾 | 可重現、永不越界；候選多於一個時不立即重複目前來源 |
| UT28 | PLAYING 後 59999／60000 ms、一次跳過多分鐘、睡眠恢復 | 未到不切、到時只切一次、不補跑漏掉的分鐘 |
| UT29 | 固定 tw.live catalog marker／detail HTML fixtures；entity、缺欄、錯誤 host、非法 camera／video ID、控制字元與超長資料 | 只產生合法有界 metadata；格式錯誤可辨識，無 panic 或把不可信資料當程式碼 |
| UT30 | `https`／`http`、允許／非允許 YouTube host、非法 camera／video ID、地點中的 HTML／script／控制字元 | 只接受規格允許的來源；video ID 與地點以安全 JSON data 傳入固定 shell，不執行不可信內容 |
| UT31 | mock transport 的 200／3xx／404／timeout／TLS／取消；mock player navigation／PLAYING／error | 健康三層狀態正確；HTTP 200 不誤判成 `PlaybackHealthy`，failover 有界 |
| UT32 | 舊 generation 的 HTTP／WebView late completion；切換、resize、shutdown 重入 | 舊結果不覆蓋新來源，handler／controller／worker 只清理一次 |
| UT33 | `/p`、`/c`、TimeDate、Countdown 與 JapanTravel fallback 的 mock network 計數 | 前四條路徑計數為 0；fallback 不開 browser／dialog／installer、不觸發 UAC |
| UT34 | 控制字元、空白、極長 CJK 地名；小型、16:9、4K、直向 layout | 文字清理／省略正確；艙框及 label 不與 player rect 相交或超出 client |
| UT35 | 兩個 message-only host 的通知路由、單一來源失敗、地點／計時隔離、關閉與弱參照回收；前兩模式無 host | 狀態只更新對應 HWND；關閉全部 host 後路由失效；測試不建立可見 UI、player 或網路 |
| UT36 | schema 1～4／5／未知新版、TravelSwitchMinutes 0／1／1440／1441、錯誤型別／長度與取消 | 舊 schema 預設 1；本版合法值 round-trip；非法欄位回退，未知新版禁止寫入；取消不保存 |
| UT37 | 分鐘輸入空白、0、1、1440、1441、負數、小數、全形數字；切換模式與停用欄位 | 合法整數可提交；非法使用中欄位拒絕；不切換寫 0，其他模式不受停用輸入影響 |
| UT38 | 0／1／2／1440 分鐘設定、deadline 前後、最後 1 分鐘前後、睡眠跳過與失效 | 0 無排程／預抓但仍復原；長間隔僅最後 1 分鐘預抓，到時切一次；以 `PLAYING` 起算 |
| UT39 | 三個內附 PNG 的 WIC 解碼、輸出大小與離屏繪製 | 圖片完整、像素尺寸合法、資源釋放；無需網路、player 或可見 UI |
| UT40 | 自動色彩於 119,999／120,000 ms 邊界及完整循環 | 每 2 分鐘只前進一色，720,000 ms 顯示鐵灰色、840,000 ms 回到第一色，多螢幕共用 tick |
| UT41 | 四份指定 playlist 映射、清單更新／隨機選片／同片重播、地方散策眨眼與暗角結構 | 每種移動場景使用正確 playlist；失效復原有界；暗角與眨眼不攔截輸入 |
| UT42 | 未安裝、同版、較舊／較新版、格式異常、接受／拒絕移除、靜默衝突及 uninstall command | 同版可修復；不同版接受才移除，拒絕或靜默即停止；命令只抽取既有 uninstaller 執行檔 |
| UT43 | 顯示名稱、180 秒起點、最小播放器 UI、預備腳本與三種地方散策眨眼 | 設定／GDI／HTML 名稱一致；load/replay 都有 startSeconds；20～30 秒、BUFFERING 與完整換片動畫結構固定；預抓 completion 注入 `prepare` |
| UT44 | 安裝 helper 目標值、schema 8 鐵灰 round-trip、七色自動循環、眨眼漸層與 55 張 GDI fixture | helper 只規劃 SCRNSAVE.EXE／active／timeout 且排除安全值；鐵灰與深棕玻璃面板可辨識；漸層眼瞼仍可完全閉合 |
| UT45 | 番茄鐘玻璃的 GDI gradient、圓角 clipping、0×0～4K、96～288 DPI 與 50 次資源循環 | 不因空或極小矩形呼叫無效 clip；面板無貫穿全寬的分格式實色帶，平滑漸層在各尺寸可繪製且資源回收 |
| UT46 | 御運轉士窗孔／16:9 cover、地方散策閉合與展開、3:01～8:59 隨機起點、字幕關閉 hooks、游標停放及 Setup 完成頁入口 | player 填滿窗孔且只在容器內裁切；兩段眨眼與曲線模糊固定；所有 load/replay 使用隨機函式；ready／playing／API change 重申字幕關閉；Setup 的 `/c` 為 `postinstall skipifsilent runasoriginaluser` |

- Registry 測試用假的 store 或測試專用 HKCU 子 key；不得刪除／損壞真實使用者設定來跑預設自動測試。
- 視覺 fixture 注入固定日期、顏色、字型、尺寸與時間；不改系統時鐘。
- 純幾何測試不能取代實際 GDI 文字量測與截圖檢查。
- UT29～UT33 使用 repository 內固定 HTML fixtures、mock transport／player 或只監聽 loopback 的本機 server；預設 `cargo test` 不連公開 tw.live／YouTube、不建立 WebView2、不依賴外部網站當下狀態。

### 17.2 Windows smoke test

`scripts/smoke-test.ps1` 預設非互動，只驗證本次成品、架構、資源、版本、hash、第三模式與網路說明的 resource 字串、WebView2 靜態 loader／imports 及明確無 UI 的錯誤參數。互動模式需顯式 `-Interactive`。

- 使用唯一暫存目錄及 `.scr` 副本，不安裝、不改正式 screen saver 系統設定。
- 以 Process 物件保存 PID／開始時間，退出清理只作用於本次啟動的程序；不得按名稱廣泛結束所有 `.scr`。
- 設定有界 timeout，不留下測試視窗或永遠等待使用者。
- `/c`、`/s` 列互動驗證；測試 parent 可用小型 Win32 host。若未實作 host，`/p` 列人工 `NOT TESTED`，不能判成功。
- 故意設定 registry 損壞或改系統安全設定只能在隔離測試帳號／VM，預設腳本不得執行。
- 預設 smoke 不啟動 `/s`、不建立 WebView2、不連公開網站、不安裝／移除 WebView2 Runtime，也不觸發 UAC。`scripts/check-japan-sources.ps1` 必須另行顯式執行，並以有界非互動 HTTPS probe 輸出目錄與 camera URL、狀態碼、elapsed、解析出的 video ID、摘要及時間；它的 HTTP 成功不等於實際播放驗收。

### 17.3 人工／實機矩陣

| ID | 情境 | 必驗內容 |
| --- | --- | --- |
| MT01 | Windows 10 22H2 x64（目前必要平台） | 安裝、列舉、設定、小預覽、系統預覽、解除安裝；記錄 build |
| MT02 | 單螢幕 1920×1080／100% | 構圖、退出、色票、60分鐘內跨分鐘／日期 fixture |
| MT03 | 雙螢幕同 DPI、次螢幕左／上方 | 負座標、全覆蓋、同 generation 值、內部焦點切換不退出 |
| MT04 | 混合 100%／150%／200%、4K、直向 | 字型與 dialog 可讀、preview DPI、月底六列不裁切 |
| MT05 | Windows 設定頁保持開啟，`/c` 保存後返回 | 小預覽正常調度下 2秒內更新；取消無新設定 |
| MT06 | preview parent resize／關閉 | child 跟隨尺寸、parent 消失後程序結束 |
| MT07 | 倒數1秒、5秒、30分鐘、最大值 | 輸入、保存、預填、ceil、最後十秒、零點四次閃爍；最大值可僅驗初始顯示 |
| MT08 | 閒置自動啟動，三種模式，恢復登入選項開／關 | 實際能啟動／輸入／退出；安全流程由 Windows 掌控 |
| MT09 | 睡眠／恢復、校時、跨午夜 | 倒數按 deadline；月曆正確更新；不重播過期動畫 |
| MT10 | 斷開螢幕、變更拓撲、外部前景、登出 | 依第6節清理，不留黑窗／隱藏游標 |
| MT11 | 標準帳號用別的管理員通過 UAC | 系統檔案成功安裝；setcurrent 只作用於原使用者或明確報未套用 |
| MT12 | Setup 直接「以系統管理員身分執行」 | helper 拒絕不合條件操作，安裝不假報已切換 |
| MT13 | 較舊／較新版衝突、拒絕、接受、同版修復、靜默衝突、正在使用、解除安裝 | 顯示兩個版本；拒絕／失敗不安裝；接受後只留本版與單一解除安裝項；固定 AppId、偏好及其他 saver 設定不變 |
| MT14 | 兩種 GDI 模式各30分鐘；設定反覆切字型／DPI／resize | GDI／USER／記憶體穩定與完整釋放 |
| MT15 | 未安裝開發工具／VC++ Redistributable 的乾淨 Windows 10 x64 目標機 | 單一 `.scr` 無 VC/UCRT／WebView2Loader DLL 缺失；前兩模式離線可用，缺 WebView2 時旅行 fallback 可用 |
| MT16 | Win10 `/s` 日本旅行模式，正常網路，至少連續 5 次預設 1 分鐘輪換；另測自訂分鐘與不切換 | 實際 `PLAYING` 後依設定換不同來源；不切換持續播放且失效仍復原；地名吻合、靜音、3:01～8:59 隨機起播、控制列與可切換字幕不顯示，平台必要品牌未被覆蓋 |
| MT17 | tw.live／YouTube 不可達、timeout、所有候選失效、WebView2 Runtime 缺失 | 顯示正確 fallback、retry 有界、鍵鼠可退出；不開對話框／瀏覽器、不下載 Runtime或觸發 UAC |
| MT18 | 日本旅行模式多螢幕、150%／200%、4K、直向、resize／拓撲變化 | 每個螢幕各有一個 player 與真實地點／狀態；各自輪換與來源失敗互不覆蓋；框與 label 不裁切，清理後無殘留 child process |
| MT19 | 日本旅行模式 30 分鐘且至少 29 次來源輪換 | parent＋WebView2 process tree 的 CPU、記憶體、handle、controller 與子程序數無每分鐘持續累積 |
| MT20 | 前兩模式及所有 preview 的網路 capture；旅行模式的 outbound capture | 前者 0 request；後者只有目錄與官方 player 所需流量，無程式遙測、登入、下載、錄影或額外 top-level navigation |
| MT21 | v0.12.0 Setup 預設／取消 setcurrent、原使用者與另一管理員 UAC、受群組原則裝置；全螢幕游標及地方散策眨眼 | 預設值為本程式／active=1／timeout=60，安全值不變；取消時四值不變；policy 覆蓋時如實提示；游標顯示前隱藏且所有退出還原；眼瞼邊緣主觀柔和 |
| MT22 | v0.12.1 番茄鐘於 800×369 preview、1920×1080、4K／150% 與各固定主色 | 中央近黑褐透光、上下光衰減及左右瓶壁折射連續；細銅色唇邊與局部柔光不遮字；沒有巧克力分格感 |
| MT23 | v0.13.0 御運轉士／地方散策實際播放、鍵盤／滑鼠退出，以及一般／靜默 Setup 完成流程 | 御運轉士窗框無上下重疊與左右黑帶；地方散策可辨識閉眼與睜眼且邊緣柔和；游標先停於非 player 外角再隱藏；一般 Setup 可選擇開啟設定，靜默 Setup 不開 UI |

MT08 與 MT16～MT20 是必要產品相容性 gate：手動短暫 `Start-Process /s` 或公開網站 HTTP 200 不能代替它們。如果缺互動桌面、硬體、可控網路或 VM，一律記錄未測與缺少條件。

Windows 11 不列入目前 MT01 的必要範圍；待環境具備後補做上述矩陣，期間依第 0.4 節記錄延期，不因缺少 Windows 11 環境阻擋 Windows 10 開發與交付。

### 17.4 效能與記憶體

- 標準桌曆暨時鐘模式更新 1 Hz；離機作業番茄鐘模式在倒數進行／完成閃爍時更新 10 Hz，完成後為 1 Hz；日本旅行模式的 GDI 狀態只在狀態、resize 或低頻 timer 變更時 invalidate，影片由 WebView2 自行合成。不得存在未受事件節制的 loop。
- 基準環境：單螢幕 1920×1080、100% DPI、Release、無 debugger、預熱1分鐘後量測5分鐘。記錄 CPU 型號、邏輯核心數、解析度、DPI 及工具。
- CPU 目標：相對全機總能力平均 <1%。計算口徑為 `100×程序CPU秒增量/(牆鐘秒×邏輯核心數)`，不能把單核心百分比與工作管理員數值混用。
- 4K／多螢幕另報，不憑「一般桌面」宣稱通過。超標須記錄可重現原因及是否需優化。
- 記憶體規劃：32bpp full-screen buffer 約 `4×W×H` bytes；3840×2160 一張約31.6 MiB，雙4K約63.3 MiB，因此不採固定20MB總上限。
- 初始目標預算為 `24 MiB + 1.5×所有視窗預估 buffer bytes`，記錄 working set 與 private bytes；GDI bitmap 的實際計帳可能跨程序／系統，所以此公式是工程預算，不冒充精確實體 RAM 上限。
- 兩種 GDI 模式各跑30分鐘：預熱後每5分鐘記錄 CPU、working set、private bytes、GDI objects、USER objects。後20分鐘 GDI／USER 波動各以±10內且無持續上升趨勢為目標；private bytes 不應持續累積，超過 `max(4 MiB,預熱值10%)` 需查明。
- 日本旅行模式另跑 30 分鐘；以 parent PID 建立當次 WebView2 descendant process 集合，記錄 parent＋descendants 的 CPU、working set、private bytes、handle 與 process count，並逐次記錄 source generation。不得把 `msedgewebview2.exe` 排除後宣稱低資源；第 5 次切換後 process／controller／handle 數不得隨每分鐘持續增加，private bytes 增幅超過 `max(64 MiB,穩定值15%)` 必須查明。
- 重複建立／釋放50次視窗資源及font cache，資源計數回到穩定基線範圍；以所有權檢查與實測共同判斷 leak，不要求 allocator 立即把所有記憶體還給 OS。
- 不為達效能數字移除必要畫面、關閉高 DPI 或繞過雙緩衝。

### 17.5 視覺驗證產物

至少保留：800×369 參考比例、1920×1080、3840×2160、1080×1920、小型 Windows preview 及150%／200%設定 dialog 截圖。

- 標準桌曆暨時鐘模式的 `TimeDate` fixture 可用 2023-12-31、12:15:40 比對附件日曆與指針；產品正常模式仍使用真實時間。
- 每種色票在兩種 GDI 模式至少各檢查一次；特別確認灰白／亮綠 LCD 輪廓可讀與深紅黑底可辨。
- 日本旅行模式至少保留無網路 GDI fixture、1920×1080 實際播放、4K／150% 及直向／小型 preview 證據。實際播放圖須標示 capture 時間、地區、鏡頭、tw.live detail URL、player host 與健康層級；來源頁截圖不能冒充本程式畫面，靜態縮圖不能冒充影片播放。
- 截圖標示版本、尺寸、DPI、模式、色彩、字型、fixture／真實時間，不能用參考圖冒充實作截圖。

## 18. 驗收與完成定義

### 18.1 必要驗收清單

| ID | 通過條件 | 證據 |
| --- | --- | --- |
| AC01 | fmt、Clippy、tests、locked Release build 全通過；x64 GUI PE、正確manifest／資源／imports | 指令、exit code、PE檢查、MT15 |
| AC02 | `/s`、`/c`、無參數及錯誤參數契約正確 | UT01～UT03、MT01 |
| AC03 | `/p` 真正嵌入、resize／退出／DPI／設定刷新正確，無搶焦點或倒數輸入 | MT04～MT06 |
| AC04 | 原生設定可載入／預覽／選字型／提交／取消 | MT01、MT05 |
| AC05 | 指針、六列月曆、今天標示、左右／上下版面符合規格 | UT04～UT07、UT17～UT19、截圖 |
| AC06 | 倒數每次輸入一次，有效值開始、取消無全螢幕，真實閒置流程可用 | UT08～UT09、MT07～MT08 |
| AC07 | 六位數、沙漏、比例線、最後十秒／四次閃爍／保持零皆正確 | UT10～UT15、MT07、MT09 |
| AC08 | 多螢幕共用快照、負座標／混合 DPI 正確、拓撲變化清理 | UT25、MT03～MT04、MT10 |
| AC09 | 七種固定色、自動色彩與四字型可用、缺字fallback、暗色玻璃面板對比與極端點數不裁切 | UT18～UT22、UT40、UT44～UT45、視覺證據 |
| AC10 | 設定型別／schema／取消／部分失敗符合契約，不破壞未知值 | UT20～UT24、隔離registry測試 |
| AC11 | 所有指定輸入可退出；4px／500ms、同程序焦點、游標恢復正確 | MT02～MT03、MT10 |
| AC12 | 無busy loop／已知handle leak；兩種 GDI 模式與旅行模式各有30分鐘資源紀錄 | MT14、MT19、效能報告 |
| AC13 | Setup安裝至64位元System32、可由Windows選取、版本一致 | MT01、MT13、成品hash |
| AC14 | setcurrent 身分正確／失敗可辨；只改目前程式、60 秒逾時與啟用，不改安全、policy 或其他帳號設定 | UT44、MT11～MT13、MT21、前後值比較 |
| AC15 | README、原始碼、測試、必要成品及逐項驗收報告完整 | 第22節清單 |
| AC16 | 日本旅行的三種原創擬真場景、完整 player、城市／地區及鏡頭名稱符合 layout／第三方 player 規則 | UT26、UT34、UT39、MT16、MT18、視覺證據 |
| AC17 | 來源發現、預設 1 分鐘隨機輪換、三層健康檢查、failover、timeout 與 shutdown 均有界；自訂間隔與不切換符合 AC25 | UT27～UT32、UT38、MT16～MT19 |
| AC18 | 前兩模式與 preview 無網路；旅行模式 Runtime／斷線 fallback、outbound／隱私／授權揭露完整 | UT30～UT33、MT17、MT20、來源清單 |
| AC19 | 目前工作樹的英文產品識別、輸出檔名、resources、設定路徑、文件與 Pages 均為 `tools-screensaver-tzk`；文字及路徑掃描無舊識別 | Phase 8 report、resource smoke、repository scan、公開網頁與下載檔 |
| AC20 | 桌曆時鐘與番茄鐘在一般畫面置中於 64%W×60%H，內部元件不裁切；小型 preview 與日本旅行維持可讀面積 | layout tests、Phase 9 GDI fixtures、Phase 9 report |
| AC21 | Setup 正確略過既有 Runtime、補裝缺少 Runtime、支援取消 task，失敗與需重啟時明確停止；共用 Runtime 不隨產品解除安裝 | Phase 10 report、共用 policy tests、Bootstrapper 簽章與封裝、可互動 Win10 Runtime／網路／UAC 安裝矩陣 |
| AC22 | 多螢幕各自播放、輪換、地點與狀態正確；單一來源失敗隔離，退出清理所有 host，預覽不冒充連線 | Phase 11 report、雙 host 路由／失敗／清理測試、狀態 GDI fixture，以及可互動 Win10 多螢幕播放／DPI／資源觀察 |
| AC23 | 鐘面 12／3／6／9 目標字高 0.30R、中心距離 0.58R，與刻度分離；一般、小型、直向與高 DPI 版面無裁切 | Phase 12 GDI fixtures、視覺檢查 |
| AC24 | 兩種原創 AI 擬真場景內附於成品，GDI 預覽離線使用同圖，完整 16:9 player 與地名／狀態互不遮蔽；圖像來源如實揭露 | UT34、UT39、Phase 12 GDI fixtures、實際 Win10 player 視覺驗收 |
| AC25 | 設定可選不切換或 1～1440 整數分鐘，預設 1；schema 5 保存／舊版相容、取消不寫入、每螢幕獨立計時；不切換無預抓但失效可復原 | UT36～UT38、Phase 12 report、可互動 Win10 設定／播放驗收 |
| AC26 | 和風庭園擬真 PNG 內附於 SCR，本機 HTML 與 GDI 使用同圖；完整 16:9 player 位於庭園窗孔，來源如實揭露 | UT34、UT39、Phase 13 fixtures、實際 Win10 player 視覺驗收 |
| AC27 | 雪藍、琥珀、鐵灰色與自動模式可保存；自動模式每 120 秒循環七色且多螢幕一致，schema 6～8 舊值相容 | UT20～UT26、UT40、UT44、Phase 18 GDI fixtures、可互動 Win10 設定驗收 |
| AC28 | 四種移動場景在全螢幕啟動時更新指定 YouTube playlist 並隨機選片；和風庭園維持原來源 | UT41、Phase 14 source health、可互動 Win10 播放驗收 |
| AC29 | 御運轉士中央 16:9 player 由擬真設備包圍；地方散策為全畫面強烈攝影暗角且不含眼球／血管，換片保留眨眼 | UT34、UT39、UT41、Phase 15 fixtures、可互動 Win10 視覺驗收 |
| AC30 | Setup 準確辨識不同產品版本並詢問；接受後只留下本版及一個解除安裝項，拒絕、移除失敗或靜默衝突時不寫入新版 | UT42、Phase 16 policy report、MT13 |
| AC31 | 五個新顯示名稱一致；影片約從 3:00 開始、播放器控制列／預設字幕／註解停用、下一候選先預備；地方散策具有完整換片眨眼與依網路狀態變化的輕眨 | UT43、Phase 17 report、可互動 Win10 播放驗收 |
| AC32 | Setup 預設完整設定 60 秒閒置啟動且保留安全值；全螢幕顯示前隱藏並退出還原游標；眨眼邊緣柔和；鐵灰色與深棕玻璃番茄鐘面板完成 | UT44、MT21、Phase 18 report、可互動 Win10 安裝與視覺驗收 |
| AC33 | 番茄鐘面板以平滑 GDI 漸層形成近黑褐核心、上下光衰減與左右琥珀瓶壁折射；外框精簡且無貫穿全寬的實色分格 | UT45、MT22、Phase 19 GDI fixtures、可互動 Win10 視覺驗收 |
| AC34 | 御運轉士影片貼合窗孔；地方散策有曲線模糊的閉眼與睜眼兩段；影片從第 3 分鐘後隨機起播並主動關閉可切換字幕；全螢幕游標停放於非 player 區後隱藏；一般安裝完成頁詢問是否開啟設定且靜默安裝略過 | UT46、MT23、Phase 20 HTML fixtures、installer source／package |

### 18.2 報告格式

`docs/acceptance-report.md` 至少包含：版本、source revision、工具版本、OS build、硬體、執行日期、每個 AC／UT／MT 狀態、證據路徑及限制。

```markdown
| ID | 狀態 | 環境／實際操作 | 證據 | 失敗原因或缺少條件 |
| --- | --- | --- | --- | --- |
| AC01 | NOT TESTED | 尚未執行 | — | 待 Windows 建置 |
```

- `PASS`：實際執行並符合；`FAIL`：已測不符合；`NOT TESTED`：未測；`NOT APPLICABLE`：確實不適用且附理由。
- 三種模式、Windows 10、系統閒置／登入恢復、旅行來源實際播放／失敗及 UAC 帳號測試不能以 `NOT APPLICABLE` 逃避必要驗收。Windows 11 是使用者明確延後的項目，按第 0.4 節記錄。
- 目前 Windows 10 範圍內所有必要 AC 均 PASS，才能稱「Windows 10 完整驗收完成」。範圍內仍有未測項時可交付原始碼或標註限制的候選安裝包；Windows 11 延期不阻擋這項判定，也不能因此宣稱所有平台已通過。
- 編譯成功、幾何單元測試、程式碼審查不等於實機畫面、安裝及安全桌面驗證通過。

## 19. Codex 分階段實作計畫

### 19.1 執行規則

Phase 0 → 1 → 2 → 3 → 4 → 5 已完成既有雙模式基線；Phase 6～12 依序加入日本旅行、雙旅行場景、產品識別、置中畫布、WebView2 部署、多螢幕修正及來源切換設定；Phase 13～15 加入旅館、新色彩、指定影片清單及旅行視覺；Phase 16 加入安裝版本衝突處理；Phase 17 改良播放與顯示名稱；Phase 18 完成啟動設定、游標、眨眼及鐵灰色；Phase 19 重製番茄鐘暗色藥劑瓶玻璃面板；v2.7 的 Phase 20 修正旅行播放與安裝完成設定入口。每階段保留可建置成果與當時證據。

- 使用者只指定某階段時，只完成該階段；完整交辦時依序持續執行，不重複要求已授權的下一階段確認。
- 開始前閱讀現有專案與上階段結果；不覆蓋無關修改、不為配合文件重建已有正常程式。
- 小步完成可編譯功能，針對變更執行必要檢查；階段末執行完整相關 gates。
- 不關閉警告、刪失敗測試或以 blanket `allow` 掩蓋問題；必要例外須具體說明。
- 功能驗收若受環境阻擋，先完成不受阻擋的工作、記錄缺口；不能把缺少實機當成所有開發都必須停下。
- 每階段回報：完成內容、修改檔案、實際指令及結果、已知限制、未測項與下一步。

### Phase 0：工程基礎

**目標：** 證明 Rust／MSVC／RC／manifest／靜態CRT可以產生可執行成品。

1. 建立 Cargo binary＋library，固定 toolchain、target、lockfile、Release profile。
2. 建立最小入口與 UTF-16／error 邊界，避免空模組。
3. 加入合法圖示、名稱／版本resource、manifest與build.rs。
4. 完成 README 工具準備與從含空白／中文路徑建置說明。
5. 執行 fmt、`cargo check --locked --all-targets`、Release build，檢查 PE 資源及 imports。

**產出／完成：** 最小 x64 GUI exe、可追溯建置、RC變更會觸發重建；此階段不假裝已完成螢幕保護功能。

### Phase 1：命令列與視窗生命週期

1. 完成純 parser＋Win32 HWND驗證；測試 UT01～UT03。
2. 建立隱藏多螢幕視窗、統一顯示、共用shutdown及錯誤清理。
3. 完成 `/p` 的 child、parent檢查、resize、DPI context。
4. `/c` 可先以明確標示的暫時 Win32入口驗證 owner；Phase 3必須取代。
5. 完成鍵鼠退出、游標、同程序焦點規則、拓撲／session退出。

**驗證：** fmt、Clippy、tests、Release；單／雙螢幕黑畫面、負座標、真正parent preview、失敗不殘留視窗。

**完成：** 三條路徑正確；尚不要求正式renderer或registry。

### Phase 2：畫面與純時間邏輯

1. 建立 `FrameSnapshot`、layout、GDI雙緩衝、快取與ownership。
2. 完成鐘面／Gregorian月曆；七段／LCD／沙漏／進度線。
3. 完成deadline、ceil、最後十秒、4次閃爍、恢復時重算。
4. 完成DPI、窄版、極小預覽、位移與共用coordinator。
5. Debug限定 `--dev-render=time-date`／`--dev-render=countdown`，使用固定設定及正常window cleanup；Release拒絕。
6. 固定日期／now輸入以fixture注入，可保留在tests或Debug helper，不增正式使用者功能。
7. 執行 UT04～UT19、UT25與必要視覺比對；至少10分鐘初步資源觀察。

**驗證：** 通用gates，加：

```bat
cargo run --locked -- --dev-render=time-date
cargo run --locked -- --dev-render=countdown
```

**完成：** 可獨立驗證兩種畫面；不讀寫正式偏好、不以Debug入口冒充正式倒數輸入。

### Phase 3：設定、字型與倒數輸入

1. 完成 AppConfig／ConfigDraft、schema、registry讀寫／rollback。
2. 加入正式 RC dialogs、繁體中文標籤、tab order、owner管理。
3. 完成ChooseFont、逐角色適配、CJK fallback及四色contrast。
4. `/c`草稿即時preview；`/p`每秒設定刷新；`/s`維持啟動快照。
5. 完成每次倒數輸入、上次值、只保存所屬欄位、取消與保存失敗。
6. 執行 UT20～UT24、MT05～MT08；移除暫時設定MessageBox。

**完成：** 模式切換→保存→系統preview→fullscreen形成完整流程；實際閒置互動如有問題須於此階段揭露，不拖到發布才發現。

### Phase 4：品質與相容性

1. 稽核所有FFI、ownership、CreateWindow失敗／重入、GDI選入與釋放。
2. 完成有界smoke-test，不改正式使用者系統設定。
3. 跑必要Windows／DPI／多螢幕／安全桌面矩陣，記錄確切環境。
4. 各模式30分鐘與反覆resize／font循環，記錄資源和效能。
5. 驗證離線、靜態CRT結果、Release無Debug入口、成品manifest及版本。
6. 完成報告；不在本階段加入新功能或更換繪圖技術。

**完成：** 已有環境的必要項通過、無已知資源洩漏；缺環境則明列未驗證，不稱完整平台驗收。

### Phase 5：封裝與交付

1. 完成build／package腳本、固定版本與暫存產物處理。
2. 建立Inno Setup、穩定AppId、64位System32安裝與原使用者setcurrent helper。
3. 驗證乾淨安裝、同版覆蓋／升級、檔案使用中、解除安裝。
4. 測試task未勾／已勾、標準使用者跨帳號UAC、直接提權與helper失敗。
5. 輸出README、視覺證據、AC逐項報告、`.scr`／Setup及hash。
6. 發布候選若有 NOT TESTED 必須明確標示；公開版本依 MIT License 發布，未簽章成品須保留 NotSigned 說明。

**驗證：**

```bat
scripts\build.bat
scripts\package.bat
powershell -NoProfile -File scripts\smoke-test.ps1
```

**完成：** 同一source state可產生本次成品，使用單一Setup安裝／移除，目前 Windows 10 範圍的所有必要AC有實際結果；只有這些項目全PASS才宣稱 Windows 10 完整驗收。Windows 11 依第 0.4 節延期。

### Phase 6：日本旅行模式

**目標：** 在不破壞既有兩種離線模式、Windows 契約及遠端無互動驗證邊界的前提下，加入可安全失敗的線上日本旅行窗景。

1. 將軟體版號升為 `0.2.0`、registry schema 升為 3，加入 `JapanTravel=2` 與第三個設定 radio；驗證 v0.1.x 設定相容及較新版 schema 保護。
2. 當時完成主螢幕本機 HTML／CSS 客艙 shell、完整 player rect 外的 label、GDI 靜態伴隨／fallback，以及無網路 `/p`／`/c` 預覽；多螢幕配置於 Phase 11 改為每螢幕播放。
3. 加入固定版本 WebView2 binding 與靜態 loader，啟動時只探測 Evergreen Runtime；缺少時不安裝、不提權，前兩模式不載入 WebView2。
4. 完成 WinHTTP background source worker、tw.live catalog marker／8 個 camera detail parser、固定 HTTPS host、禁止 redirect、size／timeout／文字驗證、process-local session state、generation 及 bounded shutdown 生命週期。
5. 完成官方 YouTube embed、player 健康事件、靜音、60秒 monotonic 隨機輪換、無立即重複與有界 failover；保持 player 全部可見且不遮 attribution／controls。
6. 執行 UT26～UT34；更新 smoke resource／imports 檢查。CI 只用 fixtures／mock，不啟動 UI、WebView、外網或 UAC。
7. 另行執行有界公開來源 HTTP probe並寫入 `docs/japan-travel-sources.md`；HTTP 200 只算目錄／候選可達，不算 `PLAYING`。
8. 在可互動 Win10 桌面執行 MT16～MT20 與視覺／30分鐘資源驗收；目前遠端環境不能執行者保持 `NOT TESTED`。
9. 更新 README、Pages、視覺文件、acceptance report、Phase 6 report、Release notes、Setup 與 SHA-256。清楚揭露 WebView2、網路、第三方資料及授權邊界。

**非互動驗證：**

```bat
scripts\build.bat
scripts\package.bat
powershell -NoProfile -NonInteractive -File scripts\smoke-test.ps1
powershell -NoProfile -NonInteractive -File scripts\check-japan-sources.ps1
```

最後一項會連公開網站，須另列結果且不可成為 deterministic CI gate；前三項不得建立 WebView2 或觸發 UAC。若尚未完成 MT16～MT20，可發布明列限制的候選版，但不能宣稱日本旅行模式或 Windows 10 完整驗收完成。

### Phase 7：雙旅行場景

**目標：** 將原客艙窗框命名為「自在飛行」，加入可保存的「列車旅行」場景，且不改變既有來源、播放器與遠端無互動邊界。

1. 軟體版號升為 `0.3.0`、registry schema 升為 4，新增 `TravelStyle=0/1`；schema 3 與缺值設定預設為自在飛行。
2. 設定對話框新增兩個場景 radio，只在日本旅行模式時啟用；確定才保存，取消不寫入。
3. HTML／CSS player shell 與 GDI preview／fallback 都實作兩種自製場景；列車旅行參考暖色木質車廂、拱形頂棚、窗列與餐桌座位，但不納入附件或第三方照片。
4. 兩種場景均保留完整 16:9 player rect、player 外地名／狀態、靜音、單播放器、60 秒輪換、來源健康檢查與有界 failover。
5. 執行 noninteractive build、兩種 shell／GDI fixture、registry migration、resource smoke 與封裝；不開啟 `/s`、設定 dialog、安裝程式或 UAC。
6. 更新 v1.4 規格、README、Pages、acceptance report、Phase 7 report、v0.3.0 release notes 與 SHA-256，再發布 GitHub Release。

### Phase 8：產品識別統一

**目標：** 將 repository 中目前版本的產品、原始碼、建置、安裝、設定儲存、文件與公開網頁識別完整統一為 `tools-screensaver-tzk`。

1. 軟體版號升為 `0.4.0`；Cargo package／binary 與輸出 EXE 使用 `tools-screensaver-tzk`，Rust crate 因識別字規則使用 `tools_screensaver_tzk`。
2. 更名主要規格與 Inno source 檔，並統一 VERSIONINFO、manifest、window class／title、Registry key、WebView2 資料目錄、User-Agent、腳本、測試與 CI artifact。
3. 保留 installer 固定 AppId；不在遠端工作階段執行需 UAC 的改名前版本升級矩陣。Registry 與 WebView2 資料路徑改用新識別，既有個人設定不自動遷移。
4. 在當前工作樹對文字內容及路徑做大小寫無關掃描，不得殘留改名前的英文識別；Git commit／tag 歷史不改寫。
5. 執行 noninteractive fmt、Clippy、45 個預設測試、Release build、resource／PE／registry smoke、Runtime probe、即時來源 probe 與 Inno Setup 封裝，全程不開啟交互畫面或觸發 UAC。
6. 更新 v1.5 規格、README、Pages、acceptance report、Phase 8 report、v0.4.0 release notes、公開下載檔與 SHA-256，發布 GitHub Release 後驗證匿名直連。

### Phase 9：置中緊湊版面

**目標：** 降低桌曆時鐘與番茄鐘在大型畫面的佔用比例，保留易讀性、多螢幕、DPI、預覽與防烙印邊界。

1. 軟體版號升為 `0.5.0`。
2. 畫面至少 640×360 時，`TimeDate` 與 `Countdown` 的主群組使用置中 64%W×60%H；小於此門檻的 preview 維持 88%W×82%H。
3. `JapanTravel` 維持 88%W×82%H，不因離線模式改動而縮小 player。
4. 新增大畫面置中比例、小型 preview 比例與旅行畫布不變的 deterministic tests，並重新輸出 800×369、1920×1080、3840×2160、直向及極小 GDI fixtures。
5. 執行 noninteractive fmt、Clippy、46 個預設測試、Release build、resource／PE／registry smoke 及 Inno Setup 封裝，不開啟交互畫面或 UAC。
6. 更新 v1.6 規格、README、Pages、acceptance report、Phase 9 report、v0.5.0 release notes、公開下載檔與 SHA-256，發布 GitHub Release 後驗證匿名直連。

### Phase 10：Setup WebView2 部署

**目標：** 在使用者安裝階段完成 Runtime 偵測、缺少時補裝、錯誤處理及可略過的離線路徑。

1. 軟體版本升為 `0.6.0`，實作第 15.1.1 節並封裝已驗證的官方 Bootstrapper。
2. 同一套 Pascal policy 由正式 Setup 與非互動 harness 共用，覆蓋缺失／零／非法版本、已安裝／略過、啟動失敗、code 0 未偵測到 Runtime 及需重啟結果。
3. 重跑 build、46 個預設測試、19 個 installer policy checks、smoke 與 package；現有 37 張 GDI 圖片沿用 Phase 9，明列日期與版本。
4. 更新 README、網頁、規格、Phase 10 與逐項驗收報告、Release 與公開下載；實際 Runtime 安裝、斷網、Proxy、UAC 帳號矩陣仍待可互動 Win10 驗證。

### Phase 11：多螢幕旅行播放修正

目標：修正第二螢幕沒有 player，卻一直顯示連線中／失敗文字的問題。

1. 軟體版本升為 `0.7.0`；每個 surface 建立獨立 TravelHost，來源、播放計時、WebView2 事件、錯誤與狀態依所屬 HWND 路由。
2. 保持每個螢幕一個 player，切換重用 controller；退出時關閉全部 host，過期結果不再改寫狀態。
3. GDI 預覽明示「靜態預覽」；等待或錯誤畫面採該螢幕真實狀態與地點，區分網路問題和播放器不可用。
4. 遠端只執行非互動回歸測試、離屏 GDI 匯出、build／smoke／package；不開正式 player、全螢幕、Setup 或 UAC。實際雙螢幕播放與長時間資源觀察列 `NOT TESTED`。
5. 更新 README、Pages、逐項報告與 `docs/phase11-report.md`；發布新 Release 與匿名直連下載，核對成品 SHA-256。

### Phase 12：鐘面間距、擬真旅行場景與來源切換設定

目標：讓鐘面數字與刻度保持間距，提供逼真的飛行與列車窗框，讓使用者控制旅行來源的停留時間。

1. 軟體版本升為 `0.8.0`；鐘面數字目標字高由 0.40R 改為 0.30R，中心距離由 0.62R 改為 0.58R，保留置中群組、小型預覽與字型適配。
2. 原創 AI 擬真飛行／列車 PNG 內附於產品；WIC 解碼供 GDI 預覽及 fallback，本機 HTML 使用同圖且完整 player 不受遮蔽。公開文件清楚標示虛構場景，不宣稱第三方照片或特定運具。
3. schema 升為 5，保存 `TravelSwitchMinutes`；預設 1、0 不切換、1～1440 為整數分鐘。原生設定新增切換方式及分鐘欄位，保留草稿、取消、舊 schema 與未知新版保護。
4. 各螢幕在 `PLAYING` 後按設定獨立計時；長間隔至剩餘最後 1 分鐘才預抓，不切換停用排程與預抓，來源失效仍有界復原。
5. 執行非互動回歸測試、WIC 解碼、離屏 GDI 匯出、build／smoke／package；不開設定 dialog、正式 player、全螢幕、Setup 或 UAC。實際播放、自訂間隔、不切換、多螢幕及長時間資源觀察列明驗證缺口。
6. 更新 README、Pages、來源／視覺文件、逐項報告及 `docs/phase12-report.md`；發布新 Release 與匿名直連下載，核對成品 SHA-256。

### Phase 13：日式旅館與桌曆時鐘色彩

目標：新增安靜的日式旅館旅行場景，並擴充桌曆時鐘主色與自動換色。

1. 軟體版本升為 `0.9.0`、schema 升為 6；加入 `JapaneseInn=2`、暗淺藍、琥珀色與自動色彩的 registry round-trip，舊 schema 不誤解新版 enum。
2. 以使用者附件作空間風格參考，產生不複製附件像素、無真實旅館識別的原創擬真 PNG；內附於 SCR、本機 HTML 與 GDI fallback，保留完整無遮蔽 16:9 player。
3. 自動色彩使用共同 monotonic tick，每 120 秒依序循環六種固定色；加入精確邊界與完整循環測試。
4. 設定畫面新增第三場景及三個色彩 radio；選取、保存、草稿預覽、鍵盤分組與舊設定相容。
5. 執行非互動 build、測試、離屏 GDI／headless HTML、smoke 與 package；不開正式 UI、player、Setup 或 UAC。實際多螢幕 player、設定畫面 DPI 與長時間自動色彩列明驗證缺口。
6. 更新 README、Pages、視覺／素材／驗收文件及 `docs/phase13-report.md`；發布 v0.9.0 Release 與匿名 Pages 直連下載。

### Phase 14：影片清單、列車駕駛前方與第一人稱散步

目標：依場景使用指定的最新影片清單，新增兩種擬真視角，並讓散步換片具有自然的人眼眨眼過場。

1. 軟體版本升為 `0.10.0`、schema 升為 7；新增 `TrainCab=3` 與 `Walking=4`，schema 6 不得誤解新版 enum。
2. 自在飛行、列車旅行、列車駕駛前方與散步模式各使用指定 playlist ID；每次全螢幕啟動及切換到期時由 YouTube IFrame API 重新讀取清單、隨機排列及選片，不解析 YouTube HTML，不保存過期影片清單。日式旅館保留 tw.live 來源。
3. 新增無人物列車駕駛室及第一人稱人眼原創擬真 PNG。散步 player 縮至中央 60%×54%，四周保留大面積黑色周邊；換片時以 700 ms 上下眼瞼閉合，300 ms 完全閉合點開始載入新來源。
4. 空清單、player error、未開始播放與網路失敗必須有界復原；不切換時影片結束重播同一支，來源失效仍可重試。每個螢幕保持獨立清單與計時狀態。
5. 執行非互動 build、測試、52 張 GDI、10 張封鎖網路的 HTML、來源 HTTP probe、smoke 與 package；正式 WebView2 播放、Setup、UAC 與 Windows 11 列明未測。
6. 更新 README、Pages、來源／素材／視覺／驗收文件及 `docs/phase14-report.md`；發布 v0.10.0 Release 與匿名 Pages 直連下載。

### Phase 15：列車駕駛室修飾與散步攝影暗角

1. 軟體版本升為 `0.10.1`，registry schema 不變。
2. 列車駕駛前方的中央 player 保持 16:9；左右不得留下 player 造成的大面積純黑區，改由原創擬真設備櫃、螢幕、通風板、金屬接縫及控制台包圍。
3. 散步場景移除眼球、皮膚、血管與眼鏡語彙；影片鋪滿 16:9 場景，再以強烈徑向攝影暗角把主要視域集中在中央約 60%～70%。
4. 保留散步來源切換的 700 ms 眨眼過場；暗角層與眨眼層都不得攔截輸入。
5. 重跑非互動 build、GDI／HTML fixtures、smoke 及 package，更新 README、Pages、視覺與驗收文件，發布 v0.10.1。

### Phase 16：安裝版本衝突處理

1. 軟體版本升為 `0.10.2`，registry schema 與固定 AppId 不變。
2. Setup 讀取固定 AppId 的 64-bit uninstall metadata，比對已安裝版本與本版；同版允許修復安裝。
3. 不同版本的一般安裝顯示兩個版本並詢問；接受後在複製新檔前執行既有 uninstaller，等待完成並確認 uninstall key 消失。拒絕、找不到 uninstaller、移除失敗或 key 仍存在時停止。
4. 靜默安裝遇到不同版本時停止並記錄，不自動替使用者同意移除；沿用固定 AppId，確保單一 System32 檔與單一解除安裝項。
5. 新增共用 Pascal policy 與最低權限非互動 harness，涵蓋 15 個版本判斷及 uninstall command 案例；不得顯示 wizard、提示、執行舊 uninstaller、寫 registry 或觸發 UAC。
6. 重跑 build、smoke 與 package，更新 README、Pages、規格及驗收文件，發布 v0.10.2；實際升級／降版／同版修復與 UAC 流程列為人工驗收。

### Phase 17：播放體驗與顯示名稱

1. 軟體版本升為 `0.11.0`，registry schema、內部 enum 與固定 AppId 不變。
2. 顯示名稱改為和風庭園、御運轉士、地方散策、雪藍與琥珀；設定資源、GDI fallback、HTML aria、README、Pages 與現行來源文件一致。
3. 所有 YouTube 載入與同片重播使用約 180 秒起點；以現行官方參數停用控制列、預設字幕、註解、鍵盤與全螢幕按鈕。平台必要的短暫標題或品牌資訊列為第三方限制，不用覆蓋層遮蔽。
4. 地方散策切換來源使用約 780 ms 完全閉眼；播放順暢時每 20～30 秒輕眨，BUFFERING 或時間進度異常時使用有節制的較慢眨眼。
5. 定時切換前最後一分鐘先由原生 worker 解析來源，WebView 預選不同影片清單候選並預熱縮圖/CDN 連線；切換時重用單一 player，不能建立隱藏播放實例。
6. 重跑非互動 build、smoke、installer policy 與 package，更新 README、Pages、規格及驗收文件，發布 v0.11.0；實際 player UI、字幕偏好、三分鐘 seek、眨眼流暢度及網路節流列為可互動 Win10 驗收。

### Phase 18：啟動設定、游標、眨眼與色彩面板

1. 軟體版本升為 `0.12.0`、registry schema 升為 8；固定 AppId、既有 enum 值與來源設定保持相容。
2. Setup 的 `setcurrent` 預設勾選，以 `ExecAsOriginalUser` 執行 helper；設定 `SCRNSAVE.EXE`、`ScreenSaveActive=1` 與 `ScreenSaveTimeOut=60`，透過 `SystemParametersInfoW` 立即套用，保留 `ScreenSaverIsSecure`，並明列群組原則可能覆蓋。
3. 全螢幕在第一個 surface 顯示前隱藏游標，`WM_SETCURSOR` 持續維持隱藏；正常與錯誤退出沿同一清理路徑還原保存的 borrowed cursor handle。
4. 地方散策眼瞼高度提高為足以完整閉合的 60%，末端使用多段黑色透明漸層，柔化完整換片、定時輕眨與網路眨眼的可見切線。
5. 新增 `IronGray=7`（RGB 154,160,163）及設定 radio；自動模式每 2 分鐘循環七種實色。番茄鐘面板改成深棕玻璃基底、棕銅框、上方反光帶與底部暗帶。
6. 執行非互動 build、59 個預設測試、smoke、34 個 installer policy checks、55 張離屏 GDI fixture 與 package；不開啟 `/s`、設定 UI、Setup，不執行安裝或觸發 UAC。更新 README、Pages、規格及驗收文件並發布 v0.12.0；實際閒置啟動、policy、游標與眨眼觀感列為可互動 Win10 驗收。

### Phase 19：暗色藥劑瓶玻璃面板重製

1. 軟體版本升為 `0.12.1`，registry schema、AppId、設定值與旅行來源保持不變。
2. 移除番茄鐘面板橫跨全寬的上方反光帶與底部暗帶，避免巧克力分格外觀。
3. 在共用 GDI Canvas 加入有界 `GradientFill` 與圓角 clipping；對空或極小矩形安全略過。
4. 中央以近黑褐上下漸層模擬透光衰減，左右以相反方向琥珀褐漸層模擬厚瓶壁折射；外圈只留深棕瓶身、細銅色唇邊、低亮度內框與短局部柔光。
5. 重跑非互動 build、59 個預設測試、smoke、34 個 installer policy checks、55 張離屏 GDI fixture 與 package；更新 README、Pages、規格及驗收文件並發布 v0.12.1。實際桌面觀感列為可互動 Win10 驗收。

### Phase 20：旅行播放、游標與安裝完成設定

1. 軟體版本升為 `0.13.0`；registry schema、AppId、設定值及來源識別保持不變。
2. 全螢幕建立隱藏 surfaces 後，先把游標移到主要螢幕左上外角的非 player 區域，再保存新的輸入 baseline 並隱藏；`WM_SETCURSOR` 與既有清理路徑仍負責持續隱藏及恢復游標形狀。
3. 御運轉士窗孔調整為擬真素材的實際開口，16:9 player 置中放大並由窗孔裁掉少量上下影像，使左右填滿且不跨入窗框；地方散策保留 0.4% 非 player 外緣。
4. 地方散策換片改成約 420 ms 閉眼與約 520 ms 睜眼兩段；上下眼瞼寬 116%、高 62%，使用橢圓曲線與 9～18 px 模糊，完整閉合時保持黑色重疊。定時與網路眨眼沿用既有節制。
5. 每次影片載入、影片清單選片及片尾重播，均從 3:01～8:59 重新隨機選擇起點。player 保留 `cc_load_policy=0`，並在 ready、load、playing 與 `onApiChange` 重申關閉可切換字幕；來源影片內嵌文字不在可控制範圍。
6. 移除產品 UI 的「播放」加「清單」舊組合，改用場景名稱、「影片清單」或「清單」。
7. 一般 Setup 完成頁預設勾選是否以原使用者身分執行已安裝 `.scr /c`；使用者可取消，靜默安裝固定略過。遠端驗證不得執行 Setup、開啟設定或觸發 UAC。
8. 重跑非互動 build、60 個預設測試、smoke、installer policy、10 張離線 HTML fixture 與 package；更新 README、Pages、規格及驗收文件並發布 v0.13.0。實際字幕偏好、眨眼動態、游標位置、旅行播放與完成頁互動列為可互動 Win10 驗收。

### 19.2 可直接交給 Codex 的任務範本

以下是日後實作時可採用的提示，不表示閱讀本文件就應立即執行：

```text
請依 tools-screensaver-tzk_Codex_Spec.md v2.7 實作指定 Phase。
先讀取 AGENTS.md、現有程式與工具鏈，保留無關修改。
只完成本階段，執行文件要求且環境可執行的驗證。
回報修改檔案、實際命令、結果與未測項；不可把未驗證寫成通過。
```

若要完整交辦，可明寫「完成 Phase 6，在已授權範圍內持續執行；遠端驗證不得開 UI 或觸發 UAC，缺互動實機情境時完成其他工作並列出缺口」。

## 20. 明確禁止事項

- 以高階 GUI、通用 WebView 或新 crate 取代既有 Win32／GDI 實作；第 8.6 節限定的 WebView2 player binding 是唯一例外。
- 直接複製或嵌入 tw.live 整頁 HTML／CSS／JavaScript、執行其廣告／追蹤碼，或把遠端頁面當成產品 UI。旅行模式只解析有界 metadata 並使用官方 player。
- 在 `/p`、`/c`、TimeDate 或 Countdown 發出 request，或在 network callback 同步等待、無界 retry、接受非 HTTPS／非允許 top-level navigation。
- 把 HTTP 200、縮圖或 iframe document 成功寫成影片已播放，或把公開來源暫時可達寫成永久授權／可用性保證。
- 持續在 YouTube player 上疊旅行框／地名、以覆蓋層遮廣告／branding、同一 screen 同時 autoplay 多個 player、建立隱藏預載 player，或用錯誤 Referer／nested iframe 規避政策；地方散策只允許規格定義的短暫眨眼動畫。
- 讓倒數模式每螢幕建立自己的 deadline、按 timer 次數遞減或 paint 時各取不同時間。旅行模式的來源播放計時依第 8.6 節各自保存。
- 在preview中顯示輸入框、topmost、隱藏全域游標或啟用fullscreen退出規則。
- 倒數輸入未完成便鋪全螢幕、吞掉使用者輸入或由保存失敗直接開始。
- 刪除仍選入DC的GDI object、刪stock object、混用CloseHandle／DeleteObject／DestroyWindow。
- 把HWND截斷32位元、把registry bytes直接當可信結構或讓panic跨FFI。
- 修改 `ScreenSaverIsSecure`／密碼／policy key，或由提權 installer 猜測並修改別的帳號 HKCU；個人逾時與啟用只能依第 15.2～15.3 節由原使用者 helper 套用。
- 為通過效能門檻省略畫面，或把無法取得硬體／安全桌面環境的項目寫成PASS。
- 把授權不明字型、私鑰、憑證密碼或參考截圖放進交付安裝程式。
- 宣稱Inno Setup直接產生MSI、單純更名就完成系統整合，或編譯成功即代表產品驗收成功。

## 21. 參考資料與來源限制

既有 Win32 查核日期為2026-09-04；日本旅行來源查核日期為2026-09-07，播放器每頁／每螢幕 autoplay 規則於2026-09-08再次查核，螢幕保護逾時與系統參數於2026-09-09再次查核。連結用於 API 契約、第三方限制與既有設計來源，實作仍須以鎖定工具版本編譯及 Windows 實測。

| 來源 | 用途 |
| --- | --- |
| [Microsoft：Handling Screen Savers](https://learn.microsoft.com/en-us/windows/win32/lwef/screen-saver-library) | 系統整合、資源與傳統library背景；留意歷史範例適用範圍 |
| [Microsoft：GetTickCount64](https://learn.microsoft.com/en-us/windows/win32/api/sysinfoapi/nf-sysinfoapi-gettickcount64) | 毫秒tick與解析度 |
| [Microsoft：GetMessageW](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getmessagew) | 訊息迴圈三種返回值 |
| [Microsoft：SystemParametersInfoW](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-systemparametersinfow) | `SPI_SETSCREENSAVETIMEOUT`、`SPI_SETSCREENSAVEACTIVE` 與設定廣播 |
| [Microsoft：Group Policy Screensaver setting isn't working](https://learn.microsoft.com/en-us/troubleshoot/windows-client/group-policy/group-policy-screensaver-setting-not-work) | 缺少 `ScreenSaveTimeOut` 可能使螢幕保護程式不啟動；policy 覆蓋與逾時設定限制 |
| [Microsoft：SetParent](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setparent) | 跨程序parent與DPI差異 |
| [Microsoft：SetThreadDpiAwarenessContext](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setthreaddpiawarenesscontext) | 暫時thread DPI context |
| [Microsoft：CHOOSEFONTW](https://learn.microsoft.com/en-us/windows/win32/api/commdlg/ns-commdlg-choosefontw) | 點數、flags與限制 |
| [Microsoft：Distribute your app and the WebView2 Runtime](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/distribution) | Evergreen Runtime 探測、Windows 10 可用性與部署責任；不能假設所有 Win10 都已具備 |
| [windows-sys 0.61.2](https://docs.rs/windows-sys/0.61.2/windows_sys/) | Rust Win32 API版本入口 |
| [Rust Reference：Linkage](https://doc.rust-lang.org/reference/linkage.html#static-and-dynamic-c-runtimes) | 靜態CRT及成品檢查 |
| [Inno：Architecture identifiers](https://jrsoftware.org/ishelp/topic_archidentifiers.htm) | x64os與x64compatible區別 |
| [Inno：ArchitecturesAllowed](https://jrsoftware.org/ishelp/topic_setup_architecturesallowed.htm) | 安裝架構限制 |
| [Inno：ExecAsOriginalUser](https://jrsoftware.org/ishelp/topic_isxfunc_execasoriginaluser.htm) | 原使用者執行與uninstall限制 |
| [Inno：Run section](https://jrsoftware.org/ishelp/topic_runsection.htm) | runasoriginaluser及執行結果處理 |
| 使用者提供的私有圖片 | 已檢視；標準桌曆暨時鐘模式的構圖參考，不納入公開 repository 或交付素材 |
| [Classroom Timer](https://kisaraki.github.io/classroom-timer-tzk/?tool=countdown) | 原稿指定；本次未取得網頁內容，未聲稱當前畫面驗證 |
| [tw.live 日本旅行即時影像](https://tw.live/japan/) | 日本地區與 camera detail 的主要目錄；目前頁面標示資料來源為 YouTube，不是穩定 API 契約 |
| [tw.live 常見問題](https://tw.live/faq/) | 平台非影像擁有者、來源可能中斷／改址；嵌入、轉載或商用須確認原始來源授權 |
| [tw.live 隱私權政策](https://tw.live/privacy/) | 一般瀏覽可能記錄 IP、時間、瀏覽器與瀏覽／點選資料，README／Pages 必須揭露第三方連線 |
| [YouTube：Required Minimum Functionality](https://developers.google.com/youtube/terms/required-minimum-functionality) | player 最小尺寸、autoplay 可見性、Referer、單一 player、不得用 overlay／frame 遮蔽 |
| [YouTube：Developer Policies](https://developers.google.com/youtube/terms/developer-policies-guide) | autoplay 的資料分享、player attribution、播放完整性與隱私責任 |

## 22. 完成交付清單

完成開發後應交付下列成果；本規格編修階段不要求已產生它們：

1. 可維護的Rust原始碼、Cargo.lock、固定toolchain及`.cargo/config.toml`。
2. 指針鐘、月曆、七段數字、沙漏、倒數、GDI 旅行靜態畫面與共用 renderer；日本旅行另含本機 HTML／CSS 客艙 shell 及有界 source／player adapter。
3. 原生RC dialogs、manifest、icon、版本與名稱resource。
4. 純邏輯測試、隔離registry測試與有界Windows smoke script。
5. build／package腳本與Inno Setup安裝腳本。
6. `dist\tools-screensaver-tzk.scr`。
7. `dist\tools-screensaver-tzk-Setup.exe`。
8. `dist\SHA256SUMS.txt` 或等價本次成品hash紀錄。
9. `docs/acceptance-report.md`，逐項 AC／UT／MT、環境及真實結果。
10. `docs/visual-reference.md` 與實作截圖，說明參考範圍、色彩差異、旅行 player 邊界及 fixture 條件。
11. `docs/japan-travel-sources.md`，列來源、原始提供者、最後 HTTP／播放檢查、授權與可用性限制。
12. `docs/phase6-report.md`、`docs/phase7-report.md`、`docs/phase8-report.md`、`docs/phase9-report.md` 與各版 release notes。
13. README：安裝工具、build/test/package、`/s`／`/p`／`/c`、三種模式、WebView2／網路／隱私邊界、離線 fallback、字型fallback、registry、倒數閒置互動限制、原使用者setcurrent、解除安裝提示、已知未測項與簽章狀態。
14. MIT License 與素材來源說明；MIT 不涵蓋第三方影片，使用者附件只留在已忽略的本機開發參考目錄，不進公開 repository 或 installer。

交付說明應區分「原始碼／封裝完成」與「全部必要環境驗收通過」。任何不可重現、未執行或只經推測的結果都不能記為完成。
