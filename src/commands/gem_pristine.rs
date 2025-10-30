//! Pristine command
//!
//! Restore installed gems to pristine condition

use anyhow::{Context, Result};
use flate2::read::GzDecoder;
use lode::extensions::{builder::ExtensionBuilder, detector::detect_extension};
use lode::{Config, config, get_system_gem_dir, parse_gem_name};
use std::fs;
use std::path::{Path, PathBuf};
use tar::Archive;

/// Options for gem pristine command
#[derive(Debug)]
pub(crate) struct PristineOptions {
    /// Gem names to restore (empty = requires --all)
    pub gems: Vec<String>,

    /// Restore all installed gems
    pub all: bool,

    /// Skip gems with these names (with --all)
    pub skip: Vec<String>,

    /// Restore gems with extensions
    pub extensions: bool,

    /// Only restore gems with missing extensions
    pub only_missing_extensions: bool,

    /// Only restore executables
    pub only_executables: bool,

    /// Only restore plugins
    pub only_plugins: bool,

    /// Rewrite executables with /usr/bin/env shebang
    pub env_shebang: bool,

    /// Gem repository directory
    pub install_dir: Option<PathBuf>,

    /// Directory where executables are located
    pub bindir: Option<PathBuf>,

    /// Specific version to restore
    pub version: Option<String>,

    /// Verbose output
    pub verbose: bool,

    /// Quiet mode
    pub quiet: bool,

    /// Config file path
    pub config_file: Option<String>,

    /// Avoid loading .gemrc file
    pub norc: bool,
}

impl Default for PristineOptions {
    fn default() -> Self {
        Self {
            gems: Vec::new(),
            all: false,
            skip: Vec::new(),
            extensions: true, // Default: restore extensions
            only_missing_extensions: false,
            only_executables: false,
            only_plugins: false,
            env_shebang: false,
            install_dir: None,
            bindir: None,
            version: None,
            verbose: false,
            quiet: false,
            config_file: None,
            norc: false,
        }
    }
}

/// Gem information
#[derive(Debug, Clone)]
struct GemInfo {
    name: String,
    version: String,
    path: PathBuf,
}

/// Restore gems to pristine condition
pub(crate) fn run(options: &PristineOptions) -> Result<()> {
    if !options.all && options.gems.is_empty() {
        anyhow::bail!("Specify gem names or use --all to restore all gems");
    }

    // Get Ruby version and directories
    let config = Config::load_with_options(options.config_file.as_deref(), options.norc)
        .context("Failed to load configuration")?;
    let ruby_ver = config::ruby_version(None);

    let gem_dir = options
        .install_dir
        .clone()
        .unwrap_or_else(|| get_system_gem_dir(&ruby_ver));

    if !gem_dir.exists() {
        if !options.quiet {
            println!("Gem directory does not exist: {}", gem_dir.display());
        }
        return Ok(());
    }

    let cache_dir = config::cache_dir(Some(&config))?;

    // Find gems to restore
    let gems_to_restore = if options.all {
        find_all_gems(&gem_dir, options)?
    } else {
        find_specific_gems(&gem_dir, options)?
    };

    if gems_to_restore.is_empty() {
        if !options.quiet {
            println!("No gems to restore");
        }
        return Ok(());
    }

    if !options.quiet {
        println!(
            "Restoring {} gem(s) to pristine condition...\n",
            gems_to_restore.len()
        );
    }

    let mut restored_count = 0;
    let mut failed_count = 0;

    for gem in gems_to_restore {
        if !options.quiet {
            println!("Restoring {} ({})...", gem.name, gem.version);
        }

        match restore_gem(&gem, &cache_dir, options) {
            Ok(()) => {
                restored_count += 1;
                if options.verbose {
                    println!("  Successfully restored {} ({})", gem.name, gem.version);
                }
            }
            Err(err) => {
                failed_count += 1;
                eprintln!(
                    "  Failed to restore {} ({}): {}",
                    gem.name, gem.version, err
                );
            }
        }
    }

    if !options.quiet {
        println!("\nRestored {restored_count} gem(s)");
        if failed_count > 0 {
            println!("Failed to restore {failed_count} gem(s)");
        }
    }

    Ok(())
}

