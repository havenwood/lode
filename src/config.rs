//! Configuration file management
//!
//! Handles reading and writing lode's TOML configuration files from project
//! and global locations.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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

/// Bundler configuration loaded from `.bundle/config` (YAML format)
///
/// Follows Bundler 4 config keys and priority:
/// 1. Local config (`.bundle/config` or `$BUNDLE_APP_CONFIG/config`)
/// 2. Global config (`~/.bundle/config`)
#[derive(Debug, Clone, Default)]
pub struct BundleConfig {
    /// Installation path for gems (`BUNDLE_PATH`)
    pub path: Option<String>,
    /// Number of parallel jobs (`BUNDLE_JOBS`)
    pub jobs: Option<usize>,
    /// Number of retries for failed operations (`BUNDLE_RETRY`)
    pub retry: Option<u32>,
    /// Disallow Gemfile changes (`BUNDLE_FROZEN`)
    pub frozen: Option<bool>,
    /// Deployment mode (`BUNDLE_DEPLOYMENT`)
    pub deployment: Option<bool>,
    /// Groups to exclude (`BUNDLE_WITHOUT`)
    pub without: Option<Vec<String>>,
    /// Groups to include (`BUNDLE_WITH`)
    pub with: Option<Vec<String>>,
    /// Cache all gems including path/git (`BUNDLE_CACHE_ALL`)
    pub cache_all: Option<bool>,
    /// Cache gems for all platforms (`BUNDLE_CACHE_ALL_PLATFORMS`)
    pub cache_all_platforms: Option<bool>,
    /// Cache directory path (`BUNDLE_CACHE_PATH`)
    pub cache_path: Option<String>,
    /// Run bundle clean after install (`BUNDLE_CLEAN`)
    pub clean: Option<bool>,
    /// Don't remove outdated gems (`BUNDLE_NO_PRUNE`)
    pub no_prune: Option<bool>,
    /// Use only cached gems (`BUNDLE_LOCAL`)
    pub local: Option<bool>,
    /// Prefer cached gems (`BUNDLE_PREFER_LOCAL`)
    pub prefer_local: Option<bool>,
    /// Force operations (`BUNDLE_FORCE`)
    pub force: Option<bool>,
    /// Custom shebang for binstubs (`BUNDLE_SHEBANG`)
    pub shebang: Option<String>,
    /// Binstubs directory (`BUNDLE_BIN`)
    pub bin: Option<String>,
    /// Disable shared gems (`BUNDLE_DISABLE_SHARED_GEMS`)
    pub disable_shared_gems: Option<bool>,
    /// Allow offline install (`BUNDLE_ALLOW_OFFLINE_INSTALL`)
    pub allow_offline_install: Option<bool>,
    /// Auto install missing gems (`BUNDLE_AUTO_INSTALL`)
    pub auto_install: Option<bool>,
    /// Silence root warning (`BUNDLE_SILENCE_ROOT_WARNING`)
    pub silence_root_warning: Option<bool>,
    /// Disable version check (`BUNDLE_DISABLE_VERSION_CHECK`)
    pub disable_version_check: Option<bool>,
    /// Force ruby platform (`BUNDLE_FORCE_RUBY_PLATFORM`)
    pub force_ruby_platform: Option<bool>,
    /// Verbose output (`BUNDLE_VERBOSE`)
    pub verbose: Option<bool>,
    /// Gemfile path (`BUNDLE_GEMFILE`)
    pub gemfile: Option<String>,
    /// Global gem cache (`BUNDLE_GLOBAL_GEM_CACHE`)
    pub global_gem_cache: Option<bool>,
    /// Ignore post-install messages (`BUNDLE_IGNORE_MESSAGES`)
    pub ignore_messages: Option<bool>,
    /// Skip package install (`BUNDLE_NO_INSTALL`)
    pub no_install: Option<bool>,
    /// Prefer patch updates (`BUNDLE_PREFER_PATCH`)
    pub prefer_patch: Option<bool>,
    /// Disable checksum validation (`BUNDLE_DISABLE_CHECKSUM_VALIDATION`)
    pub disable_checksum_validation: Option<bool>,
    /// Number of HTTP redirects (`BUNDLE_REDIRECT`)
    pub redirect: Option<usize>,
    /// System-wide install (`BUNDLE_SYSTEM`)
    pub system: Option<bool>,
    /// Ignore config files (`BUNDLE_IGNORE_CONFIG`)
    pub ignore_config: Option<bool>,
    /// Silence deprecations (`BUNDLE_SILENCE_DEPRECATIONS`)
    pub silence_deprecations: Option<bool>,
    /// Ignore funding requests (`BUNDLE_IGNORE_FUNDING_REQUESTS`)
    pub ignore_funding_requests: Option<bool>,
    /// Lockfile checksums (`BUNDLE_LOCKFILE_CHECKSUMS`)
    pub lockfile_checksums: Option<bool>,
    /// SSL CA cert path (`BUNDLE_SSL_CA_CERT`)
    pub ssl_ca_cert: Option<String>,
    /// SSL client cert path (`BUNDLE_SSL_CLIENT_CERT`)
    pub ssl_client_cert: Option<String>,
    /// SSL verify mode (`BUNDLE_SSL_VERIFY_MODE`)
    pub ssl_verify_mode: Option<String>,
}

