//! Update command
//!
//! Update gems to newer versions

use anyhow::{Context, Result};
use futures_util::stream::{self, StreamExt};
use indicatif::{ProgressBar, ProgressStyle};
use lode::{lockfile::Lockfile, rubygems_client::RubyGemsClient};
use semver::Version;
use std::collections::HashSet;
use std::fs;
use std::sync::Arc;
use std::time::Duration;

/// Update gems to their latest versions within constraints
///
/// If specific gems are provided, only those will be updated.
/// Otherwise, all gems will be checked for updates.
#[allow(
    clippy::too_many_lines,
    clippy::too_many_arguments,
    clippy::cognitive_complexity,
    clippy::fn_params_excessive_bools
)]
pub(crate) async fn run(
    gems_to_update: &[String],
    all: bool,
    conservative: bool,
    gemfile: Option<&str>,
    jobs: Option<usize>,
    quiet: bool,
    retry: Option<usize>,
    patch: bool,
    minor: bool,
    major: bool,
    strict: bool,
    local: bool,
    pre: bool,
    group: Option<&str>,
    source: Option<&str>,
    ruby: bool,
    bundler: Option<&str>,
    _redownload: bool,
    _full_index: bool,
) -> Result<()> {
    // Note: --redownload and --full-index accepted for Bundler compatibility
    // --redownload: Use `lode fetch --force` to re-download gems
    // --full-index: Update uses dependency API (full index not needed)

    let lockfile_path = gemfile.as_ref().map_or_else(
        || "Gemfile.lock".to_string(),
        |gemfile_path| format!("{gemfile_path}.lock"),
    );

    // Apply BUNDLE_PREFER_PATCH if no explicit update level is provided
    let patch = patch || (!minor && !major && lode::env_vars::bundle_prefer_patch());

    if !quiet {
        if all {
            println!("Updating all gems in Gemfile");
        }
        if conservative {
            println!("Conservative update mode (preferring minimal version changes)");
            println!("Note: Bundler's --conservative prevents updating indirect dependencies.");
            println!("      Lode's implementation prefers minimal version changes instead.");
        }
        if patch {
            println!("Patch update mode: only patch-level updates (x.y.Z)");
        }
        if minor {
            println!("Minor update mode: only minor/patch updates (x.Y.z)");
        }
        if major {
            println!("Major update mode: allow major updates (X.y.z) - default");
        }
        if strict {
            println!("Strict mode: enforce update level limits");
        }
        if local {
            println!("Local mode (using cached gems only)");
        }
        if pre {
            println!("Prerelease mode (allowing prerelease versions)");
        }
        if let Some(grp) = group {
            println!("Updating only group: {grp}");
        }
        if let Some(src) = source {
            println!("Updating only source: {src}");
        }
    }
    // Read and parse lockfile
    let content = fs::read_to_string(&lockfile_path)
        .with_context(|| format!("Failed to read lockfile: {lockfile_path}"))?;

    let lockfile = Lockfile::parse(&content)
        .with_context(|| format!("Failed to parse lockfile: {lockfile_path}"))?;

    if lockfile.gems.is_empty() {
        println!("No gems found in lockfile");
        return Ok(());
    }

    // Parse Gemfile for group and source filtering
    let gemfile_path_buf = gemfile.map_or_else(lode::paths::find_gemfile, std::path::PathBuf::from);
    let parsed_gemfile = lode::Gemfile::parse_file(&gemfile_path_buf).ok();

    // Determine which gems to check
    let mut gems_to_check: HashSet<String> = if gems_to_update.is_empty() {
        // Update all gems
        lockfile.gems.iter().map(|g| g.name.clone()).collect()
    } else {
        // Only update specified gems
        let specified: HashSet<String> = gems_to_update.iter().cloned().collect();

        // Validate that all specified gems exist
        for gem in &specified {
            if !lockfile.gems.iter().any(|g| &g.name == gem) {
                anyhow::bail!("Gem '{gem}' not found in lockfile");
            }
        }

        specified
    };

    // Apply group filtering if specified
    if let (Some(filter_group), Some(parsed_gf)) = (group, &parsed_gemfile) {
        gems_to_check.retain(|gem_name| {
            parsed_gf
                .gems
                .iter()
                .any(|g| &g.name == gem_name && g.groups.contains(&filter_group.to_string()))
        });
        if !quiet {
            println!(
                "Filtered to {} gems in group '{filter_group}'",
                gems_to_check.len()
            );
        }
    }

    // Apply source filtering if specified
    if let (Some(filter_source), Some(parsed_gf)) = (source, &parsed_gemfile) {
        gems_to_check.retain(|gem_name| {
            parsed_gf.gems.iter().any(|g| {
                &g.name == gem_name && g.source.as_ref().is_some_and(|s| s == filter_source)
            })
        });
        if !quiet {
            println!(
                "Filtered to {} gems from source '{filter_source}'",
                gems_to_check.len()
            );
        }
    }

    if !quiet {
        println!("Checking for updates...\n");
    }

    let client = RubyGemsClient::new(lode::gem_source_url())
        .context("Failed to create RubyGems client")?
        .with_cache_only(local)
        .with_prerelease(pre);

    // Count gems to check for progress bar
    let total_to_check = lockfile
        .gems
        .iter()
        .filter(|g| gems_to_check.contains(&g.name))
        .count();

    // Create progress bar
    let pb = ProgressBar::new(total_to_check as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("#>-"),
    );

    // Determine concurrency level (default to 10 concurrent requests)
    let concurrency = jobs.unwrap_or(10);
    let max_retries = retry.unwrap_or(0);

    // Wrap client and progress bar in Arc for sharing across tasks
    let client = Arc::new(client);
    let pb = Arc::new(pb);

    // Create a stream of gems to check
    let gems_to_process: Vec<_> = lockfile
        .gems
        .iter()
        .filter(|g| gems_to_check.contains(&g.name))
        .collect();

    // Process gems in parallel with controlled concurrency
    let results = stream::iter(gems_to_process)
        .map(|gem| {
            let client = Arc::clone(&client);
            let pb = Arc::clone(&pb);
            let gem_name = gem.name.clone();
            let gem_version = gem.version.clone();

            async move {
                pb.set_message(format!("Checking {gem_name}"));

                // Query RubyGems.org for latest version with retry logic
                let versions = {
                    let mut last_error = None;
                    let mut result = None;

                    for attempt in 0..=max_retries {
                        match client.fetch_versions(&gem_name).await {
                            Ok(versions) => {
                                result = Some(versions);
                                break;
                            }
                            Err(err) => {
                                last_error = Some(err);
                                if attempt < max_retries {
                                    // Exponential backoff before retry
                                    let delay =
                                        Duration::from_millis(100 * 2_u64.pow(attempt as u32));
                                    tokio::time::sleep(delay).await;
                                }
                            }
                        }
                    }

                    if let Some(versions) = result {
                        versions
                    } else {
                        let err = last_error.unwrap();
                        pb.println(format!(
                            "Failed to check {gem_name} after {} attempts: {err}",
                            max_retries + 1
                        ));
                        pb.inc(1);
                        return (None, true); // (update_info, is_error)
                    }
                };

                if versions.is_empty() {
                    pb.inc(1);
                    return (None, true);
                }

                // Get the appropriate version based on update mode
                let latest = if patch {
                    find_patch_update(&gem_version, &versions, pre, strict)
                } else if minor {
                    find_minor_update(&gem_version, &versions, pre, strict)
                } else if conservative {
                    find_conservative_update(&gem_version, &versions, pre)
                } else if pre {
                    versions.first()
                } else {
                    versions
                        .iter()
                        .find(|v| !is_prerelease(&v.number))
                        .or_else(|| versions.first())
                };

                let update_info = latest.and_then(|latest_version| {
                    if is_newer(&latest_version.number, &gem_version) {
                        Some((gem_name, gem_version, latest_version.number.clone()))
                    } else {
                        None
                    }
                });

                pb.inc(1);
                (update_info, false)
            }
        })
        .buffer_unordered(concurrency)
        .collect::<Vec<_>>()
        .await;

    pb.finish_with_message("Done!");

    // Process results
    let mut updatable_gems = Vec::new();
    let mut up_to_date = 0;
    let mut errors = 0;

    for (update_info, is_error) in results {
        if is_error {
            errors += 1;
        } else if let Some(info) = update_info {
            updatable_gems.push(info);
        } else {
            up_to_date += 1;
        }
    }

    // Handle --ruby and --bundler flags first (before early return)
    // These update lockfile metadata and don't require gems to be updated
    if ruby || bundler.is_some() {
        let lockfile_content = fs::read_to_string(&lockfile_path)
            .with_context(|| format!("Failed to read lockfile: {lockfile_path}"))?;

        let mut lockfile = Lockfile::parse(&lockfile_content)
            .with_context(|| format!("Failed to parse lockfile: {lockfile_path}"))?;

        if ruby {
            // Update Ruby version to current system Ruby
            let ruby_version = lode::ruby::detect_ruby_version(
                Option::<&str>::None,
                Option::<&str>::None,
                "3.3.0",
            );
            lockfile.ruby_version = Some(ruby_version.clone());
            if !quiet {
                println!("\nUpdated Ruby version to: {ruby_version}");
            }
        }

        if let Some(bundler_version) = bundler {
            // Update Bundler version to specified version or current lode version if empty
            let version_to_use = if bundler_version.is_empty() {
                env!("CARGO_PKG_VERSION")
            } else {
                bundler_version
            };
            lockfile.bundled_with = Some(version_to_use.to_string());
            if !quiet {
                println!("\nUpdated Bundler version to: {version_to_use}");
            }
        }

        // Write updated lockfile
        let lockfile_content = lockfile.to_string();
        fs::write(&lockfile_path, lockfile_content)
            .with_context(|| format!("Failed to write lockfile: {lockfile_path}"))?;

        // If only updating metadata (no gem updates), we're done
        if updatable_gems.is_empty() {
            if !quiet {
                println!("\nLockfile metadata updated");
            }
            return Ok(());
        }
    }

    // Display results
    if updatable_gems.is_empty() {
        println!("All gems are up to date!");
        if gems_to_update.is_empty() {
            println!("   {} gems checked, {} errors", lockfile.gems.len(), errors);
        } else {
            println!("   {} gem(s) checked", gems_to_check.len());
        }
        return Ok(());
    }

    println!("Gems with updates available ({}):\n", updatable_gems.len());

    // Find the longest gem name for alignment
    let max_name_len = updatable_gems
        .iter()
        .map(|(name, _, _)| name.len())
        .max()
        .unwrap_or(0);

    for (name, current, latest) in &updatable_gems {
        println!("  • {name:<max_name_len$}  {current} -> {latest}");
    }

    println!(
        "\n{} gems up to date, {} can be updated, {} errors",
        up_to_date,
        updatable_gems.len(),
        errors
    );

    // Now regenerate the lockfile to actually update
    if !quiet {
        println!("\nRegenerating lockfile with updated versions...");
    }

    // Call the lock command to regenerate the lockfile
    // This will fetch the latest versions respecting Gemfile constraints
    let gemfile_path = gemfile.map_or_else(lode::paths::find_gemfile, std::path::PathBuf::from);
    let gemfile_str = gemfile_path.to_str().unwrap_or("Gemfile");

    crate::commands::lock::run(
        gemfile_str,
        None,   // lockfile_path
        &[],    // add_platforms
        &[],    // remove_platforms
        &[],    // update_gems
        false,  // print
        !quiet, // verbose
        patch,
        minor,
        major,
        strict,
        conservative,
        local,
        pre,
        None,  // bundler
        false, // normalize_platforms
        false, // add_checksums
        false, // full_index
        quiet, // quiet
    )
    .await?;

    println!("\nUpdate complete!");
    println!("   Run `lode install` to install the updated gems");

    Ok(())
}

