//! Ruby environment detection and version management
//!
//! Detects Ruby versions, engines, and system gem directories.

use crate::gemfile::Gemfile;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Ruby engine types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RubyEngine {
    /// Standard CRuby/MRI
    Mri,
    /// `JRuby` (Java implementation)
    JRuby,
    /// `TruffleRuby` (`GraalVM`)
    TruffleRuby,
    /// mruby (embedded)
    MRuby,
    /// Unknown or custom engine
    Unknown(String),
}

impl std::str::FromStr for RubyEngine {
    type Err = std::convert::Infallible;

    /// Parse engine from string (always succeeds; unknown engines return `Unknown` variant).
    ///
    /// # Examples
    ///
    /// ```
    /// use lode::ruby::RubyEngine;
    /// use std::str::FromStr;
    ///
    /// assert_eq!(RubyEngine::from_str("mri").unwrap(), RubyEngine::Mri);
    /// assert_eq!(RubyEngine::from_str("custom").unwrap(), RubyEngine::Unknown("custom".into()));
    /// ```
    fn from_str(name: &str) -> Result<Self, Self::Err> {
        let normalized = name.to_lowercase().trim().to_string();

        Ok(match normalized.as_str() {
            "ruby" | "cruby" | "mri" => Self::Mri,
            name if name.starts_with("jruby") => Self::JRuby,
            name if name.starts_with("truffleruby") => Self::TruffleRuby,
            name if name.starts_with("mruby") => Self::MRuby,
            _ => Self::Unknown(normalized),
        })
    }
}

impl RubyEngine {
    /// Check if engine supports native C extensions
    #[inline]
    pub const fn supports_native_extensions(&self) -> bool {
        matches!(self, Self::Mri | Self::TruffleRuby)
    }

    /// Get platform suffix for this engine (e.g., "java" for `JRuby`)
    #[inline]
    pub const fn platform_suffix(&self) -> Option<&str> {
        match self {
            Self::JRuby => Some("java"),
            _ => None,
        }
    }

    /// Get engine name as string
    #[inline]
    pub fn as_str(&self) -> &str {
        match self {
            Self::Mri => "mri",
            Self::JRuby => "jruby",
            Self::TruffleRuby => "truffleruby",
            Self::MRuby => "mruby",
            Self::Unknown(name) => name,
        }
    }
}

impl std::fmt::Display for RubyEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Detect Ruby engine from environment or command
///
/// Never panics - always returns a valid engine (defaults to Mri if detection fails).
#[must_use]
#[allow(clippy::missing_panics_doc)]
pub fn detect_engine() -> RubyEngine {
    use std::str::FromStr;

    // Try RUBY_ENGINE environment variable first (fast)
    if let Ok(engine) = env::var("RUBY_ENGINE") {
        return RubyEngine::from_str(&engine).expect("infallible error type should never occur");
    }

    // Try running Ruby command
    detect_engine_from_command().unwrap_or(RubyEngine::Mri)
}

/// Detect engine by running ruby command
fn detect_engine_from_command() -> Option<RubyEngine> {
    use std::str::FromStr;

    let output = Command::new("ruby")
        .args(["-e", "puts RUBY_ENGINE"])
        .output()
        .ok()?;

    if output.status.success() {
        let engine_name = String::from_utf8_lossy(&output.stdout);
        Some(
            RubyEngine::from_str(engine_name.trim())
                .expect("infallible error type should never occur"),
        )
    } else {
        None
    }
}

/// Detect engine from platform string (e.g., "java" -> `JRuby`)
#[must_use]
pub fn detect_engine_from_platform(platform: &str) -> RubyEngine {
    let platform_lower = platform.to_lowercase();

    if platform_lower == "java" {
        RubyEngine::JRuby
    } else {
        RubyEngine::Mri
    }
}

