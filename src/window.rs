//! Window enumeration and manipulation utilities

use windows::{
    Win32::Foundation::{BOOL, HWND, LPARAM, RECT, WPARAM},
    Win32::Graphics::Gdi::{InvalidateRect, UpdateWindow},
    Win32::UI::WindowsAndMessaging::{
        EnumChildWindows, EnumWindows, GetClassNameW, GetParent, GetWindowRect, GetWindowTextW,
        GetWindowThreadProcessId, IsWindow, IsWindowVisible, SendMessageW, SetWindowPos,
        ShowWindow, HWND_TOP, SWP_NOMOVE, SW_HIDE, WM_CLOSE,
    },
};

/// Check if a window handle is still valid
pub fn is_window_valid(hwnd: HWND) -> bool {
    unsafe { IsWindow(hwnd).as_bool() }
}

/// Check if a window is visible
pub fn is_window_visible(hwnd: HWND) -> bool {
    unsafe { IsWindowVisible(hwnd).as_bool() }
}

/// Window class names used by KakaoTalk
pub mod class_names {
    pub const EVA_WINDOW_DBLCLK: &str = "EVA_Window_Dblclk";
    pub const EVA_WINDOW: &str = "EVA_Window";
    pub const EVA_CHILD_WINDOW: &str = "EVA_ChildWindow";
}

/// Window text prefixes for identifying window types
pub mod window_texts {
    pub const ONLINE_MAIN_VIEW: &str = "OnlineMainView";
    pub const LOCK_MODE_VIEW: &str = "LockModeView";
    pub const CHROME_LEGACY: &str = "Chrome Legacy Window";
}

/// Get the class name of a window
pub fn get_class_name(hwnd: HWND) -> String {
    unsafe {
        let mut buffer = [0u16; 256];
        let len = GetClassNameW(hwnd, &mut buffer);
        if len > 0 {
            String::from_utf16_lossy(&buffer[..len as usize])
        } else {
            String::new()
        }
    }
}

/// Get the window text (title) of a window
pub fn get_window_text(hwnd: HWND) -> String {
    unsafe {
        let mut buffer = [0u16; 256];
        let len = GetWindowTextW(hwnd, &mut buffer);
        if len > 0 {
            String::from_utf16_lossy(&buffer[..len as usize])
        } else {
            String::new()
        }
    }
}

/// Get the parent window handle
pub fn get_parent(hwnd: HWND) -> HWND {
    unsafe { GetParent(hwnd).unwrap_or(HWND::default()) }
}

/// Get the process ID that owns the window
pub fn get_window_process_id(hwnd: HWND) -> u32 {
    unsafe {
        let mut pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        pid
    }
}

/// Get window rectangle
pub fn get_window_rect(hwnd: HWND) -> Option<RECT> {
    unsafe {
        let mut rect = RECT::default();
        if GetWindowRect(hwnd, &mut rect).is_ok() {
            Some(rect)
        } else {
            None
        }
    }
}

/// Enumerate all top-level windows
pub fn enum_windows<F>(mut callback: F)
where
    F: FnMut(HWND) -> bool,
{
    unsafe extern "system" fn enum_proc<F>(hwnd: HWND, lparam: LPARAM) -> BOOL
    where
        F: FnMut(HWND) -> bool,
    {
        let callback = &mut *(lparam.0 as *mut F);
        BOOL::from(callback(hwnd))
    }

    unsafe {
        let _ = EnumWindows(
            Some(enum_proc::<F>),
            LPARAM(&mut callback as *mut F as isize),
        );
    }
}

/// Enumerate child windows of a parent window
pub fn enum_child_windows<F>(parent: HWND, mut callback: F)
where
    F: FnMut(HWND) -> bool,
{
    unsafe extern "system" fn enum_proc<F>(hwnd: HWND, lparam: LPARAM) -> BOOL
    where
        F: FnMut(HWND) -> bool,
    {
        let callback = &mut *(lparam.0 as *mut F);
        BOOL::from(callback(hwnd))
    }

    unsafe {
        let _ = EnumChildWindows(
            parent,
            Some(enum_proc::<F>),
            LPARAM(&mut callback as *mut F as isize),
        );
    }
}

/// Get all child window handles
pub fn get_child_windows(parent: HWND) -> Vec<HWND> {
    let mut children = Vec::new();
    enum_child_windows(parent, |hwnd| {
        children.push(hwnd);
        true
    });
    children
}

/// Send WM_CLOSE message to a window
pub fn close_window(hwnd: HWND) {
    unsafe {
        SendMessageW(hwnd, WM_CLOSE, WPARAM(0), LPARAM(0));
    }
}

/// Hide a window
pub fn hide_window(hwnd: HWND) {
    unsafe {
        let _ = ShowWindow(hwnd, SW_HIDE);
    }
}

/// Update window
pub fn update_window(hwnd: HWND) {
    unsafe {
        let _ = UpdateWindow(hwnd);
    }
}

/// Force window to redraw (invalidate + update)
pub fn refresh_window(hwnd: HWND) {
    unsafe {
        // Invalidate entire client area, erase background
        let _ = InvalidateRect(hwnd, None, true);
        // Force immediate repaint
        let _ = UpdateWindow(hwnd);
    }
}

/// Set window position and size
pub fn set_window_pos(hwnd: HWND, x: i32, y: i32, width: i32, height: i32, flags: u32) {
    unsafe {
        let _ = SetWindowPos(
            hwnd,
            HWND_TOP,
            x,
            y,
            width,
            height,
            windows::Win32::UI::WindowsAndMessaging::SET_WINDOW_POS_FLAGS(flags),
        );
    }
}

/// Set window size only (keeps position)
pub fn set_window_size(hwnd: HWND, width: i32, height: i32) {
    set_window_pos(hwnd, 0, 0, width, height, SWP_NOMOVE.0);
}

/// Find all windows belonging to a specific process
pub fn find_windows_by_pid(target_pid: u32) -> Vec<HWND> {
    let mut windows = Vec::new();

    enum_windows(|hwnd| {
        let pid = get_window_process_id(hwnd);
        if pid == target_pid {
            windows.push(hwnd);
        }
        true
    });

    windows
}

/// Check if a window or any of its children has a class name starting with the given prefix
pub fn has_child_class_starting_with(hwnd: HWND, prefix: &str) -> bool {
    let class_name = get_class_name(hwnd);
    if class_name.starts_with(prefix) {
        return true;
    }

    let children = get_child_windows(hwnd);
    for child in children {
        if has_child_class_starting_with(child, prefix) {
            return true;
        }
    }

    false
}

/// Check if a window or any of its children has the "Chrome Legacy Window" text
pub fn has_chrome_legacy_window(hwnd: HWND) -> bool {
    let text = get_window_text(hwnd);
    if text == window_texts::CHROME_LEGACY {
        return true;
    }

    let children = get_child_windows(hwnd);
    for child in children {
        if has_chrome_legacy_window(child) {
            return true;
        }
    }

    false
}
