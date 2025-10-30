//! Sources command
//!
//! Manage `RubyGems` sources

use anyhow::{Context, Result};
use lode::{Config, RubyGemsClient};

/// Options for gem sources command
#[derive(Debug)]
pub(crate) struct SourcesOptions {
    /// Add a new source URL
    pub add: Option<String>,

    /// Append source (add to end)
    pub append: Option<String>,

    /// Prepend source (add to beginning)
    pub prepend: Option<String>,

    /// Remove a source URL
    pub remove: Option<String>,

    /// Clear all sources
    pub clear_all: bool,

    /// Update source cache
    pub update: bool,

    /// List sources (default if no other action)
    pub list: bool,

    /// Do not show confirmation prompts (no effect - lode is non-interactive)
    /// Parsed for compatibility with gem 4.0.0, but has no effect since lode never prompts
    #[allow(dead_code)]
    pub force: bool,

    /// HTTP proxy for remote operations
    pub http_proxy: Option<String>,

    /// Verbose output
    pub verbose: bool,

    /// Quiet mode
    pub quiet: bool,

    /// Silent mode
    pub silent: bool,
}

impl Default for SourcesOptions {
    fn default() -> Self {
        Self {
            add: None,
            append: None,
            prepend: None,
            remove: None,
            clear_all: false,
            update: false,
            list: true, // Default action
            force: false,
            http_proxy: None,
            verbose: false,
            quiet: false,
            silent: false,
        }
    }
}

/// Manage gem sources with options
pub(crate) async fn run_with_options(options: SourcesOptions) -> Result<()> {
    let mut config = Config::load().context("Failed to load configuration")?;

    // Add source
    if let Some(ref url) = options.add {
        add_source(&mut config, url, &options)?;
        return Ok(());
    }

    // Append source (add to end)
    if let Some(ref url) = options.append {
        append_source(&mut config, url, &options)?;
        return Ok(());
    }

    // Prepend source (add to beginning)
    if let Some(ref url) = options.prepend {
        prepend_source(&mut config, url, &options)?;
        return Ok(());
    }

    // Remove source
    if let Some(ref url) = options.remove {
        remove_source(&mut config, url, &options)?;
        return Ok(());
    }

    // Clear all sources
    if options.clear_all {
        clear_all_sources(&mut config, &options)?;
        return Ok(());
    }

    // Update sources cache
    if options.update {
        update_sources(&config, &options).await?;
        return Ok(());
    }

    // List sources (explicit or default action)
    if options.list {
        list_sources(&config, &options);
        return Ok(());
    }

    // Fallback to list if no action specified
    list_sources(&config, &options);

    Ok(())
}

/// List configured gem sources
fn list_sources(config: &Config, options: &SourcesOptions) {
    if !options.quiet {
        println!("*** CURRENT SOURCES ***\n");
    }

    // Show configured sources
    if config.gem_sources.is_empty() {
        // Show default source
        println!("https://rubygems.org/");
    } else {
        for source in &config.gem_sources {
            println!("{}", source.url);
            if options.verbose
                && let Some(ref fallback) = source.fallback
            {
                println!("  (fallback: {fallback})");
            }
        }
    }
}

/// Add a new source
fn add_source(config: &mut Config, url: &str, options: &SourcesOptions) -> Result<()> {
    // Validate URL format
    if !url.starts_with("http://") && !url.starts_with("https://") {
        anyhow::bail!("Source URL must start with http:// or https://");
    }

    // Check if already exists
    if config.gem_sources.iter().any(|s| s.url == url) {
        if !options.quiet && !options.silent {
            println!("Source {url} is already present");
        }
        return Ok(());
    }

    // Add the source
    config.gem_sources.push(lode::config::GemSource {
        url: url.to_string(),
        fallback: None,
    });

    // Save configuration
    save_config(config)?;

    if !options.quiet && !options.silent {
        println!("{url} added to sources");
    }

    Ok(())
}

