//! Integration tests for system-level gem commands
//!
//! Tests gem commands that modify or interact with the system gem environment:
//! - gem-uninstall: Uninstall gems (system mutation)
//! - gem-rebuild: Rebuild installed gems (extension building)
//! - gem-pristine: Restore gems to pristine state (cache restoration)
//!
//! These tests use safe approaches:
//! - Flag acceptance verification (parsing correctness)
//! - --help output validation (functionality verification)
//! - Isolated testing (no real system gem modifications)

mod common;

use std::process::Command;

use common::get_lode_binary;

// ============================================================================
// gem-uninstall Tests - Uninstall gems
// ============================================================================

/// Test gem-uninstall accepts gem name
#[test]
fn gem_uninstall_gem_name() {
    let output = Command::new(get_lode_binary())
        .args(["gem-uninstall", "nonexistent-gem-xyz"])
        .output()
        .expect("Failed to execute lode gem-uninstall");

    // Should handle gracefully (gem doesn't exist, but flag is accepted)
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument") && !stderr.contains("unrecognized"),
        "gem-uninstall should accept gem name. stderr: {stderr}"
    );
}

/// Test gem-uninstall --all flag
#[test]
fn gem_uninstall_all_flag() {
    let output = Command::new(get_lode_binary())
        .args(["gem-uninstall", "nonexistent-gem", "--all"])
        .output()
        .expect("Failed to execute lode gem-uninstall --all");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-uninstall should accept --all flag. stderr: {stderr}"
    );
}

/// Test gem-uninstall --version flag
#[test]
fn gem_uninstall_version_flag() {
    let output = Command::new(get_lode_binary())
        .args(["gem-uninstall", "nonexistent-gem", "--version", "1.0.0"])
        .output()
        .expect("Failed to execute lode gem-uninstall --version");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-uninstall should accept --version flag. stderr: {stderr}"
    );
}

/// Test gem-uninstall --executables flag
#[test]
fn gem_uninstall_executables_flag() {
    let output = Command::new(get_lode_binary())
        .args(["gem-uninstall", "nonexistent-gem", "--executables"])
        .output()
        .expect("Failed to execute lode gem-uninstall --executables");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-uninstall should accept --executables flag. stderr: {stderr}"
    );
}

/// Test gem-uninstall --ignore-dependencies flag
#[test]
fn gem_uninstall_ignore_dependencies() {
    let output = Command::new(get_lode_binary())
        .args(["gem-uninstall", "nonexistent-gem", "--ignore-dependencies"])
        .output()
        .expect("Failed to execute lode gem-uninstall --ignore-dependencies");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-uninstall should accept --ignore-dependencies flag. stderr: {stderr}"
    );
}

/// Test gem-uninstall --force flag
#[test]
fn gem_uninstall_force_flag() {
    let output = Command::new(get_lode_binary())
        .args(["gem-uninstall", "nonexistent-gem", "--force"])
        .output()
        .expect("Failed to execute lode gem-uninstall --force");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-uninstall should accept --force flag. stderr: {stderr}"
    );
}

/// Test gem-uninstall --user-install flag
#[test]
fn gem_uninstall_user_install_flag() {
    let output = Command::new(get_lode_binary())
        .args(["gem-uninstall", "nonexistent-gem", "--user-install"])
        .output()
        .expect("Failed to execute lode gem-uninstall --user-install");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-uninstall should accept --user-install flag. stderr: {stderr}"
    );
}

/// Test gem-uninstall --help shows usage
#[test]
fn gem_uninstall_help() {
    let output = Command::new(get_lode_binary())
        .args(["gem-uninstall", "--help"])
        .output()
        .expect("Failed to execute lode gem-uninstall --help");

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should display help output
    assert!(
        !stdout.is_empty(),
        "gem-uninstall --help should display help text"
    );
}

// ============================================================================
// gem-rebuild Tests - Rebuild installed gems
// ============================================================================

/// Test gem-rebuild accepts gem names
#[test]
fn gem_rebuild_gem_name() {
    let output = Command::new(get_lode_binary())
        .args(["gem-rebuild", "nonexistent-gem"])
        .output()
        .expect("Failed to execute lode gem-rebuild");

    // Should handle gracefully (gem doesn't exist, but command is accepted)
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument") && !stderr.contains("unrecognized"),
        "gem-rebuild should accept gem name. stderr: {stderr}"
    );
}

