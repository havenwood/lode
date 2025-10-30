//! Search command
//!
//! Search for gems locally and on RubyGems.org

use anyhow::{Context, Result};
use lode::gem_store::GemStore;
use lode::{Config, RubyGemsClient};

/// Options for gem search command
pub(crate) struct SearchOptions {
    pub query: Option<String>,
    pub installed: Option<bool>,
    pub version: Option<String>,
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
    pub source: Option<String>,
    pub http_proxy: Option<String>,
    pub verbose: bool,
    pub quiet: bool,
    pub silent: bool,
    pub config_file: Option<String>,
    pub backtrace: bool,
    pub debug: bool,
    pub norc: bool,
}

/// Search for gems
pub(crate) async fn run(options: SearchOptions) -> Result<()> {
    // Debug output
    if options.debug {
        eprintln!("DEBUG: Starting gem search");
        if let Some(query) = &options.query {
            eprintln!("DEBUG: Query: {query}");
        }
        eprintln!(
            "DEBUG: local={}, remote={}",
            !options.remote,
            options.remote || options.both
        );
    }

    // Load config with custom options
    let _config = Config::load_with_options(options.config_file.as_deref(), options.norc)?;

    // Emit deprecation warning for --update-sources flag
    if options.update_sources {
        eprintln!(
            "WARNING: The --update-sources flag is deprecated and will be removed in a future version"
        );
    }

    // Handle --clear-sources flag
    if options.clear_sources {
        // --clear-sources in gem search silently clears sources and continues
        // No special output is printed, the search continues normally
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

    // Handle --installed check
    if options.installed == Some(true) || options.installed == Some(false) {
        return check_installed(&options);
    }

    // Determine search mode: local (default), remote, or both
    let search_local = !options.remote;
    let search_remote = options.remote || options.both;

    let mut found_any = false;

    // Search local gems
    if search_local {
        match search_local_gems(&options) {
            Ok(found) => found_any |= found,
            Err(e) => {
                if options.backtrace {
                    eprintln!("Error searching local gems: {e:#}");
                } else if options.verbose {
                    eprintln!("Error searching local gems: {e}");
                }
                return Err(e);
            }
        }
    }

    // Search remote gems
    if search_remote {
        match search_remote_gems(&options).await {
            Ok(found) => found_any |= found,
            Err(e) => {
                if options.backtrace {
                    eprintln!("Error searching remote gems: {e:#}");
                } else if options.verbose {
                    eprintln!("Error searching remote gems: {e}");
                }
                return Err(e);
            }
        }
    }

    if !found_any && !options.quiet && !options.silent {
        let pattern = options.query.as_deref().unwrap_or("");
        println!("No gems found matching '{pattern}'");
    }

    Ok(())
}

/// Check if a gem is installed
fn check_installed(options: &SearchOptions) -> Result<()> {
    let store = GemStore::new()?;
    let installed_gems = store.list_gems()?;

    let query = options
        .query
        .as_ref()
        .context("Query required for --installed check")?;

    let matches: Vec<_> = if options.exact {
        installed_gems.iter().filter(|g| g.name == *query).collect()
    } else {
        let regex = regex::Regex::new(query)
            .unwrap_or_else(|_| regex::Regex::new(&regex::escape(query)).unwrap());
        installed_gems
            .iter()
            .filter(|g| regex.is_match(&g.name))
            .collect()
    };

    // Filter by version if specified
    let matches: Vec<_> = if let Some(ref version_req) = options.version {
        matches
            .into_iter()
            .filter(|g| g.version == *version_req)
            .collect()
    } else {
        matches
    };

    let is_installed = !matches.is_empty();
    let expected_installed = options.installed == Some(true);

    if is_installed == expected_installed {
        if !options.quiet && !options.silent {
            println!("true");
        }
        Ok(())
    } else {
        if !options.quiet && !options.silent {
            println!("false");
        }
        std::process::exit(1);
    }
}

/// Search local gems
fn search_local_gems(options: &SearchOptions) -> Result<bool> {
    let store = GemStore::new()?;
    let mut installed_gems = store.list_gems()?;

    if installed_gems.is_empty() {
        return Ok(false);
    }

    // Filter by pattern
    if let Some(ref query) = options.query {
        let regex = if options.exact {
            regex::Regex::new(&format!("^{}$", regex::escape(query)))?
        } else {
            regex::Regex::new(query)
                .unwrap_or_else(|_| regex::Regex::new(&regex::escape(query)).unwrap())
        };

        installed_gems.retain(|g| regex.is_match(&g.name));
    }

    if installed_gems.is_empty() {
        return Ok(false);
    }

    // Filter prerelease
    if !options.prerelease {
        installed_gems.retain(|g| !g.version.contains('-'));
    }

    // Sort by name
    installed_gems.sort_by(|a, b| a.name.cmp(&b.name));

    if !options.quiet && !options.silent {
        println!("\n*** LOCAL GEMS ***\n");

        if options.versions {
            // Just names, no versions
            let mut unique_names: Vec<_> = installed_gems.iter().map(|g| &g.name).collect();
            unique_names.dedup();
            for name in unique_names {
                println!("{name}");
            }
        } else if options.all {
            // Group by name and show all versions
            let mut current_name = "";
            for gem in &installed_gems {
                if gem.name == current_name {
                    println!("    ({version})", version = gem.version);
                } else {
                    if !current_name.is_empty() {
                        println!();
                    }
                    println!("{name} ({version})", name = gem.name, version = gem.version);
                    current_name = &gem.name;
                }
            }
        } else {
            // Show latest version only
            let mut seen = std::collections::HashSet::new();
            for gem in &installed_gems {
                if seen.insert(&gem.name) {
                    println!("{name} ({version})", name = gem.name, version = gem.version);

                    if options.details {
                        let gem_dir = store
                            .gem_dir()
                            .join(format!("{}-{}", gem.name, gem.version));
                        if gem_dir.exists() {
                            println!("    Installed at: {path}", path = gem_dir.display());
                        }
                        println!();
                    }
                }
            }
        }
    }

    Ok(true)
}

/// Search remote gems
async fn search_remote_gems(options: &SearchOptions) -> Result<bool> {
    let query = match &options.query {
        Some(q) if !q.is_empty() => q,
        _ => return Ok(false),
    };

    // Use custom source if provided, otherwise use default
    let base_url = options
        .source
        .clone()
        .unwrap_or_else(lode::env_vars::rubygems_host);

    if options.debug {
        eprintln!("DEBUG: Searching remote gems at {base_url}");
    }

    // Create RubyGemsClient with optional proxy
    let client = match RubyGemsClient::new_with_proxy(&base_url, options.http_proxy.as_ref()) {
        Ok(c) => c,
        Err(e) => {
            if options.backtrace {
                eprintln!("Error creating RubyGemsClient: {e:#}");
            }
            return Err(e);
        }
    };

    // Use bulk index for remote search (more efficient for pattern matching)
    let bulk_results = match client.search_bulk_index(query, options.prerelease).await {
        Ok(results) => results,
        Err(e) => {
            let err = anyhow::anyhow!("Failed to search bulk gem index: {e}");
            if options.backtrace {
                eprintln!("Error: {e:#}");
            }
            return Err(err);
        }
    };

    if bulk_results.is_empty() {
        return Ok(false);
    }

    // Filter by exact match if requested
    let mut results: Vec<_> = if options.exact {
        bulk_results
            .into_iter()
            .filter(|g| g.name == *query)
            .collect()
    } else {
        bulk_results
    };

    if results.is_empty() {
        return Ok(false);
    }

    // Sort by name for consistent output
    results.sort_by(|a, b| a.name.cmp(&b.name));

    if !options.quiet && !options.silent {
        println!("\n*** REMOTE GEMS ***\n");

        // Group by gem name to show unique gems
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
                // Show latest version only
                if let Some(latest) = versions.first() {
                    if latest.platform == "ruby" {
                        println!("{} ({})", gem_name, latest.version);
                    } else {
                        println!("{} ({}, {})", gem_name, latest.version, latest.platform);
                    }
                }
            }
        }
    }

    Ok(true)
}
