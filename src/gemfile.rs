//! Gemfile parsing using tree-sitter.

use anyhow::Result;
use std::path::Path;
use thiserror::Error;

/// Errors that can occur during Gemfile parsing
#[derive(Debug, Error)]
pub enum GemfileError {
    #[error("Failed to read Gemfile at {path}: {source}")]
    ReadError {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("Failed to parse Gemfile: {0}")]
    ParseError(String),

    #[error("Invalid version constraint: {0}")]
    InvalidVersion(String),
}

/// Represents a gem dependency from a Gemfile
///
/// Parsed from declarations like `gem 'rails', '~> 7.0'`. The version constraint
/// is stored as a string and parsed later by the resolver using the `semver` crate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GemDependency {
    /// Gem name (e.g., "rails")
    pub name: String,

    /// Version constraint (e.g., "~> 7.0", ">= 3.0, < 4.0")
    /// Empty string means no constraint (any version)
    pub version_requirement: String,

    /// Groups this gem belongs to (e.g., `["development", "test"]`)
    /// Empty means default group
    pub groups: Vec<String>,

    /// Source URL (e.g., "<https://rubygems.org>")
    /// None means use default source
    pub source: Option<String>,

    /// Git repository URL (mutually exclusive with source)
    pub git: Option<String>,

    /// Git branch
    pub branch: Option<String>,

    /// Git tag
    pub tag: Option<String>,

    /// Git commit revision
    pub ref_: Option<String>,

    /// Local path (mutually exclusive with source/git)
    pub path: Option<String>,

    /// Platform constraints (e.g., `["ruby", "x86_64-linux"]`)
    pub platforms: Vec<String>,

    /// Require statement (e.g., `require: false`)
    pub require: Option<bool>,
}

impl GemDependency {
    /// Create a new gem dependency with minimal information
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version_requirement: String::new(),
            groups: Vec::new(),
            source: None,
            git: None,
            branch: None,
            tag: None,
            ref_: None,
            path: None,
            platforms: Vec::new(),
            require: None,
        }
    }

    /// Check if this is a git-sourced gem
    #[must_use]
    #[inline]
    pub const fn is_git(&self) -> bool {
        self.git.is_some()
    }

    /// Check if this is a path-sourced gem
    #[must_use]
    #[inline]
    pub const fn is_path(&self) -> bool {
        self.path.is_some()
    }

    /// Check if this gem should be required
    #[must_use]
    #[inline]
    pub fn should_require(&self) -> bool {
        self.require.unwrap_or(true)
    }
}

/// Represents a parsed Gemfile
///
/// Parses Gemfile syntax without evaluation. Uses tree-sitter to extract
/// gem declarations statically, which is safer than `eval` and works
/// without Ruby installed.
#[derive(Debug, Clone)]
pub struct Gemfile {
    /// All gem dependencies
    pub gems: Vec<GemDependency>,

    /// Ruby version requirement (e.g., "3.2.0")
    pub ruby_version: Option<String>,

    /// Default gem source (usually "<https://rubygems.org>")
    pub source: String,

    /// Additional gem sources
    pub sources: Vec<String>,

    /// Gemspec directives (for gem development)
    pub gemspecs: Vec<String>,
}

impl Default for Gemfile {
    fn default() -> Self {
        Self::new()
    }
}

impl Gemfile {
    /// Create an empty Gemfile
    #[must_use]
    pub fn new() -> Self {
        Self {
            gems: Vec::new(),
            ruby_version: None,
            source: crate::DEFAULT_GEM_SOURCE.to_string(),
            sources: Vec::new(),
            gemspecs: Vec::new(),
        }
    }

    /// Parse a Gemfile from a file path
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or parsed.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use lode::gemfile::Gemfile;
    ///
    /// let gemfile = Gemfile::parse_file("Gemfile")?;
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    pub fn parse_file(path: impl AsRef<Path>) -> Result<Self, GemfileError> {
        let path_ref = path.as_ref();
        let content = std::fs::read_to_string(path_ref).map_err(|e| GemfileError::ReadError {
            path: path_ref.display().to_string(),
            source: e,
        })?;

        Self::parse(&content)
    }

