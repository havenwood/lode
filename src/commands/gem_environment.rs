//! Environment command
//!
//! Display gem environment information

use anyhow::{Context, Result};
use lode::{Config, config, get_system_gem_dir};
use std::env;
use std::path::PathBuf;

/// Options for gem environment command
#[derive(Debug, Default)]
pub(crate) struct EnvironmentOptions {
    /// Show specific variable (gemdir, gempath, version, remotesources, platform, etc.)
    pub variable: Option<String>,

    /// Verbose output
    pub verbose: bool,

    /// Quiet mode
    pub quiet: bool,
}

/// Display `RubyGems` environment information
pub(crate) fn run(options: EnvironmentOptions) -> Result<()> {
    let config = Config::load().context("Failed to load configuration")?;
    let ruby_ver = config::ruby_version(None);

    // If specific variable requested, show only that
    if let Some(var) = options.variable {
        return show_variable(&var, &config, &ruby_ver);
    }

    // Show full environment
    show_full_environment(&config, &ruby_ver, &options);

    Ok(())
}

/// Show specific environment variable
fn show_variable(var: &str, config: &Config, ruby_ver: &str) -> Result<()> {
    match var.to_lowercase().as_str() {
        "gemdir" => {
            let gem_dir = get_system_gem_dir(ruby_ver);
            println!("{}", gem_dir.display());
        }
        "gempath" | "path" => {
            let gem_paths = get_gem_paths(ruby_ver);
            let path_str = gem_paths
                .iter()
                .map(|p| p.display().to_string())
                .collect::<Vec<_>>()
                .join(":");
            println!("{path_str}");
        }
        "version" => {
            println!("{}", env!("CARGO_PKG_VERSION"));
        }
        "remotesources" => {
            let sources = get_remote_sources(config);
            for source in sources {
                println!("{source}");
            }
        }
        "platform" => {
            let platform = get_platform_string();
            println!("ruby:{platform}");
        }
        "home" | "gemhome" => {
            let home = get_gem_home(ruby_ver);
            println!("{}", home.display());
        }
        "user_gemhome" | "user_gemdir" => {
            let user_dir = get_user_gem_dir(ruby_ver);
            println!("{}", user_dir.display());
        }
        _ => {
            anyhow::bail!("Unknown environment variable: {var}");
        }
    }

    Ok(())
}

/// Show full environment information
fn show_full_environment(config: &Config, ruby_ver: &str, options: &EnvironmentOptions) {
    // In quiet mode, suppress all output
    if options.quiet {
        return;
    }

    println!("RubyGems Environment:");
    println!("  - RUBYGEMS VERSION: {}", env!("CARGO_PKG_VERSION"));
    println!("  - RUBY VERSION: {}", get_ruby_version_full());
    println!(
        "  - INSTALLATION DIRECTORY: {}",
        get_system_gem_dir(ruby_ver).display()
    );

    let user_dir = get_user_gem_dir(ruby_ver);
    println!("  - USER INSTALLATION DIRECTORY: {}", user_dir.display());

    let bin_dir = get_bin_dir(ruby_ver);
    println!("  - RUBY EXECUTABLE: {}", get_ruby_executable());
    println!("  - EXECUTABLE DIRECTORY: {}", bin_dir.display());

    println!(
        "  - SPEC CACHE DIRECTORY: {}",
        get_spec_cache_dir().display()
    );
    println!(
        "  - SYSTEM CONFIGURATION DIRECTORY: {}",
        get_system_config_dir().display()
    );

    // Gem paths
    println!("  - RUBYGEMS PLATFORMS:");
    for platform in get_platforms() {
        println!("    - {platform}");
    }

    // Remote sources
    println!("  - GEM PATHS:");
    for path in get_gem_paths(ruby_ver) {
        println!("     - {}", path.display());
    }

    println!("  - GEM CONFIGURATION:");
    if let Ok(cache_dir) = config::cache_dir(Some(config)) {
        println!("     - :cachedir => {:?}", cache_dir.to_string_lossy());
    }
    println!("     - :concurrent_downloads => 8");

    println!("  - REMOTE SOURCES:");
    for source in get_remote_sources(config) {
        println!("     - {source}");
    }

    println!("  - SHELL PATH:");
    if let Ok(path_var) = env::var("PATH") {
        for path in path_var.split(':') {
            println!("     - {path}");
        }
    }

    if options.verbose {
        println!("\n  - ENVIRONMENT VARIABLES:");
        if let Ok(gem_home) = env::var("GEM_HOME") {
            println!("     - GEM_HOME: {gem_home}");
        }
        if let Ok(gem_path) = env::var("GEM_PATH") {
            println!("     - GEM_PATH: {gem_path}");
        }
        if let Ok(gem_spec_cache) = env::var("GEM_SPEC_CACHE") {
            println!("     - GEM_SPEC_CACHE: {gem_spec_cache}");
        }
    }
}

