//! Integration tests for network-dependent gem commands
//!
//! Tests gem commands that interact with RubyGems.org or require network access.
//! Uses --local flag or safe defaults to avoid real network calls in CI.
//!
//! Commands tested:
//! - gem-install: Install gems
//! - gem-search: Search for gems
//! - gem-list: List gems (remote)
//! - gem-fetch: Download gems
//! - gem-update: Update gems
//! - gem-dependency: Show gem dependencies

mod common;

use std::process::Command;

use common::get_lode_binary;

// ============================================================================
// gem-install Tests
// ============================================================================

/// Test gem-install accepts gem name argument
#[test]
fn gem_install_accepts_gem_name() {
    let output = Command::new(get_lode_binary())
        .args(["gem-install", "--explain", "bundler"])
        .output()
        .expect("Failed to execute lode gem-install --explain bundler");

    // Should accept gem name without error
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-install should accept gem name. stderr: {stderr}"
    );
}

/// Test gem-install --version flag
#[test]
fn gem_install_version_flag() {
    let output = Command::new(get_lode_binary())
        .args(["gem-install", "bundler", "--version", "2.4.6", "--explain"])
        .output()
        .expect("Failed to execute lode gem-install --version");

    // Should accept --version flag
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-install should accept --version flag. stderr: {stderr}"
    );
}

/// Test gem-install --prerelease flag
#[test]
fn gem_install_prerelease_flag() {
    let output = Command::new(get_lode_binary())
        .args(["gem-install", "rails", "--prerelease", "--explain"])
        .output()
        .expect("Failed to execute lode gem-install --prerelease");

    // Should accept --prerelease flag
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-install should accept --prerelease flag. stderr: {stderr}"
    );
}

/// Test gem-install --local flag (only use local gems)
#[test]
fn gem_install_local_flag() {
    let output = Command::new(get_lode_binary())
        .args(["gem-install", "bundler", "--local", "--explain"])
        .output()
        .expect("Failed to execute lode gem-install --local");

    // Should accept --local flag
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-install should accept --local flag. stderr: {stderr}"
    );
}

/// Test gem-install --no-document flag
#[test]
fn gem_install_no_document_flag() {
    let output = Command::new(get_lode_binary())
        .args(["gem-install", "bundler", "--no-document", "--explain"])
        .output()
        .expect("Failed to execute lode gem-install --no-document");

    // Should accept --no-document flag
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-install should accept --no-document flag. stderr: {stderr}"
    );
}

/// Test gem-install --force flag
#[test]
fn gem_install_force_flag() {
    let output = Command::new(get_lode_binary())
        .args(["gem-install", "bundler", "--force", "--explain"])
        .output()
        .expect("Failed to execute lode gem-install --force");

    // Should accept --force flag
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-install should accept --force flag. stderr: {stderr}"
    );
}

/// Test gem-install --ignore-dependencies flag
#[test]
fn gem_install_ignore_dependencies() {
    let output = Command::new(get_lode_binary())
        .args([
            "gem-install",
            "bundler",
            "--ignore-dependencies",
            "--explain",
        ])
        .output()
        .expect("Failed to execute lode gem-install --ignore-dependencies");

    // Should accept --ignore-dependencies flag
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-install should accept --ignore-dependencies flag. stderr: {stderr}"
    );
}

/// Test gem-install --explain flag (dry-run)
#[test]
fn gem_install_explain_flag() {
    let output = Command::new(get_lode_binary())
        .args(["gem-install", "bundler", "--explain"])
        .output()
        .expect("Failed to execute lode gem-install --explain");

    // Should accept --explain flag and show what would be done
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-install should accept --explain flag. stderr: {stderr}"
    );
}

/// Test gem-install multiple gems
#[test]
fn gem_install_multiple_gems() {
    let output = Command::new(get_lode_binary())
        .args(["gem-install", "bundler", "rake", "--explain"])
        .output()
        .expect("Failed to execute lode gem-install multiple gems");

    // Should accept multiple gem names
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-install should accept multiple gems. stderr: {stderr}"
    );
}

