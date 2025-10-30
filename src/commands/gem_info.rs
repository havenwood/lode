//! Info command
//!
//! Show information about a gem

use anyhow::{Context, Result};
use lode::{Config, gem_store::GemStore};
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
struct RemoteGemInfo {
    name: String,
    version: String,
    #[serde(default)]
    platform: String,
    #[serde(default, deserialize_with = "deserialize_string_or_vec")]
    authors: Vec<String>,
    #[serde(default)]
    licenses: Vec<String>,
    #[serde(default)]
    info: String,
    #[serde(default)]
    homepage_uri: String,
    #[serde(default)]
    documentation_uri: String,
    #[serde(default)]
    source_code_uri: String,
    #[serde(default)]
    bug_tracker_uri: String,
    #[serde(default)]
    downloads: u64,
    #[serde(default)]
    dependencies: GemDependencies,
}

/// Custom deserializer that handles both String and Vec<String>
fn deserialize_string_or_vec<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de;

    struct StringOrVec;

    impl<'de> de::Visitor<'de> for StringOrVec {
        type Value = Vec<String>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("string or array of strings")
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(vec![value.to_string()])
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: de::SeqAccess<'de>,
        {
            let mut vec = Vec::new();
            while let Some(value) = seq.next_element()? {
                vec.push(value);
            }
            Ok(vec)
        }
    }

    deserializer.deserialize_any(StringOrVec)
}

#[derive(Debug, Deserialize, Default)]
struct GemDependencies {
    #[serde(default)]
    runtime: Vec<Dependency>,
    #[serde(default)]
    development: Vec<Dependency>,
}

#[derive(Debug, Deserialize)]
struct Dependency {
    name: String,
    #[serde(default)]
    requirements: String,
}

/// Local gem information
#[derive(Debug)]
struct LocalGemInfo {
    name: String,
    version: String,
    path: PathBuf,
    summary: Option<String>,
    homepage: Option<String>,
    authors: Option<String>,
    licenses: Option<String>,
}

/// Options for gem info command
#[derive(Debug)]
pub(crate) struct InfoOptions {
    /// Gem name to query
    pub gem: String,

    /// Check if gem is installed
    pub installed: bool,

    /// Version to check for (with --installed)
    pub version: Option<String>,

    /// Display only gem names (no versions)
    pub versions: bool,

    /// Display all gem versions
    pub all: bool,

    /// Exact name matching
    pub exact: bool,

    /// Include prerelease versions
    pub prerelease: bool,

    /// Deprecated flag - emit warning if used
    pub update_sources: bool,

    /// Local domain only (default: true)
    pub local: bool,

    /// Remote domain only
    pub remote: bool,

    /// Both local and remote
    pub both: bool,

    /// Threshold for switching to bulk synchronization (not used in gem info)
    pub bulk_threshold: usize,

    /// Clear the gem sources
    pub clear_sources: bool,

    /// Append URL to list of remote gem sources
    pub source: Option<String>,

    /// Use HTTP proxy for remote operations
    pub http_proxy: Option<String>,

    /// Verbose mode
    pub verbose: bool,

    /// Quiet mode
    pub quiet: bool,

    /// Silent mode
    pub silent: bool,

    /// Use this config file instead of default
    pub config_file: Option<String>,

    /// Show stack backtrace on errors
    pub backtrace: bool,

    /// Turn on Ruby debugging
    pub debug: bool,

    /// Avoid loading any .gemrc file
    pub norc: bool,
}

impl Default for InfoOptions {
    fn default() -> Self {
        Self {
            gem: String::new(),
            installed: false,
            version: None,
            versions: false,
            all: false,
            exact: false,
            prerelease: false,
            update_sources: false,
            local: true, // gem info defaults to --local
            remote: false,
            both: false,
            bulk_threshold: 1000,
            clear_sources: false,
            source: None,
            http_proxy: None,
            verbose: false,
            quiet: false,
            silent: false,
            config_file: None,
            backtrace: false,
            debug: false,
            norc: false,
        }
    }
}

