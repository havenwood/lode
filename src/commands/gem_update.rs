//! Update command
//!
//! Update installed gems to their latest versions

use anyhow::{Context, Result};
use lode::gem_store::GemStore;
use lode::trust_policy::TrustPolicy;
use lode::{Config, DownloadManager, ExtensionBuilder, GemSpec, RubyGemsClient, config};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

/// Options for gem update command
#[derive(Debug)]
pub(crate) struct UpdateOptions {
    pub gems: Vec<String>,
    pub system: bool,
    pub platform: Option<String>,
    pub prerelease: bool,
    pub bindir: Option<String>,
    pub build_root: Option<String>,
    pub install_dir: Option<String>,
    pub document: Option<String>,
    pub no_document: bool,
    pub vendor: bool,
    pub env_shebang: bool,
    pub force: bool,
    pub wrappers: bool,
    pub trust_policy: Option<String>,
    pub ignore_dependencies: bool,
    pub format_executable: bool,
    pub user_install: bool,
    pub development: bool,
    pub development_all: bool,
    pub conservative: bool,
    pub minimal_deps: bool,
    pub post_install_message: bool,
    pub file: Option<String>,
    pub without: Option<String>,
    pub explain: bool,
    pub lock: bool,
    pub suggestions: bool,
    pub target_rbconfig: Option<String>,
    pub default: bool,
    pub local: bool,
    pub remote: bool,
    pub both: bool,
    pub bulk_threshold: Option<usize>,
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

/// Update installed gems to latest versions
#[allow(clippy::cognitive_complexity)]
pub(crate) async fn run(options: UpdateOptions) -> Result<()> {
    // Debug output
    if options.debug {
        eprintln!("DEBUG: Starting gem update");
        eprintln!("DEBUG: Options: {options:?}");
    }

    // Load config with custom options
    let _config = Config::load_with_options(options.config_file.as_deref(), options.norc)?;

    // Emit deprecation warning for --default flag
    if options.default {
        eprintln!(
            "WARNING: The --default flag is deprecated and will be removed in a future version"
        );
    }

    // Handle --clear-sources flag
    if options.clear_sources {
        // --clear-sources in gem update silently clears sources and continues
        // No special output is printed, the update continues normally
        if options.debug {
            eprintln!("DEBUG: --clear-sources flag set (sources cleared)");
        }
    }

    // Handle --system flag to update RubyGems itself
    if options.system {
        if !options.quiet && !options.silent {
            println!(
                "Updating RubyGems is not supported by lode (RubyGems is a Ruby-specific tool)"
            );
        }
        return Ok(());
    }

    // Handle --without flag (exclude gem groups)
    if let Some(ref without_groups) = options.without {
        if options.debug {
            eprintln!("DEBUG: --without flag set to: {without_groups}");
        }
        // Note: Gem groups are primarily a Bundler/Gemfile concept
        // For gem update with --file, we'll filter groups from the Gemfile
        if options.file.is_none() && options.verbose {
            println!("Note: --without only applies when used with --file");
        }
    }

    // Handle --file flag (read gems from Gemfile)
    let mut gems_from_file = Vec::new();
    if let Some(gemfile_path) = &options.file {
        let gemfile_content = fs::read_to_string(gemfile_path)
            .context(format!("Failed to read Gemfile: {gemfile_path}"))?;

        if options.debug {
            eprintln!("DEBUG: Reading gems from: {gemfile_path}");
        }

        // Parse gem names from Gemfile (simple regex extraction)
        // Matches lines like: gem 'name' or gem "name" or gem 'name', '~> 1.0'
        let gem_regex = regex::Regex::new(r#"^\s*gem\s+['"]([^'"]+)['"]"#)
            .context("Failed to compile gem regex")?;

        // Parse group directive for --without filtering
        let group_regex =
            regex::Regex::new(r"^\s*group\s+:(\w+)").context("Failed to compile group regex")?;

        let excluded_groups: HashSet<String> =
            options
                .without
                .as_ref()
                .map_or_else(HashSet::new, |without_str| {
                    without_str
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .collect()
                });

        let mut in_excluded_group = false;

        for line in gemfile_content.lines() {
            // Check for group declarations
            if let Some(group_cap) = group_regex.captures(line) {
                if let Some(group_name) = group_cap.get(1) {
                    in_excluded_group = excluded_groups.contains(group_name.as_str());
                    if options.debug && in_excluded_group {
                        eprintln!("DEBUG: Skipping group: {}", group_name.as_str());
                    }
                }
            } else if line.trim() == "end" {
                // End of group block
                in_excluded_group = false;
            } else if !in_excluded_group {
                // Parse gem declarations
                if let Some(captures) = gem_regex.captures(line)
                    && let Some(gem_name) = captures.get(1)
                {
                    gems_from_file.push(gem_name.as_str().to_string());
                }
            }
        }

        if gems_from_file.is_empty() {
            anyhow::bail!("No gems found in Gemfile: {gemfile_path}");
        }

        if options.verbose {
            println!("Read {} gems from {}", gems_from_file.len(), gemfile_path);
        }
    }

    // Show dry-run mode notice
    if options.explain && !options.quiet && !options.silent {
        println!("Dry run: gems would be updated as follows:\n");
    }

    // Determine installation directory based on options
    let install_dir = determine_install_dir(&options)?;

    if options.debug {
        eprintln!("DEBUG: Install directory: {}", install_dir.display());
    }

    let store = GemStore::new()?;
    let installed_gems = store.list_gems()?;

    if installed_gems.is_empty() {
        if !options.quiet && !options.silent {
            println!("No gems installed");
        }
        return Ok(());
    }

    // Determine which gems to update
    let gems_to_update: Vec<String> = if !gems_from_file.is_empty() {
        // Use gems from Gemfile (--file flag)
        gems_from_file
    } else if options.gems.is_empty() {
        // Update all gems
        let mut gem_names: Vec<_> = installed_gems.iter().map(|g| g.name.clone()).collect();
        gem_names.sort_unstable();
        gem_names.dedup();
        gem_names
    } else {
        // Update only specified gems
        options.gems.clone()
    };

    if !options.quiet && !options.silent && !options.explain {
        println!("Checking for gem updates...");
        println!();
    }

    let cache_dir = config::cache_dir(None).context("Failed to get cache directory")?;
    let dm = DownloadManager::new(cache_dir)?;

    // Determine search scope based on --local/--remote/--both
    let search_local = options.local && !options.remote && !options.both;
    let search_remote = options.remote || options.both || !options.local;

    // Create custom RubyGems client with optional source and proxy
    let base_url = options
        .source
        .clone()
        .unwrap_or_else(|| lode::RUBYGEMS_ORG_URL.to_string());
    let client = RubyGemsClient::new_with_proxy(&base_url, options.http_proxy.as_deref())?;

    // Determine bulk API threshold (default: 1000 gems)
    let bulk_threshold = options.bulk_threshold.unwrap_or(1000);
    let use_bulk_api = gems_to_update.len() >= bulk_threshold;

    if options.debug {
        eprintln!(
            "DEBUG: Bulk threshold: {bulk_threshold}, updating {} gems, use bulk API: {use_bulk_api}",
            gems_to_update.len()
        );
    }

    // Handle --minimal-deps flag
    if options.minimal_deps && options.debug {
        eprintln!("DEBUG: --minimal-deps enabled (won't upgrade satisfied dependencies)");
    }

    // Parse trust policy
    let trust_policy = if let Some(policy) = &options.trust_policy {
        TrustPolicy::parse(policy).context("Invalid trust policy")?
    } else {
        TrustPolicy::NoSecurity
    };

    let mut updated_count = 0;
    let mut would_update_count = 0;
    let mut skipped_count = 0;
    let mut updated_gems: Vec<GemSpec> = Vec::with_capacity(gems_to_update.len());

    for gem_name in gems_to_update {
        // Skip remote lookup if --local-only mode
        if search_local && !search_remote {
            if !options.quiet && !options.silent {
                eprintln!("Skipping {gem_name}: local-only mode (cached versions not available)");
            }
            skipped_count += 1;
            continue;
        }

        // Fetch latest version from RubyGems
        match client.fetch_versions(&gem_name).await {
            Ok(mut versions) => {
                // Filter by prerelease if not requested
                if !options.prerelease {
                    versions.retain(|v| !v.number.contains('-'));
                }

                // Filter by platform if specified
                if let Some(ref platform) = options.platform {
                    versions.retain(|v| v.platform == *platform);
                }

                if let Some(latest_version) = versions.first() {
                    // Check if any installed version is older than latest
                    let installed = store.find_gem_by_name(&gem_name)?;
                    if let Some(latest_installed) = installed.last() {
                        // In conservative mode, skip if already installed
                        if options.conservative {
                            if options.verbose && !options.explain {
                                println!(
                                    "{gem_name} ({}) already satisfies requirements, skipping (conservative mode)",
                                    latest_installed.version
                                );
                            }
                        } else if latest_installed.version < latest_version.number {
                            // Check dependencies unless --ignore-dependencies is set
                            if !options.ignore_dependencies
                                && !latest_version.dependencies.runtime.is_empty()
                                && options.verbose
                                && !options.explain
                            {
                                println!(
                                    "Note: {} has {} runtime dependencies",
                                    gem_name,
                                    latest_version.dependencies.runtime.len()
                                );
                            }

                            if options.explain {
                                if !options.quiet && !options.silent {
                                    println!(
                                        "  {} ({} -> {})",
                                        gem_name, latest_installed.version, latest_version.number
                                    );
                                }
                                would_update_count += 1;
                            } else {
                                if !options.quiet && !options.silent {
                                    println!(
                                        "Updating {} from {} to {}...",
                                        gem_name, latest_installed.version, latest_version.number
                                    );
                                }

                                // Create gem spec for installation
                                let spec = GemSpec::new(
                                    gem_name.clone(),
                                    latest_version.number.clone(),
                                    latest_version.platform.clone().into(),
                                    vec![],
                                    vec![],
                                );

                                // Download the gem
                                match dm.download_gem(&spec).await {
                                    Ok(gem_path) => {
                                        // Extract to determined directory
                                        let gem_dir = install_dir.join(format!(
                                            "{}-{}",
                                            gem_name, latest_version.number
                                        ));

                                        if let Err(e) = extract_gem(&gem_path, &gem_dir) {
                                            if !options.silent {
                                                eprintln!("Failed to extract {gem_name}: {e}");
                                            }
                                            if options.backtrace {
                                                eprintln!("  Details: {e:#}");
                                            }
                                            skipped_count += 1;
                                        } else {
                                            // Verify signature if trust policy enabled
                                            if trust_policy != TrustPolicy::NoSecurity
                                                && verify_gem_signature(&gem_path, trust_policy)
                                                    .is_err()
                                            {
                                                if !options.silent {
                                                    eprintln!("Failed to verify {gem_name}");
                                                }
                                                skipped_count += 1;
                                                continue;
                                            }

                                            // Build extensions if present
                                            if has_extensions(&gem_dir) && !options.force {
                                                if options.verbose {
                                                    println!(
                                                        "Building native extensions for {gem_name}..."
                                                    );
                                                }
                                                build_extensions(&gem_dir, &options)?;
                                            }

                                            // Install executables if bindir specified
                                            if let Some(bindir) = &options.bindir {
                                                match install_executables(
                                                    &gem_dir, bindir, &options,
                                                ) {
                                                    Err(e) if options.verbose => {
                                                        eprintln!(
                                                            "Warning: Failed to install executables: {e}"
                                                        );
                                                    }
                                                    _ => {}
                                                }
                                            }

                                            // Generate documentation
                                            generate_documentation(&gem_dir, &spec, &options)?;

                                            // Install development dependencies if requested
                                            if (options.development_all
                                                || (options.development
                                                    && options.gems.contains(&gem_name)))
                                                && let Err(e) = install_development_dependencies(
                                                    &gem_name,
                                                    &gem_dir,
                                                    &options,
                                                    &client,
                                                    &dm,
                                                    &install_dir,
                                                )
                                                .await
                                                && options.verbose
                                            {
                                                eprintln!(
                                                    "  Warning: Failed to install development dependencies: {e}"
                                                );
                                            }

                                            // Display post-install message if present
                                            if options.post_install_message
                                                && let Ok(metadata) = client
                                                    .fetch_gem_info(&spec.name, &spec.version)
                                                    .await
                                                && let Some(message) = metadata.post_install_message
                                                && !options.quiet
                                                && !options.silent
                                            {
                                                println!(
                                                    "\nPost-install message from {}:",
                                                    spec.name
                                                );
                                                println!("{message}");
                                            }

                                            if !options.quiet && !options.silent {
                                                println!(
                                                    "Successfully updated {} to {}",
                                                    gem_name, latest_version.number
                                                );
                                            }
                                            updated_count += 1;
                                            updated_gems.push(spec.clone());
                                        }
                                    }
                                    Err(e) => {
                                        if !options.silent {
                                            eprintln!("Failed to download {gem_name}: {e}");
                                        }
                                        if options.backtrace {
                                            eprintln!("  Details: {e:#}");
                                        }
                                        skipped_count += 1;
                                    }
                                }
                            }
                        } else if options.verbose && !options.explain {
                            println!(
                                "{name} is already up to date ({version})",
                                name = gem_name,
                                version = latest_installed.version
                            );
                        }
                    }
                } else if !options.silent {
                    eprintln!("No suitable version found for {gem_name}");
                    // Show suggestions if requested
                    if options.suggestions && !options.explain {
                        eprintln!(
                            "  Suggestion: Try searching with 'gem search {gem_name}' to find available gems"
                        );
                    }
                    skipped_count += 1;
                }
            }
            Err(e) => {
                if !options.silent {
                    eprintln!("Failed to check {gem_name} for updates: {e}");
                }
                if options.backtrace {
                    eprintln!("  Details: {e:#}");
                }
                skipped_count += 1;
            }
        }
    }

    if !options.quiet && !options.silent {
        if options.explain {
            println!("\nWould update {would_update_count} gem(s)");
        } else {
            println!("\nUpdated {updated_count} gem(s), skipped {skipped_count}");
        }
    }

    // Create lock file if requested and gems were updated
    if options.lock && !updated_gems.is_empty() {
        create_lock_file(&updated_gems, &options)?;
    }

    Ok(())
}

/// Create a lock file with updated gems
fn create_lock_file(updated: &[GemSpec], options: &UpdateOptions) -> Result<()> {
    use std::fmt::Write;

    let lock_file_path = "gem.lock";

    if options.debug {
        eprintln!("DEBUG: Creating lock file: {lock_file_path}");
    }

    let mut lock_content = String::new();
    lock_content.push_str("# gem.lock - Generated by lode gem-update\n");
    lock_content.push_str("# DO NOT EDIT - This file is auto-generated\n\n");

    lock_content.push_str("GEMS:\n");
    for spec in updated {
        writeln!(lock_content, "  {} ({})", spec.name, spec.version)
            .expect("Writing to string should not fail");
        writeln!(
            lock_content,
            "    platform: {}",
            spec.platform.as_deref().unwrap_or("ruby")
        )
        .expect("Writing to string should not fail");
    }

    lock_content.push_str("\nUPDATED WITH:\n");
    writeln!(lock_content, "   lode {}", env!("CARGO_PKG_VERSION"))
        .expect("Writing to string should not fail");

    fs::write(lock_file_path, lock_content).context("Failed to write lock file")?;

    if options.verbose {
        println!("Created lock file: {lock_file_path}");
    }

    Ok(())
}

/// Extract a gem file to the installation directory
///
/// Gem files are tar.gz archives containing:
/// - metadata.gz (gemspec and other metadata)
/// - data.tar.gz (the actual gem contents: lib/, bin/, etc.)
/// - checksums.yaml.gz (optional checksums)
fn extract_gem(gem_path: &std::path::PathBuf, install_dir: &std::path::PathBuf) -> Result<()> {
    use flate2::read::GzDecoder;
    use tar::Archive;

    fs::create_dir_all(install_dir).context("Failed to create gem directory")?;

    // Step 1: Extract the outer tar (not tar.gz - gems are plain tar) to a temp directory
    let temp_dir = tempfile::TempDir::new().context("Failed to create temporary directory")?;

    let file = std::fs::File::open(gem_path)
        .context(format!("Failed to open gem file: {}", gem_path.display()))?;

    // Gem files are plain tar archives (not tar.gz)
    let mut archive = Archive::new(file);
    archive
        .unpack(temp_dir.path())
        .context("Failed to extract gem archive to temp directory")?;

    // Step 2: Read data.tar.gz from temp directory
    let data_tar_gz_path = temp_dir.path().join("data.tar.gz");
    let data_file = std::fs::File::open(&data_tar_gz_path).context("Failed to open data.tar.gz")?;

    // Step 3: Extract data.tar.gz contents to install directory
    let data_gz = GzDecoder::new(data_file);
    let mut data_archive = Archive::new(data_gz);
    data_archive
        .unpack(install_dir)
        .context("Failed to extract gem contents from data.tar.gz")?;

    Ok(())
}

/// Parse documentation types from --document flag
fn parse_doc_types(doc_format: Option<&str>, verbose: bool) -> HashSet<&'static str> {
    let mut types = HashSet::new();

    if let Some(formats) = doc_format {
        for format in formats.split(',') {
            match format.trim() {
                "rdoc" => {
                    types.insert("rdoc");
                }
                "ri" => {
                    types.insert("ri");
                }
                _ => {
                    if verbose {
                        println!("  Unknown documentation format: {format}");
                    }
                }
            }
        }
    } else {
        // Default: generate both rdoc and ri if --document is not specified
        types.insert("rdoc");
        types.insert("ri");
    }

    types
}

/// Check if gem has native extensions
fn has_extensions(gem_dir: &Path) -> bool {
    let ext_dir = gem_dir.join("ext");
    ext_dir.exists() && ext_dir.is_dir()
}

/// Build native extensions for a gem
fn build_extensions(gem_dir: &Path, options: &UpdateOptions) -> Result<()> {
    let gem_name = gem_dir
        .file_name()
        .and_then(|n| n.to_str())
        .context("Invalid gem directory name")?;

    let mut builder = ExtensionBuilder::new(
        false, // skip_extensions
        options.verbose,
        options.target_rbconfig.clone(),
    );

    let platform = options.platform.as_deref();
    if let Some(result) = builder.build_if_needed(gem_name, gem_dir, platform)
        && !result.success
    {
        anyhow::bail!("Failed to build native extensions: {}", result.output);
    }

    Ok(())
}

/// Install gem executables to bin directory
fn install_executables(gem_dir: &Path, bindir: &str, options: &UpdateOptions) -> Result<()> {
    let bin_src = gem_dir.join("bin");
    if !bin_src.exists() {
        return Ok(());
    }

    let bin_dest = PathBuf::from(bindir);
    fs::create_dir_all(&bin_dest).context("Failed to create bin directory")?;

    for entry in fs::read_dir(&bin_src).context("Failed to read bin directory")? {
        let entry = entry?;
        let file_name = entry.file_name();
        let src_path = entry.path();

        // Apply format_executable if requested (adds gem name as suffix)
        let dest_filename = if options.format_executable {
            let gem_name_version = gem_dir
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown");

            let base_name = file_name.to_str().unwrap_or("unknown");
            format!("{base_name}-{gem_name_version}")
        } else {
            file_name.to_string_lossy().to_string()
        };

        let dest_path = bin_dest.join(&dest_filename);

        if options.wrappers {
            create_wrapper_script(&src_path, &dest_path, gem_dir, options)?;
        } else {
            fs::copy(&src_path, &dest_path).context("Failed to copy executable")?;
        }

        // Make executable on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&dest_path)?.permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&dest_path, perms)?;
        }

        if options.verbose {
            println!("  Installed executable: {dest_filename}");
        }
    }

