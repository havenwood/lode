//! Contents command
//!
//! List all files in an installed gem

use anyhow::{Context, Result};
use lode::{Config, config};
use std::path::{Path, PathBuf};

/// Options for the contents command
#[derive(Debug)]
pub(crate) struct ContentsOptions {
    pub all: bool,
    pub lib_only: bool,
    pub prefix: bool,
    pub show_install_dir: bool,
}

/// List all files in an installed gem.
///
/// Searches for the gem in:
/// 1. Vendor directory (project gems)
/// 2. System gem directory
pub(crate) fn run(
    gems: &[String],
    version: Option<&str>,
    spec_dirs: &[String],
    options: &ContentsOptions,
) -> Result<()> {
    // Load configuration
    let config = Config::load().context("Failed to load configuration")?;

    // If --all, get all installed gems
    let gems_to_process: Vec<String> = if options.all {
        get_all_installed_gems(&config)?
    } else if gems.is_empty() {
        anyhow::bail!("No gem name specified. Use --all to show all gems.");
    } else {
        gems.to_vec()
    };

    for gem_name in &gems_to_process {
        // Determine gem directory
        let gem_dir = if spec_dirs.is_empty() {
            find_gem_directory(gem_name, version, None, &config)?
        } else {
            find_gem_in_spec_dirs(gem_name, version, spec_dirs)?
        };

        // If --show-install-dir, just print the directory
        if options.show_install_dir {
            println!("{}", gem_dir.display());
            continue;
        }

        // List all files recursively
        let mut files = list_files_recursive(&gem_dir)?;

        // Filter for lib_only if requested
        if options.lib_only {
            files.retain(|f| {
                f.strip_prefix(&gem_dir)
                    .map(|p| p.starts_with("lib"))
                    .unwrap_or(false)
            });
        }

        if files.is_empty() {
            println!("No files found in gem {gem_name}");
            continue;
        }

        // Print files
        for file in &files {
            if options.prefix {
                // Show absolute path
                println!("{}", file.display());
            } else {
                // Show path relative to gem root
                let relative = file.strip_prefix(&gem_dir).unwrap_or(file);
                println!("{}", relative.display());
            }
        }

        if options.all && gems_to_process.len() > 1 {
            println!();
        }
    }

    Ok(())
}

/// Get all installed gems
fn get_all_installed_gems(config: &Config) -> Result<Vec<String>> {
    let vendor_dir = config::vendor_dir(Some(config))?;
    let ruby_version = config::ruby_version(None);
    let gems_dir = vendor_dir.join("ruby").join(&ruby_version).join("gems");

    let mut gem_names = Vec::new();

    if gems_dir.exists() {
        for entry in std::fs::read_dir(&gems_dir)? {
            if let Ok(entry) = entry
                && entry.path().is_dir()
                && let Some(name) = entry.file_name().to_str()
                && let Some(base_name) = extract_gem_name(name)
                && !gem_names.contains(&base_name.to_string())
            {
                gem_names.push(base_name.to_string());
            }
        }
    }

    gem_names.sort();
    Ok(gem_names)
}

/// Extract gem name from directory name (e.g., "rack-3.0.8" -> "rack")
fn extract_gem_name(dir_name: &str) -> Option<&str> {
    dir_name.rfind('-').map(|pos| &dir_name[..pos])
}

/// Find gem in specific spec directories
fn find_gem_in_spec_dirs(
    gem_name: &str,
    version: Option<&str>,
    spec_dirs: &[String],
) -> Result<PathBuf> {
    for spec_dir in spec_dirs {
        let path = PathBuf::from(spec_dir);
        if let Ok(Some(gem_dir)) = find_matching_gem(&path, gem_name, version) {
            return Ok(gem_dir);
        }
    }
    anyhow::bail!(
        "Gem '{gem_name}' not found in specified directories: {spec_dirs}",
        spec_dirs = spec_dirs.join(", ")
    );
}

