//! Dependency command
//!
//! Show gem dependencies

use anyhow::{Context, Result};
use lode::{Config, RubyGemsClient, gem_store::GemStore, parse_gem_name};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;

/// Options for gem dependency command
#[derive(Debug)]
pub(crate) struct DependencyOptions {
    /// Gem name pattern (regexp)
    pub gem_pattern: String,

    /// Specific version
    pub version: Option<String>,

    /// Platform
    pub platform: Option<String>,

    /// Include prerelease versions
    pub prerelease: bool,

    /// Show reverse dependencies
    pub reverse_dependencies: bool,

    /// Pipe format output
    pub pipe: bool,

    /// Local only
    pub local: bool,

    /// Remote only
    pub remote: bool,

    /// Both local and remote
    pub both: bool,

    /// Bulk threshold for switching to bulk synchronization (default 1000)
    /// When set or when the pattern might match multiple gems, uses bulk index for efficiency.
    pub bulk_threshold: Option<usize>,

    /// Clear sources (use only explicit --source)
    pub clear_sources: bool,

    /// Source URL
    pub source: Option<String>,

    /// HTTP proxy URL (overrides `HTTP_PROXY` env var)
    pub http_proxy: Option<String>,

    /// Verbose output
    pub verbose: bool,

    /// Quiet mode
    pub quiet: bool,

    /// Silent mode
    pub silent: bool,
}

impl Default for DependencyOptions {
    fn default() -> Self {
        Self {
            gem_pattern: String::new(),
            version: None,
            platform: None,
            prerelease: false,
            reverse_dependencies: false,
            pipe: false,
            local: true, // gem dependency defaults to --local
            remote: false,
            both: false,
            bulk_threshold: None,
            clear_sources: false,
            source: None,
            http_proxy: None,
            verbose: false,
            quiet: false,
            silent: false,
        }
    }
}

/// Dependency information
#[derive(Debug, Clone)]
struct Dependency {
    name: String,
    requirements: String,
}

/// Gem with dependencies
#[derive(Debug, Clone)]
struct GemWithDeps {
    name: String,
    version: String,
    dependencies: Vec<Dependency>,
}

/// Show gem dependencies
pub(crate) async fn run(options: DependencyOptions) -> Result<()> {
    if options.gem_pattern.is_empty() {
        anyhow::bail!("Gem name or pattern is required");
    }

    let query_local = options.local || options.both || !options.remote;
    let query_remote = options.remote || options.both;

    let mut found_any = false;

    // Query local gems
    if query_local && let Ok(found) = show_local_dependencies(&options) {
        found_any = found_any || found;
    }

    // Query remote gems
    if query_remote && let Ok(found) = show_remote_dependencies(&options).await {
        found_any = found_any || found;
    }

    if !found_any && !options.quiet && !options.silent {
        println!("No gems matching '{}' found", options.gem_pattern);
    }

    Ok(())
}

/// Show dependencies for local gems
fn show_local_dependencies(options: &DependencyOptions) -> Result<bool> {
    let _config = Config::load().context("Failed to load configuration")?;
    let store = GemStore::new().context("Failed to initialize gem store")?;
    let gem_dir = store.gem_dir().to_path_buf();

    if !gem_dir.exists() {
        return Ok(false);
    }

    let entries = fs::read_dir(&gem_dir)
        .with_context(|| format!("Failed to read gem directory: {}", gem_dir.display()))?;

    let mut matching_gems = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        if let Some(dir_name) = path.file_name().and_then(|n| n.to_str())
            && let Some((name, version)) = parse_gem_name(dir_name)
        {
            // Match pattern
            if !name.starts_with(&options.gem_pattern) {
                continue;
            }

            // Filter by version if specified
            if let Some(ref req_version) = options.version
                && version != req_version
            {
                continue;
            }

            // Read dependencies from gemspec
            let deps = read_dependencies_from_gemspec(&path);

            matching_gems.push(GemWithDeps {
                name: name.to_string(),
                version: version.to_string(),
                dependencies: deps,
            });
        }
    }

    if matching_gems.is_empty() {
        return Ok(false);
    }

    // Show reverse dependencies if requested
    if options.reverse_dependencies {
        show_reverse_dependencies(&matching_gems, &gem_dir, options)?;
    } else {
        // Show forward dependencies
        for gem in &matching_gems {
            display_gem_dependencies(gem, options);
        }
    }

    Ok(!matching_gems.is_empty())
}