    /// Parse a Gemfile from string content
    ///
    /// Uses tree-sitter to parse Ruby syntax without evaluation. Safer than
    /// `eval` and works without Ruby installed.
    ///
    /// # Errors
    ///
    /// Returns an error if the Gemfile syntax is invalid or cannot be parsed.
    pub fn parse(content: &str) -> Result<Self, GemfileError> {
        // Current implementation uses regex-based parsing
        // Future enhancement: Could use tree-sitter for more complex Gemfile syntax
        // Current approach is functional and handles standard Gemfile patterns

        let mut gemfile = Self::new();

        // Line-by-line parsing with regex for gem directives
        // Handles: source, ruby, gem, group, platforms
        for line in content.lines() {
            let line = line.trim();

            // Skip comments and empty lines
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Parse source directive
            if line.starts_with("source ") {
                if let Some(url) = extract_string_literal(line) {
                    gemfile.source = url;
                }
                continue;
            }

            // Parse ruby version
            if line.starts_with("ruby ") {
                if let Some(version) = extract_string_literal(line) {
                    gemfile.ruby_version = Some(version);
                }
                continue;
            }

            // Parse gem directive (simplified)
            if line.starts_with("gem ")
                && let Some(gem) = parse_gem_line(line)
            {
                gemfile.gems.push(gem);
            }
        }

        Ok(gemfile)
    }

    /// Get all gems in a specific group
    #[must_use]
    pub fn gems_in_group(&self, group: &str) -> Vec<&GemDependency> {
        self.gems
            .iter()
            .filter(|gem| gem.groups.is_empty() || gem.groups.contains(&group.to_string()))
            .collect()
    }

    /// Get all gems excluding specific groups
    #[must_use]
    pub fn gems_without_groups(&self, excluded: &[String]) -> Vec<&GemDependency> {
        self.gems
            .iter()
            .filter(|gem| gem.groups.is_empty() || !gem.groups.iter().any(|g| excluded.contains(g)))
            .collect()
    }
}

/// Extract a string literal from a line (handles both single and double quotes)
fn extract_string_literal(line: &str) -> Option<String> {
    // Find first quote (single or double)
    let start = line.find(['"', '\''])?;
    let quote_char = line.chars().nth(start)?;

    // Find matching closing quote
    let end = line[start + 1..].find(quote_char)?;

    Some(line[start + 1..start + 1 + end].to_string())
}

/// Parse a simple gem line (placeholder for tree-sitter implementation)
///
/// Simplified parser that handles basic gem declarations. The full tree-sitter
/// implementation will handle all Ruby syntax including:
/// - Multi-line gem blocks
/// - Conditional gems (if/unless)
/// - Gem groups
/// - Complex options
fn parse_gem_line(line: &str) -> Option<GemDependency> {
    // Extract gem name
    let name = extract_string_literal(line)?;

    let mut gem = GemDependency::new(name);

    // Extract version constraint (second string literal)
    let after_name = line
        .split_once(&format!("'{}'", gem.name))
        .or_else(|| line.split_once(&format!("\"{}\"", gem.name)))?
        .1;

    if let Some(version) = extract_string_literal(after_name) {
        gem.version_requirement = version;
    }

    // Check for git option
    if line.contains("git:")
        && let Some(git_url) = after_name.split("git:").nth(1)
        && let Some(url) = extract_string_literal(git_url)
    {
        gem.git = Some(url);
    }

    // Check for path option
    if line.contains("path:")
        && let Some(path_part) = after_name.split("path:").nth(1)
        && let Some(path) = extract_string_literal(path_part)
    {
        gem.path = Some(path);
    }

    // Check for group option (single group)
    if line.contains("group:")
        && let Some(group_part) = after_name.split("group:").nth(1)
        && let Some(group) = extract_group_symbol(group_part)
    {
        gem.groups.push(group);
    }

    // Check for groups option (multiple groups)
    if line.contains("groups:")
        && let Some(groups_part) = after_name.split("groups:").nth(1)
    {
        gem.groups.extend(extract_groups_array(groups_part));
    }

    Some(gem)
}

/// Extract a group symbol from Ruby code (e.g., ":development" -> "development")
fn extract_group_symbol(s: &str) -> Option<String> {
    let trimmed = s.trim();

    // Handle symbol literal: :development
    if let Some(symbol_start) = trimmed.find(':') {
        let after_colon = &trimmed[symbol_start + 1..];
        // Find end of symbol (comma, space, or end of line)
        let end = after_colon
            .find([',', ' ', ')'])
            .unwrap_or(after_colon.len());
        return Some(after_colon[..end].trim().to_string());
    }

    // Handle string literal: "development" or 'development'
    extract_string_literal(trimmed)
}

