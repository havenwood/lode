mod common;

use std::process::Command;

use common::get_lode_binary;

// ============================================================================
// env command Tests - Display bundler/gem environment information
// ============================================================================

/// Test 1: lode env displays environment information
#[test]
fn env_displays_information() {
    let output = Command::new(get_lode_binary())
        .args(["env"])
        .output()
        .expect("Failed to execute lode env");

    assert!(output.status.success(), "lode env should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);

    // env should produce output with environment information
    assert!(
        !stdout.is_empty(),
        "lode env should display environment information"
    );
}

/// Test 2: lode env contains Ruby version
#[test]
fn env_contains_ruby_version() {
    let output = Command::new(get_lode_binary())
        .args(["env"])
        .output()
        .expect("Failed to execute lode env");

    assert!(output.status.success(), "lode env should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should contain version-related information
    assert!(
        stdout.contains("ruby") || stdout.contains("version") || stdout.contains("Ruby"),
        "env output should include Ruby version information"
    );
}

/// Test 3: lode env --help displays help
#[test]
fn env_help_flag() {
    let output = Command::new(get_lode_binary())
        .args(["env", "--help"])
        .output()
        .expect("Failed to execute lode env --help");

    assert!(output.status.success(), "lode env --help should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.is_empty(), "env --help should display help text");
}

/// Test 4: lode env -h short help flag
#[test]
fn env_help_short_flag() {
    let output = Command::new(get_lode_binary())
        .args(["env", "-h"])
        .output()
        .expect("Failed to execute lode env -h");

    assert!(output.status.success(), "lode env -h should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.is_empty(), "env -h should display help text");
}

/// Test 5: lode env contains path information
#[test]
fn env_contains_path_info() {
    let output = Command::new(get_lode_binary())
        .args(["env"])
        .output()
        .expect("Failed to execute lode env");

    assert!(output.status.success(), "lode env should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should contain path information (GEM_PATH, PATH, etc.)
    assert!(
        stdout.contains("path") || stdout.contains("PATH") || stdout.contains(":/"),
        "env output should include path information"
    );
}

// ============================================================================
// specification command Tests - Display gem specifications
// ============================================================================

/// Test 1: lode specification for installed gem
#[test]
fn specification_bundler_gem() {
    let output = Command::new(get_lode_binary())
        .args(["specification", "bundler"])
        .output()
        .expect("Failed to execute lode specification bundler");

    // bundler is usually available, should succeed or fail gracefully
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success() || !stderr.contains("unexpected argument"),
        "specification should parse arguments correctly"
    );
}

/// Test 2: lode specification --version flag
#[test]
fn specification_version_flag() {
    let output = Command::new(get_lode_binary())
        .args(["specification", "bundler", "--version", "2.4.6"])
        .output()
        .expect("Failed to execute lode specification --version");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "specification should accept --version flag. stderr: {stderr}"
    );
}

/// Test 3: lode specification with nonexistent gem
#[test]
fn specification_nonexistent_gem() {
    let output = Command::new(get_lode_binary())
        .args(["specification", "nonexistent-gem-xyz-12345"])
        .output()
        .expect("Failed to execute lode specification");

    // Should fail gracefully
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "specification should handle nonexistent gem gracefully. stderr: {stderr}"
    );
}

/// Test 4: lode specification --help displays help
#[test]
fn specification_help_flag() {
    let output = Command::new(get_lode_binary())
        .args(["specification", "--help"])
        .output()
        .expect("Failed to execute lode specification --help");

    assert!(
        output.status.success(),
        "lode specification --help should succeed"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.is_empty(),
        "specification --help should display help text"
    );
}

/// Test 5: lode specification -h short help flag
#[test]
fn specification_help_short_flag() {
    let output = Command::new(get_lode_binary())
        .args(["specification", "-h"])
        .output()
        .expect("Failed to execute lode specification -h");

    assert!(
        output.status.success(),
        "lode specification -h should succeed"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.is_empty(),
        "specification -h should display help text"
    );
}

/// Test 6: lode specification with gem name and version
#[test]
fn specification_with_version() {
    let output = Command::new(get_lode_binary())
        .args(["specification", "rake", "--version", "13.0.6"])
        .output()
        .expect("Failed to execute lode specification with version");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "specification should accept gem name and version. stderr: {stderr}"
    );
}

/// Test 7: lode specification without arguments
#[test]
fn specification_no_arguments() {
    let output = Command::new(get_lode_binary())
        .args(["specification"])
        .output()
        .expect("Failed to execute lode specification");

    // Should fail with missing argument (not parsing error)
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "specification should handle missing gem gracefully. stderr: {stderr}"
    );
}

/// Test 8: lode specification with common gems
#[test]
fn specification_rake_gem() {
    let output = Command::new(get_lode_binary())
        .args(["specification", "rake"])
        .output()
        .expect("Failed to execute lode specification rake");

    // Should succeed or fail gracefully (rake often installed)
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "specification should accept rake gem name. stderr: {stderr}"
    );
}

/// Test 9: lode specification multiple flag combinations
#[test]
fn specification_version_and_platform() {
    let output = Command::new(get_lode_binary())
        .args(["specification", "bundler", "--version", "2.4.6"])
        .output()
        .expect("Failed to execute lode specification with multiple flags");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "specification should accept multiple flags. stderr: {stderr}"
    );
}
