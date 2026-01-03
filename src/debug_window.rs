//! Debug window - semi-transparent, draggable, topmost overlay with scrolling

use parking_lot::Mutex;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicPtr, Ordering};
use std::sync::Arc;
use windows::{
    core::PCWSTR,
    Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, RECT, WPARAM},
    Win32::Graphics::Gdi::{
        BeginPaint, BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, CreateFontW,
        CreateSolidBrush, DeleteDC, DeleteObject, DrawTextW, EndPaint, FillRect, InvalidateRect,
        SelectObject, SetBkMode, SetTextColor, CLEARTYPE_QUALITY, CLIP_DEFAULT_PRECIS,
        DEFAULT_CHARSET, DEFAULT_PITCH, DT_LEFT, DT_NOCLIP, DT_TOP, FF_DONTCARE, HBRUSH, HFONT,
        OUT_DEFAULT_PRECIS, PAINTSTRUCT, SRCCOPY, TRANSPARENT,
    },
    Win32::System::LibraryLoader::GetModuleHandleW,
    Win32::UI::Input::KeyboardAndMouse::ReleaseCapture,
    Win32::UI::WindowsAndMessaging::*,
};

use crate::{process, window, AdBlockState};

const DEBUG_WINDOW_CLASS: &str = "KakaoTalkAdBlockDebug";
const WINDOW_WIDTH: i32 = 320;
const WINDOW_HEIGHT: i32 = 400;
const BG_COLOR: u32 = 0x2D2D2D; // Dark gray
const TEXT_COLOR: u32 = 0x00FF00; // Green (BGR format for Windows)
const TITLE_COLOR: u32 = 0x00D4FA; // Yellow/gold
const PROCESSED_COLOR: u32 = 0x808080; // Gray for processed items
const UPDATE_TIMER_ID: usize = 1;
const UPDATE_INTERVAL_MS: u32 = 500;
const LINE_HEIGHT: i32 = 16;

// Global state for the debug window
static DEBUG_HWND: AtomicPtr<std::ffi::c_void> = AtomicPtr::new(std::ptr::null_mut());
static DEBUG_VISIBLE: AtomicBool = AtomicBool::new(false);
static SCROLL_OFFSET: AtomicI32 = AtomicI32::new(0);
static mut DEBUG_STATE: Option<Arc<Mutex<AdBlockState>>> = None;
static mut DEBUG_FONT: Option<HFONT> = None;
static mut TOTAL_LINES: i32 = 0;

/// Initialize the debug window (call once at startup)
pub fn init(state: Arc<Mutex<AdBlockState>>) {
    unsafe {
        DEBUG_STATE = Some(state);
    }
    register_window_class();
}

/// Toggle debug window visibility
pub fn toggle() -> bool {
    let visible = !DEBUG_VISIBLE.load(Ordering::Relaxed);

    if visible {
        show();
    } else {
        hide();
    }

    visible
}

/// Check if debug window is visible
#[allow(dead_code)]
pub fn is_visible() -> bool {
    DEBUG_VISIBLE.load(Ordering::Relaxed)
}

/// Show the debug window
pub fn show() {
    let hwnd = get_or_create_window();
    if !hwnd.0.is_null() {
        unsafe {
            let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
            DEBUG_VISIBLE.store(true, Ordering::Relaxed);
            // Start update timer
            SetTimer(hwnd, UPDATE_TIMER_ID, UPDATE_INTERVAL_MS, None);
            // Force initial paint
            let _ = InvalidateRect(hwnd, None, true);
        }
    }
}

/// Hide the debug window
pub fn hide() {
    let hwnd = HWND(DEBUG_HWND.load(Ordering::Relaxed));
    if !hwnd.0.is_null() {
        unsafe {
            KillTimer(hwnd, UPDATE_TIMER_ID).ok();
            let _ = ShowWindow(hwnd, SW_HIDE);
            DEBUG_VISIBLE.store(false, Ordering::Relaxed);
        }
    }
}

fn get_or_create_window() -> HWND {
    let ptr = DEBUG_HWND.load(Ordering::Relaxed);
    if !ptr.is_null() {
        return HWND(ptr);
    }

    create_window()
}

