//! Stale command
//!
//! List gems by last access time

use anyhow::{Context, Result};
use lode::{gem_store::GemStore, parse_gem_name};
use std::fs;
use std::path::PathBuf;
use std::time::SystemTime;

/// Options for gem-stale command
#[derive(Debug, Copy, Clone)]
pub(crate) struct StaleOptions {
    pub verbose: bool,
    pub quiet: bool,
    pub silent: bool,
}

/// Gem with access time information
#[derive(Debug)]
struct GemAccessInfo {
    name: String,
    version: String,
    last_access: SystemTime,
}

/// List gems sorted by last access time (oldest first)
pub(crate) fn run_with_options(options: StaleOptions) -> Result<()> {
    let store = GemStore::new().context("Failed to initialize gem store")?;
    let gem_dir = store.gem_dir().to_path_buf();

    if !gem_dir.exists() {
        if !options.silent && !options.quiet {
            println!(
                "Gem directory does not exist: {path}",
                path = gem_dir.display()
            );
        }
        return Ok(());
    }

    // Collect gems with access time
    let mut gems_with_access = Vec::new();

    let entries = fs::read_dir(&gem_dir).with_context(|| {
        format!(
            "Failed to read gem directory: {path}",
            path = gem_dir.display()
        )
    })?;

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        if let Some(dir_name) = path.file_name().and_then(|n| n.to_str())
            && let Some((name, version)) = parse_gem_name(dir_name)
        {
            // Get last access time
            if let Ok(last_access) = get_last_access_time(&path) {
                gems_with_access.push(GemAccessInfo {
                    name: name.to_string(),
                    version: version.to_string(),
                    last_access,
                });
            }
        }
    }

    if gems_with_access.is_empty() {
        if !options.silent && !options.quiet {
            println!("No gems found");
        }
        return Ok(());
    }

    // Sort by access time (oldest first)
    gems_with_access.sort_by_key(|g| g.last_access);

    // Don't output anything in silent mode
    if options.silent {
        return Ok(());
    }

    if !options.quiet {
        println!("Gems sorted by last access time (oldest first):\n");
    }

    for gem in &gems_with_access {
        let days_ago = days_since_access(&gem.last_access);
        if options.verbose {
            println!(
                "{name} ({version}) - {days} days ago (last accessed: {last_access:?})",
                name = gem.name,
                version = gem.version,
                days = days_ago,
                last_access = gem.last_access
            );
        } else {
            println!(
                "{name} ({version}) - {days} days ago",
                name = gem.name,
                version = gem.version,
                days = days_ago
            );
        }
    }

    if !options.quiet {
        println!("\n{count} gem(s) total", count = gems_with_access.len());
    }

    Ok(())
}

/// Get last access time for a gem directory
fn get_last_access_time(path: &PathBuf) -> Result<SystemTime> {
    let metadata = fs::metadata(path)
        .with_context(|| format!("Failed to read metadata for {}", path.display()))?;

    metadata
        .accessed()
        .with_context(|| format!("Failed to get access time for {}", path.display()))
}

/// Calculate days since last access
fn days_since_access(access_time: &SystemTime) -> u64 {
    let now = SystemTime::now();

    now.duration_since(*access_time)
        .map_or(0, |duration| duration.as_secs() / 86_400)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "Tests can panic")]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_days_since_access() {
        let now = SystemTime::now();
        assert_eq!(days_since_access(&now), 0);

        let two_days_ago = now
            .checked_sub(std::time::Duration::from_secs(2 * 86_400))
            .unwrap();
        assert_eq!(days_since_access(&two_days_ago), 2);

        let one_hour_ago = now
            .checked_sub(std::time::Duration::from_secs(3600))
            .unwrap();
        assert_eq!(days_since_access(&one_hour_ago), 0);

        let thirty_days_ago = now
            .checked_sub(std::time::Duration::from_secs(30 * 86_400))
            .unwrap();
        assert_eq!(days_since_access(&thirty_days_ago), 30);
    }

    #[test]
    fn test_get_last_access_time() {
        let temp_dir = TempDir::new().unwrap();
        let test_dir = temp_dir.path().join("test-gem-1.0.0");
        fs::create_dir(&test_dir).unwrap();

        let result = get_last_access_time(&test_dir);
        assert!(result.is_ok());

        let access_time = result.unwrap();
        let now = SystemTime::now();
        assert!(access_time <= now);
    }

    #[test]
    fn test_get_last_access_time_nonexistent() {
        let nonexistent = std::path::PathBuf::from("/nonexistent/path/to/gem");
        let result = get_last_access_time(&nonexistent);
        assert!(result.is_err());
    }

    #[test]
    fn test_gem_access_info_structure() {
        let now = SystemTime::now();
        let info = GemAccessInfo {
            name: "rake".to_string(),
            version: "13.0.0".to_string(),
            last_access: now,
        };

        assert_eq!(info.name, "rake");
        assert_eq!(info.version, "13.0.0");
        assert_eq!(info.last_access, now);
    }

    #[test]
    fn test_run_with_options_verbose() {
        let options = StaleOptions {
            verbose: true,
            quiet: false,
            silent: false,
        };
        let result = run_with_options(options);
        assert!(result.is_ok());
    }

    #[test]
    fn test_run_with_options_quiet() {
        let options = StaleOptions {
            verbose: false,
            quiet: true,
            silent: false,
        };
        let result = run_with_options(options);
        assert!(result.is_ok());
    }

    #[test]
    fn test_run_with_options_silent() {
        let options = StaleOptions {
            verbose: false,
            quiet: false,
            silent: true,
        };
        let result = run_with_options(options);
        assert!(result.is_ok());
    }

    #[test]
    fn test_stale_options_defaults() {
        let options = StaleOptions {
            verbose: false,
            quiet: false,
            silent: false,
        };
        assert!(!options.verbose);
        assert!(!options.quiet);
        assert!(!options.silent);
    }
}