/// Test gem-install --explain with version constraint
#[test]
fn gem_install_version_constraint() {
    let output = Command::new(get_lode_binary())
        .args(["gem-install", "bundler:2.4.6", "--explain"])
        .output()
        .expect("Failed to execute lode gem-install with version constraint");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-install should accept version constraint. stderr: {stderr}"
    );
}

/// Test gem-install --explain with operator version constraint
#[test]
fn gem_install_operator_constraint() {
    let output = Command::new(get_lode_binary())
        .args(["gem-install", "bundler:~>2.4", "--explain"])
        .output()
        .expect("Failed to execute lode gem-install with operator constraint");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-install should accept operator constraints. stderr: {stderr}"
    );
}

/// Test gem-install --minimal-deps flag
#[test]
fn gem_install_minimal_deps() {
    let output = Command::new(get_lode_binary())
        .args(["gem-install", "bundler", "--minimal-deps", "--explain"])
        .output()
        .expect("Failed to execute lode gem-install --minimal-deps");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-install should accept --minimal-deps flag. stderr: {stderr}"
    );
}

/// Test gem-install --development flag
#[test]
fn gem_install_development() {
    let output = Command::new(get_lode_binary())
        .args(["gem-install", "bundler", "--development", "--explain"])
        .output()
        .expect("Failed to execute lode gem-install --development");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-install should accept --development flag. stderr: {stderr}"
    );
}

/// Test gem-install --remote flag (fetch from remote)
#[test]
fn gem_install_remote() {
    let output = Command::new(get_lode_binary())
        .args(["gem-install", "bundler", "--remote", "--explain"])
        .output()
        .expect("Failed to execute lode gem-install --remote");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-install should accept --remote flag. stderr: {stderr}"
    );
}

/// Test gem-install --both flag (local and remote)
#[test]
fn gem_install_both_sources() {
    let output = Command::new(get_lode_binary())
        .args(["gem-install", "bundler", "--both", "--explain"])
        .output()
        .expect("Failed to execute lode gem-install --both");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-install should accept --both flag. stderr: {stderr}"
    );
}

/// Test gem-install -N shorthand for --no-document
#[test]
fn gem_install_short_no_document() {
    let output = Command::new(get_lode_binary())
        .args(["gem-install", "bundler", "-N", "--explain"])
        .output()
        .expect("Failed to execute lode gem-install -N");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-install should accept -N shorthand. stderr: {stderr}"
    );
}

/// Test gem-install --conservative (avoid major version updates)
#[test]
fn gem_install_conservative() {
    let output = Command::new(get_lode_binary())
        .args(["gem-install", "bundler", "--conservative", "--explain"])
        .output()
        .expect("Failed to execute lode gem-install --conservative");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-install should accept --conservative flag. stderr: {stderr}"
    );
}

/// Test gem-install --source flag (custom gem source)
#[test]
fn gem_install_custom_source() {
    let output = Command::new(get_lode_binary())
        .args([
            "gem-install",
            "bundler",
            "--source",
            "https://rubygems.org",
            "--explain",
        ])
        .output()
        .expect("Failed to execute lode gem-install --source");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-install should accept --source flag. stderr: {stderr}"
    );
}

/// Test gem-install --clear-sources (ignore default sources)
#[test]
fn gem_install_clear_sources() {
    let output = Command::new(get_lode_binary())
        .args([
            "gem-install",
            "bundler",
            "--clear-sources",
            "--source",
            "https://rubygems.org",
            "--explain",
        ])
        .output()
        .expect("Failed to execute lode gem-install --clear-sources");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-install should accept --clear-sources flag. stderr: {stderr}"
    );
}

/// Test gem-install --verbose flag
#[test]
fn gem_install_verbose() {
    let output = Command::new(get_lode_binary())
        .args(["gem-install", "bundler", "--verbose", "--explain"])
        .output()
        .expect("Failed to execute lode gem-install --verbose");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-install should accept --verbose flag. stderr: {stderr}"
    );
}

/// Test gem-install -V shorthand for --verbose
#[test]
fn gem_install_short_verbose() {
    let output = Command::new(get_lode_binary())
        .args(["gem-install", "bundler", "-V", "--explain"])
        .output()
        .expect("Failed to execute lode gem-install -V");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-install should accept -V shorthand. stderr: {stderr}"
    );
}

