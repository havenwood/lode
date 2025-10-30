//! Outdated command
//!
//! Compare installed gems with latest versions

use anyhow::{Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use lode::{Gemfile, lockfile::Lockfile, rubygems_client::RubyGemsClient};
use semver::Version;
use std::collections::HashSet;
use std::fs;

/// Compare installed gem versions with latest available versions on RubyGems.org
#[allow(
    clippy::fn_params_excessive_bools,
    reason = "Parameters come from CLI structure"
)]
#[allow(
    clippy::cognitive_complexity,
    reason = "Main command function with sequential logic"
)]
pub(crate) async fn run(
    lockfile_path: &str,
    parseable: bool,
    filter_major: bool,
    filter_minor: bool,
    filter_patch: bool,
    include_prerelease: bool,
    group_filter: Option<&str>,
) -> Result<()> {
    // Read and parse lockfile
    let content = fs::read_to_string(lockfile_path)
        .with_context(|| format!("Failed to read lockfile: {lockfile_path}"))?;

    let lockfile = Lockfile::parse(&content)
        .with_context(|| format!("Failed to parse lockfile: {lockfile_path}"))?;

    if lockfile.gems.is_empty() {
        if !parseable {
            println!("No gems found in lockfile");
        }
        return Ok(());
    }

    // Filter by group if requested
    let gems_in_group: Option<HashSet<String>> = if let Some(group_name) = group_filter {
        let gemfile_path = lode::paths::find_gemfile();
        let gemfile = Gemfile::parse_file(&gemfile_path).with_context(|| {
            format!(
                "Failed to parse {} for group filtering",
                gemfile_path.display()
            )
        })?;

        let filtered: HashSet<String> = gemfile
            .gems
            .iter()
            .filter(|gem| gem.groups.contains(&group_name.to_string()))
            .map(|gem| gem.name.clone())
            .collect();

        if filtered.is_empty() {
            if !parseable {
                println!("No gems found in group '{group_name}'");
            }
            return Ok(());
        }

        Some(filtered)
    } else {
        None
    };

    if !parseable {
        println!("Checking for outdated gems...\n");
    }

    let client = RubyGemsClient::new(lode::DEFAULT_GEM_SOURCE)
        .context("Failed to create RubyGems client")?;

    // Create progress bar (only if not parseable)
    let pb = if parseable {
        None
    } else {
        let progress = ProgressBar::new(lockfile.gems.len() as u64);
        progress.set_style(
            ProgressStyle::default_bar()
                .template(
                    "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {msg}",
                )
                .unwrap()
                .progress_chars("#>-"),
        );
        Some(progress)
    };

    let mut outdated_gems = Vec::new();
    let mut up_to_date_count = 0;
    let mut error_count = 0;

    for gem in &lockfile.gems {
        // Skip gems not in requested group
        if let Some(ref filter) = gems_in_group
            && !filter.contains(&gem.name)
        {
            continue;
        }

        if let Some(ref pb) = pb {
            pb.set_message(format!("Checking {}", gem.name));
        }

        // Query RubyGems.org for latest version
        let versions = match client.fetch_versions(&gem.name).await {
            Ok(versions) => versions,
            Err(err) => {
                if let Some(ref pb) = pb {
                    pb.println(format!("Failed to check {}: {}", gem.name, err));
                }
                error_count += 1;
                if let Some(ref pb) = pb {
                    pb.inc(1);
                }
                continue;
            }
        };

        if versions.is_empty() {
            if let Some(ref pb) = pb {
                pb.println(format!("No versions found for {}", gem.name));
            }
            error_count += 1;
            if let Some(ref pb) = pb {
                pb.inc(1);
            }
            continue;
        }

        // Get the latest version (stable or prerelease based on --pre flag)
        let latest = if include_prerelease {
            // Include prereleases, so just get first (latest) version
            versions
                .first()
                .expect("versions should not be empty after check")
        } else {
            // Filter out prerelease versions, fallback to first if all are prerelease
            versions
                .iter()
                .find(|v| !is_prerelease(&v.number))
                .or_else(|| versions.first())
                .expect("versions should not be empty after check")
        };

        // Compare versions
        if is_newer(&latest.number, &gem.version) {
            outdated_gems.push((gem.name.clone(), gem.version.clone(), latest.number.clone()));
        } else {
            up_to_date_count += 1;
        }

        if let Some(ref pb) = pb {
            pb.inc(1);
        }
    }

    if let Some(pb) = pb {
        pb.finish_with_message("Done!");
    }

    // Filter outdated gems by version change type if requested
    let outdated_gems = if filter_major || filter_minor || filter_patch {
        outdated_gems
            .into_iter()
            .filter(|(_, current, latest)| {
                match (
                    parse_lenient_version(current),
                    parse_lenient_version(latest),
                ) {
                    (Ok(curr_ver), Ok(latest_ver)) => {
                        if filter_major {
                            latest_ver.major > curr_ver.major
                        } else if filter_minor {
                            latest_ver.major == curr_ver.major && latest_ver.minor > curr_ver.minor
                        } else if filter_patch {
                            latest_ver.major == curr_ver.major
                                && latest_ver.minor == curr_ver.minor
                                && latest_ver.patch > curr_ver.patch
                        } else {
                            true
                        }
                    }
                    _ => true, // Include gems with non-parseable versions
                }
            })
            .collect()
    } else {
        outdated_gems
    };

    // Display results
    if parseable {
        // Machine-readable format: gem_name current_version latest_version
        for (name, current, latest) in &outdated_gems {
            println!("{name} {current} {latest}");
        }
    } else if outdated_gems.is_empty() {
        println!("All gems are up to date!");
        println!(
            "   {} gems checked, {} errors",
            lockfile.gems.len(),
            error_count
        );
    } else {
        println!("Outdated gems ({}):\n", outdated_gems.len());

        // Find the longest gem name for alignment
        let max_name_len = outdated_gems
            .iter()
            .map(|(name, _, _): &(String, String, String)| name.len())
            .max()
            .unwrap_or(0);

        for (name, current, latest) in &outdated_gems {
            println!("  • {name:<max_name_len$}  {current} -> {latest}");
        }

        println!(
            "\n{} gems up to date, {} outdated, {} errors",
            up_to_date_count,
            outdated_gems.len(),
            error_count
        );
        println!("\nRun `lode update` to update gems to their latest versions.");
    }

    Ok(())
}

