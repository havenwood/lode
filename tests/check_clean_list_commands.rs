mod common;

use std::fs;
use std::process::Command;
use tempfile::TempDir;

use common::get_lode_binary;

// ===== CHECK COMMAND TESTS =====

/// Test 1: Check with --gemfile flag
#[test]
fn check_custom_gemfile() {
    let temp = TempDir::new().unwrap();
    let gemfile = temp.path().join("Gemfile.custom");
    fs::write(&gemfile, "source 'https://rubygems.org'\ngem 'rake'\n").unwrap();

    let output = Command::new(get_lode_binary())
        .args(["check", "--gemfile", gemfile.to_string_lossy().as_ref()])
        .output()
        .expect("Failed to execute lode check --gemfile");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "check --gemfile should be accepted. stderr: {stderr}"
    );
}

/// Test 2: Check with --dry-run flag
#[test]
fn check_dry_run() {
    let temp = TempDir::new().unwrap();
    let gemfile = temp.path().join("Gemfile");
    fs::write(&gemfile, "source 'https://rubygems.org'\ngem 'rake'\n").unwrap();

    let output = Command::new(get_lode_binary())
        .args([
            "check",
            "--gemfile",
            gemfile.to_string_lossy().as_ref(),
            "--dry-run",
        ])
        .output()
        .expect("Failed to execute lode check --dry-run");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "check --dry-run should be accepted. stderr: {stderr}"
    );
}

/// Test 3: Check --help shows all flags
#[test]
fn check_help() {
    let output = Command::new(get_lode_binary())
        .args(["check", "--help"])
        .output()
        .expect("Failed to execute lode check --help");

    assert!(output.status.success(), "check --help should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains("--gemfile"),
        "help should document --gemfile"
    );
    assert!(
        stdout.contains("--dry-run"),
        "help should document --dry-run"
    );
}

/// Test 4: Check without Gemfile should error gracefully
#[test]
fn check_no_gemfile() {
    let temp = TempDir::new().unwrap();
    let nonexistent = temp.path().join("nonexistent");

    let output = Command::new(get_lode_binary())
        .args(["check", "--gemfile", nonexistent.to_string_lossy().as_ref()])
        .output()
        .expect("Failed to execute lode check");

    assert!(
        !output.status.success(),
        "check should fail when Gemfile doesn't exist"
    );
}

// ===== CLEAN COMMAND TESTS =====

/// Test 5: Clean with --vendor flag
#[test]
fn clean_vendor() {
    let temp = TempDir::new().unwrap();
    let vendor_dir = temp.path().join("vendor/cache");
    fs::create_dir_all(&vendor_dir).unwrap();

    let output = Command::new(get_lode_binary())
        .args(["clean", "--vendor", vendor_dir.to_string_lossy().as_ref()])
        .output()
        .expect("Failed to execute lode clean --vendor");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "clean --vendor should be accepted. stderr: {stderr}"
    );
}

/// Test 6: Clean with --dry-run flag
#[test]
fn clean_dry_run() {
    let temp = TempDir::new().unwrap();
    let vendor_dir = temp.path().join("vendor");
    fs::create_dir_all(&vendor_dir).unwrap();

    let output = Command::new(get_lode_binary())
        .args([
            "clean",
            "--vendor",
            vendor_dir.to_string_lossy().as_ref(),
            "--dry-run",
        ])
        .output()
        .expect("Failed to execute lode clean --dry-run");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "clean --dry-run should be accepted. stderr: {stderr}"
    );
}

/// Test 7: Clean with --force flag
#[test]
fn clean_force() {
    let temp = TempDir::new().unwrap();
    let vendor_dir = temp.path().join("vendor");
    fs::create_dir_all(&vendor_dir).unwrap();

    let output = Command::new(get_lode_binary())
        .args([
            "clean",
            "--vendor",
            vendor_dir.to_string_lossy().as_ref(),
            "--force",
        ])
        .output()
        .expect("Failed to execute lode clean --force");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "clean --force should be accepted. stderr: {stderr}"
    );
}

/// Test 8: Clean --help shows all flags
#[test]
fn clean_help() {
    let output = Command::new(get_lode_binary())
        .args(["clean", "--help"])
        .output()
        .expect("Failed to execute lode clean --help");

    assert!(output.status.success(), "clean --help should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("--vendor"), "help should document --vendor");
    assert!(
        stdout.contains("--dry-run"),
        "help should document --dry-run"
    );
    assert!(stdout.contains("--force"), "help should document --force");
}

/// Test 9: Clean with all flags combined
#[test]
fn clean_all_flags() {
    let temp = TempDir::new().unwrap();
    let vendor_dir = temp.path().join("vendor");
    fs::create_dir_all(&vendor_dir).unwrap();

    let output = Command::new(get_lode_binary())
        .args([
            "clean",
            "--vendor",
            vendor_dir.to_string_lossy().as_ref(),
            "--dry-run",
            "--force",
        ])
        .output()
        .expect("Failed to execute lode clean with all flags");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "clean with all flags should be accepted. stderr: {stderr}"
    );
}

// ===== LIST COMMAND TESTS =====

/// Test 10: List default behavior
#[test]
fn list_default() {
    let output = Command::new(get_lode_binary())
        .args(["list"])
        .output()
        .expect("Failed to execute lode list");

    let stderr = String::from_utf8_lossy(&output.stderr);
    // May fail if no Gemfile.lock, but should not have unexpected argument error
    assert!(
        !stderr.contains("unexpected argument"),
        "list should be accepted. stderr: {stderr}"
    );
}