/// Convert version string to major.minor.0 format for Bundler compatibility
///
/// Examples:
/// - "3" -> "3.0.0"
/// - "3.4" -> "3.4.0"
/// - "3.4.1" -> "3.4.0"
/// - "3.4.1p194" -> "3.4.0"
#[must_use]
pub fn to_major_minor(version: &str) -> String {
    let version = version.trim();

    // Handle empty string
    if version.is_empty() {
        return "0.0.0".to_string();
    }

    // Remove patchlevel suffix (p0, p194, etc)
    let version = version.find('p').map_or(version, |idx| &version[..idx]);

    let parts: Vec<&str> = version.split('.').take(2).collect();

    match parts.as_slice() {
        [] => "0.0.0".to_string(),
        [major] => {
            let major = major.trim();
            if major.is_empty() {
                "0.0.0".to_string()
            } else {
                format!("{major}.0.0")
            }
        }
        [major, minor, ..] => format!("{major}.{minor}.0"),
    }
}

/// Normalize Ruby version constraint to usable version
///
/// Examples:
/// - "3.4.0" -> "3.4.0"
/// - ">= 3.0.0" -> "3.0.0"
/// - "~> 3.3" -> "3.3.0"
#[must_use]
pub fn normalize_ruby_version(constraint: &str) -> String {
    let constraint = constraint
        .trim()
        .trim_start_matches(">=")
        .trim_start_matches("~>")
        .trim_start_matches('>')
        .trim();

    to_major_minor(constraint)
}

/// Parse Ruby version string into semantic version
///
/// Handles various formats including:
/// - "ruby 3.3.0p0" -> "3.3.0"
/// - "3.4.1p194" -> "3.4.1"
/// - "3.4.0" -> "3.4.0"
///
/// Returns the version part before any patchlevel suffix (p0, p194, etc)
#[must_use]
pub fn parse_ruby_version_string(version_str: &str) -> String {
    version_str
        .trim()
        .trim_start_matches("ruby ")
        .split('p')
        .next()
        .unwrap_or(version_str)
        .trim()
        .to_string()
}

/// Detect Ruby version from Gemfile.lock RUBY VERSION section
pub fn detect_ruby_version_from_lockfile<P: AsRef<Path>>(lockfile_path: P) -> Option<String> {
    let content = fs::read_to_string(lockfile_path).ok()?;
    let lines: Vec<&str> = content.lines().collect();

    let mut in_ruby_section = false;

    for line in lines {
        let trimmed = line.trim();

        // Look for "RUBY VERSION" section
        if trimmed == "RUBY VERSION" {
            in_ruby_section = true;
            continue;
        }

        // Parse "   ruby 3.4.0p0" or "   ruby 3.4.0"
        if in_ruby_section && trimmed.starts_with("ruby ") {
            let version_str = trimmed.trim_start_matches("ruby ").trim();
            return Some(to_major_minor(version_str));
        }

        // Exit Ruby section if we hit another section
        if in_ruby_section && !trimmed.is_empty() && !trimmed.starts_with("ruby ") {
            break;
        }
    }

    None
}

/// Get standard gem paths for the current OS and Ruby version
#[must_use]
pub fn get_standard_gem_paths(ruby_version: &str) -> Vec<PathBuf> {
    let mut paths = Vec::new();

    #[cfg(target_os = "macos")]
    {
        paths.push(PathBuf::from(format!("/Library/Ruby/Gems/{ruby_version}")));
        paths.push(PathBuf::from(format!(
            "/opt/homebrew/lib/ruby/gems/{ruby_version}"
        )));
        paths.push(PathBuf::from(format!(
            "/usr/local/lib/ruby/gems/{ruby_version}"
        )));
    }

    #[cfg(target_os = "linux")]
    {
        paths.push(PathBuf::from(format!("/usr/lib/ruby/gems/{ruby_version}")));
        paths.push(PathBuf::from(format!(
            "/usr/local/lib/ruby/gems/{ruby_version}"
        )));
        paths.push(PathBuf::from(format!(
            "/usr/lib64/ruby/gems/{ruby_version}"
        )));
    }

    #[cfg(target_os = "windows")]
    {
        let program_files =
            env::var("ProgramFiles").unwrap_or_else(|_| "C:\\Program Files".to_string());
        let version_no_dots = ruby_version.replace('.', "");

        paths.push(PathBuf::from(format!(
            "C:\\Ruby{version_no_dots}\\lib\\ruby\\gems\\{ruby_version}"
        )));
        paths.push(PathBuf::from(format!(
            "{program_files}\\Ruby\\lib\\ruby\\gems\\{ruby_version}"
        )));
    }

    paths
}

