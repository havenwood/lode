//! Install command
//!
//! Install gems into the system

use anyhow::{Context, Result};
use futures_util::future::BoxFuture;
use lode::gem_store::GemStore;
use lode::trust_policy::TrustPolicy;
use lode::{DownloadManager, ExtensionBuilder, GemSpec, Resolver, RubyGemsClient, config};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

/// Options for gem installation
#[derive(Debug, Clone, Default)]
pub(crate) struct InstallOptions {
    pub gems: Vec<String>,
    // Basic Options
    pub platform: Option<String>,
    pub version: Option<String>,
    pub prerelease: bool,
    /// Note: `update_sources` is deprecated in gem 4.0.0 but still parsed for compatibility
    pub update_sources: bool,
    // Install/Update Options
    pub install_dir: Option<String>,
    pub bindir: Option<String>,
    pub document: Option<String>,
    pub no_document: bool,
    pub build_root: Option<String>,
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
    // Local/Remote Options
    pub local: bool,
    pub remote: bool,
    pub both: bool,
    pub bulk_threshold: Option<usize>,
    pub clear_sources: bool,
    pub source: Option<String>,
    pub http_proxy: Option<String>,
    // Common Options
    pub verbose: bool,
    pub quiet: bool,
    pub silent: bool,
    pub config_file: Option<String>,
    pub backtrace: bool,
    pub debug: bool,
    pub norc: bool,
}

