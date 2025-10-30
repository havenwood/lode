//! Platform command
//!
//! Display platform and system information

use anyhow::Result;
use lode::{detect_current_platform, detect_engine};
use std::process::Command;

/// Display platform compatibility information
#[allow(
    clippy::unnecessary_wraps,
    reason = "Maintains consistent API with other commands"
)]
pub(crate) fn run(ruby_only: bool) -> Result<()> {
    // If --ruby flag is set, only show Ruby version
    if ruby_only {
        if let Some(version) = detect_ruby_version() {
            println!("{version}");
        } else {
            eprintln!("Error: Ruby not available");
            std::process::exit(1);
        }
        return Ok(());
    }

    // Detect current platform
    let platform = detect_current_platform();
    let engine = detect_engine();

    // Try to detect Ruby version if available
    let ruby_version = detect_ruby_version();

    println!("Platform Information:");
    println!();
    println!("  Platform:     {platform}");
    println!("  Ruby Engine:  {engine}");

    if let Some(version) = ruby_version {
        println!("  Ruby Version: {version}");
    } else {
        println!("  Ruby Version: (not detected - Ruby not available)");
    }

    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;
    let family = std::env::consts::FAMILY;

    println!();
    println!("System Information:");
    println!("  OS:           {os}");
    println!("  Architecture: {arch}");
    println!("  Family:       {family}");

    Ok(())
}

/// Detect Ruby version from system ruby command
fn detect_ruby_version() -> Option<String> {
    let output = Command::new("ruby").args(["-v"]).output().ok()?;

    if output.status.success() {
        let version_output = String::from_utf8_lossy(&output.stdout);
        // Parse "ruby 3.4.0p0 (2024-12-25 revision...) [arm64-darwin24]"
        if let Some(version_part) = version_output.split_whitespace().nth(1) {
            return Some(version_part.to_string());
        }
    }

    None
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "Tests can panic")]
mod tests {
    use super::*;

    #[test]
    fn platform_command() {
        let result = run(false);
        assert!(result.is_ok());
    }

    #[test]
    fn platform_ruby_only() {
        let result = run(true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_detect_ruby_version() {
        // This will return None if Ruby is not installed
        // which is fine for testing purposes
        let version = detect_ruby_version();
        // Either Some(version) or None is acceptable
        if let Some(v) = version {
            // If we get a version, it should not be empty
            assert!(!v.is_empty());
            // Should look like a version number
            assert!(v.contains('.'));
        }
    }
}