/// Show detailed information about a gem
pub(crate) async fn run(options: InfoOptions) -> Result<()> {
    // Load config with custom options
    let _config = Config::load_with_options(options.config_file.as_deref(), options.norc)?;

    // Debug logging
    if options.debug {
        eprintln!("DEBUG: gem info query for '{}'", options.gem);
        if options.exact {
            eprintln!("DEBUG: Using exact name match");
        }
        if options.prerelease {
            eprintln!("DEBUG: Including prerelease versions");
        }
    }

    // Emit deprecation warning for --update-sources flag
    if options.update_sources {
        eprintln!(
            "WARNING: The --update-sources flag is deprecated and will be removed in a future version"
        );
    }

    // Handle --clear-sources flag
    if options.clear_sources {
        // --clear-sources in gem info silently clears sources and continues
        // No special output is printed, the query continues normally
        if options.debug {
            eprintln!("DEBUG: --clear-sources flag set (sources cleared)");
        }
    }

    // Handle --bulk-threshold flag
    if options.debug {
        eprintln!(
            "DEBUG: --bulk-threshold set to {} (not used for single gem queries)",
            options.bulk_threshold
        );
    }

    // Handle custom source (override RUBYGEMS_HOST temporarily)
    if options.source.is_some() && options.verbose {
        eprintln!(
            "Warning: --source is not fully supported yet. Use RUBYGEMS_HOST environment variable instead."
        );
    }

    // Handle http-proxy (use HTTP_PROXY environment variable)
    if options.http_proxy.is_some() && options.verbose {
        eprintln!(
            "Warning: --http-proxy is not fully supported yet. Use HTTP_PROXY environment variable instead."
        );
    }

    // Determine mode: local, remote, or both
    let query_local = options.local || options.both || !options.remote;
    let query_remote = options.remote || options.both;

    if options.debug {
        eprintln!("DEBUG: query_local={query_local}, query_remote={query_remote}");
    }

    // Query local gems
    let mut found = if query_local && let Ok(local_found) = show_local_gem_info(&options) {
        local_found
    } else {
        false
    };

    // Query remote gems
    if query_remote {
        match show_remote_gem_info(&options).await {
            Ok(remote_found) => {
                found = found || remote_found;
            }
            Err(err) if options.backtrace => {
                eprintln!("Error fetching remote gem info: {err:#}");
            }
            Err(err) if options.verbose => {
                eprintln!("Error fetching remote gem info: {err}");
            }
            Err(_) => {}
        }
    }

    if !found && !options.quiet && !options.silent {
        println!("Gem '{}' not found", options.gem);
    }

    Ok(())
}

/// Show information for locally installed gems
fn show_local_gem_info(options: &InfoOptions) -> Result<bool> {
    let store = GemStore::new()?;
    let all_gems = store.list_gems()?;

    if options.debug {
        eprintln!("DEBUG: Searching {} local gems", all_gems.len());
    }

    let mut matching_gems = Vec::new();

    for gem in all_gems {
        // Check if name matches (case-insensitive)
        let matches = if options.exact {
            gem.name.to_lowercase() == options.gem.to_lowercase()
        } else {
            gem.name
                .to_lowercase()
                .contains(&options.gem.to_lowercase())
        };

        if !matches {
            continue;
        }

        // Filter prerelease versions if not requested
        if !options.prerelease && is_prerelease(&gem.version) {
            continue;
        }

        // Filter by specific version if requested
        if let Some(ref req_version) = options.version
            && gem.version != *req_version
        {
            continue;
        }

        // Read gem metadata from gemspec
        let local_info = read_local_gem_metadata(&gem.name, &gem.version, &gem.path);
        matching_gems.push(local_info);
    }

    if options.debug {
        eprintln!("DEBUG: Found {} matching local gems", matching_gems.len());
    }

    if matching_gems.is_empty() {
        return Ok(false);
    }

    // Handle --installed check
    if options.installed {
        if options.debug {
            eprintln!("DEBUG: Gem is installed");
        }
        if !options.quiet && !options.silent {
            println!("true");
        }
        return Ok(true);
    }

    // Display gems
    if options.versions {
        // Just gem names
        for gem in &matching_gems {
            if !options.quiet && !options.silent {
                println!("{}", gem.name);
            }
        }
    } else if options.all {
        // Show all versions
        for gem in &matching_gems {
            display_local_gem_info(gem, options.quiet || options.silent, options.verbose);
        }
    } else {
        // Show latest version only
        if let Some(latest) = get_latest_gem(&matching_gems) {
            display_local_gem_info(latest, options.quiet || options.silent, options.verbose);
        }
    }

    Ok(!matching_gems.is_empty())
}

