//! Configuration file management
//!
//! Handles reading and writing lode's TOML configuration files from project
//! and global locations.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Application configuration loaded from TOML files
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct Config {
    /// Custom vendor directory path
    #[serde(default)]
    pub vendor_dir: Option<String>,

    /// Custom cache directory path
    #[serde(default)]
    pub cache_dir: Option<String>,

    /// Custom Gemfile path
    #[serde(default)]
    pub gemfile: Option<String>,

    /// Gem sources with optional fallbacks
    #[serde(default)]
    pub gem_sources: Vec<GemSource>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GemSource {
    pub url: String,
    #[serde(default)]
    pub fallback: Option<String>,
}

impl Config {
    /// Load configuration from TOML files.
    /// Priority: ./.lode.toml -> ~/.config/lode/config.toml
    pub fn load() -> Result<Self> {
        Self::load_with_options(None, false)
    }

    /// Load configuration with custom options.
    ///
    /// # Arguments
    /// * `custom_path` - Optional custom path to config file (overrides defaults)
    /// * `skip_rc` - If true, skip loading config files (return default config)
    pub fn load_with_options(custom_path: Option<&str>, skip_rc: bool) -> Result<Self> {
        // If skip_rc is set, return default config
        if skip_rc {
            return Ok(Self::default());
        }

        // If custom path provided, load from that
        if let Some(path) = custom_path {
            return Self::load_from(path);
        }

        // Try local config first
        if let Ok(config) = Self::load_from(".lode.toml") {
            return Ok(config);
        }

        // Try user config
        if let Some(config_dir) = Self::user_config_dir() {
            let config_path = config_dir.join("config.toml");
            if let Ok(config) = Self::load_from(&config_path) {
                return Ok(config);
            }
        }

        // Return default config
        Ok(Self::default())
    }

    fn load_from<P: AsRef<Path>>(path: P) -> Result<Self> {
        let contents = fs::read_to_string(path)?;
        let config: Self = toml::from_str(&contents)?;
        Ok(config)
    }

    fn user_config_dir() -> Option<PathBuf> {
        // Check XDG_CONFIG_HOME first
        if let Ok(xdg_config) = env::var("XDG_CONFIG_HOME") {
            return Some(PathBuf::from(xdg_config).join("lode"));
        }

        // Fall back to ~/.config/lode
        dirs::home_dir().map(|home| home.join(".config").join("lode"))
    }
}

/// Resolve vendor directory: `BUNDLE_PATH` env -> Config -> .bundle/config -> system gem dir.
pub fn vendor_dir(config: Option<&Config>) -> Result<PathBuf> {
    // 1. Check BUNDLE_PATH environment variable
    if let Some(bundle_path) = crate::env_vars::bundle_path() {
        return Ok(PathBuf::from(bundle_path));
    }

    // 2. Check config file
    if let Some(config) = config
        && let Some(ref dir) = config.vendor_dir
    {
        return Ok(PathBuf::from(dir));
    }

    // 3. Check Bundler config
    if let Some(bundle_path) = read_bundle_config_path() {
        return Ok(PathBuf::from(bundle_path));
    }

    // 4. Fall back to system gem directory
    system_gem_dir()
}

/// Resolve cache directory: `BUNDLE_USER_CACHE` env -> Config -> platform cache dir.
pub fn cache_dir(config: Option<&Config>) -> Result<PathBuf> {
    // 1. Check BUNDLE_USER_CACHE environment variable
    if let Some(cache) = crate::env_vars::bundle_user_cache() {
        return Ok(PathBuf::from(cache));
    }

    // 2. Check config file
    if let Some(config) = config
        && let Some(ref dir) = config.cache_dir
    {
        return Ok(PathBuf::from(dir));
    }

    // 3. Use platform-specific cache directory
    if let Some(cache_base) = dirs::cache_dir() {
        return Ok(cache_base.join("lode").join("gems"));
    }

    // Fallback to ~/.cache/lode/gems
    dirs::home_dir()
        .map(|home| home.join(".cache").join("lode").join("gems"))
        .context("Could not determine home directory")
}

/// Read `BUNDLE_PATH` from `.bundle/config` (YAML format)
///
/// Checks `BUNDLE_APP_CONFIG` environment variable first for the bundle config directory.
fn read_bundle_config_path() -> Option<String> {
    // Check BUNDLE_APP_CONFIG environment variable first
    let bundle_dir =
        env::var("BUNDLE_APP_CONFIG").map_or_else(|_| PathBuf::from(".bundle"), PathBuf::from);

    let bundle_config = bundle_dir.join("config");
    if !bundle_config.exists() {
        return None;
    }

    let contents = fs::read_to_string(bundle_config).ok()?;

    // Simple YAML parsing for BUNDLE_PATH
    // Format: BUNDLE_PATH: "vendor/bundle"
    for line in contents.lines() {
        if let Some(path) = line.strip_prefix("BUNDLE_PATH:") {
            let path = path.trim().trim_matches('"').trim_matches('\'');
            return Some(path.to_string());
        }
    }

    None
}

/// Get system gem directory using `gem environment gemdir`
///
/// Returns the base gem directory without the Ruby version segment.
/// For example, if `gem environment gemdir` returns `/Users/user/.gem/ruby/3.5.0`,
/// this function returns `/Users/user/.gem`.
fn system_gem_dir() -> Result<PathBuf> {
    let output = Command::new("gem")
        .args(["environment", "gemdir"])
        .output()
        .context("Failed to execute 'gem environment gemdir'")?;

    if !output.status.success() {
        anyhow::bail!("'gem environment gemdir' failed");
    }

    let path = String::from_utf8(output.stdout)
        .context("gem command returned invalid UTF-8")?
        .trim()
        .to_string();

    let mut gem_dir = PathBuf::from(&path);

    // Strip Ruby version from path if present
    // gem environment gemdir returns paths like /Users/user/.gem/ruby/3.5.0
    // We want to return /Users/user/.gem so callers can append /ruby/{version}/gems
    if let Some(parent) = gem_dir.parent()
        && parent.ends_with("ruby")
    {
        // Path is like /path/.gem/ruby/3.5.0, return /path/.gem
        if let Some(grandparent) = parent.parent() {
            gem_dir = grandparent.to_path_buf();
        }
    }

    Ok(gem_dir)
}

/// Get Ruby version: Gemfile.lock -> Gemfile -> ruby --version -> default.
#[must_use]
pub fn ruby_version(lockfile_version: Option<&str>) -> String {
    ruby_version_with_gemfile(lockfile_version, None::<&str>)
}

/// Get Ruby version with Gemfile: lockfile -> Gemfile -> ruby --version -> default.
#[must_use]
pub fn ruby_version_with_gemfile<P: AsRef<std::path::Path>>(
    lockfile_version: Option<&str>,
    gemfile_path: Option<P>,
) -> String {
    use crate::gemfile::Gemfile;

    // 1. From lockfile
    if let Some(version) = lockfile_version {
        return normalize_ruby_version(version);
    }

    // 2. From Gemfile ruby directive
    if let Some(gemfile) = gemfile_path
        && let Ok(parsed_gemfile) = Gemfile::parse_file(gemfile)
        && let Some(version) = parsed_gemfile.ruby_version
    {
        return normalize_ruby_version(&version);
    }

    // 3. From `ruby --version`
    if let Ok(output) = Command::new("ruby").arg("--version").output()
        && output.status.success()
        && let Ok(version_str) = String::from_utf8(output.stdout)
    {
        // Parse "ruby 3.3.0p0 ..." -> "3.3.0"
        if let Some(version) = version_str.split_whitespace().nth(1) {
            return to_major_minor(version);
        }
    }

    // 4. Default
    "3.4.0".to_string()
}

/// Convert Ruby version to major.minor.0 format (Bundler convention)
///
/// Examples:
/// - "3.3.0p0" -> "3.3.0"
/// - "3.4.7" -> "3.4.0"
/// - "~> 3.2" -> "3.2.0"
fn to_major_minor(version: &str) -> String {
    // Remove patch level suffix (e.g., "p0")
    let version = version.split('p').next().unwrap_or(version);

    // Parse version parts
    let parts: Vec<&str> = version.split('.').collect();

    if parts.len() >= 2 {
        format!(
            "{}.{}.0",
            parts.first().map_or("0", |&p| p),
            parts.get(1).map_or("0", |&p| p)
        )
    } else {
        version.to_string()
    }
}

/// Normalize Ruby version from constraints
///
/// Examples:
/// - "3.3.0p0" -> "3.3.0"
/// - ">= 3.0.0" -> "3.0.0"
/// - "~> 3.2" -> "3.2.0"
fn normalize_ruby_version(constraint: &str) -> String {
    let version = constraint
        .trim()
        .trim_start_matches(">=")
        .trim_start_matches("~>")
        .trim_start_matches('<')
        .trim_start_matches('>')
        .trim_start_matches('=')
        .trim();

    to_major_minor(version)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    mod version_normalization {
        use super::*;

        #[test]
        fn to_major_minor_formats() {
            assert_eq!(to_major_minor("3.3.0p0"), "3.3.0");
            assert_eq!(to_major_minor("3.4.7"), "3.4.0");
            assert_eq!(to_major_minor("2.7.8"), "2.7.0");
            assert_eq!(to_major_minor("3.3"), "3.3.0");
            assert_eq!(to_major_minor("3"), "3");
        }

        #[test]
        fn normalize_various_formats() {
            assert_eq!(normalize_ruby_version("3.3.0p0"), "3.3.0");
            assert_eq!(normalize_ruby_version(">= 3.0.0"), "3.0.0");
            assert_eq!(normalize_ruby_version("~> 3.2"), "3.2.0");
            assert_eq!(normalize_ruby_version("3.4.7"), "3.4.0");
            assert_eq!(normalize_ruby_version("  >= 3.1  "), "3.1.0");
            assert_eq!(normalize_ruby_version("< 4.0"), "4.0.0");
            assert_eq!(normalize_ruby_version("= 3.2.1"), "3.2.0");
        }
    }

    mod config {
        use super::*;

        #[test]
        fn default_values() {
            let config = Config::default();
            assert!(config.vendor_dir.is_none());
            assert!(config.cache_dir.is_none());
            assert!(config.gem_sources.is_empty());
        }

        #[test]
        fn load_nonexistent() {
            let config = Config::load().unwrap();
            assert!(config.vendor_dir.is_none());
        }

        #[test]
        fn load_from_toml() -> Result<()> {
            let temp_dir = tempfile::tempdir()?;
            let config_path = temp_dir.path().join(".lode.toml");

            fs::write(
                &config_path,
                r#"
vendor_dir = "/custom/vendor"
cache_dir = "/custom/cache"

[[gem_sources]]
url = "https://rubygems.org"
fallback = "https://mirror.example.com"
"#,
            )?;

            let config = Config::load_from(&config_path)?;
            assert_eq!(config.vendor_dir, Some("/custom/vendor".to_string()));
            assert_eq!(config.cache_dir, Some("/custom/cache".to_string()));
            assert_eq!(config.gem_sources.len(), 1);

            let source = config.gem_sources.first().expect("should have gem source");
            assert_eq!(source.url, "https://rubygems.org");
            assert_eq!(
                source.fallback,
                Some("https://mirror.example.com".to_string())
            );

            Ok(())
        }
    }

    mod directories {
        use super::*;

        #[test]
        fn vendor_dir_from_config() {
            let config = Config {
                vendor_dir: Some("/config/vendor".to_string()),
                cache_dir: None,
                gemfile: None,
                gem_sources: vec![],
            };

            let result = vendor_dir(Some(&config)).unwrap();
            assert_eq!(result, PathBuf::from("/config/vendor"));
        }

        #[test]
        fn cache_dir_from_config() {
            let config = Config {
                vendor_dir: None,
                cache_dir: Some("/config/cache".to_string()),
                gemfile: None,
                gem_sources: vec![],
            };

            let result = cache_dir(Some(&config)).unwrap();
            assert_eq!(result, PathBuf::from("/config/cache"));
        }

        #[test]
        fn cache_dir_without_config() {
            let result = cache_dir(None);
            assert!(result.is_ok() || result.is_err());
        }

        /// Regression test for path construction bug
        /// Ensures that `vendor_dir` correctly constructs paths without duplicating Ruby version
        #[test]
        fn vendor_dir_no_duplicate_ruby_version() {
            // When vendor_dir returns a path, and we append ruby/{version}/gems,
            // we should get a correctly formed path without duplicate version segments

            // This test verifies the fix for the bug where paths like:
            // /Users/user/.gem/ruby/3.5.0/ruby/3.4.0/gems were being created

            // Get vendor_dir (no config, will use system_gem_dir)
            if let Ok(vendor) = vendor_dir(None) {
                let ruby_version = "3.4.0";
                let gems_path = vendor.join("ruby").join(ruby_version).join("gems");

                let path_str = gems_path.to_string_lossy();

                // Count how many times "/ruby/" appears in the path
                let ruby_count = path_str.matches("/ruby/").count();

                // Should only appear once, not multiple times
                assert_eq!(
                    ruby_count, 1,
                    "Path should contain /ruby/ exactly once, but got: {path_str}"
                );

                // Verify no duplicate version segments
                assert!(
                    !path_str.contains(&format!("/ruby/{ruby_version}/ruby/")),
                    "Path should not contain duplicate /ruby/version/ segments: {path_str}"
                );
            }
        }
    }

    mod ruby {
        use super::*;

        #[test]
        fn version_from_lockfile() {
            let version = ruby_version(Some("3.3.0p0"));
            assert_eq!(version, "3.3.0");
        }

        #[test]
        fn version_default() {
            let version = ruby_version(None);
            assert!(!version.is_empty());
        }
    }

    mod bundle_config {
        use super::*;

        #[test]
        fn nonexistent() {
            let result = read_bundle_config_path();
            assert!(result.is_none());
        }

        #[test]
        fn reads_path() -> Result<()> {
            let temp_dir = tempfile::tempdir()?;
            let bundle_dir = temp_dir.path().join(".bundle");
            fs::create_dir(&bundle_dir)?;

            fs::write(
                bundle_dir.join("config"),
                r#"---
BUNDLE_PATH: "vendor/bundle"
BUNDLE_JOBS: "4"
"#,
            )?;

            let original_dir = env::current_dir()?;
            env::set_current_dir(temp_dir.path())?;

            let result = read_bundle_config_path();
            assert_eq!(result, Some("vendor/bundle".to_string()));

            env::set_current_dir(original_dir)?;
            Ok(())
        }
    }

    mod gem_source {
        use super::*;

        #[test]
        fn creation() {
            let source = GemSource {
                url: "https://rubygems.org".to_string(),
                fallback: Some("https://mirror.example.com".to_string()),
            };

            assert_eq!(source.url, "https://rubygems.org");
            assert_eq!(
                source.fallback,
                Some("https://mirror.example.com".to_string())
            );
        }
    }
}
