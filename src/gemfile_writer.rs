//! Gemfile writing with structure preservation.
//!
//! This module provides functionality to modify Gemfiles while preserving:
//! - Comments
//! - Formatting and indentation
//! - Group blocks
//! - Original ordering (with smart insertion)
//!
//! # Approach
//!
//! Rather than parsing to an AST and regenerating, we read the file line-by-line,
//! identify gem declarations using regex, and perform targeted modifications.
//! This preserves user formatting and comments.
//!
//! Similar to `bundle add` / `bundle remove` - modifies your Gemfile programmatically
//! while keeping your formatting intact.

use anyhow::{Context, Result};
use regex::Regex;
use std::fs;
use std::path::Path;

/// A Gemfile writer that preserves structure and formatting
#[derive(Debug)]
pub struct GemfileWriter {
    path: String,
    lines: Vec<String>,
}

impl GemfileWriter {
    /// Load a Gemfile for modification
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read.
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path_str = path.as_ref().display().to_string();
        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read Gemfile at {path_str}"))?;

        let lines = content.lines().map(String::from).collect();

        Ok(Self {
            path: path_str,
            lines,
        })
    }

    /// Add a gem to the Gemfile.
    ///
    /// Inserts the gem in alphabetical order within the appropriate group.
    /// If the gem already exists, updates it in place.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use lode::gemfile_writer::GemfileWriter;
    /// let mut writer = GemfileWriter::load("Gemfile")?;
    /// writer.add_gem("rails", Some("~> 7.0"), None, None)?;
    /// writer.add_gem("rspec", None, Some("test"), None)?;
    /// writer.write()?;
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    pub fn add_gem(
        &mut self,
        name: &str,
        version: Option<&str>,
        group: Option<&str>,
        options: Option<&str>,
    ) -> Result<()> {
        // Check if gem already exists
        if let Some(line_idx) = self.find_gem(name) {
            // Update existing gem
            self.update_gem_at(line_idx, name, version, options);
        } else {
            // Insert new gem
            self.insert_gem(name, version, group, options)?;
        }

        Ok(())
    }

    /// Remove a gem from the Gemfile
    ///
    /// Removes all declarations of the gem, including those in group blocks.
    ///
    /// # Arguments
    ///
    /// * `name` - Gem name to remove
    ///
    /// # Returns
    ///
    /// Returns `true` if the gem was found and removed, `false` otherwise.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use lode::gemfile_writer::GemfileWriter;
    /// let mut writer = GemfileWriter::load("Gemfile")?;
    /// let removed = writer.remove_gem("minitest")?;
    /// if removed {
    ///     writer.write()?;
    /// }
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    pub fn remove_gem(&mut self, name: &str) -> Result<bool> {
        let mut removed = false;
        let gem_pattern = Self::gem_pattern(name);

        // Remove in reverse order to avoid index shifting
        for idx in (0..self.lines.len()).rev() {
            if let Some(line) = self.lines.get(idx)
                && gem_pattern.is_match(line)
            {
                self.lines.remove(idx);
                removed = true;
            }
        }

        Ok(removed)
    }

    /// Write the modified Gemfile back to disk
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be written.
    pub fn write(&self) -> Result<()> {
        let content = self.lines.join("\n");
        // Add trailing newline if original had one
        let content = if content.is_empty() || content.ends_with('\n') {
            content
        } else {
            format!("{content}\n")
        };

        fs::write(&self.path, content)
            .with_context(|| format!("Failed to write Gemfile to {}", self.path))
    }

    /// Find the line index of a gem declaration
    fn find_gem(&self, name: &str) -> Option<usize> {
        let pattern = Self::gem_pattern(name);
        self.lines.iter().position(|line| pattern.is_match(line))
    }

    /// Update a gem declaration at a specific line
    fn update_gem_at(
        &mut self,
        line_idx: usize,
        name: &str,
        version: Option<&str>,
        options: Option<&str>,
    ) {
        if let Some(existing_line) = self.lines.get(line_idx) {
            // Extract indentation from existing line
            let indent = existing_line
                .chars()
                .take_while(|c| c.is_whitespace())
                .collect::<String>();

            // Build new gem line
            let new_line = Self::format_gem_line(&indent, name, version, options);
            if let Some(line) = self.lines.get_mut(line_idx) {
                *line = new_line;
            }
        }
    }

    /// Insert a new gem declaration
    fn insert_gem(
        &mut self,
        name: &str,
        version: Option<&str>,
        group: Option<&str>,
        options: Option<&str>,
    ) -> Result<()> {
        if let Some(group_name) = group {
            // Insert into group block
            self.insert_into_group(name, version, group_name, options)?;
        } else {
            // Insert into default gems section (after source, before groups)
            self.insert_into_default_section(name, version, options);
        }

        Ok(())
    }

    /// Insert gem into a group block
    fn insert_into_group(
        &mut self,
        name: &str,
        version: Option<&str>,
        group_name: &str,
        options: Option<&str>,
    ) -> Result<()> {
        // Find group block
        let group_pattern = Regex::new(&format!(r"^\s*group\s+:?{group_name}"))?;

        if let Some(group_start) = self
            .lines
            .iter()
            .position(|line| group_pattern.is_match(line))
        {
            // Find end of group block
            let mut insert_idx = group_start + 1;
            let mut group_depth = 1;

            for idx in (group_start + 1)..self.lines.len() {
                let Some(line) = self.lines.get(idx) else {
                    continue;
                };

                if line.trim().starts_with("group") {
                    group_depth += 1;
                } else if line.trim() == "end" {
                    group_depth -= 1;
                    if group_depth == 0 {
                        insert_idx = idx;
                        break;
                    }
                }

                // Check for gem declarations to maintain alphabetical order
                if let Some(existing_name) = Self::extract_gem_name(line) {
                    if existing_name.as_str() > name {
                        insert_idx = idx;
                        break;
                    }
                    insert_idx = idx + 1;
                }
            }

            // Build gem line with proper indentation
            let gem_line = Self::format_gem_line("  ", name, version, options);
            self.lines.insert(insert_idx, gem_line);
        } else {
            // Group doesn't exist, create it
            self.create_group_block(name, version, group_name, options);
        }

        Ok(())
    }

    /// Insert gem into default section (not in any group)
    fn insert_into_default_section(
        &mut self,
        name: &str,
        version: Option<&str>,
        options: Option<&str>,
    ) {
        // Find insertion point: after source/ruby declarations, before first group
        let mut insert_idx = 0;

        // Skip source/ruby/gemspec lines
        for (idx, line) in self.lines.iter().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with("source")
                || trimmed.starts_with("ruby")
                || trimmed.starts_with("gemspec")
                || trimmed.is_empty()
                || trimmed.starts_with('#')
            {
                insert_idx = idx + 1;
            } else if trimmed.starts_with("group") {
                // Stop before first group
                break;
            } else if trimmed.starts_with("gem") {
                // Found existing gem, maintain alphabetical order
                if let Some(existing_name) = Self::extract_gem_name(line) {
                    if existing_name.as_str() > name {
                        insert_idx = idx;
                        break;
                    }
                    insert_idx = idx + 1;
                }
            }
        }

        let gem_line = Self::format_gem_line("", name, version, options);
        self.lines.insert(insert_idx, gem_line);
    }

    /// Create a new group block and add gem to it
    fn create_group_block(
        &mut self,
        name: &str,
        version: Option<&str>,
        group_name: &str,
        options: Option<&str>,
    ) {
        // Insert group block at end of file
        self.lines.push(String::new()); // Empty line before group
        self.lines.push(format!("group :{group_name} do"));
        self.lines
            .push(Self::format_gem_line("  ", name, version, options));
        self.lines.push("end".to_string());
    }

    /// Format a gem declaration line
    fn format_gem_line(
        indent: &str,
        name: &str,
        version: Option<&str>,
        options: Option<&str>,
    ) -> String {
        use std::fmt::Write;

        let mut line = format!("{indent}gem \"{name}\"");

        if let Some(ver) = version {
            let _ = write!(line, ", \"{ver}\"");
        }

        if let Some(opts) = options {
            let _ = write!(line, ", {opts}");
        }

        line
    }

    /// Create a regex pattern to match a gem declaration
    fn gem_pattern(name: &str) -> Regex {
        // Matches: gem "name" or gem 'name' at start of line (with optional whitespace)
        Regex::new(&format!(r#"^\s*gem\s+["']{name}["']"#)).expect("should build valid regex")
    }

    /// Extract gem name from a gem declaration line
    fn extract_gem_name(line: &str) -> Option<String> {
        let pattern = Regex::new(r#"^\s*gem\s+["']([^"']+)["']"#).ok()?;
        pattern
            .captures(line)
            .and_then(|cap| cap.get(1).map(|m| m.as_str().to_string()))
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "Tests can panic")]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn add_gem_to_empty_gemfile() {
        let temp = NamedTempFile::new().unwrap();
        fs::write(&temp, "source \"https://rubygems.org\"\n").unwrap();

        let mut writer = GemfileWriter::load(temp.path()).unwrap();
        writer.add_gem("rails", Some("~> 7.0"), None, None).unwrap();
        writer.write().unwrap();

        let content = fs::read_to_string(temp.path()).unwrap();
        assert!(content.contains("gem \"rails\", \"~> 7.0\""));
    }

    #[test]
    fn test_remove_gem() {
        let temp = NamedTempFile::new().unwrap();
        fs::write(&temp, "source \"https://rubygems.org\"\ngem \"rails\"\n").unwrap();

        let mut writer = GemfileWriter::load(temp.path()).unwrap();
        let removed = writer.remove_gem("rails").unwrap();
        assert!(removed);
        writer.write().unwrap();

        let content = fs::read_to_string(temp.path()).unwrap();
        assert!(!content.contains("gem \"rails\""));
    }

    #[test]
    fn test_extract_gem_name() {
        assert_eq!(
            GemfileWriter::extract_gem_name("gem \"rails\""),
            Some("rails".to_string())
        );
        assert_eq!(
            GemfileWriter::extract_gem_name("  gem 'rack'"),
            Some("rack".to_string())
        );
        assert_eq!(
            GemfileWriter::extract_gem_name("gem \"nokogiri\", \"~> 1.13\""),
            Some("nokogiri".to_string())
        );
        assert_eq!(GemfileWriter::extract_gem_name("# gem \"commented\""), None);
    }

    #[test]
    fn test_format_gem_line() {
        assert_eq!(
            GemfileWriter::format_gem_line("", "rails", None, None),
            "gem \"rails\""
        );
        assert_eq!(
            GemfileWriter::format_gem_line("  ", "rails", Some("~> 7.0"), None),
            "  gem \"rails\", \"~> 7.0\""
        );
        assert_eq!(
            GemfileWriter::format_gem_line("", "rails", Some("~> 7.0"), Some("require: false")),
            "gem \"rails\", \"~> 7.0\", require: false"
        );
    }

    #[test]
    fn update_existing_gem() {
        let temp = NamedTempFile::new().unwrap();
        fs::write(
            &temp,
            "source \"https://rubygems.org\"\ngem \"rails\", \"~> 6.0\"\n",
        )
        .unwrap();

        let mut writer = GemfileWriter::load(temp.path()).unwrap();
        writer.add_gem("rails", Some("~> 7.0"), None, None).unwrap();
        writer.write().unwrap();

        let content = fs::read_to_string(temp.path()).unwrap();
        assert!(content.contains("gem \"rails\", \"~> 7.0\""));
        assert!(!content.contains("~> 6.0"));
    }
}
