//! Windows startup management via registry

use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use windows::{
    core::PCWSTR,
    Win32::Foundation::WIN32_ERROR,
    Win32::System::Registry::{
        RegCloseKey, RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW, HKEY,
        HKEY_CURRENT_USER, KEY_QUERY_VALUE, KEY_SET_VALUE, REG_SZ,
    },
};

const STARTUP_KEY: &str = r"SOFTWARE\Microsoft\Windows\CurrentVersion\Run";
const APP_NAME: &str = "KakaoTalkAdBlock";

/// Convert a Rust string to a wide string (null-terminated UTF-16)
fn to_wide(s: &str) -> Vec<u16> {
    OsStr::new(s)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

/// Check if the application is set to run at startup
pub fn is_startup_enabled() -> bool {
    unsafe {
        let key_path = to_wide(STARTUP_KEY);
        let mut hkey = HKEY::default();

        let result = RegOpenKeyExW(
            HKEY_CURRENT_USER,
            PCWSTR(key_path.as_ptr()),
            0,
            KEY_QUERY_VALUE,
            &mut hkey,
        );

        if result != WIN32_ERROR(0) {
            return false;
        }

        let value_name = to_wide(APP_NAME);
        let result = RegQueryValueExW(hkey, PCWSTR(value_name.as_ptr()), None, None, None, None);

        let _ = RegCloseKey(hkey);

        result == WIN32_ERROR(0)
    }
}

/// Enable or disable running at startup
pub fn set_startup_enabled(enable: bool) -> Result<(), String> {
    unsafe {
        let key_path = to_wide(STARTUP_KEY);
        let mut hkey = HKEY::default();

        let result = RegOpenKeyExW(
            HKEY_CURRENT_USER,
            PCWSTR(key_path.as_ptr()),
            0,
            KEY_SET_VALUE,
            &mut hkey,
        );

        if result != WIN32_ERROR(0) {
            return Err(format!("Failed to open registry key: {:?}", result));
        }

        let value_name = to_wide(APP_NAME);

        let result = if enable {
            // Get current executable path
            let exe_path = std::env::current_exe()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default();

            let exe_path_wide = to_wide(&exe_path);
            let data_len = (exe_path_wide.len() * 2) as u32;

            RegSetValueExW(
                hkey,
                PCWSTR(value_name.as_ptr()),
                0,
                REG_SZ,
                Some(std::slice::from_raw_parts(
                    exe_path_wide.as_ptr() as *const u8,
                    data_len as usize,
                )),
            )
        } else {
            RegDeleteValueW(hkey, PCWSTR(value_name.as_ptr()))
        };

        let _ = RegCloseKey(hkey);

        if result == WIN32_ERROR(0) {
            Ok(())
        } else {
            Err(format!("Failed to set registry value: {:?}", result))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_startup_check() {
        // Should not panic
        let _ = is_startup_enabled();
    }
}
