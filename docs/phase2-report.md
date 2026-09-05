# Phase 2 畫面與純時間邏輯驗證報告

- 依據：開發規格 v1.2 的 Phase 2；軟體版本維持 0.1.0。
- 日期：2026-09-05（Asia/Taipei）。
- 結論：**Phase 2 的 renderer、版面、時間模型、Debug 驗證入口與初步資源觀察已完成；目前 Windows 10 環境可執行的 gates 全部通過。** 這不是整份產品規格或發布候選的完成聲明；設定、正式倒數輸入、registry 與 ChooseFont 留待 Phase 3。
- 平台：Windows 10 Education 22H2 x64，build 19045.6456；Rust／Cargo 1.97.1、MSVC 2022、Windows SDK。
- 硬體：Intel Core i5-8259U（8 logical processors）、Intel Iris Plus Graphics 655；兩台 3840×2160、144 DPI（150%），左側矩形 `(-3840,0,0,2160)`、右側 `(0,0,3840,2160)`。
- Windows 11 依使用者指示延期，不阻擋目前 Windows 10 的 Phase 2 結果。未安裝新元件，也未修改 registry、系統時鐘、顯示配置或螢幕保護設定。

## 已完成的實作

| 範圍 | 實作結果 |
| --- | --- |
| 快照與協調器 | `FrameSnapshot` 是每個 generation 的不可變輸入；單一 coordinator 每次只取一次本機時間與 `GetTickCount64`，所有 surface 共用同一快照。倒數／閃爍為 100 ms 更新，時間日期及閃爍完成後為 1 s。時間變更與電源恢復會重新取樣，不逐 tick 補跑。 |
| 時間日期 | 完成圓角方形 60 刻度鐘面、連續時／分針角度、秒針、中心軸，以及 Gregorian 月曆、固定週一首欄、六個日期列、年月寬度 fallback 與今日實心圓。 |
| 倒數 | 以毫秒 deadline 計算剩餘值，顯示秒數向上取整；完成 `HH:MM:SS` 六位七段數字、LCD、沙漏上下砂與落砂、紅色剩餘比例線、最後十秒青藍框、歸零後 8 個 420 ms 半週期（4 次閃爍）及穩定歸零狀態。 |
| 版面 | `W/H >= 1.35` 採左右排列，其餘採上下排列；內容群組限制在 88%×82% 安全區並保留至少 2% 邊界。320×180、120×80、1×1 與 0×0 有明確退化；client pixels 不重複乘 DPI。 |
| 防烙印 | 正式 `/s` 每 60 秒以可重現 xorshift32 重算一次群組位移，最大為各軸 ±5% 且受完整 bounding rect 約束；跳過多個週期只重算一次。Preview、Debug 與 fixture 固定零位移。 |
| GDI | 每個 surface 快取 memory DC、32 bpp compatible bitmap、角色字型與最多 32 支 pen；完整場景在 memory DC 畫完後 `BitBlt`。resize／零尺寸／destroy 會先恢復選入物件再釋放；`BeginPaint`／`EndPaint`、DC save/restore 及 owned/borrowed object 皆由 RAII 管理。 |
| 失敗路徑 | 顯示前先預熱必要 renderer 資源，避免第一幀延後；bitmap 建立失敗可在有效 paint DC 直接繪製簡化場景。每個 surface buffer 以 checked arithmetic 限制為 256 MiB。 |
| 字型與色彩 | 已有電子錶、Consolas、新細明體繪圖路徑及 CJK／缺 glyph fallback；四色 palette 可渲染，亮綠與灰白 LCD 加深色輪廓。正式自訂字型選擇與保存屬 Phase 3。 |
| 開發入口 | Debug 接受 `--dev-render=time-date` 與 `--dev-render=countdown`，建立可縮放的一般視窗並沿用正常 cleanup；倒數固定從 5 分鐘開始。Release 對兩個旗標皆回傳 code 2。 |

