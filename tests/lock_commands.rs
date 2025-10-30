mod common;

use std::fs;
use std::process::Command;
use tempfile::TempDir;

use common::get_lode_binary;
use common::helpers::{create_test_gemfile, create_test_lockfile};

/// Test 1: Default behavior - lode lock (no --update)
#[test]
fn lock_default_behavior() {
    let temp = TempDir::new().unwrap();
    let gemfile = create_test_gemfile(&temp, &[("rake", "13.0.6")]);

    let output = Command::new(get_lode_binary())
        .args(["lock", "--gemfile", &gemfile])
        .output()
        .expect("Failed to execute lode lock");

    assert!(output.status.success(), "lode lock should succeed");
    let lockfile_path = temp.path().join("Gemfile.lock");
    assert!(lockfile_path.exists(), "Gemfile.lock should be created");
}

/// Test 2: lode lock --update (no args) - should work now!
#[test]
fn lock_update_all_gems() {
    let temp = TempDir::new().unwrap();
    let gemfile = create_test_gemfile(&temp, &[("rake", "13.0.6")]);
    let lockfile = create_test_lockfile(&temp, &[("rake", "13.0.6")]);

    let output = Command::new(get_lode_binary())
        .args([
            "lock",
            "--gemfile",
            &gemfile,
            "--lockfile",
            &lockfile,
            "--update",
        ])
        .output()
        .expect("Failed to execute lode lock --update");

    assert!(
        output.status.success(),
        "lode lock --update (no args) should succeed. stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    // Just verify it succeeds - the main thing is the --update flag is now accepted without args
}

/// Test 3: lode lock --update with specific gem
#[test]
fn lock_update_specific_gem() {
    let temp = TempDir::new().unwrap();
    let gemfile = create_test_gemfile(&temp, &[("rake", "13.0.6"), ("rspec", "3.12.0")]);
    let lockfile = create_test_lockfile(&temp, &[("rake", "13.0.6"), ("rspec", "3.12.0")]);

    let output = Command::new(get_lode_binary())
        .args([
            "lock",
            "--gemfile",
            &gemfile,
            "--lockfile",
            &lockfile,
            "--update",
            "rake",
        ])
        .output()
        .expect("Failed to execute lode lock --update rake");

    assert!(
        output.status.success(),
        "lode lock --update rake should succeed"
    );
}

/// Test 4: lode lock --update with multiple gems
#[test]
fn lock_update_multiple_gems() {
    let temp = TempDir::new().unwrap();
    let gemfile = create_test_gemfile(
        &temp,
        &[("rake", "13.0.6"), ("rspec", "3.12.0"), ("rails", "7.0.4")],
    );
    let lockfile = create_test_lockfile(
        &temp,
        &[("rake", "13.0.6"), ("rspec", "3.12.0"), ("rails", "7.0.4")],
    );

    let output = Command::new(get_lode_binary())
        .args([
            "lock",
            "--gemfile",
            &gemfile,
            "--lockfile",
            &lockfile,
            "--update",
            "rake",
            "--update",
            "rspec",
        ])
        .output()
        .expect("Failed to execute lode lock --update rake rspec");

    assert!(
        output.status.success(),
        "lode lock --update with multiple gems should succeed"
    );
}

/// Test 5: lode lock --print (output to stdout)
#[test]
fn lock_print_to_stdout() {
    let temp = TempDir::new().unwrap();
    let gemfile = create_test_gemfile(&temp, &[("rake", "13.0.6")]);

    let output = Command::new(get_lode_binary())
        .args(["lock", "--gemfile", &gemfile, "--print"])
        .output()
        .expect("Failed to execute lode lock --print");

    assert!(output.status.success(), "lode lock --print should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("GEM") || stdout.contains("specs"),
        "Should output lockfile content"
    );
}

/// Test 6: lode lock --print should NOT create file
#[test]
fn lock_print_no_file_created() {
    let temp = TempDir::new().unwrap();
    let gemfile = create_test_gemfile(&temp, &[("rake", "13.0.6")]);
    let lockfile_path = temp.path().join("Gemfile.lock");

    assert!(!lockfile_path.exists(), "Lockfile should not exist yet");

    let output = Command::new(get_lode_binary())
        .args([
            "lock",
            "--gemfile",
            &gemfile,
            "--lockfile",
            lockfile_path.to_string_lossy().as_ref(),
            "--print",
        ])
        .output()
        .expect("Failed to execute lode lock --print");

    assert!(output.status.success());
    assert!(
        !lockfile_path.exists(),
        "Lockfile should NOT be created when using --print"
    );
}

/// Test 7: lode lock with custom lockfile path
#[test]
fn lock_custom_lockfile_path() {
    let temp = TempDir::new().unwrap();
    let custom_dir = temp.path().join("lockfiles");
    fs::create_dir_all(&custom_dir).unwrap();

    let gemfile = create_test_gemfile(&temp, &[("rake", "13.0.6")]);
    let custom_lockfile = custom_dir.join("gems.locked");

    let output = Command::new(get_lode_binary())
        .args([
            "lock",
            "--gemfile",
            &gemfile,
            "--lockfile",
            custom_lockfile.to_string_lossy().as_ref(),
        ])
        .output()
        .expect("Failed to execute lode lock with custom path");

    assert!(
        output.status.success(),
        "lode lock with custom path should succeed"
    );
    assert!(
        custom_lockfile.exists(),
        "Custom lockfile should be created"
    );
}

/// Test 8: lode lock with --local flag
#[test]
fn lock_local_mode() {
    let temp = TempDir::new().unwrap();
    let gemfile = create_test_gemfile(&temp, &[("rake", "13.0.6")]);

    let output = Command::new(get_lode_binary())
        .args(["lock", "--gemfile", &gemfile, "--local"])
        .output()
        .expect("Failed to execute lode lock --local");

    // May fail due to missing cache, but command should at least accept the flag
    // The main thing is it doesn't error about unknown flag
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success() || !stderr.contains("unexpected argument"),
        "Flag should be accepted without 'unexpected argument' error. stderr: {stderr}"
    );
}

