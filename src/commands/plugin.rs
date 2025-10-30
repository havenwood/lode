//! Plugin command
//!
//! Manage Bundler plugins

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

// Import gem_install infrastructure
use super::gem_install::{self, InstallOptions};

/// Plugin metadata stored in the index
#[derive(Debug, Clone, Serialize, Deserialize)]
struct PluginInfo {
    name: String,
    version: String,
    source: Option<String>,
    git: Option<String>,
    path: Option<String>,
}

/// Plugin index stored at ~/.bundle/plugin/index
#[derive(Debug, Default, Serialize, Deserialize)]
struct PluginIndex {
    plugins: HashMap<String, PluginInfo>,
}

impl PluginIndex {
    /// Load the plugin index from disk
    fn load() -> Result<Self> {
        let index_path = plugin_index_path()?;
        if !index_path.exists() {
            return Ok(Self::default());
        }

        let content = fs::read_to_string(&index_path)
            .with_context(|| format!("Failed to read plugin index: {}", index_path.display()))?;

        serde_json::from_str(&content).with_context(|| "Failed to parse plugin index")
    }

    /// Save the plugin index to disk
    fn save(&self) -> Result<()> {
        let index_path = plugin_index_path()?;

        // Ensure directory exists
        if let Some(parent) = index_path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("Failed to create plugin directory: {}", parent.display())
            })?;
        }

        let content = serde_json::to_string_pretty(self)
            .with_context(|| "Failed to serialize plugin index")?;

        fs::write(&index_path, content)
            .with_context(|| format!("Failed to write plugin index: {}", index_path.display()))?;

        Ok(())
    }

    /// Add a plugin to the index
    fn add(&mut self, info: PluginInfo) {
        self.plugins.insert(info.name.clone(), info);
    }

    /// Remove a plugin from the index
    fn remove(&mut self, name: &str) -> Option<PluginInfo> {
        self.plugins.remove(name)
    }

    /// Get all plugins
    fn list(&self) -> Vec<&PluginInfo> {
        let mut plugins: Vec<_> = self.plugins.values().collect();
        plugins.sort_by(|a, b| a.name.cmp(&b.name));
        plugins
    }

    /// Clear all plugins
    fn clear(&mut self) {
        self.plugins.clear();
    }
}

/// Get the path to the plugin index file
///
/// Checks `BUNDLE_USER_HOME` environment variable first, otherwise uses `~/.bundle`.
fn plugin_index_path() -> Result<PathBuf> {
    // Check BUNDLE_USER_HOME environment variable first
    let bundle_home = if let Ok(user_home) = std::env::var("BUNDLE_USER_HOME") {
        PathBuf::from(user_home)
    } else {
        let home = dirs::home_dir().with_context(|| "Could not determine home directory")?;
        home.join(".bundle")
    };

    Ok(bundle_home.join("plugin").join("index"))
}

/// Install a plugin
pub(crate) async fn install(
    plugin: &str,
    source: Option<&str>,
    version: Option<&str>,
    git: Option<&str>,
    branch: Option<&str>,
    ref_: Option<&str>,
    path: Option<&str>,
) -> Result<()> {
    println!("Installing plugin: {plugin}");

    // Validate that only one source type is specified
    let source_count = [git.is_some(), path.is_some()]
        .iter()
        .filter(|&&x| x)
        .count();
    if source_count > 1 {
        anyhow::bail!("Cannot specify multiple sources (--git, --path)");
    }

    // Load index
    let mut index = PluginIndex::load()?;

    // Check if already installed
    if index.plugins.contains_key(plugin) {
        println!("Plugin {plugin} is already installed. Uninstall first to reinstall.");
        return Ok(());
    }

    // Determine plugin version by actually installing the gem
    let installed_version: String;

    #[allow(
        clippy::option_if_let_else,
        reason = "if-let chain is clearer than map_or_else for this case"
    )]
    if let Some(_git_url) = git {
        // Git source: Not yet implemented via gem-install
        // Track in index without actual installation
        println!("  Note: Git source plugins require manual installation");
        installed_version = ref_.or(branch).unwrap_or("HEAD").to_string();
    } else if path.is_some() {
        // Path source: Local reference, no installation needed
        println!("  Note: Path source plugins reference local installation");
        installed_version = "local".to_string();
    } else {
        // Regular gem source: Use gem-install infrastructure to actually install
        let install_options = InstallOptions {
            gems: vec![plugin.to_string()],
            platform: None,
            version: version.map(String::from),
            prerelease: false,
            update_sources: false,
            install_dir: None, // Use default system gem directory
            bindir: None,
            document: None,
            no_document: true, // Skip documentation for plugins
            build_root: None,
            vendor: false,
            env_shebang: false,
            force: false,
            wrappers: true,
            trust_policy: None,
            ignore_dependencies: false, // Install dependencies
            format_executable: false,
            user_install: false, // System install for plugins
            development: false,
            development_all: false,
            conservative: false,
            minimal_deps: false,
            post_install_message: true,
            file: None,
            without: None,
            explain: false,
            lock: false,
            suggestions: false,
            target_rbconfig: None,
            local: false,
            remote: false,
            both: true, // Prefer cache but use remote if needed
            bulk_threshold: None,
            clear_sources: false,
            source: source.map(String::from),
            http_proxy: None,
            verbose: true,
            quiet: false,
            silent: false,
            config_file: None,
            backtrace: false,
            debug: false,
            norc: false,
        };

        // Actually install the gem
        gem_install::run(install_options)
            .await
            .with_context(|| format!("Failed to install plugin gem: {plugin}"))?;

        // Get installed version (use specified version or "latest")
        installed_version = version.unwrap_or("latest").to_string();
    }

    // Create plugin info based on source type
    let plugin_info = if let Some(git_url) = git {
        PluginInfo {
            name: plugin.to_string(),
            version: installed_version,
            source: None,
            git: Some(git_url.to_string()),
            path: None,
        }
    } else if let Some(path_str) = path {
        PluginInfo {
            name: plugin.to_string(),
            version: installed_version,
            source: None,
            git: None,
            path: Some(path_str.to_string()),
        }
    } else {
        // Regular gem source
        PluginInfo {
            name: plugin.to_string(),
            version: installed_version,
            source: source.map(String::from),
            git: None,
            path: None,
        }
    };

    // Add to index
    index.add(plugin_info);
    index.save()?;

    println!("Plugin {plugin} installed successfully");

    // Only show Ruby integration note for git/path sources
    if git.is_some() || path.is_some() {
        println!(
            "  Note: Plugin functionality requires integration with Ruby's plugin loading system"
        );
    }

    Ok(())
}

