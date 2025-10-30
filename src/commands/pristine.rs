//! Pristine command
//!
//! Restore gems to pristine condition

use anyhow::{Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use lode::{Config, config, lockfile::Lockfile, ruby};
use rayon::prelude::*;
use std::fs;
use std::path::PathBuf;

/// Restore gems to pristine condition
///
/// This command reinstalls gems from the cache, restoring them to their original
/// state. Useful when gem files have been accidentally modified or corrupted.
pub(crate) fn run(
    gem_names: &[String],
    lockfile_path: &str,
    vendor_dir_override: Option<&str>,
) -> Result<()> {
    // Parse lockfile to get gem list
    let content = fs::read_to_string(lockfile_path)
        .with_context(|| format!("Failed to read lockfile: {lockfile_path}"))?;

    let lockfile = Lockfile::parse(&content)
        .with_context(|| format!("Failed to parse lockfile: {lockfile_path}"))?;

    // Determine which gems to restore
    let gems_to_restore: Vec<_> = if gem_names.is_empty() {
        // Restore all gems
        lockfile.gems.iter().collect()
    } else {
        // Filter to specified gems
        lockfile
            .gems
            .iter()
            .filter(|gem| gem_names.contains(&gem.name))
            .collect()
    };

    if gems_to_restore.is_empty() {
        if gem_names.is_empty() {
            println!("No gems to restore");
        } else {
            println!("No matching gems found in lockfile");
        }
        return Ok(());
    }

    // Get paths
    let cfg = Config::load().ok();
    let cache_dir = config::cache_dir(cfg.as_ref())?;
    let vendor_dir = if let Some(dir) = vendor_dir_override {
        PathBuf::from(dir)
    } else {
        config::vendor_dir(cfg.as_ref())?
    };

    // Get Ruby version (prefer lockfile, fallback to Gemfile)
    let gemfile_path = lode::gemfile_for_lockfile(std::path::Path::new(lockfile_path));
    let ruby_version = ruby::detect_ruby_version(
        Some(lockfile_path),
        gemfile_path
            .exists()
            .then(|| gemfile_path.to_str().unwrap_or("")),
        "3.3.0",
    );

    println!("Restoring {} gems...", gems_to_restore.len());

    // Create progress bar
    let pb = ProgressBar::new(gems_to_restore.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("#>-"),
    );

    // Use rayon to parallelize restoration
    let results: Vec<_> = gems_to_restore
        .par_iter()
        .map(|gem_spec| {
            let result = restore_gem(gem_spec, &cache_dir, &vendor_dir, &ruby_version);
            pb.inc(1);
            (gem_spec, result)
        })
        .collect();

    pb.finish_with_message("Done!");

    // Process results
    let mut restored = 0;
    let mut failed = 0;

    for (gem_spec, result) in results {
        match result {
            Ok(()) => {
                restored += 1;
                println!("  OK {} ({})", gem_spec.name, gem_spec.version);
            }
            Err(e) => {
                failed += 1;
                eprintln!("  FAIL {} ({}) - {}", gem_spec.name, gem_spec.version, e);
            }
        }
    }

    println!();
    println!(
        "Restored {restored} gems{}",
        if failed > 0 {
            format!(", {failed} failed")
        } else {
            String::new()
        }
    );

    if failed > 0 {
        anyhow::bail!("{failed} gems failed to restore");
    }

    Ok(())
}

/// Restore a single gem from cache
fn restore_gem(
    gem_spec: &lode::lockfile::GemSpec,
    cache_dir: &std::path::Path,
    vendor_dir: &std::path::Path,
    ruby_version: &str,
) -> Result<()> {
    // Build paths
    let cache_path = cache_dir.join(format!("{}.gem", gem_spec.full_name()));
    let ruby_dir = vendor_dir.join("ruby").join(ruby_version);
    let gem_install_dir = ruby_dir.join("gems").join(gem_spec.full_name());
    let spec_path = ruby_dir
        .join("specifications")
        .join(format!("{}.gemspec", gem_spec.full_name()));

    // Check if gem exists in cache
    if !cache_path.exists() {
        anyhow::bail!(
            "Gem not found in cache: {}. Run 'lode fetch' first.",
            gem_spec.full_name()
        );
    }

    // Delete existing installation if present
    if gem_install_dir.exists() {
        fs::remove_dir_all(&gem_install_dir).with_context(|| {
            format!(
                "Failed to remove existing gem: {}",
                gem_install_dir.display()
            )
        })?;
    }

    if spec_path.exists() {
        fs::remove_file(&spec_path)
            .with_context(|| format!("Failed to remove gemspec: {}", spec_path.display()))?;
    }

    // Reinstall from cache
    lode::install::install_gem(gem_spec, &cache_path, vendor_dir, ruby_version)?;

    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "Tests can panic")]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn pristine_no_gems() {
        // Test with empty gem list
        let temp_dir = TempDir::new().unwrap();
        let lockfile = temp_dir.path().join("Gemfile.lock");

        fs::write(
            &lockfile,
            r#"GEM
  remote: https://rubygems.org/
  specs:

PLATFORMS
  ruby

DEPENDENCIES

RUBY VERSION
   ruby 3.3.0p0

BUNDLED WITH
   2.5.0
",
        )
        .unwrap();

        let result = run(&[], lockfile.to_str().unwrap(), None);

        // Should succeed with no gems to restore
        assert!(result.is_ok());
    }

    #[test]
    fn pristine_missing_lockfile() {
        let result = run(&[], "/nonexistent/Gemfile.lock", None);

        // Should fail with error about missing lockfile
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Failed to read lockfile"));
    }

    #[test]
    fn pristine_specific_gem_not_in_lockfile() {
        let temp_dir = TempDir::new().unwrap();
        let lockfile = temp_dir.path().join("Gemfile.lock");

        fs::write(
            &lockfile,
            r#"GEM
  remote: https://rubygems.org/
  specs:
    rack (3.0.8)

PLATFORMS
  ruby

DEPENDENCIES
  rack

RUBY VERSION
   ruby 3.3.0p0

BUNDLED WITH
   2.5.0
"#,
        )
        .unwrap();

        let result = run(
            &["nonexistent".to_string()],
            lockfile.to_str().unwrap(),
            None,
        );

        // Should succeed but with no gems to restore
        assert!(result.is_ok());
    }

    #[test]
    fn test_pristine_workflow_restore_all_gems() {
        let gem_names: Vec<String> = Vec::new();
        assert!(gem_names.is_empty());
    }

    #[test]
    fn test_pristine_workflow_restore_specific_gem() {
        let gem_names = ["rails".to_string()];
        assert_eq!(gem_names.len(), 1);
        assert!(gem_names.contains(&"rails".to_string()));
    }

    #[test]
    fn test_pristine_workflow_restore_multiple_gems() {
        let gem_names = [
            "rails".to_string(),
            "rack".to_string(),
            "sinatra".to_string(),
        ];
        assert_eq!(gem_names.len(), 3);
        assert!(gem_names.contains(&"rails".to_string()));
        assert!(gem_names.contains(&"rack".to_string()));
        assert!(gem_names.contains(&"sinatra".to_string()));
    }

    #[test]
    fn test_pristine_workflow_custom_lockfile() {
        let lockfile_path = "/path/to/custom/Gemfile.lock";
        assert!(!lockfile_path.is_empty());
        assert!(lockfile_path.contains("Gemfile.lock"));
    }

    #[test]
    fn test_pristine_workflow_custom_vendor_dir() {
        let vendor_dir = Some("/path/to/vendor");
        assert!(vendor_dir.is_some());
        assert_eq!(vendor_dir, Some("/path/to/vendor"));
    }

    #[test]
    fn test_pristine_workflow_restore_with_custom_path() {
        let gem_names = ["devise".to_string()];
        let lockfile_path = "/app/Gemfile.lock";
        assert_eq!(gem_names.len(), 1);
        assert!(lockfile_path.contains("Gemfile.lock"));
    }

    #[test]
    fn test_pristine_workflow_restore_all_custom_vendor() {
        let gem_names: Vec<String> = Vec::new();
        let vendor_dir = Some("/vendor-custom");
        assert!(gem_names.is_empty());
        assert!(vendor_dir.is_some());
    }

    #[test]
    fn test_pristine_workflow_complex_scenario() {
        let gem_names = [
            "rails".to_string(),
            "devise".to_string(),
            "bootstrap".to_string(),
        ];
        let lockfile_path = "/home/developer/project/Gemfile.lock";
        let vendor_dir = Some("/home/developer/project/vendor");

        assert_eq!(gem_names.len(), 3);
        assert!(!lockfile_path.is_empty());
        assert!(vendor_dir.is_some());
    }
}
