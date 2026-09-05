# 素材來源

- `app.ico`：本專案自行以矩形、刻度、指針及圓點繪製的黑底綠色時鐘；不是從參考圖或外部網站擷取。
- `generate-icon.ps1`：圖示的可重建幾何來源，使用 Windows 內建 PowerShell／System.Drawing，輸出 16、32、48、256 px 的 32-bit ICO。
- 私有視覺參考：使用者提供的圖片只供本機開發比對，`assets/references/` 已由 Git 忽略，不納入公開 repository、程式或安裝程式。

一般建置直接使用已提供的 `app.ico`，不必執行圖示產生器或安裝額外工具。專案原始碼以根目錄 [MIT License](../LICENSE) 發布。