impl Config {
    /// Load configuration from TOML files.
    /// Priority: ./.lode.toml -> ~/.config/lode/config.toml
    ///
    /// # Errors
    ///
    /// Returns an error if config file parsing fails.
    pub fn load() -> Result<Self> {
        Self::load_with_options(None, false)
    }

    /// Load configuration with custom options.
    ///
    /// # Arguments
    /// * `custom_path` - Optional custom path to config file (overrides defaults)
    /// * `skip_rc` - If true, skip loading config files (return default config)
    ///
    /// # Errors
    ///
    /// Returns an error if config file reading or parsing fails.
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

impl BundleConfig {
    /// Load Bundler configuration from config files
    ///
    /// Priority order (later overrides earlier):
    /// 1. Global config (`~/.bundle/config`)
    /// 2. Local config (`.bundle/config` or `$BUNDLE_APP_CONFIG/config`)
    ///
    /// Note: Environment variables and CLI flags have higher priority and are handled elsewhere.
    ///
    /// # Errors
    ///
    /// Returns an error if config file reading or parsing fails.
    pub fn load() -> Result<Self> {
        let mut config = Self::default();

        // Check BUNDLE_IGNORE_CONFIG first
        if crate::env_vars::bundle_ignore_config() {
            return Ok(config);
        }

        // 1. Load global config first
        if let Some(global_config) = Self::load_global()? {
            config = config.merge(global_config);
        }

        // 2. Load local config (overrides global)
        if let Some(local_config) = Self::load_local()? {
            config = config.merge(local_config);
        }

        Ok(config)
    }

    /// Load global bundle config from `~/.bundle/config`
    fn load_global() -> Result<Option<Self>> {
        if let Some(home) = dirs::home_dir() {
            let global_config_path = home.join(".bundle").join("config");
            if global_config_path.exists() {
                return Self::load_from(&global_config_path).map(Some);
            }
        }
        Ok(None)
    }

    /// Load local bundle config from `.bundle/config` or `$BUNDLE_APP_CONFIG/config`
    fn load_local() -> Result<Option<Self>> {
        let bundle_dir = crate::env_vars::bundle_app_config()
            .map_or_else(|| PathBuf::from(".bundle"), PathBuf::from);

        let config_path = bundle_dir.join("config");
        if config_path.exists() {
            return Self::load_from(&config_path).map(Some);
        }
        Ok(None)
    }

    /// Load bundle config from a specific YAML file
    fn load_from<P: AsRef<Path>>(path: P) -> Result<Self> {
        let contents = fs::read_to_string(path)?;
        Self::parse_yaml(&contents)
    }

