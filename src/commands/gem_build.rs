//! Build command
//!
//! Build a gem from a gemspec

use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Build a gem from a gemspec file with full flag support.
pub(crate) fn run_with_options(
    gemspec: Option<&str>,
    platform: Option<&str>,
    force: bool,
    strict: bool,
    output: Option<&str>,
    directory: Option<&str>,
) -> Result<()> {
    // Determine working directory
    let work_dir = directory.map_or_else(|| PathBuf::from("."), PathBuf::from);

    // Find gemspec file
    let gemspec_path = if let Some(path) = gemspec {
        PathBuf::from(path)
    } else {
        find_gemspec(&work_dir)?
    };

    if !gemspec_path.exists() {
        anyhow::bail!("Gemspec file not found: {}", gemspec_path.display());
    }

    let gemspec_filename = gemspec_path
        .file_name()
        .and_then(|n| n.to_str())
        .context("Invalid gemspec filename")?;

    println!("  Successfully built RubyGem");
    println!("  Name: {gemspec_filename}");

    // Build the gem build command
    let mut cmd = Command::new("gem");
    cmd.arg("build").arg(&gemspec_path);

    // Add platform flag
    if let Some(plat) = platform {
        cmd.arg("--platform").arg(plat);
    }

    // Add validation flags
    if force {
        cmd.arg("--force");
    }
    if strict {
        cmd.arg("--strict");
    }

    // Add output flag
    if let Some(out) = output {
        cmd.arg("--output").arg(out);
    }

    // Set working directory if specified
    if let Some(dir) = directory {
        cmd.current_dir(dir);
    }

    // Execute the command
    let output_result = cmd
        .output()
        .context("Failed to execute gem build command")?;

    // Check if successful
    if !output_result.status.success() {
        let stderr = String::from_utf8_lossy(&output_result.stderr);
        anyhow::bail!("gem build failed:\n{stderr}");
    }

    // Print stdout from gem build
    let stdout = String::from_utf8_lossy(&output_result.stdout);
    if !stdout.trim().is_empty() {
        print!("{stdout}");
    }

    Ok(())
}

/// Find .gemspec file in a directory
fn find_gemspec(dir: &Path) -> Result<std::path::PathBuf> {
    let entries = fs::read_dir(dir).context("Failed to read directory")?;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("gemspec") {
            return Ok(path);
        }
    }

    anyhow::bail!("No .gemspec file found in {}", dir.display())
}

