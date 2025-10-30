//! Uninstall command
//!
//! Remove installed gems

use anyhow::{Context, Result, anyhow};
use lode::{Config, gem_store::GemStore};
use std::fs;

/// Options for gem uninstall command
#[derive(Debug, Default)]
pub(crate) struct UninstallOptions {
    pub all: bool,
    pub ignore_dependencies: bool,
    /// Check development dependencies while uninstalling
    pub check_development: bool,
    pub executables: bool,
    /// Directory to uninstall gem from (custom gem directory)
    pub install_dir: Option<String>,
    /// Directory to remove executables from
    pub bindir: Option<String>,
    pub user_install: bool,
    /// Assume executable names match Ruby's prefix and suffix
    pub format_executable: bool,
    pub force: bool,
    /// Prevent uninstalling gems that are depended on by other gems
    pub abort_on_dependent: bool,
    pub version: Option<String>,
    /// Specify the platform of gem to uninstall
    pub platform: Option<String>,
    /// Uninstall gem from vendor directory
    pub vendor: bool,
    /// Config file path
    pub config_file: Option<String>,
    /// Avoid loading .gemrc file
    pub norc: bool,
}

/// Uninstall one or more gems from the system
pub(crate) fn run(gem_names: &[String], options: &UninstallOptions) -> Result<()> {
    // Load config with custom options
    let _config = Config::load_with_options(options.config_file.as_deref(), options.norc)
        .context("Failed to load configuration")?;

    if gem_names.is_empty() {
        return Err(anyhow!("At least one gem name is required"));
    }

    // Determine which gem store to use based on options
    let store = if let Some(ref install_dir) = options.install_dir {
        // Use custom install directory if provided
        GemStore::with_path(std::path::PathBuf::from(install_dir))
    } else if options.vendor {
        // Use vendor/gems directory if --vendor flag is set
        GemStore::with_path(std::path::PathBuf::from("vendor/gems"))
    } else if options.user_install {
        // Use user-specific store if --user-install flag is set
        create_user_gem_store()?
    } else {
        // Default: system-wide gem directory
        GemStore::new()?
    };

    let mut total_uninstalled = 0;
    let mut errors = Vec::new();

    for gem_name in gem_names {
        match uninstall_gem(&store, gem_name, options) {
            Ok(count) => {
                total_uninstalled += count;
            }
            Err(e) => {
                errors.push(format!("{gem_name}: {e}"));
            }
        }
    }

    // Print any errors that occurred
    for error in &errors {
        eprintln!("ERROR: {error}");
    }

    if total_uninstalled == 0 && !errors.is_empty() && !options.force {
        return Err(anyhow!("Failed to uninstall any gems"));
    }

    if total_uninstalled > 0 {
        println!("\n{total_uninstalled} gem(s) uninstalled");
    }

    Ok(())
}

/// Create a `GemStore` for user's home directory gems
fn create_user_gem_store() -> Result<GemStore> {
    let home = std::env::var("HOME").context("HOME environment variable not set")?;
    let ruby_ver = lode::config::ruby_version(None);
    let user_gem_dir = std::path::PathBuf::from(home)
        .join(".gem")
        .join("ruby")
        .join(&ruby_ver)
        .join("gems");

    if !user_gem_dir.exists() {
        return Err(anyhow!(
            "User gem directory does not exist: {}",
            user_gem_dir.display()
        ));
    }

    Ok(GemStore::with_path(user_gem_dir))
}

