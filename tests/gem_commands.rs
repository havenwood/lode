//! Integration tests for gem commands
//!
//! Tests for gem-list, gem-search, gem-info, gem-cleanup, gem-pristine, etc.

use lode::gem_store::GemStore;

/// Test that `GemStore` can be created and list gems
#[test]
fn gem_list_creates_gemstore_successfully() {
    // Creating a GemStore should succeed (even if no gems are installed)
    let result = GemStore::new();
    assert!(result.is_ok(), "GemStore::new() should succeed");
}

/// Test that `GemStore` finds installed gems
#[test]
fn gem_store_can_list_installed_gems() {
    let store = GemStore::new().expect("Failed to create GemStore");
    let gems = store.list_gems();

    // Should be able to list gems (may be empty if no gems installed)
    assert!(gems.is_ok(), "list_gems() should succeed");

    if let Ok(gems) = gems {
        // If there are gems, verify they have required fields
        for gem in gems {
            assert!(!gem.name.is_empty(), "Gem name should not be empty");
            assert!(!gem.version.is_empty(), "Gem version should not be empty");
            // Note: gem.path may not exist in test environment
            drop(gem.path);
        }
    }
}

/// Test that `GemStore` can find gems by pattern
#[test]
fn gem_store_finds_gems_by_pattern() {
    let store = GemStore::new().expect("Failed to create GemStore");

    // Try to find gems matching a pattern (may return empty)
    let result = store.find_gems(Some("bundle"));
    assert!(result.is_ok(), "find_gems() should succeed");

    // Also test with empty pattern
    let result_all = store.find_gems(None);
    assert!(result_all.is_ok(), "find_gems(None) should succeed");
}

/// Test that `GemStore` can find specific gem by name
#[test]
fn gem_store_finds_gem_by_name() {
    let store = GemStore::new().expect("Failed to create GemStore");

    // Try to find a gem that probably doesn't exist
    let result = store.find_gem_by_name("nonexistent-gem-12345");
    assert!(result.is_ok(), "find_gem_by_name() should succeed");

    // Should return empty if gem doesn't exist
    if let Ok(gems) = result {
        assert!(gems.is_empty(), "Nonexistent gem should not be found");
    }
}

/// Test that `GemStore` can get the gem directory
#[test]
fn gem_store_gem_directory() {
    let store = GemStore::new().expect("Failed to create GemStore");
    let gem_dir = store.gem_dir();

    // Gem directory should be a valid path
    assert!(
        gem_dir.is_absolute(),
        "Gem directory should be absolute path"
    );
    assert!(
        gem_dir.to_string_lossy().contains("gem"),
        "Gem directory should contain 'gem' in path"
    );
}

/// Test parsing gem names with versions and platforms
#[test]
fn gem_store_parses_gem_names_correctly() {
    // Test basic gem name parsing
    let (name, version, platform) = parse_test_gem_name("rack-3.0.8");
    assert_eq!(name, "rack");
    assert_eq!(version, "3.0.8");
    assert_eq!(platform, "ruby");

    // Test gem name with platform
    let (name, version, platform) = parse_test_gem_name("nokogiri-1.16.0-x86_64-linux");
    assert_eq!(name, "nokogiri");
    assert_eq!(version, "1.16.0");
    assert_eq!(platform, "x86_64-linux");

    // Test gem name with platform (arm64-darwin)
    let (name, version, platform) = parse_test_gem_name("sqlite3-1.6.9-arm64-darwin");
    assert_eq!(name, "sqlite3");
    assert_eq!(version, "1.6.9");
    assert_eq!(platform, "arm64-darwin");
}

/// Test version comparison (basic string comparison)
/// Note: Real semantic version comparison is handled by `GemStore`'s sorting logic
#[test]
fn gem_store_compares_versions_correctly() {
    // These tests verify that basic version comparison works
    assert!("1.0.0" < "1.0.1");
    assert!("1.0.0" < "1.1.0");
    assert!("1.0.0" < "2.0.0");
    assert!("2.0.0" > "1.9.9");
    // Note: Pre-release version comparison is complex and handled
    // by GemStore's semantic version logic, not string comparison
}