/// Find all gems in the gem directory
fn find_all_gems(gem_dir: &PathBuf, options: &PristineOptions) -> Result<Vec<GemInfo>> {
    let entries = fs::read_dir(gem_dir)
        .with_context(|| format!("Failed to read gem directory: {}", gem_dir.display()))?;

    let mut gems = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        if let Some(dir_name) = path.file_name().and_then(|n| n.to_str())
            && let Some((name, version)) = parse_gem_name(dir_name)
        {
            // Skip gems in skip list
            if options.skip.contains(&name.to_string()) {
                continue;
            }

            gems.push(GemInfo {
                name: name.to_string(),
                version: version.to_string(),
                path: path.clone(),
            });
        }
    }

    Ok(gems)
}

/// Find specific gems by name
fn find_specific_gems(gem_dir: &PathBuf, options: &PristineOptions) -> Result<Vec<GemInfo>> {
    let mut gems = Vec::new();

    for gem_name in &options.gems {
        let matching = find_gem_by_name(gem_dir, gem_name, options.version.as_deref())?;
        gems.extend(matching);
    }

    Ok(gems)
}

/// Find a gem by name
fn find_gem_by_name(
    gem_dir: &PathBuf,
    gem_name: &str,
    version: Option<&str>,
) -> Result<Vec<GemInfo>> {
    let entries = fs::read_dir(gem_dir)
        .with_context(|| format!("Failed to read gem directory: {}", gem_dir.display()))?;

    let mut matching = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        if let Some(dir_name) = path.file_name().and_then(|n| n.to_str())
            && let Some((name, ver)) = parse_gem_name(dir_name)
        {
            if name != gem_name {
                continue;
            }

            // Filter by version if specified
            if let Some(req_version) = version
                && ver != req_version
            {
                continue;
            }

            matching.push(GemInfo {
                name: name.to_string(),
                version: ver.to_string(),
                path: path.clone(),
            });
        }
    }

    Ok(matching)
}

/// Restore a single gem to pristine condition
fn restore_gem(
    gem: &GemInfo,
    cache_dir: &std::path::Path,
    options: &PristineOptions,
) -> Result<()> {
    // Find cached .gem file
    let gem_file = format!("{}-{}.gem", gem.name, gem.version);
    let cached_gem_path = cache_dir.join(&gem_file);

    if !cached_gem_path.exists() {
        anyhow::bail!(
            "Cached gem file not found: {}. Try downloading it first with: lode gem-fetch {}",
            cached_gem_path.display(),
            gem.name
        );
    }

    // Only restore executables
    if options.only_executables {
        if options.verbose {
            println!("    Restoring executables only...");
        }
        extract_specific_directories(&cached_gem_path, &gem.path, &["exe/", "bin/"], options)?;
        if options.verbose {
            println!("    Executables restored");
        }
        return Ok(());
    }

    // Only restore plugins
    if options.only_plugins {
        if options.verbose {
            println!("    Restoring plugins only...");
        }
        extract_specific_directories(&cached_gem_path, &gem.path, &["plugins/"], options)?;
        if options.verbose {
            println!("    Plugins restored");
        }
        return Ok(());
    }

    // Extract gem file over existing installation
    if options.verbose {
        println!("    Extracting from cache: {}", cached_gem_path.display());
    }

    extract_gem_to_directory(&cached_gem_path, &gem.path, options)?;

    // Rebuild extensions if requested
    if options.extensions || options.only_missing_extensions {
        if options.verbose {
            println!("    Checking for extensions...");
        }

        // Detect extension type
        let ext_type = detect_extension(&gem.path, &gem.name, None);

        // Check if this gem has extensions
        if ext_type.needs_building() {
            if options.verbose {
                println!("    Found: {}", ext_type.description());
            }

            // Build the extension
            let mut builder = ExtensionBuilder::new(false, options.verbose, None);

            if options.verbose {
                println!("    Building extension...");
            }

            match builder.build_if_needed(&gem.name, &gem.path, None) {
                Some(result) => {
                    if result.success {
                        if options.verbose {
                            println!("      Successfully rebuilt in {:?}", result.duration);
                        }
                    } else {
                        eprintln!(
                            "    Failed to rebuild extension: {}",
                            result.error.unwrap_or_else(|| "Unknown error".to_string())
                        );
                        if !result.output.is_empty() && options.verbose {
                            eprintln!("    Output: {}", result.output);
                        }
                    }
                }
                None => {
                    if options.verbose {
                        println!("       Extension already built");
                    }
                }
            }
        } else if options.verbose {
            println!("    No extensions to build ({})", ext_type.description());
        }
    }

    Ok(())
}

