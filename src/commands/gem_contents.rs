//! Contents command
//!
//! List all files in an installed gem

use anyhow::{Context, Result};
use lode::gem_store::GemStore;
use std::fs;
use std::path::{Path, PathBuf};

/// Options for the gem-contents command
#[derive(Debug, Clone)]
pub(crate) struct ContentsOptions {
    pub gem_name: String,
    pub version: Option<String>,
    pub all: bool,
    pub spec_dir: Option<Vec<String>>,
    pub lib_only: bool,
    pub prefix: bool,
    pub show_install_dir: bool,
    pub verbose: bool,
    pub quiet: bool,
    pub silent: bool,
}

/// List all files in an installed gem
pub(crate) fn run(opts: &ContentsOptions) -> Result<()> {
    // Create gem stores - either from spec_dir or default
    let stores: Vec<GemStore> = if let Some(ref spec_dirs) = opts.spec_dir {
        // Use custom spec directories
        spec_dirs
            .iter()
            .map(|dir| {
                let gems_path = Path::new(dir).join("gems");
                if gems_path.exists() {
                    GemStore::with_path(gems_path)
                } else {
                    // Try the path directly as a gems directory
                    GemStore::with_path(PathBuf::from(dir))
                }
            })
            .collect()
    } else {
        // Use default system gem directory
        vec![GemStore::new()?]
    };

    // If --all flag is set, list contents for all gems
    if opts.all {
        return list_all_gems_from_stores(&stores, opts);
    }

    // Find matching gems across all stores
    let mut matching_gems = Vec::new();
    for store in &stores {
        if let Ok(gems) = store.find_gem_by_name(&opts.gem_name) {
            matching_gems.extend(gems);
        }
    }

    if matching_gems.is_empty() {
        anyhow::bail!("Gem '{}' not found", opts.gem_name);
    }

    // If version specified, find that specific version
    let gem = if let Some(ref v) = opts.version {
        matching_gems
            .iter()
            .find(|g| g.version == *v)
            .with_context(|| format!("Version '{v}' of gem '{}' not found", opts.gem_name))?
    } else {
        // Use the latest version (last in sorted list)
        matching_gems.last().context("No gems found")?
    };

    // If --show-install-dir, just show the install directory
    if opts.show_install_dir {
        println!("{}", gem.path.display());
        return Ok(());
    }

    // List all files recursively
    let mut files = list_files_recursive(&gem.path)?;

    // Filter for lib_only if requested
    if opts.lib_only {
        let lib_dir = gem.path.join("lib");
        files.retain(|f| f.starts_with(&lib_dir));
    }

    if files.is_empty() {
        if !opts.silent && !opts.quiet {
            println!("No files found in {}", gem.path.display());
        }
        return Ok(());
    }

    // Display files
    for file in files {
        if opts.prefix {
            // Show full path
            println!("{}", file.display());
        } else {
            // Show relative path from gem directory
            if let Ok(rel_path) = file.strip_prefix(&gem.path) {
                println!("{}", rel_path.display());
            } else {
                println!("{}", file.display());
            }
        }
    }

    Ok(())
}

/// List contents for all installed gems from multiple stores
fn list_all_gems_from_stores(stores: &[GemStore], opts: &ContentsOptions) -> Result<()> {
    let mut found_any = false;

    for store in stores {
        if let Ok(all_gems) = store.list_gems()
            && !all_gems.is_empty()
        {
            found_any = true;
            for gem in all_gems {
                if opts.show_install_dir {
                    println!("{}", gem.path.display());
                } else {
                    if opts.verbose {
                        println!("{}:", gem.name);
                    }

                    let mut files = list_files_recursive(&gem.path)?;

                    // Filter for lib_only if requested
                    if opts.lib_only {
                        let lib_dir = gem.path.join("lib");
                        files.retain(|f| f.starts_with(&lib_dir));
                    }

                    for file in files {
                        if opts.prefix {
                            println!("{}", file.display());
                        } else if let Ok(rel_path) = file.strip_prefix(&gem.path) {
                            println!("{}", rel_path.display());
                        } else {
                            println!("{}", file.display());
                        }
                    }

                    if opts.verbose {
                        println!();
                    }
                }
            }
        }
    }

    if !found_any && !opts.silent && !opts.quiet {
        println!("No gems installed");
    }

    Ok(())
}

