//! Process enumeration utilities for finding KakaoTalk process

use windows::{
    Win32::Foundation::CloseHandle,
    Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
        TH32CS_SNAPPROCESS,
    },
};

/// Target executable name (case-insensitive)
pub const KAKAOTALK_EXE: &str = "kakaotalk.exe";

/// Find all process IDs matching the given executable name
pub fn find_process_ids(exe_name: &str) -> Vec<u32> {
    let mut pids = Vec::new();

    unsafe {
        let snapshot = match CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) {
            Ok(handle) => handle,
            Err(_) => return pids,
        };

        let mut entry = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };

        if Process32FirstW(snapshot, &mut entry).is_ok() {
            loop {
                let process_name = String::from_utf16_lossy(&entry.szExeFile)
                    .trim_matches('\0')
                    .to_lowercase();

                if process_name == exe_name.to_lowercase() {
                    pids.push(entry.th32ProcessID);
                }

                if Process32NextW(snapshot, &mut entry).is_err() {
                    break;
                }
            }
        }

        let _ = CloseHandle(snapshot);
    }

    pids
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_process_ids() {
        // Should not panic
        let _ = find_process_ids("explorer.exe");
    }
}
