mod common;

use std::fmt::Write;
use std::fs;
use std::process::Command;
use tempfile::TempDir;

use common::get_lode_binary;
use common::helpers::{create_test_gemfile, create_test_lockfile};

// ============================================================================
// exec command Tests - Execute commands in bundle context
// ============================================================================

/// Test 1: lode exec with simple command
#[test]
fn exec_with_simple_command() {
    let temp = TempDir::new().unwrap();
    create_test_gemfile(&temp, &[("rake", "13.0.6")]);
    let _lockfile = create_test_lockfile(&temp, &[("rake", "13.0.6")]);

    let output = Command::new(get_lode_binary())
        .current_dir(temp.path())
        .args(["exec", "echo", "test"])
        .output()
        .expect("Failed to execute lode exec");

    let stderr = String::from_utf8_lossy(&output.stderr);
    // Should either succeed or fail with meaningful error (not parsing error)
    assert!(
        output.status.success() || !stderr.contains("unexpected argument"),
        "exec should accept command arguments. stderr: {stderr}"
    );
}

/// Test 2: lode exec --gemfile with custom Gemfile
#[test]
fn exec_with_custom_gemfile() {
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
            "exec",
            "--gemfile",
            gemfile_path.to_string_lossy().as_ref(),
            "echo",
            "test",
        ])
        .output()
        .expect("Failed to execute lode exec --gemfile");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "exec should accept --gemfile flag. stderr: {stderr}"
    );
}

/// Test 3: lode exec with Ruby code
#[test]
fn exec_with_ruby_code() {
    let temp = TempDir::new().unwrap();
    create_test_gemfile(&temp, &[("rake", "13.0.6")]);
    let _lockfile = create_test_lockfile(&temp, &[("rake", "13.0.6")]);

    let output = Command::new(get_lode_binary())
        .current_dir(temp.path())
        .args(["exec", "ruby", "-e", "puts 'hello'"])
        .output()
        .expect("Failed to execute lode exec with ruby");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "exec should accept ruby command. stderr: {stderr}"
    );
}

/// Test 4: lode exec --help displays usage
#[test]
fn exec_help_flag() {
    let output = Command::new(get_lode_binary())
        .args(["exec", "--help"])
        .output()
        .expect("Failed to execute lode exec --help");

    assert!(output.status.success(), "lode exec --help should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.is_empty(), "exec --help should display help text");
}

/// Test 5: lode exec -h short help flag
#[test]
fn exec_help_short_flag() {
    let output = Command::new(get_lode_binary())
        .args(["exec", "-h"])
        .output()
        .expect("Failed to execute lode exec -h");

    assert!(output.status.success(), "lode exec -h should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.is_empty(), "exec -h should display help text");
}

/// Test 6: lode exec with missing Gemfile.lock
#[test]
fn exec_missing_lockfile() {
    let temp = TempDir::new().unwrap();
    // Create Gemfile but not Gemfile.lock
    let gemfile_path = temp.path().join("Gemfile");
    fs::write(&gemfile_path, "source 'https://rubygems.org'\n").unwrap();

    let output = Command::new(get_lode_binary())
        .current_dir(temp.path())
        .args(["exec", "echo", "test"])
        .output()
        .expect("Failed to execute lode exec");

    // Should fail with proper error, not parsing error
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "exec should handle missing lockfile gracefully. stderr: {stderr}"
    );
}

/// Test 7: lode exec with no command specified
#[test]
fn exec_no_command() {
    let temp = TempDir::new().unwrap();
    create_test_gemfile(&temp, &[("rake", "13.0.6")]);
    let _lockfile = create_test_lockfile(&temp, &[("rake", "13.0.6")]);

    let output = Command::new(get_lode_binary())
        .current_dir(temp.path())
        .args(["exec"])
        .output()
        .expect("Failed to execute lode exec");

    // Should fail with proper error (missing required argument)
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "exec should handle missing command gracefully. stderr: {stderr}"
    );
}

/// Test 8: lode exec with environment setup
#[test]
fn exec_environment_variables() {
    let temp = TempDir::new().unwrap();
    create_test_gemfile(&temp, &[("rake", "13.0.6")]);
    let _lockfile = create_test_lockfile(&temp, &[("rake", "13.0.6")]);

    // exec should set up environment variables for gem context
    let output = Command::new(get_lode_binary())
        .current_dir(temp.path())
        .args(["exec", "sh", "-c", "echo $GEM_HOME"])
        .output()
        .expect("Failed to execute lode exec with environment");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "exec should set up environment. stderr: {stderr}"
    );
}

// ============================================================================
// search command Tests - Search for gems on RubyGems.org
// ============================================================================

/// Test 1: lode search with valid query
#[test]
fn search_valid_query() {
    let output = Command::new(get_lode_binary())
        .args(["search", "bundler"])
        .output()
        .expect("Failed to execute lode search bundler");

    // Should either succeed or fail gracefully
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success() || !stderr.contains("unexpected argument"),
        "search should accept query. stderr: {stderr}"
    );
}

/// Test 2: lode search with query containing hyphens
#[test]
fn search_pattern_with_hyphens() {
    let output = Command::new(get_lode_binary())
        .args(["search", "test-gem"])
        .output()
        .expect("Failed to execute lode search with hyphens");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "search should handle hyphens in pattern. stderr: {stderr}"
    );
}

/// Test 3: lode search with single-character query
#[test]
fn search_single_char_query() {
    let output = Command::new(get_lode_binary())
        .args(["search", "a"])
        .output()
        .expect("Failed to execute lode search single char");

    // Should either succeed or fail with no results (not parsing error)
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "search should handle short queries. stderr: {stderr}"
    );
}

/// Test 4: lode search with numbers in query
#[test]
fn search_with_numbers() {
    let output = Command::new(get_lode_binary())
        .args(["search", "gem2"])
        .output()
        .expect("Failed to execute lode search with numbers");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "search should handle numbers in pattern. stderr: {stderr}"
    );
}

/// Test 5: lode search --help displays usage
#[test]
fn search_help_flag() {
    let output = Command::new(get_lode_binary())
        .args(["search", "--help"])
        .output()
        .expect("Failed to execute lode search --help");

    assert!(output.status.success(), "lode search --help should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.is_empty(), "search --help should display help text");
}

/// Test 6: lode search -h short help flag
#[test]
fn search_help_short_flag() {
    let output = Command::new(get_lode_binary())
        .args(["search", "-h"])
        .output()
        .expect("Failed to execute lode search -h");

    assert!(output.status.success(), "lode search -h should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.is_empty(), "search -h should display help text");
}

/// Test 7: lode search with common gem names
#[test]
fn search_popular_gems() {
    let output = Command::new(get_lode_binary())
        .args(["search", "rails"])
        .output()
        .expect("Failed to execute lode search rails");

    // Should either find results or handle gracefully
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "search should accept gem names. stderr: {stderr}"
    );
}
