//! List command
//!
//! List installed gems

use anyhow::{Context, Result};
use lode::gem_store::GemStore;
use lode::{Config, RubyGemsClient};
use std::process;

/// Options for gem list command
#[derive(Debug, Clone)]
pub(crate) struct ListOptions<'a> {
    pub pattern: Option<&'a str>,
    pub installed: Option<bool>,
    pub version: Option<&'a str>,
    pub details: bool,
    pub versions: bool,
    pub all: bool,
    pub exact: bool,
    pub prerelease: bool,
    pub update_sources: bool,
    #[allow(dead_code)]
    pub local: bool,
    pub remote: bool,
    pub both: bool,
    pub bulk_threshold: usize,
    pub clear_sources: bool,
    pub source: Option<&'a str>,
    pub http_proxy: Option<&'a str>,
    pub verbose: bool,
    pub quiet: bool,
    pub silent: bool,
    pub config_file: Option<&'a str>,
    pub backtrace: bool,
    pub debug: bool,
    pub norc: bool,
}

/// Run the gem list command
pub(crate) async fn run(options: ListOptions<'_>) -> Result<()> {
    // Debug output
    if options.debug {
        eprintln!("DEBUG: Starting gem list");
        if let Some(pattern) = options.pattern {
            eprintln!("DEBUG: Pattern: {pattern}");
        }
        eprintln!(
            "DEBUG: local={}, remote={}",
            !options.remote,
            options.remote || options.both
        );
    }

    // Load config with custom options
    let _config = Config::load_with_options(options.config_file, options.norc)?;

    // Emit deprecation warning for --update-sources flag
    if options.update_sources {
        eprintln!(
            "WARNING: The --update-sources flag is deprecated and will be removed in a future version"
        );
    }

    // Handle --clear-sources flag
    if options.clear_sources {
        // --clear-sources silently clears sources and continues listing
        // No special output is printed
        if options.debug {
            eprintln!("DEBUG: --clear-sources flag set (sources cleared)");
        }
    }

    // Handle --bulk-threshold flag
    if options.debug {
        eprintln!(
            "DEBUG: --bulk-threshold set to {} (used for bulk API operations)",
            options.bulk_threshold
        );
    }

    // Handle --http-proxy flag
    if let Some(proxy) = options.http_proxy
        && options.debug
    {
        eprintln!("DEBUG: --http-proxy set to: {proxy}");
    }

    // Determine if we're doing a check (--installed) or a list
    let is_check = options.installed.is_some();

    // Determine mode: local (default), remote, or both
    let show_remote = options.remote || options.both;
    let show_local = !options.remote || options.both;

    // For --installed checks, we only search locally
    if is_check {
        return check_installed(&options);
    }

    // List gems
    if show_local && let Err(e) = list_local_gems(&options) {
        if options.backtrace {
            eprintln!("Error listing local gems: {e:#}");
        } else {
            eprintln!("Error listing local gems: {e}");
        }
        return Err(e);
    }

    if show_remote && let Err(e) = list_remote_gems(&options).await {
        if options.backtrace {
            eprintln!("Error listing remote gems: {e:#}");
        } else {
            eprintln!("Error listing remote gems: {e}");
        }
        return Err(e);
    }

    Ok(())
}

/// Check if a gem is installed (--installed flag)
fn check_installed(options: &ListOptions<'_>) -> Result<()> {
    let store = GemStore::new()?;
    let pattern = options.pattern.unwrap_or("");

    if pattern.is_empty() {
        eprintln!("Error: --installed requires a gem name");
        process::exit(1);
    }

    // Find matching gems
    let gems = store.find_gems(Some(pattern))?;

    // Filter by exact match if requested
    let gems: Vec<_> = if options.exact {
        gems.into_iter().filter(|g| g.name == pattern).collect()
    } else {
        gems
    };

    // Filter by version if specified
    let gems: Vec<_> = if let Some(version) = options.version {
        gems.into_iter().filter(|g| g.version == version).collect()
    } else {
        gems
    };

    // Check result
    let is_installed = !gems.is_empty();
    let should_be_installed = options.installed.unwrap_or(true);

    if is_installed == should_be_installed {
        if !options.silent && options.verbose {
            if should_be_installed {
                if let Some(version) = options.version {
                    println!("{pattern} ({version}) is installed");
                } else {
                    println!("{pattern} is installed");
                }
            } else {
                println!("{pattern} is not installed");
            }
        }
        process::exit(0);
    } else {
        if !options.silent && options.verbose {
            if should_be_installed {
                println!("{pattern} is not installed");
            } else {
                println!("{pattern} is installed");
            }
        }
        process::exit(1);
    }
}

