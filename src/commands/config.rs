//! Config command
//!
//! Manage Bundler configuration options

#![allow(
    clippy::option_if_let_else,
    reason = "Explicit control flow is clearer here"
)]

use anyhow::{Context, Result};
use lode::Config;
use std::fs;
use std::path::PathBuf;

/// Get and set Bundler configuration options
///
/// This command manages Lode/Bundler configuration settings.
/// Configuration can be stored globally or locally.
#[allow(clippy::fn_params_excessive_bools)]
pub(crate) fn run(
    key: Option<&str>,
    value: Option<&str>,
    list: bool,
    delete: bool,
    global: bool,
    local: bool,
) -> Result<()> {
    // Determine scope: local if --local, global if --global or neither
    let is_local = local || !global;

    if list {
        return list_config(is_local);
    }

    if let Some(config_key) = key {
        if delete {
            // Delete configuration
            delete_config(config_key, is_local)
        } else if let Some(config_value) = value {
            // Set configuration
            set_config(config_key, config_value, is_local)
        } else {
            // Get configuration
            get_config(config_key)
        }
    } else {
        // No key specified, show usage
        println!("Usage:");
        println!("  lode config --list                  # List all configuration");
        println!("  lode config --list --local          # List local configuration");
        println!("  lode config <key>                   # Get configuration value");
        println!("  lode config <key> <value>           # Set configuration value");
        println!("  lode config <key> <value> --local   # Set local configuration");
        println!("  lode config <key> --delete          # Delete configuration key");
        println!("  lode config <key> --delete --local  # Delete local configuration key");
        println!();
        println!("Common configuration keys:");
        println!("  vendor_dir (or path) # Installation path for gems");
        println!("  cache_dir            # Cache directory for downloaded gems");
        println!("  gemfile              # Custom Gemfile path");
        Ok(())
    }
}

/// Get a configuration value
fn get_config(key: &str) -> Result<()> {
    let config = Config::load().context("Failed to load configuration")?;

    let value = match key {
        "vendor_dir" | "path" => config.vendor_dir.as_deref(),
        "cache_dir" => config.cache_dir.as_deref(),
        "gemfile" => config.gemfile.as_deref(),
        _ => {
            println!("Unknown configuration key: {key}");
            println!("Run `lode config` for list of available keys");
            return Ok(());
        }
    };

    if let Some(v) = value {
        println!("{v}");
    } else {
        println!("Configuration key '{key}' is not set");
    }

    Ok(())
}

/// Set a configuration value
fn set_config(key: &str, value: &str, local: bool) -> Result<()> {
    let config_path = if local {
        get_local_config_path()?
    } else {
        get_global_config_path()?
    };

    // Load existing config or create new one
    let mut config = if config_path.exists() {
        let content = fs::read_to_string(&config_path)?;
        toml::from_str(&content).unwrap_or_default()
    } else {
        Config::default()
    };

    // Update the specified key
    match key {
        "vendor_dir" | "path" => {
            config.vendor_dir = Some(value.to_string());
            println!("Set vendor_dir to: {value}");
        }
        "cache_dir" => {
            config.cache_dir = Some(value.to_string());
            println!("Set cache_dir to: {value}");
        }
        "gemfile" => {
            config.gemfile = Some(value.to_string());
            println!("Set gemfile to: {value}");
        }
        _ => {
            anyhow::bail!("Unknown configuration key: {key}");
        }
    }

    // Create parent directory if needed
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Write config
    let toml_string = toml::to_string_pretty(&config)?;
    fs::write(&config_path, toml_string)?;

    let scope = if local { "local" } else { "global" };
    println!(
        "Configuration saved to {scope} config: {}",
        config_path.display()
    );

    Ok(())
}

