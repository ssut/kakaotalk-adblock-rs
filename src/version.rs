//! Version checking functionality

use serde::Deserialize;

const RELEASES_API_URL: &str =
    "https://api.github.com/repos/ssut/kakaotalk-adblock-rs/releases/latest";
pub const RELEASES_PAGE_URL: &str = "https://github.com/ssut/kakaotalk-adblock-rs/releases";

#[derive(Deserialize)]
struct GitHubRelease {
    tag_name: String,
}

/// Check for the latest version on GitHub
/// Returns (tag_name, has_new_release)
pub fn check_latest_version(current_version: &str) -> (String, bool) {
    let result = check_latest_version_inner(current_version);
    result.unwrap_or_else(|_| (current_version.to_string(), false))
}

fn check_latest_version_inner(
    current_version: &str,
) -> Result<(String, bool), Box<dyn std::error::Error>> {
    let response: GitHubRelease = ureq::get(RELEASES_API_URL)
        .set("User-Agent", "KakaoTalkAdBlock")
        .call()?
        .into_json()?;

    let tag_name = response.tag_name;
    let has_new = has_new_release(current_version, &tag_name);

    Ok((tag_name, has_new))
}

/// Compare version strings and determine if there's a new release
fn has_new_release(current: &str, latest: &str) -> bool {
    let parse_version = |v: &str| -> Vec<u32> {
        v.trim_start_matches('v')
            .split('.')
            .filter_map(|s| s.parse().ok())
            .collect()
    };

    let current_parts = parse_version(current);
    let latest_parts = parse_version(latest);

    for i in 0..current_parts.len().max(latest_parts.len()) {
        let c = current_parts.get(i).copied().unwrap_or(0);
        let l = latest_parts.get(i).copied().unwrap_or(0);

        if c > l {
            return false;
        } else if c < l {
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_has_new_release() {
        assert!(has_new_release("2.2.3", "2.2.4"));
        assert!(has_new_release("2.2.3", "2.3.0"));
        assert!(has_new_release("2.2.3", "3.0.0"));
        assert!(!has_new_release("2.2.3", "2.2.3"));
        assert!(!has_new_release("2.2.3", "2.2.2"));
        assert!(!has_new_release("2.2.3", "2.1.0"));
        assert!(has_new_release("2.2.3", "v2.2.4"));
    }
}
