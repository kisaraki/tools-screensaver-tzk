# Phase 5 封裝與交付報告

日期：2026-09-06<br>
軟體版本：0.1.1<br>
規格：`MyDateTimeScreensaver_Codex_Spec.md` v1.2（修訂版）  
必要平台：Windows 10 x64；Windows 11 依使用者指示延期

## 結果

Phase 5 的原始碼、建置與封裝已完成，產生可重建的 `.scr`、單一 Inno Setup EXE 和 SHA-256 清單。v0.1.1 將使用者可見模式名稱更新為「標準桌曆暨時鐘模式」與「離機作業番茄鐘模式」，並在設定畫面加入「KOMSMOS TOOLKIT 探真拓知酷」識別；內部 `TimeDate`／`Countdown` 值保持相容。開發候選未簽章。非互動 gate、PE／資源／manifest／靜態 CRT、版本一致性、helper 非安裝位置拒絕和受保護 HKCU 值不變均已實際通過。

需要管理員桌面的實際安裝測試曾啟動一次，但 Windows 回報 UAC「操作被使用者取消」。其後確認 System32、產品目錄、解除安裝登錄及 `SCRNSAVE.EXE` 都未被改動。因此乾淨安裝、task 已勾、同版覆蓋、檔案使用中、解除安裝、不同帳號 UAC 和直接提權情境仍是 `NOT TESTED`。這個狀態使本次成果只能稱為「有已知驗證限制的開發候選」，不能稱為 Windows 10 完整驗收。

## 實作

- `scripts/build.bat`：可由任意目錄執行；檢查 Rust 1.97.1、x64 MSVC target、rustfmt、Clippy、RC 與 MSVC x64 linker，依序執行 fmt、Clippy、tests、locked Release build，再以同目錄暫存檔替換 `dist/MyDateTimeScreensaver.scr`。失敗不會把舊檔標成新成果。
- `scripts/package.bat`：先呼叫 build；只接受已登錄的 Inno Setup 6.7.3；在 repository `target/package-stage-*` 編譯，驗證 Setup 與 `.scr` 版本後才替換正式 Setup；輸出兩檔大小、SHA-256 和簽章存在狀態。
- `installer/MyDateTimeScreensaver.iss`：固定 AppId `{E4D6978B-A2A2-4D9A-8FD8-8F0C3A4E94E1}`，限定 `x64os`／64-bit install mode／Windows 10 build 15063 以上，`.scr` 只安裝到 `{sys}`，uninstaller 放 `{autopf}\MyDateTimeScreensaver`。
- `src/install.rs`：單一 `--install-set-current` helper；先驗證自身完整路徑是 64 位元 System32 的預期檔名，再驗證 token 未提權，只寫目前 token 的 `HKCU\Control Panel\Desktop\SCRNSAVE.EXE`，讀回查核，最後以 5 秒 timeout 廣播 `WM_SETTINGCHANGE`。
- Setup 的 set-current task 預設不勾；只有勾選時以 `ExecAsOriginalUser` 等待 helper。code 4 或無法啟動時安裝本體可完成，但互動安裝會顯示尚未設定與 Windows 設定頁指引，靜默安裝寫入 log。
- Uninstaller 不清除 `SCRNSAVE.EXE`，不枚舉其他使用者 hive，並在互動解除安裝前提示使用者改選其他項目。產品 HKCU 偏好保留。
- `scripts/test-installation.ps1` 是有界實機 harness：拒絕覆蓋既有安裝，只追蹤自己啟動的程序，保存／恢復原本 `SCRNSAVE.EXE`，逐次記錄 UAC 安裝、task、覆蓋、檔案使用中、提權 helper 與解除安裝結果。

## 成品

| 檔案 | 版本 | Bytes | SHA-256 | Authenticode |
| --- | --- | ---: | --- | --- |
| `dist/MyDateTimeScreensaver.scr` | 0.1.1 | __SCR_BYTES__ | `__SCR_SHA256__` | NotSigned |
| `dist/MyDateTimeScreensaver-Setup.exe` | 0.1.1.0 | __SETUP_BYTES__ | `__SETUP_SHA256__` | NotSigned |

