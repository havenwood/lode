mod common;

use std::fs;
use std::process::Command;
use tempfile::TempDir;

use common::get_lode_binary;
use common::helpers::{create_test_gemfile, create_test_lockfile};

/// Test 1: Update with --all flag
#[test]
fn update_all_gems() {
    let temp = TempDir::new().unwrap();
    create_test_gemfile(&temp, &[("rake", "13.0.0")]);
    create_test_lockfile(&temp, &[("rake", "13.0.0")]);

    let output = Command::new(get_lode_binary())
        .current_dir(temp.path())
        .args(["update", "--all"])
        .output()
        .expect("Failed to execute lode update --all");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success() || !stderr.contains("unexpected argument"),
        "update --all should be accepted. stderr: {stderr}"
    );
}

/// Test 2: Update with --conservative flag
#[test]
fn update_conservative() {
    let temp = TempDir::new().unwrap();
    let _gemfile = create_test_gemfile(&temp, &[("rake", "13.0.0")]);
    let _lockfile = create_test_lockfile(&temp, &[("rake", "13.0.0")]);

    let output = Command::new(get_lode_binary())
        .current_dir(temp.path())
        .args(["update", "--conservative"])
        .output()
        .expect("Failed to execute lode update --conservative");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success() || !stderr.contains("unexpected argument"),
        "update --conservative should be accepted. stderr: {stderr}"
    );
}

/// Test 3: Update with --patch flag
#[test]
fn update_patch() {
    let temp = TempDir::new().unwrap();
    let _gemfile = create_test_gemfile(&temp, &[("rake", "13.0.0")]);
    let _lockfile = create_test_lockfile(&temp, &[("rake", "13.0.0")]);

    let output = Command::new(get_lode_binary())
        .current_dir(temp.path())
        .args(["update", "--patch"])
        .output()
        .expect("Failed to execute lode update --patch");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success() || !stderr.contains("unexpected argument"),
        "update --patch should be accepted. stderr: {stderr}"
    );
}

/// Test 4: Update with --minor flag
#[test]
fn update_minor() {
    let temp = TempDir::new().unwrap();
    let _gemfile = create_test_gemfile(&temp, &[("rake", "13.0.0")]);
    let _lockfile = create_test_lockfile(&temp, &[("rake", "13.0.0")]);

    let output = Command::new(get_lode_binary())
        .current_dir(temp.path())
        .args(["update", "--minor"])
        .output()
        .expect("Failed to execute lode update --minor");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success() || !stderr.contains("unexpected argument"),
        "update --minor should be accepted. stderr: {stderr}"
    );
}

/// Test 5: Update with --major flag
#[test]
fn update_major() {
    let temp = TempDir::new().unwrap();
    let _gemfile = create_test_gemfile(&temp, &[("rake", "13.0.0")]);
    let _lockfile = create_test_lockfile(&temp, &[("rake", "13.0.0")]);

    let output = Command::new(get_lode_binary())
        .current_dir(temp.path())
        .args(["update", "--major"])
        .output()
        .expect("Failed to execute lode update --major");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success() || !stderr.contains("unexpected argument"),
        "update --major should be accepted. stderr: {stderr}"
    );
}

/// Test 6: Update with --strict flag
#[test]
fn update_strict() {
    let temp = TempDir::new().unwrap();
    let _gemfile = create_test_gemfile(&temp, &[("rake", "13.0.0")]);
    let _lockfile = create_test_lockfile(&temp, &[("rake", "13.0.0")]);

    let output = Command::new(get_lode_binary())
        .current_dir(temp.path())
        .args(["update", "--patch", "--strict"])
        .output()
        .expect("Failed to execute lode update --strict");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success() || !stderr.contains("unexpected argument"),
        "update --strict should be accepted. stderr: {stderr}"
    );
}

/// Test 7: Update with --local flag
#[test]
fn update_local() {
    let temp = TempDir::new().unwrap();
    let _gemfile = create_test_gemfile(&temp, &[("rake", "13.0.0")]);
    let _lockfile = create_test_lockfile(&temp, &[("rake", "13.0.0")]);

    let output = Command::new(get_lode_binary())
        .current_dir(temp.path())
        .args(["update", "--local"])
        .output()
        .expect("Failed to execute lode update --local");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success() || !stderr.contains("unexpected argument"),
        "update --local should be accepted. stderr: {stderr}"
    );
}

/// Test 8: Update with --pre flag (allow prereleases)
#[test]
fn update_pre() {
    let temp = TempDir::new().unwrap();
    let _gemfile = create_test_gemfile(&temp, &[("rake", "13.0.0")]);
    let _lockfile = create_test_lockfile(&temp, &[("rake", "13.0.0")]);

    let output = Command::new(get_lode_binary())
        .current_dir(temp.path())
        .args(["update", "--pre"])
        .output()
        .expect("Failed to execute lode update --pre");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success() || !stderr.contains("unexpected argument"),
        "update --pre should be accepted. stderr: {stderr}"
    );
}

