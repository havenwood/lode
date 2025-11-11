//! System gem directory operations
//!
//! Provides access to installed gems in system directories, supporting
//! gem enumeration, version queries, and gemspec parsing.

#![allow(clippy::empty_line_after_doc_comments)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::use_self)]
#![allow(clippy::collapsible_if)]
#![allow(clippy::indexing_slicing)]
#![allow(clippy::flat_map_option)]
#![allow(clippy::needless_continue)]

use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone)]
pub struct InstalledGem {
    /// Gem name
    pub name: String,
    /// Version string
    pub version: String,
    /// Platform (e.g., x86_64-linux, arm64-darwin, ruby)
    pub platform: String,
    /// Full path to gem directory
    pub path: PathBuf,
}

/// Manages system gem directory operations
#[derive(Debug)]
pub struct GemStore {
    /// Path to system gems directory
    gem_dir: PathBuf,
}

impl GemStore {
    /// Create a new `GemStore`, auto-detecting system gem directory
    ///
    /// # Errors
    ///
    /// Returns an error if system gem directory cannot be detected.
    pub fn new() -> Result<Self> {
        let gem_dir = Self::find_gem_dir()?;
        Ok(Self { gem_dir })
    }

    /// Create a `GemStore` with explicit gem directory
    #[must_use]
    pub const fn with_path(path: PathBuf) -> Self {
        Self { gem_dir: path }
    }

    /// Get the system gem directory path
    #[inline]
    #[must_use]
    pub fn gem_dir(&self) -> &Path {
        &self.gem_dir
    }

    /// Find system gem directory, trying multiple methods
    fn find_gem_dir() -> Result<PathBuf> {
        // Method 1: Ask Ruby's gem command
        if let Ok(output) = Command::new("gem").args(["environment", "gemdir"]).output()
            && output.status.success()
        {
            let gem_dir = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let path = PathBuf::from(&gem_dir).join("gems");
            if path.exists() {
                return Ok(path);
            }
        }

        // Method 2: Try common installation paths
        if let Ok(home) = std::env::var("HOME") {
            let home_path = Path::new(&home);

            // ~/.gem/ruby/X.X/gems
            if let Ok(entries) = fs::read_dir(home_path.join(".gem/ruby")) {
                for entry in entries.flatten() {
                    let gems_dir = entry.path().join("gems");
                    if gems_dir.exists() {
                        return Ok(gems_dir);
                    }
                }
            }
        }

        // Method 3: Try system-wide installation
        let system_paths = vec![
            "/usr/local/lib/ruby/gems",
            "/usr/lib/ruby/gems",
            "/opt/homebrew/lib/ruby/gems",
        ];

        for base_path in system_paths {
            if let Ok(entries) = fs::read_dir(base_path) {
                for entry in entries.flatten() {
                    let gems_dir = entry.path().join("gems");
                    if gems_dir.exists() {
                        return Ok(gems_dir);
                    }
                }
            }
        }

        anyhow::bail!("Could not find system gem directory. Verify Ruby installation.")
    }

    /// List all installed gems
    ///
    /// # Errors
    ///
    /// Returns an error if gem directory cannot be read.
    pub fn list_gems(&self) -> Result<Vec<InstalledGem>> {
        let mut gems = Vec::new();

        if !self.gem_dir.exists() {
            return Ok(gems);
        }

        for entry in fs::read_dir(&self.gem_dir).context("Failed to read gem directory")? {
            let entry = entry?;
            let path = entry.path();

            if !path.is_dir() {
                continue;
            }

            if let Some(dir_name) = path.file_name().and_then(|n| n.to_str()) {
                if let Some(gem) = Self::parse_gem_dir(dir_name, path.clone()) {
                    gems.push(gem);
                }
            }
        }

        // Sort by name, then version
        gems.sort_by(|a, b| {
            match a.name.cmp(&b.name) {
                std::cmp::Ordering::Equal => {
                    // Simple semver-like comparison: try to parse as X.Y.Z
                    Self::compare_versions(&a.version, &b.version)
                }
                other => other,
            }
        });

        Ok(gems)
    }

    /// Find gems matching a pattern
    ///
    /// # Errors
    ///
    /// Returns an error if gem listing fails.
    pub fn find_gems(&self, pattern: Option<&str>) -> Result<Vec<InstalledGem>> {
        let mut gems = self.list_gems()?;

        if let Some(pattern) = pattern {
            let pattern = pattern.to_lowercase();
            gems.retain(|gem| gem.name.to_lowercase().contains(&pattern));
        }

        Ok(gems)
    }

    /// Find a specific gem by name (returns all versions)
    ///
    /// # Errors
    ///
    /// Returns an error if gem listing fails.
    pub fn find_gem_by_name(&self, name: &str) -> Result<Vec<InstalledGem>> {
        let name_lower = name.to_lowercase();
        let all_gems = self.list_gems()?;
        let matching: Vec<_> = all_gems
            .into_iter()
            .filter(|g| g.name.to_lowercase() == name_lower)
            .collect();

        Ok(matching)
    }