/// Install gems with all specified options
#[allow(clippy::cognitive_complexity)] // Complex by nature - handles many options
pub(crate) async fn run(mut options: InstallOptions) -> Result<()> {
    // Debug output
    if options.debug {
        eprintln!("DEBUG: Starting gem installation");
        eprintln!("DEBUG: Options: {options:?}");
    }

    // Handle --norc flag (skip loading .gemrc)
    if options.norc && options.debug {
        eprintln!("DEBUG: Skipping .gemrc configuration file");
    }

    // Handle --config-file flag (use custom config)
    if let Some(ref config_file) = options.config_file
        && options.debug
    {
        eprintln!("DEBUG: Using custom config file: {config_file}");
    }
    // Note: Config file loading not yet implemented in lode
    // This is a placeholder for future config system integration

    // Emit deprecation warning for --update-sources flag
    if options.update_sources {
        eprintln!(
            "WARNING: The --update-sources flag is deprecated and will be removed in a future version"
        );
    }

    // Handle --without flag (exclude gem groups)
    if let Some(ref without_groups) = options.without {
        if options.debug {
            eprintln!("DEBUG: --without flag set to: {without_groups}");
        }
        // Note: Gem groups are a Bundler concept and don't apply to single gem installation
        // This flag is accepted for compatibility but has no effect for gem install
        if options.verbose {
            println!("Note: --without is a Bundler flag and doesn't apply to gem install");
        }
    }

    // Handle --clear-sources flag
    if options.clear_sources {
        if options.debug {
            eprintln!("DEBUG: --clear-sources flag set");
        }
        // Note: lode uses RubyGemsClient which doesn't have persistent sources
        // This flag is accepted for compatibility but has no effect
        if options.verbose {
            println!("Note: Source clearing not applicable (lode doesn't persist sources)");
        }
    }

    // Handle --bulk-threshold flag
    if let Some(threshold) = options.bulk_threshold
        && options.debug
    {
        eprintln!("DEBUG: Bulk threshold set to: {threshold}");
    }
    // Note: This would need to be passed to RubyGemsClient for bulk API operations
    // Currently not implemented

    // Handle --file flag (read gems from Gemfile)
    if let Some(gemfile_path) = &options.file {
        let gemfile_content = std::fs::read_to_string(gemfile_path)
            .context(format!("Failed to read Gemfile: {gemfile_path}"))?;

        // Parse gem names from Gemfile (simple regex extraction)
        // Matches lines like: gem 'name' or gem "name" or gem 'name', '~> 1.0'
        let gem_regex = regex::Regex::new(r#"^\s*gem\s+['"]([^'"]+)['"]"#)
            .context("Failed to compile gem regex")?;

        for line in gemfile_content.lines() {
            if let Some(captures) = gem_regex.captures(line)
                && let Some(gem_name) = captures.get(1)
            {
                options.gems.push(gem_name.as_str().to_string());
            }
        }

        if options.gems.is_empty() {
            anyhow::bail!("No gems found in Gemfile: {gemfile_path}");
        }

        if options.verbose {
            println!("Read {} gems from {}", options.gems.len(), gemfile_path);
        }
    }

    // Handle --explain flag (dry run)
    if options.explain {
        return explain_install(&options).await;
    }

    // Determine install directory
    let install_dir = determine_install_dir(&options)?;

    if options.debug {
        eprintln!("DEBUG: Install directory: {}", install_dir.display());
        eprintln!("DEBUG: Installing {} gems", options.gems.len());
    }

    // Output verbosity
    if !options.quiet && !options.silent {
        println!("Installing {} gem(s)...", options.gems.len());
    }

    // Parse trust policy
    let trust_policy = if let Some(policy) = &options.trust_policy {
        TrustPolicy::parse(policy).context("Invalid trust policy")?
    } else {
        TrustPolicy::NoSecurity
    };

    // Initialize RubyGems client
    let source_url = options.source.as_deref().unwrap_or(lode::RUBYGEMS_ORG_URL);
    let client = RubyGemsClient::new_with_proxy(source_url, options.http_proxy.as_deref())?;

    // Process each gem (with dependency resolution)
    let mut installed = Vec::new();
    let mut installed_names = HashSet::new();

    for gem_name in &options.gems {
        let result = install_gem_with_dependencies(
            gem_name,
            None, // No specific version constraint
            &options,
            &client,
            &install_dir,
            &trust_policy,
            &mut installed_names,
        )
        .await;

        match result {
            Ok(specs) => {
                installed.extend(specs);
            }
            Err(e) => {
                if options.backtrace {
                    eprintln!("Error installing {gem_name}: {e:#}");
                } else {
                    eprintln!("Error installing {gem_name}: {e}");
                }
                if !options.force {
                    return Err(e);
                }
            }
        }
    }

    // Output summary
    if !options.quiet && !options.silent {
        println!("\nSuccessfully installed {} gem(s)", installed.len());
        for spec in &installed {
            println!("  - {} ({})", spec.name, spec.version);
        }
    }

    // Create lock file if requested
    if options.lock {
        create_lock_file(&installed, &options)?;
    }

    Ok(())
}

/// Install a single gem with dependency resolution
fn install_gem_with_dependencies<'a>(
    gem_name: &'a str,
    version_requirement: Option<&'a str>,
    options: &'a InstallOptions,
    client: &'a RubyGemsClient,
    install_dir: &'a Path,
    trust_policy: &'a TrustPolicy,
    installed: &'a mut HashSet<String>,
) -> BoxFuture<'a, Result<Vec<GemSpec>>> {
    Box::pin(async move {
        let mut specs = Vec::new();

        // Install the requested gem first
        let spec = install_single_gem(
            gem_name,
            version_requirement,
            options,
            client,
            install_dir,
            trust_policy,
        )
        .await?;

        let gem_key = format!("{}-{}", spec.name, spec.version);

        // Skip if already installed
        if installed.contains(&gem_key) {
            return Ok(specs);
        }

        installed.insert(gem_key);
        specs.push(spec.clone());

        // Fetch gem metadata for post-install messages and dependencies
        let metadata = match client.fetch_gem_info(&spec.name, &spec.version).await {
            Ok(meta) => meta,
            Err(e) => {
                if options.verbose {
                    eprintln!(
                        "Warning: Could not fetch metadata for {} ({}): {}",
                        spec.name, spec.version, e
                    );
                    eprintln!("  Skipping dependency installation and post-install message");
                }
                return Ok(specs);
            }
        };

        // Display post-install message if present (unless disabled via flag)
        if let Some(ref message) = metadata.post_install_message
            && options.post_install_message  // Respect the flag
            && !options.quiet
            && !options.silent
        {
            println!("\nPost-install message from {}:", spec.name);
            println!("{message}");
        }

        // Install dependencies unless --ignore-dependencies is set
        if !options.ignore_dependencies {
            if options.debug {
                eprintln!(
                    "DEBUG: Installing {} runtime dependencies for {}",
                    metadata.dependencies.runtime.len(),
                    spec.name
                );
            }

            // Install runtime dependencies
            for dep in &metadata.dependencies.runtime {
                if options.verbose {
                    println!("  Installing dependency: {} {}", dep.name, dep.requirements);
                }

                let dep_specs = install_gem_with_dependencies(
                    &dep.name,
                    Some(&dep.requirements),
                    options,
                    client,
                    install_dir,
                    trust_policy,
                    installed,
                )
                .await?;

                specs.extend(dep_specs);
            }

            // Install development dependencies if requested (unless minimal_deps is set)
            if (options.development || options.development_all) && !options.minimal_deps {
                for dep in &metadata.dependencies.development {
                    if options.verbose {
                        println!(
                            "  Installing development dependency: {} {}",
                            dep.name, dep.requirements
                        );
                    }

                    let dep_specs = install_gem_with_dependencies(
                        &dep.name,
                        Some(&dep.requirements),
                        options,
                        client,
                        install_dir,
                        trust_policy,
                        installed,
                    )
                    .await?;

                    specs.extend(dep_specs);
                }
            }
        }

        Ok(specs)
    })
}

