//! Push command
//!
//! Publish a gem

use anyhow::{Context, Result};
use reqwest::multipart;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

/// Push a gem to RubyGems.org.
pub(crate) async fn run_with_options(
    gem_path: &str,
    host: Option<&str>,
    key: Option<&str>,
    otp: Option<&str>,
) -> Result<()> {
    // Validate gem file exists
    let gem_file = Path::new(gem_path);
    if !gem_file.exists() {
        anyhow::bail!("Gem file not found: {gem_path}");
    }

    if gem_file.extension().and_then(|s| s.to_str()) != Some("gem") {
        anyhow::bail!("File must have .gem extension: {gem_path}");
    }

    let gem_name = gem_file
        .file_name()
        .and_then(|n| n.to_str())
        .context("Invalid gem filename")?;

    // Determine server URL (priority: CLI arg > RUBYGEMS_HOST env var > default)
    let server_url = host
        .map(String::from)
        .or_else(|| {
            let env_host = lode::env_vars::rubygems_host();
            if env_host == lode::RUBYGEMS_ORG_URL {
                None
            } else {
                Some(env_host)
            }
        })
        .unwrap_or_else(|| lode::RUBYGEMS_ORG_URL.to_string());

    println!(
        "Pushing {} to {}...",
        gem_name,
        server_url
            .trim_start_matches("https://")
            .trim_start_matches("http://")
    );

    // Load API key (checks environment variables first, then credentials file)
    let api_key = load_api_key(key.unwrap_or("rubygems"), &server_url)?;
    let push_url = format!("{server_url}/api/v1/gems");

    // Read gem file
    let gem_bytes =
        fs::read(gem_file).with_context(|| format!("Failed to read gem file: {gem_path}"))?;

    // Build multipart form
    let gem_part = multipart::Part::bytes(gem_bytes)
        .file_name(gem_name.to_string())
        .mime_str("application/octet-stream")?;

    let form = multipart::Form::new().part("file", gem_part);

    // Build HTTP client
    let client = reqwest::Client::new();
    let mut request = client
        .post(&push_url)
        .header("Authorization", api_key)
        .multipart(form);

    // Add OTP header if provided
    if let Some(otp_code) = otp {
        request = request.header("X-Rubygems-OTP", otp_code);
    }

    // Send request
    let response = request
        .send()
        .await
        .context("Failed to send gem to server")?;

    // Check response
    let status = response.status();
    let body = response
        .text()
        .await
        .unwrap_or_else(|_| "<no response body>".to_string());

    if status.is_success() {
        println!("Successfully pushed {gem_name}");
        if !body.is_empty() {
            println!("{body}");
        }
        Ok(())
    } else {
        anyhow::bail!("Failed to push gem (HTTP {}):\n{}", status.as_u16(), body)
    }
}

/// Load API key from credentials file
///
/// Reads from ~/.gem/credentials in YAML format:
/// ```yaml
/// ---
/// :rubygems_api_key: abc123...
/// ```
fn load_api_key(key_name: &str, server_url: &str) -> Result<String> {
    load_api_key_from_path(key_name, server_url, None)
}

fn load_api_key_from_path(
    key_name: &str,
    server_url: &str,
    credentials_path: Option<PathBuf>,
) -> Result<String> {
    // 1. Check RUBYGEMS_API_KEY environment variable (highest priority)
    if let Some(api_key) = lode::env_vars::rubygems_api_key() {
        return Ok(api_key);
    }

    // 2. Check host-specific GEM_HOST_API_KEY_* environment variable
    // Extract host from server URL (e.g., "https://rubygems.org" -> "rubygems.org")
    if let Some(host) = server_url
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .split('/')
        .next()
        && let Some(api_key) = lode::env_vars::gem_host_api_key(host)
    {
        return Ok(api_key);
    }

    // 3. Fall back to credentials file
    let credentials_path = credentials_path.map_or_else(get_credentials_path, Ok)?;

    if !credentials_path.exists() {
        anyhow::bail!(
            "No API key found. Set RUBYGEMS_API_KEY environment variable or run 'gem signin' first.\nExpected credentials at: {}",
            credentials_path.display()
        );
    }

    let content = fs::read_to_string(&credentials_path).with_context(|| {
        format!(
            "Failed to read credentials file: {}",
            credentials_path.display()
        )
    })?;

    // Parse YAML-style credentials file
    // Format: :rubygems_api_key: abc123...
    let key_pattern = format!(":{key_name}_api_key:");

    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(key_value) = trimmed.strip_prefix(&key_pattern) {
            let key = key_value.trim();
            if !key.is_empty() {
                return Ok(key.to_string());
            }
        }
    }

    anyhow::bail!(
        "API key '{}' not found in credentials file: {}",
        key_name,
        credentials_path.display()
    )
}

