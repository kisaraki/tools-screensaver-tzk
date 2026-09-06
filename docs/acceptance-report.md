# MyDateTimeScreensaver 驗收報告

軟體版本：0.1.1<br>
規格文件：v1.2（修訂版）  
執行日期：2026-09-06<br>
Source revision：以 Git tag `v0.1.1` 與 `docs/evidence/phase5/source-sha256.txt` 追溯<br>
環境：Windows 10 Education 22H2 x64，build 19045.6456；Intel Core i5-8259U，4 cores／8 logical processors，約 24 GiB RAM；Intel Iris Plus Graphics 655；雙 3840×2160、兩者 144 DPI／150%，左側螢幕為負 X  
工具：Rust／Cargo 1.97.1、windows-sys 0.61.2、MSVC x64 toolset 14.51.36231（link.exe 14.51.36256.0）、Windows SDK RC 10.0.26100.0、Inno Setup 6.7.3

狀態只使用 `PASS`、`FAIL`、`NOT TESTED`、`NOT APPLICABLE`。凡必要實機情境只完成一部分，整項列為 `NOT TESTED` 並說明已取得的局部證據。本報告目前沒有 `FAIL`，但仍有必要項目 `NOT TESTED`，因此不宣稱 Windows 10 完整驗收完成。

## AC01～AC15

| ID | 狀態 | 環境／實際操作 | 證據 | 失敗原因或缺少條件 |
| --- | --- | --- | --- | --- |
| AC01 | NOT TESTED | fmt、Clippy、tests、locked Release、PE／resources／imports 全部實跑通過 | `phase5/build.txt`、`smoke-report.json` | MT15 的無開發工具乾淨 Win10 尚無環境 |
| AC02 | PASS | Debug／Release 真實命令列與原生視窗測試 | Phase 4 `native-release.txt`、UT01～03 | — |
| AC03 | NOT TESTED | 真實跨程序 parent、三種 host DPI context、resize／退出／刷新已通過 | Phase 3 native、Phase 4 native Release | 缺實體混合 100%／150%／200% 桌面與 Windows 設定頁 |
| AC04 | NOT TESTED | 原生設定、preview、ChooseFont、提交／取消閉環已通過 | Phase 3 report／截圖 | MT01 安裝後 Windows 設定頁調度未測 |
| AC05 | PASS | Gregorian／鐘角／layout tests 與 24 張 GDI fixture | Phase 2 visuals、UT04～07、17～19 | — |
| AC06 | NOT TESTED | 倒數原生輸入與取消已通過 | Phase 3 native、UT08～09、MT07 | MT08 真實閒置／恢復登入未測 |
| AC07 | NOT TESTED | 七段、沙漏、進度、最後十秒、四次閃爍測試與 MT07 通過 | UT10～15、Phase 2 fixtures | MT09 實際睡眠／恢復／校時未測 |
| AC08 | NOT TESTED | 雙 4K、負 X、共用 generation 與合成 display-change 清理已通過 | UT25、MT03、Phase 4 native | 缺實體混合 DPI、拔除顯示器與登出 |
| AC09 | NOT TESTED | 四色、四字型、fallback 與極端點數有測試／fixture | UT18～22、visual-reference | 缺 200% 實體 dialog 與混合 DPI 實機 |
| AC10 | PASS | 專用測試 key 與 fake store 驗證型別、schema、取消、rollback、未知值 | UT20～24、Phase 3 report | — |
| AC11 | NOT TESTED | 指定鍵鼠、4px／500ms、同程序焦點與十一種訊息清理已有實測 | Phase 1／4 native | 外部 foreground 被 Windows policy 拒絕一次；實際登出未測 |
| AC12 | NOT TESTED | 標準桌曆暨時鐘模式與離機作業番茄鐘模式的 Release preview 各 30 分鐘、各 59 次 cache 循環穩定 | Phase 4 resource report | 正式全螢幕離機作業番茄鐘模式（`Countdown`）10 Hz 長測與實體 DPI 反覆變更未測 |
| AC13 | NOT TESTED | Setup 已編譯且版本一致 | `phase5/package.txt`、`SHA256SUMS.txt` | UAC 被取消，尚未實際安裝、列舉、移除 |
| AC14 | NOT TESTED | 非 System32 helper code 4 且四值不變 | `smoke-report.json` | 原使用者成功路徑、另一管理員 UAC、直接提權 installer 未測 |
| AC15 | PASS | README、原始碼、測試、`.scr`、Setup、hash、視覺與逐項報告均存在 | repository 交付清單 | 成品明確標示 NotSigned 與未測限制 |