/// Check if a version string indicates a prerelease version
///
/// Prerelease versions typically contain: alpha, beta, rc, pre, dev
fn is_prerelease(version: &str) -> bool {
    let version_lower = version.to_lowercase();
    version_lower.contains("alpha")
        || version_lower.contains("beta")
        || version_lower.contains("rc")
        || version_lower.contains("pre")
        || version_lower.contains("dev")
}

/// Compare two version strings to determine if first is newer than second
///
/// Uses the `semver` crate for robust semantic version comparison.
/// Handles non-strict semver formats by normalizing to semver format.
fn is_newer(version1: &str, version2: &str) -> bool {
    // Normalize versions to semver format
    let Ok(v1) = parse_lenient_version(version1) else {
        // Fallback to string comparison if parsing fails
        return version1 > version2;
    };
    let Ok(v2) = parse_lenient_version(version2) else {
        return version1 > version2;
    };

    v1 > v2
}

/// Parse version string leniently, handling non-semver Ruby gem formats
///
/// Ruby gems can have versions like "1.2.3.4" or "3.2.1-beta" which aren't strict semver.
/// This normalizes them by extracting only numeric parts for consistent comparison.
fn parse_lenient_version(version: &str) -> std::result::Result<Version, String> {
    // Ruby gems can have 4-part versions like "1.2.3.4" or prerelease like "3.2.1-beta"
    // Normalize by taking only the first 3 numeric parts
    let parts: Vec<&str> = version.split(&['.', '-', '+'][..]).collect();
    let numeric_parts: Vec<&str> = parts
        .iter()
        .take(3)
        .copied()
        .filter(|p| p.parse::<u32>().is_ok())
        .collect();

    // Build semver-compatible version string (numeric only)
    let normalized = match numeric_parts.as_slice() {
        [] => return Err(format!("No valid version parts in: {version}")),
        [major] => format!("{major}.0.0"),
        [major, minor] => format!("{major}.{minor}.0"),
        [major, minor, patch, ..] => format!("{major}.{minor}.{patch}"),
    };

    Version::parse(&normalized).map_err(|e| e.to_string())
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
        assert!(is_prerelease("1.2.3.pre"));
        assert!(is_prerelease("0.1.0.dev"));

        assert!(!is_prerelease("1.0.0"));
        assert!(!is_prerelease("2.5.3"));
        assert!(!is_prerelease("10.0.0"));
    }

    #[test]
    fn test_parse_lenient_version() {
        // Standard semver
        assert_eq!(
            parse_lenient_version("1.2.3").unwrap(),
            Version::new(1, 2, 3)
        );

        // Ruby 4-part versions (normalize to 3-part)
        assert_eq!(
            parse_lenient_version("1.2.3.4").unwrap(),
            Version::new(1, 2, 3)
        );

        // Short versions (pad with zeros)
        assert_eq!(parse_lenient_version("1.2").unwrap(), Version::new(1, 2, 0));
        assert_eq!(parse_lenient_version("2").unwrap(), Version::new(2, 0, 0));

        // Prerelease versions (parse first 3 numeric parts)
        assert_eq!(
            parse_lenient_version("2.0.0.pre").unwrap(),
            Version::new(2, 0, 0)
        );
        assert_eq!(
            parse_lenient_version("3.2.1-beta").unwrap(),
            Version::new(3, 2, 1)
        );
    }

    #[test]
    fn test_is_newer() {
        // Clear cases
        assert!(is_newer("2.0.0", "1.0.0"));
        assert!(is_newer("1.1.0", "1.0.0"));
        assert!(is_newer("1.0.1", "1.0.0"));

        assert!(!is_newer("1.0.0", "2.0.0"));
        assert!(!is_newer("1.0.0", "1.1.0"));
        assert!(!is_newer("1.0.0", "1.0.1"));

        // Equal versions
        assert!(!is_newer("1.0.0", "1.0.0"));

        // 4-part versions normalize to 3 parts (both become 1.0.0)
        assert!(!is_newer("1.0.0.1", "1.0.0"));
        assert!(!is_newer("1.0.0", "1.0.0.1"));
    }

    #[test]
    fn version_comparison_edge_cases() {
        assert!(is_newer("10.0.0", "9.0.0"));
        assert!(is_newer("1.10.0", "1.9.0"));
        assert!(is_newer("1.0.10", "1.0.9"));

        assert!(!is_newer("9.0.0", "10.0.0"));
        assert!(!is_newer("1.9.0", "1.10.0"));
        assert!(!is_newer("1.0.9", "1.0.10"));
    }
}