/// Extract a .gem file to a directory
fn extract_gem_to_directory(
    gem_path: &PathBuf,
    dest_dir: &PathBuf,
    options: &PristineOptions,
) -> Result<()> {
    // Remove existing directory
    if dest_dir.exists() {
        fs::remove_dir_all(dest_dir).with_context(|| {
            format!(
                "Failed to remove existing directory: {}",
                dest_dir.display()
            )
        })?;
    }

    // Gem files are tar.gz archives containing:
    // - metadata.gz
    // - data.tar.gz (actual gem contents)
    // - checksums.yaml.gz

    let file = fs::File::open(gem_path)
        .with_context(|| format!("Failed to open gem file: {}", gem_path.display()))?;

    let decoder = GzDecoder::new(file);
    let mut archive = Archive::new(decoder);

    // Extract to temporary directory first
    let temp_dir = std::env::temp_dir().join(format!(
        "lode-pristine-{}",
        dest_dir.file_name().unwrap().to_string_lossy()
    ));
    fs::create_dir_all(&temp_dir)?;

    // Find and extract data.tar.gz
    for entry in archive.entries()? {
        let mut entry = entry?;
        let path = entry.path()?.to_path_buf();

        if path.to_string_lossy() == "data.tar.gz" {
            let data_path = temp_dir.join("data.tar.gz");
            entry.unpack(&data_path)?;

            // Extract the actual gem contents
            let data_file = fs::File::open(&data_path)?;
            let data_decoder = GzDecoder::new(data_file);
            let mut data_archive = Archive::new(data_decoder);

            fs::create_dir_all(dest_dir)?;

            data_archive
                .unpack(dest_dir)
                .with_context(|| format!("Failed to extract gem data to {}", dest_dir.display()))?;

            // Cleanup temp directory
            drop(fs::remove_dir_all(&temp_dir));

            if options.verbose {
                println!("    Extracted to: {}", dest_dir.display());
            }

            return Ok(());
        }
    }

    anyhow::bail!("Invalid gem file: data.tar.gz not found")
}

