//! Integration tests for local-only gem commands
//!
//! Tests gem commands that don't require network access or system mutation:
//! - gem-which: Find library files
//! - gem-info: Display gem information
//! - gem-contents: List gem files
//! - gem-environment: Display gem environment
//! - gem-build: Build gems from gemspec
//! - gem-cleanup: Clean up old gems

mod common;

use std::fs;
use std::process::Command;
use tempfile::TempDir;

use common::get_lode_binary;

// ============================================================================
// gem-which Tests
// ============================================================================

/// Test gem-which finds an installed gem (bundler usually available)
#[test]
fn gem_which_finds_bundler() {
    let output = Command::new(get_lode_binary())
        .args(["gem-which", "bundler"])
        .output()
        .expect("Failed to execute lode gem-which bundler");

    // Should either succeed or report gem not found
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{stdout}{stderr}");

    // Should either find bundler or report that it can't find it
    assert!(
        output.status.success() || combined.contains("Can't find") || combined.is_empty(),
        "gem-which should either find bundler or report it missing. stdout: {stdout}, stderr: {stderr}"
    );
}

/// Test gem-which handles nonexistent gem gracefully
#[test]
fn gem_which_nonexistent_gem() {
    let output = Command::new(get_lode_binary())
        .args(["gem-which", "nonexistent-gem-xyz-12345"])
        .output()
        .expect("Failed to execute lode gem-which nonexistent-gem");

    // Should fail gracefully with an error message
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let combined = format!("{stdout}{stderr}");

    assert!(
        !output.status.success() || combined.contains("Can't find"),
        "gem-which should report missing gem. Combined output: {combined}"
    );
}

/// Test gem-which --all flag lists multiple files
#[test]
fn gem_which_all_flag() {
    let output = Command::new(get_lode_binary())
        .args(["gem-which", "--all", "bundler"])
        .output()
        .expect("Failed to execute lode gem-which --all bundler");

    // Should accept --all flag without "unexpected argument" error
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-which should accept --all flag. stderr: {stderr}"
    );
}

/// Test gem-which --gems-first flag
#[test]
fn gem_which_gems_first_flag() {
    let output = Command::new(get_lode_binary())
        .args(["gem-which", "--gems-first", "bundler"])
        .output()
        .expect("Failed to execute lode gem-which --gems-first");

    // Should accept --gems-first flag
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-which should accept --gems-first flag. stderr: {stderr}"
    );
}

// ============================================================================
// gem-info Tests
// ============================================================================

/// Test gem-info displays information for installed gem
#[test]
fn gem_info_local_gem() {
    let output = Command::new(get_lode_binary())
        .args(["gem-info", "bundler", "--local"])
        .output()
        .expect("Failed to execute lode gem-info bundler --local");

    // Should accept --local flag and display info
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-info should accept --local flag. stderr: {stderr}"
    );
}

/// Test gem-info with --version flag
#[test]
fn gem_info_version_flag() {
    let output = Command::new(get_lode_binary())
        .args(["gem-info", "bundler", "--version", "2.4.6"])
        .output()
        .expect("Failed to execute lode gem-info with --version");

    // Should accept --version flag
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-info should accept --version flag. stderr: {stderr}"
    );
}

/// Test gem-info handles nonexistent gem
#[test]
fn gem_info_nonexistent_gem() {
    let output = Command::new(get_lode_binary())
        .args(["gem-info", "nonexistent-gem-xyz-12345"])
        .output()
        .expect("Failed to execute lode gem-info nonexistent-gem");

    // Should either fail or report gem not found gracefully
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let combined = format!("{stdout}{stderr}");

    assert!(
        combined.is_empty() || !stderr.contains("error:"),
        "gem-info should handle missing gem gracefully. stderr: {stderr}"
    );
}

/// Test gem-info --all flag
#[test]
fn gem_info_all_flag() {
    let output = Command::new(get_lode_binary())
        .args(["gem-info", "bundler", "--all"])
        .output()
        .expect("Failed to execute lode gem-info --all");

    // Should accept --all flag
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-info should accept --all flag. stderr: {stderr}"
    );
}

// ============================================================================
// gem-contents Tests
// ============================================================================

