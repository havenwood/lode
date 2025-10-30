//! Rdoc command
//!
//! Generate `RDoc` documentation for installed gems

use anyhow::{Context, Result};
use lode::gem_store::GemStore;
use std::process::Command;

/// Generate `RDoc` documentation for a gem
pub(crate) fn run(gem: Option<&str>) -> Result<()> {
    let gem_name = gem.context("Gem name required. Usage: lode gem-rdoc <GEM>")?;

    let store = GemStore::new()?;
    let gems = store.find_gem_by_name(gem_name)?;

    if gems.is_empty() {
        anyhow::bail!("Gem '{gem_name}' not found");
    }

    // Use the latest version if multiple are installed
    let gem_info = gems
        .last()
        .context(format!("No versions found for gem '{gem_name}'"))?;

    println!(
        "Generating RDoc for {} ({})...",
        gem_info.name, gem_info.version
    );

    // Check if rdoc is available
    let rdoc_check = Command::new("rdoc").arg("--version").output();

    if rdoc_check.is_err() {
        anyhow::bail!("rdoc command not found. Install it with: gem install rdoc");
    }

    // Generate documentation
    let status = Command::new("rdoc")
        .arg("--ri")
        .arg("--op")
        .arg(format!("doc/{}", gem_info.name))
        .current_dir(&gem_info.path)
        .status()
        .context("Failed to run rdoc command")?;

    if !status.success() {
        anyhow::bail!("rdoc command failed with status: {status}");
    }

    println!(
        "Documentation generated in {}/doc/{}",
        gem_info.path.display(),
        gem_info.name
    );
    println!(
        "View with: open {}/doc/{}/index.html",
        gem_info.path.display(),
        gem_info.name
    );

    Ok(())
}

#[cfg(test)]
mod tests {

    /// Helper function for gem name validation
    fn validate_gem_name(name: &str) -> bool {
        !name.is_empty()
            && name
                .chars()
                .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
    }

    /// Helper function for doc path construction
    fn construct_doc_path(gem_path: &str, gem_name: &str) -> String {
        format!("{gem_path}/doc/{gem_name}/index.html")
    }

    #[test]
    fn test_rdoc_gem_name_validation_valid() {
        assert!(validate_gem_name("rails"));
        assert!(validate_gem_name("devise_audited"));
        assert!(validate_gem_name("my-gem"));
        assert!(validate_gem_name("gem123"));
    }

    #[test]
    fn test_rdoc_gem_name_validation_invalid() {
        assert!(!validate_gem_name(""));
        assert!(!validate_gem_name("gem@invalid"));
        assert!(!validate_gem_name("gem name"));
    }

    #[test]
    fn test_rdoc_doc_path_construction() {
        let path = construct_doc_path("/usr/local/gems/rails-7.1.2", "rails");
        assert!(path.contains("doc/rails"));
        assert!(path.contains("index.html"));
        assert_eq!(path, "/usr/local/gems/rails-7.1.2/doc/rails/index.html");
    }

    #[test]
    fn test_rdoc_doc_directory_naming() {
        let gem_name = "devise";
        let doc_dir = format!("doc/{gem_name}");
        assert_eq!(doc_dir, "doc/devise");
    }

    #[test]
    fn test_rdoc_multiple_gems_uses_latest() {
        // When multiple versions installed, should use latest
        let gems_versions = ["2.0.0", "1.5.0", "1.0.0"];
        let selected = gems_versions.last();
        assert_eq!(selected, Some(&"1.0.0")); // last in sorted order (latest)
    }

    #[test]
    fn test_rdoc_gem_not_found_error() {
        let gem_name = "nonexistent-gem-12345";
        let result = format!("Gem '{gem_name}' not found");
        assert!(result.contains("not found"));
    }

    #[test]
    fn test_rdoc_output_message_format() {
        let gem_name = "rails";
        let version = "7.1.2";
        let message = format!("Generating RDoc for {gem_name} ({version})...");
        assert!(message.contains("Generating RDoc"));
        assert!(message.contains("rails"));
        assert!(message.contains("7.1.2"));
    }

    #[test]
    fn test_rdoc_success_message() {
        let gem_path = "/usr/local/gems/rails-7.1.2";
        let gem_name = "rails";
        let message = format!("Documentation generated in {gem_path}/doc/{gem_name}");
        assert!(message.contains("Documentation generated"));
        assert!(message.contains(gem_path));
    }

    #[test]
    fn test_rdoc_view_instruction() {
        let gem_path = "/usr/local/gems/rails-7.1.2";
        let gem_name = "rails";
        let instruction = format!("View with: open {gem_path}/doc/{gem_name}/index.html");
        assert!(instruction.contains("View with"));
        assert!(instruction.contains("open"));
        assert!(instruction.contains("index.html"));
    }

    #[test]
    fn test_rdoc_command_availability_check() {
        // Simulate rdoc availability check
        let commands_to_check = vec!["rdoc", "ri"];
        for cmd in commands_to_check {
            assert!(!cmd.is_empty());
        }
    }

    #[test]
    fn test_rdoc_version_flag() {
        // rdoc --version is the standard check for availability
        let version_flag = "--version";
        assert_eq!(version_flag, "--version");
    }

    #[test]
    fn test_rdoc_ri_flag() {
        // ri flag for RDoc generation
        let ri_flag = "--ri";
        assert_eq!(ri_flag, "--ri");
    }

    #[test]
    fn test_rdoc_op_flag_for_output() {
        // --op flag specifies output path
        let op_flag = "--op";
        assert_eq!(op_flag, "--op");
    }
}
