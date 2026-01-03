#![windows_subsystem = "windows"]

//! KakaoTalk AdBlock - Rust implementation
//!
//! This application runs in the background and removes ads from the KakaoTalk
//! Windows client by monitoring and manipulating its windows.

mod debug_window;
mod icon;
mod locale;
mod process;
mod startup;
mod version;
mod window;

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use parking_lot::Mutex;
use windows::core::PCWSTR;
use windows::Win32::Foundation::{HWND, WAIT_OBJECT_0};
use windows::Win32::System::Threading::{CreateMutexW, WaitForSingleObject};
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, PeekMessageW, TranslateMessage, MSG, PM_REMOVE,
};

use muda::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{TrayIconBuilder, TrayIconEvent};

const VERSION: &str = env!("BUILD_VERSION");
const SLEEP_INTERVAL: Duration = Duration::from_millis(100);

// Layout constants from the original Go implementation
const LAYOUT_SHADOW_PADDING: i32 = 2;
const MAIN_VIEW_PADDING: i32 = 31;

/// Shared state for tracking KakaoTalk windows
pub struct AdBlockState {
    /// Main window handles (EVA_Window_Dblclk with title and no parent)
    pub main_windows: HashSet<isize>,
    /// Ad subwindow candidates with processed status (true = hidden/processed)
    pub ad_candidates: HashMap<isize, bool>,
    /// Cache for window class (class names don't change, safe to cache)
    pub window_class_cache: HashMap<isize, String>,
    /// Cache for custom scroll detection
    pub custom_scroll_cache: HashMap<isize, bool>,
}

impl AdBlockState {
    fn new() -> Self {
        Self {
            main_windows: HashSet::new(),
            ad_candidates: HashMap::new(),
            window_class_cache: HashMap::new(),
            custom_scroll_cache: HashMap::new(),
        }
    }

    /// Remove invalid window handles from all collections
    fn cleanup_invalid_handles(&mut self) {
        // Cleanup main_windows
        self.main_windows.retain(|&hwnd_key| {
            let hwnd = HWND(hwnd_key as *mut _);
            window::is_window_valid(hwnd)
        });

        // Cleanup ad_candidates
        self.ad_candidates.retain(|&hwnd_key, _| {
            let hwnd = HWND(hwnd_key as *mut _);
            window::is_window_valid(hwnd)
        });

        // Cleanup caches - keep only entries for valid windows
        let valid_main: HashSet<isize> = self.main_windows.clone();
        let valid_ads: HashSet<isize> = self.ad_candidates.keys().copied().collect();

        self.window_class_cache
            .retain(|k, _| valid_main.contains(k) || valid_ads.contains(k));
        self.custom_scroll_cache
            .retain(|k, _| valid_main.contains(k));
    }

    fn get_window_class(&mut self, hwnd: HWND) -> String {
        let key = hwnd.0 as isize;
        if let Some(class) = self.window_class_cache.get(&key) {
            return class.clone();
        }
        let class = window::get_class_name(hwnd);
        self.window_class_cache.insert(key, class.clone());
        class
    }
}

/// Check if HWND is null/invalid
fn is_hwnd_null(hwnd: HWND) -> bool {
    hwnd.0.is_null()
}