/// Test 9: lode lock --verbose output
#[test]
fn lock_verbose_output() {
    let temp = TempDir::new().unwrap();
    let gemfile = create_test_gemfile(&temp, &[("rake", "13.0.6")]);

    let output = Command::new(get_lode_binary())
        .args(["lock", "--gemfile", &gemfile, "--verbose"])
        .output()
        .expect("Failed to execute lode lock --verbose");

    assert!(
        output.status.success(),
        "lode lock --verbose should succeed"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Resolving") || stdout.contains("Gemfile") || stdout.contains("gems"),
        "Verbose output should contain progress information"
    );
}

/// Test 10: lode lock with --patch constraint
#[test]
fn lock_patch_constraint() {
    let temp = TempDir::new().unwrap();
    let gemfile = create_test_gemfile(&temp, &[("rake", "13.0.6")]);
    let lockfile = create_test_lockfile(&temp, &[("rake", "13.0.6")]);

    let output = Command::new(get_lode_binary())
        .args([
            "lock",
            "--gemfile",
            &gemfile,
            "--lockfile",
            &lockfile,
            "--patch",
        ])
        .output()
        .expect("Failed to execute lode lock --patch");

    assert!(output.status.success(), "lode lock --patch should succeed");
}

/// Test 11: lode lock with --minor constraint
#[test]
fn lock_minor_constraint() {
    let temp = TempDir::new().unwrap();
    let gemfile = create_test_gemfile(&temp, &[("rake", "13.0.6")]);
    let lockfile = create_test_lockfile(&temp, &[("rake", "13.0.6")]);

    let output = Command::new(get_lode_binary())
        .args([
            "lock",
            "--gemfile",
            &gemfile,
            "--lockfile",
            &lockfile,
            "--minor",
        ])
        .output()
        .expect("Failed to execute lode lock --minor");

    assert!(output.status.success(), "lode lock --minor should succeed");
}

/// Test 12: lode lock with --conservative flag
#[test]
fn lock_conservative_mode() {
    let temp = TempDir::new().unwrap();
    let gemfile = create_test_gemfile(&temp, &[("rake", "13.0.6")]);
    let lockfile = create_test_lockfile(&temp, &[("rake", "13.0.6")]);

    let output = Command::new(get_lode_binary())
        .args([
            "lock",
            "--gemfile",
            &gemfile,
            "--lockfile",
            &lockfile,
            "--update",
            "rake",
            "--conservative",
        ])
        .output()
        .expect("Failed to execute lode lock --conservative");

    assert!(
        output.status.success(),
        "lode lock --conservative should succeed"
    );
}

/// Test 13: lode lock --help shows the command
#[test]
fn lock_help_flag() {
    let output = Command::new(get_lode_binary())
        .args(["lock", "--help"])
        .output()
        .expect("Failed to execute lode lock --help");

    assert!(output.status.success(), "lode lock --help should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("lock") || stdout.contains("Lock"));
    assert!(stdout.contains("--update") || stdout.contains("update"));
}

/// Test 14: Verify help text mentions optional argument for --update
#[test]
fn lock_help_shows_update_flexibility() {
    let output = Command::new(get_lode_binary())
        .args(["lock", "--help"])
        .output()
        .expect("Failed to execute lode lock --help");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should mention --update in help
    assert!(stdout.contains("--update"));
}

/// Test 15: Verify basic lock functionality persists
#[test]
fn lock_basic_creation() {
    let temp = TempDir::new().unwrap();
    let gemfile = create_test_gemfile(&temp, &[("rake", "13.0.6"), ("rspec", "3.12.0")]);
    let lockfile_path = temp.path().join("Gemfile.lock");

    let output = Command::new(get_lode_binary())
        .args([
            "lock",
            "--gemfile",
            &gemfile,
            "--lockfile",
            lockfile_path.to_string_lossy().as_ref(),
        ])
        .output()
        .expect("Failed to execute lode lock");

    assert!(output.status.success(), "Basic lock should succeed");
    assert!(lockfile_path.exists(), "Lockfile should be created");

    let content = fs::read_to_string(&lockfile_path).unwrap();
    assert!(
        content.contains("GEM"),
        "Lockfile should contain GEM section"
    );
    assert!(
        content.contains("PLATFORMS"),
        "Lockfile should contain PLATFORMS section"
    );
}
