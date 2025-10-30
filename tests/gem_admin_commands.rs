//! Integration tests for admin/auth gem commands
//!
//! Tests gem commands for administration, authentication, and server operations:
//! - gem-sources: Manage gem sources
//! - gem-server: Run local gem server
//! - gem-push: Upload gems to `RubyGems`
//! - gem-yank: Remove gems from `RubyGems`
//! - gem-signin: Authenticate with `RubyGems`
//! - gem-signout: Logout from `RubyGems`
//! - gem-cert: Manage gem signing certificates
//! - gem-mirror: Mirror gem repositories
//! - gem-rdoc: Generate `RDoc` documentation
//! - gem-help: Show help for gem commands

mod common;

use std::process::Command;

use common::get_lode_binary;

// ============================================================================
// gem-sources Tests - Manage gem sources
// ============================================================================

/// Test gem-sources lists sources
#[test]
fn gem_sources_list() {
    let output = Command::new(get_lode_binary())
        .args(["gem-sources"])
        .output()
        .expect("Failed to execute lode gem-sources");

    // Should list gem sources successfully
    assert!(
        output.status.success() || output.status.code() == Some(0),
        "gem-sources should succeed"
    );
}

/// Test gem-sources --list flag
#[test]
fn gem_sources_list_flag() {
    let output = Command::new(get_lode_binary())
        .args(["gem-sources", "--list"])
        .output()
        .expect("Failed to execute lode gem-sources --list");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-sources should accept --list flag. stderr: {stderr}"
    );
}

/// Test gem-sources --add flag
#[test]
fn gem_sources_add_flag() {
    let output = Command::new(get_lode_binary())
        .args(["gem-sources", "--add", "https://example.com"])
        .output()
        .expect("Failed to execute lode gem-sources --add");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-sources should accept --add flag. stderr: {stderr}"
    );
}

/// Test gem-sources --update flag
#[test]
fn gem_sources_update_flag() {
    let output = Command::new(get_lode_binary())
        .args(["gem-sources", "--update"])
        .output()
        .expect("Failed to execute lode gem-sources --update");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-sources should accept --update flag. stderr: {stderr}"
    );
}

// ============================================================================
// gem-server Tests - Run local gem server
// ============================================================================

/// Test gem-server accepts port flag
#[test]
fn gem_server_port_flag() {
    let output = Command::new(get_lode_binary())
        .args(["gem-server", "--port", "8808"])
        .output()
        .expect("Failed to execute lode gem-server --port");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-server should accept --port flag. stderr: {stderr}"
    );
}

/// Test gem-server --bind flag
#[test]
fn gem_server_bind_flag() {
    let output = Command::new(get_lode_binary())
        .args(["gem-server", "--bind", "localhost"])
        .output()
        .expect("Failed to execute lode gem-server --bind");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-server should accept --bind flag. stderr: {stderr}"
    );
}

/// Test gem-server --dir flag
#[test]
fn gem_server_dir_flag() {
    let output = Command::new(get_lode_binary())
        .args(["gem-server", "--dir", "/tmp"])
        .output()
        .expect("Failed to execute lode gem-server --dir");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-server should accept --dir flag. stderr: {stderr}"
    );
}

/// Test gem-server --daemon flag
#[test]
fn gem_server_daemon_flag() {
    let output = Command::new(get_lode_binary())
        .args(["gem-server", "--daemon"])
        .output()
        .expect("Failed to execute lode gem-server --daemon");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-server should accept --daemon flag. stderr: {stderr}"
    );
}

// ============================================================================
// gem-push Tests - Upload gems to RubyGems
// ============================================================================

/// Test gem-push with gem file argument
#[test]
fn gem_push_gem_file() {
    let output = Command::new(get_lode_binary())
        .args(["gem-push", "nonexistent.gem"])
        .output()
        .expect("Failed to execute lode gem-push");

    // Should handle missing file gracefully
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-push should accept gem file. stderr: {stderr}"
    );
}

/// Test gem-push --key flag
#[test]
fn gem_push_key_flag() {
    let output = Command::new(get_lode_binary())
        .args(["gem-push", "test.gem", "--key", "my-key"])
        .output()
        .expect("Failed to execute lode gem-push --key");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-push should accept --key flag. stderr: {stderr}"
    );
}

