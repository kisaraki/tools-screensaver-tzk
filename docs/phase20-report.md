# Phase 20：旅行播放、游標與安裝完成設定

產品版本：0.13.0

規格版本：v2.7

執行日期：2026-09-09
平台：Windows 10 Education 22H2 x64（build 19045.6456）

## 完成內容

- 全螢幕先將游標停到主要螢幕左上外角的非 player 區域，再保存輸入 baseline 並隱藏；既有 `WM_SETCURSOR` 與清理路徑繼續負責維持隱藏及恢復游標形狀。
- 御運轉士窗孔改為素材實際開口的 46.4%×35.2%。16:9 player 置中放大，左右填滿，超出的少量上下影像只在窗孔容器內裁切，不壓住框景。
- 地方散策保留 0.4% 非 player 外緣；眼瞼改為寬 116%、高 62% 的橢圓曲線並套用 9～18 px 模糊。來源切換使用 420 ms 閉眼與 520 ms 睜眼兩段動畫。
- 每次初始載入、隨機選片、已預備候選與片尾重播，均以 181～539 秒的獨立隨機值作為起點。
- 保留 `cc_load_policy=0`，並在 player ready、load、playing 與 `onApiChange` 時要求關閉字幕 track、卸載 captions module。這不會移除影片畫面本身燒錄的文字。
- 產品 UI 不再出現使用者指定移除的舊詞，改用場景名稱、「影片清單」或「清單」。
- Setup 完成頁新增預設勾選的開啟設定選項，以原使用者身分執行已安裝的 `.scr /c`；使用者可取消，靜默安裝固定略過。

## 非互動驗證

| 項目 | 結果 | 證據 |
| --- | --- | --- |
| fmt／Clippy／locked build | PASS | [package.txt](evidence/phase20/package.txt) |
| 預設測試 | PASS | 60 passed：lib 40、CLI 8、native noninteractive 2、layout 10；9 ignored |
| JavaScript syntax | PASS | Node `--check`；只檢查抽出的本機 shell script，未建立 player |
| HTML 幾何 | PASS | [geometry.json](evidence/phase20/html/geometry.json) 與 10 張離線 headless Edge fixture；網路 host 被阻擋，未載入影片 |
| 來源 HTTP | PASS | 8／8 和風庭園候選、4／4 指定 YouTube embed endpoint 可達；[source-health.json](evidence/phase20/source-health.json) |
| Smoke | PASS | [smoke-report.json](evidence/phase20/smoke/smoke-report.json)；版本、PE、resources、manifest、imports、靜態 CRT 與 registry 不變 |
| Installer policy | PASS | 19 個 WebView2 與 15 個產品版本 checks；無 wizard、prompt、程序執行、registry write 或 UAC |
| Package | PASS | Inno Setup 6.7.3；SCR 與 Setup 版本一致，兩者 NotSigned |

## 成品

| 檔案 | Bytes | SHA-256 |
| --- | ---: | --- |
| `tools-screensaver-tzk.scr` | 9,555,456 | `6bbf6ae650aadf59a389476129f3d06ffb0cb20f3507f6842484c0fd8a12e098` |
| `tools-screensaver-tzk-Setup.exe` | 12,608,062 | `ff4689d14f8e421b3613af9cef97e79ef42c5ec801b0b1f810e50129f9c9bcc5` |

## 尚未驗證

依遠端開發限制，本階段沒有執行 Setup、開啟設定面板、啟動 `/s`、建立 WebView2 player、移動實際桌面游標或觸發 UAC。御運轉士實際影片裁切、地方散策動畫動態、使用者字幕偏好、來源影片 seek、完成頁勾選流程及靜默安裝須在可互動 Windows 10 補驗。Windows 11 依使用者指示延期。