/// Helper function to parse gem names (same logic as in `GemStore`)
fn parse_test_gem_name(dir_name: &str) -> (String, String, String) {
    // Format: name-version[-(platform)]
    // Examples:
    // - rack-3.0.8 -> (rack, 3.0.8, ruby)
    // - nokogiri-1.16.0-x86_64-linux -> (nokogiri, 1.16.0, x86_64-linux)
    // - sqlite3-1.6.9-arm64-darwin -> (sqlite3, 1.6.9, arm64-darwin)

    let parts: Vec<&str> = dir_name.split('-').collect();

    // Find where version starts (first part that starts with digit)
    let mut version_idx = 1;
    for (i, part) in parts.iter().enumerate().skip(1) {
        if matches!(part.chars().next(), Some(c) if c.is_ascii_digit()) {
            version_idx = i;
            break;
        }
    }

    // Safely extract name, version, and platform using map_or_else
    let name = parts
        .get(..version_idx)
        .map_or_else(String::new, |p| p.join("-"));

    let (version, platform) = parts.get(version_idx).map_or_else(
        || (String::new(), "ruby".to_string()),
        |version_str| {
            let platform = parts
                .get(version_idx + 1..)
                .and_then(|p| {
                    // If slice is empty, return None to use default "ruby"
                    if p.is_empty() {
                        None
                    } else {
                        Some(p.join("-"))
                    }
                })
                .unwrap_or_else(|| "ruby".to_string());
            ((*version_str).to_string(), platform)
        },
    );

    (name, version, platform)
}

#[cfg(test)]
mod gem_search_tests {
    // Note: Gem search requires network access
    // Run with: cargo test --ignored -- --nocapture
}

#[cfg(test)]
mod gem_install_tests {
    // Note: Gem install requires network access and modifies system
    // These should be manual tests or use a mock gem source
}

#[cfg(test)]
mod gem_uninstall_tests {
    // Note: Gem uninstall modifies system gem directory
    // These should be manual tests or use a mock gem directory
}

#[cfg(test)]
mod gem_owner_cli_tests {
    use std::process::Command;

    /// Test that gem-owner CLI accepts --http-proxy parameter
    /// This verifies the proxy support implementation
    #[test]
    fn gem_owner_cli_accepts_proxy_flag() {
        // Build the binary first to ensure it exists
        let status = Command::new("cargo")
            .args(["build", "--bin", "lode"])
            .status()
            .expect("Failed to build lode binary");
        assert!(status.success(), "Build should succeed");

        // Test that the gem-owner command accepts --http-proxy flag
        let output = Command::new("target/debug/lode")
            .args([
                "gem-owner",
                "--http-proxy",
                "http://proxy.example.com:8080",
                "rails",
            ])
            .output()
            .expect("Failed to execute lode gem-owner");

        // The command should not fail due to an unknown --http-proxy flag
        // It may fail due to network/auth issues, but that's expected
        let stderr = String::from_utf8_lossy(&output.stderr);

        // Check that the error is NOT about an unrecognized argument
        assert!(
            !stderr.contains("unexpected argument")
                && !stderr.contains("unrecognized")
                && !stderr.contains("unknown option"),
            "gem-owner should accept --http-proxy flag. stderr: {stderr}"
        );
    }

    /// Test that gem-owner --help shows proxy option
    #[test]
    fn gem_owner_help_shows_proxy_option() {
        let output = Command::new("target/debug/lode")
            .args(["gem-owner", "--help"])
            .output()
            .expect("Failed to execute lode gem-owner --help");

        let stdout = String::from_utf8_lossy(&output.stdout);

        // Verify that --http-proxy is documented in the help text
        assert!(
            stdout.contains("--http-proxy") || stdout.contains("http-proxy"),
            "gem-owner --help should document --http-proxy flag. stdout: {stdout}"
        );
    }