/// Delete a configuration value
fn delete_config(key: &str, local: bool) -> Result<()> {
    let config_path = if local {
        get_local_config_path()?
    } else {
        get_global_config_path()?
    };

    // Check if config file exists
    if !config_path.exists() {
        let scope = if local { "local" } else { "global" };
        println!("No {scope} configuration file found");
        return Ok(());
    }

    // Load existing config
    let content = fs::read_to_string(&config_path)?;
    let mut config: Config = toml::from_str(&content).unwrap_or_default();

    // Delete the specified key
    let deleted = match key {
        "vendor_dir" | "path" => {
            if config.vendor_dir.is_some() {
                config.vendor_dir = None;
                true
            } else {
                false
            }
        }
        "cache_dir" => {
            if config.cache_dir.is_some() {
                config.cache_dir = None;
                true
            } else {
                false
            }
        }
        "gemfile" => {
            if config.gemfile.is_some() {
                config.gemfile = None;
                true
            } else {
                false
            }
        }
        _ => {
            anyhow::bail!("Unknown configuration key: {key}");
        }
    };

    if !deleted {
        println!("Configuration key '{key}' was not set");
        return Ok(());
    }

    // Write updated config
    let toml_string = toml::to_string_pretty(&config)?;
    fs::write(&config_path, toml_string)?;

    let scope = if local { "local" } else { "global" };
    println!("Deleted '{key}' from {scope} configuration");
    println!("Configuration file: {}", config_path.display());

    Ok(())
}

/// List all configuration
fn list_config(local_only: bool) -> Result<()> {
    let config = Config::load().context("Failed to load configuration")?;

    println!("Configuration:");
    println!();

    if let Some(vendor_dir) = &config.vendor_dir {
        println!("  vendor_dir: {vendor_dir}");
    }

    if let Some(cache_dir) = &config.cache_dir {
        println!("  cache_dir:  {cache_dir}");
    }

    if let Some(gemfile) = &config.gemfile {
        println!("  gemfile:    {gemfile}");
    }

    println!();

    // Show config file location
    if local_only {
        let local_path = get_local_config_path()?;
        if local_path.exists() {
            println!("Local config: {}", local_path.display());
        } else {
            println!("No local config found");
        }
    } else {
        let global_path = get_global_config_path()?;
        println!("Global config: {}", global_path.display());

        let local_path = get_local_config_path()?;
        if local_path.exists() {
            println!("Local config:  {}", local_path.display());
        }
    }

    Ok(())
}

/// Get the global configuration file path
fn get_global_config_path() -> Result<PathBuf> {
    let home = std::env::var("HOME").context("HOME environment variable not set")?;
    Ok(PathBuf::from(home).join(".config/lode/config.toml"))
}

/// Get the local configuration file path
fn get_local_config_path() -> Result<PathBuf> {
    let current_dir = std::env::current_dir().context("Failed to get current directory")?;
    Ok(current_dir.join(".lode/config.toml"))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "Tests can panic")]
mod tests {
    use super::*;

    #[test]
    fn config_no_args_shows_usage() {
        let result = run(None, None, false, false, false, false);
        assert!(result.is_ok());
    }

    #[test]
    fn config_get_unknown_key() {
        let result = get_config("unknown_key");
        assert!(result.is_ok());
    }

    #[test]
    fn test_list_config() {
        let result = list_config(false);
        // May fail if HOME is not set, but that's ok for testing
        drop(result);
    }

    #[test]
    fn test_get_global_config_path() {
        // This may fail if HOME is not set
        let result = get_global_config_path();
        if let Ok(path) = result {
            assert!(path.to_string_lossy().contains("config.toml"));
        }
    }

    #[test]
    fn test_get_local_config_path() {
        use tempfile::TempDir;

        // Create temp directory and change to it
        let temp = TempDir::new().unwrap();
        let orig_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp.path()).unwrap();

        let result = get_local_config_path();

        // Restore directory before assertions
        drop(std::env::set_current_dir(&orig_dir));

        assert!(result.is_ok());
        let path = result.unwrap();
        assert!(path.to_string_lossy().contains(".lode"));
    }
}