/// Find a conservative update (prefers minimal version changes)
///
/// NOTE: This does NOT match Bundler's --conservative behavior exactly.
/// Bundler's --conservative prevents updating INDIRECT dependencies (dependencies of dependencies).
/// This implementation prefers patch -> minor -> major updates (minimal version changes).
///
/// To match Bundler exactly would require:
/// 1. Tracking which gems are direct vs indirect dependencies
/// 2. Locking indirect dependencies to exact versions
/// 3. Only updating direct dependencies and their immediate requirements
///
/// Respects the `allow_prerelease` flag for including prerelease versions.
fn find_conservative_update<'a>(
    current_version: &str,
    available_versions: &'a [lode::rubygems_client::GemVersion],
    allow_prerelease: bool,
) -> Option<&'a lode::rubygems_client::GemVersion> {
    // Parse current version
    let Ok(current) = parse_lenient_version(current_version) else {
        // If we can't parse, fall back to latest (stable or prerelease based on flag)
        return if allow_prerelease {
            available_versions.first()
        } else {
            available_versions
                .iter()
                .find(|v| !is_prerelease(&v.number))
        };
    };

    // Filter versions based on prerelease flag
    let filtered_versions: Vec<_> = if allow_prerelease {
        available_versions.iter().collect()
    } else {
        available_versions
            .iter()
            .filter(|v| !is_prerelease(&v.number))
            .collect()
    };

    // Try to find a patch update (same major.minor, higher patch)
    for version in &filtered_versions {
        if let Ok(v) = parse_lenient_version(&version.number)
            && v.major == current.major
            && v.minor == current.minor
            && v.patch > current.patch
        {
            return Some(version);
        }
    }

    // Try to find a minor update (same major, higher minor)
    for version in &filtered_versions {
        if let Ok(v) = parse_lenient_version(&version.number)
            && v.major == current.major
            && v.minor > current.minor
        {
            return Some(version);
        }
    }

    // Try to find a major update (higher major)
    for version in &filtered_versions {
        if let Ok(v) = parse_lenient_version(&version.number)
            && v.major > current.major
        {
            return Some(version);
        }
    }

    // No updates found
    None
}