## UT01～UT25

| ID | 狀態 | 證據 |
| --- | --- | --- |
| UT01～UT03 | PASS | `tests/cli_tests.rs`、`tests/native_modes.rs`；含真實 Release command line |
| UT04～UT07 | PASS | `tests/time_layout.rs` Gregorian 與連續鐘角 |
| UT08～UT09 | PASS | duration 欄位拒絕、邊界與 round trip |
| UT10～UT16 | PASS | deadline、ceil、ratio、最後十秒、四次暗半週期、segment、xorshift |
| UT17～UT19 | PASS | 五種 aspect、1.349／1.350、0×0～4K、96～288 DPI、安全位移 bounds |
| UT20～UT24 | PASS | config／registry fallback、schema、font、取消與注入失敗 rollback |
| UT25 | PASS | 共用 generation、idempotent shutdown、最後視窗 quit |

Phase 5 另新增兩項：內部 helper 只接受精確單一旗標；從非 System32 執行的真實 binary 必須無 UI 且 code 4。兩項均 PASS。

## MT01～MT15

| ID | 狀態 | 已取得的證據 | 缺少條件 |
| --- | --- | --- | --- |
| MT01 | NOT TESTED | Win10 build、`/s`／`/p`／`/c` 已測 | UAC 被取消；安裝、列舉、系統 preview、解除安裝未測 |
| MT02 | NOT TESTED | 1920×1080 fixture／96-DPI preview 長測 | 無實體單螢幕 100% 全螢幕環境 |
| MT03 | PASS | 雙 4K／150%、左側負 X、全覆蓋、共用 generation、內部焦點 |
| MT04 | NOT TESTED | 4K／150% 與直向 fixture | 無混合 100%／150%／200%、實體直向與 200% dialog |
| MT05 | NOT TESTED | 真實 host 保存後 2 秒刷新、取消不寫 | Windows 設定頁中的已安裝 `.scr` 尚未測 |
| MT06 | PASS | unaware／system／PMv2 host，四種 resize，parent 關閉退出 |
| MT07 | PASS | 1 秒、5 秒、30 分鐘、99:59:59 原生流程與時間邏輯 |
| MT08 | NOT TESTED | 手動 `/s` 已測但不採計本項 | 真實閒置、自動啟動、恢復登入開／關與安全桌面 |
| MT09 | NOT TESTED | 純 deadline／校時／跨午夜模型已測 | 實際睡眠／恢復、校時與跨午夜 |
| MT10 | NOT TESTED | 合成 display change、session end、close 已測 | 實際拔線、外部前景成功切換分支與登出 |
| MT11 | NOT TESTED | helper 身分檢查已實作 | 獨立標準帳號與另一管理員 UAC |
| MT12 | NOT TESTED | helper 非安裝路徑拒絕已測 | Setup 從已提權程序啟動及安裝後 elevated helper |
| MT13 | NOT TESTED | 固定 AppId／restartreplace／保留偏好規則已編譯 | 實際同版覆蓋、舊版升級、檔案使用中、解除安裝 |
| MT14 | NOT TESTED | 標準桌曆暨時鐘模式與離機作業番茄鐘模式的 Release preview 各 30 分鐘與 59 次循環通過 | 全螢幕 10 Hz 長測與實體 DPI 變更 |
| MT15 | NOT TESTED | fresh offline hash 可重現、無 VC/UCRT 動態 import | 無未裝工具／VC++ Redistributable 的乾淨 Win10 目標機 |

Windows 11：`NOT TESTED`，依使用者指示延期，沒有當成 Windows 10 的阻擋理由，也沒有宣稱已支援通過。

## Phase 5 成品

| 成品 | SHA-256 | 狀態 |
| --- | --- | --- |
| `dist/MyDateTimeScreensaver.scr` | `__SCR_SHA256__` | Build／smoke PASS；NotSigned |
| `dist/MyDateTimeScreensaver-Setup.exe` | `__SETUP_SHA256__` | Package／版本檢查 PASS；實際安裝 NOT TESTED；NotSigned |

Phase 5 的封裝原始碼與 v0.1.1 候選成品已交付，並依專案擁有者指示採 MIT License 公開發布。完成 Windows 10 完整驗收仍需在可互動的本機環境執行 `scripts/test-installation.ps1` 並補做本表列出的必要實機情境；遠端工作階段不觸發 UAC。程式碼簽章憑證尚未提供，成品維持 NotSigned。
