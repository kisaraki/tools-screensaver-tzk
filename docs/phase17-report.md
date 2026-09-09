# Phase 17：播放體驗與顯示名稱

執行日期：2026-09-09  
產品：tools-screensaver-tzk 0.11.0  
規格：v2.4

## 實作結果

設定資源、GDI fallback、HTML aria、來源標籤、README 與 Pages 已統一使用「和風庭園」、「御運轉士」、「地方散策」、「雪藍」與「琥珀」。`JapaneseInn=2`、`TrainCab=3`、`Walking=4`、`MutedLightBlue=4` 與 `Amber=5` 的內部值不變，因此不需要提升 registry schema。

YouTube IFrame player 的 URL 與 `playerVars` 改用 `controls=0`、`cc_load_policy=0`、`iv_load_policy=3`、`disablekb=1`、`fs=0`，iframe 也不接收滑鼠事件。初始載入、輪換、影片清單隨機選片與片尾重播均指定 180 秒起點。YouTube 已停用 `showinfo` 與 `modestbranding`，所以本版不宣稱能移除平台強制顯示的短暫標題或品牌資訊，也不用覆蓋層遮住它們。

地方散策具有三種本機動畫：來源切換使用 780 ms 完整上下閉眼並在約 380 ms 閉合點換片；健康播放每 20～30 秒隨機作 380 ms 輕眨；播放器 BUFFERING 或每 5 秒進度檢查發現異常時，使用 560 ms 較深眨眼，且至少間隔 8 秒。原有強烈暗角與中央約 65% 視域保持不變。

來源輪換仍在最後一分鐘由原生 worker 預抓。成功的 prefetch completion 會把候選送入 WebView：和風庭園先取得下一個 YouTube video ID；影片清單場景從本次啟動讀取的清單預選不同 ID。shell 預熱候選縮圖/CDN 連線，切換時直接以同一個可見 player 載入；沒有建立第二個 player、背景播放影音或持久快取。沒有有效預選時回退到 `loadPlaylist`。

## 非互動驗證

| 項目 | 結果 |
| --- | --- |
| 內嵌 JavaScript `node --check` | PASS |
| `scripts\build.bat` | PASS；fmt、Clippy、58 tests、Release build |
| `scripts\smoke-test.ps1` | PASS；v0.11.0、x64 PE32+ GUI、靜態 CRT、新 labels、manifest、imports、無 UI 錯誤參數與 registry 不變 |
| WebView2 installer policy | PASS；19 checks，未執行 Bootstrapper |
| 產品版本 installer policy | PASS；15 checks，未顯示提示或執行 uninstaller |
| `scripts\package.bat` | PASS；Inno Setup 6.7.3，SCR／Setup 版本一致 |
| 來源健康探測 | PASS；8／8 camera、4／4 playlist endpoint 可達 |

證據：[smoke report](evidence/phase17/smoke/smoke-report.json) · [source health](evidence/phase17/source-health.json)

## 成品

| 成品 | Bytes | SHA-256 | 簽章 |
| --- | ---: | --- | --- |
| `tools-screensaver-tzk.scr` | 9,548,288 | `e69d3834cc1f16e1dd0bb4f87520753e9dacc0d31895d331c79df4d031acf56e` | NotSigned |
| `tools-screensaver-tzk-Setup.exe` | 12,605,431 | `198a12beebc004e1f6a77c926b2288afbb26ec2b2d0f4777a35d9cb9b26080d3` | NotSigned |

## 尚待人工驗收

遠端工作階段沒有啟動 `/s`、WebView2 player 或 Setup，也沒有觸發 UAC。實際五種場景的 `PLAYING`、使用者字幕偏好、三分鐘 seek、地方散策動畫觀感、限速／斷線恢復、多螢幕與 30 分鐘資源觀察仍為 `NOT TESTED`。