/// Show information for remote gems (RubyGems.org or `RUBYGEMS_HOST`)
async fn show_remote_gem_info(options: &InfoOptions) -> Result<bool> {
    let host = lode::env_vars::rubygems_host();
    let url = format!("{}/api/v1/gems/{}.json", host, options.gem);

    if options.debug {
        eprintln!("DEBUG: Fetching remote gem info from: {url}");
    }

    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .send()
        .await
        .context("Failed to fetch gem info")?;

    if response.status() == 404 {
        if options.debug {
            eprintln!("DEBUG: Remote gem not found (404)");
        }
        return Ok(false);
    }

    if !response.status().is_success() {
        let msg = format!("Failed to fetch gem info: {}", response.status());
        if options.debug {
            eprintln!("DEBUG: {msg}");
        }
        anyhow::bail!(msg);
    }

    let info: RemoteGemInfo = response.json().await.context("Failed to parse gem info")?;

    if options.debug {
        eprintln!("DEBUG: Successfully parsed remote gem info");
    }

    // Handle --installed check (remote gems are never "installed")
    if options.installed {
        std::process::exit(1);
    }

    if options.versions {
        if !options.quiet && !options.silent {
            println!("{}", info.name);
        }
    } else {
        display_remote_gem_info(&info, options.quiet || options.silent, options.verbose);
    }

    Ok(true)
}

/// Read metadata from a local gem's gemspec file
fn read_local_gem_metadata(name: &str, version: &str, gem_path: &PathBuf) -> LocalGemInfo {
    let mut summary = None;
    let mut homepage = None;
    let mut authors = None;
    let mut licenses = None;

    // Try to find and read gemspec file
    if let Ok(entries) = fs::read_dir(gem_path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(ext) = path.extension()
                && ext == "gemspec"
            {
                if let Ok(content) = fs::read_to_string(&path) {
                    summary = extract_gemspec_field(&content, "summary");
                    homepage = extract_gemspec_field(&content, "homepage");
                    authors = extract_gemspec_field(&content, "authors");
                    licenses = extract_gemspec_field(&content, "licenses");
                }
                break;
            }
        }
    }

    LocalGemInfo {
        name: name.to_string(),
        version: version.to_string(),
        path: gem_path.clone(),
        summary,
        homepage,
        authors,
        licenses,
    }
}

/// Display local gem information
fn display_local_gem_info(gem: &LocalGemInfo, quiet: bool, verbose: bool) {
    if quiet {
        return;
    }

    println!("\n*** {} ({}) ***", gem.name, gem.version);
    println!("    Path: {}", gem.path.display());

    if let Some(ref summary) = gem.summary {
        println!("    Summary: {summary}");
    }

    if let Some(ref homepage) = gem.homepage {
        println!("    Homepage: {homepage}");
    }

    if let Some(ref authors) = gem.authors {
        println!("    Authors: {authors}");
    }

    if let Some(ref licenses) = gem.licenses {
        println!("    Licenses: {licenses}");
    }

    // Show additional details in verbose mode
    if verbose {
        println!("    Platform: ruby");
    }
}

/// Display remote gem information
fn display_remote_gem_info(info: &RemoteGemInfo, quiet: bool, verbose: bool) {
    if quiet {
        return;
    }

    println!("\n*** {} ***", info.name);
    println!("version: {}", info.version);

    if !info.platform.is_empty() && info.platform != "ruby" {
        println!("platform: {}", info.platform);
    } else if verbose {
        println!("platform: ruby");
    }

    if !info.authors.is_empty() {
        println!("authors: {}", info.authors.join(", "));
    }

    if !info.licenses.is_empty() {
        println!("licenses: {}", info.licenses.join(", "));
    }

    println!("downloads: {}", format_number(info.downloads));

    if !info.info.is_empty() {
        println!("\ndescription:");
        println!("{}", info.info);
    }

    if !info.homepage_uri.is_empty() {
        println!("\nhomepage: {}", info.homepage_uri);
    }

    if !info.source_code_uri.is_empty() {
        println!("source: {}", info.source_code_uri);
    }

    if !info.documentation_uri.is_empty() {
        println!("docs: {}", info.documentation_uri);
    }

    if !info.bug_tracker_uri.is_empty() {
        println!("bug tracker: {}", info.bug_tracker_uri);
    }

    // Show dependencies
    if !info.dependencies.runtime.is_empty() {
        println!("\nruntime dependencies:");
        for dep in &info.dependencies.runtime {
            println!("  - {} {}", dep.name, dep.requirements);
        }
    }

    if !info.dependencies.development.is_empty() {
        println!("\ndevelopment dependencies:");
        for dep in &info.dependencies.development {
            println!("  - {} {}", dep.name, dep.requirements);
        }
    }
}

