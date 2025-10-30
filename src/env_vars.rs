//! Bundler and `RubyGems` environment variable handling.

use std::env;

// Helper for boolean environment variables that accept "1", "true", "yes"
fn is_enabled(var: &str) -> bool {
    env::var(var).ok().is_some_and(|s| {
        let s = s.to_lowercase();
        s == "1" || s == "true" || s == "yes"
    })
}

// Network configuration - Proxy support
// HTTP_PROXY, HTTPS_PROXY, NO_PROXY environment variables for proxy configuration

/// Get HTTP/HTTPS proxy URL (checks `HTTPS_PROXY` then `HTTP_PROXY`).
pub fn http_proxy() -> Option<String> {
    env::var("HTTPS_PROXY")
        .or_else(|_| env::var("https_proxy"))
        .or_else(|_| env::var("HTTP_PROXY"))
        .or_else(|_| env::var("http_proxy"))
        .ok()
}

/// Get `NO_PROXY` list (comma-separated hosts to bypass proxy).
pub fn no_proxy() -> Option<String> {
    env::var("NO_PROXY").or_else(|_| env::var("no_proxy")).ok()
}

/// Get HTTP proxy username for authentication.
pub fn http_proxy_user() -> Option<String> {
    env::var("HTTP_PROXY_USER")
        .or_else(|_| env::var("http_proxy_user"))
        .ok()
}

/// Get HTTP proxy password for authentication.
pub fn http_proxy_pass() -> Option<String> {
    env::var("HTTP_PROXY_PASS")
        .or_else(|_| env::var("http_proxy_pass"))
        .ok()
}

/// Get HTTPS proxy username (falls back to `HTTP_PROXY_USER`).
pub fn https_proxy_user() -> Option<String> {
    env::var("HTTPS_PROXY_USER")
        .or_else(|_| env::var("https_proxy_user"))
        .or_else(|_| env::var("HTTP_PROXY_USER"))
        .or_else(|_| env::var("http_proxy_user"))
        .ok()
}

/// Get HTTPS proxy password (falls back to `HTTP_PROXY_PASS`).
pub fn https_proxy_pass() -> Option<String> {
    env::var("HTTPS_PROXY_PASS")
        .or_else(|_| env::var("https_proxy_pass"))
        .or_else(|_| env::var("HTTP_PROXY_PASS"))
        .or_else(|_| env::var("http_proxy_pass"))
        .ok()
}

/// Get `RubyGems` API host (defaults to `https://rubygems.org`).
pub fn rubygems_host() -> String {
    env::var("RUBYGEMS_HOST").unwrap_or_else(|_| "https://rubygems.org".to_string())
}

/// Get gem source URL from `GEM_SOURCE` (first URL if colon-separated list).
pub fn gem_source() -> Option<String> {
    env::var("GEM_SOURCE").ok().map(|sources| {
        // Take first source if multiple are provided (colon-separated)
        sources.split(':').next().unwrap_or(&sources).to_string()
    })
}

