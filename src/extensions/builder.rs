//! Extension Builder Orchestration
//!
//! Coordinates the building of native extensions. Detects the extension type
//! and delegates to the appropriate builder (similar to `bundle install` behavior
//! for gems with extensions).

use super::c_extension::CExtensionBuilder;
use super::cmake_extension::CMakeExtensionBuilder;
use super::detector::detect_extension;
use super::rust_extension::RustExtensionBuilder;
use super::types::{BuildResult, ExtensionType};
use std::path::Path;

/// Extension builder coordinator
///
/// High-level interface for building extensions. Handles detection, delegation,
/// and error handling for all extension types.
#[derive(Debug)]
pub struct ExtensionBuilder {
    /// Skip building extensions entirely
    skip_extensions: bool,
    /// Enable verbose output
    verbose: bool,
    /// Path to alternative `RbConfig` for cross-compilation
    rbconfig_path: Option<String>,
    /// C extension builder (lazy-initialized)
    c_builder: Option<CExtensionBuilder>,
    /// Rust extension builder (lazy-initialized)
    rust_builder: Option<RustExtensionBuilder>,
    /// `CMake` extension builder (lazy-initialized)
    cmake_builder: Option<CMakeExtensionBuilder>,
}

impl ExtensionBuilder {
    /// Create a new extension builder.
    #[must_use]
    pub const fn new(skip_extensions: bool, verbose: bool, rbconfig_path: Option<String>) -> Self {
        Self {
            skip_extensions,
            verbose,
            rbconfig_path,
            c_builder: None,
            rust_builder: None,
            cmake_builder: None,
        }
    }

    /// Build extension if needed
    ///
    /// Detects extension type and builds if necessary. Skips precompiled and pure Ruby gems.
    ///
    /// # Arguments
    /// * `gem_name` - Name of the gem
    /// * `gem_dir` - Directory containing the gem
    /// * `platform` - Platform string (e.g., "arm64-darwin", "ruby")
    ///
    /// # Returns
    /// `None` if no building needed, `Some(BuildResult)` if build attempted
    #[allow(clippy::too_many_lines)]
    #[must_use]
    pub fn build_if_needed(
        &mut self,
        gem_name: &str,
        gem_dir: &Path,
        platform: Option<&str>,
    ) -> Option<BuildResult> {
        // Skip if disabled
        if self.skip_extensions {
            if self.verbose {
                println!("Skipping extensions (--skip-extensions enabled)");
            }
            return None;
        }

        // Detect extension type
        let ext_type = detect_extension(gem_dir, gem_name, platform);

        if self.verbose {
            println!("Extension type for {gem_name}: {}", ext_type.description());
        }

        // Build based on type
        match ext_type {
            ExtensionType::CExtension {
                ext_dir,
                extconf_path,
            } => {
                if self.verbose {
                    println!("Building C extension for {gem_name}...");
                }

                // Lazy-initialize C builder
                if self.c_builder.is_none() {
                    match CExtensionBuilder::new(self.verbose) {
                        Ok(builder) => self.c_builder = Some(builder),
                        Err(e) => {
                            return Some(BuildResult::failure(
                                gem_name.to_string(),
                                std::time::Duration::from_secs(0),
                                format!("Failed to initialize C extension builder: {e}"),
                                String::new(),
                            ));
                        }
                    }
                }

                // Build with C builder
                // Safety: c_builder is initialized above, so this is guaranteed to exist
                self.c_builder.as_ref().map_or_else(
                    || {
                        // This should never happen, but handle gracefully
                        Some(BuildResult::failure(
                            gem_name.to_string(),
                            std::time::Duration::from_secs(0),
                            "C extension builder not initialized".to_string(),
                            String::new(),
                        ))
                    },
                    |builder| {
                        Some(builder.build(
                            gem_name,
                            &ext_dir,
                            &extconf_path,
                            gem_dir,
                            self.rbconfig_path.as_deref(),
                        ))
                    },
                )
            }

            ExtensionType::RustExtension { cargo_toml } => {
                if self.verbose {
                    println!("Building Rust extension for {gem_name}...");
                }

                // Lazy-initialize Rust builder
                if self.rust_builder.is_none() {
                    match RustExtensionBuilder::new(self.verbose) {
                        Ok(builder) => self.rust_builder = Some(builder),
                        Err(e) => {
                            return Some(BuildResult::failure(
                                gem_name.to_string(),
                                std::time::Duration::from_secs(0),
                                format!("Failed to initialize Rust extension builder: {e}"),
                                String::new(),
                            ));
                        }
                    }
                }

                // Build with Rust builder
                self.rust_builder.as_ref().map_or_else(
                    || {
                        Some(BuildResult::failure(
                            gem_name.to_string(),
                            std::time::Duration::from_secs(0),
                            "Rust extension builder not initialized".to_string(),
                            String::new(),
                        ))
                    },
                    |builder| {
                        builder
                            .build(gem_name, gem_dir, &cargo_toml)
                            .ok()
                            .or_else(|| {
                                Some(BuildResult::failure(
                                    gem_name.to_string(),
                                    std::time::Duration::from_secs(0),
                                    "Rust extension build failed".to_string(),
                                    String::new(),
                                ))
                            })
                    },
                )
            }

            ExtensionType::CMakeExtension { cmake_lists } => {
                if self.verbose {
                    println!("Building CMake extension for {gem_name}...");
                }

                // Lazy-initialize CMake builder
                if self.cmake_builder.is_none() {
                    match CMakeExtensionBuilder::new(self.verbose) {
                        Ok(builder) => self.cmake_builder = Some(builder),
                        Err(e) => {
                            return Some(BuildResult::failure(
                                gem_name.to_string(),
                                std::time::Duration::from_secs(0),
                                format!("Failed to initialize CMake extension builder: {e}"),
                                String::new(),
                            ));
                        }
                    }
                }

                // Build with CMake builder
                let Some(ext_dir) = cmake_lists.parent() else {
                    return Some(BuildResult::failure(
                        gem_name.to_string(),
                        std::time::Duration::from_secs(0),
                        "Failed to get parent directory of CMakeLists.txt".to_string(),
                        String::new(),
                    ));
                };

                self.cmake_builder.as_ref().map_or_else(
                    || {
                        Some(BuildResult::failure(
                            gem_name.to_string(),
                            std::time::Duration::from_secs(0),
                            "CMake extension builder not initialized".to_string(),
                            String::new(),
                        ))
                    },
                    |builder| {
                        builder.build(gem_name, ext_dir, gem_dir).ok().or_else(|| {
                            Some(BuildResult::failure(
                                gem_name.to_string(),
                                std::time::Duration::from_secs(0),
                                "CMake extension build failed".to_string(),
                                String::new(),
                            ))
                        })
                    },
                )
            }

            ExtensionType::Precompiled => {
                // No building needed - already compiled
                if self.verbose {
                    println!("{gem_name} is precompiled, no build needed");
                }
                None
            }

            ExtensionType::None => {
                // Pure Ruby gem - no extension to build
                None
            }
        }
    }