/// Uninstall a plugin
pub(crate) fn uninstall(plugin: Option<&str>, all: bool) -> Result<()> {
    let mut index = PluginIndex::load()?;

    if all {
        if index.plugins.is_empty() {
            println!("No plugins installed");
            return Ok(());
        }

        let count = index.plugins.len();
        index.clear();
        index.save()?;

        println!("Uninstalled {count} plugin(s)");
    } else if let Some(name) = plugin {
        if let Some(info) = index.remove(name) {
            index.save()?;
            println!("Uninstalled plugin: {} ({})", info.name, info.version);
        } else {
            println!("Plugin {name} is not installed");
        }
    } else {
        anyhow::bail!("Must specify plugin name or --all");
    }

    Ok(())
}

/// List installed plugins
pub(crate) fn list() -> Result<()> {
    let index = PluginIndex::load()?;

    if index.plugins.is_empty() {
        println!("No plugins installed");
        return Ok(());
    }

    println!("Installed plugins:");
    for plugin in index.list() {
        if let Some(git) = &plugin.git {
            println!("  {} ({}) - git: {}", plugin.name, plugin.version, git);
        } else if let Some(path) = &plugin.path {
            println!("  {} ({}) - path: {}", plugin.name, plugin.version, path);
        } else if let Some(source) = &plugin.source {
            println!(
                "  {} ({}) - source: {}",
                plugin.name, plugin.version, source
            );
        } else {
            println!("  {} ({})", plugin.name, plugin.version);
        }
    }

    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "Tests can panic")]
mod tests {
    use super::*;

    #[test]
    fn plugin_index_roundtrip() {
        let mut index = PluginIndex::default();

        index.add(PluginInfo {
            name: "test-plugin".to_string(),
            version: "1.0.0".to_string(),
            source: Some("https://rubygems.org".to_string()),
            git: None,
            path: None,
        });

        let json = serde_json::to_string(&index).unwrap();
        let loaded: PluginIndex = serde_json::from_str(&json).unwrap();

        assert_eq!(loaded.plugins.len(), 1);
        assert_eq!(loaded.plugins.get("test-plugin").unwrap().version, "1.0.0");
    }

    #[test]
    fn plugin_index_add_remove() {
        let mut index = PluginIndex::default();

        index.add(PluginInfo {
            name: "plugin1".to_string(),
            version: "1.0.0".to_string(),
            source: None,
            git: None,
            path: None,
        });

        assert_eq!(index.plugins.len(), 1);

        let removed = index.remove("plugin1");
        assert!(removed.is_some());
        assert_eq!(index.plugins.len(), 0);
    }

    #[test]
    fn plugin_index_list_sorted() {
        let mut index = PluginIndex::default();

        for name in ["zebra", "alpha", "beta"] {
            index.add(PluginInfo {
                name: name.to_string(),
                version: "1.0.0".to_string(),
                source: None,
                git: None,
                path: None,
            });
        }

        let list = index.list();
        assert_eq!(list.first().unwrap().name, "alpha");
        assert_eq!(list.get(1).unwrap().name, "beta");
        assert_eq!(list.get(2).unwrap().name, "zebra");
    }
}
