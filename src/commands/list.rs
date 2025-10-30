//! List command
//!
//! List all gems in the current bundle

use anyhow::{Context, Result};
use lode::{Config, Gemfile, config, lockfile::Lockfile, ruby};
use std::collections::HashSet;
use std::fs;

/// List all gems in the current bundle
pub(crate) fn run(
    lockfile_path: &str,
    name_only: bool,
    show_paths: bool,
    only_group: Option<&str>,
    without_group: Option<&str>,
) -> Result<()> {
    // Read and parse lockfile
    let content = fs::read_to_string(lockfile_path)
        .with_context(|| format!("Failed to read lockfile: {lockfile_path}"))?;

    let lockfile = Lockfile::parse(&content)
        .with_context(|| format!("Failed to parse lockfile: {lockfile_path}"))?;

    // Track whether we're in include mode (only_group) or exclude mode (without_group)
    let is_exclude_mode = without_group.is_some();

    // If filtering by group, load Gemfile (supports both Gemfile and gems.rb)
    let group_filter: Option<HashSet<String>> = if let Some(group_name) = only_group {
        let gemfile_path = lode::paths::find_gemfile();
        let gemfile = Gemfile::parse_file(&gemfile_path).with_context(|| {
            format!(
                "Failed to parse {} for group filtering",
                gemfile_path.display()
            )
        })?;

        let filtered_gems: HashSet<String> = gemfile
            .gems
            .iter()
            .filter(|gem| gem.groups.contains(&group_name.to_string()))
            .map(|gem| gem.name.clone())
            .collect();

        Some(filtered_gems)
    } else if let Some(groups_to_exclude) = without_group {
        // Parse comma-separated groups
        let excluded_groups: Vec<String> = groups_to_exclude
            .split(',')
            .map(|s| s.trim().to_string())
            .collect();

        let gemfile_path = lode::paths::find_gemfile();
        let gemfile = Gemfile::parse_file(&gemfile_path).with_context(|| {
            format!(
                "Failed to parse {} for group filtering",
                gemfile_path.display()
            )
        })?;

        // Find gems in excluded groups
        let gems_to_exclude: HashSet<String> = gemfile
            .gems
            .iter()
            .filter(|gem| gem.groups.iter().any(|g| excluded_groups.contains(g)))
            .map(|gem| gem.name.clone())
            .collect();

        // Invert: we want to keep gems NOT in the excluded set
        // Return the exclusion set so we can filter later
        Some(gems_to_exclude)
    } else {
        None
    };

    // Collect and sort all gems
    let mut all_gems: Vec<(String, String, &str)> = Vec::new();

    // Regular gems from rubygems.org
    for gem in &lockfile.gems {
        if let Some(ref filter) = group_filter {
            let in_filter = filter.contains(&gem.name);
            // Include mode: skip if NOT in filter; Exclude mode: skip if IN filter
            if (is_exclude_mode && in_filter) || (!is_exclude_mode && !in_filter) {
                continue;
            }
        }
        all_gems.push((gem.name.clone(), gem.version.clone(), "gem"));
    }

    // Git gems
    for git_gem in &lockfile.git_gems {
        if let Some(ref filter) = group_filter {
            let in_filter = filter.contains(&git_gem.name);
            if (is_exclude_mode && in_filter) || (!is_exclude_mode && !in_filter) {
                continue;
            }
        }
        all_gems.push((git_gem.name.clone(), git_gem.version.clone(), "git"));
    }

    // Path gems
    for path_gem in &lockfile.path_gems {
        if let Some(ref filter) = group_filter {
            let in_filter = filter.contains(&path_gem.name);
            if (is_exclude_mode && in_filter) || (!is_exclude_mode && !in_filter) {
                continue;
            }
        }
        all_gems.push((path_gem.name.clone(), path_gem.version.clone(), "path"));
    }

    // Sort alphabetically by name
    all_gems.sort_by(|a, b| a.0.cmp(&b.0));

    // Get vendor directory and ruby version for paths
    let (vendor_dir, ruby_version) = if show_paths {
        let cfg = Config::load().unwrap_or_default();
        let vendor = config::vendor_dir(Some(&cfg))?;
        let ruby_ver = lockfile.ruby_version.as_ref().map_or_else(
            || "3.4.0".to_string(),
            |v| ruby::parse_ruby_version_string(v),
        );
        (Some(vendor), Some(ruby_ver))
    } else {
        (None, None)
    };

    // Print gems
    if name_only {
        // Print only gem names, one per line
        for (name, _, _) in &all_gems {
            println!("{name}");
        }
    } else if show_paths {
        // Print with paths
        let vendor = vendor_dir.as_ref().unwrap();
        let ruby_ver = ruby_version.as_ref().unwrap();
        let gems_dir = vendor.join("ruby").join(ruby_ver).join("gems");

        for (name, version, _gem_type) in &all_gems {
            let gem_dir = gems_dir.join(format!("{name}-{version}"));
            println!("{}", gem_dir.display());
        }
    } else {
        // Print with type indicators, versions, and formatting
        println!("Gems included in the bundle:");
        for (name, version, gem_type) in &all_gems {
            let type_label = match *gem_type {
                "git" => "(git) ",
                "path" => "(path) ",
                _ => "",
            };
            println!("  * {type_label}{name} ({version})");
        }

        println!("\nTotal: {} gems", all_gems.len());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn list_simple_lockfile() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let lockfile_content = r"
GEM
  remote: https://rubygems.org/
  specs:
    rack (3.0.8)
    rails (7.0.8)

PLATFORMS
  ruby

BUNDLED WITH
   2.5.3
";
        temp_file.write_all(lockfile_content.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let result = run(temp_file.path().to_str().unwrap(), false, false, None, None);
        assert!(result.is_ok());
    }

    #[test]
    fn list_nonexistent_file() {
        let result = run("/nonexistent/Gemfile.lock", false, false, None, None);
        assert!(result.is_err());
    }

    #[test]
    fn list_name_only() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let lockfile_content = r"
GEM
  remote: https://rubygems.org/
  specs:
    rack (3.0.8)
    rails (7.0.8)

PLATFORMS
  ruby

BUNDLED WITH
   2.5.3
";
        temp_file.write_all(lockfile_content.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let result = run(temp_file.path().to_str().unwrap(), true, false, None, None);
        assert!(result.is_ok());
    }
}
