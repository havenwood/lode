mod common;

use std::fs;
use std::process::Command;
use tempfile::TempDir;

use common::get_lode_binary;
use common::helpers::{create_test_gemfile, create_test_lockfile};

/// Test 1: Install with --gemfile flag
#[test]
fn install_custom_gemfile() {
    let temp = TempDir::new().unwrap();
    create_test_gemfile(&temp, &[("rake", "13.0.6")]);
    let lockfile = create_test_lockfile(&temp, &[("rake", "13.0.6")]);

    let output = Command::new(get_lode_binary())
        .args(["install", "--gemfile", &lockfile])
        .output()
        .expect("Failed to execute lode install");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success() || !stderr.contains("unexpected argument"),
        "install --gemfile should be accepted. stderr: {stderr}"
    );
}

/// Test 2: Install with --jobs flag (parallel downloads)
#[test]
fn install_with_jobs() {
    let temp = TempDir::new().unwrap();
    create_test_gemfile(&temp, &[("rake", "13.0.6")]);
    let lockfile = create_test_lockfile(&temp, &[("rake", "13.0.6")]);

    let output = Command::new(get_lode_binary())
        .args(["install", "--gemfile", &lockfile, "--jobs", "4"])
        .output()
        .expect("Failed to execute lode install --jobs");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success() || !stderr.contains("unexpected argument"),
        "install --jobs should be accepted. stderr: {stderr}"
    );
}

/// Test 3: Install with --retry flag
#[test]
fn install_with_retry() {
    let temp = TempDir::new().unwrap();
    create_test_gemfile(&temp, &[("rake", "13.0.6")]);
    let lockfile = create_test_lockfile(&temp, &[("rake", "13.0.6")]);

    let output = Command::new(get_lode_binary())
        .args(["install", "--gemfile", &lockfile, "--retry", "3"])
        .output()
        .expect("Failed to execute lode install --retry");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success() || !stderr.contains("unexpected argument"),
        "install --retry should be accepted. stderr: {stderr}"
    );
}

/// Test 4: Install with --quiet flag
#[test]
fn install_quiet() {
    let temp = TempDir::new().unwrap();
    create_test_gemfile(&temp, &[("rake", "13.0.6")]);
    let lockfile = create_test_lockfile(&temp, &[("rake", "13.0.6")]);

    let output = Command::new(get_lode_binary())
        .args(["install", "--gemfile", &lockfile, "--quiet"])
        .output()
        .expect("Failed to execute lode install --quiet");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success() || !stderr.contains("unexpected argument"),
        "install --quiet should be accepted. stderr: {stderr}"
    );
}

/// Test 5: Install with --verbose flag
#[test]
fn install_verbose() {
    let temp = TempDir::new().unwrap();
    create_test_gemfile(&temp, &[("rake", "13.0.6")]);
    let lockfile = create_test_lockfile(&temp, &[("rake", "13.0.6")]);

    let output = Command::new(get_lode_binary())
        .args(["install", "--gemfile", &lockfile, "--verbose"])
        .output()
        .expect("Failed to execute lode install --verbose");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success() || !stderr.contains("unexpected argument"),
        "install --verbose should be accepted. stderr: {stderr}"
    );
}

/// Test 6: Install with --local flag (use cached gems only)
#[test]
fn install_local() {
    let temp = TempDir::new().unwrap();
    create_test_gemfile(&temp, &[("rake", "13.0.6")]);
    let lockfile = create_test_lockfile(&temp, &[("rake", "13.0.6")]);

    let output = Command::new(get_lode_binary())
        .args(["install", "--gemfile", &lockfile, "--local"])
        .output()
        .expect("Failed to execute lode install --local");

    let stderr = String::from_utf8_lossy(&output.stderr);
    // May fail due to missing cache, but should accept the flag
    assert!(
        !stderr.contains("unexpected argument"),
        "install --local should be accepted. stderr: {stderr}"
    );
}

/// Test 7: Install with --prefer-local flag
#[test]
fn install_prefer_local() {
    let temp = TempDir::new().unwrap();
    create_test_gemfile(&temp, &[("rake", "13.0.6")]);
    let lockfile = create_test_lockfile(&temp, &[("rake", "13.0.6")]);

    let output = Command::new(get_lode_binary())
        .args(["install", "--gemfile", &lockfile, "--prefer-local"])
        .output()
        .expect("Failed to execute lode install --prefer-local");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success() || !stderr.contains("unexpected argument"),
        "install --prefer-local should be accepted. stderr: {stderr}"
    );
}