/// Find a patch-level update
///
/// With --strict: Only returns patch updates (same major.minor, higher patch).
/// Without --strict: Prefers patch, but falls back to minor/major if no patch available.
/// This matches Bundler's behavior where --patch changes preference order, not hard limits.
fn find_patch_update<'a>(
    current_version: &str,
    available_versions: &'a [lode::rubygems_client::GemVersion],
    allow_prerelease: bool,
    strict: bool,
) -> Option<&'a lode::rubygems_client::GemVersion> {
    // Parse current version
    let Ok(current) = parse_lenient_version(current_version) else {
        // Can't parse current version, fall back to latest
        return if allow_prerelease {
            available_versions.first()
        } else {
            available_versions
                .iter()
                .find(|v| !is_prerelease(&v.number))
        };
    };

    // Filter versions based on prerelease flag
    let filtered_versions: Vec<_> = if allow_prerelease {
        available_versions.iter().collect()
    } else {
        available_versions
            .iter()
            .filter(|v| !is_prerelease(&v.number))
            .collect()
    };

    // First, try to find a patch update (same major.minor, higher patch)
    for version in &filtered_versions {
        if let Ok(v) = parse_lenient_version(&version.number)
            && v.major == current.major
            && v.minor == current.minor
            && v.patch > current.patch
        {
            return Some(version);
        }
    }

    // If --strict, don't allow updates beyond patch level
    if strict {
        return None;
    }

    // Without --strict, fall back to minor updates (same major, higher minor)
    for version in &filtered_versions {
        if let Ok(v) = parse_lenient_version(&version.number)
            && v.major == current.major
            && v.minor > current.minor
        {
            return Some(version);
        }
    }

    // Still no update? Fall back to major updates
    for version in &filtered_versions {
        if let Ok(v) = parse_lenient_version(&version.number)
            && v.major > current.major
        {
            return Some(version);
        }
    }

    // No updates available at all
    None
}

