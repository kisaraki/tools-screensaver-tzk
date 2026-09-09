# tools-screensaver-tzk v0.11.0

本版更新日本旅行模式的播放體驗與介面名稱。

- 顯示名稱更新為「和風庭園」、「御運轉士」、「地方散策」、「雪藍」與「琥珀」，內部 enum 與既有使用者設定保持相容。
- 所有 YouTube 影片由約 3:00 開始播放；直播、短片或 keyframe 位置仍由 YouTube 決定實際起點。
- 停用播放器控制列、預設字幕、註解、鍵盤與全螢幕按鈕，並讓 player 不接受滑鼠事件。YouTube 仍可能依平台規則短暫顯示必要的標題或品牌資訊。
- 地方散策在來源切換時以上下眼瞼完全閉合；順暢播放時每 20～30 秒輕眨，BUFFERING 或播放進度異常時改用較慢、較深且有節制的眨眼。
- 定時輪換前先解析下一來源、預選不同的播放清單影片並預熱縮圖/CDN 連線；切換時重用唯一可見 player，沒有背景播放或隱藏 player。

## 驗證

- 58 個預設非互動 Rust／CLI／layout 測試通過；9 個互動、長時間或環境測試維持 ignored。
- 格式、Clippy `-D warnings`、locked Release build 與非互動 PE／resources／registry smoke 通過。
- 19 個 WebView2 與 15 個產品版本 installer policy checks 通過；未建立精靈、執行安裝程式、寫入 registry 或觸發 UAC。
- 2026-09-09 非互動來源探測：8／8 和風庭園候選與 4／4 YouTube playlist endpoint 可達。HTTP 可達不等於實際 `PLAYING`。
- SCR 與 Setup 均未簽章。實際 WebView2 播放、字幕偏好、三分鐘 seek、網路節流與多螢幕流暢度仍需在可互動 Windows 10 環境驗收。