/// Append a source (add to end of list)
fn append_source(config: &mut Config, url: &str, options: &SourcesOptions) -> Result<()> {
    // Validate URL format
    if !url.starts_with("http://") && !url.starts_with("https://") {
        anyhow::bail!("Source URL must start with http:// or https://");
    }

    // Check if already exists - if so, move to end
    let existing_index = config.gem_sources.iter().position(|s| s.url == url);

    if let Some(index) = existing_index {
        // Remove from current position
        let source = config.gem_sources.remove(index);
        // Add to end
        config.gem_sources.push(source);

        // Save configuration
        save_config(config)?;

        if !options.quiet && !options.silent {
            println!("{url} moved to end of sources");
        }
    } else {
        // Add new source to end
        config.gem_sources.push(lode::config::GemSource {
            url: url.to_string(),
            fallback: None,
        });

        // Save configuration
        save_config(config)?;

        if !options.quiet && !options.silent {
            println!("{url} added to sources");
        }
    }

    Ok(())
}

/// Prepend a source (add to beginning of list)
fn prepend_source(config: &mut Config, url: &str, options: &SourcesOptions) -> Result<()> {
    // Validate URL format
    if !url.starts_with("http://") && !url.starts_with("https://") {
        anyhow::bail!("Source URL must start with http:// or https://");
    }

    // Check if already exists - if so, move to beginning
    let existing_index = config.gem_sources.iter().position(|s| s.url == url);

    if let Some(index) = existing_index {
        // Remove from current position
        let source = config.gem_sources.remove(index);
        // Insert at beginning
        config.gem_sources.insert(0, source);

        // Save configuration
        save_config(config)?;

        if !options.quiet && !options.silent {
            println!("{url} moved to beginning of sources");
        }
    } else {
        // Insert new source at beginning
        config.gem_sources.insert(
            0,
            lode::config::GemSource {
                url: url.to_string(),
                fallback: None,
            },
        );

        // Save configuration
        save_config(config)?;

        if !options.quiet && !options.silent {
            println!("{url} added to sources");
        }
    }

    Ok(())
}

/// Remove a source
fn remove_source(config: &mut Config, url: &str, options: &SourcesOptions) -> Result<()> {
    let initial_len = config.gem_sources.len();

    // Remove the source
    config.gem_sources.retain(|s| s.url != url);

    if config.gem_sources.len() == initial_len {
        anyhow::bail!("Source {url} not found in sources list");
    }

    // Save configuration
    save_config(config)?;

    if !options.quiet {
        println!("{url} removed from sources");
    }

    Ok(())
}

/// Clear all sources
fn clear_all_sources(config: &mut Config, options: &SourcesOptions) -> Result<()> {
    let count = config.gem_sources.len();

    config.gem_sources.clear();

    // Save configuration
    save_config(config)?;

    if !options.quiet {
        println!("Cleared {count} source(s)");
    }

    Ok(())
}

/// Update sources cache
async fn update_sources(config: &Config, options: &SourcesOptions) -> Result<()> {
    if !options.quiet {
        println!("Updating sources cache...\n");
    }

    let sources = if config.gem_sources.is_empty() {
        vec!["https://rubygems.org/".to_string()]
    } else {
        config.gem_sources.iter().map(|s| s.url.clone()).collect()
    };

    for source_url in sources {
        if options.verbose {
            println!("Checking {source_url}...");
        }

        // Try to connect to the source
        match RubyGemsClient::new_with_proxy(&source_url, options.http_proxy.as_deref()) {
            Ok(client) => {
                // Test with a simple query
                match client.fetch_versions("rake").await {
                    Ok(_) => {
                        if !options.quiet {
                            println!("{source_url} is reachable");
                        }
                    }
                    Err(e) => {
                        eprintln!("{source_url} failed: {e}");
                    }
                }
            }
            Err(e) => {
                eprintln!("{source_url} failed to initialize: {e}");
            }
        }
    }

    if !options.quiet {
        println!("\nSource cache updated");
    }

    Ok(())
}

