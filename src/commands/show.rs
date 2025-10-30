//! Show command
//!
//! Show installed gems and their locations

use anyhow::{Context, Result};
use lode::{Config, config, lockfile::Lockfile};
use std::fs;

/// Show the source location of a gem
pub(crate) fn run(gem_name: Option<&str>, paths: bool, lockfile_path: &str) -> Result<()> {
    // Read and parse lockfile
    let content = fs::read_to_string(lockfile_path)
        .with_context(|| format!("Failed to read lockfile: {lockfile_path}"))?;

    let lockfile = Lockfile::parse(&content)
        .with_context(|| format!("Failed to parse lockfile: {lockfile_path}"))?;

    // Get vendor directory
    let cfg = Config::load().unwrap_or_default();
    let vendor_dir = config::vendor_dir(Some(&cfg))?;

    // Determine Ruby version from lockfile or detect from active Ruby
    let ruby_version = config::ruby_version(lockfile.ruby_version.as_deref());

    let gems_dir = vendor_dir.join("ruby").join(&ruby_version).join("gems");

    // If --paths flag is set, list all gem paths (sorted by name)
    if paths {
        let mut all_gems = Vec::new();

        // Collect all gems with their paths
        for gem in &lockfile.gems {
            let gem_dir = gems_dir.join(gem.full_name());
            if gem_dir.exists() {
                all_gems.push((gem.name.clone(), gem_dir));
            }
        }
        for gem in &lockfile.git_gems {
            let gem_dir = gems_dir.join(format!("{}-{}", gem.name, gem.version));
            if gem_dir.exists() {
                all_gems.push((gem.name.clone(), gem_dir));
            }
        }
        for gem in &lockfile.path_gems {
            let gem_dir = gems_dir.join(format!("{}-{}", gem.name, gem.version));
            if gem_dir.exists() {
                all_gems.push((gem.name.clone(), gem_dir));
            }
        }

        // Sort by gem name and print
        all_gems.sort_by(|a, b| a.0.cmp(&b.0));
        for (_name, gem_dir) in all_gems {
            println!("{}", gem_dir.display());
        }
        return Ok(());
    }

    // If no gem specified, list all gems with versions (default behavior)
    let Some(gem_name) = gem_name else {
        let mut all_gems = Vec::new();

        // Collect all gems with their versions
        for gem in &lockfile.gems {
            all_gems.push((gem.name.clone(), gem.version.clone()));
        }
        for gem in &lockfile.git_gems {
            all_gems.push((gem.name.clone(), gem.version.clone()));
        }
        for gem in &lockfile.path_gems {
            all_gems.push((gem.name.clone(), gem.version.clone()));
        }

        // Sort by gem name and print
        all_gems.sort_by(|a, b| a.0.cmp(&b.0));
        for (name, version) in all_gems {
            println!("{name} ({version})");
        }
        return Ok(());
    };

    // Find the gem in the lockfile
    // Check regular gems
    if let Some(gem) = lockfile.gems.iter().find(|gem| gem.name == gem_name) {
        let gem_dir = gems_dir.join(gem.full_name());
        if gem_dir.exists() {
            println!("{}", gem_dir.display());
            return Ok(());
        }
        anyhow::bail!(
            "Gem {} ({}) is in the lockfile but not installed at {}",
            gem.name,
            gem.version,
            gem_dir.display()
        );
    }

    // Check git gems
    if let Some(gem) = lockfile.git_gems.iter().find(|gem| gem.name == gem_name) {
        let gem_dir = gems_dir.join(format!("{}-{}", gem.name, gem.version));
        if gem_dir.exists() {
            println!("{}", gem_dir.display());
            return Ok(());
        }
        anyhow::bail!(
            "Gem {} ({}) [git] is in the lockfile but not installed at {}",
            gem.name,
            gem.version,
            gem_dir.display()
        );
    }

    // Check path gems
    if let Some(gem) = lockfile.path_gems.iter().find(|gem| gem.name == gem_name) {
        let gem_dir = gems_dir.join(format!("{}-{}", gem.name, gem.version));
        if gem_dir.exists() {
            println!("{}", gem_dir.display());
            return Ok(());
        }
        anyhow::bail!(
            "Gem {} ({}) [path] is in the lockfile but not installed at {}",
            gem.name,
            gem.version,
            gem_dir.display()
        );
    }

    // Gem not found in any collection
    anyhow::bail!(
        "Gem '{}' not found in lockfile. Available gems:\n{}",
        gem_name,
        lockfile
            .gems
            .iter()
            .map(|g| format!("  - {}", g.name))
            .collect::<Vec<_>>()
            .join("\n")
    );
}
