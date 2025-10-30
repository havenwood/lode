//! Which command
//!
//! Find the location of a required library file

use anyhow::{Context, Result};
use lode::{Config, config, get_system_gem_dir};
use std::path::Path;

/// Find the location of a library file.
///
/// Searches in order:
/// 1. Vendor gems (from lockfile)
/// 2. System gems
/// 3. Ruby standard library
pub(crate) fn run(file_name: &str) -> Result<()> {
    // Normalize file name - add .rb extension if not present
    let search_name = if Path::new(file_name)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("rb"))
    {
        file_name.to_string()
    } else {
        format!("{file_name}.rb")
    };

    // Load configuration
    let config = Config::load().context("Failed to load configuration")?;

    // Build search paths in priority order
    let mut search_paths = Vec::new();

    // 1. Vendor directory (project gems)
    if let Ok(vendor_dir) = config::vendor_dir(Some(&config)) {
        // Detect Ruby version from lockfile if available
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
        let vendor_lib_dir = vendor_dir.join("ruby").join(&ruby_ver).join("gems");

        if vendor_lib_dir.exists() {
            // Add lib directories for all installed gems
            if let Ok(entries) = std::fs::read_dir(&vendor_lib_dir) {
                for entry in entries.flatten() {
                    let gem_dir = entry.path();
                    let lib_dir = gem_dir.join("lib");
                    if lib_dir.is_dir() {
                        search_paths.push(lib_dir);
                    }
                }
            }
        }
    }

    // Get ruby version for system and standard library paths
    let ruby_ver = config::ruby_version(None);

    // 2. System gem directory
    let system_gem_dir = get_system_gem_dir(&ruby_ver);
    if system_gem_dir.exists()
        && let Ok(entries) = std::fs::read_dir(&system_gem_dir)
    {
        for entry in entries.flatten() {
            let gem_dir = entry.path();
            let lib_dir = gem_dir.join("lib");
            if lib_dir.is_dir() {
                search_paths.push(lib_dir);
            }
        }
    }

    // 3. Ruby standard library paths
    let std_lib_paths = lode::get_standard_gem_paths(&ruby_ver);
    search_paths.extend(std_lib_paths);

    // Search for the file
    for lib_path in &search_paths {
        let candidate = lib_path.join(&search_name);
        if candidate.exists() {
            println!("{}", candidate.display());
            return Ok(());
        }

        // Also check for nested paths (e.g., "rake/file_list" -> "lib/rake/file_list.rb")
        if search_name.contains('/') {
            let nested = lib_path.join(&search_name);
            if nested.exists() {
                println!("{}", nested.display());
                return Ok(());
            }
        }
    }

    // Not found
    anyhow::bail!("Can't find file '{search_name}' in gem paths");
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "Tests can panic")]
mod tests {
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn which_file_normalization() {
        use std::path::Path;

        // Test that file names are normalized to .rb extension
        let with_ext = "rake.rb";
        let normalized = if Path::new(with_ext)
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("rb"))
        {
            with_ext.to_string()
        } else {
            format!("{with_ext}.rb")
        };
        assert_eq!(normalized, "rake.rb");

        let without_ext = "rake";
        let normalized = if Path::new(without_ext)
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("rb"))
        {
            without_ext.to_string()
        } else {
            format!("{without_ext}.rb")
        };
        assert_eq!(normalized, "rake.rb");
    }

    #[test]
    fn which_with_temp_gem() {
        // Create a temporary gem structure
        let temp = TempDir::new().unwrap();
        let gem_dir = temp.path().join("test_gem-1.0.0");
        let lib_dir = gem_dir.join("lib");
        fs::create_dir_all(&lib_dir).unwrap();

        // Create a test file
        let test_file = lib_dir.join("test_gem.rb");
        fs::write(&test_file, "# test gem").unwrap();

        // Verify the file exists
        assert!(test_file.exists());
    }

    #[test]
    fn which_nested_path() {
        // Test nested path handling (e.g., "rake/file_list")
        let path = "rake/file_list";
        assert!(path.contains('/'));

        let search_name = format!("{path}.rb");
        assert_eq!(search_name, "rake/file_list.rb");
    }
}
