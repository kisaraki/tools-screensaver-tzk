# Phase 14：影片清單、新旅行場景與散步眨眼

日期：2026-09-08  
產品：tools-screensaver-tzk 0.10.0  
規格：v2.1

## 完成內容

- 自在飛行使用 `PLdsqwBj2O1Nw`，列車旅行使用 `PLBH60D9AGfu0`，列車駕駛前方使用 `PLB-Fmt68BNm4`，散步使用 `PLbYZr39owNGo`。
- 每次全螢幕啟動與設定時間到期時，YouTube IFrame Player API 重新讀取影片清單、shuffle 並隨機選片；不保存清單、不使用 API key、不解析 YouTube HTML。
- 日式旅館保持既有 tw.live 即時來源與有界 camera failover。
- 新增原創擬真列車駕駛室及第一人稱人眼 PNG，兩者同時供 GDI preview／fallback 與 HTML shell 使用。
- 散步視野縮至中央 60%×54%，四周為大面積黑色；換片使用 700 ms 上下眼瞼動畫，在 300 ms 閉合點載入新來源。
- Registry schema 7 新增 `TrainCab=3`、`Walking=4`，舊 schema 不誤解新值。

## 非互動驗證

| 項目 | 結果 |
| --- | --- |
| Rust／Cargo build gate | PASS：fmt、Clippy `-D warnings`、58 個預設測試、locked Release |
| 旅行來源 HTTP | PASS：tw.live catalog、8／8 旅館候選、4／4 playlist embed endpoint；[證據](evidence/phase14/source-health.json) |
| GDI fixture | PASS：52 張，含兩個新場景及旅行狀態 |
| Headless HTML fixture | PASS：10 張；五場景各有 1600×1000 與 900×1600，[幾何結果](evidence/phase14/html/geometry.json) |
| Smoke／封裝 | PASS：見 `evidence/phase14/smoke`、`package.txt` |

上述程序不建立正式 WebView2 player、不顯示設定畫面或 Setup、不安裝成品、不寫 System32，也不觸發 UAC。

## 驗證限制

`PlaybackHealthy`、實際影片清單內容、地區限制、禁止嵌入影片、眨眼動畫的實際動態觀感、多螢幕播放、長時間 GPU／記憶體、安裝／升級／解除安裝及 Windows 11 均為 `NOT TESTED`。HTTP 200 與靜態 HTML fixture 不代表影片已進入 `PLAYING`。
