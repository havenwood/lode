//! Init command
//!
//! Create a new Gemfile in the current directory

use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

/// Template for a new Gemfile
const GEMFILE_TEMPLATE: &str = r#"# frozen_string_literal: true

source "{}"

# Specify your gem dependencies here
# gem "rails"
# gem "pg"

# Specify your Ruby version (optional)
# ruby "3.3.0"
"#;

/// Template for a Gemfile from gemspec
const GEMFILE_FROM_GEMSPEC_TEMPLATE: &str = r#"# frozen_string_literal: true

source "{}"

# Specify your gem's dependencies in {}.gemspec
gemspec
"#;

/// Create a new Gemfile in the specified directory
pub(crate) fn run(path: &str, from_gemspec: bool) -> Result<()> {
    let gemfile_path = Path::new(path).join("Gemfile");

    // Check if Gemfile already exists
    if gemfile_path.exists() {
        anyhow::bail!("Gemfile already exists at {}", gemfile_path.display());
    }

    // Create the directory if it doesn't exist
    if let Some(parent) = gemfile_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory {}", parent.display()))?;
    }

    // Write the template Gemfile
    let content = if from_gemspec {
        // Find the gemspec file
        let dir_path = Path::new(path);
        let gemspec_files: Vec<_> = fs::read_dir(dir_path)
            .with_context(|| format!("Failed to read directory {}", dir_path.display()))?
            .filter_map(Result::ok)
            .filter(|entry| {
                entry.path().extension().and_then(|ext| ext.to_str()) == Some("gemspec")
            })
            .collect();

        if gemspec_files.is_empty() {
            anyhow::bail!("No .gemspec file found in {}", dir_path.display());
        }

        if gemspec_files.len() > 1 {
            let names: Vec<_> = gemspec_files
                .iter()
                .filter_map(|e| e.file_name().to_str().map(String::from))
                .collect();
            anyhow::bail!(
                "Multiple .gemspec files found: {}. Please specify which one to use.",
                names.join(", ")
            );
        }

        let gemspec_path = gemspec_files
            .first()
            .map(std::fs::DirEntry::path)
            .context("No gemspec files found")?;

        let gemspec_name = gemspec_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("project")
            .to_string();

        GEMFILE_FROM_GEMSPEC_TEMPLATE
            .replacen("{}", lode::DEFAULT_GEM_SOURCE, 1)
            .replacen("{}", &gemspec_name, 1)
    } else {
        GEMFILE_TEMPLATE.replace("{}", lode::DEFAULT_GEM_SOURCE)
    };

    fs::write(&gemfile_path, content)
        .with_context(|| format!("Failed to write Gemfile to {}", gemfile_path.display()))?;

    println!("Writing new Gemfile to {}", gemfile_path.display());

    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "Tests can panic")]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn init_creates_gemfile() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path().to_str().unwrap();

        let result = run(temp_path, false);
        assert!(result.is_ok());

        let gemfile_path = temp_dir.path().join("Gemfile");
        assert!(gemfile_path.exists());

        let content = fs::read_to_string(gemfile_path).unwrap();
        assert!(content.contains("source \"https://rubygems.org\""));
        assert!(content.contains("frozen_string_literal"));
    }

    #[test]
    fn init_fails_if_gemfile_exists() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path().to_str().unwrap();

        // Create a Gemfile first
        let gemfile_path = temp_dir.path().join("Gemfile");
        fs::write(&gemfile_path, "# existing gemfile").unwrap();

        // Try to init again
        let result = run(temp_path, false);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already exists"));
    }

    #[test]
    fn init_creates_nested_directory() {
        let temp_dir = TempDir::new().unwrap();
        let nested_path = temp_dir.path().join("nested").join("path");
        let nested_str = nested_path.to_str().unwrap();

        let result = run(nested_str, false);
        assert!(result.is_ok());

        let gemfile_path = nested_path.join("Gemfile");
        assert!(gemfile_path.exists());
    }

    #[test]
    fn init_from_gemspec() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path().to_str().unwrap();

        // Create a mock gemspec file
        let gemspec_path = temp_dir.path().join("test.gemspec");
        fs::write(&gemspec_path, "# gemspec content").unwrap();

        let result = run(temp_path, true);
        assert!(result.is_ok());

        let gemfile_path = temp_dir.path().join("Gemfile");
        assert!(gemfile_path.exists());

        let content = fs::read_to_string(gemfile_path).unwrap();
        assert!(content.contains("gemspec"));
        assert!(content.contains("test.gemspec"));
    }

    #[test]
    fn init_from_gemspec_no_file() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path().to_str().unwrap();

        let result = run(temp_path, true);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("No .gemspec file found")
        );
    }
}