/// Test 11: List with --name-only flag
#[test]
fn list_name_only() {
    let output = Command::new(get_lode_binary())
        .args(["list", "--name-only"])
        .output()
        .expect("Failed to execute lode list --name-only");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "list --name-only should be accepted. stderr: {stderr}"
    );
}

/// Test 12: List with --paths flag
#[test]
fn list_paths() {
    let output = Command::new(get_lode_binary())
        .args(["list", "--paths"])
        .output()
        .expect("Failed to execute lode list --paths");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "list --paths should be accepted. stderr: {stderr}"
    );
}

/// Test 13: List with --only-group flag
#[test]
fn list_only_group() {
    let output = Command::new(get_lode_binary())
        .args(["list", "--only-group", "test"])
        .output()
        .expect("Failed to execute lode list --only-group");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "list --only-group should be accepted. stderr: {stderr}"
    );
}

/// Test 14: List with --without-group flag
#[test]
fn list_without_group() {
    let output = Command::new(get_lode_binary())
        .args(["list", "--without-group", "development"])
        .output()
        .expect("Failed to execute lode list --without-group");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "list --without-group should be accepted. stderr: {stderr}"
    );
}

/// Test 15: List --help shows all flags
#[test]
fn list_help() {
    let output = Command::new(get_lode_binary())
        .args(["list", "--help"])
        .output()
        .expect("Failed to execute lode list --help");

    assert!(output.status.success(), "list --help should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains("--name-only"),
        "help should document --name-only"
    );
    assert!(stdout.contains("--paths"), "help should document --paths");
    assert!(
        stdout.contains("--only-group"),
        "help should document --only-group"
    );
    assert!(
        stdout.contains("--without-group"),
        "help should document --without-group"
    );
}

/// Test 16: List with multiple flags
#[test]
fn list_combined_flags() {
    let output = Command::new(get_lode_binary())
        .args(["list", "--name-only", "--only-group", "test"])
        .output()
        .expect("Failed to execute lode list with combined flags");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "list with combined flags should be accepted. stderr: {stderr}"
    );
}

// ===== OUTDATED COMMAND TESTS =====

/// Test 17: Outdated default behavior
#[test]
fn outdated_default() {
    let output = Command::new(get_lode_binary())
        .args(["outdated"])
        .output()
        .expect("Failed to execute lode outdated");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "outdated should be accepted. stderr: {stderr}"
    );
}

/// Test 18: Outdated with --major flag
#[test]
fn outdated_major() {
    let output = Command::new(get_lode_binary())
        .args(["outdated", "--major"])
        .output()
        .expect("Failed to execute lode outdated --major");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "outdated --major should be accepted. stderr: {stderr}"
    );
}

/// Test 19: Outdated with --minor flag
#[test]
fn outdated_minor() {
    let output = Command::new(get_lode_binary())
        .args(["outdated", "--minor"])
        .output()
        .expect("Failed to execute lode outdated --minor");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "outdated --minor should be accepted. stderr: {stderr}"
    );
}

/// Test 20: Outdated with --patch flag
#[test]
fn outdated_patch() {
    let output = Command::new(get_lode_binary())
        .args(["outdated", "--patch"])
        .output()
        .expect("Failed to execute lode outdated --patch");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "outdated --patch should be accepted. stderr: {stderr}"
    );
}

/// Test 21: Outdated with --pre flag
#[test]
fn outdated_pre() {
    let output = Command::new(get_lode_binary())
        .args(["outdated", "--pre"])
        .output()
        .expect("Failed to execute lode outdated --pre");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "outdated --pre should be accepted. stderr: {stderr}"
    );
}

/// Test 22: Outdated with --group flag
#[test]
fn outdated_group() {
    let output = Command::new(get_lode_binary())
        .args(["outdated", "--group", "test"])
        .output()
        .expect("Failed to execute lode outdated --group");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "outdated --group should be accepted. stderr: {stderr}"
    );
}

/// Test 23: Outdated with --parseable flag
#[test]
fn outdated_parseable() {
    let output = Command::new(get_lode_binary())
        .args(["outdated", "--parseable"])
        .output()
        .expect("Failed to execute lode outdated --parseable");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "outdated --parseable should be accepted. stderr: {stderr}"
    );
}

/// Test 24: Outdated --help shows all flags
#[test]
fn outdated_help() {
    let output = Command::new(get_lode_binary())
        .args(["outdated", "--help"])
        .output()
        .expect("Failed to execute lode outdated --help");

    assert!(output.status.success(), "outdated --help should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("--major"), "help should document --major");
    assert!(stdout.contains("--minor"), "help should document --minor");
    assert!(stdout.contains("--patch"), "help should document --patch");
    assert!(stdout.contains("--pre"), "help should document --pre");
    assert!(stdout.contains("--group"), "help should document --group");
    assert!(
        stdout.contains("--parseable"),
        "help should document --parseable"
    );
}

/// Test 25: Outdated with combined flags
#[test]
fn outdated_combined() {
    let output = Command::new(get_lode_binary())
        .args(["outdated", "--patch", "--parseable"])
        .output()
        .expect("Failed to execute lode outdated with combined flags");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "outdated with combined flags should be accepted. stderr: {stderr}"
    );
}