    /// Build extensions for multiple gems in parallel
    ///
    /// Useful for batch building during install. Currently builds sequentially;
    /// parallel support coming soon.
    ///
    /// # Arguments
    /// * `gems` - List of (`gem_name`, `gem_dir`, `platform`) tuples
    ///
    /// # Returns
    /// Vector of build results (only for gems that needed building)
    #[must_use]
    pub fn build_many(&mut self, gems: &[(&str, &Path, Option<&str>)]) -> Vec<BuildResult> {
        let mut results = Vec::new();

        for (gem_name, gem_dir, platform) in gems {
            if let Some(result) = self.build_if_needed(gem_name, gem_dir, *platform) {
                results.push(result);
            }
        }

        results
    }

    /// Get summary statistics
    ///
    /// Reports build results to the user with formatted output.
    ///
    /// # Arguments
    /// * `results` - Build results from `build_many()` or multiple `build_if_needed()` calls
    ///
    /// # Returns
    /// (`successful_count`, `failed_count`, `total_duration`)
    #[must_use]
    pub fn summarize(results: &[BuildResult]) -> (usize, usize, std::time::Duration) {
        let successful = results.iter().filter(|r| r.success).count();
        let failed = results.len() - successful;
        let total_duration = results.iter().map(|r| r.duration).sum();

        (successful, failed, total_duration)
    }
}