/// Watch for KakaoTalk windows and categorize them
fn watch_windows(state: Arc<Mutex<AdBlockState>>, running: Arc<AtomicBool>) {
    let mut cleanup_counter = 0u32;

    while running.load(Ordering::Relaxed) {
        // Find all KakaoTalk process IDs
        let pids = process::find_process_ids(process::KAKAOTALK_EXE);

        let mut state = state.lock();

        // Periodic cleanup of invalid handles (every ~1 second)
        cleanup_counter += 1;
        if cleanup_counter >= 10 {
            cleanup_counter = 0;
            state.cleanup_invalid_handles();
        }

        for pid in &pids {
            // Enumerate all windows for this process
            let windows = window::find_windows_by_pid(*pid);

            for hwnd in windows {
                let class_name = state.get_window_class(hwnd);
                // Don't cache window text - it can change when ads re-render
                let window_text = window::get_window_text(hwnd);
                let parent = window::get_parent(hwnd);
                let parent_key = parent.0 as isize;
                let hwnd_key = hwnd.0 as isize;

                match class_name.as_str() {
                    window::class_names::EVA_WINDOW_DBLCLK => {
                        if !window_text.is_empty() && is_hwnd_null(parent) {
                            // Main window
                            state.main_windows.insert(hwnd_key);
                        } else if window_text.is_empty() && !is_hwnd_null(parent) {
                            // Potential ad window if parent is a main window
                            if state.main_windows.contains(&parent_key) {
                                state.ad_candidates.entry(hwnd_key).or_insert(false);
                            }
                        }
                    }
                    window::class_names::EVA_WINDOW => {
                        if window_text.is_empty() && is_hwnd_null(parent) {
                            // Ad popup window
                            state.ad_candidates.entry(hwnd_key).or_insert(false);
                        }
                    }
                    _ => {}
                }
            }
        }

        drop(state);
        thread::sleep(SLEEP_INTERVAL);
    }
}

/// Check if window has custom scroll (starts with _EVA_)
fn has_custom_scroll(hwnd: HWND) -> bool {
    window::has_child_class_starting_with(hwnd, "_EVA_")
}

/// Check if this is a main window (has OnlineMainView or LockModeView child)
fn is_main_window(children: &[HWND], state: &mut AdBlockState) -> bool {
    for &child in children {
        let class_name = state.get_window_class(child);
        if class_name != window::class_names::EVA_CHILD_WINDOW {
            continue;
        }

        let text = window::get_window_text(child);
        if text.starts_with(window::window_texts::ONLINE_MAIN_VIEW)
            || text.starts_with(window::window_texts::LOCK_MODE_VIEW)
        {
            return true;
        }
    }
    false
}

/// Hide the main view ad area by resizing
fn hide_main_view_ad_area(window_text: &str, rect: &windows::Win32::Foundation::RECT, hwnd: HWND) {
    if window_text.starts_with(window::window_texts::ONLINE_MAIN_VIEW) {
        let width = rect.right - rect.left - LAYOUT_SHADOW_PADDING;
        let height = rect.bottom - rect.top - MAIN_VIEW_PADDING;

        if height < 1 {
            return;
        }

        window::update_window(hwnd);
        window::set_window_size(hwnd, width, height);
    }
}

/// Hide the lock screen ad area by resizing
fn hide_lock_screen_ad_area(
    window_text: &str,
    rect: &windows::Win32::Foundation::RECT,
    hwnd: HWND,
) {
    if window_text.starts_with(window::window_texts::LOCK_MODE_VIEW) {
        let width = rect.right - rect.left - LAYOUT_SHADOW_PADDING;
        let height = rect.bottom - rect.top;

        window::update_window(hwnd);
        window::set_window_size(hwnd, width, height);
    }
}

