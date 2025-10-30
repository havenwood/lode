//! Platform detection and compatibility
//!
//! Detects the current platform in `RubyGems` format (e.g., "arm64-darwin",
//! "x86_64-linux") and checks gem platform compatibility.

use std::env;
use std::process::Command;
use std::sync::LazyLock;

/// Cached platform detection (computed once, reused throughout execution)
static CURRENT_PLATFORM: LazyLock<String> = LazyLock::new(detect_platform_impl);

/// Detect the current platform in `RubyGems` format
///
/// Examples: "ruby", "x86_64-darwin", "arm64-darwin", "x86_64-linux"
///
/// This function uses a cached result - the platform is detected once on first call
/// and the result is reused for all subsequent calls (zero-cost after first call).
#[must_use]
pub fn detect_current_platform() -> String {
    CURRENT_PLATFORM.clone()
}

/// Internal implementation of platform detection (called once by `LazyLock`)
fn detect_platform_impl() -> String {
    detect_via_ruby().unwrap_or_else(detect_via_rust)
}

fn detect_via_ruby() -> Option<String> {
    let output = Command::new("ruby")
        .args(["-e", "require 'rbconfig'; puts RbConfig::CONFIG['arch']"])
        .output()
        .ok()?;

    output.status.success().then_some(())?;

    let platform = String::from_utf8(output.stdout).ok()?.trim().to_string();

    (!platform.is_empty()).then_some(platform)
}

fn detect_via_rust() -> String {
    // Map Rust's GOARCH/GOOS to RubyGems platform strings
    let arch = match env::consts::ARCH {
        "x86_64" => "x86_64",
        "aarch64" => "arm64",
        "arm" => "arm",
        "x86" => "x86",
        _ => env::consts::ARCH,
    };

    let os = match env::consts::OS {
        "macos" => "darwin",
        "linux" => "linux",
        "windows" => "mingw32",
        _ => env::consts::OS,
    };

    format!("{arch}-{os}")
}

/// Check if a gem platform matches the current platform
///
/// Handles platform variants like "arm64-darwin-23" matching "arm64-darwin"
#[must_use]
pub fn platform_matches(gem_platform: &Option<String>, current_platform: &str) -> bool {
    let Some(platform) = gem_platform else {
        return true;
    }; // Pure Ruby gem, always compatible

    // Exact match or pure Ruby
    if platform == current_platform || platform == "ruby" {
        return true;
    }

    // Platform variants - compare arch and OS components
    // Examples: arm64-darwin-24 matches arm64-darwin
    //           x86_64-linux-gnu matches x86_64-linux
    let gem_parts: Vec<&str> = platform.split('-').collect();
    let current_parts: Vec<&str> = current_platform.split('-').collect();

    gem_parts.len() >= 2
        && current_parts.len() >= 2
        && gem_parts.first() == current_parts.first()
        && gem_parts.get(1) == current_parts.get(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platform_matches_exact() {
        let current = "arm64-darwin";
        assert!(platform_matches(&Some("arm64-darwin".to_string()), current));
        assert!(!platform_matches(
            &Some("x86_64-linux".to_string()),
            current
        ));
    }

    #[test]
    fn platform_matches_variant() {
        let current = "arm64-darwin";
        assert!(platform_matches(
            &Some("arm64-darwin-23".to_string()),
            current
        ));
        assert!(platform_matches(
            &Some("arm64-darwin-24".to_string()),
            current
        ));
    }

    #[test]
    fn platform_matches_pure_ruby() {
        let current = "x86_64-linux";
        assert!(platform_matches(&None, current));
        assert!(platform_matches(&Some("ruby".to_string()), current));
    }

    #[test]
    fn detect_platform() {
        let platform = detect_current_platform();
        assert!(!platform.is_empty());
        assert!(platform.contains('-') || platform == "ruby");
    }
}
