mod common;

use lode::{Gemfile, Lockfile, RubyGemsClient};
use std::fs;
use std::process::Command;
use tempfile::TempDir;

use common::get_lode_binary;

#[test]
fn parses_gemfile_from_file() {
    let temp = TempDir::new().unwrap();
    let gemfile_path = temp.path().join("Gemfile");
    fs::write(
        &gemfile_path,
        "source 'https://rubygems.org'\ngem 'rack', '~> 3.0'",
    )
    .unwrap();

    let gemfile = Gemfile::parse_file(&gemfile_path).unwrap();
    assert!(!gemfile.gems.is_empty());
    assert_eq!(
        gemfile.gems.first().map(|g| &g.name),
        Some(&"rack".to_string())
    );
}

#[test]
fn creates_rubygems_client() {
    let client = RubyGemsClient::new("https://rubygems.org").unwrap();
    drop(client);
}

#[tokio::test]
#[ignore = "Requires network access"]
async fn fetches_gem_versions_from_api() {
    let client = RubyGemsClient::new("https://rubygems.org").unwrap();
    let versions = client.fetch_versions("rack").await.unwrap();

    assert!(!versions.is_empty(), "Should have at least one version");
    assert!(
        versions.iter().any(|v| v.number.starts_with("3.")),
        "Should have 3.x version"
    );
}

#[test]
fn parses_lockfile_from_string() {
    let lockfile_content = r"GEM
  remote: https://rubygems.org/
  specs:
    rack (3.0.8)

PLATFORMS
  ruby

DEPENDENCIES
  rack (~> 3.0)

BUNDLED WITH
   2.4.10
";

    let lockfile = Lockfile::parse(lockfile_content).unwrap();
    assert!(!lockfile.gems.is_empty());
    assert_eq!(lockfile.gems.first().unwrap().name, "rack");
    assert_eq!(lockfile.gems.first().unwrap().version, "3.0.8");
}

#[test]
fn parses_lockfile_with_checksums() {
    let lockfile_content = r"GEM
  remote: https://rubygems.org/
  specs:
    rack (3.0.8)

PLATFORMS
  ruby

DEPENDENCIES
  rack (~> 3.0)

CHECKSUMS
  rack (3.0.8) sha256=abcdef1234567890

BUNDLED WITH
   2.4.10
";

    let lockfile = Lockfile::parse(lockfile_content).unwrap();
    assert!(!lockfile.gems.is_empty());

    let rack = lockfile.gems.iter().find(|g| g.name == "rack").unwrap();
    assert_eq!(rack.version, "3.0.8");
    assert_eq!(rack.checksum.as_deref(), Some("abcdef1234567890"));
}

// Help command tests

/// Test that general help works
#[test]
fn help_general() {
    let output = Command::new(get_lode_binary())
        .arg("--help")
        .output()
        .expect("Failed to execute lode --help");

    assert!(output.status.success(), "lode --help should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Usage: lode"), "Help should show usage");
    assert!(stdout.contains("Commands:"), "Help should list commands");
}

/// Test that help with -h flag works
#[test]
fn help_short_flag() {
    let output = Command::new(get_lode_binary())
        .arg("-h")
        .output()
        .expect("Failed to execute lode -h");

    assert!(output.status.success(), "lode -h should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Usage: lode"), "Help should show usage");
}

/// Test that help subcommand works
#[test]
fn help_subcommand() {
    let output = Command::new(get_lode_binary())
        .arg("help")
        .output()
        .expect("Failed to execute lode help");

    assert!(output.status.success(), "lode help should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Usage: lode"), "Help should show usage");
    assert!(
        stdout.contains("install"),
        "Help should list install command"
    );
    assert!(stdout.contains("update"), "Help should list update command");
}

/// Test that help for specific command works
#[test]
fn help_for_command() {
    let output = Command::new(get_lode_binary())
        .args(["help", "install"])
        .output()
        .expect("Failed to execute lode help install");

    assert!(output.status.success(), "lode help install should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Install gems"),
        "Help should describe install command"
    );
    assert!(stdout.contains("Options:"), "Help should show options");
}

/// Test that command --help works
#[test]
fn command_help_flag() {
    let output = Command::new(get_lode_binary())
        .args(["install", "--help"])
        .output()
        .expect("Failed to execute lode install --help");

    assert!(
        output.status.success(),
        "lode install --help should succeed"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Install gems"),
        "Help should describe install command"
    );
}

/// Test that command -h works
#[test]
fn command_help_short_flag() {
    let output = Command::new(get_lode_binary())
        .args(["install", "-h"])
        .output()
        .expect("Failed to execute lode install -h");

    assert!(output.status.success(), "lode install -h should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Install gems"),
        "Help should describe install command"
    );
}