/// Get system gem directory: `GEM_HOME` env -> OS paths -> user gem dir -> gem command.
#[must_use]
pub fn get_system_gem_dir(ruby_version: &str) -> PathBuf {
    // 1. Check GEM_HOME environment variable
    if let Ok(gem_home) = env::var("GEM_HOME") {
        let path = PathBuf::from(&gem_home);
        if path.is_dir() {
            return path;
        }
    }

    // 2. Try standard OS-specific gem paths
    for path in get_standard_gem_paths(ruby_version) {
        if path.is_dir() {
            return path;
        }
    }

    // 3. Try user gem directory
    if let Some(home_dir) = dirs::home_dir() {
        let user_gem_dir = home_dir
            .join(".gem")
            .join("ruby")
            .join(ruby_version)
            .join("gems");
        // Return even if doesn't exist - will be created during install
        return user_gem_dir;
    }

    // 4. Last resort: try `gem environment gemdir`
    if let Ok(output) = Command::new("gem").args(["environment", "gemdir"]).output()
        && output.status.success()
    {
        let gem_dir = String::from_utf8_lossy(&output.stdout);
        let gem_dir = gem_dir.trim();
        if !gem_dir.is_empty() {
            return PathBuf::from(gem_dir).join("gems");
        }
    }

    // Should never reach here due to step 3 always returning
    PathBuf::from("/tmp/gems")
}

