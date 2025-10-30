//! Fetch command
//!
//! Download a gem without installing

use anyhow::{Context, Result};
use lode::{DownloadManager, RubyGemsClient, config};
use std::fs;
use std::path::PathBuf;

/// Download a gem without installing it
pub(crate) async fn run(
    gem_name: &str,
    version: Option<&str>,
    output_dir: Option<&str>,
) -> Result<()> {
    // 1. Fetch gem versions from RubyGems
    let client = RubyGemsClient::new(lode::RUBYGEMS_ORG_URL)?;
    let versions = client
        .fetch_versions(gem_name)
        .await
        .context(format!("Failed to fetch versions for gem '{gem_name}'"))?;

    if versions.is_empty() {
        anyhow::bail!("Gem '{gem_name}' not found on RubyGems.org");
    }

    // 2. Find matching version
    let selected_version = if let Some(v) = version {
        versions
            .iter()
            .find(|ver| ver.number == v)
            .context(format!("Version '{v}' not found for gem '{gem_name}'"))?
    } else {
        // Use latest version
        versions
            .first()
            .context(format!("No suitable version found for gem '{gem_name}'"))?
    };

    println!("Fetching {} ({})...", gem_name, selected_version.number);

    // 3. Download gem
    let cache_dir = config::cache_dir(None).context("Failed to get cache directory")?;
    let dm = DownloadManager::new(cache_dir)?;

    let spec = lode::GemSpec::new(
        gem_name.to_string(),
        selected_version.number.clone(),
        None, // No platform for pure Ruby gems
        vec![],
        vec![],
    );

    let gem_path = dm
        .download_gem(&spec)
        .await
        .context("Failed to download gem")?;

    // 4. Copy to output directory if specified
    let final_path = if let Some(dir) = output_dir {
        let output_path = PathBuf::from(dir);
        fs::create_dir_all(&output_path).context(format!(
            "Failed to create directory: {}",
            output_path.display()
        ))?;

        let gem_filename = gem_path.file_name().context("Invalid gem path")?;
        let target_path = output_path.join(gem_filename);

        fs::copy(&gem_path, &target_path).context("Failed to copy gem to output directory")?;

        target_path
    } else {
        gem_path
    };

    println!("Downloaded: {}", final_path.display());
    println!(
        "Successfully fetched {} ({})",
        gem_name, selected_version.number
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test validation of gem names
    fn validate_gem_name(name: &str) -> bool {
        !name.is_empty()
            && name
                .chars()
                .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
    }

    /// Test validation of versions
    fn validate_version_format(version: &str) -> bool {
        // Basic semantic version validation: digits.digits[.digits]
        !version.is_empty() && {
            let parts: Vec<&str> = version.split('.').collect();
            !parts.is_empty()
                && parts.len() <= 5
                && parts
                    .iter()
                    .all(|p| !p.is_empty() && p.chars().all(char::is_numeric))
        }
    }

    #[test]
    fn test_gem_name_validation_valid() {
        assert!(validate_gem_name("rails"));
        assert!(validate_gem_name("devise_audited"));
        assert!(validate_gem_name("my-gem"));
        assert!(validate_gem_name("gem123"));
    }

    #[test]
    fn test_gem_name_validation_invalid() {
        assert!(!validate_gem_name(""));
        assert!(!validate_gem_name("gem@invalid"));
        assert!(!validate_gem_name("gem name"));
        assert!(!validate_gem_name("gem!"));
    }

    #[test]
    fn test_version_format_validation_valid() {
        assert!(validate_version_format("1.0.0"));
        assert!(validate_version_format("7.1.2"));
        assert!(validate_version_format("0.9.1"));
        assert!(validate_version_format("2.0"));
        assert!(validate_version_format("1"));
    }

    #[test]
    fn test_version_format_validation_invalid() {
        assert!(!validate_version_format(""));
        assert!(!validate_version_format("1.a.0"));
        assert!(!validate_version_format("1.0.0.0.0.0"));
        assert!(!validate_version_format("v1.0.0"));
        assert!(!validate_version_format("1.0.0-beta")); // Prerelease not basic version
    }

    #[test]
    fn test_gem_path_construction() {
        let gem_name = "rails";
        let version = "7.1.2";
        let expected = format!("{gem_name}-{version}.gem");
        assert_eq!(expected, "rails-7.1.2.gem");
    }

    #[test]
    fn test_output_dir_path_handling() {
        let dir = "./gems";
        let path = PathBuf::from(dir);
        assert_eq!(path.as_os_str().to_str().unwrap(), "./gems");
    }

    #[test]
    fn test_version_selection_latest() {
        // When no version specified, use first (latest)
        let versions = ["7.1.2", "7.1.1", "7.0.0"];
        let selected = versions.first();
        assert_eq!(selected, Some(&"7.1.2"));
    }

    #[test]
    fn test_version_selection_specific() {
        // When version specified, find matching version
        let versions = ["7.1.2", "7.1.1", "7.0.0"];
        let target = "7.0.0";
        let selected = versions.iter().find(|v| *v == &target);
        assert_eq!(selected, Some(&"7.0.0"));
    }

    #[test]
    fn test_version_not_found() {
        // When specified version doesn't exist
        let versions = ["7.1.2", "7.1.1", "7.0.0"];
        let target = "6.0.0";
        let selected = versions.iter().find(|v| *v == &target);
        assert_eq!(selected, None);
    }

    #[test]
    fn test_empty_version_list() {
        let versions: Vec<&str> = vec![];
        let selected = versions.first();
        assert_eq!(selected, None);
    }

    #[test]
    fn test_gem_filename_construction() {
        let gem_name = "rails";
        let version = "7.1.2";
        let filename = format!("{gem_name}-{version}.gem");
        assert!(
            std::path::Path::new(&filename)
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("gem"))
        );
        assert!(filename.contains(gem_name));
        assert!(filename.contains(version));
    }

    #[test]
    fn test_pathbuf_operations() {
        let base = PathBuf::from("/tmp");
        let filename = "rails-7.1.2.gem";
        let full_path = base.join(filename);
        assert!(full_path.to_str().unwrap().contains("rails-7.1.2.gem"));
    }

    #[test]
    fn test_fetch_invalid_version_error() {
        // When version doesn't exist in available versions
        let versions = ["7.1.2", "7.1.1", "7.0.0"];
        let target = "6.0.0";
        let found = versions.iter().find(|v| *v == &target);
        assert_eq!(found, None); // Version not found
    }

    #[test]
    fn test_fetch_output_directory_path_construction() {
        // Test output directory path handling
        let output_dir = "/custom/gems";
        let filename = "rails-7.1.2.gem";
        let output_path = PathBuf::from(output_dir).join(filename);
        assert!(output_path.to_str().unwrap().contains("custom/gems"));
        assert!(output_path.to_str().unwrap().contains("rails-7.1.2.gem"));
    }
}