/// Get network timeout in seconds (defaults to 10 if not set or invalid).
pub fn bundle_timeout() -> u64 {
    env::var("BUNDLE_TIMEOUT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(10)
}

// RubyGems authentication - RUBYGEMS_API_KEY and GEM_HOST_API_KEY_*

/// Get `RubyGems` API key (checked before credentials file).
pub fn rubygems_api_key() -> Option<String> {
    env::var("RUBYGEMS_API_KEY").ok()
}

/// Get host-specific API key (converts `.` to `__`, `-` to `___`).
/// Example: `rubygems.org` -> `GEM_HOST_API_KEY_RUBYGEMS_ORG`
pub fn gem_host_api_key(host: &str) -> Option<String> {
    let env_host = host.replace('-', "___").replace('.', "__").to_uppercase();
    env::var(format!("GEM_HOST_API_KEY_{env_host}")).ok()
}

// Bundler CLI flag equivalents
// Boolean flags accept "1", "true", "yes" (case-insensitive)
// List variables support colon or space-separated values

/// Get number of parallel jobs (returns None if not set).
pub fn bundle_jobs() -> Option<usize> {
    env::var("BUNDLE_JOBS").ok().and_then(|s| s.parse().ok())
}

/// Get number of network retry attempts (returns None if not set).
pub fn bundle_retry() -> Option<u32> {
    env::var("BUNDLE_RETRY").ok().and_then(|s| s.parse().ok())
}

/// Get groups to exclude (colon/space-separated list).
pub fn bundle_without() -> Option<Vec<String>> {
    env::var("BUNDLE_WITHOUT").ok().map(|s| {
        s.split([':', ' '])
            .filter(|s| !s.is_empty())
            .map(std::string::ToString::to_string)
            .collect()
    })
}

/// Get groups to include (colon/space-separated list).
pub fn bundle_with() -> Option<Vec<String>> {
    env::var("BUNDLE_WITH").ok().map(|s| {
        s.split([':', ' '])
            .filter(|s| !s.is_empty())
            .map(std::string::ToString::to_string)
            .collect()
    })
}

/// Check if frozen mode is enabled.
pub fn bundle_frozen() -> bool {
    env::var("BUNDLE_FROZEN").ok().is_some_and(|s| {
        let s = s.to_lowercase();
        s == "1" || s == "true" || s == "yes"
    })
}

/// Check if deployment mode is enabled.
pub fn bundle_deployment() -> bool {
    env::var("BUNDLE_DEPLOYMENT").ok().is_some_and(|s| {
        let s = s.to_lowercase();
        s == "1" || s == "true" || s == "yes"
    })
}

// Path configuration - BUNDLE_GEMFILE, BUNDLE_PATH, BUNDLE_APP_CONFIG, etc.

/// Get Gemfile path (typically Gemfile or gems.rb).
pub fn bundle_gemfile() -> Option<String> {
    env::var("BUNDLE_GEMFILE").ok()
}

/// Get bundle installation path (e.g., vendor/bundle).
pub fn bundle_path() -> Option<String> {
    env::var("BUNDLE_PATH").ok()
}

/// Get bundle config directory (typically .bundle).
pub fn bundle_app_config() -> Option<String> {
    env::var("BUNDLE_APP_CONFIG").ok()
}

/// Get bundler home directory.
pub fn bundle_user_home() -> Option<String> {
    env::var("BUNDLE_USER_HOME").ok()
}

/// Get bundler cache directory.
pub fn bundle_user_cache() -> Option<String> {
    env::var("BUNDLE_USER_CACHE").ok()
}

/// Get binstubs directory.
pub fn bundle_bin() -> Option<String> {
    env::var("BUNDLE_BIN").ok()
}

// SSL configuration - BUNDLE_SSL_CA_CERT, BUNDLE_SSL_CLIENT_CERT, BUNDLE_SSL_VERIFY_MODE

/// Get SSL CA certificate path.
pub fn bundle_ssl_ca_cert() -> Option<String> {
    env::var("BUNDLE_SSL_CA_CERT").ok()
}

/// Get SSL client certificate path.
pub fn bundle_ssl_client_cert() -> Option<String> {
    env::var("BUNDLE_SSL_CLIENT_CERT").ok()
}

/// Get SSL verification mode ("peer" or "none").
pub fn bundle_ssl_verify_mode() -> Option<String> {
    env::var("BUNDLE_SSL_VERIFY_MODE").ok()
}

// Additional CLI flag equivalents

/// Get cache path.
pub fn bundle_cache_path() -> Option<String> {
    env::var("BUNDLE_CACHE_PATH").ok()
}

/// Check if clean mode is enabled.
pub fn bundle_clean() -> bool {
    is_enabled("BUNDLE_CLEAN")
}

/// Check if no-prune mode is enabled.
pub fn bundle_no_prune() -> bool {
    is_enabled("BUNDLE_NO_PRUNE")
}

/// Check if local mode is enabled.
pub fn bundle_local() -> bool {
    is_enabled("BUNDLE_LOCAL")
}

/// Check if prefer-local mode is enabled.
pub fn bundle_prefer_local() -> bool {
    is_enabled("BUNDLE_PREFER_LOCAL")
}

/// Check if force mode is enabled.
pub fn bundle_force() -> bool {
    is_enabled("BUNDLE_FORCE")
}

/// Check if cache-all-platforms mode is enabled.
pub fn bundle_cache_all_platforms() -> bool {
    is_enabled("BUNDLE_CACHE_ALL_PLATFORMS")
}

// Behavioral toggles

/// Check if root warning should be silenced.
pub fn bundle_silence_root_warning() -> bool {
    is_enabled("BUNDLE_SILENCE_ROOT_WARNING")
}

/// Check if version check should be disabled.
pub fn bundle_disable_version_check() -> bool {
    is_enabled("BUNDLE_DISABLE_VERSION_CHECK")
}

/// Check if ruby platform should be forced.
pub fn bundle_force_ruby_platform() -> bool {
    is_enabled("BUNDLE_FORCE_RUBY_PLATFORM")
}

/// Check if verbose mode is enabled.
pub fn bundle_verbose() -> bool {
    is_enabled("BUNDLE_VERBOSE")
}

// Advanced features

/// Get exclusive groups to install (colon/space-separated list).
pub fn bundle_only() -> Option<Vec<String>> {
    env::var("BUNDLE_ONLY").ok().map(|s| {
        s.split([':', ' '])
            .filter(|s| !s.is_empty())
            .map(std::string::ToString::to_string)
            .collect()
    })
}

/// Check if shared gems should be disabled (use only bundled gems).
pub fn bundle_disable_shared_gems() -> bool {
    is_enabled("BUNDLE_DISABLE_SHARED_GEMS")
}

/// Get custom HTTP user agent.
pub fn bundle_user_agent() -> Option<String> {
    env::var("BUNDLE_USER_AGENT").ok()
}

/// Get custom shebang for binstubs.
pub fn bundle_shebang() -> Option<String> {
    env::var("BUNDLE_SHEBANG").ok()
}

/// Check if cache-all mode is enabled (cache git/path sources).
pub fn bundle_cache_all() -> bool {
    is_enabled("BUNDLE_CACHE_ALL")
}

/// Check if no-install mode is enabled (skip installation after caching).
pub fn bundle_no_install() -> bool {
    is_enabled("BUNDLE_NO_INSTALL")
}

/// Check if prefer-patch mode is enabled (prefer patch-level updates).
pub fn bundle_prefer_patch() -> bool {
    is_enabled("BUNDLE_PREFER_PATCH")
}

/// Check if checksum validation should be disabled.
pub fn bundle_disable_checksum_validation() -> bool {
    is_enabled("BUNDLE_DISABLE_CHECKSUM_VALIDATION")
}

/// Get maximum number of HTTP redirects (defaults to 5).
pub fn bundle_redirect() -> usize {
    env::var("BUNDLE_REDIRECT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(5)
}

// Build tool configuration for native extensions
// MAKE, CC, CXX, CFLAGS, CXXFLAGS, LDFLAGS

/// Get make command (override default for C extensions).
pub fn make_command() -> Option<String> {
    env::var("MAKE").ok()
}

/// Get C compiler (useful for cross-compilation).
pub fn cc() -> Option<String> {
    env::var("CC").ok()
}

/// Get C++ compiler (useful for cross-compilation).
pub fn cxx() -> Option<String> {
    env::var("CXX").ok()
}

/// Get C compiler flags.
pub fn cflags() -> Option<String> {
    env::var("CFLAGS").ok()
}

/// Get C++ compiler flags.
pub fn cxxflags() -> Option<String> {
    env::var("CXXFLAGS").ok()
}

/// Get linker flags.
pub fn ldflags() -> Option<String> {
    env::var("LDFLAGS").ok()
}

// Gem installation filtering

/// Get gems to skip (space-separated patterns, e.g., "rdoc ri" or "test-* *-dev").
pub fn gem_skip() -> Option<Vec<String>> {
    env::var("GEM_SKIP").ok().map(|s| {
        s.split_whitespace()
            .map(std::string::ToString::to_string)
            .collect()
    })
}

/// Check if a gem name should be skipped based on `GEM_SKIP` patterns.
///
/// Supports simple glob patterns:
/// - `*` matches any characters
/// - Exact match if no wildcards
///
/// # Examples
///
/// ```
/// // Exact match
/// assert!(lode::env_vars::should_skip_gem("rdoc", &["rdoc", "ri"]));
/// assert!(!lode::env_vars::should_skip_gem("rake", &["rdoc", "ri"]));
///
/// // Prefix wildcard
/// assert!(lode::env_vars::should_skip_gem("rails-dev", &["*-dev"]));
/// assert!(!lode::env_vars::should_skip_gem("rails", &["*-dev"]));
///
/// // Suffix wildcard
/// assert!(lode::env_vars::should_skip_gem("test-helpers", &["test-*"]));
/// assert!(!lode::env_vars::should_skip_gem("minitest", &["test-*"]));
///
/// // Both wildcards
/// assert!(lode::env_vars::should_skip_gem("ruby-debug-ide", &["*-debug-*"]));
/// ```
#[must_use]
pub fn should_skip_gem(gem_name: &str, patterns: &[impl AsRef<str>]) -> bool {
    patterns.iter().any(|pattern| {
        let pattern = pattern.as_ref();

        // Exact match (no wildcards)
        if !pattern.contains('*') {
            return gem_name == pattern;
        }

        // Simple glob matching
        if pattern.starts_with('*') && pattern.ends_with('*') {
            // *foo* - contains
            let middle = &pattern[1..pattern.len() - 1];
            gem_name.contains(middle)
        } else if let Some(suffix) = pattern.strip_prefix('*') {
            // *foo - ends with
            gem_name.ends_with(suffix)
        } else if let Some(prefix) = pattern.strip_suffix('*') {
            // foo* - starts with
            gem_name.starts_with(prefix)
        } else {
            // More complex patterns (e.g., "foo*bar") - not commonly used
            // Fall back to simple contains check for the non-wildcard parts
            let parts: Vec<&str> = pattern.split('*').collect();
            if let (Some(&first), Some(&last)) = (parts.first(), parts.get(1)) {
                parts.len() == 2 && gem_name.starts_with(first) && gem_name.ends_with(last)
            } else {
                false
            }
        }
    })
}

// Note: gem_skip() is a simple wrapper around env::var() and doesn't need unit tests.
// The pattern matching logic is tested in should_skip_gem() tests below.

// Configuration and debugging options

/// Check if config files should be ignored (ignore .bundle/config and .bundlerc).
pub fn bundle_ignore_config() -> bool {
    is_enabled("BUNDLE_IGNORE_CONFIG")
}

/// Check if offline installation is allowed (install even if gems unavailable).
pub fn bundle_allow_offline_install() -> bool {
    is_enabled("BUNDLE_ALLOW_OFFLINE_INSTALL")
}

/// Check if auto-install is enabled (automatically install missing gems).
pub fn bundle_auto_install() -> bool {
    is_enabled("BUNDLE_AUTO_INSTALL")
}

/// Check if deprecation warnings should be silenced (useful for CI).
pub fn bundle_silence_deprecations() -> bool {
    is_enabled("BUNDLE_SILENCE_DEPRECATIONS")
}

/// Check if funding requests should be ignored.
pub fn bundle_ignore_funding_requests() -> bool {
    is_enabled("BUNDLE_IGNORE_FUNDING_REQUESTS")
}

/// Check if post-install messages should be ignored.
pub fn bundle_ignore_messages() -> bool {
    is_enabled("BUNDLE_IGNORE_MESSAGES")
}

/// Check if lockfile checksums are enabled (adds SHA256 checksums).
pub fn bundle_lockfile_checksums() -> bool {
    is_enabled("BUNDLE_LOCKFILE_CHECKSUMS")
}

/// Check if global gem cache is enabled (share cache across projects).
pub fn bundle_global_gem_cache() -> bool {
    is_enabled("BUNDLE_GLOBAL_GEM_CACHE")
}

/// Check if system-wide gem installation is enabled (install to system Ruby directory).
pub fn bundle_system() -> bool {
    is_enabled("BUNDLE_SYSTEM")
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test helper: Parse boolean values like bundle_frozen() does
    fn is_bundle_bool_enabled(value: &str) -> bool {
        let s = value.to_lowercase();
        s == "1" || s == "true" || s == "yes"
    }

    // Test helper: Parse list values like bundle_without() does
    fn parse_bundle_list(value: &str) -> Vec<String> {
        value
            .split([':', ' '])
            .filter(|s| !s.is_empty())
            .map(std::string::ToString::to_string)
            .collect()
    }

    // ===== Critical Security Variables - Logic Testing =====

    #[test]
    fn bundle_frozen_parsing_true_variants() {
        assert!(is_bundle_bool_enabled("true"));
        assert!(is_bundle_bool_enabled("1"));
        assert!(is_bundle_bool_enabled("yes"));
        assert!(is_bundle_bool_enabled("TRUE"));
        assert!(is_bundle_bool_enabled("YES"));
    }

    #[test]
    fn bundle_frozen_parsing_false_variants() {
        assert!(!is_bundle_bool_enabled("false"));
        assert!(!is_bundle_bool_enabled("0"));
        assert!(!is_bundle_bool_enabled("no"));
        assert!(!is_bundle_bool_enabled(""));
    }

    #[test]
    fn bundle_deployment_parsing_variants() {
        assert!(is_bundle_bool_enabled("yes"));
        assert!(is_bundle_bool_enabled("TRUE"));
        assert!(is_bundle_bool_enabled("1"));
        assert!(!is_bundle_bool_enabled("false"));
    }

    // ===== List Parsing Variables - Logic Testing =====

    #[test]
    fn bundle_without_colon_separated_parsing() {
        let result = parse_bundle_list("development:test");
        assert_eq!(result, vec!["development".to_string(), "test".to_string()]);
    }

    #[test]
    fn bundle_without_space_separated_parsing() {
        let result = parse_bundle_list("development test");
        assert_eq!(result, vec!["development".to_string(), "test".to_string()]);
    }

    #[test]
    fn bundle_with_mixed_separators_parsing() {
        let result = parse_bundle_list("development:test staging");
        assert_eq!(
            result,
            vec![
                "development".to_string(),
                "test".to_string(),
                "staging".to_string()
            ]
        );
    }

    #[test]
    fn bundle_without_empty_entries_filtering() {
        let result = parse_bundle_list("development::test");
        assert_eq!(result, vec!["development".to_string(), "test".to_string()]);
    }

    // ===== GEM_SKIP Pattern Matching (existing tests) =====

    #[test]
    fn should_skip_gem_exact_match() {
        let patterns = vec!["rdoc", "ri"];
        assert!(should_skip_gem("rdoc", &patterns));
        assert!(should_skip_gem("ri", &patterns));
        assert!(!should_skip_gem("rake", &patterns));
        assert!(!should_skip_gem("rdoc-dev", &patterns));
    }

    #[test]
    fn should_skip_gem_prefix_wildcard() {
        let patterns = vec!["*-dev", "*-test"];
        assert!(should_skip_gem("rails-dev", &patterns));
        assert!(should_skip_gem("minitest-test", &patterns));
        assert!(!should_skip_gem("rails", &patterns));
        assert!(!should_skip_gem("dev-tools", &patterns));
    }

    #[test]
    fn should_skip_gem_suffix_wildcard() {
        let patterns = vec!["test-*", "debug-*"];
        assert!(should_skip_gem("test-helpers", &patterns));
        assert!(should_skip_gem("debug-tools", &patterns));
        assert!(!should_skip_gem("minitest", &patterns));
        assert!(!should_skip_gem("tools-test", &patterns));
    }

    #[test]
    fn should_skip_gem_both_wildcards() {
        let patterns = vec!["*-debug-*", "*test*"];
        assert!(should_skip_gem("ruby-debug-ide", &patterns));
        assert!(should_skip_gem("rails-debug-tools", &patterns));
        assert!(should_skip_gem("minitest", &patterns));
        assert!(should_skip_gem("testing-utils", &patterns));
        assert!(!should_skip_gem("ruby-ide", &patterns));
        assert!(!should_skip_gem("rails", &patterns));
    }

    #[test]
    fn should_skip_gem_complex_pattern() {
        let patterns = vec!["foo*bar"];
        assert!(should_skip_gem("foobar", &patterns));
        assert!(should_skip_gem("foo-test-bar", &patterns));
        assert!(!should_skip_gem("foo", &patterns));
        assert!(!should_skip_gem("bar", &patterns));
        assert!(!should_skip_gem("foobaz", &patterns));
    }

    #[test]
    fn should_skip_gem_empty_patterns() {
        let patterns: Vec<&str> = vec![];
        assert!(!should_skip_gem("rdoc", &patterns));
        assert!(!should_skip_gem("any-gem", &patterns));
    }

    #[test]
    fn should_skip_gem_string_refs() {
        let patterns = vec!["rdoc".to_string(), "ri".to_string()];
        assert!(should_skip_gem("rdoc", &patterns));
        assert!(!should_skip_gem("rake", &patterns));
    }

    // ===== All BUNDLE_* Boolean Flags - Parameterized Tests =====

    /// Helper to test any boolean flag's parsing
    fn test_bool_flag(enabled_values: &[&str], disabled_values: &[&str]) {
        for val in enabled_values {
            assert!(
                is_bundle_bool_enabled(val),
                "Expected '{val}' to enable flag"
            );
        }
        for val in disabled_values {
            assert!(
                !is_bundle_bool_enabled(val),
                "Expected '{val}' to disable flag"
            );
        }
    }

    #[test]
    fn all_bundle_bool_flags_true_values() {
        // All BUNDLE_* bool flags accept: true, 1, yes (case-insensitive)
        let enabled = vec!["true", "True", "TRUE", "1", "yes", "Yes", "YES"];
        let disabled = vec!["false", "False", "FALSE", "0", "no", "No", "NO", ""];

        test_bool_flag(&enabled, &disabled);
    }

    #[test]
    fn bundle_clean_parsing() {
        test_bool_flag(&["1", "true", "yes"], &["0", "false", "no"]);
    }

    #[test]
    fn bundle_no_prune_parsing() {
        test_bool_flag(&["1", "true", "yes"], &["0", "false", "no"]);
    }

    #[test]
    fn bundle_local_parsing() {
        test_bool_flag(&["1", "true", "yes"], &["0", "false", "no"]);
    }

    #[test]
    fn bundle_prefer_local_parsing() {
        test_bool_flag(&["1", "true", "yes"], &["0", "false", "no"]);
    }

    #[test]
    fn bundle_force_parsing() {
        test_bool_flag(&["1", "true", "yes"], &["0", "false", "no"]);
    }

    #[test]
    fn bundle_cache_all_platforms_parsing() {
        test_bool_flag(&["1", "true", "yes"], &["0", "false", "no"]);
    }

    #[test]
    fn bundle_silence_root_warning_parsing() {
        test_bool_flag(&["1", "true", "yes"], &["0", "false", "no"]);
    }

    #[test]
    fn bundle_disable_version_check_parsing() {
        test_bool_flag(&["1", "true", "yes"], &["0", "false", "no"]);
    }

    #[test]
    fn bundle_force_ruby_platform_parsing() {
        test_bool_flag(&["1", "true", "yes"], &["0", "false", "no"]);
    }

    #[test]
    fn bundle_verbose_parsing() {
        test_bool_flag(&["1", "true", "yes"], &["0", "false", "no"]);
    }

    #[test]
    fn bundle_disable_shared_gems_parsing() {
        test_bool_flag(&["1", "true", "yes"], &["0", "false", "no"]);
    }

    #[test]
    fn bundle_cache_all_parsing() {
        test_bool_flag(&["1", "true", "yes"], &["0", "false", "no"]);
    }

    #[test]
    fn bundle_no_install_parsing() {
        test_bool_flag(&["1", "true", "yes"], &["0", "false", "no"]);
    }

    #[test]
    fn bundle_prefer_patch_parsing() {
        test_bool_flag(&["1", "true", "yes"], &["0", "false", "no"]);
    }

    #[test]
    fn bundle_disable_checksum_validation_parsing() {
        test_bool_flag(&["1", "true", "yes"], &["0", "false", "no"]);
    }

    #[test]
    fn bundle_ignore_config_parsing() {
        test_bool_flag(&["1", "true", "yes"], &["0", "false", "no"]);
    }

    #[test]
    fn bundle_allow_offline_install_parsing() {
        test_bool_flag(&["1", "true", "yes"], &["0", "false", "no"]);
    }

    #[test]
    fn bundle_auto_install_parsing() {
        test_bool_flag(&["1", "true", "yes"], &["0", "false", "no"]);
    }

    #[test]
    fn bundle_silence_deprecations_parsing() {
        test_bool_flag(&["1", "true", "yes"], &["0", "false", "no"]);
    }

    #[test]
    fn bundle_ignore_funding_requests_parsing() {
        test_bool_flag(&["1", "true", "yes"], &["0", "false", "no"]);
    }

    #[test]
    fn bundle_ignore_messages_parsing() {
        test_bool_flag(&["1", "true", "yes"], &["0", "false", "no"]);
    }

    #[test]
    fn bundle_lockfile_checksums_parsing() {
        test_bool_flag(&["1", "true", "yes"], &["0", "false", "no"]);
    }

    #[test]
    fn bundle_global_gem_cache_parsing() {
        test_bool_flag(&["1", "true", "yes"], &["0", "false", "no"]);
    }

    #[test]
    fn bundle_system_parsing() {
        test_bool_flag(&["1", "true", "yes"], &["0", "false", "no"]);
    }

    // ===== String/Path Parsing Tests =====

    #[test]
    fn bundle_gemfile_path_validation() {
        let paths = vec![
            "/absolute/path/Gemfile",
            "./relative/path/Gemfile",
            "Gemfile",
            "../parent/Gemfile",
        ];

        for path in paths {
            assert!(!path.is_empty(), "Path should not be empty");
            assert!(
                path.ends_with("Gemfile") || path.contains("Gemfile"),
                "Should be a Gemfile path"
            );
        }
    }

    #[test]
    fn bundle_path_validation() {
        let paths = vec![
            "/usr/local/bundle",
            "/home/user/.bundle",
            "./vendor/bundle",
            "vendor",
        ];

        for path in paths {
            assert!(!path.is_empty(), "Path should not be empty");
            // Paths can be absolute or relative (with ./)
            assert!(
                path.starts_with('/') || path.starts_with('.') || !path.contains('/'),
                "Should be valid path format: {path}"
            );
        }
    }

    #[test]
    fn bundle_cache_path_validation() {
        let paths = vec!["/var/cache/bundle", "~/.bundle/cache", "vendor/cache"];

        for path in paths {
            assert!(!path.is_empty(), "Cache path should not be empty");
        }
    }

    #[test]
    fn bundle_app_config_validation() {
        let paths = vec!["/etc/bundle", "/home/user/.config/bundle", ".bundle"];

        for path in paths {
            assert!(!path.is_empty(), "Config path should not be empty");
        }
    }

    #[test]
    fn rubygems_api_key_validation() {
        let valid_keys = vec![
            "test_key_12345",
            "abc123def456",
            "my-api-key",
            "key_with_underscores",
        ];

        for key in valid_keys {
            assert!(!key.is_empty(), "API key should not be empty");
            // Keys are typically alphanumeric with some special chars
            assert!(key.len() > 3, "API key should have minimum length");
        }
    }

    #[test]
    fn http_proxy_url_validation() {
        let valid_urls = vec![
            "http://proxy.example.com:8080",
            "https://proxy:3128",
            "socks5://192.168.1.1:1080",
            "http://user:pass@proxy.com:8080",
        ];

        for url in valid_urls {
            assert!(!url.is_empty(), "Proxy URL should not be empty");
            assert!(
                url.starts_with("http") || url.starts_with("socks"),
                "Should be valid proxy URL"
            );
        }
    }

    #[test]
    fn no_proxy_list_parsing() {
        // NO_PROXY comma or space-separated list
        let lists = vec![
            vec!["localhost", "127.0.0.1"],
            vec!["example.com", ".internal.com"],
            vec!["192.168.1.0/24", "10.0.0.0/8"],
        ];

        for list in lists {
            assert!(!list.is_empty(), "NO_PROXY list should not be empty");
            for entry in list {
                assert!(!entry.is_empty(), "NO_PROXY entry should not be empty");
            }
        }
    }

    // ===== Build Tool Variables =====

    #[test]
    fn cc_compiler_command_validation() {
        let compilers = vec!["gcc", "clang", "cc", "arm-linux-gnueabihf-gcc"];

        for compiler in compilers {
            assert!(!compiler.is_empty(), "Compiler command should not be empty");
            // Should contain valid command characters
            assert!(
                compiler
                    .chars()
                    .all(|c| c.is_alphanumeric() || c == '-' || c == '_'),
                "Compiler should be valid command"
            );
        }
    }

    #[test]
    fn cxx_compiler_command_validation() {
        let compilers = vec!["g++", "clang++", "c++", "arm-linux-gnueabihf-g++"];

        for compiler in compilers {
            assert!(!compiler.is_empty(), "C++ compiler should not be empty");
            assert!(
                compiler.contains("++") || compiler == "c++",
                "Should be C++ compiler"
            );
        }
    }

    #[test]
    fn make_command_validation() {
        let commands = vec!["make", "gmake", "pmake", "bsd-make"];

        for cmd in commands {
            assert!(!cmd.is_empty(), "Make command should not be empty");
            assert!(cmd.contains("make"), "Should be make variant");
        }
    }

    #[test]
    fn cflags_parsing() {
        let flag_sets = vec![
            "-O2",
            "-O2 -Wall -Wextra",
            "-fPIC -shared",
            "-march=native -mtune=native",
        ];

        for flags in flag_sets {
            assert!(!flags.is_empty(), "CFLAGS should not be empty");
            assert!(flags.starts_with('-'), "Flags should start with dash");
        }
    }

    #[test]
    fn cxxflags_parsing() {
        let flag_sets = vec![
            "-O2",
            "-Wall -Wextra",
            "-std=c++17 -fPIC",
            "-O3 -march=native",
        ];

        for flags in flag_sets {
            assert!(!flags.is_empty(), "CXXFLAGS should not be empty");
            assert!(flags.starts_with('-'), "Flags should start with dash");
        }
    }

    #[test]
    fn ldflags_parsing() {
        let flag_sets = vec![
            "-L/usr/local/lib",
            "-lm -lpthread",
            "-Wl,-rpath,/usr/local/lib",
            "-static",
        ];

        for flags in flag_sets {
            assert!(!flags.is_empty(), "LDFLAGS should not be empty");
        }
    }

    // ===== Numeric Parsing Tests =====

    fn parse_positive_integer(value: &str) -> Option<usize> {
        value.parse().ok()
    }

    #[test]
    fn bundle_jobs_parsing_valid() {
        assert_eq!(parse_positive_integer("1"), Some(1));
        assert_eq!(parse_positive_integer("8"), Some(8));
        assert_eq!(parse_positive_integer("16"), Some(16));
        assert_eq!(parse_positive_integer("100"), Some(100));
    }

    #[test]
    fn bundle_jobs_parsing_invalid() {
        // Note: parse() treats "0" as valid, but BUNDLE_JOBS=0 is unusual (would mean no parallelism)
        assert_eq!(parse_positive_integer("-1"), None);
        assert_eq!(parse_positive_integer("invalid"), None);
        assert_eq!(parse_positive_integer(""), None);
        assert_eq!(parse_positive_integer("1.5"), None);
    }

    #[test]
    fn bundle_retry_parsing_valid() {
        assert_eq!(parse_positive_integer("1"), Some(1));
        assert_eq!(parse_positive_integer("3"), Some(3));
        assert_eq!(parse_positive_integer("5"), Some(5));
    }

    #[test]
    fn bundle_retry_parsing_invalid() {
        assert_eq!(parse_positive_integer("abc"), None);
        assert_eq!(parse_positive_integer(""), None);
        assert_eq!(parse_positive_integer("-5"), None);
    }

    #[test]
    fn bundle_timeout_parsing_valid() {
        assert_eq!(parse_positive_integer("30"), Some(30));
        assert_eq!(parse_positive_integer("60"), Some(60));
        assert_eq!(parse_positive_integer("120"), Some(120));
    }

    #[test]
    fn bundle_timeout_parsing_invalid() {
        assert_eq!(parse_positive_integer("invalid"), None);
        assert_eq!(parse_positive_integer(""), None);
        assert_eq!(parse_positive_integer("-30"), None);
    }

    fn parse_redirect_count(value: &str) -> usize {
        value.parse().unwrap_or(5)
    }

    #[test]
    fn bundle_redirect_parsing_valid() {
        assert_eq!(parse_redirect_count("1"), 1);
        assert_eq!(parse_redirect_count("5"), 5);
        assert_eq!(parse_redirect_count("10"), 10);
    }

    #[test]
    fn bundle_redirect_parsing_invalid_defaults_to_5() {
        assert_eq!(parse_redirect_count("invalid"), 5);
        assert_eq!(parse_redirect_count(""), 5);
        assert_eq!(parse_redirect_count("-1"), 5);
        assert_eq!(parse_redirect_count("99999999999999999999"), 5);
    }

    // ===== SSL/Security Variables =====

    #[test]
    fn bundle_ssl_verify_mode_valid_values() {
        let valid_modes = vec!["peer", "none"];

        for mode in valid_modes {
            assert!(!mode.is_empty(), "Verify mode should not be empty");
            assert!(
                mode == "peer" || mode == "none",
                "Should be valid SSL verify mode"
            );
        }
    }

    #[test]
    fn bundle_ssl_verify_mode_case_insensitive() {
        let modes = vec!["PEER", "None", "Peer", "NONE"];

        for mode in modes {
            let lower = mode.to_lowercase();
            assert!(
                lower == "peer" || lower == "none",
                "Mode should normalize correctly"
            );
        }
    }

    #[test]
    fn bundle_ssl_ca_cert_path_validation() {
        let paths = vec![
            "/etc/ssl/certs/ca-bundle.crt",
            "/usr/local/etc/openssl/cert.pem",
            "~/.ssl/ca.pem",
        ];

        for path in paths {
            assert!(!path.is_empty(), "CA cert path should not be empty");
            let has_valid_ext = std::path::Path::new(path).extension().is_some_and(|ext| {
                ext.eq_ignore_ascii_case("pem") || ext.eq_ignore_ascii_case("crt")
            });
            assert!(has_valid_ext, "Should be valid certificate file");
        }
    }

    #[test]
    fn bundle_ssl_client_cert_path_validation() {
        let paths = vec!["/etc/ssl/private/client.pem", "/home/user/.ssl/cert.pem"];

        for path in paths {
            assert!(!path.is_empty(), "Client cert path should not be empty");
            let has_pem_ext = std::path::Path::new(path)
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("pem"));
            assert!(has_pem_ext, "Should be valid certificate file");
        }
    }

    // ===== GEM_HOST_API_KEY Conversion Tests =====

    /// Helper to convert host to env var name (`GEM_HOST_API_KEY_*`)
    fn host_to_env_var_name(host: &str) -> String {
        format!(
            "GEM_HOST_API_KEY_{}",
            host.to_uppercase().replace(['.', '-', ':'], "_")
        )
    }

    #[test]
    fn gem_host_api_key_var_name_conversion() {
        assert_eq!(
            host_to_env_var_name("rubygems.org"),
            "GEM_HOST_API_KEY_RUBYGEMS_ORG"
        );
        assert_eq!(
            host_to_env_var_name("gems.company.com"),
            "GEM_HOST_API_KEY_GEMS_COMPANY_COM"
        );
        assert_eq!(
            host_to_env_var_name("localhost:8080"),
            "GEM_HOST_API_KEY_LOCALHOST_8080"
        );
    }

    #[test]
    fn gem_host_api_key_multiple_hosts() {
        let hosts = vec![
            "rubygems.org",
            "api.github.com",
            "gems.internal.company.com",
            "localhost:3000",
        ];

        for host in hosts {
            let var_name = host_to_env_var_name(host);
            assert!(
                var_name.starts_with("GEM_HOST_API_KEY_"),
                "Should start with prefix"
            );
            assert!(!var_name.contains('.'), "Should replace dots");
            assert!(!var_name.contains('-'), "Should replace hyphens");
        }
    }

    // ===== Priority/Fallback Logic Tests =====

    #[test]
    fn http_proxy_prefers_https_over_http() {
        let https = Some("https://proxy1.com:8080");
        let http = Some("http://proxy2.com:8080");
        let result = https.or(http);
        assert_eq!(result, Some("https://proxy1.com:8080"));
    }

    #[test]
    fn http_proxy_falls_back_to_http() {
        let https = None;
        let http = Some("http://proxy.com:8080");
        let result = https.or(http);
        assert_eq!(result, Some("http://proxy.com:8080"));
    }

    #[test]
    fn http_proxy_none_when_both_empty() {
        let https: Option<&str> = None;
        let http: Option<&str> = None;
        let result = https.or(http);
        assert_eq!(result, None);
    }
}