/// Install a single gem without dependencies
async fn install_single_gem(
    gem_name: &str,
    version_requirement: Option<&str>,
    options: &InstallOptions,
    client: &RubyGemsClient,
    install_dir: &Path,
    trust_policy: &TrustPolicy,
) -> Result<GemSpec> {
    // 1. Fetch gem versions from RubyGems
    let versions = client
        .fetch_versions(gem_name)
        .await
        .context(format!("Failed to fetch versions for gem '{gem_name}'"))?;

    if versions.is_empty() {
        if options.suggestions {
            // Try to find similar gem names by searching
            if options.debug {
                eprintln!("DEBUG: Searching for gems similar to '{gem_name}'");
            }

            // Search for gems with similar names (this will use RubyGems search API)
            // For simplicity, we'll suggest using gem-search instead of implementing full search here
            eprintln!("Gem '{gem_name}' not found.");
            eprintln!("Suggestions:");
            eprintln!("  - Check spelling and try again");
            eprintln!("  - Search for similar gems: lode gem-search {gem_name}");
            eprintln!("  - Browse gems at: https://rubygems.org/search?query={gem_name}");

            anyhow::bail!("Gem '{gem_name}' not found on RubyGems.org");
        }
        anyhow::bail!("Gem '{gem_name}' not found on RubyGems.org");
    }

    // 2. Select version based on requirements
    let selected_version =
        select_gem_version(gem_name, version_requirement, &versions, options, client)?;

    if options.verbose {
        println!("Selected {} version {}", gem_name, selected_version.number);
    }

    // 4. Create gem spec
    let spec = GemSpec::new(
        gem_name.to_string(),
        selected_version.number.clone(),
        Some(selected_version.platform.clone()),
        vec![],
        vec![],
    );

    // 5. Check if already installed (for --conservative)
    if options.conservative && !options.force && is_gem_installed(&spec, install_dir) {
        if options.verbose {
            println!(
                "Skipping {} ({}) - already installed",
                spec.name, spec.version
            );
        }
        return Ok(spec);
    }

    // 5a. Force reinstallation if --force is set
    if options.force {
        let existing_dir = install_dir.join(format!("{}-{}", spec.name, spec.version));
        if existing_dir.exists() {
            if options.verbose {
                println!(
                    "Force reinstalling {} ({}) - removing existing installation",
                    spec.name, spec.version
                );
            }
            fs::remove_dir_all(&existing_dir).context(format!(
                "Failed to remove existing gem directory: {}",
                existing_dir.display()
            ))?;
        }
    }

    // 6. Download gem
    let cache_dir = config::cache_dir(None)?;
    let mut dm = DownloadManager::new(cache_dir)?;

    // Configure local/remote mode
    if options.both {
        // --both: Use default behavior (check cache first, download if needed)
        // This is the default, so no configuration needed
    } else if options.local {
        dm = dm.with_local_only(true);
    } else if options.remote {
        dm = dm.with_skip_cache(true);
    }
    // Default (no flag) uses same behavior as --both: check cache first, download if needed

    if options.verbose {
        if options.local {
            println!(
                "Using local cache only for {} ({})...",
                spec.name, spec.version
            );
        } else if options.remote {
            println!(
                "Downloading {} ({}) from remote...",
                spec.name, spec.version
            );
        } else {
            println!("Downloading {} ({})...", spec.name, spec.version);
        }
    }

    let gem_path = dm.download_gem(&spec).await.context(format!(
        "Failed to download {} ({})",
        spec.name, spec.version
    ))?;

    // 7. Verify gem signature if trust policy is enabled
    if *trust_policy != TrustPolicy::NoSecurity {
        verify_gem_signature(&gem_path, *trust_policy)?;
    }

    // 8. Extract gem to installation directory
    if !options.quiet && !options.silent {
        println!("Installing {} ({})...", spec.name, spec.version);
    }

    let gem_install_dir = install_dir.join(format!("{}-{}", spec.name, spec.version));
    extract_gem(&gem_path, &gem_install_dir)?;

    // 9. Build extensions if present
    if has_extensions(&gem_install_dir) {
        if options.verbose {
            println!("Building native extensions for {}...", spec.name);
        }
        build_extensions(&gem_install_dir, options)?;
    }

    // 10. Install executables
    if let Some(bindir) = &options.bindir {
        install_executables(&gem_install_dir, bindir, options)?;
    }

    // 11. Generate documentation
    generate_documentation(&gem_install_dir, &spec, options)?;

    // Note: Post-install messages are displayed in install_gem_with_dependencies()

    if !options.quiet && !options.silent {
        println!("Successfully installed {} ({})", spec.name, spec.version);
    }

    Ok(spec)
}

