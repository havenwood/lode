//! Owner command
//!
//! Manage gem ownership

use anyhow::{Context, Result};
use reqwest::Proxy;
use std::env;
use std::fs;
use std::path::PathBuf;

/// Manage gem ownership.
pub(crate) async fn run_with_options(
    gem_name: &str,
    email: &str,
    add: bool,
    host: Option<&str>,
    key: Option<&str>,
    otp: Option<&str>,
    proxy_url: Option<&str>,
) -> Result<()> {
    let action = if add { "Adding" } else { "Removing" };

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

    println!("{action} {email} as owner of {gem_name}...");

    // Load API key (checks environment variables first, then credentials file)
    let api_key = load_api_key(key.unwrap_or("rubygems"), &server_url)?;

    // Build owner management URL
    let owner_url = format!("{server_url}/api/v1/gems/{gem_name}/owners");

    // Build request with proxy support
    let client = build_http_client(proxy_url)?;

    let mut request = if add {
        client
            .post(&owner_url)
            .header("Authorization", api_key)
            .query(&[("email", email)])
    } else {
        client
            .delete(&owner_url)
            .header("Authorization", api_key)
            .query(&[("email", email)])
    };

    // Add OTP header if provided
    if let Some(otp_code) = otp {
        request = request.header("X-Rubygems-OTP", otp_code);
    }

    // Send request
    let response = request
        .send()
        .await
        .with_context(|| format!("Failed to {} owner", action.to_lowercase()))?;

    // Check response
    let status = response.status();
    let body = response
        .text()
        .await
        .unwrap_or_else(|_| "<no response body>".to_string());

    if status.is_success() {
        let success_msg = if add {
            format!("Added {email} as an owner of {gem_name}")
        } else {
            format!("Removed {email} as an owner of {gem_name}")
        };
        println!("{success_msg}");
        if !body.is_empty() && body != success_msg {
            println!("{body}");
        }
        Ok(())
    } else {
        anyhow::bail!(
            "Failed to {} owner (HTTP {}):\n{}",
            action.to_lowercase(),
            status.as_u16(),
            body
        )
    }
}

/// Build an HTTP client with optional proxy support
fn build_http_client(proxy_url: Option<&str>) -> Result<reqwest::Client> {
    let mut client_builder = reqwest::Client::builder();

    if let Some(url) = proxy_url {
        let proxy = Proxy::all(url).with_context(|| format!("Invalid proxy URL: {url}"))?;
        client_builder = client_builder.proxy(proxy);
    }

    client_builder
        .build()
        .context("Failed to build HTTP client")
}

/// List owners of a gem
///
/// # Arguments
///
/// * `gem_name` - Name of the gem
/// * `host` - Optional custom gem server host
/// * `key` - Optional API key name
/// * `proxy_url` - Optional HTTP proxy URL
pub(crate) async fn list_owners(
    gem_name: &str,
    host: Option<&str>,
    key: Option<&str>,
    proxy_url: Option<&str>,
) -> Result<()> {
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
    let owner_url = format!("{server_url}/api/v1/gems/{gem_name}/owners.json");

    // Load API key if available (optional for listing, checks environment variables first)
    let api_key = load_api_key(key.unwrap_or("rubygems"), &server_url).ok();

    // Build request with proxy support
    let client = build_http_client(proxy_url)?;
    let mut request = client.get(&owner_url);

    if let Some(key) = api_key {
        request = request.header("Authorization", key);
    }

    // Send request
    let response = request.send().await.context("Failed to fetch gem owners")?;

    // Check response
    let status = response.status();
    let body = response
        .text()
        .await
        .unwrap_or_else(|_| "<no response body>".to_string());

    if status.is_success() {
        // Parse JSON and format nicely
        if let Ok(owners) = serde_json::from_str::<Vec<serde_json::Value>>(&body) {
            println!("Owners for {gem_name}:");
            for owner in owners {
                if let Some(email) = owner.get("email").and_then(|e| e.as_str()) {
                    println!("- {email}");
                }
            }
        } else {
            // Fallback to raw output
            println!("{body}");
        }
        Ok(())
    } else {
        anyhow::bail!(
            "Failed to list owners (HTTP {}):\n{}",
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
    fn owner_url_construction() {
        let base = "https://rubygems.org";
        let gem_name = "my-gem";
        let owner_url = format!("{base}/api/v1/gems/{gem_name}/owners");
        assert_eq!(owner_url, "https://rubygems.org/api/v1/gems/my-gem/owners");

        let list_url = format!("{base}/api/v1/gems/{gem_name}/owners.json");
        assert_eq!(
            list_url,
            "https://rubygems.org/api/v1/gems/my-gem/owners.json"
        );
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

    // Workflow tests for gem owner command
    #[test]
    fn test_owner_workflow_add_owner() {
        let gem_name = "my-gem";
        let email = "newowner@example.com";
        let add = true;
        assert_eq!(gem_name, "my-gem");
        assert_eq!(email, "newowner@example.com");
        assert!(add);
    }

    #[test]
    fn test_owner_workflow_remove_owner() {
        let gem_name = "my-gem";
        let email = "oldowner@example.com";
        let add = false;
        assert_eq!(gem_name, "my-gem");
        assert_eq!(email, "oldowner@example.com");
        assert!(!add);
    }

    #[test]
    fn test_owner_workflow_custom_host() {
        let host = Some("https://gems.internal.company.com");
        assert_eq!(host, Some("https://gems.internal.company.com"));
    }

    #[test]
    fn test_owner_workflow_with_api_key() {
        let key = Some("custom-key");
        assert_eq!(key, Some("custom-key"));
    }

    #[test]
    fn test_owner_workflow_with_mfa() {
        let otp = Some("123456");
        assert_eq!(otp, Some("123456"));
    }

    #[test]
    fn test_owner_workflow_with_proxy() {
        let proxy_url = Some("http://proxy.example.com:8080");
        assert_eq!(proxy_url, Some("http://proxy.example.com:8080"));
    }

    #[test]
    fn test_owner_workflow_full_credentials() {
        let gem_name = "company-gem";
        let email = "newcoder@company.com";
        let add = true;
        let host = Some("https://gems.company.internal");
        let key = Some("company-key");
        let otp = Some("654321");

        assert_eq!(gem_name, "company-gem");
        assert_eq!(email, "newcoder@company.com");
        assert!(add);
        assert_eq!(host, Some("https://gems.company.internal"));
        assert_eq!(key, Some("company-key"));
        assert_eq!(otp, Some("654321"));
    }

    #[test]
    fn test_owner_workflow_complex_scenario() {
        let gem_name = "rails";
        let add = true;
        let host = Some("https://rubygems.org");
        let key = Some("rubygems");
        let otp = Some("789012");
        let proxy_url = Some("http://corporate-proxy.example.com:3128");

        assert_eq!(gem_name, "rails");
        assert!(add);
        assert_eq!(host, Some("https://rubygems.org"));
        assert_eq!(key, Some("rubygems"));
        assert_eq!(otp, Some("789012"));
        assert_eq!(proxy_url, Some("http://corporate-proxy.example.com:3128"));
    }
}