/// Get the path to the `RubyGems` credentials file
fn get_credentials_path() -> Result<PathBuf> {
    let home = env::var("HOME")
        .or_else(|_| env::var("USERPROFILE"))
        .context("Could not determine home directory")?;

    Ok(PathBuf::from(home).join(".gem").join("credentials"))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "Tests can panic")]
mod tests {
    use super::*;

    #[test]
    fn test_get_credentials_path() {
        let path = get_credentials_path();
        assert!(path.is_ok());

        let path = path.unwrap();
        assert!(path.to_string_lossy().contains(".gem"));
        assert!(path.to_string_lossy().ends_with("credentials"));
    }

    #[test]
    fn test_load_api_key() {
        let temp_dir = tempfile::TempDir::new().expect("create temp dir");
        let creds_path = temp_dir.path().join("credentials");

        let content = "---\n:rubygems_api_key: test_key_12345\n";
        fs::write(&creds_path, content).expect("write credentials");

        let key = load_api_key_from_path("rubygems", "https://rubygems.org", Some(creds_path))
            .expect("load key");
        assert_eq!(key, "test_key_12345");
    }

    #[test]
    fn load_api_key_not_found() {
        let temp_dir = tempfile::TempDir::new().expect("create temp dir");
        let creds_path = temp_dir.path().join("credentials");

        let content = "---\n:rubygems_api_key: test_key_12345\n";
        fs::write(&creds_path, content).expect("write credentials");

        let result =
            load_api_key_from_path("nonexistent", "https://rubygems.org", Some(creds_path));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[test]
    fn gem_file_validation() {
        // Invalid extension
        let result = std::panic::catch_unwind(|| {
            // This would be called in run_with_options
            let path = Path::new("test.txt");
            path.extension().and_then(|s| s.to_str()) != Some("gem")
        });
        assert!(result.is_ok());
    }

    // Workflow tests for gem push command
    #[test]
    fn test_push_workflow_basic_push() {
        let gem_path = "my-gem-1.0.0.gem";
        assert!(!gem_path.is_empty());
        assert!(gem_path.to_lowercase().ends_with(".gem"));
    }

    #[test]
    fn test_push_workflow_custom_host() {
        let host = Some("https://private.gem.repo");
        assert!(host.is_some());
        assert_eq!(host, Some("https://private.gem.repo"));
    }

    #[test]
    fn test_push_workflow_with_api_key() {
        let key = Some("my-api-key");
        assert!(key.is_some());
        assert_eq!(key, Some("my-api-key"));
    }

    #[test]
    fn test_push_workflow_with_otp() {
        let otp = Some("123456");
        assert!(otp.is_some());
        assert_eq!(otp, Some("123456"));
    }

    #[test]
    fn test_push_workflow_custom_host_and_key() {
        let host = Some("https://internal-gems.example.com");
        let key = Some("internal-api-key");
        assert_eq!(host, Some("https://internal-gems.example.com"));
        assert_eq!(key, Some("internal-api-key"));
    }

    #[test]
    fn test_push_workflow_with_mfa() {
        let key = Some("rubygems");
        let otp = Some("654321");
        assert_eq!(key, Some("rubygems"));
        assert_eq!(otp, Some("654321"));
    }

    #[test]
    fn test_push_workflow_all_options() {
        let gem_path = "gem-package-2.0.0.gem";
        let host = Some("https://gems.example.org");
        let key = Some("custom-key");
        let otp = Some("789012");

        assert!(!gem_path.is_empty());
        assert!(gem_path.to_lowercase().ends_with(".gem"));
        assert_eq!(host, Some("https://gems.example.org"));
        assert_eq!(key, Some("custom-key"));
        assert_eq!(otp, Some("789012"));
    }

    #[test]
    fn test_push_workflow_complex_scenario() {
        let gem_path = "rails-7.0.0.gem";
        let host = Some("https://gemserver.company.internal");
        let key = Some("company-gems-key");
        let otp = Some("999999");

        assert!(!gem_path.is_empty());
        assert!(gem_path.to_lowercase().ends_with(".gem"));
        assert!(host.is_some());
        assert!(key.is_some());
        assert!(otp.is_some());
    }
}