/// Test 8: Install with --redownload flag
#[test]
fn install_redownload() {
    let temp = TempDir::new().unwrap();
    create_test_gemfile(&temp, &[("rake", "13.0.6")]);
    let lockfile = create_test_lockfile(&temp, &[("rake", "13.0.6")]);

    let output = Command::new(get_lode_binary())
        .args(["install", "--gemfile", &lockfile, "--redownload"])
        .output()
        .expect("Failed to execute lode install --redownload");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success() || !stderr.contains("unexpected argument"),
        "install --redownload should be accepted. stderr: {stderr}"
    );
}

/// Test 9: Install with --no-cache flag
#[test]
fn install_no_cache() {
    let temp = TempDir::new().unwrap();
    create_test_gemfile(&temp, &[("rake", "13.0.6")]);
    let lockfile = create_test_lockfile(&temp, &[("rake", "13.0.6")]);

    let output = Command::new(get_lode_binary())
        .args(["install", "--gemfile", &lockfile, "--no-cache"])
        .output()
        .expect("Failed to execute lode install --no-cache");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success() || !stderr.contains("unexpected argument"),
        "install --no-cache should be accepted. stderr: {stderr}"
    );
}

/// Test 10: Install with --trust-policy flag
#[test]
fn install_trust_policy() {
    let temp = TempDir::new().unwrap();
    create_test_gemfile(&temp, &[("rake", "13.0.6")]);
    let lockfile = create_test_lockfile(&temp, &[("rake", "13.0.6")]);

    let output = Command::new(get_lode_binary())
        .args([
            "install",
            "--gemfile",
            &lockfile,
            "--trust-policy",
            "MediumSecurity",
        ])
        .output()
        .expect("Failed to execute lode install --trust-policy");

    let stderr = String::from_utf8_lossy(&output.stderr);
    // May fail due to missing signatures, but should accept the flag
    assert!(
        !stderr.contains("unexpected argument"),
        "install --trust-policy should be accepted. stderr: {stderr}"
    );
}

/// Test 11: Install with --without flag (exclude groups)
/// NOTE: --without is deprecated in Bundler 4, use `BUNDLE_WITHOUT` environment variable instead
#[test]
#[ignore = "Tests deprecated --without flag (use BUNDLE_WITHOUT env var instead)"]
fn install_without() {
    let temp = TempDir::new().unwrap();
    let gemfile_path = temp.path().join("Gemfile");
    let content = "source 'https://rubygems.org'\n\ngroup :test do\n  gem 'rspec', '3.12.0'\nend\n";
    fs::write(&gemfile_path, content).unwrap();
    let lockfile = create_test_lockfile(&temp, &[("rspec", "3.12.0")]);

    let output = Command::new(get_lode_binary())
        .args(["install", "--gemfile", &lockfile, "--without", "test"])
        .output()
        .expect("Failed to execute lode install --without");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success() || !stderr.contains("unexpected argument"),
        "install --without should be accepted. stderr: {stderr}"
    );
}

/// Test 12: Install with --with flag (include groups)
/// NOTE: --with is deprecated in Bundler 4, use `BUNDLE_WITH` environment variable instead
#[test]
#[ignore = "Tests deprecated --with flag (use BUNDLE_WITH env var instead)"]
fn install_with() {
    let temp = TempDir::new().unwrap();
    let gemfile_path = temp.path().join("Gemfile");
    let content =
        "source 'https://rubygems.org'\n\ngroup :development do\n  gem 'pry', '0.14.2'\nend\n";
    fs::write(&gemfile_path, content).unwrap();
    let lockfile = create_test_lockfile(&temp, &[("pry", "0.14.2")]);

    let output = Command::new(get_lode_binary())
        .args(["install", "--gemfile", &lockfile, "--with", "development"])
        .output()
        .expect("Failed to execute lode install --with");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success() || !stderr.contains("unexpected argument"),
        "install --with should be accepted. stderr: {stderr}"
    );
}

/// Test 13: Install with --only flag (only specific groups)
/// NOTE: --only is deprecated in Bundler 4, use `BUNDLE_ONLY` environment variable instead
#[test]
#[ignore = "Tests deprecated --only flag (use BUNDLE_ONLY env var instead)"]
fn install_only() {
    let temp = TempDir::new().unwrap();
    let gemfile_path = temp.path().join("Gemfile");
    let content = "source 'https://rubygems.org'\n\ngroup :test do\n  gem 'rspec', '3.12.0'\nend\ngroup :development do\n  gem 'pry', '0.14.2'\nend\n";
    fs::write(&gemfile_path, content).unwrap();
    let lockfile = create_test_lockfile(&temp, &[("rspec", "3.12.0"), ("pry", "0.14.2")]);

    let output = Command::new(get_lode_binary())
        .args(["install", "--gemfile", &lockfile, "--only", "test"])
        .output()
        .expect("Failed to execute lode install --only");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success() || !stderr.contains("unexpected argument"),
        "install --only should be accepted. stderr: {stderr}"
    );
}