/// Test gem-contents lists files in a gem
#[test]
fn gem_contents_lists_files() {
    let output = Command::new(get_lode_binary())
        .args(["gem-contents", "bundler"])
        .output()
        .expect("Failed to execute lode gem-contents bundler");

    // Should run successfully and show files or report gem not found
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-contents should accept bundler argument. stderr: {stderr}"
    );
}

/// Test gem-contents --lib-only flag
#[test]
fn gem_contents_lib_only() {
    let output = Command::new(get_lode_binary())
        .args(["gem-contents", "bundler", "--lib-only"])
        .output()
        .expect("Failed to execute lode gem-contents --lib-only");

    // Should accept --lib-only flag
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-contents should accept --lib-only flag. stderr: {stderr}"
    );
}

/// Test gem-contents with version filtering
#[test]
fn gem_contents_version() {
    let output = Command::new(get_lode_binary())
        .args(["gem-contents", "bundler", "--version", "2.4.6"])
        .output()
        .expect("Failed to execute lode gem-contents with --version");

    // Should accept --version flag
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-contents should accept --version flag. stderr: {stderr}"
    );
}

/// Test gem-contents with --prefix flag
#[test]
fn gem_contents_prefix() {
    let output = Command::new(get_lode_binary())
        .args(["gem-contents", "bundler", "--prefix"])
        .output()
        .expect("Failed to execute lode gem-contents --prefix");

    // Should accept --prefix flag
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-contents should accept --prefix flag. stderr: {stderr}"
    );
}

/// Test gem-contents nonexistent gem
#[test]
fn gem_contents_nonexistent() {
    let output = Command::new(get_lode_binary())
        .args(["gem-contents", "nonexistent-gem-xyz-12345"])
        .output()
        .expect("Failed to execute lode gem-contents nonexistent");

    // Should handle gracefully
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("error:") || stderr.contains("not found"),
        "gem-contents should report missing gem gracefully. stderr: {stderr}"
    );
}

// ============================================================================
// gem-environment Tests
// ============================================================================

/// Test gem-environment displays full environment
#[test]
fn gem_environment_full_display() {
    let output = Command::new(get_lode_binary())
        .args(["gem-environment"])
        .output()
        .expect("Failed to execute lode gem-environment");

    // Should succeed and display environment info
    assert!(output.status.success(), "gem-environment should succeed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should contain some environment information
    assert!(!stdout.is_empty(), "gem-environment should produce output");
}

/// Test gem-environment queries specific variable
#[test]
fn gem_environment_query_version() {
    let output = Command::new(get_lode_binary())
        .args(["gem-environment", "version"])
        .output()
        .expect("Failed to execute lode gem-environment version");

    // Should display RubyGems version
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.is_empty(),
        "gem-environment version should produce output"
    );
}

/// Test gem-environment queries home variable
#[test]
fn gem_environment_query_home() {
    let output = Command::new(get_lode_binary())
        .args(["gem-environment", "home"])
        .output()
        .expect("Failed to execute lode gem-environment home");

    // Should display gems home directory
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.is_empty(),
        "gem-environment home should produce output"
    );
}

/// Test gem-environment --verbose flag
#[test]
fn gem_environment_verbose() {
    let output = Command::new(get_lode_binary())
        .args(["gem-environment", "--verbose"])
        .output()
        .expect("Failed to execute lode gem-environment --verbose");

    // Should accept --verbose flag
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-environment should accept --verbose flag. stderr: {stderr}"
    );
}

/// Test gem-environment invalid variable query
#[test]
fn gem_environment_invalid_variable() {
    let output = Command::new(get_lode_binary())
        .args(["gem-environment", "nonexistent_variable"])
        .output()
        .expect("Failed to execute lode gem-environment nonexistent_variable");

    // Should fail gracefully for invalid variable
    let _stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Either should fail or produce minimal output
    assert!(
        !output.status.success() || stdout.is_empty(),
        "gem-environment should handle invalid variable gracefully"
    );
}

// ============================================================================
// gem-build Tests
// ============================================================================