compatible bitmap 由畫面 DC 建立，符合 [Microsoft `CreateCompatibleBitmap` 文件](https://learn.microsoft.com/en-us/windows/win32/api/wingdi/nf-wingdi-createcompatiblebitmap) 的色彩相容規則；零尺寸不呼叫該 API，避免其 1×1 單色 bitmap 行為。字型 fallback 使用 [Microsoft `GetGlyphIndicesW` 文件](https://learn.microsoft.com/en-us/windows/win32/api/wingdi/nf-wingdi-getglyphindicesw) 定義的缺字 `0xFFFF` 判定。

## 純邏輯與 GDI 測試

預設 `cargo test --locked -- --nocapture` 共通過 25 項非互動測試，另有 6 項必須明確啟動的 fixture／Windows 互動／長時間測試保持 ignored。Phase 2 規格指定的 UT04～UT19、UT25 對應如下：

| 規格 ID | 狀態 | 自動測試範圍 |
| --- | --- | --- |
| UT04～UT06 | PASS | 1900／2000／2024 閏年、週一首欄 offset、2023-12-31 位於第五日期列第七欄、固定六列。 |
| UT07 | PASS | 00:00、03:00、12:30 及秒數造成的連續時／分針角度。 |
| UT08～UT09 | PASS | 空白、非 ASCII、全形數字與 60 分拒絕；1、3599、359999 秒解析及拆解重組。 |
| UT10～UT12 | PASS | deadline 邊界、ceil、ratio clamp、10001／10000／1／0 ms 最後十秒、四次閃爍的 419／420／3359／3360 ms 邊界。 |
| UT13～UT14 | PASS | 本機時間調整只更新 TimeDate；倒數維持 monotonic deadline；跳過 tick／睡眠後直接依 deadline 重算並 clamp 為零。 |
| UT15～UT16 | PASS | 0～9 七段 mask 與 seeded xorshift32，包括零 seed fallback。 |
| UT17～UT19 | PASS | 16:9、16:10、4:3、直向、超寬、1.349／1.350 分界、多 DPI、小尺寸、零尺寸、±5% 位移及無空間軸退化。 |
| UT25 | PASS | coordinator 的同 generation 快照與既有冪等 shutdown／最後視窗才 quit 測試。 |

真正 GDI 小尺寸測試在 96／144／192／288 DPI 渲染 1×1、120×80、320×180；50 次 renderer、bitmap、font 與 pen cache 建立／釋放前後，本程序 GDI object 計數為 **1→1**。完整輸出見 [tests.txt](evidence/phase2/tests.txt)。

## 視覺比較

[Phase 2 視覺檢查](phase2-visuals.md) 保存 25 張由本程式實際 GDI renderer 產生的無損 PNG，涵蓋兩種模式、四色、三種目前可直接驅動的字型路徑、800×369、1920×1080、3840×2160、1080×1920、320×180、120×80，以及最後十秒／暗半週期／完成狀態。固定 TimeDate fixture 為 2023-12-31 12:15:40，未修改系統時鐘；所有影像的安全矩形外均驗證為純黑。

與附件 `iphonetips-3.jpg` 比較後，實作保留黑底、左鐘右月曆、圓角方形刻度軌跡、今日實心圓與六列月曆，且不繪製來源圖的裝置狀態圖示。預設主色依規格採亮綠，因此不宣稱與附件逐像素相同。

2026-09-04 另查閱目前 [Classroom Timer 原始碼與 README](https://github.com/kisaraki/classroom-timer-tzk)：目前專案仍描述倒數、水平紅色進度、色彩／字型選項及七段顯示。這次本機 fixture 依鎖定的 v1.2 數值與原生 GDI 限制實作；沒有複製網站程式碼，也未把 repository 說明當成即時頁面截圖或逐像素基準。

## Windows 10 實際程序驗證

| 情境 | 結果 | 證據 |
| --- | --- | --- |
| Debug 指定入口 | 兩個規格指令皆 code 0；實際 client 938×544、144 DPI，正常關閉且無殘留視窗。 | [developer-entrypoints.json](evidence/phase2/developer-entrypoints.json) |
| Release 限制 | Release 對兩個 `--dev-render` 旗標皆 code 2。 | [developer-entrypoints.json](evidence/phase2/developer-entrypoints.json) |
| Release `/s` | 真實雙 4K surface 精確符合含負 X 的 monitor rect；中心保持黑色，鐘面取樣區存在亮綠像素。11 種退出訊息各清除兩個 surface。 | [native-release-tests.txt](evidence/phase2/native-release-tests.txt) |
| Release `/p` | 真正跨程序 child，在 Unaware／System-aware／PMv2 host 下 DPI 對應正確；480×270、0×0、1×1、320×180 resize 與 parent 消失 cleanup 通過。 | [native-release-tests.txt](evidence/phase2/native-release-tests.txt) |
| Release `/c` 與參數 | owner、取消／確定、owner 消失及真實非法命令列通過；對話框仍是明示的 Phase 2 暫時入口。 | [native-release-tests.txt](evidence/phase2/native-release-tests.txt) |
| 前景轉移 | 同程序視窗取得前景後，兩個 surface 皆維持存在，PASS；外部程序取得前景仍為 **NOT TESTED**，因 Windows policy 拒絕該次 `SetForegroundWindow`。 | [native-release-tests.txt](evidence/phase2/native-release-tests.txt) |

Release 整合 harness 顯示 `5 passed`，其中前景轉移測試只完成同程序分支，外部程序分支採上述條件式未測路徑。因此可計為 **4 個完整整合案例通過、1 個部分通過且含 NOT TESTED 分支**，不能解讀為五個情境都已完整驗證。

## 初步 10 分鐘資源觀察

依 Phase 2 要求，同時啟動 TimeDate 與 Countdown Debug 一般視窗，從 0 秒至 600 秒每分鐘取樣。以下以第 60 秒預熱點至第 600 秒計算；CPU 公式為 `100×程序 CPU 秒增量/(牆鐘秒×8 logical processors)`。

| 模式 | 全機 CPU 平均 | Working set | Private bytes | GDI objects | USER objects |
| --- | ---: | ---: | ---: | ---: | ---: |
| TimeDate | 0.0723% | 10,817,536→10,821,632（+4,096） | 1,851,392→1,818,624（−32,768） | 26→26；範圍 26～27 | 14→14 |
| Countdown | 0.0275% | 10,813,440→10,821,632（+8,192） | 1,859,584→1,826,816（−32,768） | 29→30；範圍 29～30 | 14→14 |

兩個程序的 private bytes 沒有持續累積，GDI／USER object 亦未呈現持續成長。這是 **Debug、一般視窗、10 分鐘的初步觀察**；不替代規格 Phase 4 要求的單 1080p／100%／Release 基準與兩模式各 30 分鐘矩陣。原始資料：[resource-observation.csv](evidence/phase2/resource-observation.csv)、[resource-observation.txt](evidence/phase2/resource-observation.txt)。

觀察開始前記錄的被測 Debug SHA-256 為 `BB5E8A8DC850293B082C4B297B713EBFDB3A1DF0C81F5C87F8FFFC9544B848DE`，見 [observed-debug-sha256.txt](evidence/phase2/observed-debug-sha256.txt)。觀察結束後只修正測試顯示名稱與原始碼註解並重跑 gates；重新連結後的目前 Debug hash 另記於 `verification.json`，不回寫冒充當時量測檔案。

## Gates 與成品

| 操作 | 結果 | 證據 |
| --- | --- | --- |
| `cargo fmt --check` | PASS | [fmt.txt](evidence/phase2/fmt.txt) |
| `cargo check --locked --all-targets` | PASS | [check.txt](evidence/phase2/check.txt) |
| `cargo clippy --locked --all-targets -- -D warnings` | PASS，0 warning | [clippy.txt](evidence/phase2/clippy.txt) |
| `cargo test --locked -- --nocapture` | PASS，25 項非互動測試 | [tests.txt](evidence/phase2/tests.txt) |
| 25 張真正 GDI fixture | PASS，全部有前景像素，安全矩形外全黑 | [fixtures.txt](evidence/phase2/fixtures.txt) |
| Release 真實程序整合 | 4 個完整案例 PASS；1 個部分 PASS 且含 NOT TESTED 分支 | [native-release-tests.txt](evidence/phase2/native-release-tests.txt) |
| 10 分鐘初步資源觀察 | PASS，1 個長時間測試、600.44 秒 | [resource-observation.txt](evidence/phase2/resource-observation.txt) |
| `cargo build --release --locked --offline` | PASS | [release-build.txt](evidence/phase2/release-build.txt) |

- Release 執行檔：`target/x86_64-pc-windows-msvc/release/my_datetime_screensaver.exe`
- 大小：555,520 bytes。
- SHA-256：`9729DC2F8131611CE6F79D83E7B52BAE027DE48F365EB24F4D84B6A56A1A22D6`。
- FileVersion：0.1.0；FileDescription：`日期時間螢幕保護程式（Phase 2 畫面與時間邏輯）`。
- DLL imports 只有 Windows 系統 DLL；靜態 CRT target 設定保留。詳見 [dependencies.txt](evidence/phase2/dependencies.txt) 與 [verification.json](evidence/phase2/verification.json)。
- 此歷史階段執行時 repository 尚未建立第一個 Git commit，因此當時的 source revision 記為 `UNCOMMITTED WORKTREE (no HEAD)`；逐檔指紋見 [source-sha256.txt](evidence/phase2/source-sha256.txt)。

## 尚未驗證與階段界線

| 項目 | 狀態／原因 |
| --- | --- |
| Windows 11 | 使用者指定延期；保留相容性目標，未宣稱通過。 |
| 外部前景退出 | 同程序前景不退出已實測通過；外部程序切換仍受 Windows 前景政策阻擋，列 NOT TESTED。 |
| 單 1920×1080／100%、混合 DPI 實體顯示器、直向硬體、負 Y | 版面純測試及 GDI fixture 已覆蓋相應尺寸／DPI；本次實機只有雙 4K／150% 且左側負 X。 |
| 實際拔插顯示器、真實 DPI 變更、睡眠恢復、登出 | handler／純時間跳躍已測；未變更使用者硬體或 session 來做實機測試。 |
| Windows 系統閒置／安全桌面倒數流程 | 正式倒數輸入、模式偏好與 `.scr` 尚未存在，屬 Phase 3 及後續封裝驗收。 |
| 四種正式字型設定 | Renderer 已驗證七段、Consolas、新細明體與 fallback；ChooseFont、自訂字型、點數保存及 registry 屬 Phase 3。 |
| 30 分鐘 Release 效能／資源矩陣 | Phase 2 只要求至少 10 分鐘初步觀察；完整矩陣保留 Phase 4。 |
| `.scr`、Setup、簽章與乾淨機驗證 | 屬 Phase 4～5；目前成品仍是開發階段 `.exe`。 |

下一階段是 Phase 3：設定、字型與倒數輸入。本次停在 Phase 2。
