//! Build script for automatic version detection and Windows resource embedding
//!
//! Version format:
//! - CI (main branch): YYYYMMDD-NN (from RELEASE_VERSION env)
//! - CI (other branches): dev-{branch}-{short_sha}
//! - Local build: dev

use std::env;
use std::process::Command;

fn main() {
    // Tell Cargo to rerun if these change
    println!("cargo:rerun-if-env-changed=CI");
    println!("cargo:rerun-if-env-changed=RELEASE_VERSION");
    println!("cargo:rerun-if-env-changed=GITHUB_REF_NAME");
    println!("cargo:rerun-if-env-changed=GITHUB_SHA");

    let version = determine_version();
    println!("cargo:rustc-env=BUILD_VERSION={}", version);

    // Windows resource embedding
    if env::var("CARGO_CFG_TARGET_OS").unwrap_or_default() == "windows" {
        embed_windows_resource(&version);
    }
}

fn embed_windows_resource(version: &str) {
    let mut res = winres::WindowsResource::new();

    // File version (use 0.0.0.0 for dev builds, parse for release)
    let (major, minor, patch, build) = parse_version_numbers(version);

    res.set("FileDescription", "KakaoTalk AdBlock")
        .set("ProductName", "KakaoTalk AdBlock")
        .set("OriginalFilename", "kakaotalk_adblock.exe")
        .set("LegalCopyright", "MIT License")
        .set("CompanyName", "ssut")
        .set("Comments", "https://github.com/ssut/kakaotalk-adblock-rs")
        .set("FileVersion", version)
        .set("ProductVersion", version);

    // Set numeric version
    res.set_version_info(
        winres::VersionInfo::FILEVERSION,
        file_version(major, minor, patch, build),
    );
    res.set_version_info(
        winres::VersionInfo::PRODUCTVERSION,
        file_version(major, minor, patch, build),
    );

    if let Err(e) = res.compile() {
        eprintln!("Warning: Failed to compile Windows resource: {}", e);
    }
}

fn parse_version_numbers(version: &str) -> (u16, u16, u16, u16) {
    // Try to parse YYYYMMDD-NN format
    if let Some((date, build_num)) = version.split_once('-') {
        if date.len() == 8 && date.chars().all(|c| c.is_ascii_digit()) {
            let year: u16 = date[0..4].parse().unwrap_or(0);
            let month: u16 = date[4..6].parse().unwrap_or(0);
            let day: u16 = date[6..8].parse().unwrap_or(0);
            let build: u16 = build_num.parse().unwrap_or(0);
            // Encode as: year.month.day.build
            return (year, month, day, build);
        }
    }

    // Dev build - use 0.0.0.0
    (0, 0, 0, 0)
}

fn file_version(major: u16, minor: u16, patch: u16, build: u16) -> u64 {
    ((major as u64) << 48) | ((minor as u64) << 32) | ((patch as u64) << 16) | (build as u64)
}

fn determine_version() -> String {
    // Check if running in CI
    let is_ci = env::var("CI").is_ok();

    if is_ci {
        // Check for explicit release version (set by GitHub Actions for main branch)
        if let Ok(release_version) = env::var("RELEASE_VERSION") {
            return release_version;
        }

        // For non-main branches, use dev-{branch}-{short_sha}
        let branch = env::var("GITHUB_REF_NAME").unwrap_or_else(|_| "unknown".to_string());
        let sha = env::var("GITHUB_SHA").unwrap_or_else(|_| "unknown".to_string());
        let short_sha = &sha[..7.min(sha.len())];

        return format!("dev-{}-{}", sanitize_branch(&branch), short_sha);
    }

    // Local build: try to get git info, fallback to "dev"
    if let Some(version) = get_local_git_version() {
        return version;
    }

    "dev".to_string()
}

fn get_local_git_version() -> Option<String> {
    // Get current branch
    let branch_output = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .ok()?;

    if !branch_output.status.success() {
        return None;
    }

    let branch = String::from_utf8_lossy(&branch_output.stdout)
        .trim()
        .to_string();

    // Get short SHA
    let sha_output = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()?;

    if !sha_output.status.success() {
        return Some("dev".to_string());
    }

    let short_sha = String::from_utf8_lossy(&sha_output.stdout)
        .trim()
        .to_string();

    if branch == "main" || branch == "master" {
        Some(format!("dev-{}", short_sha))
    } else {
        Some(format!("dev-{}-{}", sanitize_branch(&branch), short_sha))
    }
}

fn sanitize_branch(branch: &str) -> String {
    branch
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' {
                c
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}