/// Check if the query should use bulk index search (pattern-based)
fn should_use_bulk_search(pattern: &str, bulk_threshold: Option<usize>) -> bool {
    // If bulk_threshold is explicitly set, use bulk search
    if bulk_threshold.is_some() {
        return true;
    }

    // If pattern is very short (likely a prefix search), use bulk
    if pattern.len() <= 2 {
        return true;
    }

    // If pattern contains wildcards or looks like regex, use bulk
    // Note: RubyGems gem dependency doesn't support wildcards in practice,
    // but we support prefix matching like the local search does
    false
}

/// Show dependencies for remote gems
async fn show_remote_dependencies(options: &DependencyOptions) -> Result<bool> {
    // When --clear-sources is used, require an explicit --source
    let source = if options.clear_sources {
        options
            .source
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("--clear-sources requires --source to be specified"))?
    } else {
        options.source.as_deref().unwrap_or("https://rubygems.org")
    };

    let client = RubyGemsClient::new_with_proxy(source, options.http_proxy.as_deref())?;

    // Decide whether to use bulk index search or exact match
    if should_use_bulk_search(&options.gem_pattern, options.bulk_threshold) {
        return show_remote_dependencies_bulk(&client, options).await;
    }

    // Try exact match first (more efficient for single gems)
    match client.fetch_versions(&options.gem_pattern).await {
        Ok(versions) if !versions.is_empty() => Ok(show_gem_dependencies(
            &options.gem_pattern,
            &versions,
            options,
        )),
        Ok(_) => Ok(false), // No versions found
        Err(_) => {
            // If exact match fails, try bulk search as fallback
            // This handles cases where the pattern might be a partial name
            return show_remote_dependencies_bulk(&client, options).await;
        }
    }
}

/// Show dependencies using bulk index search (for pattern queries)
async fn show_remote_dependencies_bulk(
    client: &RubyGemsClient,
    options: &DependencyOptions,
) -> Result<bool> {
    // Search bulk index for matching gems
    let bulk_results = client
        .search_bulk_index(&options.gem_pattern, options.prerelease)
        .await
        .with_context(|| {
            format!(
                "Failed to search bulk index for pattern '{}'",
                options.gem_pattern
            )
        })?;

    if bulk_results.is_empty() {
        return Ok(false);
    }

    // Collect unique gem names from bulk results
    let mut gem_names: Vec<String> = bulk_results.iter().map(|spec| spec.name.clone()).collect();
    gem_names.sort();
    gem_names.dedup();

    // Limit results to avoid overwhelming output
    let max_gems = options.bulk_threshold.unwrap_or(50); // Default to 50 gems max
    if gem_names.len() > max_gems {
        if !options.quiet && !options.silent {
            eprintln!(
                "Pattern '{}' matches {} gems (showing first {})",
                options.gem_pattern,
                gem_names.len(),
                max_gems
            );
        }
        gem_names.truncate(max_gems);
    }

    let mut found_any = false;

    // Fetch and display dependencies for each gem
    for gem_name in gem_names {
        if let Ok(versions) = client.fetch_versions(&gem_name).await
            && !versions.is_empty()
            && show_gem_dependencies(&gem_name, &versions, options)
        {
            found_any = true;
        }
    }

    Ok(found_any)
}

