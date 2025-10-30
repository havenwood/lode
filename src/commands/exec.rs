//! Exec command
//!
//! Run a command with the lode managed gem environment

use anyhow::{Context, Result};
use lode::{Config, config, lockfile::Lockfile};
use std::env;
use std::fs;
use std::process::Command;

/// Run a command with the lode-managed gem environment
pub(crate) fn run(command: &[String], lockfile_path: &str) -> Result<()> {
    if command.is_empty() {
        anyhow::bail!("No command specified. Usage: lode exec -- <command> [args...]");
    }

    // Read and parse lockfile to get Ruby version
    let content = fs::read_to_string(lockfile_path)
        .with_context(|| format!("Failed to read lockfile: {lockfile_path}"))?;

    let lockfile = Lockfile::parse(&content)
        .with_context(|| format!("Failed to parse lockfile: {lockfile_path}"))?;

    // Get vendor directory
    let cfg = Config::load().unwrap_or_default();
    let vendor_dir = config::vendor_dir(Some(&cfg))?;

    // Determine Ruby version from lockfile or detect active Ruby
    let ruby_version = config::ruby_version(lockfile.ruby_version.as_deref());

    // Build gem paths
    let gems_root = vendor_dir.join("ruby").join(&ruby_version);
    let gems_dir = gems_root.join("gems");
    let bin_dir = gems_root.join("bin");

    // Prepare environment variables
    let first_cmd = command.first().context("Command cannot be empty")?;
    let mut cmd = Command::new(first_cmd);

    // Add command arguments
    if let Some(args) = command.get(1..) {
        cmd.args(args);
    }

    // Set GEM_HOME to our vendor directory
    cmd.env("GEM_HOME", &gems_root);

    // Set GEM_PATH to include our vendor directory
    let gem_path = env::var("GEM_PATH").map_or_else(
        |_| gems_root.display().to_string(),
        |existing_path| format!("{}:{existing_path}", gems_root.display()),
    );
    cmd.env("GEM_PATH", gem_path);

    // Set BUNDLE_GEMFILE to absolute path (supports both Gemfile and gems.rb)
    let gemfile_path = env::current_dir()?.join(lode::paths::find_gemfile());
    if gemfile_path.exists() {
        cmd.env("BUNDLE_GEMFILE", gemfile_path);
    }

    // Prepend bin directory to PATH
    if bin_dir.exists() {
        let path = env::var("PATH").map_or_else(
            |_| bin_dir.display().to_string(),
            |existing_path| format!("{}:{existing_path}", bin_dir.display()),
        );
        cmd.env("PATH", path);
    }

    // Set RUBYLIB to include gem lib directories (for require to work)
    let mut ruby_lib_paths = Vec::new();
    if gems_dir.exists() {
        // Add all gem lib directories to RUBYLIB
        if let Ok(entries) = fs::read_dir(&gems_dir) {
            for entry in entries.flatten() {
                let gem_lib = entry.path().join("lib");
                if gem_lib.is_dir() {
                    ruby_lib_paths.push(gem_lib.display().to_string());
                }
            }
        }
    }

    if !ruby_lib_paths.is_empty() {
        let joined = ruby_lib_paths.join(":");
        let rubylib = env::var("RUBYLIB").map_or_else(
            |_| joined.clone(),
            |existing_lib| format!("{joined}:{existing_lib}"),
        );
        cmd.env("RUBYLIB", rubylib);
    }

    // Execute the command
    let status = cmd
        .status()
        .with_context(|| format!("Failed to execute command: {first_cmd}"))?;

    // Exit with the same code as the command
    if !status.success() {
        let code = status.code().unwrap_or(1);
        std::process::exit(code);
    }

    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "Tests can panic")]
mod tests {
    use super::*;

    #[test]
    fn exec_empty_command() {
        let result = run(&[], "Gemfile.lock");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No command"));
    }

    #[test]
    fn exec_nonexistent_lockfile() {
        let result = run(&["echo".to_string()], "/nonexistent/Gemfile.lock");
        assert!(result.is_err());
    }
}