/// Test gem-push --otp flag
#[test]
fn gem_push_otp_flag() {
    let output = Command::new(get_lode_binary())
        .args(["gem-push", "test.gem", "--otp", "123456"])
        .output()
        .expect("Failed to execute lode gem-push --otp");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-push should accept --otp flag. stderr: {stderr}"
    );
}

/// Test gem-push --host flag
#[test]
fn gem_push_host_flag() {
    let output = Command::new(get_lode_binary())
        .args([
            "gem-push",
            "test.gem",
            "--host",
            "https://custom.example.com",
        ])
        .output()
        .expect("Failed to execute lode gem-push --host");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-push should accept --host flag. stderr: {stderr}"
    );
}

// ============================================================================
// gem-yank Tests - Remove gems from RubyGems
// ============================================================================

/// Test gem-yank with gem name
#[test]
fn gem_yank_gem_name() {
    let output = Command::new(get_lode_binary())
        .args(["gem-yank", "example-gem"])
        .output()
        .expect("Failed to execute lode gem-yank");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-yank should accept gem name. stderr: {stderr}"
    );
}

/// Test gem-yank --version flag
#[test]
fn gem_yank_version_flag() {
    let output = Command::new(get_lode_binary())
        .args(["gem-yank", "example-gem", "--version", "1.0.0"])
        .output()
        .expect("Failed to execute lode gem-yank --version");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-yank should accept --version flag. stderr: {stderr}"
    );
}

/// Test gem-yank --platform flag
#[test]
fn gem_yank_platform_flag() {
    let output = Command::new(get_lode_binary())
        .args(["gem-yank", "example-gem", "--platform", "x86_64-linux"])
        .output()
        .expect("Failed to execute lode gem-yank --platform");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-yank should accept --platform flag. stderr: {stderr}"
    );
}

// ============================================================================
// gem-signin Tests - Authenticate with RubyGems
// ============================================================================

/// Test gem-signin --host flag
#[test]
fn gem_signin_host_flag() {
    let output = Command::new(get_lode_binary())
        .args(["gem-signin", "--host", "https://rubygems.org"])
        .output()
        .expect("Failed to execute lode gem-signin --host");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-signin should accept --host flag. stderr: {stderr}"
    );
}

/// Test gem-signin --verbose flag
#[test]
fn gem_signin_verbose_flag() {
    let output = Command::new(get_lode_binary())
        .args(["gem-signin", "--verbose"])
        .output()
        .expect("Failed to execute lode gem-signin --verbose");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-signin should accept --verbose flag. stderr: {stderr}"
    );
}

// ============================================================================
// gem-signout Tests - Logout from RubyGems
// ============================================================================

/// Test gem-signout command
#[test]
fn gem_signout_basic() {
    let output = Command::new(get_lode_binary())
        .args(["gem-signout"])
        .output()
        .expect("Failed to execute lode gem-signout");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-signout should execute. stderr: {stderr}"
    );
}

/// Test gem-signout --verbose flag
#[test]
fn gem_signout_verbose_flag() {
    let output = Command::new(get_lode_binary())
        .args(["gem-signout", "--verbose"])
        .output()
        .expect("Failed to execute lode gem-signout --verbose");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-signout should accept --verbose flag. stderr: {stderr}"
    );
}

// ============================================================================
// gem-cert Tests - Manage gem signing certificates
// ============================================================================

/// Test gem-cert --build flag
#[test]
fn gem_cert_build_flag() {
    let output = Command::new(get_lode_binary())
        .args(["gem-cert", "--build", "test@example.com"])
        .output()
        .expect("Failed to execute lode gem-cert --build");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-cert should accept --build flag. stderr: {stderr}"
    );
}

/// Test gem-cert --list flag
#[test]
fn gem_cert_list_flag() {
    let output = Command::new(get_lode_binary())
        .args(["gem-cert", "--list"])
        .output()
        .expect("Failed to execute lode gem-cert --list");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-cert should accept --list flag. stderr: {stderr}"
    );
}

/// Test gem-cert --add flag
#[test]
fn gem_cert_add_flag() {
    let output = Command::new(get_lode_binary())
        .args(["gem-cert", "--add", "cert.pem"])
        .output()
        .expect("Failed to execute lode gem-cert --add");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-cert should accept --add flag. stderr: {stderr}"
    );
}

