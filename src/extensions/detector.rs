//! Extension detection
//!
//! Scans a gem directory to determine what type of extension it has (if any).
//!
//! Checks the gem's `ext/` directory for extconf.rb, Cargo.toml,
//! CMakeLists.txt, etc.

use super::types::ExtensionType;
use std::path::Path;

/// Detect what type of extension a gem has
///
/// Checks for the presence of extension markers:
/// - `ext/*/extconf.rb` -> C extension (most common)
/// - `Cargo.toml` -> Rust extension (newer gems)
/// - `ext/*/CMakeLists.txt` -> `CMake` extension
/// - Platform suffix in name -> Precompiled
/// - None of the above -> Pure Ruby
///
/// # Example
///
/// ```rust,ignore
/// use lode::extensions::detect_extension;
/// use std::path::Path;
///
/// let gem_dir = Path::new("vendor/bundle/gems/nokogiri-1.14.0");
/// let ext_type = detect_extension(gem_dir, "nokogiri", Some("arm64-darwin"));
///
/// // nokogiri has a C extension
/// assert!(ext_type.needs_building());
/// ```
#[must_use]
pub fn detect_extension(gem_dir: &Path, _gem_name: &str, platform: Option<&str>) -> ExtensionType {
    // Check if this is a platform-specific (precompiled) gem
    if let Some(plat) = platform
        && plat != "ruby"
        && !plat.is_empty()
    {
        return ExtensionType::Precompiled;
    }

    // Check for C extension (most common)
    // Look in ext/ directory for extconf.rb
    let ext_dir = gem_dir.join("ext");
    if ext_dir.exists() && ext_dir.is_dir() {
        // Check for extconf.rb directly in ext/ first (before iterating)
        let extconf = ext_dir.join("extconf.rb");
        if extconf.exists() {
            return ExtensionType::CExtension {
                ext_dir,
                extconf_path: extconf,
            };
        }

        // Some gems have ext/gem_name/extconf.rb
        // Scan subdirectories
        if let Ok(entries) = std::fs::read_dir(&ext_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let extconf = path.join("extconf.rb");
                    if extconf.exists() {
                        return ExtensionType::CExtension {
                            ext_dir: path,
                            extconf_path: extconf,
                        };
                    }

                    // Check for CMakeLists.txt
                    let cmake = path.join("CMakeLists.txt");
                    if cmake.exists() {
                        return ExtensionType::CMakeExtension { cmake_lists: cmake };
                    }
                }
            }
        }
    }

    // Check for Rust extension
    let cargo_toml = gem_dir.join("Cargo.toml");
    if cargo_toml.exists() {
        return ExtensionType::RustExtension { cargo_toml };
    }

    // No extension found - pure Ruby gem
    ExtensionType::None
}

/// Check if a gem name indicates it's precompiled (has platform suffix)
///
/// Examples:
/// - `nokogiri-1.14.0-arm64-darwin` -> true
/// - `nokogiri-1.14.0` -> false
/// - `pg-1.5.0-x86_64-linux` -> true
#[must_use]
pub fn has_platform_suffix(gem_name: &str) -> bool {
    // Common platform patterns
    let platforms = [
        "arm64-darwin",
        "x86_64-darwin",
        "x86_64-linux",
        "aarch64-linux",
        "x86-mingw32",
        "x64-mingw32",
        "java",
    ];

    platforms.iter().any(|p| gem_name.contains(p))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_gem(_name: &str, files: &[&str]) -> TempDir {
        let dir = TempDir::new().unwrap();

        for file in files {
            let file_path = dir.path().join(file);
            if let Some(parent) = file_path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(&file_path, "").unwrap();
        }

        dir
    }

    #[test]
    fn detect_c_extension() {
        let gem_dir = create_test_gem("nokogiri", &["ext/nokogiri/extconf.rb"]);

        let ext_type = detect_extension(gem_dir.path(), "nokogiri", None);

        assert!(matches!(ext_type, ExtensionType::CExtension { .. }));
        assert!(ext_type.needs_building());
    }

    #[test]
    fn detect_c_extension_in_root() {
        let gem_dir = create_test_gem("simple", &["ext/extconf.rb"]);

        let ext_type = detect_extension(gem_dir.path(), "simple", None);

        assert!(matches!(ext_type, ExtensionType::CExtension { .. }));
    }

    #[test]
    fn detect_rust_extension() {
        let gem_dir = create_test_gem("rust_gem", &["Cargo.toml"]);

        let ext_type = detect_extension(gem_dir.path(), "rust_gem", None);

        assert!(matches!(ext_type, ExtensionType::RustExtension { .. }));
        assert!(ext_type.needs_building());
    }

    #[test]
    fn detect_precompiled() {
        let gem_dir = create_test_gem("nokogiri", &["lib/nokogiri.rb"]);

        let ext_type = detect_extension(gem_dir.path(), "nokogiri", Some("arm64-darwin"));

        assert_eq!(ext_type, ExtensionType::Precompiled);
        assert!(!ext_type.needs_building());
    }

    #[test]
    fn detect_pure_ruby() {
        let gem_dir = create_test_gem("rack", &["lib/rack.rb"]);

        let ext_type = detect_extension(gem_dir.path(), "rack", None);

        assert_eq!(ext_type, ExtensionType::None);
        assert!(!ext_type.needs_building());
    }

    #[test]
    fn test_has_platform_suffix() {
        assert!(has_platform_suffix("nokogiri-1.14.0-arm64-darwin"));
        assert!(has_platform_suffix("pg-1.5.0-x86_64-linux"));
        assert!(!has_platform_suffix("rack-3.0.8"));
        assert!(!has_platform_suffix("rails-7.0.8"));
    }
}