/// Check if a version string is a prerelease
fn is_prerelease(version: &str) -> bool {
    version.contains('-')
        || version.contains(".pre")
        || version.contains(".rc")
        || version.contains(".alpha")
        || version.contains(".beta")
}

/// Extract a field value from gemspec content
fn extract_gemspec_field(content: &str, field: &str) -> Option<String> {
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with(&format!("s.{field}"))
            || trimmed.starts_with(&format!("spec.{field}"))
        {
            return extract_string_value(line);
        }
    }
    None
}

/// Extract quoted string value from a gemspec line
fn extract_string_value(line: &str) -> Option<String> {
    if let Some(start) = line.find(['"', '\'']) {
        let quote = line.chars().nth(start)?;
        let rest = &line[start + 1..];
        if let Some(end) = rest.find(quote) {
            return Some(rest[..end].to_string());
        }
    }
    None
}

/// Get the latest gem from a list
fn get_latest_gem(gems: &[LocalGemInfo]) -> Option<&LocalGemInfo> {
    gems.iter()
        .max_by(|a, b| version_sort_key(&a.version).cmp(&version_sort_key(&b.version)))
}

/// Create a sortable key from a version string
fn version_sort_key(version: &str) -> Vec<u64> {
    version
        .split(&['.', '-', '+'][..])
        .filter_map(|part| part.parse::<u64>().ok())
        .collect()
}

