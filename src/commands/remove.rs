//! Remove command
//!
//! Remove a gem from the Gemfile

use anyhow::{Context, Result};
use lode::GemfileWriter;

/// Remove gems from the Gemfile.
///
/// This command removes gem declarations from the Gemfile while preserving
/// the original formatting and structure.
///
/// # Example
///
/// ```bash
/// lode remove minitest
/// lode remove rspec webmock  # Remove multiple gems
/// lode remove rails --skip-lock   # Don't run lock
/// lode remove webmock --skip-clean  # Don't run clean
/// lode remove rails --install  # Run install after removing
/// ```
pub(crate) async fn run(gem_names: &[String], quiet: bool) -> Result<()> {
    if gem_names.is_empty() {
        anyhow::bail!("No gems specified. Usage: lode remove GEM [GEM ...]");
    }

    // Default behavior: always run lock and clean, never run install
    run_with_gemfile(gem_names, None, false, true, true, quiet).await
}

#[allow(clippy::fn_params_excessive_bools)]
async fn run_with_gemfile(
    gem_names: &[String],
    gemfile_path: Option<&str>,
    run_install: bool,
    run_lock: bool,
    run_clean: bool,
    quiet: bool,
) -> Result<()> {
    let gemfile_path = gemfile_path.map_or_else(lode::find_gemfile, std::path::PathBuf::from);

    if !gemfile_path.exists() {
        anyhow::bail!("Gemfile or gems.rb not found");
    }

    let gemfile_name = gemfile_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("Gemfile");

    // Load Gemfile for modification
    let mut writer = GemfileWriter::load(&gemfile_path).context("Failed to load Gemfile")?;

    // Remove gems from Gemfile
    let mut removed_gems = Vec::new();
    let mut not_found_gems = Vec::new();

    for gem_name in gem_names {
        let removed = writer
            .remove_gem(gem_name)
            .with_context(|| format!("Failed to remove gem '{gem_name}' from {gemfile_name}"))?;

        if removed {
            removed_gems.push(gem_name.clone());
        } else {
            not_found_gems.push(gem_name.clone());
        }
    }

    // Check if any gems were not found
    if !not_found_gems.is_empty() {
        if removed_gems.is_empty() {
            // All gems not found - error
            if let [gem] = &not_found_gems[..] {
                anyhow::bail!("Gem '{gem}' not found in {gemfile_name}");
            }
            anyhow::bail!(
                "Gems not found in {}: {}",
                gemfile_name,
                not_found_gems.join(", ")
            );
        }
        // Some gems not found - warn
        if !quiet {
            println!(
                "Warning: Gems not found in {}: {}",
                gemfile_name,
                not_found_gems.join(", ")
            );
        }
    }

    // Write changes
    writer.write().context("Failed to write updated Gemfile")?;

    // Print success message
    if !quiet {
        println!("Removing gems from {}", gemfile_path.display());
        for gem in &removed_gems {
            println!("{gem} was removed.");
        }
    }

    // Run lock if requested
    if run_lock {
        let lockfile_path = lode::lockfile_for_gemfile(&gemfile_path);
        let lockfile_name = lockfile_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Gemfile.lock");
        if !quiet {
            println!();
            println!("Updating {lockfile_name}...");
        }
        crate::commands::lock::run(
            gemfile_path.to_str().unwrap_or("Gemfile"),
            None,  // lockfile_path
            &[],   // add_platforms
            &[],   // remove_platforms
            &[],   // update_gems
            false, // print
            false, // verbose
            false, // patch
            false, // minor
            false, // major
            false, // strict
            false, // conservative
            false, // local
            false, // pre
            None,  // bundler
            false, // normalize_platforms
            false, // add_checksums
            false, // full_index
            quiet, // quiet
        )
        .await?;
        if !quiet {
            println!("{lockfile_name} updated");
        }
    } else if !quiet {
        println!("\nRun `lode lock` to update lockfile");
    }

    // Run clean if requested
    if run_clean && run_lock {
        if !quiet {
            println!();
            println!("Cleaning unused gems...");
        }
        crate::commands::clean::run(None, false, false)?;
    } else if run_clean && !quiet {
        println!("\nRun `lode clean` to remove unused gems from vendor directory");
    }

    // Run install if requested
    if run_install {
        if !quiet {
            println!();
            println!("Running install...");
        }
        let lockfile_path = lode::lockfile_for_gemfile(&gemfile_path);
        let lockfile_str = lockfile_path.to_str().unwrap_or("Gemfile.lock");
        crate::commands::install::run(crate::commands::install::InstallOptions {
            lockfile_path: lockfile_str,
            redownload: false,
            verbose: false,
            quiet,
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
        if !quiet {
            println!("Install complete");
        }
    }

    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "Tests can panic")]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_remove_gem_not_found() {
        let temp = TempDir::new().unwrap();
        let gemfile = temp.path().join("Gemfile");
        fs::write(&gemfile, "source \"https://rubygems.org\"\n").unwrap();

        let result = run_with_gemfile(
            &[String::from("rails")],
            Some(gemfile.to_str().unwrap()),
            false,
            false,
            false,
            false,
        )
        .await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[tokio::test]
    async fn test_remove_gem_no_gemfile() {
        let temp = TempDir::new().unwrap();
        let gemfile = temp.path().join("Gemfile");

        let result = run_with_gemfile(
            &[String::from("rails")],
            Some(gemfile.to_str().unwrap()),
            false,
            false,
            false,
            false,
        )
        .await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }
}