/// Display dependencies for a specific gem's versions
fn show_gem_dependencies(
    gem_name: &str,
    versions: &[lode::GemVersion],
    options: &DependencyOptions,
) -> bool {
    // Filter versions
    let candidates: Vec<_> = versions
        .iter()
        .filter(|v| {
            // Filter prerelease
            if !options.prerelease && is_prerelease(&v.number) {
                return false;
            }

            // Filter by version
            if let Some(ref req_version) = options.version
                && &v.number != req_version
            {
                return false;
            }

            // Filter by platform
            if let Some(ref platform) = options.platform
                && &v.platform != platform
                && v.platform != "ruby"
            {
                return false;
            }

            true
        })
        .collect();

    if candidates.is_empty() {
        return false;
    }

    // Show dependencies for each matching version
    if !options.silent {
        for version in candidates {
            if options.pipe {
                println!("{gem_name} --version {}", version.number);
                for dep in &version.dependencies.runtime {
                    println!("  {} ({})", dep.name, dep.requirements);
                }
            } else {
                println!("Gem {gem_name} ({})", version.number);

                if version.dependencies.runtime.is_empty() {
                    println!("  No dependencies");
                } else {
                    for dep in &version.dependencies.runtime {
                        println!("  {} ({})", dep.name, dep.requirements);
                    }
                }

                if !version.dependencies.development.is_empty() && options.verbose {
                    println!("\n  Development dependencies:");
                    for dep in &version.dependencies.development {
                        println!("    {} ({})", dep.name, dep.requirements);
                    }
                }

                println!();
            }
        }
    }

    true
}

/// Show reverse dependencies (which gems depend on the specified gems)
fn show_reverse_dependencies(
    target_gems: &[GemWithDeps],
    gem_dir: &PathBuf,
    options: &DependencyOptions,
) -> Result<()> {
    if options.silent {
        return Ok(());
    }

    // Build map of gem names from targets
    let target_names: HashSet<String> = target_gems.iter().map(|g| g.name.clone()).collect();

    // Scan all gems to find reverse dependencies
    let entries = fs::read_dir(gem_dir)
        .with_context(|| format!("Failed to read gem directory: {}", gem_dir.display()))?;

    let mut reverse_deps: HashMap<String, Vec<(String, String)>> = HashMap::new();

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        if let Some(dir_name) = path.file_name().and_then(|n| n.to_str())
            && let Some((name, version)) = parse_gem_name(dir_name)
        {
            let deps = read_dependencies_from_gemspec(&path);

            // Check if this gem depends on any of our targets
            for dep in deps {
                if target_names.contains(&dep.name) {
                    reverse_deps
                        .entry(dep.name.clone())
                        .or_default()
                        .push((name.to_string(), version.to_string()));
                }
            }
        }
    }

    // Display results
    for gem in target_gems {
        if options.pipe {
            println!("{} --version {}", gem.name, gem.version);
            if let Some(rdeps) = reverse_deps.get(&gem.name) {
                for (dep_name, dep_version) in rdeps {
                    println!("  {dep_name} ({dep_version})");
                }
            }
        } else {
            println!("Gem {} ({})", gem.name, gem.version);

            if let Some(rdeps) = reverse_deps.get(&gem.name) {
                println!("  Used by:");
                for (dep_name, dep_version) in rdeps {
                    println!("    {dep_name} ({dep_version})");
                }
            } else {
                println!("  No gems depend on this");
            }

            println!();
        }
    }

    Ok(())
}

/// Display gem dependencies
fn display_gem_dependencies(gem: &GemWithDeps, options: &DependencyOptions) {
    if options.silent {
        return;
    }

    if options.pipe {
        println!("{} --version {}", gem.name, gem.version);
        for dep in &gem.dependencies {
            println!("  {} ({})", dep.name, dep.requirements);
        }
    } else {
        println!("Gem {} ({})", gem.name, gem.version);

        if gem.dependencies.is_empty() {
            println!("  No dependencies");
        } else {
            for dep in &gem.dependencies {
                println!("  {} ({})", dep.name, dep.requirements);
            }
        }

        println!();
    }
}

/// Read dependencies from gemspec file
fn read_dependencies_from_gemspec(gem_path: &PathBuf) -> Vec<Dependency> {
    let mut dependencies = Vec::new();

    if let Ok(entries) = fs::read_dir(gem_path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(ext) = path.extension()
                && ext == "gemspec"
            {
                if let Ok(content) = fs::read_to_string(&path) {
                    // Parse dependencies from gemspec
                    // Look for lines like: s.add_dependency "name", "version"
                    for line in content.lines() {
                        if let Some(dep) = parse_dependency_line(line) {
                            dependencies.push(dep);
                        }
                    }
                }
                break;
            }
        }
    }

    dependencies
}

