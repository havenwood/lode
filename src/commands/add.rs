//! Add command
//!
//! Add a gem to the Gemfile

use anyhow::{Context, Result};
use lode::GemfileWriter;
use std::fmt::Write;

/// Add a gem to the Gemfile.
///
/// This command adds a gem declaration to the Gemfile with optional version
/// and group constraints. It preserves the original formatting and structure.
///
/// # Example
///
/// ```bash
/// lode add rails --version "~> 7.0"
/// lode add rspec --group test
/// lode add bootsnap --skip-lock  # Don't run lock
/// ```
#[allow(
    clippy::too_many_arguments,
    clippy::fn_params_excessive_bools,
    clippy::cognitive_complexity
)]
pub(crate) async fn run(
    gem_name: &str,
    version: Option<&str>,
    group: Option<&str>,
    require: Option<bool>,
    source: Option<&str>,
    git: Option<&str>,
    github: Option<&str>,
    branch: Option<&str>,
    git_ref: Option<&str>,
    glob: Option<&str>,
    path: Option<&str>,
    strict: bool,
    optimistic: bool,
    quiet: bool,
    run_lock: bool,
) -> Result<()> {
    let gemfile_path = lode::find_gemfile();

    if !gemfile_path.exists() {
        anyhow::bail!("Gemfile or gems.rb not found. Run `lode init` first.");
    }

    // Load Gemfile for modification
    let mut writer = GemfileWriter::load(&gemfile_path).context("Failed to load Gemfile")?;

    // Apply strict or optimistic version constraint
    let version = version.map(|v| {
        if strict {
            format!("= {v}")
        } else if optimistic {
            format!(">= {v}")
        } else {
            v.to_string()
        }
    });

    // Build options string
    let mut options_parts = Vec::new();

    // Add require option
    if let Some(r) = require {
        if r {
            options_parts.push("require: true".to_string());
        } else {
            options_parts.push("require: false".to_string());
        }
    }

    // Add source option
    if let Some(src) = source {
        options_parts.push(format!("source: '{src}'"));
    }

    // Convert --github to full git URL
    let git_url = github.map_or_else(
        || git.map(ToString::to_string),
        |github_repo| Some(format!("https://github.com/{github_repo}")),
    );

    // Add git options
    if let Some(ref git_url_str) = git_url {
        options_parts.push(format!("git: '{git_url_str}'"));
        if let Some(branch_name) = branch {
            options_parts.push(format!("branch: '{branch_name}'"));
        }
        if let Some(ref_name) = git_ref {
            options_parts.push(format!("ref: '{ref_name}'"));
        }
    }

    // Add glob option
    if let Some(glob_pattern) = glob {
        options_parts.push(format!("glob: '{glob_pattern}'"));
    }

    // Add path option
    if let Some(local_path) = path {
        options_parts.push(format!("path: '{local_path}'"));
    }

    let options = if options_parts.is_empty() {
        None
    } else {
        Some(options_parts.join(", "))
    };

    // Add gem to Gemfile
    writer
        .add_gem(gem_name, version.as_deref(), group, options.as_deref())
        .with_context(|| format!("Failed to add gem '{gem_name}' to Gemfile"))?;

    // Write changes
    writer.write().context("Failed to write updated Gemfile")?;

    // Build and display success message
    if !quiet {
        let mut message = format!("gem \"{gem_name}\"");
        if let Some(ref ver) = version {
            let _ = write!(message, ", \"{ver}\"");
        }
        if let Some(grp) = group {
            let _ = write!(message, " (group: {grp})");
        }
        if let Some(src) = source {
            let _ = write!(message, ", source: {src}");
        }
        if let Some(github_repo) = github {
            let _ = write!(message, ", github: {github_repo}");
        } else if let Some(ref git_url_str) = git_url {
            let _ = write!(message, ", git: {git_url_str}");
            if let Some(branch_name) = branch {
                let _ = write!(message, ", branch: {branch_name}");
            }
            if let Some(ref_name) = git_ref {
                let _ = write!(message, ", ref: {ref_name}");
            }
        }
        if let Some(glob_pattern) = glob {
            let _ = write!(message, ", glob: {glob_pattern}");
        }
        if let Some(local_path) = path {
            let _ = write!(message, ", path: {local_path}");
        }
        if let Some(req) = require
            && !req
        {
            message.push_str(", require: false");
        }

        println!("Added {message}");
    }

    // Run lock if requested
    if run_lock {
        let lockfile_path = lode::lockfile_for_gemfile(&gemfile_path);
        let lockfile_name = lockfile_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Gemfile.lock");

        if !quiet {
            println!("\nUpdating {lockfile_name}...");
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

    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "Tests can panic")]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_add_gem_basic() {
        let temp = TempDir::new().unwrap();
        let gemfile = temp.path().join("Gemfile");
        fs::write(&gemfile, "source \"https://rubygems.org\"\n").unwrap();

        let result = run(
            "rails",
            Some("~> 7.0"),
            None,  // group
            None,  // require
            None,  // source
            None,  // git
            None,  // github
            None,  // branch
            None,  // git_ref
            None,  // glob
            None,  // path
            false, // strict
            false, // optimistic
            false, // quiet
            false, // run_lock
        )
        .await;

        // Should fail because we're not in the temp directory
        // This test is mainly to verify the function signature compiles
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_add_gem_no_gemfile() {
        let temp = TempDir::new().unwrap();
        let orig_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(&temp).unwrap();

        let result = run(
            "rails", None,  // version
            None,  // group
            None,  // require
            None,  // source
            None,  // git
            None,  // github
            None,  // branch
            None,  // git_ref
            None,  // glob
            None,  // path
            false, // strict
            false, // optimistic
            false, // quiet
            false, // run_lock
        )
        .await;

        // Restore directory before assertions
        drop(std::env::set_current_dir(&orig_dir));

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }
}