/// List local gems
fn list_local_gems(options: &ListOptions<'_>) -> Result<()> {
    if options.silent {
        return Ok(());
    }

    let store = GemStore::new()?;
    let mut gems = store.find_gems(options.pattern)?;

    // Filter by exact match if requested
    if options.exact
        && let Some(pattern) = options.pattern
    {
        gems.retain(|g| g.name == pattern);
    }

    // Filter by prerelease
    if !options.prerelease {
        gems.retain(|g| !is_prerelease(&g.version));
    }

    if gems.is_empty() {
        if let Some(pattern) = options.pattern {
            if !options.quiet {
                println!("No gems matching '{pattern}'");
            }
        } else if !options.quiet {
            println!("No gems installed");
        }
        return Ok(());
    }

    if !options.quiet {
        println!("\n*** LOCAL GEMS ***\n");
        if let Some(pattern) = options.pattern {
            if options.exact {
                println!("Gems exactly matching '{pattern}':\n");
            } else {
                println!("Gems matching '{pattern}':\n");
            }
        }
    }

    // Display gems
    if options.versions {
        // Only show names
        let mut names: Vec<String> = gems.iter().map(|g| g.name.clone()).collect();
        names.sort();
        names.dedup();
        for name in names {
            println!("{name}");
        }
    } else if options.details {
        // Show detailed information
        display_detailed_gems(&gems, options);
    } else if options.all {
        // Show all versions
        display_all_versions(&gems, options);
    } else {
        // Show latest version only (default)
        display_latest_versions(&gems, options);
    }

    if !options.quiet && !options.versions {
        println!();
    }

    Ok(())
}

/// List remote gems from RubyGems.org
async fn list_remote_gems(options: &ListOptions<'_>) -> Result<()> {
    if options.silent {
        return Ok(());
    }

    let pattern = options.pattern.unwrap_or("");

    if pattern.is_empty() {
        if !options.quiet {
            eprintln!("Error: Remote listing requires a gem name pattern");
        }
        return Ok(());
    }

    // Use custom source if provided, otherwise default to RUBYGEMS_HOST env var (or rubygems.org)
    let base_url = options.source.map_or_else(
        lode::env_vars::rubygems_host,
        std::string::ToString::to_string,
    );

    // Create RubyGemsClient with optional proxy
    let client = RubyGemsClient::new_with_proxy(&base_url, options.http_proxy)?;

    // Use bulk index for remote listing (more efficient for pattern matching)
    let bulk_results = client
        .search_bulk_index(pattern, options.prerelease)
        .await
        .context("Failed to search bulk gem index")?;

    // Filter by exact match if requested
    let mut results: Vec<_> = if options.exact {
        bulk_results
            .into_iter()
            .filter(|g| g.name == pattern)
            .collect()
    } else {
        bulk_results
    };

    // Sort by name for consistent output
    results.sort_by(|a, b| a.name.cmp(&b.name));

    if results.is_empty() {
        if !options.quiet {
            println!("\n*** REMOTE GEMS ***\n");
            println!("No gems matching '{pattern}' on RubyGems.org");
        }
        return Ok(());
    }

    if !options.quiet {
        println!("\n*** REMOTE GEMS ***\n");
        if options.exact {
            println!("Gems exactly matching '{pattern}':\n");
        } else {
            println!("Gems matching '{pattern}':\n");
        }
    }

    // Group results by gem name to show all versions
    let mut gems_by_name: std::collections::HashMap<String, Vec<_>> =
        std::collections::HashMap::new();
    for spec in results {
        gems_by_name
            .entry(spec.name.clone())
            .or_default()
            .push(spec);
    }

    // Sort gem names
    let mut gem_names: Vec<_> = gems_by_name.keys().cloned().collect();
    gem_names.sort();

    // Display results
    for gem_name in &gem_names {
        let versions = gems_by_name
            .get(gem_name)
            .expect("gem_name came from keys, must exist");

        if options.versions {
            // Only show names
            println!("{gem_name}");
        } else if options.all {
            // Show all versions
            println!("{gem_name}");
            for spec in versions {
                if spec.platform == "ruby" {
                    println!("    ({})", spec.version);
                } else {
                    println!("    ({}, {})", spec.version, spec.platform);
                }
            }
        } else {
            // Show latest version only (first in list after sorting)
            if let Some(latest) = versions.first() {
                if latest.platform == "ruby" {
                    println!("{} ({})", gem_name, latest.version);
                } else {
                    println!("{} ({}, {})", gem_name, latest.version, latest.platform);
                }
            }
        }
    }

    if !options.quiet && !options.versions {
        println!();
    }

    Ok(())
}