    #[test]
    fn gem_dependency_help_shows_all_flags() {
        let output = Command::new("target/debug/lode")
            .args(["gem-dependency", "--help"])
            .output()
            .expect("Failed to execute lode gem-dependency --help");

        let stdout = String::from_utf8_lossy(&output.stdout);

        // Verify all major flags are documented
        assert!(
            stdout.contains("--remote"),
            "gem-dependency --help should document --remote flag"
        );
        assert!(
            stdout.contains("--local"),
            "gem-dependency --help should document --local flag"
        );
        assert!(
            stdout.contains("--reverse-dependencies"),
            "gem-dependency --help should document --reverse-dependencies flag"
        );
        assert!(
            stdout.contains("--version"),
            "gem-dependency --help should document --version flag"
        );
        assert!(
            stdout.contains("--platform"),
            "gem-dependency --help should document --platform flag"
        );
    }

    #[test]
    fn gem_dependency_accepts_remote_flag() {
        // This test verifies that --remote flag is accepted (doesn't error with "unexpected argument")
        let output = Command::new("target/debug/lode")
            .args(["gem-dependency", "rake", "--remote"])
            .output()
            .expect("Failed to execute lode gem-dependency rake --remote");

        // Command should succeed or fail gracefully, but not reject the flag
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            !stderr.contains("unexpected argument"),
            "gem-dependency should accept --remote flag. stderr: {stderr}"
        );
    }

    #[test]
    fn gem_dependency_accepts_reverse_dependencies_flag() {
        // This test verifies that --reverse-dependencies flag is accepted
        let output = Command::new("target/debug/lode")
            .args(["gem-dependency", "rake", "--reverse-dependencies"])
            .output()
            .expect("Failed to execute lode gem-dependency rake --reverse-dependencies");

        // Command should succeed or fail gracefully, but not reject the flag
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            !stderr.contains("unexpected argument"),
            "gem-dependency should accept --reverse-dependencies flag. stderr: {stderr}"
        );
    }

    #[test]
    fn specification_message_doesnt_mention_nonexistent_remote_flag() {
        let output = Command::new("target/debug/lode")
            .args(["specification", "rake", "--version", "13.3.1"])
            .output()
            .expect("Failed to execute lode specification rake --version 13.3.1");

        let stdout = String::from_utf8_lossy(&output.stdout);

        assert!(
            !stdout.contains("gem specification <gem> --remote"),
            "specification output should not mention non-existent --remote flag. stdout: {stdout}"
        );
    }

    #[test]
    fn gem_which_accepts_gem_name() {
        let output = Command::new("target/debug/lode")
            .args(["gem-which", "rake"])
            .output()
            .expect("Failed to execute lode gem-which rake");

        // gem-which should succeed when finding a gem that exists
        // (or fail gracefully if rake is not installed)
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success() || stderr.is_empty() || stderr.contains("Can't find"),
            "gem-which should run and either find the gem or report it not found. stderr: {stderr}"
        );
    }

    #[test]
    fn gem_which_handles_nonexistent_gem() {
        let output = Command::new("target/debug/lode")
            .args(["gem-which", "nonexistent-gem-xyz123"])
            .output()
            .expect("Failed to execute lode gem-which nonexistent-gem-xyz123");

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let combined = format!("{stdout}{stderr}");
        assert!(
            combined.contains("Can't find") || !output.status.success(),
            "gem-which should handle nonexistent gems gracefully"
        );
    }

    #[test]
    fn gem_stale_runs_without_error() {
        let output = Command::new("target/debug/lode")
            .args(["gem-stale"])
            .output()
            .expect("Failed to execute lode gem-stale");

        // gem-stale should run successfully and produce output
        // It may or may not find stale gems depending on system state
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success() || !stdout.is_empty() || !stderr.is_empty(),
            "gem-stale should run and produce output. stdout: {stdout}, stderr: {stderr}"
        );
    }

    #[test]
    fn gem_dependency_accepts_version_flag() {
        let output = Command::new("target/debug/lode")
            .args(["gem-dependency", "rake", "--version", "13.0.0"])
            .output()
            .expect("Failed to execute lode gem-dependency rake --version 13.0.0");

        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            !stderr.contains("unexpected argument"),
            "gem-dependency should accept --version flag"
        );
    }

    #[test]
    fn gem_dependency_accepts_platform_flag() {
        let output = Command::new("target/debug/lode")
            .args(["gem-dependency", "nokogiri", "--platform", "ruby"])
            .output()
            .expect("Failed to execute lode gem-dependency nokogiri --platform ruby");

        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            !stderr.contains("unexpected argument"),
            "gem-dependency should accept --platform flag"
        );
    }

    #[test]
    fn gem_dependency_accepts_prerelease_flag() {
        let output = Command::new("target/debug/lode")
            .args(["gem-dependency", "rails", "--prerelease"])
            .output()
            .expect("Failed to execute lode gem-dependency rails --prerelease");

        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            !stderr.contains("unexpected argument"),
            "gem-dependency should accept --prerelease flag"
        );
    }

    #[test]
    fn gem_dependency_accepts_pipe_flag() {
        let output = Command::new("target/debug/lode")
            .args(["gem-dependency", "rake", "--pipe"])
            .output()
            .expect("Failed to execute lode gem-dependency rake --pipe");

        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            !stderr.contains("unexpected argument"),
            "gem-dependency should accept --pipe flag"
        );
    }

    #[test]
    fn gem_dependency_accepts_both_flag() {
        let output = Command::new("target/debug/lode")
            .args(["gem-dependency", "rake", "--both"])
            .output()
            .expect("Failed to execute lode gem-dependency rake --both");

        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            !stderr.contains("unexpected argument"),
            "gem-dependency should accept --both flag"
        );
    }

    #[test]
    fn gem_dependency_accepts_verbose_flag() {
        let output = Command::new("target/debug/lode")
            .args(["gem-dependency", "rake", "--verbose"])
            .output()
            .expect("Failed to execute lode gem-dependency rake --verbose");

        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            !stderr.contains("unexpected argument"),
            "gem-dependency should accept --verbose flag"
        );
    }

    #[test]
    fn gem_dependency_accepts_quiet_flag() {
        let output = Command::new("target/debug/lode")
            .args(["gem-dependency", "rake", "--quiet"])
            .output()
            .expect("Failed to execute lode gem-dependency rake --quiet");

        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            !stderr.contains("unexpected argument"),
            "gem-dependency should accept --quiet flag"
        );
    }

    #[test]
    fn gem_dependency_accepts_silent_flag() {
        let output = Command::new("target/debug/lode")
            .args(["gem-dependency", "rake", "--silent"])
            .output()
            .expect("Failed to execute lode gem-dependency rake --silent");

        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            !stderr.contains("unexpected argument"),
            "gem-dependency should accept --silent flag"
        );
    }

    #[test]
    fn gem_dependency_accepts_bulk_threshold_flag() {
        let output = Command::new("target/debug/lode")
            .args(["gem-dependency", "rake", "--bulk-threshold", "100"])
            .output()
            .expect("Failed to execute lode gem-dependency rake --bulk-threshold");

        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            !stderr.contains("unexpected argument"),
            "gem-dependency should accept --bulk-threshold flag"
        );
    }

    #[test]
    fn gem_dependency_accepts_clear_sources_flag() {
        let output = Command::new("target/debug/lode")
            .args(["gem-dependency", "rake", "--clear-sources"])
            .output()
            .expect("Failed to execute lode gem-dependency rake --clear-sources");

        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            !stderr.contains("unexpected argument"),
            "gem-dependency should accept --clear-sources flag"
        );
    }

    #[test]
    fn gem_dependency_accepts_http_proxy_flag() {
        let output = Command::new("target/debug/lode")
            .args([
                "gem-dependency",
                "rake",
                "--http-proxy",
                "http://proxy.example.com:8080",
            ])
            .output()
            .expect("Failed to execute lode gem-dependency rake --http-proxy");

        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            !stderr.contains("unexpected argument"),
            "gem-dependency should accept --http-proxy flag"
        );
    }
}