/// Find the gem directory for a given gem name
fn find_gem_directory(
    gem_name: &str,
    version: Option<&str>,
    vendor: Option<&str>,
    config: &Config,
) -> Result<PathBuf> {
    // Try vendor directory first
    let vendor_dir = vendor
        .map(PathBuf::from)
        .map_or_else(|| config::vendor_dir(Some(config)), Ok)?;

    // Detect Ruby version
    let ruby_version = if Path::new("Gemfile.lock").exists() {
        let lockfile_content =
            std::fs::read_to_string("Gemfile.lock").context("Failed to read Gemfile.lock")?;
        let lockfile =
            lode::Lockfile::parse(&lockfile_content).context("Failed to parse Gemfile.lock")?;
        lockfile.ruby_version
    } else {
        None
    };

    let ruby_ver = config::ruby_version(ruby_version.as_deref());
    let gems_dir = vendor_dir.join("ruby").join(&ruby_ver).join("gems");

    if gems_dir.exists() {
        // Search for gem directory
        if let Some(gem_dir) = find_matching_gem(&gems_dir, gem_name, version)? {
            return Ok(gem_dir);
        }
    }

    // Try system gem directory
    let system_gem_dir = lode::get_system_gem_dir(&ruby_ver);
    if system_gem_dir.exists()
        && let Some(gem_dir) = find_matching_gem(&system_gem_dir, gem_name, version)?
    {
        return Ok(gem_dir);
    }

    // Not found
    anyhow::bail!("Gem '{gem_name}' not found in vendor or system directories");
}

/// Find a gem directory matching the name and optional version
fn find_matching_gem(
    gems_dir: &Path,
    gem_name: &str,
    version: Option<&str>,
) -> Result<Option<PathBuf>> {
    let entries = std::fs::read_dir(gems_dir)
        .with_context(|| format!("Failed to read directory: {}", gems_dir.display()))?;

    let mut matches = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let dir_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        // Check if directory matches gem name pattern
        if let Some(version_str) = version {
            // Exact match with version
            let expected = format!("{gem_name}-{version_str}");
            if dir_name == expected {
                return Ok(Some(path));
            }
        } else {
            // Match any version of this gem
            if dir_name.starts_with(&format!("{gem_name}-")) {
                matches.push((path.clone(), dir_name.to_string()));
            }
        }
    }

    // If no version specified and multiple matches, use the latest
    if !matches.is_empty() && version.is_none() {
        // Sort by version (simple string sort, latest is usually highest)
        matches.sort_by(|(_, a), (_, b)| b.cmp(a));
        return Ok(Some(matches.first().expect("should have match").0.clone()));
    }

    Ok(None)
}

/// List all files recursively in a directory
fn list_files_recursive(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    collect_files(dir, &mut files)?;
    files.sort();
    Ok(files)
}

/// Recursively collect all files in a directory
fn collect_files(dir: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    if !dir.is_dir() {
        return Ok(());
    }

    let entries = std::fs::read_dir(dir)
        .with_context(|| format!("Failed to read directory: {}", dir.display()))?;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() {
            files.push(path);
        } else if path.is_dir() {
            collect_files(&path, files)?;
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

    #[test]
    fn find_matching_gem_exact() {
        // Create temporary gem structure
        let temp = TempDir::new().unwrap();
        let gems_dir = temp.path();

        // Create gem directories
        fs::create_dir_all(gems_dir.join("rack-3.0.8")).unwrap();
        fs::create_dir_all(gems_dir.join("rack-3.0.9")).unwrap();
        fs::create_dir_all(gems_dir.join("rails-7.0.8")).unwrap();

        // Find exact match
        let result = find_matching_gem(gems_dir, "rack", Some("3.0.8")).unwrap();
        assert!(result.is_some());
        assert_eq!(
            result.unwrap().file_name().unwrap().to_str().unwrap(),
            "rack-3.0.8"
        );
    }

    #[test]
    fn find_matching_gem_latest() {
        // Create temporary gem structure
        let temp = TempDir::new().unwrap();
        let gems_dir = temp.path();

        // Create gem directories
        fs::create_dir_all(gems_dir.join("rack-3.0.8")).unwrap();
        fs::create_dir_all(gems_dir.join("rack-3.0.9")).unwrap();

        // Find latest (no version specified)
        let result = find_matching_gem(gems_dir, "rack", None).unwrap();
        assert!(result.is_some());
        // Should find one of them
        let found = result.unwrap();
        assert!(
            found
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .starts_with("rack-")
        );
    }

    #[test]
    fn test_list_files_recursive() {
        // Create temporary directory structure
        let temp = TempDir::new().unwrap();
        let gem_dir = temp.path();

        // Create some files
        fs::write(gem_dir.join("README.md"), "readme").unwrap();
        fs::create_dir(gem_dir.join("lib")).unwrap();
        fs::write(gem_dir.join("lib/gem.rb"), "code").unwrap();

        // List files
        let files = list_files_recursive(gem_dir).unwrap();
        assert_eq!(files.len(), 2);
        assert!(files.iter().any(|f| f.ends_with("README.md")));
        assert!(files.iter().any(|f| f.ends_with("gem.rb")));
    }

    #[test]
    fn collect_files_empty_dir() {
        let temp = TempDir::new().unwrap();
        let mut files = Vec::new();
        collect_files(temp.path(), &mut files).unwrap();
        assert_eq!(files.len(), 0);
    }
}