/// Find a minor-level or patch-level update
///
/// With --strict: Only returns minor or patch updates (same major version).
/// Without --strict: Prefers minor/patch, but falls back to major if needed.
/// This matches Bundler's behavior where --minor changes preference order, not hard limits.
fn find_minor_update<'a>(
    current_version: &str,
    available_versions: &'a [lode::rubygems_client::GemVersion],
    allow_prerelease: bool,
    strict: bool,
) -> Option<&'a lode::rubygems_client::GemVersion> {
    // Parse current version
    let Ok(current) = parse_lenient_version(current_version) else {
        // Can't parse current version, fall back to latest
        return if allow_prerelease {
            available_versions.first()
        } else {
            available_versions
                .iter()
                .find(|v| !is_prerelease(&v.number))
        };
    };

    // Filter versions based on prerelease flag
    let filtered_versions: Vec<_> = if allow_prerelease {
        available_versions.iter().collect()
    } else {
        available_versions
            .iter()
            .filter(|v| !is_prerelease(&v.number))
            .collect()
    };

    // Try to find a patch update first (same major.minor, higher patch)
    for version in &filtered_versions {
        if let Ok(v) = parse_lenient_version(&version.number)
            && v.major == current.major
            && v.minor == current.minor
            && v.patch > current.patch
        {
            return Some(version);
        }
    }

    // Try to find a minor update (same major, higher minor)
    for version in &filtered_versions {
        if let Ok(v) = parse_lenient_version(&version.number)
            && v.major == current.major
            && v.minor > current.minor
        {
            return Some(version);
        }
    }

    // If --strict, don't allow updates beyond minor level (same major)
    if strict {
        return None;
    }

    // Without --strict, fall back to major updates
    for version in &filtered_versions {
        if let Ok(v) = parse_lenient_version(&version.number)
            && v.major > current.major
        {
            return Some(version);
        }
    }

    // No updates available at all
    None
}

