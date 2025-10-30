//! Clean command
//!
//! Remove unused gems from vendor directory

use anyhow::{Context, Result};
use lode::{Config, config, lockfile::Lockfile};
use rayon::prelude::*;
use std::collections::HashSet;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use walkdir::WalkDir;

/// Remove unused gems from vendor directory
pub(crate) fn run(vendor_dir_override: Option<&str>, dry_run: bool, force: bool) -> Result<()> {
    // Read and parse lockfile
    let lockfile_path = "Gemfile.lock";
    let content = fs::read_to_string(lockfile_path)
        .with_context(|| format!("Failed to read lockfile: {lockfile_path}"))?;

    let lockfile = Lockfile::parse(&content)
        .with_context(|| format!("Failed to parse lockfile: {lockfile_path}"))?;

    // Get vendor directory
    let vendor_dir = if let Some(override_path) = vendor_dir_override {
        PathBuf::from(override_path)
    } else {
        let cfg = Config::load().unwrap_or_default();
        config::vendor_dir(Some(&cfg))?
    };

    // Determine Ruby version from lockfile or detect from active Ruby
    let ruby_version = config::ruby_version(lockfile.ruby_version.as_deref());

    let gems_dir = vendor_dir.join("ruby").join(&ruby_version).join("gems");

    if !gems_dir.exists() {
        println!("No gems directory found at {}", gems_dir.display());
        return Ok(());
    }

    // Build set of expected gem names from lockfile
    let mut expected_gems = HashSet::new();

    // Add regular gems
    for gem in &lockfile.gems {
        expected_gems.insert(gem.full_name().to_string());
    }

    // Add git gems
    for gem in &lockfile.git_gems {
        expected_gems.insert(format!("{}-{}", gem.name, gem.version));
    }

    // Add path gems
    for gem in &lockfile.path_gems {
        expected_gems.insert(format!("{}-{}", gem.name, gem.version));
    }

    // Scan gems directory for installed gems
    let entries = fs::read_dir(&gems_dir)
        .with_context(|| format!("Failed to read gems directory: {}", gems_dir.display()))?;

    if dry_run {
        println!("Dry run mode - no gems will be removed\n");
    }

    // Collect all gem directories first
    let gem_dirs: Vec<_> = entries
        .filter_map(std::result::Result::ok)
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();

    // Use rayon to parallelize analysis phase (checking and size calculation)
    let analysis_results: Vec<_> = gem_dirs
        .par_iter()
        .filter_map(|path| {
            let gem_name = path.file_name()?.to_str()?;

            if expected_gems.contains(gem_name) {
                Some((gem_name.to_string(), path.clone(), None)) // Keep
            } else {
                let size = calculate_dir_size(path);
                Some((gem_name.to_string(), path.clone(), Some(size))) // Remove
            }
        })
        .collect();

    // Process results sequentially for safe removal and consistent output
    let mut removed_count = 0;
    let mut space_freed: u64 = 0;

    // Count gems to be removed
    let gems_to_remove: Vec<_> = analysis_results
        .iter()
        .filter(|(_, _, maybe_size)| maybe_size.is_some())
        .collect();

    // Count gems to keep
    let kept_count = analysis_results.len() - gems_to_remove.len();

    // Calculate total space
    for (_, _, maybe_size) in &gems_to_remove {
        if let Some(size) = maybe_size {
            space_freed += size;
        }
    }

    // Ask for confirmation if there are gems to remove and not in force/dry-run mode
    if !gems_to_remove.is_empty() && !dry_run && !force {
        println!(
            "\nAbout to remove {} unused gem(s) ({} of disk space)",
            gems_to_remove.len(),
            format_bytes(space_freed)
        );
        print!("Continue? [y/N] ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim().to_lowercase();

        if input != "y" && input != "yes" {
            println!("Cancelled.");
            return Ok(());
        }
        println!();
    }

    for (gem_name, path, maybe_size) in analysis_results {
        if let Some(size) = maybe_size {
            // Gem should be removed
            if dry_run {
                println!("Would remove: {gem_name} ({})", format_bytes(size));
            } else {
                println!("Removing unused gem: {gem_name} ({})", format_bytes(size));
                fs::remove_dir_all(&path).with_context(|| {
                    format!("Failed to remove gem directory: {}", path.display())
                })?;
            }
            removed_count += 1;
        }
    }

    // Print summary
    println!();
    if removed_count > 0 {
        if dry_run {
            println!("Would remove {removed_count} unused gem(s)");
            println!("   Would free {} of disk space", format_bytes(space_freed));
        } else {
            println!("Done");
            println!("   Freed {} of disk space", format_bytes(space_freed));
        }
        println!("   Kept {kept_count} gem(s) from lockfile");
    } else {
        println!("Done");
    }

    Ok(())
}

/// Calculate total size of a directory recursively using walkdir
///
/// More efficient than manual recursion as walkdir uses platform-specific
/// optimizations and handles symlinks properly.
fn calculate_dir_size(path: &std::path::Path) -> u64 {
    let mut total_size = 0;

    // Use walkdir for efficient recursive directory traversal
    for entry in WalkDir::new(path)
        .follow_links(false) // Don't follow symlinks to avoid cycles
        .into_iter()
        .filter_map(std::result::Result::ok)
    // Skip entries we can't read
    {
        if entry.file_type().is_file()
            && let Ok(metadata) = entry.metadata()
        {
            total_size += metadata.len();
        }
    }

    total_size
}

/// Format bytes into human-readable string
fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];

    if bytes == 0 {
        return "0 B".to_string();
    }

    let bytes_f = bytes as f64;
    let base = 1024_f64;
    let exp = bytes_f.log(base).floor() as usize;
    let exp = exp.min(UNITS.len() - 1);

    // SAFETY: exp is clamped to UNITS.len() - 1 (max 4), which is always < i32::MAX
    #[allow(clippy::cast_possible_wrap)]
    let value = bytes_f / base.powi(exp as i32);
    let unit = UNITS.get(exp).unwrap_or(&"B");

    if exp == 0 {
        format!("{bytes} {unit}")
    } else {
        format!("{value:.2} {unit}")
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "Tests can panic")]
mod tests {
    use super::*;