fn register_window_class() {
    unsafe {
        let class_name = to_wide(DEBUG_WINDOW_CLASS);
        let hinstance = GetModuleHandleW(None).unwrap_or_default();

        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(window_proc),
            hInstance: hinstance.into(),
            hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
            hbrBackground: HBRUSH(std::ptr::null_mut()),
            lpszClassName: PCWSTR(class_name.as_ptr()),
            ..Default::default()
        };

        RegisterClassExW(&wc);
    }
}

fn create_window() -> HWND {
    unsafe {
        let class_name = to_wide(DEBUG_WINDOW_CLASS);
        let hinstance = GetModuleHandleW(None).unwrap_or_default();

        // Get screen dimensions for positioning
        let screen_width = GetSystemMetrics(SM_CXSCREEN);
        let screen_height = GetSystemMetrics(SM_CYSCREEN);
        let x = screen_width - WINDOW_WIDTH - 20;
        let y = screen_height - WINDOW_HEIGHT - 60;

        let hwnd = CreateWindowExW(
            WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_LAYERED,
            PCWSTR(class_name.as_ptr()),
            PCWSTR::null(),
            WS_POPUP,
            x,
            y,
            WINDOW_WIDTH,
            WINDOW_HEIGHT,
            None,
            None,
            hinstance,
            None,
        )
        .unwrap_or_default();

        if !hwnd.0.is_null() {
            // Set transparency (200 out of 255 = ~78% opaque)
            SetLayeredWindowAttributes(hwnd, COLORREF(0), 220, LWA_ALPHA).ok();

            // Create font
            let font_name = to_wide("Consolas");
            DEBUG_FONT = Some(CreateFontW(
                14,
                0,
                0,
                0,
                400,
                0,
                0,
                0,
                DEFAULT_CHARSET.0 as u32,
                OUT_DEFAULT_PRECIS.0 as u32,
                CLIP_DEFAULT_PRECIS.0 as u32,
                CLEARTYPE_QUALITY.0 as u32,
                (DEFAULT_PITCH.0 | FF_DONTCARE.0) as u32,
                PCWSTR(font_name.as_ptr()),
            ));

            DEBUG_HWND.store(hwnd.0, Ordering::Relaxed);
        }

        hwnd
    }
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_PAINT => {
            let mut ps = PAINTSTRUCT::default();
            let hdc = BeginPaint(hwnd, &mut ps);

            let mut rect = RECT::default();
            GetClientRect(hwnd, &mut rect).ok();

            // Double buffering: create off-screen DC
            let mem_dc = CreateCompatibleDC(hdc);
            let mem_bitmap = CreateCompatibleBitmap(hdc, rect.right, rect.bottom);
            let old_bitmap = SelectObject(mem_dc, mem_bitmap);

            // Fill background
            let bg_brush = CreateSolidBrush(COLORREF(BG_COLOR));
            FillRect(mem_dc, &rect, bg_brush);
            let _ = DeleteObject(bg_brush);

            // Set up text drawing
            SetBkMode(mem_dc, TRANSPARENT);
            if let Some(font) = DEBUG_FONT {
                SelectObject(mem_dc, font);
            }

            // Get debug info and draw
            let (info, line_colors) = get_debug_info_with_colors();
            let lines: Vec<&str> = info.lines().collect();

            TOTAL_LINES = lines.len() as i32;
            let scroll = SCROLL_OFFSET.load(Ordering::Relaxed);

            let mut y = 10 - scroll;
            for (i, line) in lines.iter().enumerate() {
                if y + LINE_HEIGHT > 0 && y < rect.bottom {
                    let color = line_colors.get(i).copied().unwrap_or(TEXT_COLOR);
                    SetTextColor(mem_dc, COLORREF(color));

                    let mut text = to_wide(line);
                    let mut line_rect = RECT {
                        left: 10,
                        top: y,
                        right: rect.right - 10,
                        bottom: y + LINE_HEIGHT,
                    };
                    DrawTextW(
                        mem_dc,
                        &mut text,
                        &mut line_rect,
                        DT_LEFT | DT_TOP | DT_NOCLIP,
                    );
                }
                y += LINE_HEIGHT;
            }

            // Copy to screen
            let _ = BitBlt(hdc, 0, 0, rect.right, rect.bottom, mem_dc, 0, 0, SRCCOPY);

            // Cleanup
            SelectObject(mem_dc, old_bitmap);
            let _ = DeleteObject(mem_bitmap);
            let _ = DeleteDC(mem_dc);

            let _ = EndPaint(hwnd, &ps);
            LRESULT(0)
        }
        WM_TIMER => {
            if wparam.0 == UPDATE_TIMER_ID {
                // Redraw with updated info (no flicker due to double buffering)
                let _ = InvalidateRect(hwnd, None, false);
            }
            LRESULT(0)
        }
        WM_MOUSEWHEEL => {
            // Handle scroll
            let delta = (wparam.0 >> 16) as i16;
            let current = SCROLL_OFFSET.load(Ordering::Relaxed);
            let max_scroll = (TOTAL_LINES * LINE_HEIGHT - WINDOW_HEIGHT + 40).max(0);
            let new_scroll = (current - (delta as i32 / 4)).clamp(0, max_scroll);
            SCROLL_OFFSET.store(new_scroll, Ordering::Relaxed);
            let _ = InvalidateRect(hwnd, None, false);
            LRESULT(0)
        }
        WM_LBUTTONDOWN => {
            // Allow dragging the window
            let _ = ReleaseCapture();
            SendMessageW(
                hwnd,
                WM_NCLBUTTONDOWN,
                WPARAM(HTCAPTION as usize),
                LPARAM(0),
            );
            LRESULT(0)
        }
        WM_RBUTTONDOWN => {
            // Right-click to close
            hide();
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

/// Returns (info_string, colors_per_line)
fn get_debug_info_with_colors() -> (String, Vec<u32>) {
    let mut info = String::new();
    let mut colors: Vec<u32> = Vec::new();

    info.push_str("KakaoTalk AdBlock Debug\n");
    colors.push(TITLE_COLOR);
    info.push_str("━━━━━━━━━━━━━━━━━━━━━━━\n");
    colors.push(TITLE_COLOR);

    let pids = process::find_process_ids(process::KAKAOTALK_EXE);

    if pids.is_empty() {
        info.push_str("\n");
        colors.push(TEXT_COLOR);
        info.push_str("[!] KakaoTalk not running\n");
        colors.push(0x0000FF); // Red for warning
    } else {
        info.push_str(&format!("\nPIDs: {:?}\n", pids));
        colors.push(TEXT_COLOR);
        colors.push(TEXT_COLOR);
    }

    unsafe {
        if let Some(ref state_arc) = DEBUG_STATE {
            let state = state_arc.lock();

            info.push_str(&format!("\nMain Windows: {}\n", state.main_windows.len()));
            colors.push(TEXT_COLOR);
            colors.push(TEXT_COLOR);

            for &hwnd_key in &state.main_windows {
                let hwnd = HWND(hwnd_key as *mut _);
                let valid = window::is_window_valid(hwnd);
                let title = if valid {
                    window::get_window_text(hwnd)
                } else {
                    String::new()
                };
                let status = if !valid {
                    " [INVALID]"
                } else if title.is_empty() {
                    " (empty)"
                } else {
                    ""
                };
                let title_display = if title.is_empty() {
                    String::new()
                } else {
                    format!(" {}", title)
                };
                info.push_str(&format!(
                    "  0x{:08X}{}{}\n",
                    hwnd_key, title_display, status
                ));
                colors.push(if !valid { PROCESSED_COLOR } else { TEXT_COLOR });
            }

            let processed_count = state.ad_candidates.values().filter(|&&v| v).count();
            info.push_str(&format!(
                "\nAd Candidates: {} ({} blocked)\n",
                state.ad_candidates.len(),
                processed_count
            ));
            colors.push(TEXT_COLOR);
            colors.push(TEXT_COLOR);

            for (&hwnd_key, &processed) in &state.ad_candidates {
                let status = if processed { "✓" } else { "○" };
                info.push_str(&format!("  {} 0x{:08X}\n", status, hwnd_key));
                colors.push(if processed {
                    PROCESSED_COLOR
                } else {
                    TEXT_COLOR
                });
            }

            info.push_str(&format!(
                "\nCached: {} classes\n",
                state.window_class_cache.len()
            ));
            colors.push(TEXT_COLOR);
            colors.push(TEXT_COLOR);
        }
    }

    info.push_str("\n─────────────────────\n");
    colors.push(TEXT_COLOR);
    colors.push(TITLE_COLOR);
    info.push_str("Drag | RClick close | Scroll");
    colors.push(TITLE_COLOR);

    (info, colors)
}

fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}
