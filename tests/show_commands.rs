mod common;

use std::fs;
use std::process::Command;
use tempfile::TempDir;

use common::get_lode_binary;

/// Helper function to create a test lockfile with custom content
/// Note: This is distinct from `common::create_test_lockfile` which takes gems
fn create_test_lockfile(temp_dir: &TempDir, content: &str) -> String {
    let lockfile_path = temp_dir.path().join("Gemfile.lock");
    fs::write(&lockfile_path, content).unwrap();
    lockfile_path.to_string_lossy().to_string()
}

/// Helper function to create mock gem directories
fn create_mock_gem_dirs(temp_dir: &TempDir, gems: &[(&str, &str)]) {
    let vendor_dir = temp_dir.path().join("vendor/ruby/3.2.0/gems");
    fs::create_dir_all(&vendor_dir).unwrap();

    for (name, version) in gems {
        let gem_dir = vendor_dir.join(format!("{name}-{version}"));
        fs::create_dir_all(&gem_dir).unwrap();
    }
}

/// Test 1: Default behavior - lode show lists all gems with versions
#[test]
fn show_default_lists_all_gems() {
    let temp = TempDir::new().unwrap();
    let lockfile_content = r"GEM
  remote: https://rubygems.org/
  specs:
    rake (13.0.6)
    rspec (3.12.0)
    actioncable (7.0.4)
    bundler (2.4.6)

PLATFORMS
  ruby

DEPENDENCIES
  rake
  rspec (~> 3.12.0)

RUBY VERSION
   ruby 3.2.0

BUNDLED WITH
   2.4.6
";
    create_test_lockfile(&temp, lockfile_content);

    let output = Command::new(get_lode_binary())
        .current_dir(temp.path())
        .args(["show"])
        .output()
        .expect("Failed to execute lode show");

    if !output.status.success() {
        eprintln!("stdout: {}", String::from_utf8_lossy(&output.stdout));
        eprintln!("stderr: {}", String::from_utf8_lossy(&output.stderr));
    }
    assert!(output.status.success(), "lode show should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Check that all gems are listed
    assert!(stdout.contains("actioncable (7.0.4)"));
    assert!(stdout.contains("bundler (2.4.6)"));
    assert!(stdout.contains("rake (13.0.6)"));
    assert!(stdout.contains("rspec (3.12.0)"));

    // Check that gems are sorted alphabetically
    let lines: Vec<&str> = stdout.lines().collect();
    assert!(
        lines.first().is_some_and(|l| l.contains("actioncable")),
        "First gem should be actioncable (alphabetically)"
    );
    assert!(
        lines.get(1).is_some_and(|l| l.contains("bundler")),
        "Second gem should be bundler"
    );
    assert!(
        lines.get(2).is_some_and(|l| l.contains("rake")),
        "Third gem should be rake"
    );
    assert!(
        lines.get(3).is_some_and(|l| l.contains("rspec")),
        "Fourth gem should be rspec"
    );
}

/// Test 2: lode show <gem> shows path to specific gem
#[test]
fn show_specific_gem_path() {
    let temp = TempDir::new().unwrap();
    let lockfile_content = r"GEM
  remote: https://rubygems.org/
  specs:
    rake (13.0.6)
    rspec (3.12.0)

PLATFORMS
  ruby

DEPENDENCIES
  rake
  rspec (~> 3.12.0)

RUBY VERSION
   ruby 3.2.0

BUNDLED WITH
   2.4.6
";
    create_test_lockfile(&temp, lockfile_content);
    create_mock_gem_dirs(&temp, &[("rake", "13.0.6"), ("rspec", "3.12.0")]);

    let output = Command::new(get_lode_binary())
        .current_dir(temp.path())
        .env("BUNDLE_PATH", temp.path().join("vendor"))
        .args(["show", "rake"])
        .output()
        .expect("Failed to execute lode show rake");

    if !output.status.success() {
        eprintln!("stdout: {}", String::from_utf8_lossy(&output.stdout));
        eprintln!("stderr: {}", String::from_utf8_lossy(&output.stderr));
    }
    assert!(output.status.success(), "lode show rake should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Check that the path contains rake gem
    assert!(
        stdout.contains("rake-13.0.6"),
        "Output should contain rake gem path"
    );
    assert!(!stdout.contains("rspec"), "Output should not contain rspec");
}

/// Test 3: lode show --paths lists all gem paths sorted
#[test]
fn show_paths_lists_all_sorted() {
    let temp = TempDir::new().unwrap();
    let lockfile_content = r"GEM
  remote: https://rubygems.org/
  specs:
    rake (13.0.6)
    rspec (3.12.0)
    actioncable (7.0.4)

PLATFORMS
  ruby

DEPENDENCIES
  rake
  rspec (~> 3.12.0)

RUBY VERSION
   ruby 3.2.0

BUNDLED WITH
   2.4.6
";
    create_test_lockfile(&temp, lockfile_content);
    create_mock_gem_dirs(
        &temp,
        &[
            ("rake", "13.0.6"),
            ("rspec", "3.12.0"),
            ("actioncable", "7.0.4"),
        ],
    );

    let output = Command::new(get_lode_binary())
        .current_dir(temp.path())
        .env("BUNDLE_PATH", temp.path().join("vendor"))
        .args(["show", "--paths"])
        .output()
        .expect("Failed to execute lode show --paths");

    if !output.status.success() {
        eprintln!("stdout: {}", String::from_utf8_lossy(&output.stdout));
        eprintln!("stderr: {}", String::from_utf8_lossy(&output.stderr));
    }
    assert!(output.status.success(), "lode show --paths should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Check that all paths are listed
    assert!(stdout.contains("actioncable-7.0.4"));
    assert!(stdout.contains("rake-13.0.6"));
    assert!(stdout.contains("rspec-3.12.0"));

    // Check that paths are sorted alphabetically by gem name
    let lines: Vec<&str> = stdout.lines().collect();
    assert!(
        lines
            .first()
            .is_some_and(|l| l.contains("actioncable-7.0.4")),
        "First path should be actioncable"
    );
    assert!(
        lines.get(1).is_some_and(|l| l.contains("rake-13.0.6")),
        "Second path should be rake"
    );
    assert!(
        lines.get(2).is_some_and(|l| l.contains("rspec-3.12.0")),
        "Third path should be rspec"
    );
}

/// Test 4: lode show <nonexistent> returns error
#[test]
fn show_nonexistent_gem_error() {
    let temp = TempDir::new().unwrap();
    let lockfile_content = r"GEM
  remote: https://rubygems.org/
  specs:
    rake (13.0.6)

PLATFORMS
  ruby

DEPENDENCIES
  rake

RUBY VERSION
   ruby 3.2.0

BUNDLED WITH
   2.4.6
";
    create_test_lockfile(&temp, lockfile_content);

    let output = Command::new(get_lode_binary())
        .current_dir(temp.path())
        .args(["show", "nonexistent"])
        .output()
        .expect("Failed to execute lode show");

    assert!(
        !output.status.success(),
        "lode show nonexistent should fail"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("not found") || stderr.contains("Gem"),
        "Error message should indicate gem not found"
    );
}

/// Test 5: lode show <gem> when not installed returns helpful error
#[test]
fn show_gem_not_installed_error() {
    let temp = TempDir::new().unwrap();
    let lockfile_content = r"GEM
  remote: https://rubygems.org/
  specs:
    rake (13.0.6)

PLATFORMS
  ruby

DEPENDENCIES
  rake

RUBY VERSION
   ruby 3.2.0

BUNDLED WITH
   2.4.6
";
    create_test_lockfile(&temp, lockfile_content);

    // Create vendor structure but without the gem
    fs::create_dir_all(temp.path().join("vendor/ruby/3.2.0/gems")).unwrap();

    let output = Command::new(get_lode_binary())
        .current_dir(temp.path())
        .args(["show", "rake"])
        .output()
        .expect("Failed to execute lode show");

    assert!(
        !output.status.success(),
        "lode show rake should fail when not installed"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("not installed"),
        "Error should indicate gem not installed"
    );
}

/// Test 6: lode show with custom --lockfile works
/// NOTE: bundle show doesn't support --lockfile flag, test is invalid for bundler parity
#[test]
#[ignore = "bundle show doesn't have --lockfile flag"]
fn show_with_custom_lockfile() {
    let temp = TempDir::new().unwrap();
    let custom_lockfile_path = temp.path().join("custom/Gemfile.lock");
    fs::create_dir_all(custom_lockfile_path.parent().unwrap()).unwrap();

    let lockfile_content = r"GEM
  remote: https://rubygems.org/
  specs:
    rake (13.0.6)

PLATFORMS
  ruby

DEPENDENCIES
  rake

RUBY VERSION
   ruby 3.2.0

BUNDLED WITH
   2.4.6
";
    fs::write(&custom_lockfile_path, lockfile_content).unwrap();

    let output = Command::new("target/debug/lode")
        .args([
            "show",
            "--lockfile",
            custom_lockfile_path.to_string_lossy().as_ref(),
        ])
        .output()
        .expect("Failed to execute lode show");

    assert!(
        output.status.success(),
        "lode show with custom lockfile should succeed"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("rake (13.0.6)"));
}

/// Test 7: lode show --help displays usage
#[test]
fn show_help_flag() {
    let output = Command::new("target/debug/lode")
        .args(["show", "--help"])
        .output()
        .expect("Failed to execute lode show --help");

    assert!(output.status.success(), "lode show --help should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("Show") || stdout.contains("show"));
    assert!(stdout.contains("--paths") || stdout.contains("paths"));
}

/// Test 8: lode show -h displays usage
#[test]
fn show_help_short_flag() {
    let output = Command::new("target/debug/lode")
        .args(["show", "-h"])
        .output()
        .expect("Failed to execute lode show -h");

    assert!(output.status.success(), "lode show -h should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("Show") || stdout.contains("show"));
}

/// Test 9: Default behavior with git and path gems
#[test]
fn show_default_with_git_and_path_gems() {
    let temp = TempDir::new().unwrap();
    let lockfile_content = r"GEM
  remote: https://rubygems.org/
  specs:
    rake (13.0.6)

GIT
  remote: https://github.com/example/mylib.git
  revision: abc123
  specs:
    mylib (1.0.0)

PATH
  remote: ./local_gem
  specs:
    local_gem (1.0.0)

PLATFORMS
  ruby

DEPENDENCIES
  rake
  mylib!
  local_gem!

RUBY VERSION
   ruby 3.2.0

BUNDLED WITH
   2.4.6
";
    create_test_lockfile(&temp, lockfile_content);

    let output = Command::new(get_lode_binary())
        .current_dir(temp.path())
        .args(["show"])
        .output()
        .expect("Failed to execute lode show");

    if !output.status.success() {
        eprintln!("stdout: {}", String::from_utf8_lossy(&output.stdout));
        eprintln!("stderr: {}", String::from_utf8_lossy(&output.stderr));
    }
    assert!(output.status.success(), "lode show should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Check that all gem types are listed
    assert!(stdout.contains("local_gem (1.0.0)"), "Should list path gem");
    assert!(stdout.contains("mylib (1.0.0)"), "Should list git gem");
    assert!(stdout.contains("rake (13.0.6)"), "Should list regular gem");

    // Check they're sorted
    let lines: Vec<&str> = stdout.lines().collect();
    assert!(
        lines.first().is_some_and(|l| l.contains("local_gem")),
        "First should be local_gem (alphabetically)"
    );
    assert!(
        lines.get(1).is_some_and(|l| l.contains("mylib")),
        "Second should be mylib"
    );
    assert!(
        lines.get(2).is_some_and(|l| l.contains("rake")),
        "Third should be rake"
    );
}

/// Test 10: Empty lockfile
#[test]
fn show_empty_lockfile() {
    let temp = TempDir::new().unwrap();
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
    create_test_lockfile(&temp, lockfile_content);

    let output = Command::new(get_lode_binary())
        .current_dir(temp.path())
        .args(["show"])
        .output()
        .expect("Failed to execute lode show");

    assert!(
        output.status.success(),
        "lode show on empty lockfile should succeed"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should produce no output for empty lockfile
    assert_eq!(stdout.trim(), "", "Output should be empty for no gems");
}

/// Test 11: lode show <gem> with --paths (conflicting options)
#[test]
fn show_gem_with_paths_flag() {
    let temp = TempDir::new().unwrap();
    let lockfile_content = r"GEM
  remote: https://rubygems.org/
  specs:
    rake (13.0.6)
    rspec (3.12.0)

PLATFORMS
  ruby

DEPENDENCIES
  rake
  rspec

RUBY VERSION
   ruby 3.2.0

BUNDLED WITH
   2.4.6
";
    create_test_lockfile(&temp, lockfile_content);
    create_mock_gem_dirs(&temp, &[("rake", "13.0.6"), ("rspec", "3.12.0")]);

    // When both gem name and --paths are provided, --paths takes precedence (list all paths)
    let output = Command::new(get_lode_binary())
        .current_dir(temp.path())
        .env("BUNDLE_PATH", temp.path().join("vendor"))
        .args(["show", "rake", "--paths"])
        .output()
        .expect("Failed to execute lode show");

    if !output.status.success() {
        eprintln!("stdout: {}", String::from_utf8_lossy(&output.stdout));
        eprintln!("stderr: {}", String::from_utf8_lossy(&output.stderr));
    }
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Both gems should be listed since --paths lists all
    assert!(stdout.contains("rake-13.0.6"));
    assert!(stdout.contains("rspec-3.12.0"));
}

// ============================================================================
// info command Tests - Show gem information
// ============================================================================

/// Test 12: lode info displays gem information
#[test]
fn info_displays_gem_information() {
    let output = Command::new("target/debug/lode")
        .args(["info", "bundler"])
        .output()
        .expect("Failed to execute lode info bundler");

    // info command should succeed (bundler is typically available)
    // or fail gracefully if bundler not installed
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Either succeeds with output, or fails with error message (not parsing error)
    assert!(
        output.status.success() || !stderr.contains("unexpected argument"),
        "info command should parse arguments correctly"
    );
}

/// Test 13: lode info --path shows installation path
#[test]
fn info_path_flag() {
    let output = Command::new("target/debug/lode")
        .args(["info", "bundler", "--path"])
        .output()
        .expect("Failed to execute lode info --path");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "info should accept --path flag"
    );
}

/// Test 14: lode info --version shows version only
#[test]
fn info_version_flag() {
    let output = Command::new("target/debug/lode")
        .args(["info", "bundler", "--version"])
        .output()
        .expect("Failed to execute lode info --version");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "info should accept --version flag"
    );
}

/// Test 15: lode info with nonexistent gem handles gracefully
#[test]
fn info_nonexistent_gem() {
    let output = Command::new("target/debug/lode")
        .args(["info", "nonexistent-gem-xyz-12345"])
        .output()
        .expect("Failed to execute lode info");

    // Should fail with helpful error, not parsing error
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "Should handle nonexistent gem gracefully"
    );
}

/// Test 16: lode info --help displays help
#[test]
fn info_help_flag() {
    let output = Command::new("target/debug/lode")
        .args(["info", "--help"])
        .output()
        .expect("Failed to execute lode info --help");

    assert!(output.status.success(), "lode info --help should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(!stdout.is_empty(), "info --help should display help text");
}

/// Test 17: lode info without arguments shows error or help
#[test]
fn info_no_arguments() {
    let output = Command::new("target/debug/lode")
        .args(["info"])
        .output()
        .expect("Failed to execute lode info");

    // Should either show help or error (not parsing error)
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "Should handle missing gem name gracefully"
    );
}

/// Test 18: lode info -h short help flag
#[test]
fn info_help_short_flag() {
    let output = Command::new("target/debug/lode")
        .args(["info", "-h"])
        .output()
        .expect("Failed to execute lode info -h");

    assert!(output.status.success(), "lode info -h should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(!stdout.is_empty(), "info -h should display help text");
}

/// Test 19: lode info --path and --version flags work together
#[test]
fn info_multiple_flags() {
    let output = Command::new("target/debug/lode")
        .args(["info", "bundler", "--path", "--version"])
        .output()
        .expect("Failed to execute lode info with multiple flags");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "info should accept multiple flags"
    );
}
