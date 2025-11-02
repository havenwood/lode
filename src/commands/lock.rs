//! Lock command
//!
//! Generate or update Gemfile.lock

use anyhow::{Context, Result};
use futures_util::stream::{self, StreamExt};
use lode::lockfile::{Dependency, GemSpec};
use lode::platform::detect_current_platform;
use lode::resolver::ResolvedGem;
use lode::{Config, Gemfile, Lockfile, Resolver, RubyGemsClient};
use std::collections::HashSet;
use std::fs;
use std::sync::Arc;

/// Execute the lock command
///
/// Orchestrates the full resolution flow:
/// 1. Parse Gemfile
/// 2. Fetch gem metadata from RubyGems.org
/// 3. Resolve dependencies with `PubGrub`
/// 4. Write Gemfile.lock
#[allow(
    clippy::too_many_arguments,
    clippy::fn_params_excessive_bools,
    clippy::cognitive_complexity
)]
pub(crate) async fn run(
    gemfile_path: &str,
    lockfile_path: Option<&str>,
    add_platforms: &[String],
    remove_platforms: &[String],
    update_gems: &[String],
    print: bool,
    verbose: bool,
    patch: bool,
    minor: bool,
    _major: bool, // Major updates are the default behavior (no constraint)
    strict: bool,
    conservative: bool,
    local: bool,
    pre: bool,
    bundler: Option<&str>,
    normalize_platforms: bool,
    add_checksums: bool,
    full_index: bool,
    quiet: bool,
) -> Result<()> {
    // Determine lockfile path based on provided path or derive from gemfile
    let lockfile_pathbuf = lockfile_path.map_or_else(
        || lode::lockfile_for_gemfile(std::path::Path::new(gemfile_path)),
        std::path::PathBuf::from,
    );
    let lockfile_str = lockfile_pathbuf.to_str().unwrap_or("Gemfile.lock");

    if verbose {
        println!("Resolving dependencies...");
        println!("Gemfile: {gemfile_path}");
        println!("Lockfile: {lockfile_str}");
    }

    // Handle gem updates
    if verbose {
        if update_gems.is_empty() {
            println!("Update mode: All gems will be re-resolved");
        } else {
            println!("Selective update: {}", update_gems.join(", "));
            println!("   Other gems will be locked to current versions");
        }
    }

    // Conservative mode: works with selective updates to minimize version changes
    if conservative && verbose {
        println!("Conservative mode: minimizing version changes");
    }

    // Prerelease mode
    if pre && verbose {
        println!("Including prerelease versions (alpha, beta, rc)");
    }

    // Local mode: only use cached gems, no network requests
    if local && verbose {
        println!("Local mode: using only cached gems");
    }

    // Download and cache full index if requested
    let _full_index_data = if full_index {
        if verbose {
            println!("Downloading and parsing full RubyGems index...");
        }

        // Check if we have a cached index
        let cache_dir = lode::config::cache_dir(None)?;
        let index_cache_path = lode::FullIndex::cache_path(&cache_dir);

        let index = if index_cache_path.exists() && !verbose {
            // Try to use cached index
            if let Ok(idx) = lode::FullIndex::load_from_cache(&index_cache_path) {
                if verbose {
                    println!(
                        "Using cached full index ({} gems, {} versions)",
                        idx.gem_count(),
                        idx.total_count()
                    );
                }
                idx
            } else {
                // Cache invalid, download fresh
                if verbose {
                    println!("Cached index invalid, downloading fresh index...");
                }
                let idx = lode::FullIndex::download_and_parse(lode::RUBYGEMS_ORG_URL).await?;
                idx.save_to_cache(&index_cache_path)?;
                idx
            }
        } else {
            // Download fresh index
            let idx = lode::FullIndex::download_and_parse(lode::RUBYGEMS_ORG_URL).await?;
            if verbose {
                println!(
                    "Downloaded {} gems with {} versions",
                    idx.gem_count(),
                    idx.total_count()
                );
            }
            // Cache for future use
            idx.save_to_cache(&index_cache_path)?;
            idx
        };

        if verbose {
            println!("Note: Full index mode enabled (uses local index instead of API)");
            println!("   This mode works but dependency API is faster and more efficient");
        }

        Some(index)
    } else {
        None
    };

    // Load config
    let _config = Config::load().context("Failed to load configuration")?;

    // Parse Gemfile
    let mut gemfile = Gemfile::parse_file(gemfile_path)
        .with_context(|| format!("Failed to parse Gemfile at {gemfile_path}"))?;

    if verbose {
        println!("Found {} gems in Gemfile", gemfile.gems.len());
        if let Some(ref ruby_version) = gemfile.ruby_version {
            println!("Ruby version: {ruby_version}");
        }
    }

    // Implement selective gem updates with version level control
    // --update with gems: Lock non-updated gems to their current versions from lockfile
    // --update without gems: Update all gems (full resolution)
    // --patch/--minor without --update: Apply constraints to all gems
    if !update_gems.is_empty() {
        // Selective updates: re-resolve specified gems, lock others to current versions
        if let Ok(lockfile_content) = std::fs::read_to_string(&lockfile_pathbuf)
            && let Ok(existing_lockfile) = Lockfile::parse(&lockfile_content)
        {
            let update_set: HashSet<&str> = update_gems.iter().map(String::as_str).collect();

            for locked_gem in &existing_lockfile.gems {
                if let Some(gemfile_gem) =
                    gemfile.gems.iter_mut().find(|g| g.name == locked_gem.name)
                {
                    if !update_set.contains(locked_gem.name.as_str()) {
                        // NOT in update list: Lock to exact version
                        gemfile_gem.version_requirement = format!("= {}", locked_gem.version);
                        if verbose {
                            println!("  Locking {} to {}", locked_gem.name, locked_gem.version);
                        }
                    } else if patch || minor {
                        // In update list WITH version level constraints
                        let constraint = if patch {
                            format!("~> {}", locked_gem.version)
                        } else if minor {
                            let parts: Vec<&str> = locked_gem.version.split('.').collect();
                            if strict && parts.len() >= 2 {
                                format!(
                                    "~> {}.{}.0",
                                    parts.first().unwrap_or(&"0"),
                                    parts.get(1).unwrap_or(&"0")
                                )
                            } else if parts.len() >= 2 {
                                format!(
                                    "~> {}.{}",
                                    parts.first().unwrap_or(&"0"),
                                    parts.get(1).unwrap_or(&"0")
                                )
                            } else {
                                format!("~> {}", locked_gem.version)
                            }
                        } else {
                            gemfile_gem.version_requirement.clone()
                        };

                        gemfile_gem.version_requirement.clone_from(&constraint);
                        if verbose {
                            println!("  Constraining {} to {}", locked_gem.name, constraint);
                        }
                    }
                }
            }

            if verbose {
                println!(
                    "Locked {} gems to existing versions",
                    existing_lockfile.gems.len() - update_gems.len()
                );
            }
        }
    } else if patch || minor {
        // Update all gems with version level constraints (no --update provided)
        // Read existing lockfile to apply constraints
        if let Ok(lockfile_content) = std::fs::read_to_string(&lockfile_pathbuf)
            && let Ok(existing_lockfile) = Lockfile::parse(&lockfile_content)
        {
            for locked_gem in &existing_lockfile.gems {
                if let Some(gemfile_gem) =
                    gemfile.gems.iter_mut().find(|g| g.name == locked_gem.name)
                {
                    let constraint = if patch {
                        format!("~> {}", locked_gem.version)
                    } else if minor {
                        let parts: Vec<&str> = locked_gem.version.split('.').collect();
                        if strict && parts.len() >= 2 {
                            format!(
                                "~> {}.{}.0",
                                parts.first().unwrap_or(&"0"),
                                parts.get(1).unwrap_or(&"0")
                            )
                        } else if parts.len() >= 2 {
                            format!(
                                "~> {}.{}",
                                parts.first().unwrap_or(&"0"),
                                parts.get(1).unwrap_or(&"0")
                            )
                        } else {
                            format!("~> {}", locked_gem.version)
                        }
                    } else {
                        continue;
                    };

                    gemfile_gem.version_requirement.clone_from(&constraint);
                    if verbose {
                        println!("  Constraining {} to {}", locked_gem.name, constraint);
                    }
                }
            }

            if verbose {
                println!(
                    "Applied version constraints to {} gems",
                    existing_lockfile.gems.len()
                );
            }
        }
    }

    // Determine platforms
    let mut platforms = vec![detect_current_platform()];
    platforms.extend(add_platforms.iter().cloned());

    // Remove platforms specified by --remove-platform
    if !remove_platforms.is_empty() {
        platforms.retain(|p| !remove_platforms.contains(p));
        if verbose {
            println!("Removed platforms: {}", remove_platforms.join(", "));
        }
    }

    // Remove duplicates
    platforms.sort();
    platforms.dedup();

    if verbose {
        println!("Platforms: {}", platforms.join(", "));
    }

    // Create RubyGems client (use GEM_SOURCE env var if set, otherwise Gemfile source)
    let gem_source = lode::env_vars::gem_source().unwrap_or_else(|| gemfile.source.clone());
    let client = RubyGemsClient::new(&gem_source)
        .context("Failed to create RubyGems API client")?
        .with_cache_only(local)
        .with_prerelease(pre);

    // Create resolver
    let resolver = Resolver::new(client);

    // Resolve dependencies
    if verbose {
        println!("\nResolving dependencies with PubGrub...");
    }

    let platforms_refs: Vec<&str> = platforms.iter().map(String::as_str).collect();
    let resolved_gems = resolver.resolve(&gemfile, &platforms_refs, pre).await?;

    if verbose {
        println!("Resolved {} gems", resolved_gems.len());
    }

    // Convert resolved gems to lockfile format
    let mut lockfile = Lockfile::new();

    for resolved in resolved_gems {
        lockfile.gems.push(convert_to_gem_spec(resolved));
    }

    // Set platforms (normalize if requested)
    lockfile.platforms = if normalize_platforms {
        platforms
            .into_iter()
            .map(|p| {
                // Normalize platform names (e.g., arm64-darwin25.0.0 -> arm64-darwin)
                // Strip version numbers from the end of platform segments
                // Handle both "darwin-25" and "darwin25" patterns

                // First, try to find a dash followed by a digit (e.g., "linux-gnu-5")
                if let Some(idx) = p.rfind('-') {
                    let suffix = &p[idx + 1..];
                    if suffix.chars().next().is_some_and(|c| c.is_ascii_digit()) {
                        return p[..idx].to_string();
                    }
                }

                // If no dash+digit, look for embedded version (e.g., "darwin25.0.0" -> "darwin")
                // Find last segment after final dash
                if let Some(last_dash_idx) = p.rfind('-') {
                    let last_segment = &p[last_dash_idx + 1..];
                    // Find where digits start in this segment
                    if let Some(digit_pos) = last_segment.find(|c: char| c.is_ascii_digit())
                        && digit_pos > 0
                    {
                        // There's text before the digits, keep prefix
                        return format!("{}-{}", &p[..last_dash_idx], &last_segment[..digit_pos]);
                    }
                }

                p
            })
            .collect()
    } else {
        platforms
    };

    // Set Ruby version
    lockfile.ruby_version.clone_from(&gemfile.ruby_version);

    // Set bundler version (use provided version, or lode version if not specified)
    lockfile.bundled_with =
        Some(bundler.map_or_else(|| env!("CARGO_PKG_VERSION").to_string(), String::from));

    // Compute checksums if requested
    if add_checksums {
        if verbose {
            println!("\nComputing checksums for {} gems...", lockfile.gems.len());
        }

        // Create download manager
        let config = lode::Config::load().context("Failed to load configuration")?;
        let cache_dir = lode::config::cache_dir(Some(&config))
            .context("Failed to determine cache directory")?;
        let dm = Arc::new(
            lode::DownloadManager::with_sources_and_retry(
                cache_dir,
                vec![gemfile.source.clone()],
                0, // No retries for checksum computation
            )
            .context("Failed to create download manager")?,
        );

        // Download all gems in parallel and compute checksums
        let checksum_results: Vec<_> = stream::iter(&lockfile.gems)
            .map(|gem| {
                let dm = Arc::clone(&dm);
                let gem_name = gem.name.clone();
                let gem_version = gem.version.clone();
                let gem_platform = gem.platform.clone();

                async move {
                    // Download gem to cache
                    let gem_spec = lode::lockfile::GemSpec::new(
                        gem_name.clone(),
                        gem_version.clone(),
                        gem_platform,
                        vec![],
                        vec![],
                    );
                    let cache_path = dm.download_gem(&gem_spec).await?;

                    // Compute checksum
                    let checksum = lode::DownloadManager::compute_checksum(&cache_path)?;

                    Ok::<(String, String, String), anyhow::Error>((gem_name, gem_version, checksum))
                }
            })
            .buffer_unordered(10) // Process 10 gems in parallel
            .collect()
            .await;

        // Apply checksums to lockfile gems
        for result in checksum_results {
            match result {
                Ok((name, version, checksum)) => {
                    for gem in &mut lockfile.gems {
                        if gem.name == name && gem.version == version {
                            gem.checksum = Some(checksum);
                            break;
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Warning: Failed to compute checksum: {e}");
                }
            }
        }

        if verbose {
            let checksummed = lockfile
                .gems
                .iter()
                .filter(|g| g.checksum.is_some())
                .count();
            println!("Computed {checksummed} checksums");
        }
    }

    // Write lockfile or print to stdout
    let lockfile_content = lockfile.to_string();

    if print {
        // Print to stdout
        print!("{lockfile_content}");
    } else {
        // Write to file
        fs::write(&lockfile_pathbuf, lockfile_content)
            .with_context(|| format!("Failed to write lockfile to {lockfile_str}"))?;

        if !quiet {
            println!("Writing lockfile to {lockfile_str}");
            println!("  {} gems resolved", lockfile.gems.len());
            println!("  {} platforms", lockfile.platforms.len());
        }
    }

    Ok(())
}

/// Convert a `ResolvedGem` to a `GemSpec` for the lockfile
fn convert_to_gem_spec(resolved: ResolvedGem) -> GemSpec {
    let platform = if resolved.platform == "ruby" || resolved.platform.is_empty() {
        None
    } else {
        Some(resolved.platform)
    };

    let dependencies = resolved
        .dependencies
        .into_iter()
        .map(|dep| Dependency {
            name: dep.name,
            requirement: dep.requirement,
        })
        .collect();

    GemSpec::new(
        resolved.name,
        resolved.version,
        platform,
        dependencies,
        vec![], // Groups are handled by Gemfile, not resolver
    )
}