/// Build extensions for a list of gems (convenience function)
///
/// Simplest API for building extensions. Pass your gem list and it handles
/// everything automatically.
///
/// # Example
///
/// ```no_run
/// use lode::extensions::build_extensions;
/// use std::path::Path;
///
/// let gems = vec![
///     ("nokogiri", Path::new("vendor/gems/nokogiri-1.14.0"), Some("ruby")),
///     ("pg", Path::new("vendor/gems/pg-1.5.0"), Some("ruby")),
/// ];
///
/// let results = build_extensions(&gems, false, true);
///
/// for result in results {
///     if result.success {
///         println!(" Built {} in {:?}", result.gem_name, result.duration);
///     } else {
///         eprintln!(" Failed to build {}: {}", result.gem_name, result.error.unwrap());
///     }
/// }
/// ```
pub fn build_extensions(
    gems: &[(&str, &Path, Option<&str>)],
    skip_extensions: bool,
    verbose: bool,
) -> Vec<BuildResult> {
    let mut builder = ExtensionBuilder::new(skip_extensions, verbose, None);
    builder.build_many(gems)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_gem_with_c_extension() -> TempDir {
        let dir = TempDir::new().unwrap();
        let ext_dir = dir.path().join("ext").join("test_gem");

        fs::create_dir_all(&ext_dir).unwrap();

        // Create a minimal extconf.rb
        fs::write(
            ext_dir.join("extconf.rb"),
            "# Minimal extconf.rb\nrequire 'mkmf'\ncreate_makefile('test_gem')\n",
        )
        .unwrap();

        dir
    }

    fn create_pure_ruby_gem() -> TempDir {
        let dir = TempDir::new().unwrap();
        let lib_dir = dir.path().join("lib");

        fs::create_dir_all(&lib_dir).unwrap();
        fs::write(lib_dir.join("test_gem.rb"), "# Pure Ruby gem").unwrap();

        dir
    }

    #[test]
    fn builder_creation() {
        let builder = ExtensionBuilder::new(false, false, None);
        assert!(!builder.skip_extensions);
        assert!(!builder.verbose);
        assert!(builder.c_builder.is_none());
    }

    #[test]
    fn skip_extensions() {
        let mut builder = ExtensionBuilder::new(true, false, None);
        let gem_dir = create_gem_with_c_extension();

        let result = builder.build_if_needed("test_gem", gem_dir.path(), Some("ruby"));

        assert!(
            result.is_none(),
            "Should skip building when skip_extensions is true"
        );
    }

    #[test]
    fn pure_ruby_gem() {
        let mut builder = ExtensionBuilder::new(false, false, None);
        let gem_dir = create_pure_ruby_gem();

        let result = builder.build_if_needed("test_gem", gem_dir.path(), Some("ruby"));

        assert!(result.is_none(), "Pure Ruby gems should not trigger builds");
    }

    #[test]
    fn precompiled_gem() {
        let mut builder = ExtensionBuilder::new(false, false, None);
        let gem_dir = create_pure_ruby_gem(); // Doesn't matter for precompiled

        let result = builder.build_if_needed("test_gem", gem_dir.path(), Some("arm64-darwin"));

        assert!(
            result.is_none(),
            "Precompiled gems should not trigger builds"
        );
    }

    #[test]
    fn test_build_many() {
        let mut builder = ExtensionBuilder::new(false, false, None);

        let first_gem = create_pure_ruby_gem();
        let second_gem = create_pure_ruby_gem();

        let test_gems = vec![
            ("gem1", first_gem.path(), Some("ruby")),
            ("gem2", second_gem.path(), Some("ruby")),
        ];

        let results = builder.build_many(&test_gems);

        assert_eq!(
            results.len(),
            0,
            "No extensions should be built for pure Ruby gems"
        );
    }

    #[test]
    fn summarize_empty() {
        let results = vec![];
        let (successful, failed, duration) = ExtensionBuilder::summarize(&results);

        assert_eq!(successful, 0);
        assert_eq!(failed, 0);
        assert_eq!(duration, std::time::Duration::from_secs(0));
    }

    #[test]
    fn summarize_mixed() {
        let results = vec![
            BuildResult::success(
                "gem1".to_string(),
                std::time::Duration::from_secs(1),
                "output".to_string(),
            ),
            BuildResult::failure(
                "gem2".to_string(),
                std::time::Duration::from_secs(2),
                "error".to_string(),
                "output".to_string(),
            ),
            BuildResult::success(
                "gem3".to_string(),
                std::time::Duration::from_secs(3),
                "output".to_string(),
            ),
        ];

        let (successful, failed, duration) = ExtensionBuilder::summarize(&results);

        assert_eq!(successful, 2);
        assert_eq!(failed, 1);
        assert_eq!(duration, std::time::Duration::from_secs(6));
    }
}