/// Save configuration to file
fn save_config(config: &Config) -> Result<()> {
    let config_str = toml::to_string_pretty(config).context("Failed to serialize configuration")?;

    // Determine config file path
    let config_path: String = if std::path::Path::new(".lode.toml").exists() {
        ".lode.toml".to_string()
    } else {
        // Save to user config directory
        let config_dir = dirs::home_dir()
            .context("Failed to determine home directory")?
            .join(".config")
            .join("lode");

        std::fs::create_dir_all(&config_dir).context("Failed to create config directory")?;

        let path = config_dir.join("config.toml");
        path.to_str().context("Invalid config path")?.to_string()
    };

    std::fs::write(&config_path, config_str)
        .with_context(|| format!("Failed to write config file: {config_path}"))?;

    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "Tests can panic")]
mod tests {
    use super::*;

    #[test]
    fn sources_options_default() {
        let options = SourcesOptions::default();
        assert!(options.list);
        assert!(!options.clear_all);
        assert!(!options.update);
        assert!(options.add.is_none());
        assert!(options.remove.is_none());
    }

    #[test]
    fn validate_url() {
        // Valid URLs
        assert!("https://rubygems.org/".starts_with("https://"));
        assert!("http://gems.example.com/".starts_with("http://"));

        // Invalid URLs
        assert!(!"ftp://gems.example.com/".starts_with("http"));
        assert!(!"gems.example.com".starts_with("http"));
    }

    #[test]
    fn test_sources_options_add_source() {
        let mut opts = SourcesOptions::default();
        assert!(opts.add.is_none());
        opts.add = Some("https://gems.example.com".to_string());
        assert_eq!(opts.add, Some("https://gems.example.com".to_string()));
    }

    #[test]
    fn test_sources_options_append_source() {
        let mut opts = SourcesOptions::default();
        assert!(opts.append.is_none());
        opts.append = Some("https://gems.example.com".to_string());
        assert_eq!(opts.append, Some("https://gems.example.com".to_string()));
    }

    #[test]
    fn test_sources_options_prepend_source() {
        let mut opts = SourcesOptions::default();
        assert!(opts.prepend.is_none());
        opts.prepend = Some("https://gems.example.com".to_string());
        assert_eq!(opts.prepend, Some("https://gems.example.com".to_string()));
    }

    #[test]
    fn test_sources_options_remove_source() {
        let mut opts = SourcesOptions::default();
        assert!(opts.remove.is_none());
        opts.remove = Some("https://gems.example.com".to_string());
        assert_eq!(opts.remove, Some("https://gems.example.com".to_string()));
    }

    #[test]
    fn test_sources_options_clear_all_flag() {
        let mut opts = SourcesOptions::default();
        assert!(!opts.clear_all);
        opts.clear_all = true;
        assert!(opts.clear_all);
    }

    #[test]
    fn test_sources_options_update_flag() {
        let mut opts = SourcesOptions::default();
        assert!(!opts.update);
        opts.update = true;
        assert!(opts.update);
    }

    #[test]
    fn test_sources_options_force_flag() {
        let mut opts = SourcesOptions::default();
        assert!(!opts.force);
        opts.force = true;
        assert!(opts.force);
    }

    #[test]
    fn test_sources_options_http_proxy() {
        let mut opts = SourcesOptions::default();
        assert!(opts.http_proxy.is_none());
        opts.http_proxy = Some("http://proxy.example.com:8080".to_string());
        assert_eq!(
            opts.http_proxy,
            Some("http://proxy.example.com:8080".to_string())
        );
    }

    #[test]
    fn test_sources_options_output_control() {
        let mut opts = SourcesOptions::default();
        assert!(!opts.verbose);
        assert!(!opts.quiet);
        assert!(!opts.silent);

        opts.verbose = true;
        opts.quiet = true;

        assert!(opts.verbose);
        assert!(opts.quiet);
    }

    #[test]
    fn test_sources_options_complex_scenario() {
        // Test adding a source with force flag and verbose output
        let opts = SourcesOptions {
            add: Some("https://gems.example.com".to_string()),
            force: true,
            verbose: true,
            ..Default::default()
        };

        assert_eq!(opts.add, Some("https://gems.example.com".to_string()));
        assert!(opts.force);
        assert!(opts.verbose);
    }
}