/// Recursively list all files in a directory
fn list_files_recursive(dir: &Path) -> Result<Vec<std::path::PathBuf>> {
    let mut files = Vec::new();

    if !dir.exists() {
        return Ok(files);
    }

    let entries = fs::read_dir(dir)
        .with_context(|| format!("Failed to read directory: {}", dir.display()))?;

    for entry in entries.flatten() {
        let path = entry.path();

        if path.is_file() {
            files.push(path);
        } else if path.is_dir() {
            // Recursively list files in subdirectory
            files.extend(list_files_recursive(&path)?);
        }
    }

    // Sort for consistent output
    files.sort();

    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

    #[test]
    fn test_list_files_recursive_empty_dir() {
        let temp_dir = TempDir::new().unwrap();
        let files = list_files_recursive(temp_dir.path()).unwrap();
        assert!(files.is_empty());
    }

    #[test]
    fn test_list_files_recursive_with_files() {
        let temp_dir = TempDir::new().unwrap();
        let file1 = temp_dir.path().join("file1.txt");
        let file2 = temp_dir.path().join("file2.txt");
        fs::write(&file1, "test").unwrap();
        fs::write(&file2, "test").unwrap();

        let files = list_files_recursive(temp_dir.path()).unwrap();
        assert_eq!(files.len(), 2);
    }

    #[test]
    fn test_list_files_recursive_with_subdirs() {
        let temp_dir = TempDir::new().unwrap();
        let subdir = temp_dir.path().join("subdir");
        fs::create_dir(&subdir).unwrap();
        let file1 = temp_dir.path().join("file1.txt");
        let file2 = subdir.join("file2.txt");
        fs::write(&file1, "test").unwrap();
        fs::write(&file2, "test").unwrap();

        let files = list_files_recursive(temp_dir.path()).unwrap();
        assert_eq!(files.len(), 2);
    }

    #[test]
    fn test_list_files_recursive_nonexistent() {
        let nonexistent = PathBuf::from("/nonexistent/path");
        let files = list_files_recursive(&nonexistent).unwrap();
        assert!(files.is_empty());
    }

    #[test]
    fn test_contents_options_defaults() {
        let opts = ContentsOptions {
            gem_name: String::new(),
            version: None,
            all: false,
            spec_dir: None,
            lib_only: false,
            prefix: false,
            show_install_dir: false,
            verbose: false,
            quiet: false,
            silent: false,
        };

        assert_eq!(opts.gem_name, "");
        assert!(opts.version.is_none());
        assert!(!opts.all);
        assert!(opts.spec_dir.is_none());
        assert!(!opts.lib_only);
        assert!(!opts.prefix);
        assert!(!opts.show_install_dir);
    }

    #[test]
    fn test_contents_options_gem_name() {
        let opts = ContentsOptions {
            gem_name: "rails".to_string(),
            version: None,
            all: false,
            spec_dir: None,
            lib_only: false,
            prefix: false,
            show_install_dir: false,
            verbose: false,
            quiet: false,
            silent: false,
        };

        assert_eq!(opts.gem_name, "rails");
    }

    #[test]
    fn test_contents_options_version_specification() {
        let opts = ContentsOptions {
            gem_name: "rails".to_string(),
            version: Some("7.0.0".to_string()),
            all: false,
            spec_dir: None,
            lib_only: false,
            prefix: false,
            show_install_dir: false,
            verbose: false,
            quiet: false,
            silent: false,
        };

        assert_eq!(opts.version, Some("7.0.0".to_string()));
    }

    #[test]
    fn test_contents_options_all_flag() {
        let opts = ContentsOptions {
            gem_name: String::new(),
            version: None,
            all: true,
            spec_dir: None,
            lib_only: false,
            prefix: false,
            show_install_dir: false,
            verbose: false,
            quiet: false,
            silent: false,
        };

        assert!(opts.all);
    }

    #[test]
    fn test_contents_options_spec_dir() {
        let spec_dirs = vec!["/custom/gems".to_string(), "/another/gems".to_string()];
        let opts = ContentsOptions {
            gem_name: "rails".to_string(),
            version: None,
            all: false,
            spec_dir: Some(spec_dirs),
            lib_only: false,
            prefix: false,
            show_install_dir: false,
            verbose: false,
            quiet: false,
            silent: false,
        };

        assert!(opts.spec_dir.is_some());
        assert_eq!(opts.spec_dir.unwrap().len(), 2);
    }

    #[test]
    fn test_contents_options_lib_only_flag() {
        let opts = ContentsOptions {
            gem_name: "rails".to_string(),
            version: None,
            all: false,
            spec_dir: None,
            lib_only: true,
            prefix: false,
            show_install_dir: false,
            verbose: false,
            quiet: false,
            silent: false,
        };

        assert!(opts.lib_only);
    }

    #[test]
    fn test_contents_options_prefix_flag() {
        let opts = ContentsOptions {
            gem_name: "rails".to_string(),
            version: None,
            all: false,
            spec_dir: None,
            lib_only: false,
            prefix: true,
            show_install_dir: false,
            verbose: false,
            quiet: false,
            silent: false,
        };

        assert!(opts.prefix);
    }

    #[test]
    fn test_contents_options_show_install_dir() {
        let opts = ContentsOptions {
            gem_name: "rails".to_string(),
            version: None,
            all: false,
            spec_dir: None,
            lib_only: false,
            prefix: false,
            show_install_dir: true,
            verbose: false,
            quiet: false,
            silent: false,
        };

        assert!(opts.show_install_dir);
    }

    #[test]
    fn test_contents_options_output_control() {
        let opts = ContentsOptions {
            gem_name: "rails".to_string(),
            version: None,
            all: false,
            spec_dir: None,
            lib_only: false,
            prefix: false,
            show_install_dir: false,
            verbose: true,
            quiet: true,
            silent: false,
        };

        assert!(opts.verbose);
        assert!(opts.quiet);
    }

    #[test]
    fn test_contents_options_complex_scenario() {
        // Test listing contents with specific version, lib only, and verbose
        let opts = ContentsOptions {
            gem_name: "rails".to_string(),
            version: Some("7.0.0".to_string()),
            all: false,
            spec_dir: None,
            lib_only: true,
            prefix: true,
            show_install_dir: false,
            verbose: true,
            quiet: false,
            silent: false,
        };

        assert_eq!(opts.gem_name, "rails");
        assert_eq!(opts.version, Some("7.0.0".to_string()));
        assert!(opts.lib_only);
        assert!(opts.prefix);
        assert!(opts.verbose);
    }
}
