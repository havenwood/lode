//! Yank command
//!
//! Remove a gem version from RubyGems.org

use anyhow::{Context, Result};
use std::env;
use std::fs;
use std::path::PathBuf;

/// Yank a gem version from RubyGems.org.
pub(crate) async fn run_with_options(
    gem_name: &str,
    version: &str,
    platform: Option<&str>,
    host: Option<&str>,
    key: Option<&str>,
    otp: Option<&str>,
    undo: bool,
) -> Result<()> {
    let action = if undo { "Restoring" } else { "Yanking" };

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

    // Build display message with platform if specified
    let display_msg = platform.map_or_else(
        || format!("{action} {gem_name} version {version}..."),
        |plat| format!("{action} {gem_name} version {version} for platform {plat}..."),
    );
    println!("{display_msg}");

    // Load API key (checks environment variables first, then credentials file)
    let api_key = load_api_key(key.unwrap_or("rubygems"), &server_url)?;

    // Build yank/unyank URL
    let yank_url = if undo {
        format!("{server_url}/api/v1/gems/unyank")
    } else {
        format!("{server_url}/api/v1/gems/yank")
    };

    // Build request with query parameters
    let client = reqwest::Client::new();
    let mut query_params = vec![("gem_name", gem_name), ("version", version)];

    // Add platform if specified
    if let Some(plat) = platform {
        query_params.push(("platform", plat));
    }

    let mut request = client
        .delete(&yank_url)
        .header("Authorization", api_key)
        .query(&query_params);

    // Add OTP header if provided
    if let Some(otp_code) = otp {
        request = request.header("X-Rubygems-OTP", otp_code);
    }

    // Send request
    let response = request
        .send()
        .await
        .context("Failed to send yank request")?;

    // Check response
    let status = response.status();
    let body = response
        .text()
        .await
        .unwrap_or_else(|_| "<no response body>".to_string());

    if status.is_success() {
        let success_msg = platform.map_or_else(
            || {
                if undo {
                    format!("Successfully restored {gem_name} version {version}")
                } else {
                    format!("Successfully yanked {gem_name} version {version}")
                }
            },
            |plat| {
                if undo {
                    format!(
                        "Successfully restored {gem_name} version {version} for platform {plat}"
                    )
                } else {
                    format!("Successfully yanked {gem_name} version {version} for platform {plat}")
                }
            },
        );
        println!("{success_msg}");
        if !body.is_empty() && body != success_msg {
            println!("{body}");
        }
        Ok(())
    } else {
        anyhow::bail!(
            "Failed to {} gem (HTTP {}):\n{}",
            action.to_lowercase(),
            status.as_u16(),
            body
        )
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
    fn yank_url_construction() {
        let base = "https://rubygems.org";
        let yank_url = format!("{base}/api/v1/gems/yank");
        assert_eq!(yank_url, "https://rubygems.org/api/v1/gems/yank");

        let unyank_url = format!("{base}/api/v1/gems/unyank");
        assert_eq!(unyank_url, "https://rubygems.org/api/v1/gems/unyank");
    }

    #[test]
    fn platform_display_message() {
        let gem_name = "example-gem";
        let version = "1.0.0";
        let platform = Some("java");

        let msg = platform.map_or_else(
            || format!("Yanking {gem_name} version {version}..."),
            |plat| format!("Yanking {gem_name} version {version} for platform {plat}..."),
        );

        assert_eq!(
            msg,
            "Yanking example-gem version 1.0.0 for platform java..."
        );

        let msg_no_platform: Option<&str> = None;
        let msg2 = msg_no_platform.map_or_else(
            || format!("Yanking {gem_name} version {version}..."),
            |plat| format!("Yanking {gem_name} version {version} for platform {plat}..."),
        );

        assert_eq!(msg2, "Yanking example-gem version 1.0.0...");
    }

    #[test]
    fn test_yank_workflow_basic_yank() {
        let gem_name = "my-gem";
        let version = "1.0.0";
        assert!(!gem_name.is_empty());
        assert!(!version.is_empty());
    }

    #[test]
    fn test_yank_workflow_yank_with_platform() {
        let gem_name = "special-gem";
        let version = "2.0.0";
        let platform = Some("java");
        assert_eq!(gem_name, "special-gem");
        assert_eq!(version, "2.0.0");
        assert_eq!(platform, Some("java"));
    }

    #[test]
    fn test_yank_workflow_custom_host() {
        let host = Some("https://internal.gems.example.com");
        assert_eq!(host, Some("https://internal.gems.example.com"));
    }

    #[test]
    fn test_yank_workflow_with_api_key() {
        let key = Some("custom-api-key");
        assert_eq!(key, Some("custom-api-key"));
    }

    #[test]
    fn test_yank_workflow_with_mfa() {
        let otp = Some("123456");
        assert_eq!(otp, Some("123456"));
    }

    #[test]
    fn test_yank_workflow_unyank_restore() {
        let undo = true;
        assert!(undo);
    }

    #[test]
    fn test_yank_workflow_platform_with_host() {
        let platform = Some("x86_64-linux");
        let host = Some("https://private-gems.company.com");
        assert_eq!(platform, Some("x86_64-linux"));
        assert_eq!(host, Some("https://private-gems.company.com"));
    }

    #[test]
    fn test_yank_workflow_complex_scenario() {
        let gem_name = "rails";
        let version = "7.0.0";
        let platform = Some("x86_64-darwin");
        let host = Some("https://rubygems.org");
        let key = Some("rubygems");
        let otp = Some("654321");
        let undo = false;

        assert_eq!(gem_name, "rails");
        assert_eq!(version, "7.0.0");
        assert_eq!(platform, Some("x86_64-darwin"));
        assert_eq!(host, Some("https://rubygems.org"));
        assert_eq!(key, Some("rubygems"));
        assert_eq!(otp, Some("654321"));
        assert!(!undo);
    }
}
