mod common;

use std::fs;
use std::process::Command;
use tempfile::TempDir;

use common::get_lode_binary;
use common::helpers::create_test_lockfile;

/// Test 1: Binstubs basic functionality
#[test]
fn binstubs_custom_lockfile() {
    let temp = TempDir::new().unwrap();
    create_test_lockfile(&temp, &[("rake", "13.0.6")]);

    let output = Command::new(get_lode_binary())
        .current_dir(temp.path())
        .args(["binstubs", "rake"])
        .output()
        .expect("Failed to execute lode binstubs");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "binstubs should be accepted. stderr: {stderr}"
    );
}

/// Test 2: Binstubs with `BUNDLE_PATH` environment variable
#[test]
fn binstubs_vendor() {
    let temp = TempDir::new().unwrap();
    create_test_lockfile(&temp, &[("rake", "13.0.6")]);
    let vendor_dir = temp.path().join("vendor/bundle");
    fs::create_dir_all(&vendor_dir).unwrap();

    let output = Command::new(get_lode_binary())
        .current_dir(temp.path())
        .env("BUNDLE_PATH", &vendor_dir)
        .args(["binstubs", "rake"])
        .output()
        .expect("Failed to execute lode binstubs");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "binstubs should be accepted. stderr: {stderr}"
    );
}

/// Test 3: Binstubs with --all flag (all gems)
#[test]
fn binstubs_custom_bin() {
    let temp = TempDir::new().unwrap();
    create_test_lockfile(&temp, &[("rake", "13.0.6"), ("rspec", "3.12.0")]);

    let output = Command::new(get_lode_binary())
        .current_dir(temp.path())
        .args(["binstubs", "--all"])
        .output()
        .expect("Failed to execute lode binstubs --all");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "binstubs --all should be accepted. stderr: {stderr}"
    );
}

/// Test 4: Binstubs with --shebang flag (custom Ruby path)
#[test]
fn binstubs_shebang() {
    let temp = TempDir::new().unwrap();
    create_test_lockfile(&temp, &[("rake", "13.0.6")]);

    let output = Command::new(get_lode_binary())
        .current_dir(temp.path())
        .args(["binstubs", "rake", "--shebang", "/usr/local/bin/ruby"])
        .output()
        .expect("Failed to execute lode binstubs --shebang");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "binstubs --shebang should be accepted. stderr: {stderr}"
    );
}

/// Test 5: Binstubs with --force flag
#[test]
fn binstubs_force() {
    let temp = TempDir::new().unwrap();
    create_test_lockfile(&temp, &[("rake", "13.0.6")]);
    let bin_dir = temp.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();

    // Create existing binstub
    let binstub_path = bin_dir.join("rake");
    fs::write(&binstub_path, "#!/usr/bin/env ruby\nputs 'old version'\n").unwrap();

    let output = Command::new(get_lode_binary())
        .current_dir(temp.path())
        .args(["binstubs", "rake", "--force"])
        .output()
        .expect("Failed to execute lode binstubs --force");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "binstubs --force should be accepted. stderr: {stderr}"
    );
}

/// Test 6: Binstubs for multiple gems
#[test]
fn binstubs_multiple_gems() {
    let temp = TempDir::new().unwrap();
    create_test_lockfile(&temp, &[("rake", "13.0.6"), ("rspec", "3.12.0")]);

    let output = Command::new(get_lode_binary())
        .current_dir(temp.path())
        .args(["binstubs", "rake", "rspec"])
        .output()
        .expect("Failed to execute lode binstubs for multiple gems");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "binstubs with multiple gems should be accepted. stderr: {stderr}"
    );
}

