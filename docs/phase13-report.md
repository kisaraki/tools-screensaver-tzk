# Phase 13：日式旅館與桌曆時鐘色彩

日期：2026-09-08<br>
產品：tools-screensaver-tzk v0.9.0 開發候選版<br>
規格：v2.0；最終原始碼與成品由 Git tag `v0.9.0` 鎖定

## 完成內容

日本旅行模式新增 `JapaneseInn=2`「日式旅館」。內附的 1586×992 PNG 使用 Codex 內建 imagegen 工具生成，使用者附件只作障子、榻榻米與自然木構的風格參考，沒有複製附件像素、家具配置、文字或可識別旅館。畫面中央保留完整 16:9 player 窗孔；本機 HTML、GDI 靜態預覽與 fallback 使用同一張圖。

桌曆時鐘新增暗淺藍 `#6597B2`、琥珀色 `#FFBF00` 與「自動切換（2 分鐘）」。自動模式以所有螢幕共用的 `FrameSnapshot.tick` 為時基，每 120,000 ms 依序循環六種固定色，避免各螢幕自行取時造成不同步。番茄鐘與旅行模式在保存自動設定時保留亮綠主色。

Registry schema 升為 6，新增 `TravelStyle=2` 與 `ColorPreset=4/5/6`。舊 schema version 4／5 中偶然出現新版值時逐欄回退；version 6 可完整 round-trip，version 7 以上仍禁止降版寫入。

## 非互動驗證

| 項目 | 結果 | 證據 |
| --- | --- | --- |
| fmt／Clippy／locked tests／Release／封裝 | PASS | [package.txt](evidence/phase13/package.txt)；Rust 1.97.1、MSVC 14.51、Windows SDK 10.0.26100.0、Inno Setup 6.7.3 |
| 程式測試 | PASS | 57 passed：lib 37、CLI 8、native noninteractive 2、layout 10；9 ignored |
| Installer policy | PASS | 19 checks；未建立精靈、未執行 Bootstrapper、未寫 registry |
| WIC 圖像 | PASS | 三張 1586×992 PNG 解碼為 32-bpp、快取重用、MTA 生命週期保持、離屏繪製 |
| GDI fixtures | PASS | 50 張；含 7 個 palette 選項、第三場景、小型／直向／4K 與狀態文字；[清單](evidence/phase13/fixtures.txt) |
| HTML 幾何 | PASS | 三場景 × 1600×1000／900×1600 共 6 張；封鎖所有 host，player 16:9、位於窗孔內，caption 位於圖像下方；[結果](evidence/phase13/html/geometry.json) |
| SCR smoke | PASS | v0.9.0、x64 GUI、resources、三場景及七色標籤、manifest、imports、靜態 CRT、無 UI 錯誤參數、helper 拒絕及受保護 registry 前後一致；[JSON](evidence/phase13/smoke/smoke-report.json) |

本輪沒有開啟設定對話框、全螢幕、WebView2 player、Setup 或 UAC。HTML fixture 已移除 player script 並封鎖網路；畫面中的測試色塊只驗證 player 區域。實際旅館場景串流、多螢幕同步播放、設定畫面在 100%／150%／200% DPI 的操作、長時間自動換色與安裝／解除安裝仍為 `NOT TESTED`。

## 成品

| 成品 | Bytes | SHA-256 | 簽章 |
| --- | ---: | --- | --- |
| `tools-screensaver-tzk.scr` | 7,099,904 | `09f7e63fbcb8b46b58016b8009e023f7b5d26de8074c3b02d57138a6aaaca8aa` | NotSigned |
| `tools-screensaver-tzk-Setup.exe` | 10,208,748 | `fe83b4c0135247107111c9aab8bb3c18afe1082ad2dbe7be49f0104dfe8b1176` | NotSigned |

Pages 的 `downloads/v0.9.0/` 保存同一批 SCR、Setup 與 `SHA256SUMS.txt`，供匿名直接下載。圖片生成提示、尺寸與 hash 見 [旅行素材紀錄](travel-artwork.md)。