/// Test gem-rebuild --verbose flag
#[test]
fn gem_rebuild_verbose_flag() {
    let output = Command::new(get_lode_binary())
        .args(["gem-rebuild", "nonexistent-gem", "--verbose"])
        .output()
        .expect("Failed to execute lode gem-rebuild --verbose");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-rebuild should accept --verbose flag. stderr: {stderr}"
    );
}

/// Test gem-rebuild --quiet flag
#[test]
fn gem_rebuild_quiet_flag() {
    let output = Command::new(get_lode_binary())
        .args(["gem-rebuild", "nonexistent-gem", "--quiet"])
        .output()
        .expect("Failed to execute lode gem-rebuild --quiet");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-rebuild should accept --quiet flag. stderr: {stderr}"
    );
}

/// Test gem-rebuild --help shows usage
#[test]
fn gem_rebuild_help() {
    let output = Command::new(get_lode_binary())
        .args(["gem-rebuild", "--help"])
        .output()
        .expect("Failed to execute lode gem-rebuild --help");

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should display help output
    assert!(
        !stdout.is_empty(),
        "gem-rebuild --help should display help text"
    );
}

// ============================================================================
// gem-pristine Tests - Restore gems to pristine state
// ============================================================================

/// Test gem-pristine accepts gem names
#[test]
fn gem_pristine_gem_name() {
    let output = Command::new(get_lode_binary())
        .args(["gem-pristine", "nonexistent-gem"])
        .output()
        .expect("Failed to execute lode gem-pristine");

    // Should handle gracefully
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-pristine should accept gem name. stderr: {stderr}"
    );
}

/// Test gem-pristine --all flag
#[test]
fn gem_pristine_all_flag() {
    let output = Command::new(get_lode_binary())
        .args(["gem-pristine", "--all"])
        .output()
        .expect("Failed to execute lode gem-pristine --all");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-pristine should accept --all flag. stderr: {stderr}"
    );
}

/// Test gem-pristine --skip flag
#[test]
fn gem_pristine_skip_flag() {
    let output = Command::new(get_lode_binary())
        .args(["gem-pristine", "--all", "--skip", "bundler"])
        .output()
        .expect("Failed to execute lode gem-pristine --skip");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-pristine should accept --skip flag. stderr: {stderr}"
    );
}

/// Test gem-pristine --extensions flag
#[test]
fn gem_pristine_extensions_flag() {
    let output = Command::new(get_lode_binary())
        .args(["gem-pristine", "nonexistent-gem", "--extensions"])
        .output()
        .expect("Failed to execute lode gem-pristine --extensions");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-pristine should accept --extensions flag. stderr: {stderr}"
    );
}

/// Test gem-pristine --only-missing-extensions flag
#[test]
fn gem_pristine_only_missing_extensions() {
    let output = Command::new(get_lode_binary())
        .args([
            "gem-pristine",
            "nonexistent-gem",
            "--only-missing-extensions",
        ])
        .output()
        .expect("Failed to execute lode gem-pristine --only-missing-extensions");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-pristine should accept --only-missing-extensions flag. stderr: {stderr}"
    );
}

/// Test gem-pristine --only-executables flag
#[test]
fn gem_pristine_only_executables() {
    let output = Command::new(get_lode_binary())
        .args(["gem-pristine", "nonexistent-gem", "--only-executables"])
        .output()
        .expect("Failed to execute lode gem-pristine --only-executables");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-pristine should accept --only-executables flag. stderr: {stderr}"
    );
}

/// Test gem-pristine --env-shebang flag
#[test]
fn gem_pristine_env_shebang() {
    let output = Command::new(get_lode_binary())
        .args(["gem-pristine", "nonexistent-gem", "--env-shebang"])
        .output()
        .expect("Failed to execute lode gem-pristine --env-shebang");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-pristine should accept --env-shebang flag. stderr: {stderr}"
    );
}

/// Test gem-pristine --help shows usage
#[test]
fn gem_pristine_help() {
    let output = Command::new(get_lode_binary())
        .args(["gem-pristine", "--help"])
        .output()
        .expect("Failed to execute lode gem-pristine --help");

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should display help output
    assert!(
        !stdout.is_empty(),
        "gem-pristine --help should display help text"
    );
}