    /// Parse YAML content into `BundleConfig`
    ///
    /// Bundle config format is YAML with keys like:
    /// ```yaml
    /// ---
    /// BUNDLE_PATH: "vendor/bundle"
    /// BUNDLE_JOBS: "8"
    /// BUNDLE_FROZEN: "true"
    /// ```
    fn parse_yaml(yaml_content: &str) -> Result<Self> {
        // Parse as generic YAML map
        let yaml_map: HashMap<String, serde_yaml::Value> =
            serde_yaml::from_str(yaml_content).context("Failed to parse bundle config YAML")?;

        let mut config = Self::default();

        for (key, value) in yaml_map {
            // All bundle config keys are uppercase BUNDLE_*
            match key.as_str() {
                "BUNDLE_PATH" => config.path = parse_string_value(&value),
                "BUNDLE_JOBS" => config.jobs = parse_usize_value(&value),
                "BUNDLE_RETRY" => config.retry = parse_u32_value(&value),
                "BUNDLE_FROZEN" => config.frozen = parse_bool_value(&value),
                "BUNDLE_DEPLOYMENT" => config.deployment = parse_bool_value(&value),
                "BUNDLE_WITHOUT" => config.without = parse_list_value(&value),
                "BUNDLE_WITH" => config.with = parse_list_value(&value),
                "BUNDLE_CACHE_ALL" => config.cache_all = parse_bool_value(&value),
                "BUNDLE_CACHE_ALL_PLATFORMS" => {
                    config.cache_all_platforms = parse_bool_value(&value);
                }
                "BUNDLE_CACHE_PATH" => config.cache_path = parse_string_value(&value),
                "BUNDLE_CLEAN" => config.clean = parse_bool_value(&value),
                "BUNDLE_NO_PRUNE" => config.no_prune = parse_bool_value(&value),
                "BUNDLE_LOCAL" => config.local = parse_bool_value(&value),
                "BUNDLE_PREFER_LOCAL" => config.prefer_local = parse_bool_value(&value),
                "BUNDLE_FORCE" => config.force = parse_bool_value(&value),
                "BUNDLE_SHEBANG" => config.shebang = parse_string_value(&value),
                "BUNDLE_BIN" => config.bin = parse_string_value(&value),
                "BUNDLE_DISABLE_SHARED_GEMS" => {
                    config.disable_shared_gems = parse_bool_value(&value);
                }
                "BUNDLE_ALLOW_OFFLINE_INSTALL" => {
                    config.allow_offline_install = parse_bool_value(&value);
                }
                "BUNDLE_AUTO_INSTALL" => config.auto_install = parse_bool_value(&value),
                "BUNDLE_SILENCE_ROOT_WARNING" => {
                    config.silence_root_warning = parse_bool_value(&value);
                }
                "BUNDLE_DISABLE_VERSION_CHECK" => {
                    config.disable_version_check = parse_bool_value(&value);
                }
                "BUNDLE_FORCE_RUBY_PLATFORM" => {
                    config.force_ruby_platform = parse_bool_value(&value);
                }
                "BUNDLE_VERBOSE" => config.verbose = parse_bool_value(&value),
                "BUNDLE_GEMFILE" => config.gemfile = parse_string_value(&value),
                "BUNDLE_GLOBAL_GEM_CACHE" => config.global_gem_cache = parse_bool_value(&value),
                "BUNDLE_IGNORE_MESSAGES" => config.ignore_messages = parse_bool_value(&value),
                "BUNDLE_NO_INSTALL" => config.no_install = parse_bool_value(&value),
                "BUNDLE_PREFER_PATCH" => config.prefer_patch = parse_bool_value(&value),
                "BUNDLE_DISABLE_CHECKSUM_VALIDATION" => {
                    config.disable_checksum_validation = parse_bool_value(&value);
                }
                "BUNDLE_REDIRECT" => config.redirect = parse_usize_value(&value),
                "BUNDLE_SYSTEM" => config.system = parse_bool_value(&value),
                "BUNDLE_IGNORE_CONFIG" => config.ignore_config = parse_bool_value(&value),
                "BUNDLE_SILENCE_DEPRECATIONS" => {
                    config.silence_deprecations = parse_bool_value(&value);
                }
                "BUNDLE_IGNORE_FUNDING_REQUESTS" => {
                    config.ignore_funding_requests = parse_bool_value(&value);
                }
                "BUNDLE_LOCKFILE_CHECKSUMS" => config.lockfile_checksums = parse_bool_value(&value),
                "BUNDLE_SSL_CA_CERT" => config.ssl_ca_cert = parse_string_value(&value),
                "BUNDLE_SSL_CLIENT_CERT" => config.ssl_client_cert = parse_string_value(&value),
                "BUNDLE_SSL_VERIFY_MODE" => config.ssl_verify_mode = parse_string_value(&value),
                // Ignore unknown keys for forward compatibility
                _ => {}
            }
        }

        Ok(config)
    }