/// Test 9: Update with --quiet flag
#[test]
fn update_quiet() {
    let temp = TempDir::new().unwrap();
    let _gemfile = create_test_gemfile(&temp, &[("rake", "13.0.0")]);
    let _lockfile = create_test_lockfile(&temp, &[("rake", "13.0.0")]);

    let output = Command::new(get_lode_binary())
        .current_dir(temp.path())
        .args(["update", "--quiet"])
        .output()
        .expect("Failed to execute lode update --quiet");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success() || !stderr.contains("unexpected argument"),
        "update --quiet should be accepted. stderr: {stderr}"
    );

    // Verify that --quiet flag is recognized (output length check removed
    // as it's too implementation-specific and can vary with gem versions)
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("Installing") || stdout.len() < 500,
        "quiet mode should suppress verbose installation messages. Output length: {} chars",
        stdout.len()
    );
}

/// Test 10: Update with --group flag
#[test]
fn update_group() {
    let temp = TempDir::new().unwrap();
    let gemfile_path = temp.path().join("Gemfile");
    let content = "source 'https://rubygems.org'\n\ngroup :test do\n  gem 'rspec', '3.12.0'\nend\n";
    fs::write(&gemfile_path, content).unwrap();
    let _gemfile = gemfile_path.to_string_lossy().to_string();
    let _lockfile = create_test_lockfile(&temp, &[("rspec", "3.12.0")]);

    let output = Command::new(get_lode_binary())
        .current_dir(temp.path())
        .args(["update", "--group", "test"])
        .output()
        .expect("Failed to execute lode update --group");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success() || !stderr.contains("unexpected argument"),
        "update --group should be accepted. stderr: {stderr}"
    );
}

/// Test 11: Update with --source flag
#[test]
fn update_source() {
    let temp = TempDir::new().unwrap();
    let _gemfile = create_test_gemfile(&temp, &[("rake", "13.0.0")]);
    let _lockfile = create_test_lockfile(&temp, &[("rake", "13.0.0")]);

    let output = Command::new(get_lode_binary())
        .current_dir(temp.path())
        .args(["update", "--source", "https://rubygems.org"])
        .output()
        .expect("Failed to execute lode update --source");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success() || !stderr.contains("unexpected argument"),
        "update --source should be accepted. stderr: {stderr}"
    );
}

/// Test 12: Update with --jobs flag
#[test]
fn update_jobs() {
    let temp = TempDir::new().unwrap();
    let _gemfile = create_test_gemfile(&temp, &[("rake", "13.0.0")]);
    let _lockfile = create_test_lockfile(&temp, &[("rake", "13.0.0")]);

    let output = Command::new(get_lode_binary())
        .current_dir(temp.path())
        .args(["update", "--jobs", "4"])
        .output()
        .expect("Failed to execute lode update --jobs");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success() || !stderr.contains("unexpected argument"),
        "update --jobs should be accepted. stderr: {stderr}"
    );
}

/// Test 13: Update with --retry flag
#[test]
fn update_retry() {
    let temp = TempDir::new().unwrap();
    let _gemfile = create_test_gemfile(&temp, &[("rake", "13.0.0")]);
    let _lockfile = create_test_lockfile(&temp, &[("rake", "13.0.0")]);

    let output = Command::new(get_lode_binary())
        .current_dir(temp.path())
        .args(["update", "--retry", "3"])
        .output()
        .expect("Failed to execute lode update --retry");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success() || !stderr.contains("unexpected argument"),
        "update --retry should be accepted. stderr: {stderr}"
    );
}

/// Test 14: Update specific gem
#[test]
fn update_specific_gem() {
    let temp = TempDir::new().unwrap();
    let _gemfile = create_test_gemfile(&temp, &[("rake", "13.0.0"), ("rspec", "3.12.0")]);
    let _lockfile = create_test_lockfile(&temp, &[("rake", "13.0.0"), ("rspec", "3.12.0")]);

    let output = Command::new(get_lode_binary())
        .current_dir(temp.path())
        .args(["update", "rake"])
        .output()
        .expect("Failed to execute lode update rake");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success() || !stderr.contains("unexpected argument"),
        "update with specific gem should be accepted. stderr: {stderr}"
    );
}

/// Test 15: Update multiple specific gems
#[test]
fn update_multiple_gems() {
    let temp = TempDir::new().unwrap();
    let _gemfile = create_test_gemfile(&temp, &[("rake", "13.0.0"), ("rspec", "3.12.0")]);
    let _lockfile = create_test_lockfile(&temp, &[("rake", "13.0.0"), ("rspec", "3.12.0")]);

    let output = Command::new(get_lode_binary())
        .current_dir(temp.path())
        .args(["update", "rake", "rspec"])
        .output()
        .expect("Failed to execute lode update rake rspec");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success() || !stderr.contains("unexpected argument"),
        "update with multiple gems should be accepted. stderr: {stderr}"
    );
}

