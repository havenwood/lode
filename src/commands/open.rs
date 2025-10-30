//! Open command
//!
//! Open a gem in your editor

use anyhow::{Context, Result};
use lode::{Config, config, lockfile::Lockfile};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

/// Open a gem's source code in your editor
///
/// This command opens the gem's installation directory in your configured editor.
/// If a relative path is specified, it opens that specific file within the gem.
/// It respects the following environment variables in order:
/// 1. `BUNDLER_EDITOR`
/// 2. `VISUAL`
/// 3. `EDITOR`
/// 4. Falls back to "vi"
pub(crate) fn run(gem_name: &str, relative_path: Option<&str>) -> Result<()> {
    // Find the gem's installation directory
    let gem_dir = find_gem_path(gem_name)?;

    // Determine the path to open (gem dir or specific file within it)
    let path_to_open = if let Some(rel_path) = relative_path {
        let target_path = gem_dir.join(rel_path);
        if !target_path.exists() {
            anyhow::bail!("Path '{rel_path}' not found in gem '{gem_name}'");
        }
        target_path
    } else {
        gem_dir
    };

    // Get the editor to use
    let editor = get_editor();

    if let Some(rel_path) = relative_path {
        println!("Opening {rel_path} in {gem_name} with {editor}...");
    } else {
        println!("Opening {gem_name} in {editor}...");
    }

    // Spawn the editor
    let status = Command::new(&editor)
        .arg(&path_to_open)
        .status()
        .with_context(|| format!("Failed to spawn editor '{editor}'"))?;

    if !status.success() {
        // Don't fail if editor returns non-zero - user might have cancelled
        eprintln!("Editor exited with status: {status}");
    }

    Ok(())
}

/// Get the editor to use from environment variables
///
/// Priority order:
/// 1. `BUNDLER_EDITOR` (Bundler-specific)
/// 2. `VISUAL` (standard Unix)
/// 3. `EDITOR` (standard Unix)
/// 4. "vi" (fallback)
fn get_editor() -> String {
    get_editor_from_env(|key| std::env::var(key))
}

/// Get the editor from provided environment variable lookup function
///
/// This function is separated for testability without manipulating global state.
fn get_editor_from_env<F>(env_lookup: F) -> String
where
    F: Fn(&str) -> Result<String, std::env::VarError>,
{
    env_lookup("BUNDLER_EDITOR")
        .or_else(|_| env_lookup("VISUAL"))
        .or_else(|_| env_lookup("EDITOR"))
        .unwrap_or_else(|_| "vi".to_string())
}

/// Find the installation path of a gem
///
/// This duplicates logic from show.rs but keeps the command self-contained.
fn find_gem_path(gem_name: &str) -> Result<PathBuf> {
    // Read and parse lockfile
    let lockfile_path = "Gemfile.lock";
    let content = fs::read_to_string(lockfile_path)
        .with_context(|| format!("Failed to read lockfile: {lockfile_path}"))?;

    let lockfile = Lockfile::parse(&content)
        .with_context(|| format!("Failed to parse lockfile: {lockfile_path}"))?;

    // Get vendor directory
    let cfg = Config::load().unwrap_or_default();
    let vendor_dir = config::vendor_dir(Some(&cfg))?;

    // Determine Ruby version from lockfile
    let ruby_version = lockfile
        .ruby_version
        .as_ref()
        .map_or_else(|| "3.4.0".to_string(), |v| normalize_version(v));

    let gems_dir = vendor_dir.join("ruby").join(&ruby_version).join("gems");

    // Find the gem in the lockfile
    // Check regular gems
    if let Some(gem) = lockfile.gems.iter().find(|gem| gem.name == gem_name) {
        let gem_dir = gems_dir.join(gem.full_name());
        if gem_dir.exists() {
            return Ok(gem_dir);
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
            return Ok(gem_dir);
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
            return Ok(gem_dir);
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

/// Normalize Ruby version from lockfile format
///
/// Converts "ruby 3.3.0p0" to "3.3.0"
fn normalize_version(version: &str) -> String {
    version
        .trim()
        .trim_start_matches("ruby ")
        .split('p')
        .next()
        .unwrap_or(version)
        .to_string()
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "Tests can panic")]
mod tests {
    use super::*;

    #[test]
    fn get_editor_with_bundler_editor() {
        let result = get_editor_from_env(|key: &str| match key {
            "BUNDLER_EDITOR" => Ok("code".to_string()),
            "VISUAL" => Ok("vim".to_string()),
            "EDITOR" => Ok("nano".to_string()),
            _ => Err(std::env::VarError::NotPresent),
        });

        assert_eq!(result, "code");
    }

    #[test]
    fn get_editor_with_visual() {
        let result = get_editor_from_env(|key: &str| match key {
            "VISUAL" => Ok("emacs".to_string()),
            "EDITOR" => Ok("nano".to_string()),
            _ => Err(std::env::VarError::NotPresent),
        });

        assert_eq!(result, "emacs");
    }

    #[test]
    fn get_editor_with_editor() {
        let result = get_editor_from_env(|key: &str| match key {
            "EDITOR" => Ok("nano".to_string()),
            _ => Err(std::env::VarError::NotPresent),
        });

        assert_eq!(result, "nano");
    }

    #[test]
    fn get_editor_fallback() {
        let result = get_editor_from_env(|_key: &str| Err(std::env::VarError::NotPresent));

        assert_eq!(result, "vi");
    }

    #[test]
    fn normalize_version_strips_ruby_prefix_and_patchlevel() {
        assert_eq!(normalize_version("ruby 3.3.0p0"), "3.3.0");
        assert_eq!(normalize_version("3.4.1p194"), "3.4.1");
        assert_eq!(normalize_version("3.3.0"), "3.3.0");
        assert_eq!(normalize_version("ruby 2.7.6p194"), "2.7.6");
    }
}