/// Extract specific directories from a .gem file
///
/// Used for --only-executables and --only-plugins flags to restore only specific
/// parts of a gem installation.
fn extract_specific_directories(
    gem_path: &Path,
    dest_dir: &Path,
    directory_prefixes: &[&str],
    options: &PristineOptions,
) -> Result<()> {
    // Gem files are tar.gz archives containing data.tar.gz with actual contents
    let file = fs::File::open(gem_path)
        .with_context(|| format!("Failed to open gem file: {}", gem_path.display()))?;

    let decoder = GzDecoder::new(file);
    let mut archive = Archive::new(decoder);

    // Extract to temporary directory first
    let temp_dir = std::env::temp_dir().join(format!(
        "lode-pristine-specific-{}",
        dest_dir.file_name().unwrap().to_string_lossy()
    ));
    fs::create_dir_all(&temp_dir)?;

    // Find and extract data.tar.gz
    for entry in archive.entries()? {
        let mut entry = entry?;
        let path = entry.path()?.to_path_buf();

        if path.to_string_lossy() == "data.tar.gz" {
            let data_path = temp_dir.join("data.tar.gz");
            entry.unpack(&data_path)?;

            // Extract only specified directories from the gem contents
            let data_file = fs::File::open(&data_path)?;
            let data_decoder = GzDecoder::new(data_file);
            let mut data_archive = Archive::new(data_decoder);

            let mut extracted_count = 0;

            for entry in data_archive.entries()? {
                let mut entry = entry?;
                let entry_path = entry.path()?.to_path_buf();
                let entry_str = entry_path.to_string_lossy();

                // Check if this entry matches any of the directory prefixes
                let matches = directory_prefixes
                    .iter()
                    .any(|prefix| entry_str.starts_with(prefix));

                if matches {
                    let dest_path = dest_dir.join(&entry_path);

                    // Create parent directory if needed
                    if let Some(parent) = dest_path.parent() {
                        fs::create_dir_all(parent)?;
                    }

                    // Extract the file
                    entry.unpack(&dest_path)?;
                    extracted_count += 1;

                    // Handle executable files
                    #[cfg(unix)]
                    if (entry_str.starts_with("exe/") || entry_str.starts_with("bin/"))
                        && dest_path.is_file()
                    {
                        use std::os::unix::fs::PermissionsExt;

                        // Rewrite shebang if requested
                        if options.env_shebang {
                            rewrite_shebang(&dest_path)?;
                        }

                        // Make executable
                        let mut perms = fs::metadata(&dest_path)?.permissions();
                        perms.set_mode(0o755); // rwxr-xr-x
                        fs::set_permissions(&dest_path, perms)?;

                        // Copy to bindir if specified
                        if let Some(ref bindir) = options.bindir {
                            fs::create_dir_all(bindir)?;
                            let file_name = dest_path.file_name().unwrap();
                            let bindir_path = bindir.join(file_name);
                            fs::copy(&dest_path, &bindir_path)?;
                            let mut bindir_perms = fs::metadata(&bindir_path)?.permissions();
                            bindir_perms.set_mode(0o755);
                            fs::set_permissions(&bindir_path, bindir_perms)?;
                        }
                    }
                }
            }

            // Cleanup temp directory
            drop(fs::remove_dir_all(&temp_dir));

            if options.verbose {
                println!("    Extracted {extracted_count} files");
            }

            return Ok(());
        }
    }

    anyhow::bail!("Invalid gem file: data.tar.gz not found")
}

/// Rewrite shebang line to use /usr/bin/env
fn rewrite_shebang(file_path: &Path) -> Result<()> {
    let content = fs::read_to_string(file_path)
        .with_context(|| format!("Failed to read file: {}", file_path.display()))?;

    // Check if file starts with a shebang
    if !content.starts_with("#!") {
        return Ok(());
    }

    // Find the end of the first line
    let first_line_end = content.find('\n').unwrap_or(content.len());
    let shebang_line = &content[..first_line_end];

    // If it's already using /usr/bin/env, no need to rewrite
    if shebang_line.contains("/usr/bin/env") {
        return Ok(());
    }

    // Extract the interpreter (e.g., "ruby" from "#!/usr/bin/ruby")
    let interpreter = if let Some(pos) = shebang_line.rfind('/') {
        &shebang_line[pos + 1..]
    } else {
        // Malformed shebang, skip rewriting
        return Ok(());
    };

    // Construct new shebang with /usr/bin/env
    let new_content = format!(
        "#!/usr/bin/env {}{}",
        interpreter,
        &content[first_line_end..]
    );

    fs::write(file_path, new_content)
        .with_context(|| format!("Failed to write file: {}", file_path.display()))?;

    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "Tests can panic")]
mod tests {
    use super::*;

    /// Helper to create minimal `PristineOptions`
    fn minimal_pristine_options() -> PristineOptions {
        PristineOptions::default()
    }

    #[test]
    fn pristine_options_default() {
        let options = PristineOptions::default();
        assert!(options.extensions); // Should default to true
        assert!(!options.all);
        assert!(options.gems.is_empty());
    }