    /// Get the latest version of a gem
    ///
    /// # Errors
    ///
    /// Returns an error if gem lookup fails.
    pub fn find_gem_latest(&self, name: &str) -> Result<Option<InstalledGem>> {
        let mut versions = self.find_gem_by_name(name)?;
        Ok(versions.pop()) // Already sorted, last is latest
    }

    /// Parse gem directory name into components
    /// Examples: "rake-13.0.6", "nokogiri-1.16.0-x86_64-linux"
    fn parse_gem_dir(dir_name: &str, path: PathBuf) -> Option<InstalledGem> {
        // Find the last '-' followed by a digit (start of version)
        let mut version_start = None;

        for (idx, ch) in dir_name.char_indices() {
            if ch == '-' {
                // Check if next character is a digit
                if let Some(next_ch) = dir_name[idx + 1..].chars().next() {
                    if next_ch.is_ascii_digit() {
                        version_start = Some(idx);
                    }
                }
            }
        }

        version_start.map(|idx| {
            let name = dir_name[..idx].to_string();
            let version_and_platform = &dir_name[idx + 1..];

            // Try to extract platform if present (e.g., -x86_64-linux)
            let (version, platform) = Self::extract_platform(version_and_platform);

            InstalledGem {
                name,
                version: version.to_string(),
                platform,
                path,
            }
        })
    }

    /// Extract platform from version string
    /// Examples:
    ///   "13.0.6" -> ("13.0.6", "ruby")
    ///   "1.16.0-x86_64-linux" -> ("1.16.0", "x86_64-linux")
    fn extract_platform(version_str: &str) -> (&str, String) {
        let parts: Vec<&str> = version_str.split('-').collect();

        if parts.len() == 1 {
            // Just version, no platform
            (parts[0], "ruby".to_string())
        } else {
            // version-platform-parts
            // First part is version, rest is platform
            let version = parts[0];
            let platform = parts[1..].join("-");
            (version, platform)
        }
    }

    /// Simple version comparison (not full semver, just for sorting)
    fn compare_versions(a: &str, b: &str) -> std::cmp::Ordering {
        let a_parts: Vec<u32> = a
            .split('.')
            .take(3)
            .flat_map(|s| s.parse::<u32>().ok())
            .collect();
        let b_parts: Vec<u32> = b
            .split('.')
            .take(3)
            .flat_map(|s| s.parse::<u32>().ok())
            .collect();

        for (a_part, b_part) in a_parts.iter().zip(b_parts.iter()) {
            match a_part.cmp(b_part) {
                std::cmp::Ordering::Equal => continue,
                other => return other,
            }
        }

        a_parts.len().cmp(&b_parts.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_gem_simple() {
        let gem = GemStore::parse_gem_dir("rake-13.0.6", PathBuf::from("/test")).unwrap();
        assert_eq!(gem.name, "rake");
        assert_eq!(gem.version, "13.0.6");
        assert_eq!(gem.platform, "ruby");
    }

    #[test]
    fn parse_gem_with_platform() {
        let gem = GemStore::parse_gem_dir("nokogiri-1.16.0-x86_64-linux", PathBuf::from("/test"))
            .unwrap();
        assert_eq!(gem.name, "nokogiri");
        assert_eq!(gem.version, "1.16.0");
        assert_eq!(gem.platform, "x86_64-linux");
    }

    #[test]
    fn parse_gem_arm64_darwin() {
        let gem =
            GemStore::parse_gem_dir("sqlite3-1.6.9-arm64-darwin", PathBuf::from("/test")).unwrap();
        assert_eq!(gem.name, "sqlite3");
        assert_eq!(gem.version, "1.6.9");
        assert_eq!(gem.platform, "arm64-darwin");
    }

    #[test]
    fn test_compare_versions() {
        assert_eq!(
            GemStore::compare_versions("1.0.0", "1.0.1"),
            std::cmp::Ordering::Less
        );
        assert_eq!(
            GemStore::compare_versions("2.0.0", "1.9.9"),
            std::cmp::Ordering::Greater
        );
        assert_eq!(
            GemStore::compare_versions("1.0.0", "1.0.0"),
            std::cmp::Ordering::Equal
        );
    }

    #[test]
    fn test_extract_platform() {
        let (version, platform) = GemStore::extract_platform("13.0.6");
        assert_eq!(version, "13.0.6");
        assert_eq!(platform, "ruby");

        let (version, platform) = GemStore::extract_platform("1.16.0-x86_64-linux");
        assert_eq!(version, "1.16.0");
        assert_eq!(platform, "x86_64-linux");
    }
}
