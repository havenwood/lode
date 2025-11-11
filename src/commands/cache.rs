//! Cache command
//!
//! Package gems into vendor/cache directory

use anyhow::{Context, Result};
use lode::lockfile::Lockfile;
use std::fs;
use std::path::PathBuf;

/// Package gems into vendor/cache directory
///
/// Copies all .gem files needed to run the application into the vendor/cache
/// directory. Future `bundle install` commands will use these cached gems
/// in preference to fetching from rubygems.org.
pub(crate) async fn run(
    all_platforms: bool,
    cache_path: Option<&str>,
    gemfile: Option<&str>,
    no_install: bool,
    quiet: bool,
) -> Result<()> {
    // Apply environment variable defaults
    let all_platforms = all_platforms || lode::env_vars::bundle_cache_all_platforms();
    let no_install = no_install || lode::env_vars::bundle_no_install();

    // Determine paths
    let gemfile_path = gemfile.unwrap_or("Gemfile");
    let lockfile_path = format!("{gemfile_path}.lock");
    let env_cache_path = lode::env_vars::bundle_cache_path();
    let cache_dir = cache_path
        .or(env_cache_path.as_deref())
        .unwrap_or("vendor/cache");

    // Read and parse lockfile
    let lockfile_content = fs::read_to_string(&lockfile_path)
        .with_context(|| format!("Failed to read lockfile: {lockfile_path}"))?;

    let lockfile = Lockfile::parse(&lockfile_content)
        .with_context(|| format!("Failed to parse lockfile: {lockfile_path}"))?;

    if lockfile.gems.is_empty() {
        if !quiet {
            println!("No gems found in lockfile");
        }
        return Ok(());
    }

    // Create cache directory
    fs::create_dir_all(cache_dir)
        .with_context(|| format!("Failed to create cache directory: {cache_dir}"))?;

    // Get lode's internal cache directory (already includes /gems)
    let lode_cache =
        lode::config::cache_dir(None).context("Failed to determine lode cache directory")?;

    // Also check system gem cache (~/.gem/ruby/VERSION/cache)
    let ruby_version = lode::config::ruby_version(lockfile.ruby_version.as_deref());
    let home = dirs::home_dir().context("Could not determine home directory")?;
    let system_gem_cache = home
        .join(".gem")
        .join("ruby")
        .join(&ruby_version)
        .join("cache");

    // Check both cache locations
    let cache_locations = [lode_cache, system_gem_cache];
    let available_caches: Vec<_> = cache_locations.iter().filter(|c| c.exists()).collect();

    if available_caches.is_empty() {
        anyhow::bail!("No gem cache found.\nRun 'lode install' first to download gems");
    }

    // Determine which gems to cache
    let gems_to_cache: Vec<_> = if all_platforms {
        // Include all gems from lockfile regardless of platform
        lockfile.gems.iter().collect()
    } else {
        // Only include gems for current platform
        lockfile
            .gems
            .iter()
            .filter(|gem| {
                // Include gems with no platform specified (pure Ruby gems)
                // or gems matching the current platform
                gem.platform.is_none()
                    || gem.platform.as_deref() == Some("ruby")
                    || is_current_platform(gem.platform.as_deref())
            })
            .collect()
    };

    if !quiet {
        println!("Updating files in {cache_dir}");
        println!();
    }

    let mut copied = 0;
    let mut already_cached = 0;
    let mut missing = Vec::new();

    for gem in gems_to_cache {
        let gem_filename = gem.platform.as_ref().map_or_else(
            || format!("{}-{}.gem", gem.name, gem.version),
            |platform| {
                if platform == "ruby" {
                    format!("{}-{}.gem", gem.name, gem.version)
                } else {
                    format!("{}-{}-{}.gem", gem.name, gem.version, platform)
                }
            },
        );

        let dest_path = PathBuf::from(cache_dir).join(&gem_filename);

        if dest_path.exists() {
            already_cached += 1;
            continue;
        }

        // Try to find gem in any of the available cache locations
        let source_path = available_caches
            .iter()
            .map(|cache| cache.join(&gem_filename))
            .find(|path| path.exists());

        let Some(source_path) = source_path else {
            missing.push(gem_filename);
            continue;
        };

        // Copy gem file to vendor/cache
        fs::copy(&source_path, &dest_path)
            .with_context(|| format!("Failed to copy {} to {cache_dir}", source_path.display()))?;

        if !quiet {
            println!("  * {gem_filename}");
        }
        copied += 1;
    }

    if !quiet {
        println!();
        if copied > 0 {
            println!("Copied {copied} gem(s) to {cache_dir}");
        }
        if already_cached > 0 {
            println!("   {already_cached} gem(s) already in cache");
        }
    }

    if !missing.is_empty() {
        if !quiet {
            println!();
        }
        eprintln!("WARNING: Missing {} gem(s) from lode cache:", missing.len());
        for gem_file in &missing {
            eprintln!("   - {gem_file}");
        }
        if !quiet {
            eprintln!();
        }
        eprintln!("Run 'lode install' to download missing gems");
    }

    // Run install if not --no-install
    if !no_install && missing.is_empty() {
        if !quiet {
            println!();
            println!("Installing gems...");
        }
        crate::commands::install::run(crate::commands::install::InstallOptions {
            lockfile_path: &lockfile_path,
            redownload: false,
            verbose: false,
            quiet: true,
            workers: None,
            local: false,
            prefer_local: false,
            retry: None,
            no_cache: false,
            standalone: None,
            trust_policy: None,
            full_index: false,
            target_rbconfig: None,
            frozen: false,
            without_groups: vec![],
            with_groups: vec![],
            auto_clean: false,
        })
        .await?;
    }

    Ok(())
}

/// Check if a platform string matches the current platform
fn is_current_platform(platform: Option<&str>) -> bool {
    let Some(platform) = platform else {
        return true; // No platform specified means it works everywhere
    };

    // "ruby" platform means pure Ruby gem that works on any platform
    if platform == "ruby" {
        return true;
    }

    // Get current platform info
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;

    // Platform format: arch-os or arch-os-version
    // Examples: x86_64-linux, arm64-darwin, x86_64-darwin-20
    platform.contains(arch) && platform.contains(&os_to_platform_name(os))
}

/// Convert Rust OS name to platform name used in gems
fn os_to_platform_name(os: &str) -> String {
    match os {
        "macos" => "darwin".to_string(),
        "linux" => "linux".to_string(),
        "windows" => "mingw".to_string(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_platform_matching() {
        assert!(is_current_platform(None));
        assert!(is_current_platform(Some("ruby")));

        // Platform-specific tests would depend on the actual platform
        // Just ensure the function doesn't panic
        let _ = is_current_platform(Some("x86_64-linux"));
        let _ = is_current_platform(Some("arm64-darwin"));
        let _ = is_current_platform(Some("x86_64-mingw32"));
    }

    #[test]
    fn os_to_platform() {
        assert_eq!(os_to_platform_name("macos"), "darwin");
        assert_eq!(os_to_platform_name("linux"), "linux");
        assert_eq!(os_to_platform_name("windows"), "mingw");
    }
}