/// Display gems with detailed information
fn display_detailed_gems(gems: &[lode::gem_store::InstalledGem], _options: &ListOptions<'_>) {
    let mut current_name: Option<String> = None;

    for gem in gems {
        // Group by gem name
        if current_name.as_ref() != Some(&gem.name) {
            current_name = Some(gem.name.clone());
            println!("{} ({})", gem.name, gem.version);

            // Try to load gemspec for detailed info
            // Construct gemspec path: {parent_dir}/specifications/{name}-{version}.gemspec
            if let Some(parent) = gem.path.parent()
                && let Some(grandparent) = parent.parent()
            {
                let spec_path = grandparent
                    .join("specifications")
                    .join(format!("{}-{}.gemspec", gem.name, gem.version));

                if let Ok(content) = std::fs::read_to_string(&spec_path) {
                    // Parse gemspec YAML for summary, homepage, authors
                    for line in content.lines() {
                        if line.contains("summary:") {
                            let summary = line.split("summary:").nth(1).unwrap_or("").trim();
                            println!("    Summary: {}", summary.trim_matches('"'));
                        } else if line.contains("homepage:") {
                            let homepage = line.split("homepage:").nth(1).unwrap_or("").trim();
                            println!("    Homepage: {}", homepage.trim_matches('"'));
                        } else if line.contains("authors:") {
                            let authors = line.split("authors:").nth(1).unwrap_or("").trim();
                            println!(
                                "    Authors: {}",
                                authors.trim_matches(&['[', ']', '"'][..])
                            );
                        }
                    }
                }
            }

            println!("    Installed at: {}", gem.path.display());

            if gem.platform != "ruby" {
                println!("    Platform: {}", gem.platform);
            }
            println!();
        }
    }
}

/// Display all gem versions
fn display_all_versions(gems: &[lode::gem_store::InstalledGem], options: &ListOptions<'_>) {
    let mut current_name: Option<String> = None;

    for gem in gems {
        // Group by gem name
        if current_name.as_ref() != Some(&gem.name) {
            current_name = Some(gem.name.clone());
            if !options.quiet {
                println!("\n{}", gem.name);
            }
        }

        // Show version and platform
        if gem.platform == "ruby" {
            println!("    {}", gem.version);
        } else {
            println!("    {} ({})", gem.version, gem.platform);
        }
    }
}

/// Display only the latest version of each gem (default behavior)
fn display_latest_versions(gems: &[lode::gem_store::InstalledGem], _options: &ListOptions<'_>) {
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();

    for gem in gems {
        if seen.insert(gem.name.clone()) {
            // First occurrence (latest version due to GemStore sorting)
            if gem.platform == "ruby" {
                println!("{} ({})", gem.name, gem.version);
            } else {
                println!("{} ({}, {})", gem.name, gem.version, gem.platform);
            }
        }
    }
}

/// Check if a version string is a prerelease
fn is_prerelease(version: &str) -> bool {
    version.contains('-')
        || version.contains(".pre")
        || version.contains(".alpha")
        || version.contains(".beta")
        || version.contains(".rc")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Detects standard prerelease version patterns
    #[test]
    fn test_is_prerelease() {
        assert!(is_prerelease("1.0.0-beta"));
        assert!(is_prerelease("1.0.0.pre"));
        assert!(is_prerelease("1.0.0.alpha"));
        assert!(is_prerelease("1.0.0.rc1"));
        assert!(is_prerelease("1.0.0-alpha.1"));
        assert!(!is_prerelease("1.0.0"));
        assert!(!is_prerelease("2.5.1"));
    }

    /// Detects prerelease versions with various format patterns
    #[test]
    fn test_is_prerelease_various_formats() {
        assert!(is_prerelease("1.0.0-rc"));
        assert!(is_prerelease("1.0.0-rc.1"));
        assert!(is_prerelease("2.0.0.pre"));
        assert!(is_prerelease("1.0.0.alpha"));
        assert!(is_prerelease("1.0.0.beta"));

        assert!(!is_prerelease("1.0.0"));
        assert!(!is_prerelease("1.2.3"));
        assert!(!is_prerelease("10.0.0"));
    }
}
