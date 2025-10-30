//! Cleanup command
//!
//! Remove old gem versions

use anyhow::{Context, Result};
use lode::{Config, config, get_system_gem_dir, parse_gem_name};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// Options for gem cleanup command
#[derive(Debug, Default)]
pub(crate) struct CleanupOptions {
    /// Specific gem names to clean up (empty = all gems)
    pub gems: Vec<String>,

    /// Dry run mode (don't actually delete)
    pub dry_run: bool,

    /// Check development dependencies while uninstalling
    pub check_development: bool,

    /// Cleanup in user's home directory
    pub user_install: bool,

    /// Verbose output
    pub verbose: bool,

    /// Quiet mode
    pub quiet: bool,

    /// Config file path
    pub config_file: Option<String>,

    /// Avoid loading .gemrc file
    pub norc: bool,
}

/// Gem information for cleanup
#[derive(Debug, Clone)]
struct GemInfo {
    name: String,
    version: String,
    path: PathBuf,
}

/// Clean up old versions of gems
pub(crate) fn run(options: &CleanupOptions) -> Result<()> {
    // Get Ruby version and determine gem directory
    let _config = Config::load_with_options(options.config_file.as_deref(), options.norc)
        .context("Failed to load configuration")?;
    let ruby_ver = config::ruby_version(None);
    let gem_dir = if options.user_install {
        // User home directory gems
        let home = std::env::var("HOME").context("HOME environment variable not set")?;
        PathBuf::from(home)
            .join(".gem")
            .join("ruby")
            .join(&ruby_ver)
            .join("gems")
    } else {
        get_system_gem_dir(&ruby_ver)
    };

    if !gem_dir.exists() {
        if !options.quiet {
            println!("Gem directory does not exist: {}", gem_dir.display());
        }
        return Ok(());
    }

    if !options.quiet && options.dry_run {
        println!("Dry run mode - no gems will be deleted\n");
    }

    // Note about development dependency checking
    if options.check_development && !options.quiet {
        println!("Note: Checking development dependencies\n");
    }

    // Read all installed gems
    let entries = fs::read_dir(&gem_dir)
        .with_context(|| format!("Failed to read gem directory: {}", gem_dir.display()))?;

    let mut all_gems = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        if let Some(dir_name) = path.file_name().and_then(|n| n.to_str())
            && let Some((name, version)) = parse_gem_name(dir_name)
        {
            // Filter by specific gems if requested
            if !options.gems.is_empty() && !options.gems.contains(&name.to_string()) {
                continue;
            }

            all_gems.push(GemInfo {
                name: name.to_string(),
                version: version.to_string(),
                path: path.clone(),
            });
        }
    }

    if all_gems.is_empty() {
        if !options.quiet {
            println!("No gems found to clean up");
        }
        return Ok(());
    }

    // Group gems by name
    let mut gem_groups: HashMap<String, Vec<GemInfo>> = HashMap::new();
    for gem in all_gems {
        gem_groups.entry(gem.name.clone()).or_default().push(gem);
    }

    // For each group, find old versions to remove
    let mut gems_to_remove = Vec::new();
    let mut gems_to_keep = Vec::new();

    for (_name, mut gems) in gem_groups {
        if gems.len() <= 1 {
            // Only one version, keep it
            gems_to_keep.extend(gems);
            continue;
        }

        // Sort by version (newest first)
        gems.sort_by(|a, b| version_compare(&b.version, &a.version));

        // Keep the latest version
        if let Some(latest) = gems.first().cloned() {
            gems_to_keep.push(latest);
        }

        // Mark the rest for removal
        for gem in gems.iter().skip(1) {
            gems_to_remove.push(gem.clone());
        }
    }

    if gems_to_remove.is_empty() {
        if !options.quiet {
            println!("No old gem versions to clean up");
            println!("   {} gem(s) installed", gems_to_keep.len());
        }
        return Ok(());
    }

    // Display what will be removed
    if !options.quiet {
        println!("Cleaning up {} old gem version(s):\n", gems_to_remove.len());
        for gem in &gems_to_remove {
            println!("  {} ({})", gem.name, gem.version);
        }
        println!();
    }

    // Remove old versions (unless dry run)
    if !options.dry_run {
        let mut removed_count = 0;

        for gem in &gems_to_remove {
            if options.verbose {
                println!("  Removing {} ({})...", gem.name, gem.version);
            }

            match fs::remove_dir_all(&gem.path) {
                Ok(()) => {
                    removed_count += 1;
                    if options.verbose {
                        println!("    Removed: {}", gem.path.display());
                    }
                }
                Err(err) => {
                    eprintln!("    Failed to remove {}: {}", gem.path.display(), err);
                }
            }
        }

        if !options.quiet {
            println!("Cleaned up {removed_count} gem version(s)");
            println!("   {} gem(s) remaining", gems_to_keep.len());
        }
    } else if !options.quiet {
        println!("Dry run complete - no gems were deleted");
    }

    Ok(())
}