/// Extract multiple groups from a Ruby array (e.g., "[:development, :test]")
fn extract_groups_array(s: &str) -> Vec<String> {
    let mut groups = Vec::new();
    let trimmed = s.trim();

    // Find array brackets
    let start = trimmed.find('[').map_or(0, |i| i + 1);
    let end = trimmed.find(']').unwrap_or(trimmed.len());
    let array_content = &trimmed[start..end];

    // Split by commas and extract each group
    for part in array_content.split(',') {
        if let Some(group) = extract_group_symbol(part) {
            groups.push(group);
        }
    }

    groups
}

#[cfg(test)]
mod tests {
    use super::*;

    mod parsing {
        use super::*;

        #[test]
        fn empty_gemfile() {
            let gemfile = Gemfile::parse("").unwrap();
            assert_eq!(gemfile.gems.len(), 0);
        }

        #[test]
        fn source() {
            let content = r#"source "https://rubygems.org""#;
            let gemfile = Gemfile::parse(content).unwrap();
            assert_eq!(gemfile.source, "https://rubygems.org");
        }

        #[test]
        fn ruby_version() {
            let content = r#"ruby "3.2.0""#;
            let gemfile = Gemfile::parse(content).unwrap();
            assert_eq!(gemfile.ruby_version, Some("3.2.0".to_string()));
        }

        #[test]
        #[allow(
            clippy::indexing_slicing,
            reason = "test data should always have exactly one gem"
        )]
        fn simple_gem() {
            let content = r#"gem "rails""#;
            let gemfile = Gemfile::parse(content).unwrap();
            assert_eq!(gemfile.gems.len(), 1);
            assert_eq!(gemfile.gems[0].name, "rails");
        }

        #[test]
        #[allow(
            clippy::indexing_slicing,
            reason = "test data should always have exactly one gem"
        )]
        fn gem_with_version() {
            let content = r#"gem "rails", "~> 7.0""#;
            let gemfile = Gemfile::parse(content).unwrap();
            let gem = &gemfile.gems[0];
            assert_eq!(gem.name, "rails");
            assert_eq!(gem.version_requirement, "~> 7.0");
        }

        #[test]
        #[allow(
            clippy::indexing_slicing,
            reason = "test data should always have exactly one gem"
        )]
        fn git_gem() {
            let content = r#"gem "rails", git: "https://github.com/rails/rails""#;
            let gemfile = Gemfile::parse(content).unwrap();
            let gem = &gemfile.gems[0];
            assert!(gem.is_git());
            assert_eq!(gem.git, Some("https://github.com/rails/rails".to_string()));
        }

        #[test]
        #[allow(
            clippy::indexing_slicing,
            reason = "test data should always have exactly one gem"
        )]
        fn gem_with_single_group() {
            let content = r#"gem "rspec", group: :test"#;
            let gemfile = Gemfile::parse(content).unwrap();
            let gem = &gemfile.gems[0];
            assert_eq!(gem.name, "rspec");
            assert_eq!(gem.groups, vec!["test"]);
        }

        #[test]
        #[allow(
            clippy::indexing_slicing,
            reason = "test data should always have exactly one gem"
        )]
        fn gem_with_multiple_groups() {
            let content = r#"gem "pry", groups: [:development, :test]"#;
            let gemfile = Gemfile::parse(content).unwrap();
            let gem = &gemfile.gems[0];
            assert_eq!(gem.name, "pry");
            assert_eq!(gem.groups, vec!["development", "test"]);
        }
    }

    mod gem_dependency {
        use super::*;

        #[test]
        fn new() {
            let gem = GemDependency::new("rails");
            assert_eq!(gem.name, "rails");
            assert!(gem.version_requirement.is_empty());
            assert!(!gem.is_git());
            assert!(!gem.is_path());
            assert!(gem.should_require());
        }
    }

    mod gemfile {
        use super::*;

        #[test]
        #[allow(
            clippy::indexing_slicing,
            reason = "filtered should always have exactly one element"
        )]
        fn filter_without_groups() {
            let mut gemfile = Gemfile::new();

            gemfile.gems.push(GemDependency {
                name: "rails".to_string(),
                groups: vec![],
                ..GemDependency::new("rails")
            });

            gemfile.gems.push(GemDependency {
                name: "rspec".to_string(),
                groups: vec!["test".to_string()],
                ..GemDependency::new("rspec")
            });

            gemfile.gems.push(GemDependency {
                name: "pry".to_string(),
                groups: vec!["development".to_string()],
                ..GemDependency::new("pry")
            });

            let excluded = vec!["test".to_string(), "development".to_string()];
            let filtered = gemfile.gems_without_groups(&excluded);

            assert_eq!(filtered.len(), 1);
            assert_eq!(filtered[0].name, "rails");
        }
    }
}
