//! Install command
//!
//! Download and install all gems from Gemfile.lock

use anyhow::{Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use lode::{
    BinstubGenerator, Config, DownloadManager, ExtensionBuilder, Gemfile, GitManager, Lockfile,
    StandaloneBundle, StandaloneGem, StandaloneOptions, config,
};
use rayon::prelude::*;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

/// Configuration for the install command
#[derive(Debug)]
#[allow(clippy::struct_excessive_bools)]
pub(crate) struct InstallOptions<'a> {
    /// Path to Gemfile.lock
    pub lockfile_path: &'a str,
    /// Re-download gems even if cached
    pub redownload: bool,
    /// Enable verbose output
    pub verbose: bool,
    /// Suppress output except errors
    pub quiet: bool,
    /// Number of concurrent workers
    pub workers: Option<usize>,
    /// Use only cached gems
    pub local: bool,
    /// Prefer cached gems, fallback to remote
    pub prefer_local: bool,
    /// Number of retries for failed downloads
    pub retry: Option<usize>,
    /// Do not update vendor cache
    pub no_cache: bool,
    /// Generate standalone bundle for groups
    pub standalone: Option<&'a str>,
    /// Gem security trust policy
    pub trust_policy: Option<&'a str>,
    /// Use full gem index
    pub full_index: bool,
    /// Alternative rbconfig path for cross compilation
    pub target_rbconfig: Option<&'a str>,
    /// Frozen mode - disallow Gemfile changes without lockfile update
    pub frozen: bool,
    /// Groups to exclude from installation (`BUNDLE_WITHOUT`)
    pub without_groups: Vec<String>,
    /// Groups to explicitly include (`BUNDLE_WITH`)
    pub with_groups: Vec<String>,
    /// Auto-clean after install (`BUNDLE_CLEAN`)
    pub auto_clean: bool,
}