fn format_number(num: u64) -> String {
    let string = num.to_string();
    let mut result = String::new();
    let chars: Vec<char> = string.chars().rev().collect();

    for (index, character) in chars.iter().enumerate() {
        if index > 0 && index % 3 == 0 {
            result.push(',');
        }
        result.push(*character);
    }

    result.chars().rev().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_prerelease() {
        assert!(!is_prerelease("1.2.3"));
        assert!(!is_prerelease("0.1.0"));
        assert!(is_prerelease("1.2.3-alpha"));
        assert!(is_prerelease("2.0.0.pre"));
        assert!(is_prerelease("1.0.0.rc1"));
        assert!(is_prerelease("3.0.0.alpha.1"));
        assert!(is_prerelease("1.0.0.beta"));
    }

    #[test]
    fn test_format_number() {
        assert_eq!(format_number(0), "0");
        assert_eq!(format_number(123), "123");
        assert_eq!(format_number(1234), "1,234");
        assert_eq!(format_number(1_234_567), "1,234,567");
        assert_eq!(format_number(1_000_000_000), "1,000,000,000");
    }

    #[test]
    fn test_version_sort_key() {
        assert_eq!(version_sort_key("1.2.3"), vec![1, 2, 3]);
        assert_eq!(version_sort_key("10.0.0"), vec![10, 0, 0]);
        assert_eq!(version_sort_key("2.0.0-alpha"), vec![2, 0, 0]);
        assert_eq!(version_sort_key("1.2.3.4"), vec![1, 2, 3, 4]);
    }

    #[test]
    fn test_extract_string_value() {
        assert_eq!(
            extract_string_value("  s.summary = \"A test gem\""),
            Some("A test gem".to_string())
        );
        assert_eq!(
            extract_string_value("  s.homepage = 'https://example.com'"),
            Some("https://example.com".to_string())
        );
        assert_eq!(extract_string_value("  s.version = "), None);
    }

    #[test]
    fn test_extract_gemspec_field() {
        let gemspec = r#"
Gem::Specification.new do |s|
  s.name = "test"
  s.summary = "A test gem"
  s.homepage = "https://example.com"
end
"#;
        assert_eq!(
            extract_gemspec_field(gemspec, "summary"),
            Some("A test gem".to_string())
        );
        assert_eq!(
            extract_gemspec_field(gemspec, "homepage"),
            Some("https://example.com".to_string())
        );
        assert_eq!(extract_gemspec_field(gemspec, "license"), None);
    }

    #[test]
    fn test_info_options_default() {
        let opts = InfoOptions::default();
        assert_eq!(opts.gem, "");
        assert!(!opts.installed);
        assert!(!opts.versions);
        assert!(!opts.all);
        assert!(!opts.exact);
        assert!(!opts.prerelease);
        assert!(opts.local);
        assert!(!opts.remote);
        assert!(!opts.both);
        assert_eq!(opts.bulk_threshold, 1000);
        assert!(!opts.clear_sources);
        assert!(opts.source.is_none());
        assert!(opts.http_proxy.is_none());
        assert!(!opts.verbose);
        assert!(!opts.quiet);
        assert!(!opts.silent);
        assert!(opts.config_file.is_none());
        assert!(!opts.backtrace);
        assert!(!opts.debug);
        assert!(!opts.norc);
    }

    #[test]
    fn test_info_options_installed_flag() {
        let mut opts = InfoOptions::default();
        assert!(!opts.installed);
        opts.installed = true;
        assert!(opts.installed);
    }

    #[test]
    fn test_info_options_versions_flag() {
        let mut opts = InfoOptions::default();
        assert!(!opts.versions);
        opts.versions = true;
        assert!(opts.versions);
    }

    #[test]
    fn test_info_options_all_flag() {
        let mut opts = InfoOptions::default();
        assert!(!opts.all);
        opts.all = true;
        assert!(opts.all);
    }

    #[test]
    fn test_info_options_exact_match() {
        let mut opts = InfoOptions::default();
        assert!(!opts.exact);
        opts.exact = true;
        assert!(opts.exact);
    }

    #[test]
    fn test_info_options_prerelease_flag() {
        let mut opts = InfoOptions::default();
        assert!(!opts.prerelease);
        opts.prerelease = true;
        assert!(opts.prerelease);
    }

    #[test]
    fn test_info_options_source_selection_local() {
        let mut opts = InfoOptions::default();
        assert!(opts.local);
        assert!(!opts.remote);
        opts.local = true;
        opts.remote = false;
        assert!(opts.local);
    }

    #[test]
    fn test_info_options_source_selection_remote() {
        let opts = InfoOptions {
            local: false,
            remote: true,
            ..Default::default()
        };
        assert!(opts.remote);
        assert!(!opts.local);
    }

    #[test]
    fn test_info_options_source_selection_both() {
        let opts = InfoOptions {
            both: true,
            ..Default::default()
        };
        assert!(opts.both);
    }

    #[test]
    fn test_info_options_bulk_threshold() {
        let mut opts = InfoOptions::default();
        assert_eq!(opts.bulk_threshold, 1000);
        opts.bulk_threshold = 500;
        assert_eq!(opts.bulk_threshold, 500);
    }

    #[test]
    fn test_info_options_source_url() {
        let mut opts = InfoOptions::default();
        assert!(opts.source.is_none());
        opts.source = Some("https://gems.example.com".to_string());
        assert_eq!(opts.source, Some("https://gems.example.com".to_string()));
    }

    #[test]
    fn test_info_options_http_proxy() {
        let mut opts = InfoOptions::default();
        assert!(opts.http_proxy.is_none());
        opts.http_proxy = Some("http://proxy.example.com:8080".to_string());
        assert_eq!(
            opts.http_proxy,
            Some("http://proxy.example.com:8080".to_string())
        );
    }

    #[test]
    fn test_info_options_output_control() {
        let mut opts = InfoOptions::default();
        assert!(!opts.verbose);
        assert!(!opts.quiet);
        assert!(!opts.silent);

        opts.verbose = true;
        opts.quiet = true;

        assert!(opts.verbose);
        assert!(opts.quiet);
    }

    #[test]
    fn test_info_options_complex_scenario() {
        // Test gem info with version filtering and verbose output
        let opts = InfoOptions {
            gem: "rails".to_string(),
            versions: true,
            remote: true,
            local: false,
            verbose: true,
            prerelease: true,
            ..Default::default()
        };

        assert_eq!(opts.gem, "rails");
        assert!(opts.versions);
        assert!(opts.remote);
        assert!(!opts.local);
        assert!(opts.verbose);
        assert!(opts.prerelease);
    }
}
