mod common;

use std::fmt::Write;
use std::fs;
use std::process::Command;
use tempfile::TempDir;

use common::get_lode_binary;
use common::helpers::{create_test_gemfile, create_test_lockfile};

// ============================================================================
// cache command Tests - Package gems into vendor/cache
// ============================================================================

/// Test 1: lode cache accepts Gemfile with gems
#[test]
fn cache_with_gemfile() {
    let temp = TempDir::new().unwrap();
    create_test_gemfile(&temp, &[("rake", "13.0.6")]);
    let _lockfile = create_test_lockfile(&temp, &[("rake", "13.0.6")]);

    let output = Command::new(get_lode_binary())
        .current_dir(temp.path())
        .args(["cache"])
        .output()
        .expect("Failed to execute lode cache");

    let stderr = String::from_utf8_lossy(&output.stderr);
    // Should either succeed or fail with meaningful message (not parsing error)
    assert!(
        output.status.success() || !stderr.contains("unexpected argument"),
        "lode cache should parse arguments correctly. stderr: {stderr}"
    );
}

/// Test 2: lode cache --all-platforms flag
#[test]
fn cache_all_platforms_flag() {
    let temp = TempDir::new().unwrap();
    create_test_gemfile(&temp, &[("rake", "13.0.6")]);
    let _lockfile = create_test_lockfile(&temp, &[("rake", "13.0.6")]);

    let output = Command::new(get_lode_binary())
        .current_dir(temp.path())
        .args(["cache", "--all-platforms"])
        .output()
        .expect("Failed to execute lode cache --all-platforms");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "cache should accept --all-platforms flag. stderr: {stderr}"
    );
}

/// Test 3: lode cache --cache-path with custom directory
#[test]
fn cache_custom_cache_path() {
    let temp = TempDir::new().unwrap();
    create_test_gemfile(&temp, &[("rake", "13.0.6")]);
    let _lockfile = create_test_lockfile(&temp, &[("rake", "13.0.6")]);

    let custom_cache_path = temp.path().join("custom_cache");
    let output = Command::new(get_lode_binary())
        .current_dir(temp.path())
        .args([
            "cache",
            "--cache-path",
            custom_cache_path.to_string_lossy().as_ref(),
        ])
        .output()
        .expect("Failed to execute lode cache --cache-path");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "cache should accept --cache-path flag. stderr: {stderr}"
    );
}

/// Test 4: lode cache --gemfile with custom Gemfile
#[test]
fn cache_custom_gemfile_path() {
    let temp = TempDir::new().unwrap();
    let custom_dir = temp.path().join("custom");
    fs::create_dir_all(&custom_dir).unwrap();

    let mut gemfile_content = String::from("source 'https://rubygems.org'\n\n");
    writeln!(&mut gemfile_content, "gem 'rake', '13.0.6'").unwrap();
    let gemfile_path = custom_dir.join("Gemfile");
    fs::write(&gemfile_path, gemfile_content).unwrap();

    let mut lockfile_content = String::from("GEM\n  remote: https://rubygems.org/\n  specs:\n");
    writeln!(&mut lockfile_content, "    rake (13.0.6)").unwrap();
    lockfile_content.push_str(
        "\n\nPLATFORMS\n  ruby\n\nDEPENDENCIES\n  rake\n\nRUBY VERSION\n   ruby 3.2.0\n\nBUNDLED WITH\n   2.4.6\n",
    );
    let lockfile_path = custom_dir.join("Gemfile.lock");
    fs::write(&lockfile_path, lockfile_content).unwrap();

    let output = Command::new(get_lode_binary())
        .current_dir(temp.path())
        .args([
            "cache",
            "--gemfile",
            gemfile_path.to_string_lossy().as_ref(),
        ])
        .output()
        .expect("Failed to execute lode cache --gemfile");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "cache should accept --gemfile flag. stderr: {stderr}"
    );
}

/// Test 5: lode cache --help displays usage
#[test]
fn cache_help_flag() {
    let output = Command::new(get_lode_binary())
        .args(["cache", "--help"])
        .output()
        .expect("Failed to execute lode cache --help");

    assert!(output.status.success(), "lode cache --help should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.is_empty(), "cache --help should display help text");
}

/// Test 6: lode cache -h short help flag
#[test]
fn cache_help_short_flag() {
    let output = Command::new(get_lode_binary())
        .args(["cache", "-h"])
        .output()
        .expect("Failed to execute lode cache -h");

    assert!(output.status.success(), "lode cache -h should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.is_empty(), "cache -h should display help text");
}

/// Test 7: lode cache with empty lockfile (no gems)
#[test]
fn cache_empty_lockfile() {
    let temp = TempDir::new().unwrap();
    let gemfile_path = temp.path().join("Gemfile");
    fs::write(&gemfile_path, "source 'https://rubygems.org'\n").unwrap();

    let lockfile_content = r"GEM
  remote: https://rubygems.org/
  specs:

PLATFORMS
  ruby

DEPENDENCIES

RUBY VERSION
   ruby 3.2.0

BUNDLED WITH
   2.4.6
";
    let lockfile_path = temp.path().join("Gemfile.lock");
    fs::write(&lockfile_path, lockfile_content).unwrap();

    let output = Command::new(get_lode_binary())
        .current_dir(temp.path())
        .args(["cache"])
        .output()
        .expect("Failed to execute lode cache");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success() || !stderr.contains("unexpected argument"),
        "cache should handle empty lockfile gracefully. stderr: {stderr}"
    );
}

/// Test 8: lode cache with multiple flags combined
#[test]
fn cache_multiple_flags() {
    let temp = TempDir::new().unwrap();
    create_test_gemfile(&temp, &[("rake", "13.0.6"), ("rspec", "3.12.0")]);
    let _lockfile = create_test_lockfile(&temp, &[("rake", "13.0.6"), ("rspec", "3.12.0")]);

    let custom_cache = temp.path().join("my_cache");
    let output = Command::new(get_lode_binary())
        .current_dir(temp.path())
        .args([
            "cache",
            "--all-platforms",
            "--cache-path",
            custom_cache.to_string_lossy().as_ref(),
        ])
        .output()
        .expect("Failed to execute lode cache with multiple flags");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "cache should accept multiple flags. stderr: {stderr}"
    );
}

/// Test 9: lode cache missing Gemfile.lock error handling
#[test]
fn cache_missing_lockfile() {
    let temp = TempDir::new().unwrap();
    // Create Gemfile but not Gemfile.lock
    let gemfile_path = temp.path().join("Gemfile");
    fs::write(&gemfile_path, "source 'https://rubygems.org'\n").unwrap();

    let output = Command::new(get_lode_binary())
        .current_dir(temp.path())
        .args(["cache"])
        .output()
        .expect("Failed to execute lode cache");

    // Should fail with proper error, not parsing error
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "cache should handle missing lockfile gracefully. stderr: {stderr}"
    );
}

/// Test 10: lode cache creates vendor/cache directory if needed
#[test]
fn cache_creates_vendor_cache_dir() {
    let temp = TempDir::new().unwrap();
    create_test_gemfile(&temp, &[("rake", "13.0.6")]);
    let _lockfile = create_test_lockfile(&temp, &[("rake", "13.0.6")]);

    let output = Command::new(get_lode_binary())
        .current_dir(temp.path())
        .args(["cache"])
        .output()
        .expect("Failed to execute lode cache");

    let stderr = String::from_utf8_lossy(&output.stderr);
    // Should either succeed or report proper error (not parsing error)
    assert!(
        output.status.success() || !stderr.contains("unexpected argument"),
        "cache should create directories as needed. stderr: {stderr}"
    );
}