/// Remove ads from KakaoTalk windows
fn remove_ads(state: Arc<Mutex<AdBlockState>>, running: Arc<AtomicBool>) {
    while running.load(Ordering::Relaxed) {
        let mut state = state.lock();

        // Process main windows
        let main_windows: Vec<isize> = state.main_windows.iter().copied().collect();
        for hwnd_key in main_windows {
            let hwnd = HWND(hwnd_key as *mut _);

            // Skip invalid windows
            if is_hwnd_null(hwnd) {
                continue;
            }

            // Get child windows
            let children = window::get_child_windows(hwnd);

            // Check if this is really a main window
            if !is_main_window(&children, &mut state) {
                continue;
            }

            // Get window rect
            let rect = match window::get_window_rect(hwnd) {
                Some(r) => r,
                None => continue,
            };

            // Process child windows (skip first which is the main child)
            for child in children.iter().skip(1) {
                let class_name = state.get_window_class(*child);
                let window_text = window::get_window_text(*child);
                let parent = window::get_parent(*child);

                // Skip if not direct child of main window
                if parent != hwnd {
                    continue;
                }

                let parent_text = window::get_window_text(parent);

                // Hide ad child windows: resize to 0x0 first (instant visual removal),
                // then close (cleanup)
                if class_name == window::class_names::EVA_CHILD_WINDOW
                    && window_text.is_empty()
                    && !parent_text.is_empty()
                {
                    // Check for custom scroll
                    let has_scroll = state
                        .custom_scroll_cache
                        .get(&(hwnd.0 as isize))
                        .copied()
                        .unwrap_or_else(|| {
                            let result = has_custom_scroll(hwnd);
                            state.custom_scroll_cache.insert(hwnd.0 as isize, result);
                            result
                        });

                    if !has_scroll {
                        // Step 1: Resize to 0x0 (instant visual removal)
                        window::set_window_size(*child, 0, 0);
                        // Step 2: Close window (cleanup)
                        window::close_window(*child);
                        // Step 3: Force parent to redraw (fill blank space)
                        window::refresh_window(hwnd);
                    }
                }

                // Resize to hide ad areas
                hide_main_view_ad_area(&window_text, &rect, *child);
                hide_lock_screen_ad_area(&window_text, &rect, *child);
            }
        }

        // Hide ad popup windows (Chrome Legacy Window)
        // Check ALL candidates - ads can reappear even after being hidden
        let ad_candidates: Vec<(isize, bool)> =
            state.ad_candidates.iter().map(|(&k, &v)| (k, v)).collect();
        for (hwnd_key, was_processed) in ad_candidates {
            let hwnd = HWND(hwnd_key as *mut _);
            // Only process if window is visible (re-appeared or never hidden)
            if window::is_window_visible(hwnd) && window::has_chrome_legacy_window(hwnd) {
                window::hide_window(hwnd);
                state.ad_candidates.insert(hwnd_key, true);
            } else if was_processed && !window::is_window_visible(hwnd) {
                // Already hidden, keep processed status
            } else if !was_processed && !window::has_chrome_legacy_window(hwnd) {
                // Not a Chrome Legacy window, don't mark as processed yet
            }
        }

        drop(state);
        thread::sleep(SLEEP_INTERVAL);
    }
}

/// Message for version check result
enum VersionCheckResult {
    NewVersionAvailable(String),
}

/// Mutex name for single-instance check
const SINGLE_INSTANCE_MUTEX: &str = "Global\\KakaoTalkAdBlock_SingleInstance";