/// Check if a version string represents a prerelease
fn is_prerelease(version: &str) -> bool {
    // Prerelease versions contain "-" or "." followed by prerelease identifiers
    // Examples: "1.0.0-alpha", "2.3.0-rc1", "3.0.0-beta.2", "2.0.0.pre", "1.0.0.beta.2"
    if version.contains('-') {
        return true;
    }

    // Check for dot-based prerelease versions
    let prerelease_keywords = ["pre", "alpha", "a", "beta", "b", "rc", "c", "dev"];
    for keyword in &prerelease_keywords {
        if version.contains(&format!(".{keyword}")) {
            return true;
        }
    }

    false
}

/// Determine the installation directory based on options
fn determine_install_dir(options: &InstallOptions) -> Result<PathBuf> {
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

/// Check if a gem is already installed
fn is_gem_installed(spec: &GemSpec, install_dir: &Path) -> bool {
    let gem_dir = install_dir.join(format!("{}-{}", spec.name, spec.version));
    gem_dir.exists()
}

/// Verify gem signature using trust policy
fn verify_gem_signature(gem_path: &Path, trust_policy: TrustPolicy) -> Result<()> {
    use lode::trust_policy::GemVerifier;

    let verifier = GemVerifier::new(trust_policy)?;
    verifier.verify_gem(gem_path)?;
    Ok(())
}

/// Extract a gem file to the installation directory
fn extract_gem(gem_path: &Path, install_dir: &Path) -> Result<()> {
    use flate2::read::GzDecoder;
    use tar::Archive;

    fs::create_dir_all(install_dir).context("Failed to create gem directory")?;

    // Step 1: Extract the outer tar to a temp directory
    let temp_dir = tempfile::TempDir::new().context("Failed to create temporary directory")?;

    let file = fs::File::open(gem_path)
        .context(format!("Failed to open gem file: {}", gem_path.display()))?;

    // Gem files are plain tar archives (not tar.gz)
    let mut archive = Archive::new(file);
    archive
        .unpack(temp_dir.path())
        .context("Failed to extract gem archive to temp directory")?;

    // Step 2: Read data.tar.gz from temp directory
    let data_tar_gz_path = temp_dir.path().join("data.tar.gz");
    let data_file = fs::File::open(&data_tar_gz_path).context("Failed to open data.tar.gz")?;

    // Step 3: Extract data.tar.gz contents to install directory
    let data_gz = GzDecoder::new(data_file);
    let mut data_archive = Archive::new(data_gz);
    data_archive
        .unpack(install_dir)
        .context("Failed to extract gem contents from data.tar.gz")?;

    Ok(())
}

/// Check if gem has native extensions
fn has_extensions(gem_dir: &Path) -> bool {
    let ext_dir = gem_dir.join("ext");
    ext_dir.exists() && ext_dir.is_dir()
}

/// Build native extensions for a gem
fn build_extensions(gem_dir: &Path, options: &InstallOptions) -> Result<()> {
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

/// Parse documentation types from --document flag
fn parse_doc_types(
    doc_format: Option<&str>,
    verbose: bool,
) -> std::collections::HashSet<&'static str> {
    let mut types = std::collections::HashSet::new();

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

/// Generate documentation for a gem using `RDoc`
fn generate_documentation(gem_dir: &Path, spec: &GemSpec, options: &InstallOptions) -> Result<()> {
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

/// Install gem executables to bin directory
fn install_executables(gem_dir: &Path, bindir: &str, options: &InstallOptions) -> Result<()> {
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
            // Extract gem name and version from gem_dir
            let gem_name_version = gem_dir
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown");

            // Format: <executable>-<gem-name-version>
            // E.g., "rake" becomes "rake-rake-13.0.1"
            let base_name = file_name.to_str().unwrap_or("unknown");
            format!("{base_name}-{gem_name_version}")
        } else {
            file_name.to_string_lossy().to_string()
        };

        let dest_path = bin_dest.join(&dest_filename);

        if options.wrappers {
            // Create wrapper script
            create_wrapper_script(&src_path, &dest_path, gem_dir, options)?;
        } else {
            // Direct copy
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
    options: &InstallOptions,
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

/// Explain what would be installed without actually installing
async fn explain_install(options: &InstallOptions) -> Result<()> {
    println!("Gems that would be installed:");

    let source_url = options.source.as_deref().unwrap_or(lode::RUBYGEMS_ORG_URL);
    let client = RubyGemsClient::new_with_proxy(source_url, options.http_proxy.as_deref())?;

    for gem_name in &options.gems {
        let versions = client
            .fetch_versions(gem_name)
            .await
            .context(format!("Failed to fetch versions for gem '{gem_name}'"))?;

        if versions.is_empty() {
            println!("  {gem_name} - not found");
            continue;
        }

        // Apply version filtering using the same logic as install
        let selected_version = select_gem_version(gem_name, None, &versions, options, &client)?;

        println!("  - {} ({})", gem_name, selected_version.number);

        if !options.ignore_dependencies {
            println!("    (Dependency resolution not yet implemented)");
        }
    }

    Ok(())
}

/// Select a gem version from available versions based on requirements
fn select_gem_version(
    gem_name: &str,
    version_requirement: Option<&str>,
    versions: &[lode::rubygems_client::GemVersion],
    options: &InstallOptions,
    client: &RubyGemsClient,
) -> Result<lode::rubygems_client::GemVersion> {
    let mut filtered_versions = versions.to_vec();

    // Filter by version constraint (use parameter first, then options)
    let version_req = version_requirement.or(options.version.as_deref());
    if let Some(version_req) = version_req {
        // Parse version requirement using Resolver (supports ~>, >=, <, etc.)
        let resolver = Resolver::new(client.clone());
        let range = resolver
            .parse_version_requirement(gem_name, version_req)
            .context(format!(
                "Invalid version requirement '{version_req}' for gem '{gem_name}'"
            ))?;

        // Filter versions that match the requirement
        filtered_versions.retain(|v| {
            Resolver::parse_semantic_version(&v.number)
                .is_ok_and(|sem_ver| range.contains(&sem_ver))
        });
    }

    // Filter by prerelease (check if version contains "-" which indicates prerelease)
    if !options.prerelease {
        filtered_versions.retain(|v| !is_prerelease(&v.number));
    }

    // Filter by platform
    if let Some(platform) = &options.platform {
        filtered_versions.retain(|v| v.platform == *platform);
    }

    if filtered_versions.is_empty() {
        anyhow::bail!(
            "No matching version found for gem '{}' with constraints: version={:?}, prerelease={}, platform={:?}",
            gem_name,
            version_req,
            options.prerelease,
            options.platform
        );
    }

    // Return first (latest) version from filtered list (cloned)
    Ok(filtered_versions.first().unwrap().clone())
}

/// Create a lock file with installed gem versions
fn create_lock_file(installed: &[GemSpec], options: &InstallOptions) -> Result<()> {
    use std::fmt::Write;

    let lock_file_path = "gem.lock";

    if options.debug {
        eprintln!("DEBUG: Creating lock file: {lock_file_path}");
    }

    let mut lock_content = String::new();
    lock_content.push_str("# gem.lock - Generated by lode gem-install\n");
    lock_content.push_str("# DO NOT EDIT - This file is auto-generated\n\n");

    lock_content.push_str("GEMS:\n");
    for spec in installed {
        writeln!(lock_content, "  {} ({})", spec.name, spec.version)
            .expect("Writing to string should not fail");
        writeln!(
            lock_content,
            "    platform: {}",
            spec.platform.as_deref().unwrap_or("ruby")
        )
        .expect("Writing to string should not fail");
    }

    lock_content.push_str("\nINSTALLED WITH:\n");
    writeln!(lock_content, "   lode {}", env!("CARGO_PKG_VERSION"))
        .expect("Writing to string should not fail");

    fs::write(lock_file_path, lock_content).context("Failed to write lock file")?;

    if options.verbose {
        println!("Created lock file: {lock_file_path}");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Detects standard prerelease version patterns
    #[test]
    fn test_is_prerelease() {
        assert!(is_prerelease("1.0.0-alpha"));
        assert!(is_prerelease("2.3.0-rc1"));
        assert!(is_prerelease("3.0.0-beta.2"));
        assert!(!is_prerelease("1.0.0"));
        assert!(!is_prerelease("2.3.5"));
    }

    /// Resolves vendor directory path when --vendor flag is set
    #[test]
    fn test_determine_install_dir_vendor() {
        let options = InstallOptions {
            vendor: true,
            ..Default::default()
        };

        let result = determine_install_dir(&options).unwrap();
        assert_eq!(
            result,
            PathBuf::from("vendor/gems"),
            "should resolve to vendor/gems"
        );
    }

    /// Resolves custom install directory path
    #[test]
    fn test_install_dir_custom_path() {
        let options = InstallOptions {
            install_dir: Some("/custom/gems".to_string()),
            ..Default::default()
        };

        let result = determine_install_dir(&options).unwrap();
        assert_eq!(
            result,
            PathBuf::from("/custom/gems"),
            "should use provided install directory"
        );
    }
}