    #[test]
    fn test_pristine_options_all_flag() {
        let mut opts = minimal_pristine_options();
        assert!(!opts.all);
        opts.all = true;
        assert!(opts.all);
    }

    #[test]
    fn test_pristine_options_skip_list() {
        let mut opts = minimal_pristine_options();
        assert!(opts.skip.is_empty());
        opts.skip = vec!["rails".to_string(), "bundler".to_string()];
        assert_eq!(opts.skip.len(), 2);
        assert!(opts.skip.contains(&"rails".to_string()));
    }

    #[test]
    fn test_pristine_options_extensions_flag() {
        let mut opts = minimal_pristine_options();
        assert!(opts.extensions);
        opts.extensions = false;
        assert!(!opts.extensions);
    }

    #[test]
    fn test_pristine_options_only_missing_extensions() {
        let mut opts = minimal_pristine_options();
        assert!(!opts.only_missing_extensions);
        opts.only_missing_extensions = true;
        assert!(opts.only_missing_extensions);
    }

    #[test]
    fn test_pristine_options_only_executables() {
        let mut opts = minimal_pristine_options();
        assert!(!opts.only_executables);
        opts.only_executables = true;
        assert!(opts.only_executables);
    }

    #[test]
    fn test_pristine_options_only_plugins() {
        let mut opts = minimal_pristine_options();
        assert!(!opts.only_plugins);
        opts.only_plugins = true;
        assert!(opts.only_plugins);
    }

    #[test]
    fn test_pristine_options_env_shebang_flag() {
        let mut opts = minimal_pristine_options();
        assert!(!opts.env_shebang);
        opts.env_shebang = true;
        assert!(opts.env_shebang);
    }

    #[test]
    fn test_pristine_options_install_dir() {
        let mut opts = minimal_pristine_options();
        assert_eq!(opts.install_dir, None);
        opts.install_dir = Some(PathBuf::from("/custom/gems"));
        assert_eq!(opts.install_dir, Some(PathBuf::from("/custom/gems")));
    }

    #[test]
    fn test_pristine_options_bindir() {
        let mut opts = minimal_pristine_options();
        assert_eq!(opts.bindir, None);
        opts.bindir = Some(PathBuf::from("/usr/local/bin"));
        assert_eq!(opts.bindir, Some(PathBuf::from("/usr/local/bin")));
    }

    #[test]
    fn test_pristine_options_version_specification() {
        let mut opts = minimal_pristine_options();
        assert_eq!(opts.version, None);
        opts.version = Some("2.0.0".to_string());
        assert_eq!(opts.version, Some("2.0.0".to_string()));
    }

    #[test]
    fn test_pristine_options_verbose_flag() {
        let mut opts = minimal_pristine_options();
        assert!(!opts.verbose);
        opts.verbose = true;
        assert!(opts.verbose);
    }

    #[test]
    fn test_pristine_options_quiet_flag() {
        let mut opts = minimal_pristine_options();
        assert!(!opts.quiet);
        opts.quiet = true;
        assert!(opts.quiet);
    }

    #[test]
    fn test_pristine_options_gem_names() {
        let mut opts = minimal_pristine_options();
        assert!(opts.gems.is_empty());
        opts.gems = vec!["rails".to_string(), "devise".to_string()];
        assert_eq!(opts.gems.len(), 2);
        assert!(opts.gems.contains(&"rails".to_string()));
        assert!(opts.gems.contains(&"devise".to_string()));
    }

    #[test]
    fn test_pristine_options_complex_scenario() {
        // Test realistic combination: restore specific gems with extensions and verbose
        let mut opts = minimal_pristine_options();
        opts.gems = vec!["rails".to_string(), "devise".to_string()];
        opts.extensions = true;
        opts.verbose = true;
        opts.env_shebang = true;
        opts.version = Some("7.0.0".to_string());

        assert_eq!(opts.gems.len(), 2);
        assert!(opts.extensions);
        assert!(opts.verbose);
        assert!(opts.env_shebang);
        assert_eq!(opts.version, Some("7.0.0".to_string()));
    }
}
