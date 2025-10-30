//! Shared test utilities for lode tests
//!
//! This module provides common test helpers, fixtures, and utilities
//! to reduce code duplication across test modules.

#[cfg(test)]
pub mod fixtures {
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

    /// Create a temporary directory with a minimal Gemfile
    pub fn create_test_gemfile_dir() -> (TempDir, PathBuf) {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let gemfile_path = temp_dir.path().join("Gemfile");

        let content = r#"# frozen_string_literal: true
source "https://rubygems.org"

gem "rails", "~> 7.0"
gem "rspec", "~> 3.12", group: :test
"#;

        fs::write(&gemfile_path, content).expect("Failed to write Gemfile");
        (temp_dir, gemfile_path)
    }

    /// Create a temporary directory with a simple Gemfile.lock
    pub fn create_test_lockfile_dir() -> (TempDir, PathBuf) {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let lockfile_path = temp_dir.path().join("Gemfile.lock");

        let content = r#"GEM
  remote: https://rubygems.org/
  specs:
    actioncable (7.0.0)
      rails (= 7.0.0)
    actionmailbox (7.0.0)
      rails (= 7.0.0)
    rails (7.0.0)

PLATFORMS
  x86_64-linux

DEPENDENCIES
  rails (~> 7.0)

BUNDLED WITH
   2.3.16
"#;

        fs::write(&lockfile_path, content).expect("Failed to write Gemfile.lock");
        (temp_dir, lockfile_path)
    }

    /// Create a temporary gemspec file for testing
    pub fn create_test_gemspec(name: &str, version: &str) -> (TempDir, PathBuf) {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let gemspec_path = temp_dir.path().join(format!("{name}.gemspec"));

        let content = format!(
            r#"Gem::Specification.new do |spec|
  spec.name          = "{name}"
  spec.version       = "{version}"
  spec.authors       = ["Test Author"]
  spec.email         = ["test@example.com"]
  spec.summary       = "Test gem {name}"
  spec.description   = "A test gem for {name}"
  spec.homepage      = "https://github.com/test/{name}"
  spec.license       = "MIT"
end
"#
        );

        fs::write(&gemspec_path, content).expect("Failed to write gemspec");
        (temp_dir, gemspec_path)
    }
}

#[cfg(test)]
pub mod options {
    use crate::commands::gem_install::InstallOptions;

    /// Create a minimal InstallOptions for testing
    pub fn minimal_install_options(gem_name: &str) -> InstallOptions {
        InstallOptions {
            gems: vec![gem_name.to_string()],
            platform: None,
            version: None,
            prerelease: false,
            install_dir: None,
            bindir: None,
            document: None,
            no_document: false,
            build_root: None,
            vendor: false,
            env_shebang: false,
            force: false,
            wrappers: false,
            trust_policy: None,
            ignore_dependencies: false,
            format_executable: false,
            user_install: false,
            development: false,
            development_all: false,
            conservative: false,
            minimal_deps: false,
            post_install_message: false,
            file: None,
            without: None,
            explain: false,
            lock: false,
            suggestions: false,
            target_rbconfig: None,
            local: false,
            remote: false,
            both: false,
            bulk_threshold: None,
            clear_sources: false,
            source: None,
            http_proxy: None,
            verbose: false,
            quiet: false,
            silent: false,
            config_file: None,
            backtrace: false,
            debug: false,
            norc: false,
        }
    }
}

#[cfg(test)]
pub mod assertions {
    /// Assert that an error message contains a specific substring
    pub fn assert_error_contains(error_msg: &str, expected_text: &str) {
        assert!(
            error_msg.to_lowercase().contains(&expected_text.to_lowercase()),
            "Error message '{}' does not contain '{}'",
            error_msg,
            expected_text
        );
    }

    /// Assert that a string matches a regex pattern
    pub fn assert_matches_pattern(text: &str, pattern: &str) {
        let re = regex::Regex::new(pattern).expect("Invalid regex pattern");
        assert!(
            re.is_match(text),
            "Text '{}' does not match pattern '{}'",
            text,
            pattern
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_test_gemfile_dir() {
        let (_temp, gemfile_path) = fixtures::create_test_gemfile_dir();
        assert!(gemfile_path.exists());
        let content = std::fs::read_to_string(&gemfile_path).unwrap();
        assert!(content.contains("rails"));
    }

    #[test]
    fn test_create_test_lockfile_dir() {
        let (_temp, lockfile_path) = fixtures::create_test_lockfile_dir();
        assert!(lockfile_path.exists());
        let content = std::fs::read_to_string(&lockfile_path).unwrap();
        assert!(content.contains("GEM"));
    }

    #[test]
    fn test_create_test_gemspec() {
        let (_temp, gemspec_path) = fixtures::create_test_gemspec("test-gem", "1.0.0");
        assert!(gemspec_path.exists());
        let content = std::fs::read_to_string(&gemspec_path).unwrap();
        assert!(content.contains("test-gem"));
        assert!(content.contains("1.0.0"));
    }

    #[test]
    fn test_minimal_install_options() {
        let options = options::minimal_install_options("rails");
        assert_eq!(options.gems[0], "rails");
        assert!(!options.verbose);
        assert!(!options.quiet);
    }
}