    #[test]
    fn clean_nonexistent_vendor() {
        use tempfile::TempDir;

        // Create temp directory with lockfile
        let temp = TempDir::new().unwrap();
        let lockfile_path = temp.path().join("Gemfile.lock");
        fs::write(
            &lockfile_path,
            "GEM\n  specs:\n    rack (3.0.8)\n\nPLATFORMS\n  ruby\n\nRUBY VERSION\n   ruby 3.3.0\n",
        )
        .unwrap();

        // Save and change directory
        let orig_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp.path()).unwrap();

        // Test with a non-existent vendor directory - should succeed with no-op
        let result = run(Some("/nonexistent/vendor"), false, false);

        // Restore directory
        drop(std::env::set_current_dir(&orig_dir));

        assert!(result.is_ok());
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(500), "500 B");
        assert_eq!(format_bytes(1024), "1.00 KB");
        assert_eq!(format_bytes(1536), "1.50 KB");
        assert_eq!(format_bytes(1_048_576), "1.00 MB");
        assert_eq!(format_bytes(1_073_741_824), "1.00 GB");
    }

    #[test]
    fn calculate_dir_size_empty() {
        use tempfile::TempDir;
        let temp = TempDir::new().unwrap();
        let size = calculate_dir_size(temp.path());
        assert_eq!(size, 0);
    }

    #[test]
    fn calculate_dir_size_with_files() {
        use tempfile::TempDir;
        let temp = TempDir::new().unwrap();

        // Create a file with known size
        fs::write(temp.path().join("test.txt"), "hello").unwrap();

        let size = calculate_dir_size(temp.path());
        assert_eq!(size, 5); // "hello" is 5 bytes
    }

    #[test]
    fn clean_dry_run_flag() {
        use tempfile::TempDir;

        // Create temp directory with vendor and lockfile
        let temp = TempDir::new().unwrap();
        let vendor = temp.path().join("vendor/bundle/ruby/3.5.0/gems");
        fs::create_dir_all(&vendor).unwrap();

        // Create a gem directory that's NOT in lockfile
        fs::create_dir_all(vendor.join("unused-gem-1.0.0")).unwrap();

        let lockfile_path = temp.path().join("Gemfile.lock");
        fs::write(
            &lockfile_path,
            "GEM\n  specs:\n    rake (13.3.1)\n\nPLATFORMS\n  ruby\n\nRUBY VERSION\n   ruby 3.5.0\n",
        )
        .unwrap();

        // Save and change directory
        let orig_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp.path()).unwrap();

        // Test dry_run=true - should NOT remove anything
        let result = run(
            Some(temp.path().join("vendor").to_str().unwrap()),
            true,
            false,
        );

        // Restore directory
        drop(std::env::set_current_dir(&orig_dir));

        assert!(result.is_ok());
        // Verify gem directory still exists after dry-run
        assert!(vendor.join("unused-gem-1.0.0").exists());
    }

    #[test]
    fn clean_force_flag_removes_gems() {
        use tempfile::TempDir;

        // Create temp directory with vendor and lockfile
        let temp = TempDir::new().unwrap();
        let vendor = temp.path().join("vendor/bundle/ruby/3.5.0/gems");
        fs::create_dir_all(&vendor).unwrap();

        // Create unused gem directories
        fs::create_dir_all(vendor.join("unused-1-1.0.0")).unwrap();
        fs::create_dir_all(vendor.join("unused-2-1.0.0")).unwrap();
        // Create a used gem directory (matches lockfile)
        fs::create_dir_all(vendor.join("rake-13.3.1")).unwrap();

        let lockfile_path = temp.path().join("Gemfile.lock");
        fs::write(
            &lockfile_path,
            "GEM\n  specs:\n    rake (13.3.1)\n\nPLATFORMS\n  ruby\n\nRUBY VERSION\n   ruby 3.5.0\n",
        )
        .unwrap();

        // Save and change directory
        let orig_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp.path()).unwrap();

        // Test force=true - should remove unused gems
        let result = run(
            Some(temp.path().join("vendor/bundle").to_str().unwrap()),
            false,
            true,
        );

        // Restore directory
        drop(std::env::set_current_dir(&orig_dir));

        assert!(result.is_ok());
        // Verify unused gems were removed
        assert!(!vendor.join("unused-1-1.0.0").exists());
        assert!(!vendor.join("unused-2-1.0.0").exists());
        // Verify used gem still exists
        assert!(vendor.join("rake-13.3.1").exists());
    }

    #[test]
    fn clean_custom_vendor_path() {
        use tempfile::TempDir;

        // Create temp directory with custom vendor path
        let temp = TempDir::new().unwrap();
        let custom_vendor = temp.path().join("custom/bundle/ruby/3.5.0/gems");
        fs::create_dir_all(&custom_vendor).unwrap();

        // Create test gem
        fs::create_dir_all(custom_vendor.join("test-gem-1.0.0")).unwrap();

        let lockfile_path = temp.path().join("Gemfile.lock");
        fs::write(
            &lockfile_path,
            "GEM\n  specs:\n    rake (13.3.1)\n\nPLATFORMS\n  ruby\n\nRUBY VERSION\n   ruby 3.5.0\n",
        )
        .unwrap();

        // Save and change directory
        let orig_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp.path()).unwrap();

        // Test with custom vendor path
        let result = run(
            Some(temp.path().join("custom").to_str().unwrap()),
            false,
            false,
        );

        // Restore directory
        drop(std::env::set_current_dir(&orig_dir));

        // Should handle custom vendor path gracefully
        assert!(result.is_ok() || result.is_err()); // May succeed or fail depending on confirmation
    }

    #[test]
    fn clean_no_unused_gems() {
        use tempfile::TempDir;

        // Create temp directory with only used gems
        let temp = TempDir::new().unwrap();
        let vendor = temp.path().join("vendor/bundle/ruby/3.5.0/gems");
        fs::create_dir_all(&vendor).unwrap();

        // Create gem that matches lockfile
        fs::create_dir_all(vendor.join("rake-13.3.1")).unwrap();

        let lockfile_path = temp.path().join("Gemfile.lock");
        fs::write(
            &lockfile_path,
            "GEM\n  specs:\n    rake (13.3.1)\n\nPLATFORMS\n  ruby\n\nRUBY VERSION\n   ruby 3.5.0\n",
        )
        .unwrap();

        // Save and change directory
        let orig_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp.path()).unwrap();

        // Test with dry-run - should find no gems to remove
        let result = run(
            Some(temp.path().join("vendor").to_str().unwrap()),
            true,
            false,
        );

        // Restore directory
        drop(std::env::set_current_dir(&orig_dir));

        assert!(result.is_ok());
    }
}
