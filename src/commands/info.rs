//! Info command
//!
//! Show gem information

use anyhow::{Context, Result};
use lode::{Config, RubyGemsClient, config, lockfile::Lockfile};
use std::fs;

/// Show detailed information about a gem from RubyGems.org or its installation path
pub(crate) async fn run(gem_name: &str, show_path: bool, show_version: bool) -> Result<()> {
    // If --path flag is used, show the installation path
    if show_path {
        return show_gem_path(gem_name);
    }

    // If --version flag is used, show just the version
    if show_version {
        return show_gem_version(gem_name);
    }

    // Create RubyGems client
    let client = RubyGemsClient::new(lode::DEFAULT_GEM_SOURCE)?;

    // Fetch all versions to get the latest
    let versions = client
        .fetch_versions(gem_name)
        .await
        .with_context(|| format!("Failed to fetch versions for gem: {gem_name}"))?;

    if versions.is_empty() {
        anyhow::bail!("No versions found for gem: {gem_name}");
    }

    // Get the latest version (versions are returned in descending order)
    let latest = versions
        .first()
        .expect("versions should not be empty after check");

    // Display gem information
    println!("*** {} ({})", gem_name, latest.number);
    println!();

    println!("Platform: {}", latest.platform);

    if let Some(ruby_version) = &latest.ruby_version {
        println!("Required Ruby Version: {ruby_version}");
    }

    // Show dependencies
    let runtime_deps = &latest.dependencies.runtime;
    let dev_deps = &latest.dependencies.development;

    if !runtime_deps.is_empty() {
        println!();
        println!("Runtime Dependencies:");
        for dep in runtime_deps {
            let req = if dep.requirements.is_empty() {
                ">= 0"
            } else {
                &dep.requirements
            };
            println!("  {} ({})", dep.name, req);
        }
    }

    if !dev_deps.is_empty() {
        println!();
        println!("Development Dependencies:");
        for dep in dev_deps {
            let req = if dep.requirements.is_empty() {
                ">= 0"
            } else {
                &dep.requirements
            };
            println!("  {} ({})", dep.name, req);
        }
    }

    // Show additional available versions
    if versions.len() > 1 {
        println!();
        println!("Other versions available:");
        let display_count = versions.len().min(10);
        for version in versions.iter().skip(1).take(display_count - 1) {
            let number = &version.number;
            println!("  {number}");
        }
        let total = versions.len();
        if total > 10 {
            let more = total - 10;
            println!("  ... and {more} more versions");
        }
    }

    Ok(())
}

/// Show just the version of a gem from the lockfile
fn show_gem_version(gem_name: &str) -> Result<()> {
    // Find and read lockfile
    let lockfile_path = lode::paths::find_lockfile();
    let content = fs::read_to_string(&lockfile_path)
        .with_context(|| format!("Failed to read lockfile: {}", lockfile_path.display()))?;

    let lockfile = Lockfile::parse(&content)
        .with_context(|| format!("Failed to parse lockfile: {}", lockfile_path.display()))?;

    // Find the gem in the lockfile
    let gem = lockfile
        .gems
        .iter()
        .find(|g| g.name == gem_name)
        .with_context(|| format!("Gem '{gem_name}' not found in lockfile"))?;

    // Print just the version
    println!("{}", gem.version);

    Ok(())
}

/// Show the installation path of a gem
fn show_gem_path(gem_name: &str) -> Result<()> {
    // Find and read lockfile
    let lockfile_path = lode::paths::find_lockfile();
    let content = fs::read_to_string(&lockfile_path)
        .with_context(|| format!("Failed to read lockfile: {}", lockfile_path.display()))?;

    let lockfile = Lockfile::parse(&content)
        .with_context(|| format!("Failed to parse lockfile: {}", lockfile_path.display()))?;

    // Find the gem in the lockfile
    let gem = lockfile
        .gems
        .iter()
        .find(|g| g.name == gem_name)
        .with_context(|| format!("Gem '{gem_name}' not found in lockfile"))?;

    // Get vendor directory and Ruby version
    let cfg = Config::load().unwrap_or_default();
    let vendor_dir = config::vendor_dir(Some(&cfg))?;

    // Determine Ruby version from lockfile or detect from active Ruby
    let ruby_version = config::ruby_version(lockfile.ruby_version.as_deref());

    // Build the path to the gem
    let gem_dir = vendor_dir
        .join("ruby")
        .join(&ruby_version)
        .join("gems")
        .join(format!("{}-{}", gem.name, gem.version));

    // Print the path
    println!("{}", gem_dir.display());

    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "Tests can panic")]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore = "Requires network access to rubygems.org"]
    async fn test_info_rack() {
        let result = run("rack", false, false).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_info_nonexistent() {
        let result = run("this-gem-definitely-does-not-exist-12345", false, false).await;
        assert!(result.is_err());
    }
}