/// Get Ruby version string
fn get_ruby_version_full() -> String {
    let version = config::ruby_version(None);
    format!("{} ({})", version, get_platform_string())
}

/// Get Ruby executable path
fn get_ruby_executable() -> String {
    use std::process::Command;

    // Try to get ruby path from 'which ruby' command
    if let Ok(output) = Command::new("which").arg("ruby").output()
        && output.status.success()
        && let Ok(path) = String::from_utf8(output.stdout)
    {
        return path.trim().to_string();
    }

    String::from("ruby")
}

/// Get gem home directory
fn get_gem_home(ruby_ver: &str) -> PathBuf {
    env::var("GEM_HOME").map_or_else(|_| get_system_gem_dir(ruby_ver), PathBuf::from)
}

/// Get user gem directory
fn get_user_gem_dir(ruby_ver: &str) -> PathBuf {
    let home = env::var("HOME").unwrap_or_else(|_| String::from("/tmp"));
    PathBuf::from(home).join(".gem").join("ruby").join(ruby_ver)
}

/// Get binary directory
fn get_bin_dir(ruby_ver: &str) -> PathBuf {
    get_system_gem_dir(ruby_ver).join("bin")
}

/// Get spec cache directory
fn get_spec_cache_dir() -> PathBuf {
    let home = env::var("HOME").unwrap_or_else(|_| String::from("/tmp"));
    PathBuf::from(home).join(".gem").join("specs")
}

/// Get system configuration directory
fn get_system_config_dir() -> PathBuf {
    if cfg!(target_os = "macos") {
        PathBuf::from("/Library/Ruby/Site")
    } else if cfg!(target_os = "linux") {
        PathBuf::from("/etc/rubygems")
    } else {
        PathBuf::from("/etc")
    }
}

/// Get supported platforms
fn get_platforms() -> Vec<String> {
    let mut platforms = vec!["ruby".to_string()];

    let platform = get_platform_string();
    if !platform.is_empty() {
        platforms.push(platform);
    }

    platforms
}

/// Get platform string
fn get_platform_string() -> String {
    let os = if cfg!(target_os = "macos") {
        "darwin"
    } else if cfg!(target_os = "linux") {
        "linux"
    } else if cfg!(target_os = "windows") {
        "mingw32"
    } else {
        "unknown"
    };

    let arch = if cfg!(target_arch = "x86_64") {
        "x86_64"
    } else if cfg!(target_arch = "aarch64") {
        "arm64"
    } else {
        env::consts::ARCH
    };

    format!("{arch}-{os}")
}

/// Get gem paths
fn get_gem_paths(ruby_ver: &str) -> Vec<PathBuf> {
    let mut paths = Vec::new();

    // GEM_PATH environment variable
    if let Ok(gem_path) = env::var("GEM_PATH") {
        for path in gem_path.split(':') {
            paths.push(PathBuf::from(path));
        }
        return paths;
    }

    // Default paths
    paths.push(get_system_gem_dir(ruby_ver));
    paths.push(get_user_gem_dir(ruby_ver));

    paths
}

/// Get remote sources
fn get_remote_sources(config: &Config) -> Vec<String> {
    let mut sources = Vec::new();

    // Add sources from configuration
    for source in &config.gem_sources {
        sources.push(source.url.clone());
    }

    // Add default RubyGems.org if no sources configured
    if sources.is_empty() {
        sources.push("https://rubygems.org/".to_string());
    }

    sources
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "Tests can panic")]
mod tests {
    use super::*;

    #[test]
    fn environment_options_default() {
        let options = EnvironmentOptions::default();
        assert!(options.variable.is_none());
        assert!(!options.verbose);
        assert!(!options.quiet);
    }

    #[test]
    fn test_get_platform_string() {
        let platform = get_platform_string();
        assert!(!platform.is_empty());
        assert!(platform.contains('-'));
    }

    #[test]
    fn test_get_platforms() {
        let platforms = get_platforms();
        assert!(!platforms.is_empty());
        assert!(platforms.contains(&"ruby".to_string()));
    }

    #[test]
    fn test_get_gem_paths() {
        let ruby_ver = "3.3.0";
        let paths = get_gem_paths(ruby_ver);
        assert!(!paths.is_empty());
    }
}
