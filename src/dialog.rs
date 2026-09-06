use std::{cell::Cell, ptr};

use windows_sys::Win32::Foundation::{HINSTANCE, HWND, LPARAM, RECT, SYSTEMTIME, WPARAM};
use windows_sys::Win32::System::SystemInformation::{GetLocalTime, GetTickCount64};
use windows_sys::Win32::UI::Controls::Dialogs::{
    ChooseFontW, CommDlgExtendedError, CF_FORCEFONTEXIST, CF_INITTOLOGFONTSTRUCT, CF_LIMITSIZE,
    CF_SCREENFONTS, CHOOSEFONTW,
};
use windows_sys::Win32::UI::Controls::{
    CheckRadioButton, DRAWITEMSTRUCT, EM_SETLIMITTEXT, EM_SETSEL,
};
use windows_sys::Win32::UI::HiDpi::GetDpiForWindow;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::SetFocus;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

use crate::config::{self, ColorPreset, ConfigDraft};
use crate::error::{last_error, AppError};
use crate::font::FontSpec;
use crate::model::{
    parse_duration, Countdown, CountdownFrame, DisplayMode, FontMode, FrameSnapshot, InputError,
    LocalTime,
};
use crate::native::{set_pointer, WindowIdentity};
use crate::registry::{load_registry, RegistryStore};
use crate::render::{Renderer, Style};
use crate::{monitor, resource_ids};

// DWLP_USER follows the pointer-sized message result and dialog procedure slots.
const DIALOG_USER: i32 = (2 * size_of::<isize>()) as i32;
#[cfg(debug_assertions)]
const DIALOG_MESSAGE_RESULT: i32 = 0;
const CONFIG_TIMER: usize = 1;
const OWNER_TIMER: usize = 1;
#[cfg(debug_assertions)]
const TEST_SET_COUNTDOWN_FIELDS: u32 = WM_APP + 30;
#[cfg(debug_assertions)]
const TEST_GET_COUNTDOWN_FIELDS: u32 = WM_APP + 31;
#[cfg(debug_assertions)]
const TEST_GET_CONFIG_DRAFT: u32 = WM_APP + 33;

struct ConfigDialogState {
    owner: Option<WindowIdentity>,
    center_on: HWND,
    timer: Cell<usize>,
    error: Cell<Option<AppError>>,
    draft: Cell<ConfigDraft>,
    renderer: std::cell::RefCell<Option<Renderer>>,
    generation: Cell<u64>,
}

struct CountdownDialogState {
    owner: Option<WindowIdentity>,
    center_on: HWND,
    timer: Cell<usize>,
    error: Cell<Option<AppError>>,
    initial_seconds: u32,
    accepted_seconds: Cell<Option<u32>>,
}