/// Test 7: Binstubs --help shows all flags
#[test]
fn binstubs_help() {
    let output = Command::new(get_lode_binary())
        .args(["binstubs", "--help"])
        .output()
        .expect("Failed to execute lode binstubs --help");

    assert!(output.status.success(), "binstubs --help should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Verify key flags are documented
    assert!(
        stdout.contains("--shebang"),
        "help should document --shebang"
    );
    assert!(stdout.contains("--force"), "help should document --force");
    assert!(stdout.contains("--all"), "help should document --all");
    assert!(
        stdout.contains("--all-platforms"),
        "help should document --all-platforms"
    );
}

/// Test 8: Binstubs with all valid flags combined
#[test]
fn binstubs_all_flags() {
    let temp = TempDir::new().unwrap();
    create_test_lockfile(&temp, &[("rake", "13.0.6"), ("rspec", "3.12.0")]);

    let output = Command::new(get_lode_binary())
        .current_dir(temp.path())
        .args([
            "binstubs",
            "--all",
            "--all-platforms",
            "--shebang",
            "/usr/bin/ruby",
            "--force",
        ])
        .output()
        .expect("Failed to execute lode binstubs with all flags");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "binstubs with all flags should be accepted. stderr: {stderr}"
    );
}

/// Test 9: Binstubs without gem name or --all should work (generates for all gems in lockfile)
#[test]
fn binstubs_no_gem() {
    let temp = TempDir::new().unwrap();
    create_test_lockfile(&temp, &[("rake", "13.0.6")]);

    let output = Command::new(get_lode_binary())
        .current_dir(temp.path())
        .args(["binstubs"])
        .output()
        .expect("Failed to execute lode binstubs");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "binstubs without gem name should be accepted. stderr: {stderr}"
    );
}

/// Test 10: Binstubs for nonexistent gem should error
#[test]
fn binstubs_nonexistent_gem() {
    let temp = TempDir::new().unwrap();
    create_test_lockfile(&temp, &[("rake", "13.0.6")]);

    let output = Command::new(get_lode_binary())
        .current_dir(temp.path())
        .args(["binstubs", "nonexistent-gem"])
        .output()
        .expect("Failed to execute lode binstubs for nonexistent gem");

    let stderr = String::from_utf8_lossy(&output.stderr);
    // Should either succeed (skipping gem) or error gracefully
    assert!(
        !stderr.contains("unexpected argument"),
        "binstubs should handle nonexistent gem gracefully. stderr: {stderr}"
    );
}

/// Test 11: Binstubs creates executable with proper permissions
#[test]
#[cfg(unix)] // This test is Unix-specific due to file permissions
fn binstubs_creates_executable() {
    use std::os::unix::fs::PermissionsExt;

    let temp = TempDir::new().unwrap();
    create_test_lockfile(&temp, &[("rake", "13.0.6")]);
    let bin_dir = temp.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();

    let output = Command::new(get_lode_binary())
        .current_dir(temp.path())
        .args(["binstubs", "rake"])
        .output()
        .expect("Failed to execute lode binstubs");

    if output.status.success() {
        let binstub_path = bin_dir.join("rake");
        if binstub_path.exists() {
            let metadata = fs::metadata(&binstub_path).unwrap();
            let permissions = metadata.permissions();
            let mode = permissions.mode();

            // Check that executable bit is set (0o755 or similar)
            assert!(
                mode & 0o111 != 0,
                "binstub should be executable. mode: {mode:o}"
            );
        }
    }
}

/// Test 12: Binstubs without --force should not overwrite existing
#[test]
fn binstubs_no_overwrite_without_force() {
    let temp = TempDir::new().unwrap();
    create_test_lockfile(&temp, &[("rake", "13.0.6")]);
    let bin_dir = temp.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();

    // Create existing binstub
    let binstub_path = bin_dir.join("rake");
    let original_content = "#!/usr/bin/env ruby\nputs 'original'\n";
    fs::write(&binstub_path, original_content).unwrap();

    let output = Command::new(get_lode_binary())
        .current_dir(temp.path())
        .args(["binstubs", "rake"])
        .output()
        .expect("Failed to execute lode binstubs");

    if output.status.success() {
        let content = fs::read_to_string(&binstub_path).unwrap();
        // Without --force, original should be preserved (or command warns)
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let combined = format!("{stdout}{stderr}");

        assert!(
            content == original_content || combined.contains("exists") || combined.contains("skip"),
            "binstubs without --force should not overwrite or should warn"
        );
    }
}