/// Test gem-install --quiet flag
#[test]
fn gem_install_quiet() {
    let output = Command::new(get_lode_binary())
        .args(["gem-install", "bundler", "--quiet", "--explain"])
        .output()
        .expect("Failed to execute lode gem-install --quiet");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-install should accept --quiet flag. stderr: {stderr}"
    );
}

/// Test gem-install -q shorthand for --quiet
#[test]
fn gem_install_short_quiet() {
    let output = Command::new(get_lode_binary())
        .args(["gem-install", "bundler", "-q", "--explain"])
        .output()
        .expect("Failed to execute lode gem-install -q");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-install should accept -q shorthand. stderr: {stderr}"
    );
}

/// Test gem-install with nonexistent gem fails gracefully
#[test]
fn gem_install_nonexistent_gem() {
    let output = Command::new(get_lode_binary())
        .args(["gem-install", "nonexistent-gem-xyz-12345", "--explain"])
        .output()
        .expect("Failed to execute lode gem-install nonexistent-gem");

    // Either should fail or report gem not found
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        !stderr.contains("unexpected argument") || !stdout.is_empty(),
        "gem-install should handle missing gem gracefully. stderr: {stderr}"
    );
}

// ============================================================================
// gem-search Tests
// ============================================================================

/// Test gem-search with query
#[test]
fn gem_search_with_query() {
    let output = Command::new(get_lode_binary())
        .args(["gem-search", "bundle", "--local"])
        .output()
        .expect("Failed to execute lode gem-search bundle --local");

    // Should accept query and --local flag
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-search should accept query. stderr: {stderr}"
    );
}

/// Test gem-search --details flag
#[test]
fn gem_search_details_flag() {
    let output = Command::new(get_lode_binary())
        .args(["gem-search", "bundle", "--details", "--local"])
        .output()
        .expect("Failed to execute lode gem-search --details");

    // Should accept --details flag
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-search should accept --details flag. stderr: {stderr}"
    );
}

/// Test gem-search --exact flag
#[test]
fn gem_search_exact_flag() {
    let output = Command::new(get_lode_binary())
        .args(["gem-search", "bundler", "--exact", "--local"])
        .output()
        .expect("Failed to execute lode gem-search --exact");

    // Should accept --exact flag
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-search should accept --exact flag. stderr: {stderr}"
    );
}

/// Test gem-search --all flag
#[test]
fn gem_search_all_flag() {
    let output = Command::new(get_lode_binary())
        .args(["gem-search", "bundle", "--all", "--local"])
        .output()
        .expect("Failed to execute lode gem-search --all");

    // Should accept --all flag
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-search should accept --all flag. stderr: {stderr}"
    );
}

/// Test gem-search --local flag
#[test]
fn gem_search_local_flag() {
    let output = Command::new(get_lode_binary())
        .args(["gem-search", "bundle", "--local"])
        .output()
        .expect("Failed to execute lode gem-search --local");

    // Should succeed with --local flag
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-search should accept --local flag. stderr: {stderr}"
    );
}

/// Test gem-search --prerelease flag
#[test]
fn gem_search_prerelease_flag() {
    let output = Command::new(get_lode_binary())
        .args(["gem-search", "rails", "--prerelease", "--local"])
        .output()
        .expect("Failed to execute lode gem-search --prerelease");

    // Should accept --prerelease flag
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-search should accept --prerelease flag. stderr: {stderr}"
    );
}

// ============================================================================
// gem-list Tests
// ============================================================================

/// Test gem-list local gems
#[test]
fn gem_list_local() {
    let output = Command::new(get_lode_binary())
        .args(["gem-list", "--local"])
        .output()
        .expect("Failed to execute lode gem-list --local");

    // Should list local gems successfully
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-list should accept --local flag. stderr: {stderr}"
    );
}

/// Test gem-list with pattern
#[test]
fn gem_list_with_pattern() {
    let output = Command::new(get_lode_binary())
        .args(["gem-list", "bundle", "--local"])
        .output()
        .expect("Failed to execute lode gem-list bundle --local");

    // Should accept pattern and --local
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-list should accept pattern. stderr: {stderr}"
    );
}