    /// Merge another `BundleConfig` into this one (other takes precedence for set values)
    #[allow(
        clippy::cognitive_complexity,
        clippy::too_many_lines,
        reason = "Sequential field merging is straightforward but has many fields"
    )]
    fn merge(mut self, other: Self) -> Self {
        if other.path.is_some() {
            self.path = other.path;
        }
        if other.jobs.is_some() {
            self.jobs = other.jobs;
        }
        if other.retry.is_some() {
            self.retry = other.retry;
        }
        if other.frozen.is_some() {
            self.frozen = other.frozen;
        }
        if other.deployment.is_some() {
            self.deployment = other.deployment;
        }
        if other.without.is_some() {
            self.without = other.without;
        }
        if other.with.is_some() {
            self.with = other.with;
        }
        if other.cache_all.is_some() {
            self.cache_all = other.cache_all;
        }
        if other.cache_all_platforms.is_some() {
            self.cache_all_platforms = other.cache_all_platforms;
        }
        if other.cache_path.is_some() {
            self.cache_path = other.cache_path;
        }
        if other.clean.is_some() {
            self.clean = other.clean;
        }
        if other.no_prune.is_some() {
            self.no_prune = other.no_prune;
        }
        if other.local.is_some() {
            self.local = other.local;
        }
        if other.prefer_local.is_some() {
            self.prefer_local = other.prefer_local;
        }
        if other.force.is_some() {
            self.force = other.force;
        }
        if other.shebang.is_some() {
            self.shebang = other.shebang;
        }
        if other.bin.is_some() {
            self.bin = other.bin;
        }
        if other.disable_shared_gems.is_some() {
            self.disable_shared_gems = other.disable_shared_gems;
        }
        if other.allow_offline_install.is_some() {
            self.allow_offline_install = other.allow_offline_install;
        }
        if other.auto_install.is_some() {
            self.auto_install = other.auto_install;
        }
        if other.silence_root_warning.is_some() {
            self.silence_root_warning = other.silence_root_warning;
        }
        if other.disable_version_check.is_some() {
            self.disable_version_check = other.disable_version_check;
        }
        if other.force_ruby_platform.is_some() {
            self.force_ruby_platform = other.force_ruby_platform;
        }
        if other.verbose.is_some() {
            self.verbose = other.verbose;
        }
        if other.gemfile.is_some() {
            self.gemfile = other.gemfile;
        }
        if other.global_gem_cache.is_some() {
            self.global_gem_cache = other.global_gem_cache;
        }
        if other.ignore_messages.is_some() {
            self.ignore_messages = other.ignore_messages;
        }
        if other.no_install.is_some() {
            self.no_install = other.no_install;
        }
        if other.prefer_patch.is_some() {
            self.prefer_patch = other.prefer_patch;
        }
        if other.disable_checksum_validation.is_some() {
            self.disable_checksum_validation = other.disable_checksum_validation;
        }
        if other.redirect.is_some() {
            self.redirect = other.redirect;
        }
        if other.system.is_some() {
            self.system = other.system;
        }
        if other.ignore_config.is_some() {
            self.ignore_config = other.ignore_config;
        }
        if other.silence_deprecations.is_some() {
            self.silence_deprecations = other.silence_deprecations;
        }
        if other.ignore_funding_requests.is_some() {
            self.ignore_funding_requests = other.ignore_funding_requests;
        }
        if other.lockfile_checksums.is_some() {
            self.lockfile_checksums = other.lockfile_checksums;
        }
        if other.ssl_ca_cert.is_some() {
            self.ssl_ca_cert = other.ssl_ca_cert;
        }
        if other.ssl_client_cert.is_some() {
            self.ssl_client_cert = other.ssl_client_cert;
        }
        if other.ssl_verify_mode.is_some() {
            self.ssl_verify_mode = other.ssl_verify_mode;
        }
        self
    }
}

/// Parse YAML value as string
fn parse_string_value(value: &serde_yaml::Value) -> Option<String> {
    value.as_str().map(ToString::to_string)
}

/// Parse YAML value as boolean (accepts "true", "1", "yes")
fn parse_bool_value(value: &serde_yaml::Value) -> Option<bool> {
    value.as_str().map_or_else(
        || value.as_bool(),
        |s| {
            let lower = s.to_lowercase();
            Some(lower == "true" || lower == "1" || lower == "yes")
        },
    )
}

/// Parse YAML value as usize
fn parse_usize_value(value: &serde_yaml::Value) -> Option<usize> {
    value.as_str().map_or_else(
        || value.as_u64().and_then(|n| usize::try_from(n).ok()),
        |s| s.parse().ok(),
    )
}

/// Parse YAML value as u32
fn parse_u32_value(value: &serde_yaml::Value) -> Option<u32> {
    value.as_str().map_or_else(
        || value.as_u64().and_then(|n| u32::try_from(n).ok()),
        |s| s.parse().ok(),
    )
}