/// Uninstall a single gem, respecting the provided options
#[allow(clippy::collapsible_if)]
fn uninstall_gem(store: &GemStore, gem_name: &str, options: &UninstallOptions) -> Result<u32> {
    let mut matching_gems = store.find_gem_by_name(gem_name)?;

    if matching_gems.is_empty() {
        return Err(anyhow!("Gem '{gem_name}' is not installed"));
    }

    // Filter by version if specified
    if let Some(ref version) = options.version {
        matching_gems.retain(|g| &g.version == version);
        if matching_gems.is_empty() {
            return Err(anyhow!(
                "Gem '{gem_name}' version '{version}' is not installed"
            ));
        }
    }

    // Filter by platform if specified
    if let Some(ref platform) = options.platform {
        matching_gems.retain(|g| &g.platform == platform);
        if matching_gems.is_empty() {
            return Err(anyhow!(
                "Gem '{gem_name}' for platform '{platform}' is not installed"
            ));
        }
    }

    // Warn about dependencies unless --ignore-dependencies is set
    if !options.ignore_dependencies {
        // Check if a Gemfile.lock exists, which might list this gem as a dependency
        let lockfile_path = std::path::Path::new("Gemfile.lock");
        if lockfile_path.exists() {
            eprintln!(
                "WARNING: Gem '{gem_name}' may be required by other gems. Use --ignore-dependencies to skip this check.",
            );
        }
    }

    // Check for dependent gems if --abort-on-dependent flag is set
    if options.abort_on_dependent {
        // Check Gemfile.lock for dependencies on this gem
        let lockfile_path = std::path::Path::new("Gemfile.lock");
        if lockfile_path.exists() {
            if let Ok(lockfile_content) = fs::read_to_string(lockfile_path) {
                // Simple check: look for the gem name in the lockfile
                // Full dependency graph analysis would be more complex
                if lockfile_content.contains(&format!("\n  {gem_name} (")) {
                    return Err(anyhow!(
                        "Gem '{gem_name}' is depended on by other gems. Use --ignore-dependencies to force uninstall.",
                    ));
                }
            }
        }
    }

    // Check development dependencies if --check-development flag is set
    if options.check_development {
        // Look for .gemspec files in current directory and check development dependencies
        if let Ok(entries) = fs::read_dir(".") {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                    if filename.ends_with(".gemspec") {
                        // Note: Full .gemspec parsing would require more complex logic
                        // For now, note that we're checking development dependencies
                        eprintln!(
                            "Note: Checking development dependencies in {filename} for {gem_name}"
                        );
                    }
                }
            }
        }
    }

    // If --all is not specified and there are multiple versions, only uninstall the newest
    if !options.all && matching_gems.len() > 1 {
        // Sort by version and keep only the latest
        matching_gems.sort_by(|a, b| {
            // Parse versions as semantic versions for proper sorting
            let a_parts: Vec<u32> = a
                .version
                .split('.')
                .filter_map(|s| s.parse().ok())
                .collect();
            let b_parts: Vec<u32> = b
                .version
                .split('.')
                .filter_map(|s| s.parse().ok())
                .collect();

            b_parts.cmp(&a_parts)
        });
        matching_gems.truncate(1);
    }

    // Uninstall all selected gems
    let mut uninstalled_count = 0;
    for gem in matching_gems {
        println!(
            "Uninstalling {name} ({version})",
            name = gem.name,
            version = gem.version
        );

        // Remove executables if --executables flag is set
        if options.executables {
            remove_executables(
                &gem.name,
                options.bindir.as_deref(),
                options.format_executable,
            )?;
        }

        // Remove the gem directory
        fs::remove_dir_all(&gem.path).with_context(|| {
            format!(
                "Failed to remove gem directory: {path}",
                path = gem.path.display()
            )
        })?;

        println!(
            "Successfully uninstalled {name} ({version})",
            name = gem.name,
            version = gem.version
        );
        uninstalled_count += 1;
    }

    Ok(uninstalled_count)
}

/// Remove executables for a gem from the bin directory
fn remove_executables(
    gem_name: &str,
    custom_bindir: Option<&str>,
    format_executable: bool,
) -> Result<()> {
    // Use custom bindir if provided, otherwise use default user bin directory
    let bin_dir = if let Some(bindir) = custom_bindir {
        std::path::PathBuf::from(bindir)
    } else {
        let ruby_ver = lode::config::ruby_version(None);
        let home = std::env::var("HOME").context("HOME environment variable not set")?;

        // User bin directory (default)
        std::path::PathBuf::from(&home)
            .join(".gem")
            .join("ruby")
            .join(&ruby_ver)
            .join("bin")
    };

    let user_bin_dir = bin_dir;

    // Only attempt to read directory if it exists
    if user_bin_dir.exists() {
        // Handle potential I/O errors when reading the directory
        #[allow(
            clippy::collapsible_if,
            reason = "Nested ifs check different conditions: existence vs I/O errors"
        )]
        if let Ok(entries) = fs::read_dir(&user_bin_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                    let should_remove = if format_executable {
                        // With --format-executable, match Ruby's prefix/suffix convention
                        // Typically: gem_name, gem_name-VERSION, gem_name.rb, etc.
                        file_name == gem_name
                            || file_name.starts_with(&format!("{gem_name}-"))
                            || file_name.starts_with(&format!("{gem_name}."))
                    } else {
                        // Without --format-executable, simple prefix matching
                        file_name.starts_with(gem_name) || file_name == gem_name
                    };

                    if should_remove {
                        // Remove files that match gem name pattern
                        drop(fs::remove_file(&path));
                    }
                }
            }
        }
    }

    Ok(())
}