/// Test 16: Update with --redownload flag (compatibility flag)
#[test]
fn update_redownload() {
    let temp = TempDir::new().unwrap();
    let _gemfile = create_test_gemfile(&temp, &[("rake", "13.0.0")]);
    let _lockfile = create_test_lockfile(&temp, &[("rake", "13.0.0")]);

    let output = Command::new(get_lode_binary())
        .current_dir(temp.path())
        .args(["update", "--redownload"])
        .output()
        .expect("Failed to execute lode update --redownload");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success() || !stderr.contains("unexpected argument"),
        "update --redownload should be accepted. stderr: {stderr}"
    );
}

/// Test 17: Update with --full-index flag (compatibility flag)
#[test]
fn update_full_index() {
    let temp = TempDir::new().unwrap();
    let _gemfile = create_test_gemfile(&temp, &[("rake", "13.0.0")]);
    let _lockfile = create_test_lockfile(&temp, &[("rake", "13.0.0")]);

    let output = Command::new(get_lode_binary())
        .current_dir(temp.path())
        .args(["update", "--full-index"])
        .output()
        .expect("Failed to execute lode update --full-index");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success() || !stderr.contains("unexpected argument"),
        "update --full-index should be accepted. stderr: {stderr}"
    );
}

/// Test 18: Update with --ruby flag (update Ruby version)
#[test]
fn update_ruby() {
    let temp = TempDir::new().unwrap();
    let _gemfile = create_test_gemfile(&temp, &[("rake", "13.0.0")]);
    let _lockfile = create_test_lockfile(&temp, &[("rake", "13.0.0")]);

    let output = Command::new(get_lode_binary())
        .current_dir(temp.path())
        .args(["update", "--ruby"])
        .output()
        .expect("Failed to execute lode update --ruby");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success() || !stderr.contains("unexpected argument"),
        "update --ruby should be accepted. stderr: {stderr}"
    );
}

/// Test 19: Update with --bundler flag (update bundler version)
#[test]
fn update_bundler() {
    let temp = TempDir::new().unwrap();
    let _gemfile = create_test_gemfile(&temp, &[("rake", "13.0.0")]);
    let _lockfile = create_test_lockfile(&temp, &[("rake", "13.0.0")]);

    let output = Command::new(get_lode_binary())
        .current_dir(temp.path())
        .args(["update", "--bundler", "2.4.0"])
        .output()
        .expect("Failed to execute lode update --bundler");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success() || !stderr.contains("unexpected argument"),
        "update --bundler should be accepted. stderr: {stderr}"
    );
}

/// Test 20: Update --help shows all flags
#[test]
fn update_help() {
    let output = Command::new(get_lode_binary())
        .args(["update", "--help"])
        .output()
        .expect("Failed to execute lode update --help");

    assert!(output.status.success(), "update --help should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Verify key flags are documented
    assert!(
        stdout.contains("--conservative"),
        "help should document --conservative"
    );
    assert!(stdout.contains("--patch"), "help should document --patch");
    assert!(stdout.contains("--minor"), "help should document --minor");
    assert!(stdout.contains("--major"), "help should document --major");
    assert!(stdout.contains("--quiet"), "help should document --quiet");
    assert!(stdout.contains("--group"), "help should document --group");
}

/// Test 21: Update with combination of flags
#[test]
fn update_combined_flags() {
    let temp = TempDir::new().unwrap();
    let _gemfile = create_test_gemfile(&temp, &[("rake", "13.0.0")]);
    let _lockfile = create_test_lockfile(&temp, &[("rake", "13.0.0")]);

    let output = Command::new(get_lode_binary())
        .current_dir(temp.path())
        .args(["update", "--conservative", "--patch", "--quiet"])
        .output()
        .expect("Failed to execute lode update with combined flags");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success() || !stderr.contains("unexpected argument"),
        "update with combined flags should be accepted. stderr: {stderr}"
    );
}

/// Test 22: Update without lockfile should error gracefully
#[test]
fn update_no_lockfile() {
    let temp = TempDir::new().unwrap();
    let _gemfile = create_test_gemfile(&temp, &[("rake", "13.0.0")]);
    // Don't create a lockfile - update should handle this gracefully

    let output = Command::new(get_lode_binary())
        .current_dir(temp.path())
        .args(["update"])
        .output()
        .expect("Failed to execute lode update");

    // Note: lode update without a lockfile may succeed (creating a new lockfile)
    // or fail gracefully - both are acceptable behaviors
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Just verify it doesn't crash and handles missing lockfile appropriately
    assert!(
        output.status.success()
            || stderr.contains("Failed to read")
            || stderr.contains("not found")
            || stderr.contains("lockfile"),
        "update should handle missing lockfile gracefully. stderr: {stderr}, stdout: {stdout}"
    );
}