/// Detect Ruby version with priority: Gemfile.lock -> Gemfile -> default
pub fn detect_ruby_version<P: AsRef<Path>>(
    lockfile_path: Option<P>,
    gemfile_path: Option<P>,
    default_version: &str,
) -> String {
    // 1. Try Gemfile.lock RUBY VERSION
    if let Some(lockfile) = lockfile_path
        && let Some(version) = detect_ruby_version_from_lockfile(&lockfile)
    {
        return version;
    }

    // 2. Try Gemfile ruby directive
    if let Some(gemfile) = gemfile_path
        && let Ok(parsed_gemfile) = Gemfile::parse_file(&gemfile)
        && let Some(version) = parsed_gemfile.ruby_version
    {
        return version;
    }

    // 3. Fallback to default
    default_version.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_to_major_minor() {
        assert_eq!(to_major_minor("3"), "3.0.0");
        assert_eq!(to_major_minor("3.4"), "3.4.0");
        assert_eq!(to_major_minor("3.4.1"), "3.4.0");
        assert_eq!(to_major_minor("3.4.1p194"), "3.4.0");
        assert_eq!(to_major_minor("2.7.8p225"), "2.7.0");
    }

    #[test]
    fn to_major_minor_edge_cases() {
        assert_eq!(to_major_minor(""), "0.0.0");
        assert_eq!(to_major_minor("   3.4  "), "3.4.0");
        assert_eq!(to_major_minor("3.4.1.2"), "3.4.0");
    }

    #[test]
    fn test_normalize_ruby_version() {
        assert_eq!(normalize_ruby_version("3.4.0"), "3.4.0");
        assert_eq!(normalize_ruby_version(">= 3.0.0"), "3.0.0");
        assert_eq!(normalize_ruby_version("~> 3.3"), "3.3.0");
        assert_eq!(normalize_ruby_version("> 2.7"), "2.7.0");
        assert_eq!(normalize_ruby_version("  >= 3.4.0  "), "3.4.0");
    }

    #[test]
    fn test_detect_ruby_version_from_lockfile() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "GEM").unwrap();
        writeln!(file, "  remote: https://rubygems.org/").unwrap();
        writeln!(file, "  specs:").unwrap();
        writeln!(file, "    rack (3.0.8)").unwrap();
        writeln!(file).unwrap();
        writeln!(file, "PLATFORMS").unwrap();
        writeln!(file, "  ruby").unwrap();
        writeln!(file).unwrap();
        writeln!(file, "RUBY VERSION").unwrap();
        writeln!(file, "   ruby 3.4.1p0").unwrap();
        writeln!(file).unwrap();
        writeln!(file, "BUNDLED WITH").unwrap();
        writeln!(file, "   2.5.23").unwrap();
        file.flush().unwrap();

        let version = detect_ruby_version_from_lockfile(file.path());
        assert_eq!(version, Some("3.4.0".to_string()));
    }

    #[test]
    fn detect_ruby_version_from_lockfile_nonexistent() {
        let version = detect_ruby_version_from_lockfile("/nonexistent/lockfile");
        assert_eq!(version, None);
    }

    #[test]
    fn detect_ruby_version_from_lockfile_no_ruby_section() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "GEM").unwrap();
        writeln!(file, "  specs:").unwrap();
        writeln!(file, "    rack (3.0.8)").unwrap();
        file.flush().unwrap();

        let version = detect_ruby_version_from_lockfile(file.path());
        assert_eq!(version, None);
    }

    #[test]
    fn detect_ruby_version_priority() {
        let mut lockfile = NamedTempFile::new().unwrap();
        writeln!(lockfile, "RUBY VERSION").unwrap();
        writeln!(lockfile, "   ruby 3.4.0").unwrap();
        lockfile.flush().unwrap();

        let version = detect_ruby_version(Some(lockfile.path()), None::<&Path>, "3.3.0");
        assert_eq!(version, "3.4.0");
    }

    #[test]
    fn detect_ruby_version_from_gemfile() {
        let mut gemfile = NamedTempFile::new().unwrap();
        writeln!(gemfile, "source 'https://rubygems.org'").unwrap();
        writeln!(gemfile, "ruby '3.2.1'").unwrap();
        writeln!(gemfile, "gem 'rails'").unwrap();
        gemfile.flush().unwrap();

        let version = detect_ruby_version(None::<&Path>, Some(gemfile.path()), "3.3.0");
        assert_eq!(version, "3.2.1");
    }

    #[test]
    fn detect_ruby_version_lockfile_has_priority_over_gemfile() {
        let mut lockfile = NamedTempFile::new().unwrap();
        writeln!(lockfile, "RUBY VERSION").unwrap();
        writeln!(lockfile, "   ruby 3.4.0").unwrap();
        lockfile.flush().unwrap();

        let mut gemfile = NamedTempFile::new().unwrap();
        writeln!(gemfile, "source 'https://rubygems.org'").unwrap();
        writeln!(gemfile, "ruby '3.2.1'").unwrap();
        writeln!(gemfile, "gem 'rails'").unwrap();
        gemfile.flush().unwrap();

        let version = detect_ruby_version(Some(lockfile.path()), Some(gemfile.path()), "3.3.0");
        assert_eq!(version, "3.4.0");
    }

    #[test]
    fn detect_ruby_version_default_fallback() {
        let version = detect_ruby_version(None::<&Path>, None::<&Path>, "3.3.0");
        assert_eq!(version, "3.3.0");
    }

    #[test]
    fn engine_from_str() {
        use std::str::FromStr;

        assert_eq!(RubyEngine::from_str("mri").unwrap(), RubyEngine::Mri);
        assert_eq!(RubyEngine::from_str("ruby").unwrap(), RubyEngine::Mri);
        assert_eq!(RubyEngine::from_str("cruby").unwrap(), RubyEngine::Mri);
        assert_eq!(RubyEngine::from_str("jruby").unwrap(), RubyEngine::JRuby);
        assert_eq!(
            RubyEngine::from_str("truffleruby").unwrap(),
            RubyEngine::TruffleRuby
        );
        assert_eq!(RubyEngine::from_str("mruby").unwrap(), RubyEngine::MRuby);

        match RubyEngine::from_str("custom").unwrap() {
            RubyEngine::Unknown(name) => assert_eq!(name, "custom"),
            _ => unreachable!("Expected Unknown engine"),
        }
    }

    #[test]
    fn engine_supports_native_extensions() {
        assert!(RubyEngine::Mri.supports_native_extensions());
        assert!(RubyEngine::TruffleRuby.supports_native_extensions());
        assert!(!RubyEngine::JRuby.supports_native_extensions());
        assert!(!RubyEngine::MRuby.supports_native_extensions());
        assert!(!RubyEngine::Unknown("custom".to_string()).supports_native_extensions());
    }

    #[test]
    fn engine_platform_suffix() {
        assert_eq!(RubyEngine::JRuby.platform_suffix(), Some("java"));
        assert_eq!(RubyEngine::Mri.platform_suffix(), None);
        assert_eq!(RubyEngine::TruffleRuby.platform_suffix(), None);
        assert_eq!(RubyEngine::MRuby.platform_suffix(), None);
    }

    #[test]
    fn engine_to_string() {
        assert_eq!(RubyEngine::Mri.to_string(), "mri");
        assert_eq!(RubyEngine::JRuby.to_string(), "jruby");
        assert_eq!(RubyEngine::TruffleRuby.to_string(), "truffleruby");
        assert_eq!(RubyEngine::MRuby.to_string(), "mruby");
    }

    #[test]
    fn test_detect_engine_from_platform() {
        assert_eq!(detect_engine_from_platform("java"), RubyEngine::JRuby);
        assert_eq!(detect_engine_from_platform("JAVA"), RubyEngine::JRuby);
        assert_eq!(
            detect_engine_from_platform("x86_64-darwin"),
            RubyEngine::Mri
        );
        assert_eq!(detect_engine_from_platform("x86_64-linux"), RubyEngine::Mri);
    }

    #[test]
    fn test_get_standard_gem_paths() {
        let paths = get_standard_gem_paths("3.4.0");
        assert!(!paths.is_empty());
        for path in &paths {
            assert!(path.to_string_lossy().contains("3.4.0"));
        }
    }

    #[test]
    fn test_get_system_gem_dir() {
        let gem_dir = get_system_gem_dir("3.4.0");
        assert!(!gem_dir.as_os_str().is_empty());
        let path_str = gem_dir.to_string_lossy();
        assert!(path_str.contains("3.4.0") || path_str.contains("gem"));
    }

    #[test]
    fn detect_engine_from_environment() {
        let engine = detect_engine();
        assert!(matches!(
            engine,
            RubyEngine::Mri
                | RubyEngine::JRuby
                | RubyEngine::TruffleRuby
                | RubyEngine::MRuby
                | RubyEngine::Unknown(_)
        ));
    }

    mod parse_ruby_version_string {
        use super::*;

        #[test]
        fn basic_version() {
            assert_eq!(parse_ruby_version_string("3.4.1"), "3.4.1");
        }

        #[test]
        fn with_patchlevel() {
            assert_eq!(parse_ruby_version_string("3.4.1p194"), "3.4.1");
        }

        #[test]
        fn ruby_prefix() {
            assert_eq!(parse_ruby_version_string("ruby 3.3.0"), "3.3.0");
        }

        #[test]
        fn ruby_prefix_with_patchlevel() {
            assert_eq!(parse_ruby_version_string("ruby 3.3.0p0"), "3.3.0");
        }

        #[test]
        fn with_whitespace() {
            assert_eq!(parse_ruby_version_string("  3.4.1  "), "3.4.1");
        }

        #[test]
        fn empty_string() {
            assert_eq!(parse_ruby_version_string(""), "");
        }

        #[test]
        fn only_patchlevel() {
            assert_eq!(parse_ruby_version_string("p194"), "");
        }
    }

    mod engine_methods {
        use super::*;

        #[test]
        fn as_str_mri() {
            assert_eq!(RubyEngine::Mri.as_str(), "mri");
        }

        #[test]
        fn as_str_jruby() {
            assert_eq!(RubyEngine::JRuby.as_str(), "jruby");
        }

        #[test]
        fn as_str_truffleruby() {
            assert_eq!(RubyEngine::TruffleRuby.as_str(), "truffleruby");
        }

        #[test]
        fn as_str_unknown() {
            assert_eq!(RubyEngine::Unknown("custom".to_string()).as_str(), "custom");
        }

        #[test]
        fn parse_case_insensitive() {
            use std::str::FromStr;
            assert_eq!(RubyEngine::from_str("MRI").unwrap(), RubyEngine::Mri);
            assert_eq!(RubyEngine::from_str("JRUBY").unwrap(), RubyEngine::JRuby);
            assert_eq!(
                RubyEngine::from_str("TruffleRuby").unwrap(),
                RubyEngine::TruffleRuby
            );
        }

        #[test]
        fn parse_with_whitespace() {
            use std::str::FromStr;
            assert_eq!(RubyEngine::from_str("  mri  ").unwrap(), RubyEngine::Mri);
        }

        #[test]
        fn parse_jruby_variant() {
            use std::str::FromStr;
            assert_eq!(
                RubyEngine::from_str("jruby-9.4").unwrap(),
                RubyEngine::JRuby
            );
        }

        #[test]
        fn parse_truffleruby_variant() {
            use std::str::FromStr;
            assert_eq!(
                RubyEngine::from_str("truffleruby-24.1").unwrap(),
                RubyEngine::TruffleRuby
            );
        }
    }

    mod gemfile_ruby_directive {
        use super::*;
        use std::io::Write;

        #[test]
        fn parse_ruby_with_quoted_version() {
            let mut gemfile = NamedTempFile::new().unwrap();
            writeln!(gemfile, "ruby '2.7.0'").unwrap();
            gemfile.flush().unwrap();

            let version = detect_ruby_version(None::<&Path>, Some(gemfile.path()), "3.3.0");
            assert_eq!(version, "2.7.0");
        }

        #[test]
        fn parse_ruby_with_double_quoted_version() {
            let mut gemfile = NamedTempFile::new().unwrap();
            writeln!(gemfile, "ruby \"3.1.4\"").unwrap();
            gemfile.flush().unwrap();

            let version = detect_ruby_version(None::<&Path>, Some(gemfile.path()), "3.3.0");
            assert_eq!(version, "3.1.4");
        }

        #[test]
        fn no_ruby_directive_uses_default() {
            let mut gemfile = NamedTempFile::new().unwrap();
            writeln!(gemfile, "source 'https://rubygems.org'").unwrap();
            writeln!(gemfile, "gem 'rails'").unwrap();
            gemfile.flush().unwrap();

            let version = detect_ruby_version(None::<&Path>, Some(gemfile.path()), "3.3.0");
            assert_eq!(version, "3.3.0");
        }

        #[test]
        fn empty_gemfile_uses_default() {
            let mut gemfile = NamedTempFile::new().unwrap();
            writeln!(gemfile).unwrap();
            gemfile.flush().unwrap();

            let version = detect_ruby_version(None::<&Path>, Some(gemfile.path()), "3.3.0");
            assert_eq!(version, "3.3.0");
        }

        #[test]
        fn lockfile_takes_precedence_over_gemfile() {
            let mut lockfile = NamedTempFile::new().unwrap();
            writeln!(lockfile, "RUBY VERSION").unwrap();
            writeln!(lockfile, "   ruby 3.5.0").unwrap();
            lockfile.flush().unwrap();

            let mut gemfile = NamedTempFile::new().unwrap();
            writeln!(gemfile, "ruby '3.2.1'").unwrap();
            gemfile.flush().unwrap();

            let version = detect_ruby_version(Some(lockfile.path()), Some(gemfile.path()), "3.3.0");
            assert_eq!(version, "3.5.0");
        }
    }

    mod lockfile_parsing {
        use super::*;
        use std::io::Write;

        #[test]
        fn multiple_ruby_lines_uses_first() {
            let mut file = NamedTempFile::new().unwrap();
            writeln!(file, "RUBY VERSION").unwrap();
            writeln!(file, "   ruby 3.4.0").unwrap();
            writeln!(file, "   ruby 3.5.0").unwrap();
            file.flush().unwrap();

            let version = detect_ruby_version_from_lockfile(file.path());
            assert_eq!(version, Some("3.4.0".to_string()));
        }

        #[test]
        fn with_patchlevel() {
            let mut file = NamedTempFile::new().unwrap();
            writeln!(file, "RUBY VERSION").unwrap();
            writeln!(file, "   ruby 3.2.0p0").unwrap();
            file.flush().unwrap();

            let version = detect_ruby_version_from_lockfile(file.path());
            assert_eq!(version, Some("3.2.0".to_string()));
        }

        #[test]
        fn empty_line_in_section_continues() {
            let mut file = NamedTempFile::new().unwrap();
            writeln!(file, "RUBY VERSION").unwrap();
            writeln!(file).unwrap();
            writeln!(file, "   ruby 3.4.0").unwrap();
            file.flush().unwrap();

            let version = detect_ruby_version_from_lockfile(file.path());
            assert_eq!(version, Some("3.4.0".to_string()));
        }

        #[test]
        fn stops_at_next_section() {
            let mut file = NamedTempFile::new().unwrap();
            writeln!(file, "RUBY VERSION").unwrap();
            writeln!(file, "   ruby 3.4.0").unwrap();
            writeln!(file, "BUNDLED WITH").unwrap();
            writeln!(file, "   2.5.0").unwrap();
            file.flush().unwrap();

            let version = detect_ruby_version_from_lockfile(file.path());
            assert_eq!(version, Some("3.4.0".to_string()));
        }

        #[test]
        fn case_sensitive_section_header() {
            let mut file = NamedTempFile::new().unwrap();
            writeln!(file, "ruby version").unwrap();
            writeln!(file, "   ruby 3.4.0").unwrap();
            file.flush().unwrap();

            let version = detect_ruby_version_from_lockfile(file.path());
            assert_eq!(version, None);
        }

        #[test]
        fn empty_ruby_section() {
            let mut file = NamedTempFile::new().unwrap();
            writeln!(file, "RUBY VERSION").unwrap();
            writeln!(file).unwrap();
            writeln!(file, "BUNDLED WITH").unwrap();
            file.flush().unwrap();

            let version = detect_ruby_version_from_lockfile(file.path());
            assert_eq!(version, None);
        }
    }

    mod standard_gem_paths {
        use super::*;

        #[test]
        fn contains_ruby_version() {
            let paths = get_standard_gem_paths("3.4.0");
            for path in &paths {
                assert!(
                    path.to_string_lossy().contains("3.4.0"),
                    "Path should contain ruby version: {path:?}"
                );
            }
        }

        #[test]
        fn not_empty() {
            let paths = get_standard_gem_paths("3.4.0");
            assert!(!paths.is_empty());
        }

        #[test]
        fn platform_specific() {
            let paths = get_standard_gem_paths("3.4.0");
            let all_paths_str = paths
                .iter()
                .map(|p| p.to_string_lossy().to_string())
                .collect::<Vec<_>>()
                .join("|");

            #[cfg(target_os = "macos")]
            assert!(all_paths_str.contains("Library") || all_paths_str.contains("homebrew"));

            #[cfg(target_os = "linux")]
            assert!(all_paths_str.contains("lib/ruby/gems"));

            #[cfg(target_os = "windows")]
            assert!(all_paths_str.contains("Ruby") || all_paths_str.contains("gem"));
        }
    }

    mod detect_engine_env_var {
        use super::*;
        use std::str::FromStr;

        #[test]
        fn case_insensitive_parsing() {
            assert_eq!(RubyEngine::from_str("MRI").unwrap(), RubyEngine::Mri);
            assert_eq!(RubyEngine::from_str("JRUBY").unwrap(), RubyEngine::JRuby);
        }

        #[test]
        fn unknown_engine_variant() {
            let engine = RubyEngine::from_str("myengine").unwrap();
            assert!(matches!(engine, RubyEngine::Unknown(ref name) if name == "myengine"));
        }

        #[test]
        fn truffleruby_exact_and_variant() {
            assert_eq!(
                RubyEngine::from_str("truffleruby").unwrap(),
                RubyEngine::TruffleRuby
            );
            assert_eq!(
                RubyEngine::from_str("truffleruby-24.1.0").unwrap(),
                RubyEngine::TruffleRuby
            );
        }

        #[test]
        fn mruby_variants() {
            assert_eq!(RubyEngine::from_str("mruby").unwrap(), RubyEngine::MRuby);
            assert_eq!(
                RubyEngine::from_str("mruby-3.2.0").unwrap(),
                RubyEngine::MRuby
            );
        }
    }

    mod normalize_version_edge_cases {
        use super::*;

        #[test]
        fn multiple_operators_trim_first() {
            assert_eq!(normalize_ruby_version(">= ~> 3.0.0"), "~> 3.0.0");
        }

        #[test]
        fn operator_only() {
            assert_eq!(normalize_ruby_version(">="), "0.0.0");
        }

        #[test]
        fn with_extra_whitespace() {
            assert_eq!(normalize_ruby_version("  >=  3.4.0  "), "3.4.0");
        }

        #[test]
        fn gt_operator() {
            assert_eq!(normalize_ruby_version("> 3.1.0"), "3.1.0");
        }

        #[test]
        fn pessimistic_operator() {
            assert_eq!(normalize_ruby_version("~> 3.4"), "3.4.0");
        }

        #[test]
        fn preserves_patch_in_version() {
            assert_eq!(normalize_ruby_version(">= 3.4.1p5"), "3.4.0");
        }
    }
}