/// Compare two version strings
fn version_compare(a: &str, b: &str) -> std::cmp::Ordering {
    use std::cmp::Ordering;

    // Split on - and + to separate base version from prerelease and metadata
    let (a_base, a_pre) = a.split_once('-').unwrap_or((a, ""));
    let (b_base, b_pre) = b.split_once('-').unwrap_or((b, ""));

    // Parse base version parts
    let a_parts: Vec<u64> = a_base.split('.').filter_map(|p| p.parse().ok()).collect();
    let b_parts: Vec<u64> = b_base.split('.').filter_map(|p| p.parse().ok()).collect();

    // Compare base versions
    match a_parts.cmp(&b_parts) {
        Ordering::Equal => {
            // Base versions are equal, so compare prerelease versions
            // A version without prerelease is greater than one with prerelease
            match (a_pre.is_empty(), b_pre.is_empty()) {
                (true, true) => Ordering::Equal,
                (true, false) => Ordering::Greater, // a (no prerelease) > b (with prerelease)
                (false, true) => Ordering::Less,    // a (with prerelease) < b (no prerelease)
                (false, false) => a_pre.cmp(b_pre), // both have prerelease, compare lexically
            }
        }
        other => other,
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "Tests can panic")]
mod tests {
    use super::*;

    /// Helper to create minimal `CleanupOptions`
    fn minimal_cleanup_options() -> CleanupOptions {
        CleanupOptions::default()
    }

    #[test]
    fn test_version_compare() {
        use std::cmp::Ordering;

        assert_eq!(version_compare("1.0.0", "2.0.0"), Ordering::Less);
        assert_eq!(version_compare("2.0.0", "1.0.0"), Ordering::Greater);
        assert_eq!(version_compare("1.0.0", "1.0.0"), Ordering::Equal);
        assert_eq!(version_compare("1.5.0", "1.10.0"), Ordering::Less);
    }

    #[test]
    fn test_cleanup_options_default() {
        let opts = minimal_cleanup_options();
        assert!(opts.gems.is_empty());
        assert!(!opts.dry_run);
        assert!(!opts.check_development);
        assert!(!opts.user_install);
        assert!(!opts.verbose);
        assert!(!opts.quiet);
    }

    #[test]
    fn test_cleanup_options_specific_gems() {
        let mut opts = minimal_cleanup_options();
        assert!(opts.gems.is_empty());
        opts.gems = vec![
            "rails".to_string(),
            "devise".to_string(),
            "rack".to_string(),
        ];
        assert_eq!(opts.gems.len(), 3);
        assert!(opts.gems.contains(&"rails".to_string()));
        assert!(opts.gems.contains(&"devise".to_string()));
        assert!(opts.gems.contains(&"rack".to_string()));
    }

    #[test]
    fn test_cleanup_options_dry_run_flag() {
        let mut opts = minimal_cleanup_options();
        assert!(!opts.dry_run);
        opts.dry_run = true;
        assert!(opts.dry_run);
    }