`dist/SHA256SUMS.txt` 由成功 package 產生。專案擁有者已指定 MIT License 並授權建立公開 GitHub repository、Release 與 Pages；v0.1.0 保留既有成品，這次模式改名與設定畫面品牌更新另以 v0.1.1 未簽章開發候選版發布。若日後取得憑證，須依序簽 `.scr`、重封 Setup、簽 Setup，再重建最終雜湊。

上表是加入新模式名稱、「KOMSMOS TOOLKIT 探真拓知酷」識別與公開 metadata 後重新執行 package／非互動 smoke 的最終發布成品。Phase 5 目錄中較早的 `build.txt`、`build-arbitrary-cwd.txt`、`package-arbitrary-cwd.txt` 與 `smoke-arbitrary-cwd.txt` 保留階段執行時的舊 hash，只用來證明當時的任意工作目錄與 gate 行為；下載驗證以本表、`verification.json` 與 Release 的 `SHA256SUMS.txt` 為準。

## 實際 gate

```powershell
scripts\build.bat
scripts\package.bat
pwsh -NoLogo -NoProfile -NonInteractive -File scripts\smoke-test.ps1
```

結果：fmt PASS；Clippy `-D warnings` PASS；35 個非互動測試 PASS（16 library、8 CLI、2 native、9 time/layout；另有 1 個 fixture 測試按設計 ignored）；locked Release build PASS；package PASS；smoke PASS。Smoke 驗證 x64 PE32+ Windows GUI、六類必要資源、新模式名稱與品牌字串、manifest、版本、Windows-only imports、無動態 VC/UCRT import、Release Debug 入口拒絕、錯誤參數，以及 helper code 4 前後四個系統螢幕保護值相同。

另從 repository 外的 `%USERPROFILE%\Downloads` 以絕對腳本路徑重跑 build、package 與 smoke，三者均 PASS；證據為 `build-arbitrary-cwd.txt`、`package-arbitrary-cwd.txt`、`smoke-arbitrary-cwd.txt`。這確認 `%~dp0`／`$MyInvocation.MyCommand.Path` 的根目錄解析沒有依賴目前工作目錄。

## 安裝矩陣

| 情境 | 狀態 | 實際結果或缺少條件 |
| --- | --- | --- |
| Setup 靜態規則／固定 AppId／x64 System32 | PASS | Inno 6.7.3 實際編譯；source 與成品版本檢查通過。這是封裝規則，不代替安裝。 |
| helper 嚴格單一旗標 | PASS | CLI tests 接受唯一精確旗標，拒絕大小寫、拼法及額外參數變體。 |
| helper 從非 System32 執行 | PASS | Release `.scr` code 4、無 UI，四個受保護值前後相同。 |
| 乾淨安裝、task 未勾 | NOT TESTED | UAC 被取消；安裝未開始。 |
| task 已勾／原使用者 HKCU | NOT TESTED | 需先完成 UAC 安裝。 |
| 同版覆蓋、檔案使用中、解除安裝 | NOT TESTED | 需先完成乾淨安裝；harness 已提供。 |
| 舊版升級 | NOT TESTED | 沒有經簽核的舊版 Setup fixture。 |
| 標準帳號使用另一管理員 UAC | NOT TESTED | 缺少獨立標準帳號與另一組管理員認證。 |
| Setup 從已提權程序直接啟動 | NOT TESTED | 缺可完成 UAC 的互動管理員桌面。 |
| Windows 設定頁列舉／系統 preview／真實閒置 | NOT TESTED | 尚未成功安裝；安全桌面需獨立人工操作。 |

Windows 11：`NOT TESTED（依使用者指示延期）`。遠端開發與 CI 只執行非互動 build／test／smoke，不執行 `-Interactive` 或 `scripts/test-installation.ps1`，因此不會顯示 UAC 或改動 System32。

完整 AC／UT／MT 狀態見 [逐項驗收報告](acceptance-report.md)，原始 JSON、log、hash 與工具環境放在 `docs/evidence/phase5/`。