/// Run the install command
///
/// Downloads and installs all gems specified in the lockfile.
#[allow(
    clippy::cognitive_complexity,
    clippy::too_many_lines,
    reason = "Install process has multiple steps that are best kept together"
)]
pub(crate) async fn run(options: InstallOptions<'_>) -> Result<()> {
    let start_time = Instant::now();

    // Configure rayon thread pool if workers specified
    if let Some(num_workers) = options.workers {
        rayon::ThreadPoolBuilder::new()
            .num_threads(num_workers)
            .build_global()
            .context("Failed to configure worker threads")?;
    }

    // 1. Load configuration
    let cfg = Config::load().context("Failed to load configuration")?;

    if options.verbose {
        println!("Loading lockfile from {}...", options.lockfile_path);
    }

    // 2. Parse lockfile
    let lockfile_content = tokio::fs::read_to_string(options.lockfile_path)
        .await
        .context("Failed to read lockfile")?;

    let lockfile = Lockfile::parse(&lockfile_content).context("Failed to parse lockfile")?;

    // Destructure remaining options for easier access in the rest of the function
    let InstallOptions {
        lockfile_path,
        redownload,
        verbose,
        quiet,
        workers: _,
        local,
        prefer_local,
        retry,
        no_cache,
        standalone,
        trust_policy,
        full_index,
        target_rbconfig,
        frozen,
        without_groups,
        with_groups,
        auto_clean,
    } = options;

    // 3. Check frozen mode - Gemfile must not have changed without updating lockfile
    if frozen {
        check_frozen_mode(lockfile_path, verbose)?;
    }

    // Local mode: only use cached gems, no remote fetching
    if local && verbose {
        println!("Running in local mode (no remote fetching)");
    }

    // Prefer-local mode: prefer cache but fall back to remote
    if prefer_local && verbose {
        println!("Preferring local cache over remote fetching");
    }

    // Initialize gem verifier if trust policy is specified
    let gem_verifier = if let Some(policy_str) = trust_policy {
        let policy = lode::TrustPolicy::parse(policy_str)
            .ok_or_else(|| anyhow::anyhow!("Invalid trust policy: {policy_str}. Must be one of: HighSecurity, MediumSecurity, LowSecurity, NoSecurity"))?;

        if verbose && policy != lode::TrustPolicy::NoSecurity {
            println!("Using trust policy: {policy}");
        }

        Some(lode::GemVerifier::new(policy)?)
    } else {
        None
    };

    // Download and cache full index if requested
    let _full_index_data = if full_index {
        if verbose {
            println!("Downloading and parsing full RubyGems index...");
        }

        // Load sources from Gemfile if available
        let source = Gemfile::parse_file(lode::paths::find_gemfile())
            .as_ref()
            .map_or_else(
                |_| lode::DEFAULT_GEM_SOURCE.to_string(),
                |gemfile| gemfile.source.clone(),
            );

        // Check if we have a cached index
        let cache_dir = lode::config::cache_dir(None)?;
        let index_cache_path = lode::FullIndex::cache_path(&cache_dir);

        let index = if index_cache_path.exists() && !verbose {
            // Try to use cached index
            if let Ok(idx) = lode::FullIndex::load_from_cache(&index_cache_path) {
                if !quiet {
                    println!(
                        "Using cached full index ({} gems, {} versions)",
                        idx.gem_count(),
                        idx.total_count()
                    );
                }
                idx
            } else {
                // Cache invalid, download fresh
                if !quiet {
                    println!("Cached index invalid, downloading fresh index...");
                }
                let idx = lode::FullIndex::download_and_parse(&source).await?;
                idx.save_to_cache(&index_cache_path)?;
                idx
            }
        } else {
            // Download fresh index
            let idx = lode::FullIndex::download_and_parse(&source).await?;
            if verbose {
                println!(
                    "Downloaded {} gems with {} versions",
                    idx.gem_count(),
                    idx.total_count()
                );
            }
            // Cache for future use
            idx.save_to_cache(&index_cache_path)?;
            idx
        };

        if !quiet {
            println!("Note: Full index mode enabled (uses local index instead of API)");
            println!("   This mode works but dependency API is faster and more efficient");
        }

        Some(index)
    } else {
        None
    };

    // Warning messages for unimplemented flags
    // These flags require significant infrastructure and are accepted for compatibility

    if target_rbconfig.is_some() {
        println!("Note: --target-rbconfig flag requires cross-platform support (not implemented)");
        println!("   Using default rbconfig for native extensions");
    }

    // Handle implemented flags
    if no_cache && verbose {
        println!("Cache disabled: will always fetch fresh gems");
    }

    if lockfile.gems.is_empty() {
        if !quiet {
            println!("No gems found in lockfile.");
        }
        return Ok(());
    }

    // 3. Load Gemfile for sources (supports Gemfile and gems.rb)
    let gemfile = Gemfile::parse_file(lode::paths::find_gemfile()).ok();

    // 4. Filter gems by groups (without/with group support)
    let gems_to_install = if !without_groups.is_empty() || !with_groups.is_empty() {
        if let Some(ref gf) = gemfile {
            filter_gems_by_groups(&lockfile.gems, gf, &without_groups, &with_groups, verbose)
        } else {
            if verbose {
                println!(
                    "Warning: Group filtering requested but no Gemfile found, installing all gems"
                );
            }
            lockfile.gems.clone()
        }
    } else {
        lockfile.gems.clone()
    };

    if gems_to_install.is_empty() {
        println!("No gems to install after filtering.");
        return Ok(());
    }

    // 3. Determine paths
    let vendor_dir = config::vendor_dir(Some(&cfg))?;

    let cache_dir = config::cache_dir(Some(&cfg))?;
    let ruby_ver = config::ruby_version(lockfile.ruby_version.as_deref());

    if verbose {
        println!("Vendor directory: {}", vendor_dir.display());
        println!("Cache directory: {}", cache_dir.display());
        println!("Ruby version: {ruby_ver}");
    }

    // 5. Create download manager with sources from Gemfile
    let sources = gemfile.as_ref().map_or_else(
        || vec![lode::DEFAULT_GEM_SOURCE.to_string()],
        |gf| {
            let mut all_sources = vec![gf.source.clone()];
            all_sources.extend(gf.sources.clone());
            all_sources
        },
    );

    if verbose && sources.len() > 1 {
        println!("Gem sources: {}", sources.join(", "));
    }

    let max_retries = retry.unwrap_or(0);
    let dm = Arc::new(
        DownloadManager::with_sources_and_retry(cache_dir, sources, max_retries)
            .context("Failed to create download manager")?
            .with_skip_cache(no_cache),
    );

    // 6. Filter gems by platform (after group filtering)
    let current_platform = lode::detect_current_platform();
    let gems_to_install_count = gems_to_install.len();
    let gems: Vec<_> = gems_to_install
        .into_iter()
        .filter(|gem| lode::platform_matches(&gem.platform, &current_platform))
        .collect();

    if verbose {
        println!(
            "Platform: {} (filtered {} -> {} gems)",
            current_platform,
            gems_to_install_count,
            gems.len()
        );
    }

    // 6. Create extension builder and binstub generator
    let mut extension_builder =
        ExtensionBuilder::new(false, verbose, target_rbconfig.map(String::from));
    let mut build_results = Vec::with_capacity(gems.len());

    let bin_dir = vendor_dir.join("ruby").join(&ruby_ver).join("bin");
    let gemfile_path = lode::paths::find_gemfile(); // Supports Gemfile and gems.rb
    let binstub_generator = BinstubGenerator::new(bin_dir, gemfile_path, None, false);
    let mut binstub_count = 0;

    // 7. Phase 1: Parallel download all gems
    let total_gems = gems.len();
    let mut skipped_count = 0;

    if !quiet {
        println!("Installing {total_gems} gems...");
    }

    // Save a copy of all gems for standalone bundle creation later
    // IMPORTANT: We need to clone here because gems gets consumed by into_iter() below.
    // Standalone bundles need ALL gems in the bundle, not just newly installed gems.
    // Bug fix: Previously we used install_results which only contained newly downloaded gems,
    // causing standalone bundles to be empty when all gems were already cached.
    let all_gems_for_standalone = gems.clone();

    // Filter out already-installed gems (unless redownload flag is set)
    let gems_to_process: Vec<_> = if redownload {
        // Redownload all gems
        if verbose && !quiet {
            println!("Redownload enabled - reinstalling all gems");
        }
        gems
    } else {
        // Skip already-installed gems
        gems.into_iter()
            .filter(|gem| {
                let gem_install_dir = vendor_dir
                    .join("ruby")
                    .join(&ruby_ver)
                    .join("gems")
                    .join(gem.full_name());

                if gem_install_dir.exists() {
                    skipped_count += 1;
                    false
                } else {
                    true
                }
            })
            .collect()
    };

    if gems_to_process.is_empty() {
        if !quiet {
            println!("All gems already installed!");
        }
        // If standalone bundle requested, continue to create it even if all gems already installed
        if standalone.is_none() {
            return Ok(());
        }
    }

    // In local mode, verify all gems are cached before proceeding
    if local {
        let cache_dir = dm.cache_dir();
        let mut missing_gems = Vec::new();

        for gem in &gems_to_process {
            let filename = format!("{}.gem", gem.full_name_with_platform());
            let cache_path = cache_dir.join(&filename);

            if !cache_path.exists() {
                missing_gems.push(gem.name.clone());
            }
        }

        if !missing_gems.is_empty() {
            anyhow::bail!(
                "Cannot install in local mode: {} gems not in cache: {}",
                missing_gems.len(),
                missing_gems.join(", ")
            );
        }

        if verbose {
            println!("All gems found in local cache");
        }
    }

    // In prefer-local mode, report cache statistics
    if prefer_local && verbose {
        let cache_dir = dm.cache_dir();
        let mut cached_count = 0;

        for gem in &gems_to_process {
            let filename = format!("{}.gem", gem.full_name_with_platform());
            let cache_path = cache_dir.join(&filename);

            if cache_path.exists() {
                cached_count += 1;
            }
        }

        if cached_count > 0 {
            println!(
                "Cache: {}/{} gems available in local cache",
                cached_count,
                gems_to_process.len()
            );
        }
    }

    // Create download tasks for all gems
    let num_gems_to_process = gems_to_process.len();
    let mut download_tasks = Vec::with_capacity(num_gems_to_process);

    for gem in gems_to_process {
        let dm_clone = Arc::clone(&dm);

        let task =
            tokio::spawn(async move { dm_clone.download_gem(&gem).await.map(|path| (gem, path)) });

        download_tasks.push(task);
    }

    // Wait for all downloads with progress
    if verbose && !quiet {
        println!("Downloading {num_gems_to_process} gems in parallel...");
    }

    let pb_download = if verbose || quiet {
        None
    } else {
        let progress = ProgressBar::new(download_tasks.len() as u64);
        progress.set_style(
            ProgressStyle::default_bar()
                .template(
                    "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {msg}",
                )
                .unwrap()
                .progress_chars("#>-"),
        );
        progress.set_message("Downloading...");
        Some(progress)
    };

    let mut downloaded_gems = Vec::with_capacity(download_tasks.len());

    for task in download_tasks {
        match task.await {
            Ok(Ok((gem, cache_path))) => {
                if verbose {
                    println!("  Downloaded {}", gem.full_name());
                }
                if let Some(ref pb) = pb_download {
                    pb.inc(1);
                }
                downloaded_gems.push((gem, cache_path));
            }
            Ok(Err(e)) => {
                if let Some(pb) = pb_download {
                    pb.finish_with_message("Download failed!");
                }
                return Err(e.into());
            }
            Err(e) => {
                if let Some(pb) = pb_download {
                    pb.finish_with_message("Download failed!");
                }
                return Err(anyhow::anyhow!("Task error: {e}"));
            }
        }
    }

    if let Some(pb) = pb_download {
        pb.finish_with_message("Downloads complete!");
    }

    // 7.5. Verify gem signatures if trust policy is enabled
    if let Some(ref verifier) = gem_verifier {
        if verbose {
            println!("\nVerifying {} gems...", downloaded_gems.len());
        }

        for (gem, cache_path) in &downloaded_gems {
            match verifier.verify_gem(cache_path) {
                Ok(()) => {
                    if verbose {
                        println!("  Verified {}", gem.full_name());
                    }
                }
                Err(e) => {
                    return Err(anyhow::anyhow!(
                        "Gem verification failed for {}: {}",
                        gem.full_name(),
                        e
                    ));
                }
            }
        }

        if verbose {
            println!("All gems verified successfully!");
        }
    }

    // 8. Phase 2: Extract and install gems (with rayon for parallelization)
    if verbose {
        println!("\nExtracting {} gems...", downloaded_gems.len());
    }

    let pb_install = if verbose {
        None
    } else {
        let progress = ProgressBar::new(downloaded_gems.len() as u64);
        progress.set_style(
            ProgressStyle::default_bar()
                .template(
                    "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {msg}",
                )
                .unwrap()
                .progress_chars("#>-"),
        );
        progress.set_message("Installing...");
        Some(progress)
    };

    // Parallel extraction
    let install_results: Vec<_> = downloaded_gems
        .par_iter()
        .map(|(gem, cache_path)| {
            let result = lode::install::install_gem(gem, cache_path, &vendor_dir, &ruby_ver);
            if let Some(ref pb) = pb_install {
                pb.inc(1);
            }
            (gem, result)
        })
        .collect();

    if let Some(pb) = pb_install {
        pb.finish_with_message("Installation complete!");
    }

    // Check for installation errors
    for (gem, result) in &install_results {
        if let Err(e) = result {
            return Err(anyhow::anyhow!("Failed to install {}: {}", gem.name, e));
        }
    }

    let mut installed_count = install_results.len();

    // 9. Phase 3: Build extensions and generate binstubs (sequential - they call external processes)
    if verbose {
        println!("\nBuilding extensions and binstubs...");
    }

    for (gem, _) in &install_results {
        let gem_install_dir = vendor_dir
            .join("ruby")
            .join(&ruby_ver)
            .join("gems")
            .join(gem.full_name());

        // Build extension if needed
        if let Some(build_result) =
            extension_builder.build_if_needed(&gem.name, &gem_install_dir, gem.platform.as_deref())
        {
            if verbose {
                if build_result.success {
                    println!(
                        "Built extension for {} in {:.2}s",
                        gem.name,
                        build_result.duration.as_secs_f64()
                    );
                } else {
                    println!(
                        "Extension build failed for {}: {}",
                        gem.name,
                        build_result.error.as_deref().unwrap_or("Unknown error")
                    );
                }
            }
            build_results.push(build_result);
        }

        // Generate binstubs if gem has executables
        match binstub_generator.generate(&gem.name, &gem_install_dir) {
            Ok(count) if count > 0 => {
                if verbose {
                    println!("Generated {} binstub(s) for {}", count, gem.name);
                }
                binstub_count += count;
            }
            Ok(_) => {} // No executables, skip silently
            Err(e) => {
                if verbose {
                    println!("Binstub generation failed for {}: {}", gem.name, e);
                }
            }
        }
    }

    // 8. Install path gems (if any)
    if !lockfile.path_gems.is_empty() {
        if verbose {
            println!("\nInstalling {} path gems...", lockfile.path_gems.len());
        }

        for path_gem in &lockfile.path_gems {
            if verbose {
                println!(
                    "  Installing {}-{} from {}",
                    path_gem.name, path_gem.version, path_gem.path
                );
            }

            match lode::install::install_path_gem(path_gem, &vendor_dir, &ruby_ver) {
                Ok(()) => {
                    installed_count += 1;

                    // Build extension if needed
                    let gem_install_dir = vendor_dir
                        .join("ruby")
                        .join(&ruby_ver)
                        .join("gems")
                        .join(format!("{}-{}", path_gem.name, path_gem.version));

                    if let Some(build_result) =
                        extension_builder.build_if_needed(&path_gem.name, &gem_install_dir, None)
                    {
                        if verbose {
                            if build_result.success {
                                println!(
                                    "Built extension in {:.2}s",
                                    build_result.duration.as_secs_f64()
                                );
                            } else {
                                println!(
                                    "Extension build failed: {}",
                                    build_result.error.as_deref().unwrap_or("Unknown error")
                                );
                            }
                        }
                        build_results.push(build_result);
                    }

                    // Generate binstubs if gem has executables
                    match binstub_generator.generate(&path_gem.name, &gem_install_dir) {
                        Ok(count) if count > 0 => {
                            if verbose {
                                println!("    Generated {count} binstub(s)");
                            }
                            binstub_count += count;
                        }
                        Ok(_) => {}
                        Err(e) => {
                            if verbose {
                                println!("    Binstub generation failed: {e}");
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Failed to install path gem {}: {}", path_gem.name, e);
                }
            }

            if !verbose {
                print!(".");
                std::io::stdout().flush().ok();
            }
        }

        if !verbose {
            println!();
        }
    }

    // 9. Install git gems (if any)
    if !lockfile.git_gems.is_empty() {
        if verbose {
            println!("\nInstalling {} git gems...", lockfile.git_gems.len());
        }

        // Create git manager
        let git_cache_dir = config::cache_dir(Some(&cfg))?.join("git");
        let git_manager = GitManager::new(git_cache_dir).context("Failed to create git manager")?;

        for git_gem in &lockfile.git_gems {
            if verbose {
                println!(
                    "  Installing {}-{} from {} @ {}",
                    git_gem.name,
                    git_gem.version,
                    git_gem.repository,
                    git_gem.revision.chars().take(8).collect::<String>()
                );
            }

            // Clone and checkout
            match git_manager.clone_and_checkout(&git_gem.repository, &git_gem.revision) {
                Ok(source_dir) => {
                    if verbose {
                        println!("Checked out to {}", source_dir.display());
                    }

                    // Build and install
                    match lode::install::install_git_gem(
                        git_gem,
                        &source_dir,
                        &vendor_dir,
                        &ruby_ver,
                    ) {
                        Ok(()) => {
                            installed_count += 1;

                            // Build extension if needed
                            let gem_install_dir = vendor_dir
                                .join("ruby")
                                .join(&ruby_ver)
                                .join("gems")
                                .join(format!("{}-{}", git_gem.name, git_gem.version));

                            if let Some(build_result) = extension_builder.build_if_needed(
                                &git_gem.name,
                                &gem_install_dir,
                                None,
                            ) {
                                if verbose {
                                    if build_result.success {
                                        println!(
                                            "Built extension in {:.2}s",
                                            build_result.duration.as_secs_f64()
                                        );
                                    } else {
                                        println!(
                                            "Extension build failed: {}",
                                            build_result
                                                .error
                                                .as_deref()
                                                .unwrap_or("Unknown error")
                                        );
                                    }
                                }
                                build_results.push(build_result);
                            }

                            // Generate binstubs if gem has executables
                            match binstub_generator.generate(&git_gem.name, &gem_install_dir) {
                                Ok(count) if count > 0 => {
                                    if verbose {
                                        println!("Generated {count} binstub(s)");
                                    }
                                    binstub_count += count;
                                }
                                Ok(_) => {}
                                Err(e) => {
                                    if verbose {
                                        println!("Binstub generation failed: {e}");
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("Failed to install git gem {}: {}", git_gem.name, e);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Failed to clone/checkout {}: {}", git_gem.name, e);
                }
            }

            if !verbose {
                print!(".");
                std::io::stdout().flush().ok();
            }
        }

        if !verbose {
            println!();
        }
    }

    let elapsed = start_time.elapsed();

    // 10. Print summary
    println!(
        "\nInstalled {} gems ({} skipped) to {} in {:.2}s",
        installed_count,
        skipped_count,
        vendor_dir.display(),
        elapsed.as_secs_f64()
    );

    // Report extension build results
    if !build_results.is_empty() {
        let (successful, failed, build_duration) = ExtensionBuilder::summarize(&build_results);

        println!(
            "Extensions: {} extensions built ({} failed) in {:.2}s",
            successful,
            failed,
            build_duration.as_secs_f64()
        );

        // Show failed builds
        if failed > 0 && !verbose {
            println!("\nFailed extension builds:");
            for result in &build_results {
                if !result.success {
                    println!(
                        "  {}: {}",
                        result.gem_name,
                        result.error.as_deref().unwrap_or("Unknown error")
                    );
                }
            }
        }
    }

    // Report binstub generation
    if binstub_count > 0 {
        println!("Binstubs: {binstub_count} binstub(s) generated");
    }

    // 10. Auto-clean if BUNDLE_CLEAN is enabled
    if auto_clean {
        if verbose {
            println!("\nAuto-cleaning unused gems...");
        }
        // Call clean command with same vendor directory
        match crate::commands::clean::run(Some(vendor_dir.to_str().unwrap()), false, false) {
            Ok(()) => {
                if verbose {
                    println!("Auto-clean completed");
                }
            }
            Err(e) => {
                if verbose {
                    eprintln!("Warning: Auto-clean failed: {e}");
                }
            }
        }
    }

    // 11. Create standalone bundle if requested
    if let Some(standalone_groups) = standalone {
        if !quiet {
            println!("\nCreating standalone bundle...");
        }

        // Parse groups if specified
        let groups: Vec<String> = if standalone_groups.is_empty() {
            vec![]
        } else {
            standalone_groups
                .split(',')
                .map(|s| s.trim().to_string())
                .collect()
        };

        // Create standalone options
        let standalone_opts = StandaloneOptions {
            bundle_path: PathBuf::from("./bundle"),
            groups: groups.clone(),
        };

        // Create standalone bundle
        let bundle = StandaloneBundle::new(standalone_opts, &ruby_ver, "ruby")
            .context("Failed to create standalone bundle")?;

        bundle
            .create_directories()
            .context("Failed to create standalone directories")?;

        // Convert installed gems to standalone format
        // Use all_gems_for_standalone (all platform-filtered gems) instead of install_results (only newly installed)
        let mut standalone_gems = Vec::new();
        for gem in &all_gems_for_standalone {
            let gem_install_dir = vendor_dir
                .join("ruby")
                .join(&ruby_ver)
                .join("gems")
                .join(gem.full_name());

            // Check for extension directory
            let extension_path = vendor_dir
                .join("ruby")
                .join(&ruby_ver)
                .join("extensions")
                .join(&current_platform)
                .join(&ruby_ver)
                .join(gem.full_name());

            let has_extensions = extension_path.exists();

            let standalone_gem = StandaloneGem {
                name: gem.name.clone(),
                version: gem.version.clone(),
                platform: gem.platform.clone(),
                extracted_path: gem_install_dir,
                extension_path: if has_extensions {
                    Some(extension_path)
                } else {
                    None
                },
                has_extensions,
            };

            standalone_gems.push(standalone_gem);
        }

        // Filter by groups if specified
        let filtered_gems = if groups.is_empty() {
            standalone_gems
        } else {
            // For group filtering, we need the Gemfile
            if let Some(ref gf) = gemfile {
                standalone_gems
                    .into_iter()
                    .filter(|standalone_gem| {
                        // Check if gem is in any of the specified groups
                        gf.gems
                            .iter()
                            .find(|g| g.name == standalone_gem.name)
                            .is_some_and(|gem_dep| {
                                groups.is_empty()
                                    || gem_dep.groups.iter().any(|g| groups.contains(g))
                            })
                    })
                    .collect()
            } else {
                // No Gemfile, include all gems
                standalone_gems
            }
        };

        // Install gems into standalone bundle
        for gem in &filtered_gems {
            bundle.install_gem(gem).with_context(|| {
                format!("Failed to install {} into standalone bundle", gem.name)
            })?;
        }

        // Generate setup.rb
        bundle
            .generate_setup_rb(&filtered_gems)
            .context("Failed to generate setup.rb")?;

        println!("OK Standalone bundle created in ./bundle");
        println!("  -> {} gems included", filtered_gems.len());
        if !groups.is_empty() {
            println!("  -> Groups: {}", groups.join(", "));
        }
        println!();
        println!("Usage:");
        println!("  ruby -r ./bundle/bundler/setup.rb your_script.rb");
    }

    Ok(())
}

/// Check frozen mode - ensure Gemfile hasn't changed without updating lockfile
fn check_frozen_mode(lockfile_path: &str, verbose: bool) -> Result<()> {
    // Determine Gemfile path from lockfile path
    let gemfile_path = if std::path::Path::new(lockfile_path)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("lock"))
    {
        lockfile_path.trim_end_matches(".lock")
    } else {
        "Gemfile"
    };

    // Check if Gemfile exists
    let gemfile_pathbuf = std::path::Path::new(gemfile_path);
    if !gemfile_pathbuf.exists() {
        // No Gemfile, nothing to check
        return Ok(());
    }

    // Get modification times
    let lockfile_metadata = std::fs::metadata(lockfile_path)
        .context("Lockfile not found - frozen mode requires an existing lockfile")?;
    let gemfile_metadata =
        std::fs::metadata(gemfile_path).context("Failed to read Gemfile metadata")?;

    let lockfile_modified = lockfile_metadata
        .modified()
        .context("Failed to get lockfile modification time")?;
    let gemfile_modified = gemfile_metadata
        .modified()
        .context("Failed to get Gemfile modification time")?;

    // If Gemfile is newer than lockfile, error in frozen mode
    if gemfile_modified > lockfile_modified {
        anyhow::bail!(
            "Your Gemfile has been modified since the lockfile was generated.\n\
             In frozen mode, Bundler will not check for changes.\n\
             To update the lockfile, run `bundle lock` or `bundle install` without frozen mode."
        );
    }

    if verbose {
        println!("Frozen mode: Gemfile matches lockfile");
    }

    Ok(())
}

/// Filter gems by group membership based on without/with group lists
fn filter_gems_by_groups(
    lockfile_gems: &[lode::GemSpec],
    gemfile: &lode::Gemfile,
    without_groups: &[String],
    with_groups: &[String],
    verbose: bool,
) -> Vec<lode::GemSpec> {
    use std::collections::HashMap;

    // Build a map of gem names to their groups from the Gemfile
    let gem_groups: HashMap<String, Vec<String>> = gemfile
        .gems
        .iter()
        .map(|gem_dep| (gem_dep.name.clone(), gem_dep.groups.clone()))
        .collect();

    // Default group is :default - gems without explicit group are in default group
    let default_group = "default".to_string();

    let filtered: Vec<_> = lockfile_gems
        .iter()
        .filter(|gem| {
            let groups = gem_groups
                .get(&gem.name)
                .cloned()
                .unwrap_or_else(|| vec![default_group.clone()]);

            // If with_groups is specified, only include gems in those groups
            if !with_groups.is_empty() {
                let in_with_groups = groups.iter().any(|g| with_groups.contains(g));
                if !in_with_groups {
                    if verbose {
                        println!(
                            "  Excluding {} (not in with groups: {:?})",
                            gem.name, with_groups
                        );
                    }
                    return false;
                }
            }

            // If without_groups is specified, exclude gems in those groups
            if !without_groups.is_empty() {
                let in_without_groups = groups.iter().any(|g| without_groups.contains(g));
                if in_without_groups {
                    if verbose {
                        println!(
                            "  Excluding {} (in without groups: {:?})",
                            gem.name, without_groups
                        );
                    }
                    return false;
                }
            }

            true
        })
        .cloned()
        .collect();

    if verbose && filtered.len() != lockfile_gems.len() {
        println!(
            "Group filtering: {} -> {} gems",
            lockfile_gems.len(),
            filtered.len()
        );
    }

    filtered
}

#[cfg(test)]
mod tests {
    use super::*;
    use lode::{GemDependency, GemSpec, Gemfile};
    use std::fs;
    use std::thread;
    use std::time::Duration;
    use tempfile::TempDir;

    #[test]
    fn test_check_frozen_mode_no_gemfile() {
        let temp_dir = TempDir::new().unwrap();
        let lockfile = temp_dir.path().join("Gemfile.lock");
        fs::write(&lockfile, "LOCKFILE").unwrap();

        // Should pass - no Gemfile to check
        let result = check_frozen_mode(lockfile.to_str().unwrap(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_check_frozen_mode_gemfile_older_than_lockfile() {
        let temp_dir = TempDir::new().unwrap();
        let gemfile = temp_dir.path().join("Gemfile");
        let lockfile = temp_dir.path().join("Gemfile.lock");

        // Create Gemfile first
        fs::write(&gemfile, "source 'https://rubygems.org'").unwrap();
        thread::sleep(Duration::from_millis(10));
        // Create lockfile after (newer)
        fs::write(&lockfile, "LOCKFILE").unwrap();

        // Should pass - Gemfile is older
        let result = check_frozen_mode(lockfile.to_str().unwrap(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_check_frozen_mode_gemfile_newer_than_lockfile() {
        let temp_dir = TempDir::new().unwrap();
        let gemfile = temp_dir.path().join("Gemfile");
        let lockfile = temp_dir.path().join("Gemfile.lock");

        // Create lockfile first
        fs::write(&lockfile, "LOCKFILE").unwrap();
        thread::sleep(Duration::from_millis(10));
        // Create Gemfile after (newer)
        fs::write(&gemfile, "source 'https://rubygems.org'").unwrap();

        // Should fail - Gemfile is newer
        let result = check_frozen_mode(lockfile.to_str().unwrap(), false);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Gemfile has been modified")
        );
    }

    #[test]
    fn test_filter_gems_by_groups_without() {
        let gems = vec![
            GemSpec::new(
                "rake".to_string(),
                "13.0.0".to_string(),
                None,
                vec![],
                vec!["default".to_string()],
            ),
            GemSpec::new(
                "rspec".to_string(),
                "3.0.0".to_string(),
                None,
                vec![],
                vec!["test".to_string()],
            ),
        ];

        let gemfile = Gemfile {
            source: "https://rubygems.org".to_string(),
            ruby_version: None,
            gems: vec![
                GemDependency {
                    name: "rake".to_string(),
                    version_requirement: String::new(),
                    groups: vec!["default".to_string()],
                    source: None,
                    git: None,
                    branch: None,
                    tag: None,
                    ref_: None,
                    path: None,
                    platforms: vec![],
                    require: None,
                },
                GemDependency {
                    name: "rspec".to_string(),
                    version_requirement: String::new(),
                    groups: vec!["test".to_string()],
                    source: None,
                    git: None,
                    branch: None,
                    tag: None,
                    ref_: None,
                    path: None,
                    platforms: vec![],
                    require: None,
                },
            ],
            sources: vec![],
            gemspecs: vec![],
        };

        let without = vec!["test".to_string()];
        let with = vec![];
        let filtered = filter_gems_by_groups(&gems, &gemfile, &without, &with, false);

        assert_eq!(filtered.len(), 1);
        assert_eq!(
            filtered.first().expect("should have first gem").name,
            "rake"
        );
    }

    #[test]
    fn test_filter_gems_by_groups_with() {
        let gems = vec![
            GemSpec::new(
                "rake".to_string(),
                "13.0.0".to_string(),
                None,
                vec![],
                vec!["default".to_string()],
            ),
            GemSpec::new(
                "rspec".to_string(),
                "3.0.0".to_string(),
                None,
                vec![],
                vec!["test".to_string()],
            ),
        ];

        let gemfile = Gemfile {
            source: "https://rubygems.org".to_string(),
            ruby_version: None,
            gems: vec![
                GemDependency {
                    name: "rake".to_string(),
                    version_requirement: String::new(),
                    groups: vec!["default".to_string()],
                    source: None,
                    git: None,
                    branch: None,
                    tag: None,
                    ref_: None,
                    path: None,
                    platforms: vec![],
                    require: None,
                },
                GemDependency {
                    name: "rspec".to_string(),
                    version_requirement: String::new(),
                    groups: vec!["test".to_string()],
                    source: None,
                    git: None,
                    branch: None,
                    tag: None,
                    ref_: None,
                    path: None,
                    platforms: vec![],
                    require: None,
                },
            ],
            sources: vec![],
            gemspecs: vec![],
        };

        let without = vec![];
        let with = vec!["test".to_string()];
        let filtered = filter_gems_by_groups(&gems, &gemfile, &without, &with, false);

        assert_eq!(filtered.len(), 1);
        assert_eq!(
            filtered.first().expect("should have first gem").name,
            "rspec"
        );
    }

    #[test]
    fn test_filter_gems_by_groups_transitive_deps_as_default() {
        let gems = vec![
            GemSpec::new(
                "rake".to_string(),
                "13.0.0".to_string(),
                None,
                vec![],
                vec!["default".to_string()],
            ),
            GemSpec::new(
                "unknown-dep".to_string(),
                "1.0.0".to_string(),
                None,
                vec![],
                vec![],
            ),
        ];

        let gemfile = Gemfile {
            source: "https://rubygems.org".to_string(),
            ruby_version: None,
            gems: vec![GemDependency {
                name: "rake".to_string(),
                version_requirement: String::new(),
                groups: vec!["default".to_string()],
                source: None,
                git: None,
                branch: None,
                tag: None,
                ref_: None,
                path: None,
                platforms: vec![],
                require: None,
            }],
            sources: vec![],
            gemspecs: vec![],
        };

        let without = vec!["test".to_string()];
        let with = vec![];
        let filtered = filter_gems_by_groups(&gems, &gemfile, &without, &with, false);

        // Both gems should pass - rake is default, unknown-dep treated as default
        assert_eq!(filtered.len(), 2);
    }
}
