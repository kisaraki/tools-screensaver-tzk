# Phase 15：列車駕駛室修飾與散步攝影暗角

日期：2026-09-09<br>
產品：tools-screensaver-tzk 0.10.1<br>
規格：v2.2

## 完成內容

- 列車駕駛前方 player 改為中央 42%×37.75% 的 16:9 區域，消除 player 自身造成的兩側大面積純黑。
- 兩側改由原創擬真的金屬設備櫃、空白螢幕、通風板、接縫與控制台填滿。
- 散步模式移除眼球、皮膚、血管及眼鏡語彙，改為全畫面強烈攝影暗角。
- 散步 player 鋪滿 16:9 場景，暗角中央約 65% 保持主要可視區，邊緣漸變至黑色。
- 來源切換時的 700 ms 眨眼與 300 ms 閉合載入點保留。

## 驗證

非互動 HTML fixture 驗證列車駕駛 player 為 16:9 且寬度不超過場景 43%，散步 player 覆蓋完整場景且維持 16:9；10 張 HTML 與 52 張 GDI fixture 均已匯出。`build.bat` 的 58 個預設測試、封裝後 smoke、19 個 installer policy checks 及 Inno Setup 6.7.3 package 均為 `PASS`。驗證不開啟正式 WebView2 player、設定畫面或 Setup，不安裝成品，也不觸發 UAC。

| 成品 | Bytes | SHA-256 |
| --- | ---: | --- |
| `tools-screensaver-tzk.scr` | 9,542,656 | `dedbededfb2bf064d2b1bf31a00df2e4dd62bee59aac872e3ade7c7624f5be3e` |
| `tools-screensaver-tzk-Setup.exe` | 12,603,059 | `633967a0f7138f4ce233b999c729fdbd39293e45d1be5ebc895481d87b2c0ad0` |

實際影片的暗角觀感、眨眼動畫、多螢幕與長時間 GPU／記憶體仍為 `NOT TESTED`。
