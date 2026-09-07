# tools-screensaver-tzk v0.6.0

Setup 新增 Microsoft Edge WebView2 Runtime 檢查與安裝階段，讓日本旅行模式所需元件可隨安裝流程準備。

- 已有電腦層級 Runtime：顯示版本並略過補裝。
- 缺少 Runtime：預設使用內附的 Microsoft 官方 Evergreen Bootstrapper，連網下載並靜默安裝；完成後再次確認。
- 只用日期時鐘或番茄鐘：可取消 WebView2 選項。安裝失敗會顯示錯誤，可重試或返回取消；需重啟時提示重新啟動後再執行 Setup。
- 解除安裝本程式保留共用 Runtime；獨立 `.scr` 不會自行安裝元件。

## 公開下載

- [tools-screensaver-tzk-Setup.exe](https://kisaraki.github.io/tools-screensaver-tzk/downloads/v0.6.0/tools-screensaver-tzk-Setup.exe)
- [tools-screensaver-tzk.scr](https://kisaraki.github.io/tools-screensaver-tzk/downloads/v0.6.0/tools-screensaver-tzk.scr)
- [SHA256SUMS.txt](https://kisaraki.github.io/tools-screensaver-tzk/downloads/v0.6.0/SHA256SUMS.txt)

上述 GitHub Pages 連結不需要登入或驗證 GitHub 身分。

| 檔案 | Bytes | SHA-256 |
| --- | ---: | --- |
| `tools-screensaver-tzk.scr` | 808,448 | `3f4e1f1ea7e4ef86a8438467483814989356e075d6605f71f541beafdfaf9cf8` |
| `tools-screensaver-tzk-Setup.exe` | 3,980,373 | `f5c2cc3843aa9d35153b4b20ab496d9978e164a944b30813afe0376bf73a6c97` |

Windows 10 x64 的 46 個預設測試、19 個安裝判斷測試、Release 建置、非互動 smoke 與 Inno Setup 封裝通過。官方 Bootstrapper 的版本、雜湊及 Microsoft 簽章已驗證；完整 Runtime 仍需在安裝時連網下載。

此版本仍為未簽章開發候選版。遠端沒有執行正式 Setup、Runtime installer 或 UAC；實際安裝／重試／重啟、旅行 player 與 Windows 11 仍未完成驗證。[Phase 10 報告](https://github.com/kisaraki/tools-screensaver-tzk/blob/v0.6.0/docs/phase10-report.md) · [Runtime 部署與來源](https://github.com/kisaraki/tools-screensaver-tzk/blob/v0.6.0/docs/webview2-setup.md)
