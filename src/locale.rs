//! Localization support - Korean and English based on OS language

use windows::Win32::Globalization::GetUserDefaultUILanguage;

/// Language identifiers
const LANG_KOREAN: u16 = 0x12; // Korean primary language ID

/// Localized strings
pub struct Strings {
    pub run_on_startup: &'static str,
    pub run_on_startup_checked: &'static str,
    pub exit: &'static str,
    pub new_version_available: &'static str,
    pub check_for_updates: &'static str,
    pub show_debug_window: &'static str,
    pub hide_debug_window: &'static str,
}

/// English strings
const STRINGS_EN: Strings = Strings {
    run_on_startup: "Run on startup",
    run_on_startup_checked: "\u{2713} Run on startup", // ✓
    exit: "Exit",
    new_version_available: "New version available: ",
    check_for_updates: "Check for updates",
    show_debug_window: "Show debug window",
    hide_debug_window: "\u{2713} Show debug window", // ✓
};

/// Korean strings
const STRINGS_KO: Strings = Strings {
    run_on_startup: "시작 시 자동 실행",
    run_on_startup_checked: "\u{2713} 시작 시 자동 실행", // ✓
    exit: "종료",
    new_version_available: "새 버전: ",
    check_for_updates: "업데이트 확인",
    show_debug_window: "디버그 창 표시",
    hide_debug_window: "\u{2713} 디버그 창 표시", // ✓
};

/// Check if the system language is Korean
pub fn is_korean() -> bool {
    unsafe {
        let lang_id = GetUserDefaultUILanguage();
        // Primary language ID is in the lower 10 bits
        (lang_id & 0x3FF) == LANG_KOREAN
    }
}

/// Get localized strings based on system language
pub fn get_strings() -> &'static Strings {
    if is_korean() {
        &STRINGS_KO
    } else {
        &STRINGS_EN
    }
}
