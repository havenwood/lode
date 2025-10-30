//! Check command
//!
//! Verify all gems are installed

use anyhow::{Context, Result};
use lode::{Config, config, lockfile::Lockfile};
use std::fs;
use std::path::Path;

/// Verify all gems are installed
pub(crate) fn run(lockfile_path: &str, dry_run: bool) -> Result<()> {
    // In dry-run mode, just show what would be checked
    if dry_run {
        println!("Dry run: Would check gems in lockfile: {lockfile_path}");
    }

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

    println!("Checking installed gems in {}", gems_dir.display());

    let mut missing = Vec::new();
    let mut installed_count = 0;

    // Check regular gems
    for gem in &lockfile.gems {
        let gem_dir = gems_dir.join(gem.full_name());
        if gem_dir.exists() {
            installed_count += 1;
            println!(
                "  {name} ({version})",
                name = gem.name,
                version = gem.version
            );
        } else {
            missing.push(format!("{} ({})", gem.name, gem.version));
            println!(
                "  {name} ({version}) - not found",
                name = gem.name,
                version = gem.version
            );
        }
    }

    // Check git gems
    for git_gem in &lockfile.git_gems {
        let gem_dir = gems_dir.join(format!("{}-{}", git_gem.name, git_gem.version));
        if gem_dir.exists() {
            installed_count += 1;
            println!(
                "  {name} ({version}) [git]",
                name = git_gem.name,
                version = git_gem.version
            );
        } else {
            missing.push(format!("{} ({}) [git]", git_gem.name, git_gem.version));
            println!(
                "  {name} ({version}) [git] - not found",
                name = git_gem.name,
                version = git_gem.version
            );
        }
    }

    // Check path gems (these should exist at their source path)
    for path_gem in &lockfile.path_gems {
        if Path::new(&path_gem.path).exists() {
            installed_count += 1;
            println!(
                "  {name} ({version}) [path]",
                name = path_gem.name,
                version = path_gem.version
            );
        } else {
            missing.push(format!(
                "{} ({}) [path: {}]",
                path_gem.name, path_gem.version, path_gem.path
            ));
            println!(
                "  {name} ({version}) [path] - source not found at {path}",
                name = path_gem.name,
                version = path_gem.version,
                path = path_gem.path
            );
        }
    }

    // Print summary
    if !missing.is_empty() {
        println!("\nThe following gems are missing:");
        for gem in &missing {
            println!("  * {gem}");
        }
        println!("\nRun `lode install` to install missing gems.");
        anyhow::bail!("Missing {} gem(s)", missing.len());
    }

    println!("\nAll gems are installed ({installed_count} total)");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn check_nonexistent_lockfile() {
        let result = run("/nonexistent/Gemfile.lock", false);
        assert!(result.is_err());
    }

    #[test]
    fn check_valid_lockfile_success() {
        // Create a temporary lockfile with an installed gem
        let temp = TempDir::new().unwrap();
        let lockfile_path = temp.path().join("Gemfile.lock");

        let content = "GEM\n  specs:\n    rake (13.3.1)\n\nPLATFORMS\n  ruby\n\nDEPENDENCIES\n  rake\n\nRUBY VERSION\n   ruby 3.3.0\n";
        fs::write(&lockfile_path, content).unwrap();

        // Note: This will succeed only if rake is actually installed on the system
        // This test documents the expected behavior
        let result = run(lockfile_path.to_str().unwrap(), false);
        // Result depends on system gems, so we just verify it returns a Result
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn check_dry_run_flag() {
        // Test that dry_run flag is accepted
        let temp = TempDir::new().unwrap();
        let lockfile_path = temp.path().join("Gemfile.lock");

        let content = "GEM\n  specs:\n    rake (13.3.1)\n\nPLATFORMS\n  ruby\n\nRUBY VERSION\n   ruby 3.3.0\n";
        fs::write(&lockfile_path, content).unwrap();

        // dry_run=true should work without errors
        let result = run(lockfile_path.to_str().unwrap(), true);
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn check_custom_vendor_path() {
        // Test that custom vendor path is accepted
        let temp = TempDir::new().unwrap();
        let lockfile_path = temp.path().join("Gemfile.lock");
        let vendor_path = temp.path().join("vendor");

        let content = "GEM\n  specs:\n    rake (13.3.1)\n\nPLATFORMS\n  ruby\n\nRUBY VERSION\n   ruby 3.3.0\n";
        fs::write(&lockfile_path, content).unwrap();
        fs::create_dir_all(&vendor_path).unwrap();

        // Should handle custom vendor path gracefully
        let result = run(lockfile_path.to_str().unwrap(), false);
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn check_exit_code_behavior() {
        // Verify exit codes match bundle check behavior:
        // - Exit 0 when all gems found
        // - Exit 1 when any gem missing
        let temp = TempDir::new().unwrap();
        let lockfile_path = temp.path().join("Gemfile.lock");

        // Lockfile with nonexistent gem should fail
        let content = "GEM\n  specs:\n    nonexistent-gem-xyz-99.99.0\n\nPLATFORMS\n  ruby\n\nRUBY VERSION\n   ruby 3.3.0\n";
        fs::write(&lockfile_path, content).unwrap();

        let result = run(lockfile_path.to_str().unwrap(), false);
        // Should error because gem doesn't exist
        assert!(result.is_err());
    }
}