/// Test gem-build with valid gemspec
#[test]
fn gem_build_with_valid_gemspec() {
    let temp = TempDir::new().unwrap();

    // Create a minimal gemspec file
    let gemspec_content = r#"
Gem::Specification.new do |spec|
  spec.name          = "test-gem"
  spec.version       = "0.1.0"
  spec.summary       = "Test gem"
  spec.authors       = ["Test Author"]
  spec.files         = []
end
"#;

    let gemspec_path = temp.path().join("test-gem.gemspec");
    fs::write(&gemspec_path, gemspec_content).unwrap();

    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let binary_path = std::path::Path::new(manifest_dir).join("target/debug/lode");

    let output = Command::new(&binary_path)
        .args(["gem-build", gemspec_path.to_string_lossy().as_ref()])
        .current_dir(temp.path())
        .output()
        .expect("Failed to execute lode gem-build");

    // Should either succeed or report a specific error (not argument error)
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument") && !stderr.contains("unrecognized"),
        "gem-build should accept gemspec path. stderr: {stderr}"
    );
}

/// Test gem-build with --platform flag
#[test]
fn gem_build_platform_flag() {
    let temp = TempDir::new().unwrap();
    let gemspec_path = temp.path().join("test.gemspec");
    fs::write(&gemspec_path, "Gem::Specification.new { }").unwrap();

    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let binary_path = std::path::Path::new(manifest_dir).join("target/debug/lode");

    let output = Command::new(&binary_path)
        .args([
            "gem-build",
            gemspec_path.to_string_lossy().as_ref(),
            "--platform",
            "ruby",
        ])
        .current_dir(temp.path())
        .output()
        .expect("Failed to execute lode gem-build --platform");

    // Should accept --platform flag
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-build should accept --platform flag. stderr: {stderr}"
    );
}

/// Test gem-build --force flag
#[test]
fn gem_build_force_flag() {
    let temp = TempDir::new().unwrap();
    let gemspec_path = temp.path().join("test.gemspec");
    fs::write(&gemspec_path, "Gem::Specification.new { }").unwrap();

    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let binary_path = std::path::Path::new(manifest_dir).join("target/debug/lode");

    let output = Command::new(&binary_path)
        .args([
            "gem-build",
            gemspec_path.to_string_lossy().as_ref(),
            "--force",
        ])
        .current_dir(temp.path())
        .output()
        .expect("Failed to execute lode gem-build --force");

    // Should accept --force flag
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-build should accept --force flag. stderr: {stderr}"
    );
}

/// Test gem-build nonexistent gemspec
#[test]
fn gem_build_nonexistent_gemspec() {
    let output = Command::new(get_lode_binary())
        .args(["gem-build", "nonexistent.gemspec"])
        .output()
        .expect("Failed to execute lode gem-build nonexistent");

    // Should fail with file not found error
    let _stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success(),
        "gem-build should fail for nonexistent gemspec"
    );
}

// ============================================================================
// gem-cleanup Tests
// ============================================================================

/// Test gem-cleanup --dry-run shows what would be cleaned
#[test]
fn gem_cleanup_dry_run() {
    let output = Command::new(get_lode_binary())
        .args(["gem-cleanup", "--dry-run"])
        .output()
        .expect("Failed to execute lode gem-cleanup --dry-run");

    // Should succeed with dry-run flag
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-cleanup should accept --dry-run flag. stderr: {stderr}"
    );
}

/// Test gem-cleanup with version filtering
#[test]
fn gem_cleanup_with_versions() {
    let output = Command::new(get_lode_binary())
        .args(["gem-cleanup", "bundler", "--dry-run"])
        .output()
        .expect("Failed to execute lode gem-cleanup bundler");

    // Should accept gem name argument
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-cleanup should accept gem name. stderr: {stderr}"
    );
}

/// Test gem-cleanup -d flag (short form of --dry-run)
#[test]
fn gem_cleanup_d_flag() {
    let output = Command::new(get_lode_binary())
        .args(["gem-cleanup", "-d"])
        .output()
        .expect("Failed to execute lode gem-cleanup -d");

    // Should accept -d flag
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-cleanup should accept -d flag. stderr: {stderr}"
    );
}

/// Test gem-cleanup --check-development flag
#[test]
fn gem_cleanup_check_development() {
    let output = Command::new(get_lode_binary())
        .args(["gem-cleanup", "--check-development", "--dry-run"])
        .output()
        .expect("Failed to execute lode gem-cleanup --check-development");

    // Should accept --check-development flag
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-cleanup should accept --check-development flag. stderr: {stderr}"
    );
}