    Ok(())
}

/// Create wrapper script for gem executable
fn create_wrapper_script(
    src_path: &Path,
    dest_path: &Path,
    gem_dir: &Path,
    options: &UpdateOptions,
) -> Result<()> {
    let shebang = if options.env_shebang {
        "#!/usr/bin/env ruby"
    } else {
        "#!/usr/bin/ruby"
    };

    let wrapper = format!(
        r"{}

# This file was generated by Lode

require 'rubygems'

gem_dir = '{}'
$LOAD_PATH.unshift File.join(gem_dir, 'lib')

load File.join(gem_dir, 'bin', '{}')
",
        shebang,
        gem_dir.display(),
        src_path.file_name().unwrap().to_string_lossy()
    );

    fs::write(dest_path, wrapper).context("Failed to write wrapper script")?;
    Ok(())
}

/// Generate documentation for a gem using `RDoc`
fn generate_documentation(gem_dir: &Path, spec: &GemSpec, options: &UpdateOptions) -> Result<()> {
    // Skip if --no-document
    if options.no_document {
        return Ok(());
    }

    let lib_dir = gem_dir.join("lib");
    if !lib_dir.exists() {
        if options.verbose {
            println!("  No lib directory found, skipping documentation");
        }
        return Ok(());
    }

    // Determine what documentation types to generate
    let doc_types = parse_doc_types(options.document.as_deref(), options.verbose);

    // If no valid documentation types after parsing, skip
    if doc_types.is_empty() {
        if options.verbose {
            println!("  No valid documentation types specified, skipping documentation");
        }
        return Ok(());
    }

    // Determine documentation output directory (for rdoc HTML output)
    let doc_dir = gem_dir
        .parent()
        .context("Invalid gem directory")?
        .parent()
        .context("Invalid gem directory structure")?
        .join("doc")
        .join(format!("{}-{}", spec.name, spec.version));

    if options.verbose {
        let types_str = if doc_types.contains("rdoc") && doc_types.contains("ri") {
            "rdoc and ri"
        } else if doc_types.contains("rdoc") {
            "rdoc"
        } else {
            "ri"
        };
        println!("  Generating {types_str} documentation...");
    }

    // Create documentation directory if rdoc HTML output is needed
    if doc_types.contains("rdoc") {
        fs::create_dir_all(&doc_dir).context("Failed to create documentation directory")?;
    }

    // Run rdoc to generate documentation
    let mut cmd = std::process::Command::new("rdoc");

    // Add rdoc HTML output flag if requested
    if doc_types.contains("rdoc") {
        cmd.arg("--op").arg(&doc_dir);
    }

    // Add ri database generation flag if requested
    if doc_types.contains("ri") {
        cmd.arg("--ri");
    }

    // Add the source directory to document
    cmd.arg(&lib_dir);

    if options.quiet || options.silent {
        cmd.stdout(std::process::Stdio::null());
        cmd.stderr(std::process::Stdio::null());
    }

    // Execute rdoc
    let output = cmd.output();

    match output {
        Ok(output) => {
            if !output.status.success() {
                if options.verbose {
                    eprintln!(
                        "  Warning: Documentation generation failed (rdoc exit code {})",
                        output.status
                    );
                    if !output.stderr.is_empty() {
                        eprintln!("  rdoc error: {}", String::from_utf8_lossy(&output.stderr));
                    }
                }
                // Don't fail installation if documentation generation fails
                return Ok(());
            }

            if options.verbose {
                println!("  Documentation generated successfully");
            }
        }
        Err(e) => {
            if options.verbose {
                eprintln!(
                    "  Warning: Could not run rdoc ({e}). Skipping documentation generation."
                );
                eprintln!("  Install rdoc with: gem install rdoc");
            }
            // Don't fail installation if rdoc is not available
        }
    }

    Ok(())
}