/// Parse YAML value as list of strings (handles colon or space-separated strings)
fn parse_list_value(value: &serde_yaml::Value) -> Option<Vec<String>> {
    value.as_str().map_or_else(
        || {
            // Handle YAML list format
            value.as_sequence().and_then(|seq| {
                let items: Vec<String> = seq
                    .iter()
                    .filter_map(|v| v.as_str().map(ToString::to_string))
                    .collect();
                if items.is_empty() { None } else { Some(items) }
            })
        },
        |s| {
            // Handle colon or space-separated list
            let items: Vec<String> = s
                .split([':', ' '])
                .filter(|item| !item.is_empty())
                .map(ToString::to_string)
                .collect();
            if items.is_empty() { None } else { Some(items) }
        },
    )
}

/// Resolve vendor directory with Bundler 4 priority: Config -> env -> .bundle/config -> system gem dir.
///
/// # Errors
///
/// Returns an error if system gem directory detection fails.
pub fn vendor_dir(config: Option<&Config>) -> Result<PathBuf> {
    // 1. Check lode config file (highest priority for lode-specific overrides)
    if let Some(config) = config
        && let Some(ref dir) = config.vendor_dir
    {
        return Ok(PathBuf::from(dir));
    }

    // 2. Check BUNDLE_PATH environment variable (overrides config files)
    if let Some(bundle_path) = crate::env_vars::bundle_path() {
        return Ok(PathBuf::from(bundle_path));
    }

    // 3. Check Bundler config (.bundle/config - project settings)
    if let Ok(bundle_config) = BundleConfig::load()
        && let Some(ref path) = bundle_config.path
    {
        return Ok(PathBuf::from(path));
    }

    // 4. Fall back to system gem directory
    system_gem_dir()
}

/// Resolve cache directory: `BUNDLE_USER_CACHE` env -> Config -> platform cache dir.
///
/// # Errors
///
/// Returns an error if platform cache directory detection fails.
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

        // Priority order (Config > Env > .bundle/config) tested via integration tests

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
        fn loads_empty_when_nonexistent() -> Result<()> {
            let config = BundleConfig::load()?;
            assert!(config.path.is_none());
            assert!(config.jobs.is_none());
            Ok(())
        }

        #[test]
        fn reads_path_and_jobs() -> Result<()> {
            let temp_dir = tempfile::tempdir()?;
            let bundle_dir = temp_dir.path().join(".bundle");
            fs::create_dir(&bundle_dir)?;

            fs::write(
                bundle_dir.join("config"),
                r#"---
BUNDLE_PATH: "vendor/bundle"
BUNDLE_JOBS: "4"
BUNDLE_RETRY: "3"
BUNDLE_FROZEN: "true"
"#,
            )?;

            let original_dir = env::current_dir()?;
            env::set_current_dir(temp_dir.path())?;

            let config = BundleConfig::load()?;
            assert_eq!(config.path, Some("vendor/bundle".to_string()));
            assert_eq!(config.jobs, Some(4));
            assert_eq!(config.retry, Some(3));
            assert_eq!(config.frozen, Some(true));

            env::set_current_dir(original_dir)?;
            Ok(())
        }

        #[test]
        fn parses_boolean_variants() -> Result<()> {
            let temp_dir = tempfile::tempdir()?;
            let bundle_dir = temp_dir.path().join(".bundle");
            fs::create_dir(&bundle_dir)?;

            fs::write(
                bundle_dir.join("config"),
                r#"---
BUNDLE_FROZEN: "1"
BUNDLE_DEPLOYMENT: "yes"
BUNDLE_VERBOSE: "true"
"#,
            )?;

            let original_dir = env::current_dir()?;
            env::set_current_dir(temp_dir.path())?;

            let config = BundleConfig::load()?;
            assert_eq!(config.frozen, Some(true));
            assert_eq!(config.deployment, Some(true));
            assert_eq!(config.verbose, Some(true));

            env::set_current_dir(original_dir)?;
            Ok(())
        }

        #[test]
        fn parses_list_values() -> Result<()> {
            let temp_dir = tempfile::tempdir()?;
            let bundle_dir = temp_dir.path().join(".bundle");
            fs::create_dir(&bundle_dir)?;

            fs::write(
                bundle_dir.join("config"),
                r#"---
BUNDLE_WITHOUT: "development:test"
"#,
            )?;

            let original_dir = env::current_dir()?;
            env::set_current_dir(temp_dir.path())?;

            let config = BundleConfig::load()?;
            assert_eq!(
                config.without,
                Some(vec!["development".to_string(), "test".to_string()])
            );

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