/// Test gem-cert --remove flag
#[test]
fn gem_cert_remove_flag() {
    let output = Command::new(get_lode_binary())
        .args(["gem-cert", "--remove", "cert_digest"])
        .output()
        .expect("Failed to execute lode gem-cert --remove");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-cert should accept --remove flag. stderr: {stderr}"
    );
}

/// Test gem-cert --sign flag
#[test]
fn gem_cert_sign_flag() {
    let output = Command::new(get_lode_binary())
        .args(["gem-cert", "--sign", "cert_digest"])
        .output()
        .expect("Failed to execute lode gem-cert --sign");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-cert should accept --sign flag. stderr: {stderr}"
    );
}

// ============================================================================
// gem-mirror Tests - Mirror gem repositories
// ============================================================================

/// Test gem-mirror --list flag
#[test]
fn gem_mirror_list_flag() {
    let output = Command::new(get_lode_binary())
        .args(["gem-mirror", "--list"])
        .output()
        .expect("Failed to execute lode gem-mirror --list");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-mirror should accept --list flag. stderr: {stderr}"
    );
}

/// Test gem-mirror --add flag
#[test]
fn gem_mirror_add_flag() {
    let output = Command::new(get_lode_binary())
        .args(["gem-mirror", "--add", "https://example.com"])
        .output()
        .expect("Failed to execute lode gem-mirror --add");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-mirror should accept --add flag. stderr: {stderr}"
    );
}

/// Test gem-mirror --remove flag
#[test]
fn gem_mirror_remove_flag() {
    let output = Command::new(get_lode_binary())
        .args(["gem-mirror", "--remove", "https://example.com"])
        .output()
        .expect("Failed to execute lode gem-mirror --remove");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-mirror should accept --remove flag. stderr: {stderr}"
    );
}

// ============================================================================
// gem-rdoc Tests - Generate RDoc documentation
// ============================================================================

/// Test gem-rdoc with single gem name
#[test]
fn gem_rdoc_with_gem() {
    let output = Command::new(get_lode_binary())
        .args(["gem-rdoc", "bundler"])
        .output()
        .expect("Failed to execute lode gem-rdoc");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-rdoc should accept gem name. stderr: {stderr}"
    );
}

/// Test gem-rdoc --all flag
#[test]
fn gem_rdoc_all_flag() {
    let output = Command::new(get_lode_binary())
        .args(["gem-rdoc", "--all"])
        .output()
        .expect("Failed to execute lode gem-rdoc --all");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-rdoc should accept --all flag. stderr: {stderr}"
    );
}

/// Test gem-rdoc --version flag
#[test]
fn gem_rdoc_version_flag() {
    let output = Command::new(get_lode_binary())
        .args(["gem-rdoc", "bundler", "--version", "2.4.6"])
        .output()
        .expect("Failed to execute lode gem-rdoc --version");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-rdoc should accept --version flag. stderr: {stderr}"
    );
}

/// Test gem-rdoc --ri flag
#[test]
fn gem_rdoc_ri_flag() {
    let output = Command::new(get_lode_binary())
        .args(["gem-rdoc", "bundler", "--ri"])
        .output()
        .expect("Failed to execute lode gem-rdoc --ri");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-rdoc should accept --ri flag. stderr: {stderr}"
    );
}

// ============================================================================
// gem-help Tests - Show help for gem commands
// ============================================================================

/// Test gem-help shows help
#[test]
fn gem_help_basic() {
    let output = Command::new(get_lode_binary())
        .args(["gem-help"])
        .output()
        .expect("Failed to execute lode gem-help");

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should display help output
    assert!(
        !stdout.is_empty() || output.status.success(),
        "gem-help should produce output or succeed"
    );
}

/// Test gem-help with command name
#[test]
fn gem_help_with_command() {
    let output = Command::new(get_lode_binary())
        .args(["gem-help", "install"])
        .output()
        .expect("Failed to execute lode gem-help install");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-help should accept command name. stderr: {stderr}"
    );
}

/// Test gem-help --verbose flag
#[test]
fn gem_help_verbose_flag() {
    let output = Command::new(get_lode_binary())
        .args(["gem-help", "--verbose"])
        .output()
        .expect("Failed to execute lode gem-help --verbose");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "gem-help should accept --verbose flag. stderr: {stderr}"
    );
}