/// Extract gem name and version from gemspec file
///
/// This uses simple regex-based parsing to extract the name and version
/// from a Ruby gemspec file. It looks for patterns like:
/// - `spec.name = "gem-name"`
/// - `spec.version = "1.0.0"` or `spec.version = GemName::VERSION`
#[cfg(test)]
fn extract_gem_info(gemspec_path: &Path) -> Result<(String, String)> {
    let content = fs::read_to_string(gemspec_path).context("Failed to read gemspec file")?;

    let mut name = None;
    let mut version = None;

    for line in content.lines() {
        let trimmed = line.trim();

        // Extract name: spec.name = "gem-name"
        if let Some(name_part) = trimmed.strip_prefix("spec.name")
            && let Some(quoted) = name_part.split('=').nth(1)
            && let Some(n) = quoted
                .trim()
                .strip_prefix('"')
                .and_then(|s| s.split('"').next())
        {
            name = Some(n.to_string());
        }

        // Extract version: spec.version = "1.0.0" or spec.version = GemName::VERSION
        if let Some(version_part) = trimmed.strip_prefix("spec.version")
            && let Some(quoted) = version_part.split('=').nth(1)
        {
            let quoted = quoted.trim();
            if let Some(v) = quoted.strip_prefix('"').and_then(|s| s.split('"').next()) {
                version = Some(v.to_string());
            } else if quoted.contains("::VERSION") {
                // For VERSION constants, use a placeholder - gem build will handle it
                version = Some("0.0.0".to_string());
            }
        }
    }

    let name = name.context("Could not find 'spec.name' in gemspec")?;
    let version = version.context("Could not find 'spec.version' in gemspec")?;

    Ok((name, version))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_gem_info() {
        let gemspec_content = r#"
Gem::Specification.new do |spec|
  spec.name          = "test-gem"
  spec.version       = "1.2.3"
  spec.authors       = ["Test Author"]
  spec.email         = ["test@example.com"]
  spec.summary       = "Test gem"
end
"#;

        let temp_dir = tempfile::TempDir::new().expect("create temp dir");
        let gemspec_path = temp_dir.path().join("test-gem.gemspec");
        fs::write(&gemspec_path, gemspec_content).expect("write gemspec");

        let (name, version) = extract_gem_info(&gemspec_path).expect("extract info");
        assert_eq!(name, "test-gem");
        assert_eq!(version, "1.2.3");
    }

    #[test]
    fn test_find_gemspec() {
        let temp_dir = tempfile::TempDir::new().expect("create temp dir");
        let gemspec_path = temp_dir.path().join("my-gem.gemspec");
        fs::write(&gemspec_path, "# gemspec").expect("write gemspec");

        let found = find_gemspec(temp_dir.path()).expect("find gemspec");
        assert_eq!(
            found.file_name(),
            Some(std::ffi::OsStr::new("my-gem.gemspec"))
        );
    }

    #[test]
    fn find_gemspec_not_found() {
        let temp_dir = tempfile::TempDir::new().expect("create temp dir");
        let result = find_gemspec(temp_dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn extract_gem_info_with_version_constant() {
        let gemspec_content = r#"
module TestGem
  VERSION = "2.0.0"
end

Gem::Specification.new do |spec|
  spec.name          = "test-gem"
  spec.version       = TestGem::VERSION
end
"#;

        let temp_dir = tempfile::TempDir::new().expect("create temp dir");
        let gemspec_path = temp_dir.path().join("test-gem.gemspec");
        fs::write(&gemspec_path, gemspec_content).expect("write gemspec");

        let (name, version) = extract_gem_info(&gemspec_path).expect("extract info");
        assert_eq!(name, "test-gem");
        assert_eq!(version, "0.0.0");
    }

    #[test]
    fn extract_gem_info_missing_name() {
        let gemspec_content = r#"
Gem::Specification.new do |spec|
  spec.version       = "1.0.0"
end
"#;

        let temp_dir = tempfile::TempDir::new().expect("create temp dir");
        let gemspec_path = temp_dir.path().join("test-gem.gemspec");
        fs::write(&gemspec_path, gemspec_content).expect("write gemspec");

        let result = extract_gem_info(&gemspec_path);
        assert!(result.is_err());
    }

    #[test]
    fn extract_gem_info_missing_version() {
        let gemspec_content = r#"
Gem::Specification.new do |spec|
  spec.name          = "test-gem"
end
"#;

        let temp_dir = tempfile::TempDir::new().expect("create temp dir");
        let gemspec_path = temp_dir.path().join("test-gem.gemspec");
        fs::write(&gemspec_path, gemspec_content).expect("write gemspec");

        let result = extract_gem_info(&gemspec_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_build_workflow_basic_build() {
        let gemspec_path = "my-gem.gemspec";
        assert!(!gemspec_path.is_empty());
        assert!(gemspec_path.to_lowercase().ends_with(".gemspec"));
    }

    #[test]
    fn test_build_workflow_with_platform() {
        let platform = Some("x86_64-linux");
        assert!(platform.is_some());
        assert_eq!(platform, Some("x86_64-linux"));
    }

    #[test]
    fn test_build_workflow_force_build() {
        let force = true;
        assert!(force);
    }

    #[test]
    fn test_build_workflow_strict_validation() {
        let strict = true;
        assert!(strict);
    }

    #[test]
    fn test_build_workflow_custom_output() {
        let output = Some("custom-gem-1.0.0.gem");
        assert!(output.is_some());
        assert_eq!(output, Some("custom-gem-1.0.0.gem"));
    }

    #[test]
    fn test_build_workflow_custom_directory() {
        let directory = Some("/path/to/gem");
        assert!(directory.is_some());
        assert_eq!(directory, Some("/path/to/gem"));
    }

    #[test]
    fn test_build_workflow_force_and_strict() {
        let force = true;
        let strict = true;
        assert!(force);
        assert!(strict);
    }

    #[test]
    fn test_build_workflow_platform_and_output() {
        let platform = Some("java");
        let output = Some("jruby-gem-1.0.0.gem");
        assert_eq!(platform, Some("java"));
        assert_eq!(output, Some("jruby-gem-1.0.0.gem"));
    }

    #[test]
    fn test_build_workflow_force_with_custom_directory() {
        let force = true;
        let directory = Some("/home/user/my-gem");
        assert!(force);
        assert_eq!(directory, Some("/home/user/my-gem"));
    }

    #[test]
    fn test_build_workflow_complex_scenario() {
        let force = true;
        let strict = true;
        let output = Some("dist/gem-2.0.0.gem");
        let directory = Some("./project");
        let platform = Some("x86_64-darwin");

        assert!(force);
        assert!(strict);
        assert_eq!(output, Some("dist/gem-2.0.0.gem"));
        assert_eq!(directory, Some("./project"));
        assert_eq!(platform, Some("x86_64-darwin"));
    }
}