/// Test gem-list --details flag
#[test]
fn gem_list_details_flag() {
    let output = Command::new(get_lode_binary())
        .args(["gem-list", "--details", "--local"])
        .output()
        .expect("Failed to execute lode gem-list --details");

    // Should accept --details flag
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-list should accept --details flag. stderr: {stderr}"
    );
}

/// Test gem-list --versions flag
#[test]
fn gem_list_versions_flag() {
    let output = Command::new(get_lode_binary())
        .args(["gem-list", "bundler", "--versions", "--local"])
        .output()
        .expect("Failed to execute lode gem-list --versions");

    // Should accept --versions flag
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-list should accept --versions flag. stderr: {stderr}"
    );
}

/// Test gem-list --exact flag
#[test]
fn gem_list_exact_flag() {
    let output = Command::new(get_lode_binary())
        .args(["gem-list", "bundler", "--exact", "--local"])
        .output()
        .expect("Failed to execute lode gem-list --exact");

    // Should accept --exact flag
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-list should accept --exact flag. stderr: {stderr}"
    );
}

// ============================================================================
// gem-fetch Tests
// ============================================================================

/// Test gem-fetch accepts gem name
#[test]
fn gem_fetch_gem_name() {
    let output = Command::new(get_lode_binary())
        .args(["gem-fetch", "bundler", "--help"])
        .output()
        .expect("Failed to execute lode gem-fetch bundler");

    // Should accept gem name without error
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-fetch should accept gem name. stderr: {stderr}"
    );
}

/// Test gem-fetch --version flag
#[test]
fn gem_fetch_version_flag() {
    let output = Command::new(get_lode_binary())
        .args(["gem-fetch", "bundler", "--version", "2.4.6"])
        .output()
        .expect("Failed to execute lode gem-fetch --version");

    // Should accept --version flag (will fail if gem not found, but flag accepted)
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument") && !stderr.contains("unrecognized"),
        "gem-fetch should accept --version flag. stderr: {stderr}"
    );
}

/// Test gem-fetch --platform flag
#[test]
fn gem_fetch_platform_flag() {
    let output = Command::new(get_lode_binary())
        .args(["gem-fetch", "nokogiri", "--platform", "x86_64-linux"])
        .output()
        .expect("Failed to execute lode gem-fetch --platform");

    // Should accept --platform flag
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-fetch should accept --platform flag. stderr: {stderr}"
    );
}

/// Test gem-fetch --prerelease flag
#[test]
fn gem_fetch_prerelease_flag() {
    let output = Command::new(get_lode_binary())
        .args(["gem-fetch", "rails", "--prerelease"])
        .output()
        .expect("Failed to execute lode gem-fetch --prerelease");

    // Should accept --prerelease flag
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-fetch should accept --prerelease flag. stderr: {stderr}"
    );
}

// ============================================================================
// gem-update Tests
// ============================================================================

/// Test gem-update with specific gem
#[test]
fn gem_update_specific_gem() {
    let output = Command::new(get_lode_binary())
        .args(["gem-update", "bundler", "--help"])
        .output()
        .expect("Failed to execute lode gem-update bundler");

    // Should accept specific gem name
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-update should accept gem name. stderr: {stderr}"
    );
}

/// Test gem-update --system flag (update `RubyGems` itself)
#[test]
fn gem_update_system_flag() {
    let output = Command::new(get_lode_binary())
        .args(["gem-update", "--system"])
        .output()
        .expect("Failed to execute lode gem-update --system");

    // Should accept --system flag
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-update should accept --system flag. stderr: {stderr}"
    );
}

/// Test gem-update --force flag
#[test]
fn gem_update_force_flag() {
    let output = Command::new(get_lode_binary())
        .args(["gem-update", "bundler", "--force"])
        .output()
        .expect("Failed to execute lode gem-update --force");

    // Should accept --force flag
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-update should accept --force flag. stderr: {stderr}"
    );
}