pub(crate) fn configure(instance: HINSTANCE, owner: Option<usize>) -> Result<(), AppError> {
    let owner = owner.and_then(|value| WindowIdentity::capture(value as HWND));
    // SAFETY: This read-only snapshot is used only to select a monitor.
    let foreground = unsafe { GetForegroundWindow() };
    let state = Box::new(ConfigDialogState {
        owner,
        center_on: owner.map_or(foreground, |identity| identity.hwnd),
        timer: Cell::new(0),
        error: Cell::new(None),
        draft: Cell::new(load_registry().into()),
        renderer: std::cell::RefCell::new(Some(Renderer::default())),
        generation: Cell::new(0),
    });
    let owner_hwnd = owner.map_or(ptr::null_mut(), |identity| identity.hwnd);
    // SAFETY: The resource belongs to the executable. The caller owns state for
    // the modal call; DWLP_USER only borrows it, including during teardown.
    let result = unsafe {
        DialogBoxParamW(
            instance,
            resource_ids::IDD_CONFIG as usize as *const u16,
            owner_hwnd,
            Some(config_proc),
            &*state as *const ConfigDialogState as isize,
        )
    };
    if result == -1 {
        return Err(last_error("DialogBoxParamW(config)"));
    }
    match state.error.get() {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

pub(crate) fn countdown(
    instance: HINSTANCE,
    owner: Option<usize>,
    initial_seconds: u32,
) -> Result<Option<u32>, AppError> {
    let owner = owner.and_then(|value| WindowIdentity::capture(value as HWND));
    // SAFETY: This read-only snapshot is used only to select a monitor.
    let foreground = unsafe { GetForegroundWindow() };
    let state = Box::new(CountdownDialogState {
        owner,
        center_on: owner.map_or(foreground, |identity| identity.hwnd),
        timer: Cell::new(0),
        error: Cell::new(None),
        initial_seconds: initial_seconds.clamp(1, 359999),
        accepted_seconds: Cell::new(None),
    });
    let owner_hwnd = owner.map_or(ptr::null_mut(), |identity| identity.hwnd);
    // SAFETY: Same modal borrowed-state lifetime as the configuration dialog.
    let result = unsafe {
        DialogBoxParamW(
            instance,
            resource_ids::IDD_COUNTDOWN_INPUT as usize as *const u16,
            owner_hwnd,
            Some(countdown_proc),
            &*state as *const CountdownDialogState as isize,
        )
    };
    if result == -1 {
        return Err(last_error("DialogBoxParamW(countdown)"));
    }
    match state.error.get() {
        Some(error) => Err(error),
        None => Ok(state.accepted_seconds.get()),
    }
}

fn center(hwnd: HWND, target: HWND) -> Result<(), AppError> {
    let target = if target.is_null() { hwnd } else { target };
    let info = monitor::info_for_window(target)?;
    let mut rect = RECT::default();
    // SAFETY: The dialog is live and rect is writable.
    if unsafe { GetWindowRect(hwnd, &mut rect) } == 0 {
        return Err(last_error("GetWindowRect(dialog)"));
    }
    let width = i64::from(rect.right) - i64::from(rect.left);
    let height = i64::from(rect.bottom) - i64::from(rect.top);
    let work = info.rcWork;
    let x =
        i64::from(work.left) + (i64::from(work.right) - i64::from(work.left) - width).max(0) / 2;
    let y =
        i64::from(work.top) + (i64::from(work.bottom) - i64::from(work.top) - height).max(0) / 2;
    let (Ok(x), Ok(y)) = (i32::try_from(x), i32::try_from(y)) else {
        return Err(AppError::OperationFailed("dialog position"));
    };
    // SAFETY: Position without changing z-order, size, or activation.
    if unsafe {
        SetWindowPos(
            hwnd,
            ptr::null_mut(),
            x,
            y,
            0,
            0,
            SWP_NOACTIVATE | SWP_NOSIZE | SWP_NOZORDER,
        )
    } == 0
    {
        return Err(last_error("SetWindowPos(dialog)"));
    }
    Ok(())
}

fn local_frame(generation: u64) -> Result<FrameSnapshot, AppError> {
    let mut local = SYSTEMTIME::default();
    // SAFETY: Both APIs write/read process-independent clock state.
    let tick = unsafe {
        GetLocalTime(&mut local);
        GetTickCount64()
    };
    let local = LocalTime {
        year: local.wYear,
        month: local.wMonth,
        day: local.wDay,
        hour: local.wHour,
        minute: local.wMinute,
        second: local.wSecond,
    };
    if !local.valid() {
        return Err(AppError::OperationFailed("GetLocalTime(dialog preview)"));
    }
    let countdown = Countdown::new(300, tick)
        .map_err(|_| AppError::OperationFailed("dialog preview countdown"))?
        .frame(tick);
    Ok(FrameSnapshot {
        generation,
        local,
        tick,
        countdown,
    })
}

fn preview_frame(mode: DisplayMode, generation: u64) -> Result<FrameSnapshot, AppError> {
    let mut frame = local_frame(generation)?;
    if mode == DisplayMode::Countdown {
        frame.countdown = CountdownFrame {
            remaining_ms: 300_000,
            display_seconds: 300,
            ratio: 0.5,
            final_ten: false,
            dim: false,
            animating: false,
        };
    }
    Ok(frame)
}

fn invalidate_preview(hwnd: HWND) {
    // SAFETY: The dialog owns this child control; failure merely defers repaint.
    unsafe {
        let preview = GetDlgItem(hwnd, i32::from(resource_ids::IDC_PREVIEW));
        if !preview.is_null() {
            windows_sys::Win32::Graphics::Gdi::InvalidateRect(preview, ptr::null(), 0);
        }
    }
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain([0]).collect()
}

fn show_error(hwnd: HWND, text: &str) {
    let text = wide(text);
    // SAFETY: Both strings are terminated and live for this modal call.
    unsafe {
        MessageBoxW(
            hwnd,
            text.as_ptr(),
            windows_sys::w!("日期時間螢幕保護程式"),
            MB_OK | MB_ICONERROR,
        );
    }
}

fn initialize_config_controls(hwnd: HWND, draft: ConfigDraft) -> Result<(), AppError> {
    // SAFETY: All IDs name controls in the loaded configuration resource.
    unsafe {
        if CheckRadioButton(
            hwnd,
            i32::from(resource_ids::IDC_MODE_TIME_DATE),
            i32::from(resource_ids::IDC_MODE_JAPAN_TRAVEL),
            i32::from(match draft.display_mode {
                DisplayMode::TimeDate => resource_ids::IDC_MODE_TIME_DATE,
                DisplayMode::Countdown => resource_ids::IDC_MODE_COUNTDOWN,
                DisplayMode::JapanTravel => resource_ids::IDC_MODE_JAPAN_TRAVEL,
            }),
        ) == 0
        {
            return Err(last_error("CheckRadioButton(mode)"));
        }
        if CheckRadioButton(
            hwnd,
            i32::from(resource_ids::IDC_COLOR_DARK_RED),
            i32::from(resource_ids::IDC_COLOR_OFF_WHITE),
            i32::from(match draft.color_preset {
                ColorPreset::DarkRed => resource_ids::IDC_COLOR_DARK_RED,
                ColorPreset::DarkOrange => resource_ids::IDC_COLOR_DARK_ORANGE,
                ColorPreset::BrightGreen => resource_ids::IDC_COLOR_BRIGHT_GREEN,
                ColorPreset::OffWhite => resource_ids::IDC_COLOR_OFF_WHITE,
            }),
        ) == 0
        {
            return Err(last_error("CheckRadioButton(color)"));
        }
        for item in [
            "電子錶（預設）",
            "Consolas（打字機）",
            "新細明體",
            "自訂字型",
        ] {
            let item = wide(item);
            if SendDlgItemMessageW(
                hwnd,
                i32::from(resource_ids::IDC_FONT_COMBO),
                CB_ADDSTRING,
                0,
                item.as_ptr() as isize,
            ) == CB_ERR as isize
            {
                return Err(AppError::OperationFailed("CB_ADDSTRING(font)"));
            }
        }
        SendDlgItemMessageW(
            hwnd,
            i32::from(resource_ids::IDC_FONT_COMBO),
            CB_SETCURSEL,
            match draft.font_mode {
                FontMode::SevenSegment => 0,
                FontMode::Consolas => 1,
                FontMode::MingLiu => 2,
                FontMode::Custom => 3,
            },
            0,
        );
    }
    Ok(())
}

fn update_draft_from_command(state: &ConfigDialogState, id: u16) {
    let mut draft = state.draft.get();
    draft.display_mode = match id {
        value if value == resource_ids::IDC_MODE_TIME_DATE => DisplayMode::TimeDate,
        value if value == resource_ids::IDC_MODE_COUNTDOWN => DisplayMode::Countdown,
        value if value == resource_ids::IDC_MODE_JAPAN_TRAVEL => DisplayMode::JapanTravel,
        _ => draft.display_mode,
    };
    if (resource_ids::IDC_COLOR_DARK_RED..=resource_ids::IDC_COLOR_OFF_WHITE).contains(&id) {
        draft.color_preset = match id {
            resource_ids::IDC_COLOR_DARK_RED => ColorPreset::DarkRed,
            resource_ids::IDC_COLOR_DARK_ORANGE => ColorPreset::DarkOrange,
            resource_ids::IDC_COLOR_BRIGHT_GREEN => ColorPreset::BrightGreen,
            _ => ColorPreset::OffWhite,
        };
    }
    state.draft.set(draft);
}

fn update_font_combo(hwnd: HWND, state: &ConfigDialogState) {
    // SAFETY: Query the fixed dropdown-list selection owned by the dialog.
    let selected = unsafe {
        SendDlgItemMessageW(
            hwnd,
            i32::from(resource_ids::IDC_FONT_COMBO),
            CB_GETCURSEL,
            0,
            0,
        )
    };
    let mut draft = state.draft.get();
    draft.font_mode = match selected {
        1 => FontMode::Consolas,
        2 => FontMode::MingLiu,
        3 => FontMode::Custom,
        _ => FontMode::SevenSegment,
    };
    state.draft.set(draft);
}

fn choose_font(hwnd: HWND, state: &ConfigDialogState) {
    let draft = state.draft.get();
    // SAFETY: Query this live dialog's effective DPI.
    let dpi = unsafe { GetDpiForWindow(hwnd) }.max(96);
    let initial = draft.custom_font.unwrap_or_else(FontSpec::default_choice);
    let height = ((u64::from(initial.point_size_tenth()) * u64::from(dpi) + 360) / 720)
        .clamp(1, i32::MAX as u64) as i32;
    let mut logfont = initial.logfont(height);
    let mut choose = CHOOSEFONTW {
        lStructSize: size_of::<CHOOSEFONTW>() as u32,
        hwndOwner: hwnd,
        lpLogFont: &mut logfont,
        iPointSize: initial.point_size_tenth() as i32,
        Flags: CF_INITTOLOGFONTSTRUCT | CF_FORCEFONTEXIST | CF_SCREENFONTS | CF_LIMITSIZE,
        nSizeMin: 18,
        nSizeMax: 240,
        ..Default::default()
    };
    // SAFETY: CHOOSEFONTW and LOGFONTW remain live for the synchronous modal call.
    if unsafe { ChooseFontW(&mut choose) } != 0 {
        let Ok(points) = u32::try_from(choose.iPointSize) else {
            show_error(hwnd, "系統字型大小無效，請重新選擇。");
            return;
        };
        let Some(font) = FontSpec::from_logfont(logfont, points) else {
            show_error(hwnd, "系統字型資料無效，請重新選擇。");
            return;
        };
        let mut draft = state.draft.get();
        draft.custom_font = Some(font);
        draft.font_mode = FontMode::Custom;
        state.draft.set(draft);
        // SAFETY: Select fixed item 3; this does not send a recursive command.
        unsafe {
            SendDlgItemMessageW(
                hwnd,
                i32::from(resource_ids::IDC_FONT_COMBO),
                CB_SETCURSEL,
                3,
                0,
            );
        }
        invalidate_preview(hwnd);
    } else {
        // SAFETY: Reads the thread-local common-dialog error immediately.
        let code = unsafe { CommDlgExtendedError() };
        if code != 0 {
            show_error(hwnd, &format!("無法開啟系統字型選擇器（錯誤 {code}）。"));
        }
    }
}

fn save_config(hwnd: HWND, state: &ConfigDialogState) -> bool {
    update_font_combo(hwnd, state);
    let mut store = RegistryStore::new();
    match config::save_draft(&mut store, state.draft.get()) {
        Ok(()) => true,
        Err(error) => {
            persistence_error(hwnd, error);
            false
        }
    }
}

fn persistence_error(hwnd: HWND, error: config::SaveError) {
    let message = if matches!(error, config::SaveError::Rollback { .. }) {
        let _actual = load_registry();
        format!("{error}\n已重新讀取實際設定；目前草稿仍保留，可修正後重試。")
    } else {
        error.to_string()
    };
    show_error(hwnd, &message);
}

unsafe extern "system" fn config_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> isize {
    if message == WM_INITDIALOG {
        if lparam == 0 {
            return 0;
        }
        // SAFETY: lparam is the borrowed state supplied by configure.
        let state = unsafe { &*(lparam as *const ConfigDialogState) };
        let result = set_pointer(hwnd, DIALOG_USER, lparam)
            .and_then(|()| center(hwnd, state.center_on))
            .and_then(|()| initialize_config_controls(hwnd, state.draft.get()));
        if let Err(error) = result {
            state.error.set(Some(error));
            unsafe { EndDialog(hwnd, IDCANCEL as isize) };
            return 1;
        }
        // SAFETY: One low-frequency timer updates the clock and validates owner.
        let timer = unsafe { SetTimer(hwnd, CONFIG_TIMER, 1000, None) };
        if timer == 0 {
            state.error.set(Some(last_error("SetTimer(config)")));
            unsafe { EndDialog(hwnd, IDCANCEL as isize) };
        } else {
            state.timer.set(timer);
        }
        return 1;
    }
    // SAFETY: configure owns the allocation until DialogBoxParamW returns.
    let state =
        unsafe { (GetWindowLongPtrW(hwnd, DIALOG_USER) as *const ConfigDialogState).as_ref() };
    let Some(state) = state else {
        return 0;
    };
    let id = (wparam & 0xffff) as u16;
    let notification = ((wparam >> 16) & 0xffff) as u16;
    match message {
        #[cfg(debug_assertions)]
        TEST_GET_CONFIG_DRAFT => {
            let draft = state.draft.get();
            let result = draft.display_mode.registry_value() as isize
                | ((draft.color_preset.registry_value() as isize) << 8)
                | ((draft.font_mode.registry_value() as isize) << 16);
            // SAFETY: Publish this integer-only test result through DWLP_MSGRESULT.
            unsafe { SetWindowLongPtrW(hwnd, DIALOG_MESSAGE_RESULT, result) };
            1
        }
        WM_COMMAND if id == IDOK as u16 => {
            if save_config(hwnd, state) {
                unsafe { EndDialog(hwnd, IDOK as isize) };
            }
            1
        }
        WM_COMMAND if id == IDCANCEL as u16 => {
            unsafe { EndDialog(hwnd, IDCANCEL as isize) };
            1
        }
        WM_COMMAND if id == resource_ids::IDC_CHOOSE_FONT && notification == BN_CLICKED as u16 => {
            choose_font(hwnd, state);
            1
        }
        WM_COMMAND
            if notification == BN_CLICKED as u16
                && ((resource_ids::IDC_MODE_TIME_DATE..=resource_ids::IDC_MODE_JAPAN_TRAVEL)
                    .contains(&id)
                    || (resource_ids::IDC_COLOR_DARK_RED..=resource_ids::IDC_COLOR_OFF_WHITE)
                        .contains(&id)) =>
        {
            update_draft_from_command(state, id);
            invalidate_preview(hwnd);
            1
        }
        WM_COMMAND
            if id == resource_ids::IDC_FONT_COMBO && notification == CBN_SELCHANGE as u16 =>
        {
            update_font_combo(hwnd, state);
            invalidate_preview(hwnd);
            1
        }
        WM_DRAWITEM => {
            if lparam == 0 {
                return 0;
            }
            let item = unsafe { &*(lparam as *const DRAWITEMSTRUCT) };
            if item.CtlID != u32::from(resource_ids::IDC_PREVIEW) {
                return 0;
            }
            let generation = state.generation.get().wrapping_add(1);
            state.generation.set(generation);
            let draft = state.draft.get();
            let renderer = { state.renderer.borrow_mut().take() };
            if let Some(mut renderer) = renderer {
                let result = preview_frame(draft.display_mode, generation).and_then(|frame| {
                    renderer.draw_borrowed(
                        item.hDC,
                        item.rcItem,
                        unsafe { GetDpiForWindow(hwnd) }.max(96),
                        draft.display_mode,
                        frame,
                        Style::from_draft(draft),
                    )
                });
                state.renderer.borrow_mut().replace(renderer);
                if let Err(error) = result {
                    show_error(hwnd, &format!("預覽繪製失敗：{error}"));
                }
            }
            1
        }
        WM_TIMER if wparam == state.timer.get() => {
            if state.owner.is_some_and(|owner| !owner.alive()) {
                unsafe { EndDialog(hwnd, IDCANCEL as isize) };
            } else {
                invalidate_preview(hwnd);
            }
            1
        }
        WM_DPICHANGED => {
            state.renderer.borrow_mut().replace(Renderer::default());
            invalidate_preview(hwnd);
            0
        }
        WM_CLOSE => {
            unsafe { EndDialog(hwnd, IDCANCEL as isize) };
            1
        }
        WM_DESTROY => {
            let timer = state.timer.replace(0);
            if timer != 0 {
                unsafe { KillTimer(hwnd, timer) };
            }
            state.renderer.borrow_mut().take();
            0
        }
        WM_NCDESTROY => {
            let _ = set_pointer(hwnd, DIALOG_USER, 0);
            0
        }
        _ => 0,
    }
}

fn set_edit(hwnd: HWND, id: u16, value: u32) -> Result<(), AppError> {
    let text = wide(&format!("{value:02}"));
    // SAFETY: Control ID is an edit in the countdown resource; text is terminated.
    if unsafe { SetDlgItemTextW(hwnd, i32::from(id), text.as_ptr()) } == 0 {
        Err(last_error("SetDlgItemTextW(countdown)"))
    } else {
        unsafe {
            SendDlgItemMessageW(hwnd, i32::from(id), EM_SETLIMITTEXT, 2, 0);
        }
        Ok(())
    }
}

fn edit_text(hwnd: HWND, id: u16) -> String {
    let mut text = [0u16; 4];
    // SAFETY: Destination holds up to three characters plus NUL; edit is limited to two.
    let count = unsafe { GetDlgItemTextW(hwnd, i32::from(id), text.as_mut_ptr(), 4) };
    String::from_utf16_lossy(&text[..count as usize])
}

fn countdown_error(hwnd: HWND, error: InputError) {
    let (field, message) = match error {
        InputError::Hours => (
            resource_ids::IDC_COUNTDOWN_HOURS,
            "小時請輸入 0～99 的一或兩位半形數字。",
        ),
        InputError::Minutes => (
            resource_ids::IDC_COUNTDOWN_MINUTES,
            "分鐘請輸入 0～59 的一或兩位半形數字。",
        ),
        InputError::Seconds => (
            resource_ids::IDC_COUNTDOWN_SECONDS,
            "秒請輸入 0～59 的一或兩位半形數字。",
        ),
        InputError::Zero => (
            resource_ids::IDC_COUNTDOWN_HOURS,
            "總倒數時間必須大於 0 秒。",
        ),
        InputError::Overflow => (
            resource_ids::IDC_COUNTDOWN_HOURS,
            "總倒數時間超出有效範圍。",
        ),
    };
    let message = wide(message);
    // SAFETY: IDs are valid dialog controls; selection is confined to the edit.
    unsafe {
        SetDlgItemTextW(
            hwnd,
            i32::from(resource_ids::IDC_COUNTDOWN_ERROR),
            message.as_ptr(),
        );
        let edit = GetDlgItem(hwnd, i32::from(field));
        if !edit.is_null() {
            SetFocus(edit);
            SendMessageW(edit, EM_SETSEL, 0, -1);
        }
    }
}

fn save_countdown(hwnd: HWND, state: &CountdownDialogState) -> bool {
    let seconds = match parse_duration(
        &edit_text(hwnd, resource_ids::IDC_COUNTDOWN_HOURS),
        &edit_text(hwnd, resource_ids::IDC_COUNTDOWN_MINUTES),
        &edit_text(hwnd, resource_ids::IDC_COUNTDOWN_SECONDS),
    ) {
        Ok(seconds) => seconds,
        Err(error) => {
            countdown_error(hwnd, error);
            return false;
        }
    };
    let mut store = RegistryStore::new();
    match config::save_countdown(&mut store, seconds) {
        Ok(()) => {
            state.accepted_seconds.set(Some(seconds));
            true
        }
        Err(error) => {
            persistence_error(hwnd, error);
            false
        }
    }
}

unsafe extern "system" fn countdown_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> isize {
    if message == WM_INITDIALOG {
        if lparam == 0 {
            return 0;
        }
        // SAFETY: lparam is the borrowed state supplied by countdown.
        let state = unsafe { &*(lparam as *const CountdownDialogState) };
        let [hours, minutes, seconds] = crate::model::hms(state.initial_seconds);
        let result = set_pointer(hwnd, DIALOG_USER, lparam)
            .and_then(|()| center(hwnd, state.center_on))
            .and_then(|()| set_edit(hwnd, resource_ids::IDC_COUNTDOWN_HOURS, hours))
            .and_then(|()| set_edit(hwnd, resource_ids::IDC_COUNTDOWN_MINUTES, minutes))
            .and_then(|()| set_edit(hwnd, resource_ids::IDC_COUNTDOWN_SECONDS, seconds));
        if let Err(error) = result {
            state.error.set(Some(error));
            unsafe { EndDialog(hwnd, IDCANCEL as isize) };
            return 1;
        }
        // SAFETY: Select the initial hours value as required.
        unsafe {
            let edit = GetDlgItem(hwnd, i32::from(resource_ids::IDC_COUNTDOWN_HOURS));
            SetFocus(edit);
            SendMessageW(edit, EM_SETSEL, 0, -1);
        }
        if state.owner.is_some() {
            let timer = unsafe { SetTimer(hwnd, OWNER_TIMER, 1000, None) };
            if timer == 0 {
                state
                    .error
                    .set(Some(last_error("SetTimer(countdown owner)")));
                unsafe { EndDialog(hwnd, IDCANCEL as isize) };
            } else {
                state.timer.set(timer);
            }
        }
        return 0;
    }
    // SAFETY: countdown owns this allocation for the entire modal call.
    let state =
        unsafe { (GetWindowLongPtrW(hwnd, DIALOG_USER) as *const CountdownDialogState).as_ref() };
    let Some(state) = state else {
        return 0;
    };
    let id = (wparam & 0xffff) as u16;
    match message {
        #[cfg(debug_assertions)]
        TEST_SET_COUNTDOWN_FIELDS => {
            let hours = (wparam & 0xff) as u32;
            let minutes = ((wparam >> 8) & 0xff) as u32;
            let seconds = ((wparam >> 16) & 0xff) as u32;
            if let Err(error) = set_edit(hwnd, resource_ids::IDC_COUNTDOWN_HOURS, hours)
                .and_then(|()| set_edit(hwnd, resource_ids::IDC_COUNTDOWN_MINUTES, minutes))
                .and_then(|()| set_edit(hwnd, resource_ids::IDC_COUNTDOWN_SECONDS, seconds))
            {
                state.error.set(Some(error));
                unsafe { EndDialog(hwnd, IDCANCEL as isize) };
            }
            1
        }
        #[cfg(debug_assertions)]
        TEST_GET_COUNTDOWN_FIELDS => {
            let values = [
                edit_text(hwnd, resource_ids::IDC_COUNTDOWN_HOURS),
                edit_text(hwnd, resource_ids::IDC_COUNTDOWN_MINUTES),
                edit_text(hwnd, resource_ids::IDC_COUNTDOWN_SECONDS),
            ];
            let parsed = values
                .iter()
                .map(|value| value.parse::<u8>())
                .collect::<Result<Vec<_>, _>>();
            let result = match parsed {
                Ok(values) if values.len() == 3 => {
                    isize::from(values[0])
                        | (isize::from(values[1]) << 8)
                        | (isize::from(values[2]) << 16)
                }
                _ => -1,
            };
            // SAFETY: A dialog procedure publishes custom-message results through
            // DWLP_MSGRESULT before returning TRUE to the dialog manager.
            unsafe { SetWindowLongPtrW(hwnd, DIALOG_MESSAGE_RESULT, result) };
            1
        }
        WM_COMMAND if id == IDOK as u16 => {
            if save_countdown(hwnd, state) {
                unsafe { EndDialog(hwnd, IDOK as isize) };
            }
            1
        }
        WM_COMMAND if id == IDCANCEL as u16 => {
            unsafe { EndDialog(hwnd, IDCANCEL as isize) };
            1
        }
        WM_TIMER if wparam == state.timer.get() && state.timer.get() != 0 => {
            if state.owner.is_some_and(|owner| !owner.alive()) {
                unsafe { EndDialog(hwnd, IDCANCEL as isize) };
            }
            1
        }
        WM_CLOSE => {
            unsafe { EndDialog(hwnd, IDCANCEL as isize) };
            1
        }
        WM_DESTROY => {
            let timer = state.timer.replace(0);
            if timer != 0 {
                unsafe { KillTimer(hwnd, timer) };
            }
            0
        }
        WM_NCDESTROY => {
            let _ = set_pointer(hwnd, DIALOG_USER, 0);
            0
        }
        _ => 0,
    }
}
