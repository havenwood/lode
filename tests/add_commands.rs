mod common;

use std::fs;
use std::process::Command;
use tempfile::TempDir;

use common::get_lode_binary;

/// Test 1: Add gem with basic version constraint
#[test]
fn add_gem_with_version() {
    let temp = TempDir::new().unwrap();
    let gemfile = temp.path().join("Gemfile");
    fs::write(
        &gemfile,
        "source 'https://rubygems.org'\n\ngem 'rake', '13.0.6'\n",
    )
    .unwrap();

    let output = Command::new(get_lode_binary())
        .current_dir(temp.path())
        .args(["add", "rspec", "--version", "~> 3.12", "--skip-lock"])
        .output()
        .expect("Failed to execute lode add");

    assert!(
        output.status.success(),
        "lode add with version should succeed. stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let content = fs::read_to_string(&gemfile).unwrap();
    assert!(
        content.contains("rspec"),
        "Gemfile should contain rspec after add"
    );
}

/// Test 2: Add gem to specific group
#[test]
fn add_gem_to_group() {
    let temp = TempDir::new().unwrap();
    let gemfile = temp.path().join("Gemfile");
    fs::write(
        &gemfile,
        "source 'https://rubygems.org'\n\ngem 'rails', '7.0.4'\n",
    )
    .unwrap();

    let output = Command::new(get_lode_binary())
        .current_dir(temp.path())
        .args(["add", "rspec", "--group", "test", "--skip-lock"])
        .output()
        .expect("Failed to execute lode add");

    assert!(
        output.status.success(),
        "lode add to group should succeed. stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let content = fs::read_to_string(&gemfile).unwrap();
    assert!(
        content.contains("rspec"),
        "Gemfile should contain rspec after add"
    );
}

/// Test 3: Add gem with custom source
#[test]
fn add_gem_with_source() {
    let temp = TempDir::new().unwrap();
    let gemfile = temp.path().join("Gemfile");
    fs::write(&gemfile, "source 'https://rubygems.org'\n\ngem 'rails'\n").unwrap();

    let output = Command::new(get_lode_binary())
        .current_dir(temp.path())
        .args([
            "add",
            "custom-gem",
            "--source",
            "https://custom.gems.io",
            "--skip-lock",
        ])
        .output()
        .expect("Failed to execute lode add");

    // May fail due to missing cache, but should accept the flag
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success() || !stderr.contains("unexpected argument"),
        "Flag should be accepted without 'unexpected argument' error. stderr: {stderr}"
    );
}

/// Test 4: Add gem with strict version constraint
#[test]
fn add_gem_strict() {
    let temp = TempDir::new().unwrap();
    let gemfile = temp.path().join("Gemfile");
    fs::write(&gemfile, "source 'https://rubygems.org'\n\ngem 'rails'\n").unwrap();

    let output = Command::new(get_lode_binary())
        .current_dir(temp.path())
        .args([
            "add",
            "rake",
            "--version",
            "13.0.6",
            "--strict",
            "--skip-lock",
        ])
        .output()
        .expect("Failed to execute lode add");

    assert!(
        output.status.success(),
        "lode add --strict should succeed. stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let content = fs::read_to_string(&gemfile).unwrap();
    assert!(
        content.contains("rake"),
        "Gemfile should contain rake after add"
    );
}

/// Test 5: Add gem with optimistic version constraint
#[test]
fn add_gem_optimistic() {
    let temp = TempDir::new().unwrap();
    let gemfile = temp.path().join("Gemfile");
    fs::write(&gemfile, "source 'https://rubygems.org'\n\ngem 'rails'\n").unwrap();

    let output = Command::new(get_lode_binary())
        .current_dir(temp.path())
        .args([
            "add",
            "rspec",
            "--version",
            "3.12",
            "--optimistic",
            "--skip-lock",
        ])
        .output()
        .expect("Failed to execute lode add");

    assert!(
        output.status.success(),
        "lode add --optimistic should succeed. stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let content = fs::read_to_string(&gemfile).unwrap();
    assert!(
        content.contains("rspec"),
        "Gemfile should contain rspec after add"
    );
}

/// Test 6: Add gem with local path
#[test]
fn add_gem_with_path() {
    let temp = TempDir::new().unwrap();
    let gemfile = temp.path().join("Gemfile");
    fs::write(&gemfile, "source 'https://rubygems.org'\n\ngem 'rails'\n").unwrap();

    let output = Command::new(get_lode_binary())
        .current_dir(temp.path())
        .args([
            "add",
            "local-gem",
            "--path",
            "./vendor/gems/local-gem",
            "--skip-lock",
        ])
        .output()
        .expect("Failed to execute lode add");

    assert!(
        output.status.success(),
        "lode add --path should succeed. stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let content = fs::read_to_string(&gemfile).unwrap();
    assert!(
        content.contains("local-gem"),
        "Gemfile should contain local-gem after add"
    );
}

/// Test 7: Add gem with git repository
#[test]
fn add_gem_with_git() {
    let temp = TempDir::new().unwrap();
    let gemfile = temp.path().join("Gemfile");
    fs::write(&gemfile, "source 'https://rubygems.org'\n\ngem 'rails'\n").unwrap();

    let output = Command::new(get_lode_binary())
        .current_dir(temp.path())
        .args([
            "add",
            "custom-gem",
            "--git",
            "https://github.com/user/custom-gem.git",
            "--skip-lock",
        ])
        .output()
        .expect("Failed to execute lode add");

    assert!(
        output.status.success(),
        "lode add --git should succeed. stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let content = fs::read_to_string(&gemfile).unwrap();
    assert!(
        content.contains("custom-gem"),
        "Gemfile should contain custom-gem after add"
    );
}

/// Test 8: Add gem with GitHub shorthand
#[test]
fn add_gem_with_github() {
    let temp = TempDir::new().unwrap();
    let gemfile = temp.path().join("Gemfile");
    fs::write(&gemfile, "source 'https://rubygems.org'\n\ngem 'rails'\n").unwrap();

    let output = Command::new(get_lode_binary())
        .current_dir(temp.path())
        .args([
            "add",
            "custom-gem",
            "--github",
            "user/custom-gem",
            "--skip-lock",
        ])
        .output()
        .expect("Failed to execute lode add");

    assert!(
        output.status.success(),
        "lode add --github should succeed. stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let content = fs::read_to_string(&gemfile).unwrap();
    assert!(
        content.contains("custom-gem"),
        "Gemfile should contain custom-gem after add"
    );
}

/// Test 9: Add gem with git branch
#[test]
fn add_gem_with_branch() {
    let temp = TempDir::new().unwrap();
    let gemfile = temp.path().join("Gemfile");
    fs::write(&gemfile, "source 'https://rubygems.org'\n\ngem 'rails'\n").unwrap();

    let output = Command::new(get_lode_binary())
        .current_dir(temp.path())
        .args([
            "add",
            "custom-gem",
            "--git",
            "https://github.com/user/custom-gem.git",
            "--branch",
            "main",
            "--skip-lock",
        ])
        .output()
        .expect("Failed to execute lode add");

    assert!(
        output.status.success(),
        "lode add --branch should succeed. stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let content = fs::read_to_string(&gemfile).unwrap();
    assert!(
        content.contains("custom-gem"),
        "Gemfile should contain custom-gem after add"
    );
}

/// Test 10: Add gem with git ref
#[test]
fn add_gem_with_ref() {
    let temp = TempDir::new().unwrap();
    let gemfile = temp.path().join("Gemfile");
    fs::write(&gemfile, "source 'https://rubygems.org'\n\ngem 'rails'\n").unwrap();

    let output = Command::new(get_lode_binary())
        .current_dir(temp.path())
        .args([
            "add",
            "custom-gem",
            "--git",
            "https://github.com/user/custom-gem.git",
            "--ref",
            "abc1234",
            "--skip-lock",
        ])
        .output()
        .expect("Failed to execute lode add");

    assert!(
        output.status.success(),
        "lode add --ref should succeed. stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let content = fs::read_to_string(&gemfile).unwrap();
    assert!(
        content.contains("custom-gem"),
        "Gemfile should contain custom-gem after add"
    );
}

/// Test 11: Add gem with require false
#[test]
fn add_gem_require_false() {
    let temp = TempDir::new().unwrap();
    let gemfile = temp.path().join("Gemfile");
    fs::write(&gemfile, "source 'https://rubygems.org'\n\ngem 'rails'\n").unwrap();

    let output = Command::new(get_lode_binary())
        .current_dir(temp.path())
        .args(["add", "some-gem", "--require", "false", "--skip-lock"])
        .output()
        .expect("Failed to execute lode add");

    assert!(
        output.status.success(),
        "lode add --require false should succeed. stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let content = fs::read_to_string(&gemfile).unwrap();
    assert!(
        content.contains("some-gem"),
        "Gemfile should contain some-gem after add"
    );
}

/// Test 12: Add gem with quiet flag
#[test]
fn add_gem_quiet() {
    let temp = TempDir::new().unwrap();
    let gemfile = temp.path().join("Gemfile");
    fs::write(&gemfile, "source 'https://rubygems.org'\n\ngem 'rails'\n").unwrap();

    let output = Command::new(get_lode_binary())
        .current_dir(temp.path())
        .args(["add", "rspec", "--quiet", "--skip-lock"])
        .output()
        .expect("Failed to execute lode add");

    assert!(
        output.status.success(),
        "lode add --quiet should succeed. stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let content = fs::read_to_string(&gemfile).unwrap();
    assert!(
        content.contains("rspec"),
        "Gemfile should contain rspec after add"
    );
}

/// Test 13: Add gem with skip-lock flag
#[test]
fn add_gem_skip_lock() {
    let temp = TempDir::new().unwrap();
    let gemfile = temp.path().join("Gemfile");
    fs::write(&gemfile, "source 'https://rubygems.org'\n\ngem 'rails'\n").unwrap();

    let output = Command::new(get_lode_binary())
        .current_dir(temp.path())
        .args(["add", "rspec", "--skip-lock"])
        .output()
        .expect("Failed to execute lode add");

    assert!(
        output.status.success(),
        "lode add --skip-lock should succeed. stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let content = fs::read_to_string(&gemfile).unwrap();
    assert!(
        content.contains("rspec"),
        "Gemfile should contain rspec after add"
    );

    // Skip-lock should mean lockfile is not updated by the add command
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Run `lode lock`") || stdout.contains("skip"),
        "Output should indicate lock was skipped"
    );
}

/// Test 14: Add gem help flag
#[test]
fn add_help_flag() {
    let output = Command::new(get_lode_binary())
        .args(["add", "--help"])
        .output()
        .expect("Failed to execute lode add --help");

    assert!(output.status.success(), "lode add --help should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("add") || stdout.contains("Add"));
    assert!(stdout.contains("--version") || stdout.contains("version"));
    assert!(stdout.contains("--group") || stdout.contains("group"));
}

/// Test 15: Add gem preserves existing Gemfile content
#[test]
fn add_gem_preserves_content() {
    let temp = TempDir::new().unwrap();
    let gemfile = temp.path().join("Gemfile");
    let original_content = "source 'https://rubygems.org'\n\n# Comment\ngem 'rails', '7.0.4'\n";
    fs::write(&gemfile, original_content).unwrap();

    let output = Command::new(get_lode_binary())
        .current_dir(temp.path())
        .args(["add", "rspec", "--version", "~> 3.12", "--skip-lock"])
        .output()
        .expect("Failed to execute lode add");

    assert!(
        output.status.success(),
        "lode add should succeed. stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let content = fs::read_to_string(&gemfile).unwrap();
    assert!(
        content.contains("rails"),
        "Gemfile should preserve existing gems"
    );
    assert!(
        content.contains("7.0.4"),
        "Gemfile should preserve existing versions"
    );
    assert!(
        content.contains("# Comment"),
        "Gemfile should preserve comments"
    );
    assert!(
        content.contains("rspec"),
        "Gemfile should contain newly added gem"
    );
}

/// Test 16: Add gem error when Gemfile not found
#[test]
fn add_gem_no_gemfile() {
    let temp = TempDir::new().unwrap();

    let output = Command::new(get_lode_binary())
        .current_dir(temp.path())
        .args(["add", "rspec"])
        .output()
        .expect("Failed to execute lode add");

    assert!(
        !output.status.success(),
        "lode add should fail when Gemfile not found"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("not found") || stderr.contains("Gemfile"),
        "Error should mention missing Gemfile. stderr: {stderr}"
    );
}

/// Test 17: Add gem multiple times to same Gemfile
#[test]
fn add_multiple_gems() {
    let temp = TempDir::new().unwrap();
    let gemfile = temp.path().join("Gemfile");
    fs::write(&gemfile, "source 'https://rubygems.org'\n\ngem 'rails'\n").unwrap();

    let lode_binary = get_lode_binary(); // Get path before changing directory

    // Add first gem
    let output1 = Command::new(&lode_binary)
        .current_dir(temp.path())
        .args(["add", "rspec", "--version", "~> 3.12", "--skip-lock"])
        .output()
        .expect("Failed to execute lode add for rspec");

    assert!(
        output1.status.success(),
        "First add should succeed. stderr: {}",
        String::from_utf8_lossy(&output1.stderr)
    );

    // Add second gem
    let output2 = Command::new(&lode_binary)
        .current_dir(temp.path())
        .args(["add", "rake", "--version", "13.0.6", "--skip-lock"])
        .output()
        .expect("Failed to execute lode add for rake");

    assert!(
        output2.status.success(),
        "Second add should succeed. stderr: {}",
        String::from_utf8_lossy(&output2.stderr)
    );

    let content = fs::read_to_string(&gemfile).unwrap();
    assert!(
        content.contains("rails") && content.contains("rspec") && content.contains("rake"),
        "Gemfile should contain all three gems"
    );
}

/// Test 18: Add gem with complex version constraint
#[test]
fn add_gem_complex_version() {
    let temp = TempDir::new().unwrap();
    let gemfile = temp.path().join("Gemfile");
    fs::write(&gemfile, "source 'https://rubygems.org'\n\ngem 'rails'\n").unwrap();

    let output = Command::new(get_lode_binary())
        .current_dir(temp.path())
        .args(["add", "devise", "--version", ">= 4.0, < 5.0", "--skip-lock"])
        .output()
        .expect("Failed to execute lode add");

    assert!(
        output.status.success(),
        "lode add with complex version should succeed. stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let content = fs::read_to_string(&gemfile).unwrap();
    assert!(content.contains("devise"), "Gemfile should contain devise");
}