fn main() {
    // Single-instance check using named mutex
    let mutex_name: Vec<u16> = SINGLE_INSTANCE_MUTEX
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();

    let mutex = unsafe { CreateMutexW(None, true, PCWSTR(mutex_name.as_ptr())) };
    match mutex {
        Ok(handle) => {
            // Check if mutex was already owned (another instance exists)
            let wait_result = unsafe { WaitForSingleObject(handle, 0) };
            if wait_result != WAIT_OBJECT_0 {
                // Another instance is running, exit silently
                return;
            }
            // Handle is kept alive for the lifetime of the program
            // (HANDLE is Copy, so it doesn't have Drop - the OS releases it on process exit)
            let _ = handle;
        }
        Err(_) => {
            // Failed to create mutex, another instance likely exists
            return;
        }
    }
    // Create shared state
    let state = Arc::new(Mutex::new(AdBlockState::new()));
    let running = Arc::new(AtomicBool::new(true));

    // Initialize debug window with shared state
    debug_window::init(Arc::clone(&state));

    // Start background threads
    let state_clone = Arc::clone(&state);
    let running_clone = Arc::clone(&running);
    let watch_thread = thread::spawn(move || {
        watch_windows(state_clone, running_clone);
    });

    let state_clone = Arc::clone(&state);
    let running_clone = Arc::clone(&running);
    let remove_thread = thread::spawn(move || {
        remove_ads(state_clone, running_clone);
    });

    // Create channel for version check result
    let (version_tx, version_rx) = mpsc::channel::<VersionCheckResult>();

    // Check for new version in background
    thread::spawn(move || {
        let (tag_name, has_new) = version::check_latest_version(VERSION);
        if has_new {
            let _ = version_tx.send(VersionCheckResult::NewVersionAvailable(tag_name));
        }
    });

    // Get localized strings
    let strings = locale::get_strings();

    // Create menu
    let menu = Menu::new();

    let version_item = MenuItem::new(VERSION, false, None);
    let check_release_item =
        MenuItem::with_id("check_release", strings.check_for_updates, true, None);
    let separator = PredefinedMenuItem::separator();
    let debug_item = MenuItem::with_id("debug", strings.show_debug_window, true, None);
    let startup_item = MenuItem::with_id("startup", strings.run_on_startup, true, None);
    let exit_item = MenuItem::with_id("exit", strings.exit, true, None);

    menu.append(&version_item).unwrap();
    menu.append(&check_release_item).unwrap();
    menu.append(&separator).unwrap();
    menu.append(&debug_item).unwrap();
    menu.append(&startup_item).unwrap();
    menu.append(&exit_item).unwrap();

    // Check startup state and update menu
    if startup::is_startup_enabled() {
        // Mark as checked by changing the text
        startup_item.set_text(strings.run_on_startup_checked);
    }

    // Load icon
    let icon = icon::load_icon();

    // Create tray icon
    let _tray_icon = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip("KakaoTalkAdBlock")
        .with_icon(icon)
        .build()
        .expect("Failed to create tray icon");

    // Track startup state
    let mut startup_enabled = startup::is_startup_enabled();
    // Track debug window state for menu sync
    let mut debug_window_visible = false;

    // Event loop with Windows message pump
    let menu_channel = MenuEvent::receiver();
    let _tray_channel = TrayIconEvent::receiver();

    loop {
        // Pump Windows messages (required for tray icon context menu)
        unsafe {
            let mut msg = MSG::default();
            while PeekMessageW(&mut msg, HWND::default(), 0, 0, PM_REMOVE).as_bool() {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }

        // Sync debug window menu state (in case closed via right-click on window)
        let current_debug_visible = debug_window::is_visible();
        if current_debug_visible != debug_window_visible {
            debug_window_visible = current_debug_visible;
            let text = if debug_window_visible {
                strings.hide_debug_window
            } else {
                strings.show_debug_window
            };
            debug_item.set_text(text);
        }

        // Check for version check result
        if let Ok(result) = version_rx.try_recv() {
            match result {
                VersionCheckResult::NewVersionAvailable(tag_name) => {
                    let text = format!("{}{}", strings.new_version_available, tag_name);
                    check_release_item.set_text(&text);
                }
            }
        }

        // Handle menu events
        if let Ok(event) = menu_channel.try_recv() {
            match event.id.0.as_str() {
                "exit" => {
                    running.store(false, Ordering::Relaxed);
                    break;
                }
                "startup" => {
                    startup_enabled = !startup_enabled;
                    if let Err(e) = startup::set_startup_enabled(startup_enabled) {
                        eprintln!("Failed to set startup: {:?}", e);
                        startup_enabled = !startup_enabled; // Revert
                    }

                    let text = if startup_enabled {
                        strings.run_on_startup_checked
                    } else {
                        strings.run_on_startup
                    };
                    startup_item.set_text(text);
                }
                "check_release" => {
                    // Open releases page
                    let _ = open::that(version::RELEASES_PAGE_URL);
                }
                "debug" => {
                    // Toggle debug window
                    debug_window_visible = debug_window::toggle();
                    let text = if debug_window_visible {
                        strings.hide_debug_window
                    } else {
                        strings.show_debug_window
                    };
                    debug_item.set_text(text);
                }
                _ => {}
            }
        }

        // Small sleep to prevent busy-waiting
        thread::sleep(Duration::from_millis(10));
    }

    // Wait for background threads to finish
    let _ = watch_thread.join();
    let _ = remove_thread.join();
}