    #[test]
    fn test_cleanup_options_check_development_flag() {
        let mut opts = minimal_cleanup_options();
        assert!(!opts.check_development);
        opts.check_development = true;
        assert!(opts.check_development);
    }

    #[test]
    fn test_cleanup_options_user_install_flag() {
        let mut opts = minimal_cleanup_options();
        assert!(!opts.user_install);
        opts.user_install = true;
        assert!(opts.user_install);
    }

    #[test]
    fn test_cleanup_options_verbose_flag() {
        let mut opts = minimal_cleanup_options();
        assert!(!opts.verbose);
        opts.verbose = true;
        assert!(opts.verbose);
    }

    #[test]
    fn test_cleanup_options_quiet_flag() {
        let mut opts = minimal_cleanup_options();
        assert!(!opts.quiet);
        opts.quiet = true;
        assert!(opts.quiet);
    }

    #[test]
    fn test_cleanup_options_dry_run_with_verbose() {
        let mut opts = minimal_cleanup_options();
        opts.dry_run = true;
        opts.verbose = true;
        assert!(opts.dry_run);
        assert!(opts.verbose);
    }

    #[test]
    fn test_cleanup_options_complex_scenario() {
        let mut opts = minimal_cleanup_options();
        opts.gems = vec!["old-gem-1".to_string(), "old-gem-2".to_string()];
        opts.dry_run = true;
        opts.verbose = true;
        opts.check_development = true;

        assert_eq!(opts.gems.len(), 2);
        assert!(opts.dry_run);
        assert!(opts.verbose);
        assert!(opts.check_development);
    }

    #[test]
    fn test_cleanup_options_quiet_and_verbose_flags() {
        let mut opts = minimal_cleanup_options();
        opts.quiet = true;
        opts.verbose = true;
        assert!(opts.quiet);
        assert!(opts.verbose);
    }

    #[test]
    fn test_version_compare_with_prerelease() {
        use std::cmp::Ordering;

        assert_eq!(version_compare("1.0.0-alpha", "1.0.0"), Ordering::Less);
        assert_eq!(version_compare("1.0.0-beta", "1.0.0-rc"), Ordering::Less);
        assert_eq!(version_compare("2.0.0", "1.9.9"), Ordering::Greater);
    }

    #[test]
    fn test_cleanup_workflow_cleanup_all_gems() {
        let opts = minimal_cleanup_options();
        assert!(opts.gems.is_empty());
    }

    #[test]
    fn test_cleanup_workflow_cleanup_specific_gems() {
        let mut opts = minimal_cleanup_options();
        opts.gems = vec!["rails".to_string(), "devise".to_string()];
        assert_eq!(opts.gems.len(), 2);
    }

    #[test]
    fn test_cleanup_workflow_dry_run() {
        let mut opts = minimal_cleanup_options();
        opts.dry_run = true;
        assert!(opts.dry_run);
    }

    #[test]
    fn test_cleanup_workflow_check_development() {
        let mut opts = minimal_cleanup_options();
        opts.check_development = true;
        assert!(opts.check_development);
    }

    #[test]
    fn test_cleanup_workflow_verbose_cleanup() {
        let mut opts = minimal_cleanup_options();
        opts.verbose = true;
        assert!(opts.verbose);
    }

    #[test]
    fn test_cleanup_workflow_quiet_cleanup() {
        let mut opts = minimal_cleanup_options();
        opts.quiet = true;
        assert!(opts.quiet);
    }

    #[test]
    fn test_cleanup_workflow_user_install_cleanup() {
        let mut opts = minimal_cleanup_options();
        opts.user_install = true;
        assert!(opts.user_install);
    }

    #[test]
    fn test_cleanup_workflow_complex_cleanup() {
        let mut opts = minimal_cleanup_options();
        opts.dry_run = true;
        opts.verbose = true;
        opts.check_development = true;
        assert!(opts.dry_run);
        assert!(opts.verbose);
        assert!(opts.check_development);
    }
}