/// Verify gem signature using trust policy
fn verify_gem_signature(gem_path: &Path, trust_policy: TrustPolicy) -> Result<()> {
    use lode::trust_policy::GemVerifier;

    let verifier = GemVerifier::new(trust_policy)?;
    verifier.verify_gem(gem_path)?;
    Ok(())
}

/// Determine the installation directory based on options
fn determine_install_dir(options: &UpdateOptions) -> Result<PathBuf> {
    if let Some(dir) = &options.install_dir {
        return Ok(PathBuf::from(dir));
    }

    if let Some(build_root) = &options.build_root {
        return Ok(PathBuf::from(build_root));
    }

    if options.vendor {
        return Ok(PathBuf::from("vendor/gems"));
    }

    if options.user_install {
        let home = std::env::var("HOME").context("HOME environment variable not set")?;
        let ruby_version = config::ruby_version(None);
        return Ok(PathBuf::from(home)
            .join(".gem")
            .join("ruby")
            .join(ruby_version)
            .join("gems"));
    }

    // Default: use system gem directory
    let store = GemStore::new()?;
    Ok(store.gem_dir().to_path_buf())
}

/// Parse development dependencies from gemspec file
fn parse_development_dependencies(gem_dir: &Path) -> Result<Vec<String>> {
    let mut dev_deps = Vec::new();

    // Find .gemspec file
    let gemspec_files: Vec<_> = fs::read_dir(gem_dir)?
        .filter_map(std::result::Result::ok)
        .filter(|entry| {
            entry
                .path()
                .extension()
                .and_then(|s| s.to_str())
                .is_some_and(|s| s == "gemspec")
        })
        .collect();

    if gemspec_files.is_empty() {
        return Ok(dev_deps);
    }

    // Read first gemspec file
    let Some(first_gemspec) = gemspec_files.first() else {
        return Ok(dev_deps);
    };
    let gemspec_path = first_gemspec.path();
    let content = fs::read_to_string(&gemspec_path).context(format!(
        "Failed to read gemspec: {}",
        gemspec_path.display()
    ))?;

    // Parse development dependencies using regex
    // Matches: add_development_dependency 'name' or s.add_development_dependency "name"
    let dev_dep_regex = regex::Regex::new(r#"add_development_dependency\s*\(?['"]([^'"]+)['"]"#)
        .context("Failed to compile development dependency regex")?;

    for captures in dev_dep_regex.captures_iter(&content) {
        if let Some(dep_name) = captures.get(1) {
            dev_deps.push(dep_name.as_str().to_string());
        }
    }

    Ok(dev_deps)
}

/// Install development dependencies for a gem
async fn install_development_dependencies(
    gem_name: &str,
    gem_dir: &Path,
    options: &UpdateOptions,
    client: &RubyGemsClient,
    dm: &DownloadManager,
    install_dir: &Path,
) -> Result<()> {
    let dev_deps = parse_development_dependencies(gem_dir)?;

    if dev_deps.is_empty() {
        if options.verbose {
            println!("  No development dependencies for {gem_name}");
        }
        return Ok(());
    }

    if options.verbose {
        println!(
            "  Installing {} development dependencies for {gem_name}...",
            dev_deps.len()
        );
    }

    for dep_name in dev_deps {
        if options.debug {
            eprintln!("DEBUG: Installing development dependency: {dep_name}");
        }

        // Fetch latest version
        match client.fetch_versions(&dep_name).await {
            Ok(mut versions) => {
                if !options.prerelease {
                    versions.retain(|v| !v.number.contains('-'));
                }

                if let Some(latest) = versions.first() {
                    let spec = GemSpec::new(
                        dep_name.clone(),
                        latest.number.clone(),
                        latest.platform.clone().into(),
                        vec![],
                        vec![],
                    );

                    match dm.download_gem(&spec).await {
                        Ok(gem_path) => {
                            let dep_gem_dir =
                                install_dir.join(format!("{}-{}", dep_name, latest.number));

                            if let Err(e) = extract_gem(&gem_path, &dep_gem_dir) {
                                if options.verbose {
                                    eprintln!("  Warning: Failed to install {dep_name}: {e}");
                                }
                            } else if options.verbose {
                                println!("  Installed development dependency: {dep_name}");
                            }
                        }
                        Err(e) => {
                            if options.verbose {
                                eprintln!("  Warning: Failed to download {dep_name}: {e}");
                            }
                        }
                    }
                }
            }
            Err(e) => {
                if options.verbose {
                    eprintln!("  Warning: Failed to fetch {dep_name}: {e}");
                }
            }
        }
    }

    Ok(())
}
