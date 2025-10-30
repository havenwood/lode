//! Mirror command
//!
//! Manage gem mirror repositories

use anyhow::{Context, Result};
use std::env;
use std::fs;
use std::path::PathBuf;

/// Manage gem mirrors
pub(crate) fn run(add: Option<&str>, remove: Option<&str>, _list: bool, clear: bool) -> Result<()> {
    let mirrorrc_path = get_mirrorrc_path()?;

    // Handle clear operation
    if clear {
        if mirrorrc_path.exists() {
            fs::remove_file(&mirrorrc_path)
                .context("Failed to remove mirror configuration file")?;
            println!("Cleared all mirrors");
        } else {
            println!("No mirrors configured");
        }
        return Ok(());
    }

    // Load existing mirrors
    let mut mirrors = load_mirrors(&mirrorrc_path)?;

    // Handle add operation
    if let Some(url) = add {
        if mirrors.contains(&url.to_string()) {
            println!("Mirror already exists: {url}");
        } else {
            mirrors.push(url.to_string());
            save_mirrors(&mirrorrc_path, &mirrors)?;
            println!("Added mirror: {url}");
        }
        return Ok(());
    }

    // Handle remove operation
    if let Some(url) = remove {
        let original_len = mirrors.len();
        mirrors.retain(|m| m != url);

        if mirrors.len() == original_len {
            println!("Mirror not found: {url}");
        } else {
            save_mirrors(&mirrorrc_path, &mirrors)?;
            println!("Removed mirror: {url}");
        }
        return Ok(());
    }

    // Default: List mirrors (or explicit --list)
    if mirrors.is_empty() {
        println!("No mirrors configured");
        println!("\nAdd a mirror with: lode gem-mirror --add <URL>");
    } else {
        println!("Configured mirrors:\n");
        for (idx, mirror) in mirrors.iter().enumerate() {
            println!("  {}. {}", idx + 1, mirror);
        }
    }

    Ok(())
}

/// Get the path to the mirror configuration file
fn get_mirrorrc_path() -> Result<PathBuf> {
    let home = env::var("HOME")
        .or_else(|_| env::var("USERPROFILE"))
        .context("Could not determine home directory")?;

    let gem_dir = PathBuf::from(home).join(".gem");

    // Create .gem directory if it doesn't exist
    if !gem_dir.exists() {
        fs::create_dir_all(&gem_dir).context("Failed to create .gem directory")?;
    }

    Ok(gem_dir.join(".mirrorrc"))
}

/// Load mirrors from configuration file
fn load_mirrors(path: &PathBuf) -> Result<Vec<String>> {
    if !path.exists() {
        return Ok(Vec::new());
    }

    let content = fs::read_to_string(path).context("Failed to read mirror configuration")?;

    // Parse YAML-like format (simple line-based parsing)
    let mirrors: Vec<String> = content
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with("---") {
                None
            } else if let Some(rest) = trimmed.strip_prefix("- ") {
                Some(rest.trim().to_string())
            } else if let Some(rest) = trimmed.strip_prefix('-') {
                Some(rest.trim().to_string())
            } else {
                Some(trimmed.to_string())
            }
        })
        .collect();

    Ok(mirrors)
}

/// Save mirrors to configuration file
fn save_mirrors(path: &PathBuf, mirrors: &[String]) -> Result<()> {
    let content = if mirrors.is_empty() {
        String::from("---\n")
    } else {
        let mut yaml = String::from("---\n");
        for mirror in mirrors {
            use std::fmt::Write;
            let _ = writeln!(yaml, "- {mirror}");
        }
        yaml
    };

    fs::write(path, content).context("Failed to write mirror configuration")?;

    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "Tests can panic")]
mod tests {
    use super::*;

    #[test]
    fn test_get_mirrorrc_path() {
        let path = get_mirrorrc_path();
        assert!(path.is_ok());

        let path = path.unwrap();
        assert!(path.to_string_lossy().contains(".gem"));
        assert!(path.to_string_lossy().ends_with(".mirrorrc"));
    }

    #[test]
    fn load_empty_mirrors() {
        let temp_path = PathBuf::from("/tmp/nonexistent_mirror_test.yaml");
        let mirrors = load_mirrors(&temp_path).unwrap();
        assert!(mirrors.is_empty());
    }

    #[test]
    fn parse_yaml_mirrors() {
        let yaml_content = "---\n- https://mirror1.com\n- https://mirror2.com\n";
        let mirrors: Vec<String> = yaml_content
            .lines()
            .filter_map(|line| {
                let trimmed = line.trim();
                if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with("---") {
                    None
                } else if let Some(rest) = trimmed.strip_prefix("- ") {
                    Some(rest.trim().to_string())
                } else {
                    Some(trimmed.to_string())
                }
            })
            .collect();

        assert_eq!(mirrors.len(), 2);
        assert!(mirrors.contains(&"https://mirror1.com".to_string()));
        assert!(mirrors.contains(&"https://mirror2.com".to_string()));
    }
}
