//! Signin command
//!
//! Sign in to RubyGems.org and save credentials

use anyhow::{Context, Result};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use reqwest::Client;
use serde::Deserialize;
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

/// Response from `RubyGems` API key endpoint
#[derive(Debug, Deserialize)]
struct ApiKeyResponse {
    rubygems_api_key: String,
}

/// Sign in to RubyGems.org and save API key
pub(crate) async fn run(host: Option<&str>) -> Result<()> {
    let credentials_path = get_credentials_path()?;

    // Warn if credentials already exist
    if credentials_path.exists() {
        println!("You are already signed in.");
        print!("Do you want to sign in again and overwrite existing credentials? (y/N): ");
        io::stdout().flush()?;

        let mut response = String::new();
        io::stdin().read_line(&mut response)?;

        if !response.trim().eq_ignore_ascii_case("y") {
            println!("Sign in cancelled.");
            return Ok(());
        }
    }

    // Get email from user
    print!("Email: ");
    io::stdout().flush()?;
    let mut email = String::new();
    io::stdin()
        .read_line(&mut email)
        .context("Failed to read email")?;
    let email = email.trim();

    if email.is_empty() {
        anyhow::bail!("Email cannot be empty");
    }

    // Get password from user (hidden input)
    let password = read_password()?;

    if password.is_empty() {
        anyhow::bail!("Password cannot be empty");
    }

    // Authenticate with RubyGems
    println!("\nAuthenticating...");
    let api_key = authenticate(email, &password, host).await?;

    // Save credentials
    save_credentials(&credentials_path, &api_key)?;

    println!("Signed in successfully!");
    println!("Credentials saved to: {}", credentials_path.display());

    Ok(())
}

/// Read password from stdin with hidden input
fn read_password() -> Result<String> {
    print!("Password: ");
    io::stdout().flush()?;

    // Disable echo for password input
    let password = if cfg!(unix) {
        read_password_hidden()?
    } else {
        // On Windows or if hiding fails, just read normally with a warning
        eprintln!("Warning: Password will be visible");
        let mut pass = String::new();
        io::stdin().read_line(&mut pass)?;
        pass
    };

    println!(); // New line after password input
    Ok(password.trim().to_string())
}

/// Read password with hidden input (Unix-specific)
#[cfg(unix)]
fn read_password_hidden() -> Result<String> {
    use crossterm::event::{Event, KeyCode, KeyEvent, read};

    enable_raw_mode().context("Failed to enable raw mode")?;

    let mut password = String::new();
    let result = (|| -> Result<String> {
        loop {
            if let Event::Key(KeyEvent { code, .. }) = read()? {
                match code {
                    KeyCode::Enter => break,
                    KeyCode::Char(c) => {
                        password.push(c);
                        print!("*");
                        io::stdout().flush()?;
                    }
                    KeyCode::Backspace => {
                        if password.pop().is_some() {
                            print!("\u{8} \u{8}"); // Backspace, space, backspace
                            io::stdout().flush()?;
                        }
                    }
                    _ => {}
                }
            }
        }
        Ok(password)
    })();

    disable_raw_mode().context("Failed to disable raw mode")?;
    result
}

/// Read password with visible input (Windows fallback)
#[cfg(not(unix))]
fn read_password_hidden() -> Result<String> {
    let mut password = String::new();
    io::stdin().read_line(&mut password)?;
    Ok(password)
}

/// Authenticate with `RubyGems` and get API key
async fn authenticate(email: &str, password: &str, host: Option<&str>) -> Result<String> {
    let base_url = host.unwrap_or(lode::RUBYGEMS_ORG_URL);
    let url = format!("{base_url}/api/v1/api_key.json");

    let client = Client::new();
    let response = client
        .post(&url)
        .basic_auth(email, Some(password))
        .send()
        .await
        .context("Failed to connect to RubyGems")?;

    let status = response.status();
    if !status.is_success() {
        match status.as_u16() {
            401 => anyhow::bail!("Authentication failed: Invalid email or password"),
            403 => anyhow::bail!(
                "Account access forbidden. This may require 2FA or have other restrictions."
            ),
            404 => anyhow::bail!("API endpoint not found. Check the host URL."),
            _ => anyhow::bail!("Authentication failed with status: {status}"),
        }
    }

    let api_response: ApiKeyResponse = response
        .json()
        .await
        .context("Failed to parse API response")?;

    Ok(api_response.rubygems_api_key)
}

/// Save API key to credentials file
fn save_credentials(credentials_path: &PathBuf, api_key: &str) -> Result<()> {
    // Create .gem directory if it doesn't exist
    if let Some(parent) = credentials_path.parent() {
        fs::create_dir_all(parent).context("Failed to create .gem directory")?;
    }

    // Write credentials in YAML format
    let content = format!("---\n:rubygems_api_key: {api_key}\n");
    fs::write(credentials_path, content).context("Failed to write credentials file")?;

    // Set permissions to 0600 (owner read/write only) on Unix
    #[cfg(unix)]
    {
        let metadata = fs::metadata(credentials_path)?;
        let mut permissions = metadata.permissions();
        permissions.set_mode(0o600);
        fs::set_permissions(credentials_path, permissions)
            .context("Failed to set credentials file permissions")?;
    }

    Ok(())
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
}