/// Test 14: Install with --standalone flag
#[test]
fn install_standalone() {
    let temp = TempDir::new().unwrap();
    create_test_gemfile(&temp, &[("rake", "13.0.6")]);
    let lockfile = create_test_lockfile(&temp, &[("rake", "13.0.6")]);

    let output = Command::new(get_lode_binary())
        .args(["install", "--gemfile", &lockfile, "--standalone", "bundle"])
        .output()
        .expect("Failed to execute lode install --standalone");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success() || !stderr.contains("unexpected argument"),
        "install --standalone should be accepted. stderr: {stderr}"
    );
}

/// Test 15: Install with --binstubs flag
/// NOTE: --binstubs is deprecated in Bundler 4, use `lode binstubs` command instead
#[test]
#[ignore = "Tests deprecated --binstubs flag (use `lode binstubs` command instead)"]
fn install_binstubs() {
    let temp = TempDir::new().unwrap();
    create_test_gemfile(&temp, &[("rake", "13.0.6")]);
    let lockfile = create_test_lockfile(&temp, &[("rake", "13.0.6")]);

    let output = Command::new(get_lode_binary())
        .args(["install", "--gemfile", &lockfile, "--binstubs", "bin"])
        .output()
        .expect("Failed to execute lode install --binstubs");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success() || !stderr.contains("unexpected argument"),
        "install --binstubs should be accepted. stderr: {stderr}"
    );
}

/// Test 16: Install with --full-index flag
#[test]
fn install_full_index() {
    let temp = TempDir::new().unwrap();
    create_test_gemfile(&temp, &[("rake", "13.0.6")]);
    let lockfile = create_test_lockfile(&temp, &[("rake", "13.0.6")]);

    let output = Command::new(get_lode_binary())
        .args(["install", "--gemfile", &lockfile, "--full-index"])
        .output()
        .expect("Failed to execute lode install --full-index");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success() || !stderr.contains("unexpected argument"),
        "install --full-index should be accepted. stderr: {stderr}"
    );
}

/// Test 17: Install with --target-rbconfig flag
#[test]
fn install_target_rbconfig() {
    let temp = TempDir::new().unwrap();
    create_test_gemfile(&temp, &[("rake", "13.0.6")]);
    let lockfile = create_test_lockfile(&temp, &[("rake", "13.0.6")]);

    let output = Command::new(get_lode_binary())
        .args([
            "install",
            "--gemfile",
            &lockfile,
            "--target-rbconfig",
            "/path/to/rbconfig.rb",
        ])
        .output()
        .expect("Failed to execute lode install --target-rbconfig");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "install --target-rbconfig should be accepted. stderr: {stderr}"
    );
}

/// Test 18: Install --help shows all flags
#[test]
fn install_help() {
    let output = Command::new(get_lode_binary())
        .args(["install", "--help"])
        .output()
        .expect("Failed to execute lode install --help");

    assert!(output.status.success(), "install --help should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Verify key flags are documented
    assert!(stdout.contains("--jobs"), "help should document --jobs");
    assert!(stdout.contains("--retry"), "help should document --retry");
    assert!(stdout.contains("--quiet"), "help should document --quiet");
    assert!(
        stdout.contains("--verbose"),
        "help should document --verbose"
    );
    assert!(stdout.contains("--local"), "help should document --local");
    assert!(
        stdout.contains("--standalone"),
        "help should document --standalone"
    );
}

/// Test 19: Install with combined flags
#[test]
fn install_combined_flags() {
    let temp = TempDir::new().unwrap();
    create_test_gemfile(&temp, &[("rake", "13.0.6")]);
    let lockfile = create_test_lockfile(&temp, &[("rake", "13.0.6")]);

    let output = Command::new(get_lode_binary())
        .args([
            "install",
            "--gemfile",
            &lockfile,
            "--jobs",
            "4",
            "--retry",
            "3",
            "--quiet",
        ])
        .output()
        .expect("Failed to execute lode install with combined flags");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success() || !stderr.contains("unexpected argument"),
        "install with combined flags should be accepted. stderr: {stderr}"
    );
}

/// Test 20: Install without lockfile should error gracefully
#[test]
fn install_no_lockfile() {
    let temp = TempDir::new().unwrap();
    let nonexistent_lock = temp.path().join("nonexistent.lock");

    let output = Command::new(get_lode_binary())
        .args([
            "install",
            "--gemfile",
            nonexistent_lock.to_string_lossy().as_ref(),
        ])
        .output()
        .expect("Failed to execute lode install");

    assert!(
        !output.status.success(),
        "install should fail when lockfile doesn't exist"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Failed to read") || stderr.contains("not found"),
        "error should mention missing lockfile. stderr: {stderr}"
    );
}