/// Parse version with lenient handling of Ruby gem version formats
fn parse_lenient_version(version: &str) -> Result<Version, String> {
    // Try parsing as-is first
    if let Ok(v) = Version::parse(version) {
        return Ok(v);
    }

    // Normalize Ruby 4-part versions (e.g., "1.2.3.4" -> "1.2.3")
    let normalized = version
        .split('-')
        .next()
        .unwrap_or(version)
        .split('+')
        .next()
        .unwrap_or(version);

    let parts: Vec<&str> = normalized.split('.').collect();
    if parts.len() >= 3 {
        // Take only major.minor.patch
        let major = parts
            .first()
            .ok_or_else(|| "Missing major version".to_string())?;
        let minor = parts
            .get(1)
            .ok_or_else(|| "Missing minor version".to_string())?;
        let patch = parts
            .get(2)
            .ok_or_else(|| "Missing patch version".to_string())?;
        let semver_str = format!("{major}.{minor}.{patch}");
        Version::parse(&semver_str).map_err(|e| e.to_string())
    } else {
        Err(format!("Invalid version format: {version}"))
    }
}

/// Check if a version string indicates a prerelease version
fn is_prerelease(version: &str) -> bool {
    let version_lower = version.to_lowercase();
    version_lower.contains("alpha")
        || version_lower.contains("beta")
        || version_lower.contains("rc")
        || version_lower.contains("pre")
        || version_lower.contains("dev")
}

/// Compare two version strings to determine if first is newer than second
fn is_newer(version1: &str, version2: &str) -> bool {
    let parts1: Vec<u32> = parse_version_parts(version1);
    let parts2: Vec<u32> = parse_version_parts(version2);

    for (v1, v2) in parts1.iter().zip(parts2.iter()) {
        if v1 > v2 {
            return true;
        }
        if v1 < v2 {
            return false;
        }
    }

    parts1.len() > parts2.len()
}

/// Parse version string into numeric parts
fn parse_version_parts(version: &str) -> Vec<u32> {
    version
        .split(&['.', '-', '+'][..])
        .filter_map(|part| part.parse::<u32>().ok())
        .collect()
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "Tests can panic")]
mod tests {
    use super::*;

    #[test]
    fn test_is_prerelease() {
        assert!(is_prerelease("1.0.0.alpha"));
        assert!(is_prerelease("2.0.0.beta1"));
        assert!(is_prerelease("3.0.0-rc1"));
        assert!(!is_prerelease("1.0.0"));
        assert!(!is_prerelease("2.5.3"));
    }

    #[test]
    fn test_is_newer() {
        assert!(is_newer("2.0.0", "1.0.0"));
        assert!(is_newer("1.1.0", "1.0.0"));
        assert!(is_newer("1.0.1", "1.0.0"));
        assert!(!is_newer("1.0.0", "2.0.0"));
        assert!(!is_newer("1.0.0", "1.0.0"));
    }

    #[test]
    fn test_parse_version_parts() {
        assert_eq!(parse_version_parts("1.2.3"), vec![1, 2, 3]);
        assert_eq!(parse_version_parts("10.0.5"), vec![10, 0, 5]);
        assert_eq!(parse_version_parts("2.0.0.pre"), vec![2, 0, 0]);
    }

    #[test]
    fn test_parse_lenient_version_standard() {
        let result = parse_lenient_version("1.2.3");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().to_string(), "1.2.3");
    }

    #[test]
    fn test_parse_lenient_version_four_part() {
        let result = parse_lenient_version("1.2.3.4");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().to_string(), "1.2.3");
    }

    #[test]
    fn test_parse_lenient_version_with_prerelease() {
        // Prerelease part is removed but version is still valid
        let result = parse_lenient_version("1.2.3-alpha");
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_lenient_version_with_build() {
        // Build metadata is removed but version is still valid
        let result = parse_lenient_version("1.2.3+build123");
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_lenient_version_invalid() {
        let result = parse_lenient_version("invalid");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_lenient_version_two_part() {
        let result = parse_lenient_version("1.2");
        assert!(result.is_err());
    }
}