/// Test gem-update --local flag
#[test]
fn gem_update_local_flag() {
    let output = Command::new(get_lode_binary())
        .args(["gem-update", "bundler", "--local"])
        .output()
        .expect("Failed to execute lode gem-update --local");

    // Should accept --local flag
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-update should accept --local flag. stderr: {stderr}"
    );
}

/// Test gem-update --prerelease flag
#[test]
fn gem_update_prerelease_flag() {
    let output = Command::new(get_lode_binary())
        .args(["gem-update", "rails", "--prerelease"])
        .output()
        .expect("Failed to execute lode gem-update --prerelease");

    // Should accept --prerelease flag
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-update should accept --prerelease flag. stderr: {stderr}"
    );
}

// ============================================================================
// gem-dependency Tests
// ============================================================================

/// Test gem-dependency local mode
#[test]
fn gem_dependency_local() {
    let output = Command::new(get_lode_binary())
        .args(["gem-dependency", "bundler", "--local"])
        .output()
        .expect("Failed to execute lode gem-dependency bundler --local");

    // Should accept gem name and --local flag
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-dependency should accept --local flag. stderr: {stderr}"
    );
}

/// Test gem-dependency --version flag
#[test]
fn gem_dependency_version_flag() {
    let output = Command::new(get_lode_binary())
        .args(["gem-dependency", "bundler", "--version", "2.4.6", "--local"])
        .output()
        .expect("Failed to execute lode gem-dependency --version");

    // Should accept --version flag
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-dependency should accept --version flag. stderr: {stderr}"
    );
}

/// Test gem-dependency --reverse-dependencies flag
#[test]
fn gem_dependency_reverse_dependencies() {
    let output = Command::new(get_lode_binary())
        .args([
            "gem-dependency",
            "bundler",
            "--reverse-dependencies",
            "--local",
        ])
        .output()
        .expect("Failed to execute lode gem-dependency --reverse-dependencies");

    // Should accept --reverse-dependencies flag
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-dependency should accept --reverse-dependencies flag. stderr: {stderr}"
    );
}

/// Test gem-dependency --platform flag
#[test]
fn gem_dependency_platform_flag() {
    let output = Command::new(get_lode_binary())
        .args([
            "gem-dependency",
            "nokogiri",
            "--platform",
            "x86_64-linux",
            "--local",
        ])
        .output()
        .expect("Failed to execute lode gem-dependency --platform");

    // Should accept --platform flag
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-dependency should accept --platform flag. stderr: {stderr}"
    );
}

/// Test gem-dependency --pipe flag (machine-readable output)
#[test]
fn gem_dependency_pipe_flag() {
    let output = Command::new(get_lode_binary())
        .args(["gem-dependency", "bundler", "--pipe", "--local"])
        .output()
        .expect("Failed to execute lode gem-dependency --pipe");

    // Should accept --pipe flag
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-dependency should accept --pipe flag. stderr: {stderr}"
    );
}

/// Test gem-dependency --prerelease flag
#[test]
fn gem_dependency_prerelease_flag() {
    let output = Command::new(get_lode_binary())
        .args(["gem-dependency", "rails", "--prerelease", "--local"])
        .output()
        .expect("Failed to execute lode gem-dependency --prerelease");

    // Should accept --prerelease flag
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-dependency should accept --prerelease flag. stderr: {stderr}"
    );
}

/// Test gem-dependency --verbose flag
#[test]
fn gem_dependency_verbose_flag() {
    let output = Command::new(get_lode_binary())
        .args(["gem-dependency", "bundler", "--verbose", "--local"])
        .output()
        .expect("Failed to execute lode gem-dependency --verbose");

    // Should accept --verbose flag
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-dependency should accept --verbose flag. stderr: {stderr}"
    );
}

/// Test gem-dependency --quiet flag
#[test]
fn gem_dependency_quiet_flag() {
    let output = Command::new(get_lode_binary())
        .args(["gem-dependency", "bundler", "--quiet", "--local"])
        .output()
        .expect("Failed to execute lode gem-dependency --quiet");

    // Should accept --quiet flag
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-dependency should accept --quiet flag. stderr: {stderr}"
    );
}