/// Parse a dependency line from gemspec
fn parse_dependency_line(line: &str) -> Option<Dependency> {
    let trimmed = line.trim();

    // Match patterns like:
    // s.add_dependency "name", "version"
    // spec.add_runtime_dependency "name", "version"
    if (trimmed.starts_with("s.add_dependency")
        || trimmed.starts_with("spec.add_dependency")
        || trimmed.starts_with("s.add_runtime_dependency")
        || trimmed.starts_with("spec.add_runtime_dependency"))
        && !trimmed.contains("add_development_dependency")
    {
        // Extract quoted strings
        let parts: Vec<&str> = trimmed.split('"').collect();
        if parts.len() >= 3 {
            let name = parts.get(1)?.to_string();
            let requirements = parts.get(3).unwrap_or(&">= 0").to_string();
            return Some(Dependency { name, requirements });
        }
    }

    None
}

/// Check if a version is a prerelease
fn is_prerelease(version: &str) -> bool {
    version.contains('-')
        || version.contains(".pre")
        || version.contains(".rc")
        || version.contains(".alpha")
        || version.contains(".beta")
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "Tests can panic")]
mod tests {
    use super::*;

    /// Parses standard gemspec dependency declarations
    #[test]
    fn test_parse_dependency_line() {
        let line1 = r#"  s.add_dependency "rack", "~> 2.0""#;
        let dep1 =
            parse_dependency_line(line1).expect("should parse s.add_dependency with version");
        assert_eq!(dep1.name, "rack", "should extract gem name");
        assert_eq!(
            dep1.requirements, "~> 2.0",
            "should extract version constraint"
        );

        let line2 = r#"  spec.add_runtime_dependency "rails""#;
        let dep2 = parse_dependency_line(line2)
            .expect("should parse spec.add_runtime_dependency without version");
        assert_eq!(dep2.name, "rails", "should extract gem name");
    }

    /// Handles dependency parsing edge cases and invalid formats
    #[test]
    fn test_parse_dependency_line_edge_cases() {
        let line1 = r#"  spec.add_dependency "minitest", ">= 5.1""#;
        let dep1 = parse_dependency_line(line1).expect("should parse spec.add_dependency");
        assert_eq!(dep1.name, "minitest");
        assert_eq!(dep1.requirements, ">= 5.1");

        let line2 = r#"  s.add_runtime_dependency "concurrent-ruby""#;
        let dep2 = parse_dependency_line(line2).expect("should parse without explicit version");
        assert_eq!(dep2.name, "concurrent-ruby");
        assert_eq!(
            dep2.requirements, ">= 0",
            "should use default version constraint"
        );

        let line3 = r"  s.add_dependency 'rack', '~> 2.0'";
        assert!(
            parse_dependency_line(line3).is_none(),
            "should reject single-quoted dependencies"
        );

        let line4 = r#"  s.add_development_dependency "rspec", "~> 3.0""#;
        assert!(
            parse_dependency_line(line4).is_none(),
            "should exclude development dependencies"
        );

        let line5 = r"  s.add_dependency rack";
        assert!(
            parse_dependency_line(line5).is_none(),
            "should require quoted gem names"
        );
    }

    /// Detects standard prerelease version patterns
    #[test]
    fn test_is_prerelease() {
        assert!(is_prerelease("1.0.0-pre"));
        assert!(is_prerelease("1.0.0.rc1"));
        assert!(is_prerelease("2.0.0-alpha"));
        assert!(is_prerelease("3.0.0-beta"));
        assert!(is_prerelease("1.0.0.pre.1"));

        assert!(!is_prerelease("1.0.0"));
        assert!(!is_prerelease("2.5.10"));
        assert!(!is_prerelease("0.1.0"));
        assert!(!is_prerelease("1.0.0.1"));
        assert!(is_prerelease("1.0.0-dev"));
    }
}
