//! Lode command-line interface
//!
//! Bundler and `RubyGems` compatible package manager for Ruby

use clap::{Parser, Subcommand};
use std::process;

/// Note: backtrace display is controlled by the `--backtrace` flag
/// Actual backtrace capture requires `RUST_BACKTRACE` environment variable to be set
fn setup_backtrace(_enabled: bool) {
    // Backtrace display is handled in display_error() function
}

/// Display an error with optional backtrace information
fn display_error(err: &anyhow::Error, backtrace_enabled: bool) {
    eprintln!("error: {err}");

    // Show error chain
    let mut source = err.source();
    while let Some(err) = source {
        eprintln!("caused by: {err}");
        source = err.source();
    }

    // Show backtrace if enabled
    if backtrace_enabled {
        let backtrace = err.backtrace();
        if backtrace.status() == std::backtrace::BacktraceStatus::Captured {
            eprintln!("\nBacktrace:");
            eprintln!("{backtrace}");
        }
    }
}

#[derive(Parser)]
#[command(name = "lode")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "A Ruby package manager", long_about = None)]
#[command(disable_version_flag = true)]
pub(crate) struct Cli {
    /// Print version
    #[arg(short = 'v', long = "version", action = clap::ArgAction::Version)]
    _version: Option<bool>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
#[allow(clippy::large_enum_variant)]
enum Commands {
    /// Install gems from Gemfile.lock
    Install {
        /// Path to Gemfile (lockfile will be derived as Gemfile.lock)
        #[arg(long)]
        gemfile: Option<String>,

        /// Re-download or reinstall even if artifacts exist (replaces deprecated --force)
        #[arg(long, visible_alias = "force")]
        redownload: bool,

        /// Enable verbose output including extension build logs
        #[arg(long)]
        verbose: bool,

        /// Suppress all output except errors
        #[arg(long, short, conflicts_with = "verbose")]
        quiet: bool,

        /// Number of concurrent downloads (Bundler: --jobs/-j)
        #[arg(long, short = 'j', alias = "workers")]
        jobs: Option<usize>,

        /// Do not fetch gems remotely, use only local cache
        #[arg(long)]
        local: bool,

        /// Prefer local cache over remote fetching
        #[arg(long, conflicts_with = "local")]
        prefer_local: bool,

        /// Number of times to retry failed downloads
        #[arg(long)]
        retry: Option<usize>,

        /// Do not update the cache in vendor/cache
        #[arg(long)]
        no_cache: bool,

        /// Generate standalone bundle that works without Bundler (optional: specify groups)
        #[arg(long)]
        standalone: Option<String>,

        /// Gem security trust policy: `HighSecurity`, `MediumSecurity`, `LowSecurity`, or `NoSecurity`
        #[arg(long)]
        trust_policy: Option<String>,

        /// Use full gem index instead of dependency API
        #[arg(long)]
        full_index: bool,

        /// Use alternative rbconfig for native extensions (for cross-compilation)
        #[arg(long)]
        target_rbconfig: Option<String>,
    },

    /// Update gems to their latest versions within constraints
    Update {
        /// Specific gems to update (updates all if not specified)
        gems: Vec<String>,

        /// Update all gems specified in Gemfile
        #[arg(long)]
        all: bool,

        /// Use conservative update strategy (minimal version changes)
        #[arg(long)]
        conservative: bool,

        /// Path to Gemfile
        #[arg(long)]
        gemfile: Option<String>,

        /// Number of concurrent jobs
        #[arg(long, short = 'j')]
        jobs: Option<usize>,

        /// Suppress all output except errors
        #[arg(long)]
        quiet: bool,

        /// Number of times to retry failed requests
        #[arg(long)]
        retry: Option<usize>,

        /// Prefer updating only to next patch version
        #[arg(long, conflicts_with_all = ["minor", "major"])]
        patch: bool,

        /// Prefer updating only to next minor version
        #[arg(long, conflicts_with_all = ["patch", "major"])]
        minor: bool,

        /// Prefer updating to next major version (default)
        #[arg(long, conflicts_with_all = ["patch", "minor"])]
        major: bool,

        /// Do not allow any gem to be updated past latest patch/minor/major
        #[arg(long)]
        strict: bool,

        /// Do not attempt to fetch gems remotely (use cached gems only)
        #[arg(long)]
        local: bool,

        /// Allow prerelease versions when updating
        #[arg(long)]
        pre: bool,

        /// Only update gems in the specified group
        #[arg(long, short = 'g')]
        group: Option<String>,

        /// Update gems from the specified git or path source
        #[arg(long)]
        source: Option<String>,

        /// Update locked Ruby version in Gemfile.lock
        #[arg(long)]
        ruby: bool,

        /// Update locked Bundler version in Gemfile.lock (uses current lode version if no version specified)
        #[arg(long, num_args(0..=1), default_missing_value = "")]
        bundler: Option<String>,

        /// Force re-downloading of gems even if already cached
        #[arg(long)]
        redownload: bool,

        /// Use full gem index instead of dependency API
        #[arg(long)]
        full_index: bool,
    },

    /// Package your needed .gem files into vendor/cache
    ///
    /// Copy all of the .gem files needed to run the application into the
    /// vendor/cache directory. In the future, when running bundle install,
    /// use the gems in the cache in preference to the ones on rubygems.org.
    #[command(visible_alias = "package", visible_alias = "pack")]
    Cache {
        /// Include gems for all platforms present in the lockfile
        #[arg(long)]
        all_platforms: bool,

        /// Specify a different cache path than the default (vendor/cache)
        #[arg(long)]
        cache_path: Option<String>,

        /// Use the specified gemfile instead of Gemfile
        #[arg(long)]
        gemfile: Option<String>,

        /// Don't install the gems, only update the cache
        #[arg(long)]
        no_install: bool,

        /// Only output warnings and errors
        #[arg(long)]
        quiet: bool,
    },

    /// Run commands with lode-managed environment
    Exec {
        /// Command to execute
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        command: Vec<String>,

        /// Path to Gemfile
        #[arg(long)]
        gemfile: Option<String>,
    },

    /// Get and set Bundler configuration options
    Config {
        /// Configuration key
        key: Option<String>,

        /// Configuration value
        value: Option<String>,

        /// List all configuration
        #[arg(long)]
        list: bool,

        /// Delete configuration key
        #[arg(long)]
        delete: bool,

        /// Set configuration globally (in ~/.bundle/config)
        #[arg(long, conflicts_with = "local")]
        global: bool,
        /// Set configuration locally (in .bundle/config)
        #[arg(long)]
        local: bool,
    },

    /// Add gems to Gemfile
    Add {
        /// Name of the gem to add
        gem: String,

        /// Version constraint (e.g., "~> 3.0")
        #[arg(short, long)]
        version: Option<String>,

        /// Gem group (e.g., development, test)
        #[arg(short, long)]
        group: Option<String>,

        /// Whether to require the gem (default: true)
        #[arg(short = 'r', long)]
        require: Option<bool>,

        /// Custom gem source URL
        #[arg(short, long)]
        source: Option<String>,

        /// Git repository URL
        #[arg(long, conflicts_with_all = ["path", "source", "github"])]
        git: Option<String>,

        /// GitHub repository (shorthand for --git <https://github.com/USER/REPO>)
        #[arg(long, conflicts_with_all = ["path", "source", "git"])]
        github: Option<String>,

        /// Git branch
        #[arg(long)]
        branch: Option<String>,

        /// Git ref (tag or commit)
        #[arg(long)]
        ref_: Option<String>,

        /// Glob pattern for .gemspec location
        #[arg(long)]
        glob: Option<String>,

        /// Local path to gem
        #[arg(short = 'p', long, conflicts_with_all = ["git", "github", "source"])]
        path: Option<String>,

        /// Add strict version constraint (= version)
        #[arg(long, conflicts_with = "optimistic")]
        strict: bool,

        /// Add optimistic version constraint (>= version)
        #[arg(long, conflicts_with = "strict")]
        optimistic: bool,

        /// Suppress progress output
        #[arg(long)]
        quiet: bool,

        /// Skip running `bundle install` after adding (for Bundler compatibility)
        #[arg(long)]
        skip_install: bool,
    },

    /// Generate binstubs for gem executables
    Binstubs {
        /// Gems to generate binstubs for (generates for all if not specified)
        gems: Vec<String>,

        /// Custom Ruby executable path for shebang line
        #[arg(long)]
        shebang: Option<String>,

        /// Overwrite existing binstubs
        #[arg(long)]
        force: bool,

        /// Create binstubs for all gems
        #[arg(long)]
        all: bool,

        /// Install binstubs for all platforms
        #[arg(long)]
        all_platforms: bool,
    },

    /// Verify all gems are installed
    Check {
        /// Path to Gemfile
        #[arg(long)]
        gemfile: Option<String>,

        /// Show what would be checked without checking
        #[arg(long)]
        dry_run: bool,
    },

    /// Show the source location of a gem
    Show {
        /// Name of the gem (optional when using --paths)
        gem: Option<String>,

        /// List all gem paths instead of showing a single gem
        #[arg(long)]
        paths: bool,
    },

    /// List gems with newer versions available
    Outdated {
        /// Path to Gemfile.lock
        #[arg(long, default_value = "Gemfile.lock")]
        lockfile: String,

        /// Output in machine-readable format
        #[arg(long)]
        parseable: bool,

        /// Only show gems with major version updates
        #[arg(long, conflicts_with_all = ["minor", "patch"])]
        major: bool,

        /// Only show gems with minor version updates
        #[arg(long, conflicts_with_all = ["major", "patch"])]
        minor: bool,

        /// Only show gems with patch version updates
        #[arg(long, conflicts_with_all = ["major", "minor"])]
        patch: bool,

        /// Include prerelease versions in available versions
        #[arg(long)]
        pre: bool,

        /// Only check gems from a specific group
        #[arg(long)]
        group: Option<String>,
    },

    /// Open a gem's source code in your editor
    Open {
        /// Name of the gem
        gem: String,

        /// Specify GEM source relative path to open
        #[arg(long)]
        path: Option<String>,
    },

    /// Regenerate Gemfile.lock from Gemfile
    Lock {
        /// Path to Gemfile
        #[arg(long, default_value = "Gemfile")]
        gemfile: String,

        /// Path to lockfile (defaults to Gemfile.lock or gems.locked)
        #[arg(long)]
        lockfile: Option<String>,

        /// Add a platform to the lockfile
        #[arg(long = "add-platform")]
        add_platform: Vec<String>,

        /// Remove a platform from the lockfile
        #[arg(long = "remove-platform")]
        remove_platform: Vec<String>,

        /// Unlock specified gems for update (allows version changes)
        /// When no gems specified, updates all gems; when gems specified, updates only those
        #[arg(long, num_args(0..))]
        update: Vec<String>,

        /// Print lockfile to stdout instead of writing to file
        #[arg(long)]
        print: bool,

        /// Enable verbose output
        #[arg(short, long)]
        verbose: bool,

        /// Prefer updating only to next patch version
        #[arg(long, conflicts_with_all = ["minor", "major"])]
        patch: bool,

        /// Prefer updating only to next minor version
        #[arg(long, conflicts_with_all = ["patch", "major"])]
        minor: bool,

        /// Prefer updating to next major version (default)
        #[arg(long, conflicts_with_all = ["patch", "minor"])]
        major: bool,

        /// Do not allow any gem to be updated past latest patch/minor/major
        #[arg(long)]
        strict: bool,

        /// Use conservative update behavior (don't update shared dependencies)
        #[arg(long)]
        conservative: bool,

        /// Do not attempt to connect to rubygems.org (use cached gems only)
        #[arg(long)]
        local: bool,

        /// Allow prerelease versions when updating
        #[arg(long)]
        pre: bool,

        /// Update locked Bundler version (uses current lode version if no version specified)
        #[arg(long)]
        bundler: Option<String>,

        /// Normalize platform names in lockfile
        #[arg(long)]
        normalize_platforms: bool,

        /// Add checksums to lockfile for verification
        #[arg(long)]
        add_checksums: bool,

        /// Use full gem index instead of dependency API
        #[arg(long)]
        full_index: bool,

        /// Quiet output (suppress messages)
        #[arg(long, short = 'q')]
        quiet: bool,
    },

    /// Create a new Gemfile
    Init {
        /// Path where the Gemfile should be created
        #[arg(default_value = ".")]
        path: String,

        /// Generate Gemfile from .gemspec file
        #[arg(long)]
        gemspec: bool,
    },

    /// Generate a new gem project skeleton
    Gem {
        /// Name of the gem to create
        name: String,

        /// Create an executable in exe/
        #[arg(long, short = 'b', alias = "bin")]
        exe: bool,

        /// Add MIT license (default: true)
        #[arg(long, conflicts_with = "no_mit")]
        mit: bool,

        /// Do not include a license
        #[arg(long, conflicts_with = "mit")]
        no_mit: bool,

        /// Generate test files (rspec, minitest, test-unit)
        #[arg(long, short = 't')]
        test: Option<String>,
    },

    /// Display platform compatibility information
    Platform {
        /// Display Ruby version from environment
        #[arg(long)]
        ruby: bool,
    },

    /// Manage Bundler plugins
    Plugin {
        #[command(subcommand)]
        subcommand: PluginCommands,
    },

    /// Remove unused gems from vendor directory
    Clean {
        /// Path to vendor directory
        #[arg(long)]
        vendor: Option<String>,

        /// Show what would be removed without actually removing
        #[arg(long)]
        dry_run: bool,

        /// Skip confirmation prompts
        #[arg(long)]
        force: bool,
    },

    /// Diagnose common Bundler problems
    Doctor {
        /// Path to Gemfile
        #[arg(long)]
        gemfile: Option<String>,

        /// Only output warnings and errors
        #[arg(long)]
        quiet: bool,
    },

    /// Remove gems from Gemfile
    Remove {
        /// Name(s) of gem(s) to remove
        gems: Vec<String>,

        /// Quiet output (suppress messages)
        #[arg(long, short = 'q')]
        quiet: bool,
    },

    /// List all gems in the current bundle
    List {
        /// Print only gem names (one per line)
        #[arg(long)]
        name_only: bool,

        /// Show installation paths for each gem
        #[arg(long)]
        paths: bool,

        /// Only list gems from a specific group
        #[arg(long, conflicts_with = "without_group")]
        only_group: Option<String>,

        /// Exclude gems from specific groups (comma-separated)
        #[arg(long, conflicts_with = "only_group")]
        without_group: Option<String>,
    },

    /// Show detailed information about a gem
    Info {
        /// Name of the gem
        gem: String,

        /// Show gem installation path instead of metadata
        #[arg(long)]
        path: bool,

        /// Print gem version
        #[arg(long)]
        version: bool,
    },

    /// Search for gems on RubyGems.org
    Search {
        /// Search query
        query: String,
    },

    /// Display full gemspec metadata
    Specification {
        /// Name of the gem
        gem: String,

        /// Specific version (uses lockfile if not specified)
        #[arg(long)]
        version: Option<String>,
    },

    /// Find the location of a required library file
    Which {
        /// File name to search for (e.g., "rake", "rack.rb")
        file: String,
    },

    /// List all files in an installed gem
    Contents {
        /// Name of the gem
        gems: Vec<String>,

        /// Specify version of gem to contents
        #[arg(short = 'v', long)]
        version: Option<String>,

        /// Contents for all gems
        #[arg(long)]
        all: bool,

        /// Search for gems under specific paths
        #[arg(short = 's', long = "spec-dir")]
        spec_dir: Vec<String>,

        /// Only return files in the Gem's `lib_dirs`
        #[arg(short = 'l', long)]
        lib_only: bool,

        /// Don't include installed path prefix
        #[arg(long)]
        prefix: bool,

        /// Show only the gem install dir
        #[arg(long = "show-install-dir")]
        show_install_dir: bool,

        /// Verbose output
        #[arg(short = 'V', long)]
        verbose: bool,

        /// Quiet mode (suppress progress)
        #[arg(short = 'q', long)]
        quiet: bool,

        /// Silent mode (no output)
        #[arg(long)]
        silent: bool,

        /// Config file path (overrides default)
        #[arg(long)]
        config_file: Option<String>,

        /// Show stack backtrace on errors
        #[arg(long)]
        backtrace: bool,

        /// Turn on Ruby debugging
        #[arg(long)]
        debug: bool,

        /// Avoid loading any .gemrc file
        #[arg(long)]
        norc: bool,
    },

    /// Extract gem source to current directory
    Unpack {
        /// Name of the gem
        gem: String,

        /// Specific version (uses lockfile if not specified)
        #[arg(long)]
        version: Option<String>,

        /// Target directory (uses current directory if not specified)
        #[arg(long)]
        target: Option<String>,

        /// Unpack the gem specification
        #[arg(long)]
        spec: bool,

        /// Gem trust policy for security verification
        #[arg(short = 'P', long)]
        trust_policy: Option<String>,

        /// Verbose output
        #[arg(short = 'V', long)]
        verbose: bool,

        /// Quiet mode (suppress output)
        #[arg(short = 'q', long, conflicts_with = "verbose")]
        quiet: bool,

        /// Silent mode (no output)
        #[arg(long, conflicts_with_all = ["verbose", "quiet"])]
        silent: bool,

        /// Config file path (overrides default)
        #[arg(long = "config-file")]
        config_file: Option<String>,

        /// Show stack backtrace on errors
        #[arg(long)]
        backtrace: bool,

        /// Turn on Ruby debugging
        #[arg(long)]
        debug: bool,

        /// Avoid loading any .gemrc file
        #[arg(long)]
        norc: bool,
    },

    /// Show environment information
    Env,

    /// Restore gems to pristine condition
    Pristine {
        /// Specific gems to restore (restores all if not specified)
        gems: Vec<String>,

        /// Path to Gemfile.lock
        #[arg(long, default_value = "Gemfile.lock")]
        lockfile: String,

        /// Path to installed gems
        #[arg(long)]
        vendor: Option<String>,

        /// Verbose output
        #[arg(short = 'V', long)]
        verbose: bool,

        /// Quiet mode (suppress output)
        #[arg(short = 'q', long, conflicts_with = "verbose")]
        quiet: bool,

        /// Silent mode (no output)
        #[arg(long, conflicts_with_all = ["verbose", "quiet"])]
        silent: bool,

        /// Config file path (overrides default)
        #[arg(long = "config-file")]
        config_file: Option<String>,

        /// Show stack backtrace on errors
        #[arg(long)]
        backtrace: bool,

        /// Turn on Ruby debugging
        #[arg(long)]
        debug: bool,

        /// Avoid loading any .gemrc file
        #[arg(long)]
        norc: bool,
    },

    /// Generate shell completion scripts
    Completion {
        /// Shell to generate completion for
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },

    /// Install a gem
    #[command(name = "gem-install")]
    GemInstall {
        /// Gem name(s) to install
        #[arg(required = true)]
        gems: Vec<String>,

        // Basic Options
        /// Specify the platform of gem to install
        #[arg(long)]
        platform: Option<String>,

        /// Specify version of gem to install
        #[arg(short = 'v', long)]
        version: Option<String>,

        /// Allow prerelease versions of a gem to be installed
        #[arg(long, overrides_with = "no_prerelease")]
        prerelease: bool,

        /// Do not allow prerelease versions (negation of --prerelease)
        #[arg(long)]
        no_prerelease: bool,

        /// Update local gem source cache before installing
        #[arg(short = 'u', long, overrides_with = "no_update_sources")]
        update_sources: bool,

        /// Do not update local gem source cache (negation of --update-sources)
        #[arg(long = "no-update-sources", overrides_with = "update_sources")]
        no_update_sources: bool,

        // Install/Update Options
        /// Gem repository directory to get installed gems
        #[arg(short = 'i', long)]
        install_dir: Option<String>,

        /// Directory where executables will be placed when the gem is installed
        #[arg(short = 'n', long)]
        bindir: Option<String>,

        /// Generate documentation for installed gems (rdoc,ri)
        #[arg(long)]
        document: Option<String>,

        /// Disable documentation generation
        #[arg(short = 'N', long)]
        no_document: bool,

        /// Temporary installation root
        #[arg(long)]
        build_root: Option<String>,

        /// Install gem into the vendor directory
        #[arg(long)]
        vendor: bool,

        /// Rewrite the shebang line on installed scripts to use /usr/bin/env
        #[arg(short = 'E', long, overrides_with = "no_env_shebang")]
        env_shebang: bool,

        /// Do not rewrite the shebang line (negation of --env-shebang)
        #[arg(long, hide = true)]
        no_env_shebang: bool,

        /// Force gem to install, bypassing dependency checks
        #[arg(short = 'f', long, overrides_with = "no_force")]
        force: bool,

        /// Do not force installation (negation of --force)
        #[arg(long, hide = true)]
        no_force: bool,

        /// Use bin wrappers for executables
        #[arg(short = 'w', long, overrides_with = "no_wrappers")]
        wrappers: bool,

        /// Do not use bin wrappers (negation of --wrappers)
        #[arg(long, hide = true)]
        no_wrappers: bool,

        /// Specify gem trust policy
        #[arg(short = 'P', long)]
        trust_policy: Option<String>,

        /// Do not install any required dependent gems
        #[arg(long)]
        ignore_dependencies: bool,

        /// Make installed executable names match Ruby
        #[arg(long, overrides_with = "no_format_executable")]
        format_executable: bool,

        /// Do not make executable names match Ruby (negation of --format-executable)
        #[arg(long, hide = true)]
        no_format_executable: bool,

        /// Install in user's home directory instead of `GEM_HOME`
        #[arg(long, overrides_with = "no_user_install")]
        user_install: bool,

        /// Do not install in user's home directory (negation of --user-install)
        #[arg(long, hide = true)]
        no_user_install: bool,

        /// Install additional development dependencies
        #[arg(long)]
        development: bool,

        /// Install development dependencies for all gems
        #[arg(long)]
        development_all: bool,

        /// Don't attempt to upgrade gems already meeting version requirement
        #[arg(long)]
        conservative: bool,

        /// Don't upgrade any dependencies that already meet version requirements
        #[arg(long, overrides_with = "no_minimal_deps")]
        minimal_deps: bool,

        /// Do upgrade dependencies that don't meet version requirements (negation of --minimal-deps)
        #[arg(long)]
        no_minimal_deps: bool,

        /// Print post install message
        #[arg(long, overrides_with = "no_post_install_message")]
        post_install_message: bool,

        /// Do not print post install message (negation of --post-install-message)
        #[arg(long)]
        no_post_install_message: bool,

        /// Read from a gem dependencies API file and install the listed gems
        #[arg(short = 'g', long)]
        file: Option<String>,

        /// Omit the named groups (comma separated) when installing from a gem dependencies file
        #[arg(long)]
        without: Option<String>,

        /// Rather than install the gems, indicate which would be installed
        #[arg(long)]
        explain: bool,

        /// Create a lock file (when used with -g/--file)
        #[arg(long)]
        lock: bool,

        /// Suggest alternates when gems are not found
        #[arg(long, overrides_with = "no_suggestions")]
        suggestions: bool,

        /// Do not suggest alternates (negation of --suggestions)
        #[arg(long)]
        no_suggestions: bool,

        /// rbconfig.rb for the deployment target platform
        #[arg(long)]
        target_rbconfig: Option<String>,

        /// Add gem's full specification to default gems
        #[arg(long)]
        default: bool,

        /// Flags to pass to the build command
        #[arg(long)]
        build_flags: Option<String>,

        /// Ruby version (for cross-compilation)
        #[arg(long)]
        ruby: Option<String>,

        /// Library path for extensions
        #[arg(long)]
        with_extension_lib: Option<String>,

        // Local/Remote Options
        /// Restrict operations to the LOCAL domain
        #[arg(short = 'l', long)]
        local: bool,

        /// Restrict operations to the REMOTE domain
        #[arg(short = 'r', long)]
        remote: bool,

        /// Allow LOCAL and REMOTE operations
        #[arg(short = 'b', long)]
        both: bool,

        /// Threshold for switching to bulk synchronization
        #[arg(short = 'B', long)]
        bulk_threshold: Option<usize>,

        /// Clear the gem sources
        #[arg(long)]
        clear_sources: bool,

        /// Append URL to list of remote gem sources
        #[arg(short = 's', long)]
        source: Option<String>,

        /// Use HTTP proxy for remote operations (optional: specify URL or use environment variable)
        #[arg(short = 'p', long, num_args = 0..=1, default_missing_value = "", overrides_with = "no_http_proxy")]
        http_proxy: Option<String>,

        /// Do not use HTTP proxy (negation of --http-proxy)
        #[arg(long)]
        no_http_proxy: bool,

        // Common Options
        /// Set the verbose level of output
        #[arg(short = 'V', long)]
        verbose: bool,

        /// Silence command progress meter
        #[arg(short = 'q', long)]
        quiet: bool,

        /// Silence `RubyGems` output
        #[arg(long)]
        silent: bool,

        /// Config file path (overrides default)
        #[arg(long)]
        config_file: Option<String>,

        /// Show stack backtrace on errors
        #[arg(long)]
        backtrace: bool,

        /// Turn on Ruby debugging
        #[arg(long)]
        debug: bool,

        /// Avoid loading any .gemrc file
        #[arg(long)]
        norc: bool,
    },

    /// Uninstall a gem
    #[command(name = "gem-uninstall")]
    GemUninstall {
        /// Gem name(s) to uninstall
        #[arg(required = true)]
        gems: Vec<String>,

        /// Uninstall all matching versions
        #[arg(short = 'a', long)]
        all: bool,

        /// Ignore dependency requirements while uninstalling
        #[arg(short = 'I', long)]
        ignore_dependencies: bool,

        /// Check development dependencies while uninstalling
        #[arg(short = 'D', long)]
        check_development: bool,

        /// Uninstall applicable executables without confirmation
        #[arg(short = 'x', long)]
        executables: bool,

        /// Directory to uninstall gem from
        #[arg(short = 'i', long = "install-dir")]
        install_dir: Option<String>,

        /// Directory to remove executables from
        #[arg(short = 'n', long)]
        bindir: Option<String>,

        /// Uninstall from user's home directory in addition to `GEM_HOME`
        #[arg(long, overrides_with = "no_user_install")]
        user_install: bool,

        /// Do not uninstall from user's home directory (negation of --user-install)
        #[arg(long = "no-user-install", overrides_with = "user_install")]
        no_user_install: bool,

        /// Assume executable names match Ruby's prefix and suffix
        #[arg(long, overrides_with = "no_format_executable")]
        format_executable: bool,

        /// Do not assume executable names match (negation of --format-executable)
        #[arg(long = "no-format-executable", overrides_with = "format_executable")]
        no_format_executable: bool,

        /// Uninstall all versions of the named gems ignoring dependencies
        #[arg(long, overrides_with = "no_force")]
        force: bool,

        /// Do not force uninstallation (negation of --force)
        #[arg(long, hide = true)]
        no_force: bool,

        /// Prevent uninstalling gems that are depended on by other gems
        #[arg(long, overrides_with = "no_abort_on_dependent")]
        abort_on_dependent: bool,

        /// Do not prevent uninstalling dependent gems (negation of --abort-on-dependent)
        #[arg(long = "no-abort-on-dependent", overrides_with = "abort_on_dependent")]
        no_abort_on_dependent: bool,

        /// Specify version of gem to uninstall
        #[arg(short = 'v', long)]
        version: Option<String>,

        /// Specify the platform of gem to uninstall
        #[arg(long)]
        platform: Option<String>,

        /// Uninstall gem from the vendor directory
        #[arg(long)]
        vendor: bool,

        // Common flags
        /// Verbose output
        #[arg(short = 'V', long)]
        verbose: bool,

        /// Quiet mode (suppress output)
        #[arg(short = 'q', long, conflicts_with = "verbose")]
        quiet: bool,

        /// Silent mode (no output)
        #[arg(long, conflicts_with_all = ["verbose", "quiet"])]
        silent: bool,

        /// Config file path (overrides default)
        #[arg(long = "config-file")]
        config_file: Option<String>,

        /// Show stack backtrace on errors
        #[arg(long)]
        backtrace: bool,

        /// Turn on Ruby debugging
        #[arg(long)]
        debug: bool,

        /// Avoid loading any .gemrc file
        #[arg(long)]
        norc: bool,
    },

    /// Update installed gems
    #[command(name = "gem-update")]
    GemUpdate {
        /// Gem name(s) to update (updates all if not specified)
        #[arg(required = false)]
        gems: Vec<String>,

        /// Update the `RubyGems` system software
        #[arg(long)]
        system: bool,

        /// Specify the platform of gem to update
        #[arg(long)]
        platform: Option<String>,

        /// Allow prerelease versions of a gem as update targets
        #[arg(long, overrides_with = "no_prerelease")]
        prerelease: bool,

        /// Do not allow prerelease versions (negation of --prerelease)
        #[arg(long, hide = true)]
        no_prerelease: bool,

        /// Directory where executables will be placed when the gem is installed
        #[arg(short = 'n', long)]
        bindir: Option<String>,

        /// Temporary installation root
        #[arg(long)]
        build_root: Option<String>,

        /// Gem repository directory to get installed gems
        #[arg(short = 'i', long = "install-dir")]
        install_dir: Option<String>,

        /// Generate documentation for installed gems (rdoc,ri)
        #[arg(long, overrides_with = "no_document")]
        document: Option<String>,

        /// Disable documentation generation (negation of --document)
        #[arg(short = 'N', long, hide = true)]
        no_document: bool,

        /// Install gem into the vendor directory
        #[arg(long)]
        vendor: bool,

        /// Rewrite the shebang line on installed scripts to use /usr/bin/env
        #[arg(short = 'E', long, overrides_with = "no_env_shebang")]
        env_shebang: bool,

        /// Do not rewrite the shebang line (negation of --env-shebang)
        #[arg(long, hide = true)]
        no_env_shebang: bool,

        /// Use bin wrappers for executables
        #[arg(short = 'w', long, overrides_with = "no_wrappers")]
        wrappers: bool,

        /// Do not use bin wrappers (negation of --wrappers)
        #[arg(long, hide = true)]
        no_wrappers: bool,

        /// Make installed executable names match Ruby
        #[arg(long, overrides_with = "no_format_executable")]
        format_executable: bool,

        /// Do not make executable names match Ruby (negation of --format-executable)
        #[arg(long, hide = true)]
        no_format_executable: bool,

        /// Install in user's home directory instead of `GEM_HOME`
        #[arg(long, overrides_with = "no_user_install")]
        user_install: bool,

        /// Do not install in user's home directory (negation of --user-install)
        #[arg(long, hide = true)]
        no_user_install: bool,

        /// Print post install message
        #[arg(long, overrides_with = "no_post_install_message")]
        post_install_message: bool,

        /// Do not print post install message (negation of --post-install-message)
        #[arg(long, hide = true)]
        no_post_install_message: bool,

        /// Restrict operations to the LOCAL domain
        #[arg(short = 'l', long)]
        local: bool,

        /// Restrict operations to the REMOTE domain
        #[arg(short = 'r', long, conflicts_with = "local")]
        remote: bool,

        /// Allow LOCAL and REMOTE operations
        #[arg(short = 'b', long, conflicts_with_all = ["local", "remote"])]
        both: bool,

        /// Bulk synchronization threshold (default: 1000)
        #[arg(short = 'B', long = "bulk-threshold")]
        bulk_threshold: Option<usize>,

        /// Clear gem sources
        #[arg(long)]
        clear_sources: bool,

        /// Append URL to list of remote gem sources
        #[arg(short = 's', long)]
        source: Option<String>,

        /// Use HTTP proxy for remote operations (optional: specify URL or use environment variable)
        #[arg(short = 'p', long, num_args = 0..=1, default_missing_value = "", overrides_with = "no_http_proxy")]
        http_proxy: Option<String>,

        /// Do not use HTTP proxy (negation of --http-proxy)
        #[arg(long, hide = true)]
        no_http_proxy: bool,

        /// Force gem to install, bypassing dependency checks
        #[arg(short = 'f', long, overrides_with = "no_force")]
        force: bool,

        /// Do not force installation (negation of --force)
        #[arg(long, hide = true)]
        no_force: bool,

        /// Do not install any required dependent gems
        #[arg(long)]
        ignore_dependencies: bool,

        /// Don't upgrade any dependencies that already meet version requirements
        #[arg(long, overrides_with = "no_minimal_deps")]
        minimal_deps: bool,

        /// Do upgrade dependencies that don't meet version requirements (negation of --minimal-deps)
        #[arg(long)]
        no_minimal_deps: bool,

        /// Don't attempt to upgrade gems already meeting version requirement
        #[arg(long)]
        conservative: bool,

        /// Install additional development dependencies
        #[arg(long)]
        development: bool,

        /// Install development dependencies for all gems
        #[arg(long)]
        development_all: bool,

        /// Add gem's full specification to default gems
        #[arg(long)]
        default: bool,

        /// Specify gem trust policy
        #[arg(short = 'P', long)]
        trust_policy: Option<String>,

        /// rbconfig.rb for the deployment target platform
        #[arg(long)]
        target_rbconfig: Option<String>,

        /// Read from a gem dependencies API file and install the listed gems
        #[arg(short = 'g', long)]
        file: Option<String>,

        /// Omit the named groups (comma separated) when installing from a gem dependencies file
        #[arg(long)]
        without: Option<String>,

        /// Rather than install the gems, indicate which would be installed
        #[arg(long)]
        explain: bool,

        /// Create a lock file (when used with -g/--file)
        #[arg(long, overrides_with = "no_lock")]
        lock: bool,

        /// Do not create a lock file (negation of --lock)
        #[arg(long, hide = true)]
        no_lock: bool,

        /// Suggest alternates when gems are not found
        #[arg(long, overrides_with = "no_suggestions")]
        suggestions: bool,

        /// Do not suggest alternates (negation of --suggestions)
        #[arg(long, hide = true)]
        no_suggestions: bool,

        // Common flags
        /// Set the verbose level of output
        #[arg(short = 'V', long, overrides_with = "no_verbose")]
        verbose: bool,

        /// Do not set verbose output (negation of --verbose)
        #[arg(long, hide = true)]
        no_verbose: bool,

        /// Quiet mode (suppress output)
        #[arg(short = 'q', long, conflicts_with = "verbose")]
        quiet: bool,

        /// Silent mode (no output)
        #[arg(long, conflicts_with_all = ["verbose", "quiet"])]
        silent: bool,

        /// Config file path (overrides default)
        #[arg(long = "config-file")]
        config_file: Option<String>,

        /// Show stack backtrace on errors
        #[arg(long)]
        backtrace: bool,

        /// Turn on Ruby debugging
        #[arg(long)]
        debug: bool,

        /// Avoid loading any .gemrc file
        #[arg(long)]
        norc: bool,
    },

    /// List installed gems
    #[command(name = "gem-list")]
    GemList {
        /// Filter gems by name pattern (supports regex)
        pattern: Option<String>,

        // Query flags
        /// Check if gem is installed (returns exit code 0 if installed)
        #[arg(short = 'i', long)]
        installed: bool,

        /// Equivalent to --no-installed
        #[arg(short = 'I', conflicts_with = "installed")]
        not_installed: bool,

        /// Specify version of gem to check (use with --installed)
        #[arg(short = 'v', long)]
        version: Option<String>,

        /// Display detailed information (summary, homepage, author, locations)
        #[arg(short = 'd', long, overrides_with = "no_details")]
        details: bool,

        /// Do not display detailed information (negation of --details)
        #[arg(long, hide = true)]
        no_details: bool,

        /// Display only gem names (no versions)
        #[arg(long)]
        versions: bool,

        /// Display all gem versions (not just latest)
        #[arg(short = 'a', long)]
        all: bool,

        /// Exact name match (no partial matches)
        #[arg(short = 'e', long)]
        exact: bool,

        /// Include prerelease versions
        #[arg(long, overrides_with = "no_prerelease")]
        prerelease: bool,

        /// Do not include prerelease versions (negation of --prerelease)
        #[arg(long)]
        no_prerelease: bool,

        /// Update local gem source cache before listing
        #[arg(short = 'u', long, overrides_with = "no_update_sources")]
        update_sources: bool,

        /// Do not update local gem source cache (negation of --update-sources)
        #[arg(long = "no-update-sources", overrides_with = "update_sources")]
        no_update_sources: bool,

        // Local/Remote flags
        /// List local gems only (default)
        #[arg(short = 'l', long, conflicts_with_all = ["remote", "both"])]
        local: bool,

        /// List remote gems from RubyGems.org
        #[arg(short = 'r', long, conflicts_with_all = ["local", "both"])]
        remote: bool,

        /// List both local and remote gems
        #[arg(short = 'b', long, conflicts_with_all = ["local", "remote"])]
        both: bool,

        /// Bulk synchronization threshold (default: 1000)
        #[arg(short = 'B', long = "bulk-threshold", default_value = "1000")]
        bulk_threshold: usize,

        /// Clear gem sources
        #[arg(long)]
        clear_sources: bool,

        /// Append URL to list of remote gem sources
        #[arg(short = 's', long)]
        source: Option<String>,

        /// Use HTTP proxy for remote operations (optional: specify URL or use environment variable)
        #[arg(short = 'p', long = "http-proxy", num_args = 0..=1, default_missing_value = "", overrides_with = "no_http_proxy")]
        http_proxy: Option<String>,

        /// Do not use HTTP proxy (negation of --http-proxy)
        #[arg(long)]
        no_http_proxy: bool,

        // Common flags
        /// Verbose output
        #[arg(short = 'V', long)]
        verbose: bool,

        /// Quiet mode (suppress output)
        #[arg(short = 'q', long, conflicts_with = "verbose")]
        quiet: bool,

        /// Silent mode (no output)
        #[arg(long, conflicts_with_all = ["verbose", "quiet"])]
        silent: bool,

        /// Config file path (overrides default)
        #[arg(long = "config-file")]
        config_file: Option<String>,

        /// Show stack backtrace on errors
        #[arg(long)]
        backtrace: bool,

        /// Turn on Ruby debugging
        #[arg(long)]
        debug: bool,

        /// Avoid loading any .gemrc file
        #[arg(long)]
        norc: bool,
    },

    /// Search for gems on RubyGems.org
    #[command(name = "gem-search")]
    GemSearch {
        /// Search query (REGEXP pattern)
        query: Option<String>,

        // Query flags
        /// Check if gem is installed (returns exit code 0 if installed)
        #[arg(short = 'i', long)]
        installed: bool,

        /// Equivalent to --no-installed
        #[arg(short = 'I', conflicts_with = "installed")]
        not_installed: bool,

        /// Specify version of gem to search for use with --installed
        #[arg(short = 'v', long)]
        version: Option<String>,

        /// Display detailed information (summary, homepage, author, locations)
        #[arg(short = 'd', long, overrides_with = "no_details")]
        details: bool,

        /// Do not display detailed information (negation of --details)
        #[arg(long, hide = true)]
        no_details: bool,

        /// Display only gem names (no versions)
        #[arg(long)]
        versions: bool,

        /// Display all gem versions (not just latest)
        #[arg(short = 'a', long)]
        all: bool,

        /// Exact name match (no partial matches)
        #[arg(short = 'e', long)]
        exact: bool,

        /// Include prerelease versions
        #[arg(long, overrides_with = "no_prerelease")]
        prerelease: bool,

        /// Do not include prerelease versions (negation of --prerelease)
        #[arg(long)]
        no_prerelease: bool,

        /// Update local gem source cache before searching
        #[arg(short = 'u', long, overrides_with = "no_update_sources")]
        update_sources: bool,

        /// Do not update local gem source cache (negation of --update-sources)
        #[arg(long = "no-update-sources", overrides_with = "update_sources")]
        no_update_sources: bool,

        // Local/Remote flags
        /// List local gems only (default)
        #[arg(short = 'l', long, conflicts_with_all = ["remote", "both"])]
        local: bool,

        /// List remote gems from RubyGems.org
        #[arg(short = 'r', long, conflicts_with_all = ["local", "both"])]
        remote: bool,

        /// List both local and remote gems
        #[arg(short = 'b', long, conflicts_with_all = ["local", "remote"])]
        both: bool,

        /// Bulk synchronization threshold (default: 1000)
        #[arg(short = 'B', long = "bulk-threshold", default_value = "1000")]
        bulk_threshold: usize,

        /// Clear gem sources
        #[arg(long)]
        clear_sources: bool,

        /// Append URL to list of remote gem sources
        #[arg(short = 's', long)]
        source: Option<String>,

        /// Use HTTP proxy for remote operations (optional: specify URL or use environment variable)
        #[arg(short = 'p', long = "http-proxy", num_args = 0..=1, default_missing_value = "", overrides_with = "no_http_proxy")]
        http_proxy: Option<String>,

        /// Do not use HTTP proxy (negation of --http-proxy)
        #[arg(long)]
        no_http_proxy: bool,

        // Common flags
        /// Verbose output
        #[arg(short = 'V', long)]
        verbose: bool,

        /// Quiet mode (suppress output)
        #[arg(short = 'q', long, conflicts_with = "verbose")]
        quiet: bool,

        /// Silent mode (no output)
        #[arg(long, conflicts_with_all = ["verbose", "quiet"])]
        silent: bool,

        /// Config file path (overrides default)
        #[arg(long = "config-file")]
        config_file: Option<String>,

        /// Show stack backtrace on errors
        #[arg(long)]
        backtrace: bool,

        /// Turn on Ruby debugging
        #[arg(long)]
        debug: bool,

        /// Avoid loading any .gemrc file
        #[arg(long)]
        norc: bool,
    },

    /// Build a gem from a gemspec
    #[command(name = "gem-build")]
    GemBuild {
        /// Gemspec file to build
        gemspec: Option<String>,

        /// Specify the platform of gem to build
        #[arg(long)]
        platform: Option<String>,

        /// Skip validation of the spec
        #[arg(long)]
        force: bool,

        /// Consider warnings as errors when validating the spec
        #[arg(long)]
        strict: bool,

        /// Output gem with the given filename
        #[arg(short = 'o', long)]
        output: Option<String>,

        /// Run as if gem build was started in <PATH> instead of the current working directory
        #[arg(short = 'C')]
        directory: Option<String>,

        // Common flags
        /// Verbose output
        #[arg(short = 'V', long)]
        verbose: bool,

        /// Quiet mode (suppress output)
        #[arg(short = 'q', long, conflicts_with = "verbose")]
        quiet: bool,

        /// Silent mode (no output)
        #[arg(long, conflicts_with_all = ["verbose", "quiet"])]
        silent: bool,

        /// Config file path (overrides default)
        #[arg(long = "config-file")]
        config_file: Option<String>,

        /// Show stack backtrace on errors
        #[arg(long)]
        backtrace: bool,

        /// Turn on Ruby debugging
        #[arg(long)]
        debug: bool,

        /// Avoid loading any .gemrc file
        #[arg(long)]
        norc: bool,
    },

    /// Push a gem to `RubyGems`
    #[command(name = "gem-push")]
    GemPush {
        /// Gem file to push
        gem: String,
        /// Use the given API key from ~/.gem/credentials
        #[arg(short = 'k', long)]
        key: Option<String>,
        /// Digit code for multifactor authentication
        #[arg(long)]
        otp: Option<String>,
        /// Push to another gemcutter-compatible host
        #[arg(long)]
        host: Option<String>,
        /// Push with sigstore attestations
        #[arg(long)]
        attestation: Option<String>,
        /// Use HTTP proxy for remote operations (optional: specify URL or use environment variable)
        #[arg(short = 'p', long = "http-proxy", num_args = 0..=1, default_missing_value = "", overrides_with = "no_http_proxy")]
        http_proxy: Option<String>,
        /// Do not use HTTP proxy (negation of --http-proxy)
        #[arg(long)]
        no_http_proxy: bool,
        /// Set the verbose level of output
        #[arg(short = 'V', long)]
        verbose: bool,
        /// Silence command progress meter
        #[arg(short, long)]
        quiet: bool,
        /// Silence `RubyGems` output
        #[arg(long)]
        silent: bool,
        /// Config file path (overrides default)
        #[arg(long)]
        config_file: Option<String>,
        /// Show stack backtrace on errors
        #[arg(long)]
        backtrace: bool,
        /// Turn on Ruby debugging
        #[arg(long)]
        debug: bool,
        /// Avoid loading any .gemrc file
        #[arg(long)]
        norc: bool,
    },

    /// Yank a gem version from `RubyGems`
    #[command(name = "gem-yank")]
    GemYank {
        /// Gem name
        gem: String,

        /// Version of gem to yank
        #[arg(short = 'v', long)]
        version: String,

        /// Platform of gem to yank (e.g., ruby, java, x86_64-linux)
        #[arg(short = 'p', long)]
        platform: Option<String>,

        /// Digit code for multifactor authentication
        #[arg(long)]
        otp: Option<String>,

        /// Yank from another gemcutter-compatible host
        #[arg(long)]
        host: Option<String>,

        /// Use the given API key from ~/.gem/credentials
        #[arg(short = 'k', long)]
        key: Option<String>,

        /// Undo a yank, i.e. restore a previously yanked gem
        #[arg(long, hide = true)]
        undo: bool,

        /// Increase verbosity (enabled by default for yank output)
        #[arg(short = 'V', long)]
        verbose: bool,

        /// Suppress all output except errors
        #[arg(short, long, conflicts_with = "verbose")]
        quiet: bool,

        /// Suppress all `RubyGems` output
        #[arg(long)]
        silent: bool,

        /// Use the specified config file instead of default
        #[arg(long)]
        config_file: Option<String>,

        /// Show stack backtrace on errors
        #[arg(long)]
        backtrace: bool,

        /// Turn on Ruby debugging
        #[arg(long)]
        debug: bool,

        /// Avoid loading any .gemrc file
        #[arg(long)]
        norc: bool,
    },

    /// Manage gem ownership
    #[command(name = "gem-owner")]
    GemOwner {
        /// Gem name
        gem: String,

        /// Add an owner by user identifier (email or handle)
        #[arg(short = 'a', long)]
        add: Vec<String>,

        /// Remove an owner by user identifier (email or handle)
        #[arg(short = 'r', long)]
        remove: Vec<String>,

        /// Use the given API key from ~/.gem/credentials
        #[arg(short = 'k', long)]
        key: Option<String>,

        /// Digit code for multifactor authentication
        #[arg(long)]
        otp: Option<String>,

        /// Use another gemcutter-compatible host
        #[arg(long)]
        host: Option<String>,

        /// Use HTTP proxy for remote operations
        #[arg(short = 'p', long = "http-proxy", num_args = 0..=1, default_missing_value = "", overrides_with = "no_http_proxy")]
        http_proxy: Option<String>,

        /// Do not use HTTP proxy (negation of --http-proxy)
        #[arg(long, hide = true)]
        no_http_proxy: bool,

        /// Verbose output
        #[arg(short = 'V', long)]
        verbose: bool,

        /// Quiet mode (suppress output)
        #[arg(short = 'q', long)]
        quiet: bool,

        /// Silent mode (no output)
        #[arg(long)]
        silent: bool,

        /// Config file path (overrides default)
        #[arg(long)]
        config_file: Option<String>,

        /// Show stack backtrace on errors
        #[arg(long)]
        backtrace: bool,

        /// Turn on Ruby debugging
        #[arg(long)]
        debug: bool,

        /// Avoid loading any .gemrc file
        #[arg(long)]
        norc: bool,
    },

    /// Sign in to `RubyGems`
    #[command(name = "gem-signin")]
    GemSignin {
        /// Use another gemcutter-compatible host
        #[arg(long)]
        host: Option<String>,

        /// Verbose output
        #[arg(short = 'V', long)]
        verbose: bool,

        /// Quiet mode (suppress output)
        #[arg(short = 'q', long)]
        quiet: bool,

        /// Silent mode (no output)
        #[arg(long)]
        silent: bool,

        /// Config file path (overrides default)
        #[arg(long)]
        config_file: Option<String>,

        /// Show stack backtrace on errors
        #[arg(long)]
        backtrace: bool,

        /// Turn on Ruby debugging
        #[arg(long)]
        debug: bool,

        /// Avoid loading any .gemrc file
        #[arg(long)]
        norc: bool,
    },

    /// Sign out from `RubyGems`
    #[command(name = "gem-signout")]
    GemSignout {
        /// Verbose output
        #[arg(short = 'V', long)]
        verbose: bool,

        /// Quiet mode (suppress output)
        #[arg(short = 'q', long)]
        quiet: bool,

        /// Silent mode (no output)
        #[arg(long)]
        silent: bool,

        /// Config file path (overrides default)
        #[arg(long)]
        config_file: Option<String>,

        /// Show stack backtrace on errors
        #[arg(long)]
        backtrace: bool,

        /// Turn on Ruby debugging
        #[arg(long)]
        debug: bool,

        /// Avoid loading any .gemrc file
        #[arg(long)]
        norc: bool,
    },

    // Information & Maintenance
    /// Show gem information
    #[command(name = "gem-info")]
    GemInfo {
        /// Gem name
        gem: String,

        /// Check if gem is installed
        #[arg(short = 'i', long, action = clap::ArgAction::SetTrue, overrides_with = "no_installed")]
        installed: bool,

        /// Check if gem is NOT installed (-I)
        #[arg(short = 'I', long = "no-installed", overrides_with = "installed")]
        no_installed: bool,

        /// Gem version
        #[arg(short = 'v', long)]
        version: Option<String>,

        /// Display only gem names
        #[arg(long, overrides_with = "no_versions")]
        versions: bool,

        /// Do not display only gem names
        #[arg(long = "no-versions", overrides_with = "versions")]
        no_versions: bool,

        /// Display all versions
        #[arg(short = 'a', long)]
        all: bool,

        /// Exact name match
        #[arg(short = 'e', long)]
        exact: bool,

        /// Include prerelease versions
        #[arg(long, overrides_with = "no_prerelease")]
        prerelease: bool,

        /// Do not include prerelease versions
        #[arg(long = "no-prerelease", overrides_with = "prerelease")]
        no_prerelease: bool,

        /// Update local gem source cache before listing
        #[arg(short = 'u', long, overrides_with = "no_update_sources")]
        update_sources: bool,

        /// Do not update local gem source cache (negation of --update-sources)
        #[arg(long = "no-update-sources", overrides_with = "update_sources")]
        no_update_sources: bool,

        /// Local gems only
        #[arg(short = 'l', long)]
        local: bool,

        /// Remote gems only
        #[arg(short = 'r', long)]
        remote: bool,

        /// Both local and remote
        #[arg(short = 'b', long)]
        both: bool,

        /// Threshold for switching to bulk synchronization
        #[arg(short = 'B', long, value_name = "COUNT", default_value = "1000")]
        bulk_threshold: usize,

        /// Clear the gem sources
        #[arg(long)]
        clear_sources: bool,

        /// Append URL to list of remote gem sources
        #[arg(short = 's', long, value_name = "URL")]
        source: Option<String>,

        /// Use HTTP proxy for remote operations
        #[arg(short = 'p', long = "http-proxy", num_args = 0..=1, default_missing_value = "", overrides_with = "no_http_proxy")]
        http_proxy: Option<String>,

        /// Do not use HTTP proxy (negation of --http-proxy)
        #[arg(long, hide = true)]
        no_http_proxy: bool,

        /// Set the verbose level of output
        #[arg(short = 'V', long, overrides_with = "no_verbose")]
        verbose: bool,

        /// Do not set verbose output
        #[arg(long = "no-verbose", overrides_with = "verbose")]
        no_verbose: bool,

        /// Silence command progress meter
        #[arg(short = 'q', long)]
        quiet: bool,

        /// Silence `RubyGems` output
        #[arg(long)]
        silent: bool,

        /// Config file path (overrides default)
        #[arg(long, value_name = "FILE")]
        config_file: Option<String>,

        /// Show stack backtrace on errors
        #[arg(long)]
        backtrace: bool,

        /// Turn on Ruby debugging
        #[arg(long)]
        debug: bool,

        /// Avoid loading any .gemrc file
        #[arg(long)]
        norc: bool,
    },

    /// List files in an installed gem
    #[command(name = "gem-contents")]
    GemContents {
        /// Gem name
        gem: String,

        /// Specific version (uses latest if not specified)
        #[arg(short = 'v', long)]
        version: Option<String>,

        /// Contents for all gems
        #[arg(long)]
        all: bool,

        /// Search for gems under specific paths
        #[arg(short = 's', long = "spec-dir", value_delimiter = ',')]
        spec_dir: Option<Vec<String>>,

        /// Only return files in the Gem's `lib_dirs`
        #[arg(short = 'l', long = "lib-only")]
        lib_only: bool,

        /// Don't include installed path prefix
        #[arg(long)]
        prefix: bool,

        /// Don't include installed path prefix (negation)
        #[arg(long = "no-prefix", conflicts_with = "prefix")]
        no_prefix: bool,

        /// Show only the gem install dir
        #[arg(long = "show-install-dir")]
        show_install_dir: bool,

        /// Don't show the gem install dir
        #[arg(long = "no-show-install-dir", conflicts_with = "show_install_dir")]
        no_show_install_dir: bool,

        // Common flags
        /// Set the verbose level of output
        #[arg(short = 'V', long)]
        verbose: bool,

        /// Silence command progress meter
        #[arg(short = 'q', long, conflicts_with = "verbose")]
        quiet: bool,

        /// Silence `RubyGems` output
        #[arg(long, conflicts_with_all = ["verbose", "quiet"])]
        silent: bool,

        /// Config file path (overrides default)
        #[arg(long = "config-file")]
        config_file: Option<String>,

        /// Show stack backtrace on errors
        #[arg(long)]
        backtrace: bool,

        /// Turn on Ruby debugging
        #[arg(long)]
        debug: bool,

        /// Avoid loading any .gemrc file
        #[arg(long)]
        norc: bool,
    },

    /// Show gem dependencies
    #[command(name = "gem-dependency")]
    GemDependency {
        /// Gem name or pattern
        gem: String,

        /// Specific version
        #[arg(short = 'v', long)]
        version: Option<String>,

        /// Platform filter
        #[arg(long)]
        platform: Option<String>,

        /// Include prerelease versions
        #[arg(long)]
        prerelease: bool,

        /// Show reverse dependencies (which gems depend on this one)
        #[arg(short = 'R', long = "reverse-dependencies")]
        reverse_dependencies: bool,

        /// Pipe format output
        #[arg(long)]
        pipe: bool,

        // Local/Remote Options
        /// Restrict to locally installed gems only (default)
        #[arg(short = 'l', long)]
        local: bool,

        /// Restrict to remote gems only
        #[arg(short = 'r', long)]
        remote: bool,

        /// Include both local and remote gems
        #[arg(short = 'b', long)]
        both: bool,

        /// Threshold for switching to bulk synchronization (default 1000)
        #[arg(short = 'B', long = "bulk-threshold")]
        bulk_threshold: Option<usize>,

        /// Clear the gem sources
        #[arg(long = "clear-sources")]
        clear_sources: bool,

        /// Append URL to list of remote gem sources
        #[arg(short = 's', long)]
        source: Option<String>,

        /// Use HTTP proxy for remote operations
        #[arg(short = 'p', long = "http-proxy", num_args = 0..=1, default_missing_value = "", overrides_with = "no_http_proxy")]
        http_proxy: Option<String>,

        /// Do not use HTTP proxy (negation of --http-proxy)
        #[arg(long, hide = true)]
        no_http_proxy: bool,

        // Common flags
        /// Set the verbose level of output
        #[arg(short = 'V', long)]
        verbose: bool,

        /// Silence command progress meter
        #[arg(short = 'q', long, conflicts_with = "verbose")]
        quiet: bool,

        /// Silence `RubyGems` output
        #[arg(long, conflicts_with_all = ["verbose", "quiet"])]
        silent: bool,

        /// Config file path (overrides default)
        #[arg(long = "config-file")]
        config_file: Option<String>,

        /// Show stack backtrace on errors
        #[arg(long)]
        backtrace: bool,

        /// Turn on Ruby debugging
        #[arg(long)]
        debug: bool,

        /// Avoid loading any .gemrc file
        #[arg(long)]
        norc: bool,
    },

    /// Find the location of a library file you can require
    #[command(name = "gem-which")]
    GemWhich {
        /// File name(s) to find (e.g., rake, json, nokogiri)
        #[arg(required = true)]
        files: Vec<String>,

        /// Show all matching files (not just the first)
        #[arg(short, long)]
        all: bool,

        /// Search gems before non-gems
        #[arg(short = 'g', long)]
        gems_first: bool,

        /// Verbose output
        #[arg(short = 'V', long)]
        verbose: bool,

        /// Quiet mode (suppress progress meter)
        #[arg(short, long)]
        quiet: bool,

        /// Silent mode (suppress all output)
        #[arg(long)]
        silent: bool,

        /// Config file path (overrides default)
        #[arg(long)]
        config_file: Option<String>,

        /// Show stack backtrace on errors
        #[arg(long)]
        backtrace: bool,

        /// Turn on Ruby debugging
        #[arg(long)]
        debug: bool,

        /// Avoid loading any .gemrc file
        #[arg(long)]
        norc: bool,
    },

    /// Download a gem without installing it
    #[command(name = "gem-fetch")]
    GemFetch {
        /// Gem name to download
        gem: String,

        /// Gem version
        #[arg(long)]
        version: Option<String>,

        /// Download to specific directory
        #[arg(long)]
        output_dir: Option<String>,

        /// Specify the platform of gem to fetch
        #[arg(long)]
        platform: Option<String>,

        /// Allow prerelease versions
        #[arg(long)]
        prerelease: bool,

        /// Suggest alternates when gems not found
        #[arg(long)]
        suggestions: bool,

        /// Threshold for switching to bulk synchronization
        #[arg(short = 'B', long)]
        bulk_threshold: Option<u32>,

        /// Use HTTP proxy for remote operations
        #[arg(short = 'p', long = "http-proxy", num_args = 0..=1, default_missing_value = "", overrides_with = "no_http_proxy")]
        http_proxy: Option<String>,

        /// Do not use HTTP proxy (negation of --http-proxy)
        #[arg(long, hide = true)]
        no_http_proxy: bool,

        /// Append URL to list of remote gem sources
        #[arg(short = 's', long)]
        source: Option<String>,

        /// Clear the gem sources
        #[arg(long)]
        clear_sources: bool,

        /// Verbose output
        #[arg(short = 'V', long)]
        verbose: bool,

        /// Quiet mode (suppress output)
        #[arg(short = 'q', long)]
        quiet: bool,

        /// Silent mode (no output)
        #[arg(long)]
        silent: bool,

        /// Config file path (overrides default)
        #[arg(long)]
        config_file: Option<String>,

        /// Show stack backtrace on errors
        #[arg(long)]
        backtrace: bool,

        /// Turn on Ruby debugging
        #[arg(long)]
        debug: bool,

        /// Avoid loading any .gemrc file
        #[arg(long)]
        norc: bool,
    },

    /// List stale gems
    #[command(name = "gem-stale")]
    GemStale {
        // Common flags
        /// Verbose output
        #[arg(short = 'V', long)]
        verbose: bool,

        /// Quiet mode (suppress output)
        #[arg(short = 'q', long, conflicts_with = "verbose")]
        quiet: bool,

        /// Silent mode (no output)
        #[arg(long, conflicts_with_all = ["verbose", "quiet"])]
        silent: bool,

        /// Config file path (overrides default)
        #[arg(long = "config-file")]
        config_file: Option<String>,

        /// Show stack backtrace on errors
        #[arg(long)]
        backtrace: bool,

        /// Turn on Ruby debugging
        #[arg(long)]
        debug: bool,

        /// Avoid loading any .gemrc file
        #[arg(long)]
        norc: bool,
    },

    /// Clean up gem cache
    #[command(name = "gem-cleanup")]
    GemCleanup {
        /// Gem names to cleanup (cleans all if not specified)
        gems: Vec<String>,

        /// Do not uninstall gems (dry run)
        #[arg(short = 'n', short_alias = 'd', long)]
        dry_run: bool,

        /// Check development dependencies while uninstalling
        #[arg(short = 'D', long)]
        check_development: bool,

        /// Cleanup in user's home directory instead of `GEM_HOME`
        #[arg(long)]
        user_install: bool,

        // Common flags
        /// Verbose output
        #[arg(short = 'V', long)]
        verbose: bool,

        /// Quiet mode (suppress output)
        #[arg(short = 'q', long, conflicts_with = "verbose")]
        quiet: bool,

        /// Silent mode (no output)
        #[arg(long, conflicts_with_all = ["verbose", "quiet"])]
        silent: bool,

        /// Config file path (overrides default)
        #[arg(long = "config-file")]
        config_file: Option<String>,

        /// Show stack backtrace on errors
        #[arg(long)]
        backtrace: bool,

        /// Turn on Ruby debugging
        #[arg(long)]
        debug: bool,

        /// Avoid loading any .gemrc file
        #[arg(long)]
        norc: bool,
    },

    /// Restore original files in gem install
    #[command(name = "gem-pristine")]
    GemPristine {
        /// Gem names to restore (empty requires --all)
        gems: Vec<String>,

        /// Restore all installed gems to pristine condition
        #[arg(long)]
        all: bool,

        /// Skip gem names (used with --all)
        #[arg(long)]
        skip: Vec<String>,

        /// Restore gems with extensions in addition to regular gems
        #[arg(long)]
        extensions: bool,

        /// Only restore gems with missing extensions
        #[arg(long)]
        only_missing_extensions: bool,

        /// Only restore executables
        #[arg(long)]
        only_executables: bool,

        /// Only restore plugins
        #[arg(long)]
        only_plugins: bool,

        /// Rewrite executables with /usr/bin/env shebang
        #[arg(short = 'E', long)]
        env_shebang: bool,

        /// Gem repository to restore gems from
        #[arg(short = 'i', long)]
        install_dir: Option<String>,

        /// Directory where executables are located
        #[arg(short = 'n', long)]
        bindir: Option<String>,

        /// Specify version of gem to restore
        #[arg(short = 'v', long)]
        version: Option<String>,

        // Common flags
        /// Verbose output
        #[arg(short = 'V', long)]
        verbose: bool,

        /// Quiet mode (suppress output)
        #[arg(short = 'q', long, conflicts_with = "verbose")]
        quiet: bool,

        /// Silent mode (no output)
        #[arg(long, conflicts_with_all = ["verbose", "quiet"])]
        silent: bool,

        /// Config file path (overrides default)
        #[arg(long = "config-file")]
        config_file: Option<String>,

        /// Show stack backtrace on errors
        #[arg(long)]
        backtrace: bool,

        /// Turn on Ruby debugging
        #[arg(long)]
        debug: bool,

        /// Avoid loading any .gemrc file
        #[arg(long)]
        norc: bool,
    },

    /// Rebuild installed gems
    #[command(name = "gem-rebuild")]
    GemRebuild {
        /// Gem name
        gem: String,

        /// If the files don't match, compare them using diffoscope
        #[arg(long)]
        diff: bool,

        /// Skip validation of the spec
        #[arg(long)]
        force: bool,

        /// Consider warnings as errors when validating the spec
        #[arg(long)]
        strict: bool,

        /// Specify the source to download the gem from
        #[arg(long)]
        source: Option<String>,

        /// Specify a local file to compare against
        #[arg(long)]
        original: Option<String>,

        /// Specify the name of the gemspec file
        #[arg(long)]
        gemspec: Option<String>,

        /// Run as if gem build was started in PATH
        #[arg(short = 'C')]
        working_dir: Option<String>,

        /// Verbose output
        #[arg(short = 'V', long)]
        verbose: bool,

        /// Quiet mode (suppress output)
        #[arg(short = 'q', long)]
        quiet: bool,

        /// Silent mode (no output)
        #[arg(long)]
        silent: bool,

        /// Config file path (overrides default)
        #[arg(long)]
        config_file: Option<String>,

        /// Show stack backtrace on errors
        #[arg(long)]
        backtrace: bool,

        /// Turn on Ruby debugging
        #[arg(long)]
        debug: bool,

        /// Avoid loading any .gemrc file
        #[arg(long)]
        norc: bool,
    },

    // Configuration & Advanced
    /// Manage gem sources
    #[command(name = "gem-sources")]
    GemSources {
        /// Add source
        #[arg(short = 'a', long, conflicts_with_all = ["append", "prepend", "remove", "clear_all", "update"])]
        add: Option<String>,

        /// Append source (adds to end of list)
        #[arg(long, conflicts_with_all = ["add", "prepend", "remove", "clear_all", "update"])]
        append: Option<String>,

        /// Prepend source (adds to beginning of list)
        #[arg(long, conflicts_with_all = ["add", "append", "remove", "clear_all", "update"])]
        prepend: Option<String>,

        /// List sources (default)
        #[arg(short = 'l', long)]
        list: bool,

        /// Remove source
        #[arg(short = 'r', long, conflicts_with_all = ["add", "append", "prepend", "clear_all", "update"])]
        remove: Option<String>,

        /// Remove all sources (clear the cache)
        #[arg(short = 'c', long, conflicts_with_all = ["add", "append", "prepend", "remove", "update"])]
        clear_all: bool,

        /// Update source cache
        #[arg(short = 'u', long, conflicts_with_all = ["add", "append", "prepend", "remove", "clear_all"])]
        update: bool,

        /// Do not show any confirmation prompts
        #[arg(short = 'f', long)]
        force: bool,

        /// Use HTTP proxy for remote operations
        #[arg(short = 'p', long = "http-proxy", num_args = 0..=1, default_missing_value = "", overrides_with = "no_http_proxy")]
        http_proxy: Option<String>,

        /// Do not use HTTP proxy (negation of --http-proxy)
        #[arg(long, hide = true)]
        no_http_proxy: bool,

        /// Set verbose level of output
        #[arg(short = 'V', long)]
        verbose: bool,

        /// Silence command progress meter
        #[arg(short = 'q', long)]
        quiet: bool,

        /// Silence `RubyGems` output
        #[arg(long)]
        silent: bool,

        /// Config file path (overrides default)
        #[arg(long)]
        config_file: Option<String>,

        /// Show stack backtrace on errors
        #[arg(long)]
        backtrace: bool,

        /// Turn on Ruby debugging
        #[arg(long)]
        debug: bool,

        /// Avoid loading any .gemrc file
        #[arg(long)]
        norc: bool,
    },

    /// Manage gem certificates
    #[command(name = "gem-cert")]
    GemCert {
        /// Build a certificate for `EMAIL_ADDR`
        #[arg(short = 'b', long)]
        build: Option<String>,

        /// Add a trusted certificate
        #[arg(short = 'a', long)]
        add: Option<String>,

        /// List trusted certificates where the subject contains FILTER
        #[arg(short = 'l', long)]
        list: bool,

        /// Filter for list
        #[arg(value_name = "FILTER", requires = "list")]
        list_filter: Option<String>,

        /// Remove trusted certificates where subject contains FILTER
        #[arg(short = 'r', long)]
        remove: Option<String>,

        /// Sign CERT with the key from -K and certificate from -C
        #[arg(short = 's', long)]
        sign: Option<String>,

        /// Signing certificate for --sign
        #[arg(short = 'C', long)]
        certificate: Option<String>,

        /// Key for --sign or --build
        #[arg(short = 'K', long)]
        private_key: Option<String>,

        /// Select which key algorithm to use for --build
        #[arg(short = 'A', long, value_name = "ALGORITHM")]
        key_algorithm: Option<String>,

        /// Days before the certificate expires
        #[arg(short = 'd', long, value_name = "NUMBER_OF_DAYS")]
        days: Option<u32>,

        /// Re-sign the certificate from -C with the key from -K
        #[arg(short = 'R', long)]
        re_sign: bool,

        /// Verbose output
        #[arg(short = 'V', long)]
        verbose: bool,

        /// Quiet mode (suppress output)
        #[arg(short = 'q', long)]
        quiet: bool,

        /// Silent mode (no output)
        #[arg(long)]
        silent: bool,

        /// Config file path (overrides default)
        #[arg(long)]
        config_file: Option<String>,

        /// Show stack backtrace on errors
        #[arg(long)]
        backtrace: bool,

        /// Turn on Ruby debugging
        #[arg(long)]
        debug: bool,

        /// Avoid loading any .gemrc file
        #[arg(long)]
        norc: bool,
    },

    /// Build `RDoc` for installed gems
    #[command(name = "gem-rdoc")]
    GemRdoc {
        /// Gem name (optional)
        gem: Option<String>,

        /// Generate RDoc/RI documentation for all installed gems
        #[arg(long)]
        all: bool,

        /// Generate `RDoc` HTML
        #[arg(long)]
        rdoc: bool,

        /// Do not generate `RDoc` HTML
        #[arg(long, overrides_with = "rdoc")]
        no_rdoc: bool,

        /// Generate RI data
        #[arg(long)]
        ri: bool,

        /// Do not generate RI data
        #[arg(long, overrides_with = "ri")]
        no_ri: bool,

        /// Overwrite installed documents
        #[arg(long)]
        overwrite: bool,

        /// Do not overwrite installed documents
        #[arg(long, overrides_with = "overwrite")]
        no_overwrite: bool,

        /// Specify version of gem
        #[arg(short = 'v', long)]
        version: Option<String>,

        /// Verbose output
        #[arg(short = 'V', long)]
        verbose: bool,

        /// Quiet mode (suppress output)
        #[arg(short = 'q', long)]
        quiet: bool,

        /// Silent mode (no output)
        #[arg(long)]
        silent: bool,

        /// Config file path (overrides default)
        #[arg(long)]
        config_file: Option<String>,

        /// Show stack backtrace on errors
        #[arg(long)]
        backtrace: bool,

        /// Turn on Ruby debugging
        #[arg(long)]
        debug: bool,

        /// Avoid loading any .gemrc file
        #[arg(long)]
        norc: bool,
    },

    /// Show help for gem commands
    #[command(name = "gem-help")]
    GemHelp {
        /// Command to show help for (shows all commands if not specified)
        command: Option<String>,

        // Common flags
        /// Set the verbose level of output
        #[arg(short = 'V', long)]
        verbose: bool,

        /// Silence command progress meter
        #[arg(short = 'q', long, conflicts_with = "verbose")]
        quiet: bool,

        /// Silence `RubyGems` output
        #[arg(long, conflicts_with_all = ["verbose", "quiet"])]
        silent: bool,

        /// Config file path (overrides default)
        #[arg(long = "config-file")]
        config_file: Option<String>,

        /// Show stack backtrace on errors
        #[arg(long)]
        backtrace: bool,

        /// Turn on Ruby debugging
        #[arg(long)]
        debug: bool,

        /// Avoid loading any .gemrc file
        #[arg(long)]
        norc: bool,
    },

    /// Display `RubyGems` environment information
    #[command(name = "gem-environment")]
    GemEnvironment {
        /// Show specific variable (gemdir, gempath, version, remotesources, platform, etc.)
        variable: Option<String>,

        /// Verbose output
        #[arg(short = 'V', long)]
        verbose: bool,

        /// Quiet mode (suppress output)
        #[arg(short = 'q', long, conflicts_with = "verbose")]
        quiet: bool,

        /// Silent mode (no output)
        #[arg(long, conflicts_with_all = ["verbose", "quiet"])]
        silent: bool,

        /// Config file path (overrides default)
        #[arg(long = "config-file")]
        config_file: Option<String>,

        /// Show stack backtrace on errors
        #[arg(long)]
        backtrace: bool,

        /// Turn on Ruby debugging
        #[arg(long)]
        debug: bool,

        /// Avoid loading any .gemrc file
        #[arg(long)]
        norc: bool,
    },
}

#[derive(Subcommand)]
enum PluginCommands {
    /// Install a plugin
    Install {
        /// Plugin name to install
        plugin: String,

        /// Install from a specific source
        #[arg(long)]
        source: Option<String>,

        /// Install a specific version
        #[arg(long)]
        version: Option<String>,

        /// Install from a git repository
        #[arg(long)]
        git: Option<String>,

        /// Git branch to use
        #[arg(long)]
        branch: Option<String>,

        /// Git ref (tag or commit) to use
        #[arg(long, name = "ref")]
        ref_: Option<String>,

        /// Install from a local path
        #[arg(long)]
        path: Option<String>,
    },

    /// Uninstall a plugin
    Uninstall {
        /// Plugin name to uninstall
        plugin: Option<String>,

        /// Uninstall all plugins
        #[arg(long)]
        all: bool,
    },

    /// List installed plugins
    List,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    // Extract debug and backtrace flags before consuming cli.command
    let (debug, backtrace) = match &cli.command {
        Commands::GemInfo {
            debug, backtrace, ..
        }
        | Commands::GemList {
            debug, backtrace, ..
        }
        | Commands::GemSearch {
            debug, backtrace, ..
        }
        | Commands::GemUpdate {
            debug, backtrace, ..
        }
        | Commands::GemCleanup {
            debug, backtrace, ..
        }
        | Commands::GemPristine {
            debug, backtrace, ..
        }
        | Commands::GemUninstall {
            debug, backtrace, ..
        } => (*debug, *backtrace),
        _ => (false, false),
    };

    // Initialize debug mode
    lode::init_debug(debug);

    // Setup backtrace
    setup_backtrace(backtrace);

    let result = match cli.command {
        Commands::Init { path, gemspec } => commands::init::run(&path, gemspec),
        Commands::Add {
            gem,
            version,
            group,
            require,
            source,
            git,
            github,
            branch,
            ref_,
            glob,
            path,
            strict,
            optimistic,
            quiet,
            skip_install,
        } => {
            commands::add::run(
                &gem,
                version.as_deref(),
                group.as_deref(),
                require,
                source.as_deref(),
                git.as_deref(),
                github.as_deref(),
                branch.as_deref(),
                ref_.as_deref(),
                glob.as_deref(),
                path.as_deref(),
                strict,
                optimistic,
                quiet,
                !skip_install,
            )
            .await
        }
        Commands::Remove { gems, quiet } => commands::remove::run(&gems, quiet).await,
        Commands::Update {
            gems,
            all,
            conservative,
            gemfile,
            jobs,
            quiet,
            retry,
            patch,
            minor,
            major,
            strict,
            local,
            pre,
            group,
            source,
            ruby,
            bundler,
            redownload,
            full_index,
        } => {
            let bundle_config = lode::BundleConfig::load().unwrap_or_default();

            // Merge settings with proper priority (CLI > Config > Env > Default)
            let jobs_merged = jobs
                .or(bundle_config.jobs)
                .or_else(lode::env_vars::bundle_jobs);
            let retry_merged = retry
                .or_else(|| bundle_config.retry.map(|v| v as usize))
                .or_else(|| lode::env_vars::bundle_retry().map(|v| v as usize));
            let local_merged =
                local || bundle_config.local.unwrap_or(false) || lode::env_vars::bundle_local();
            let redownload_merged = redownload
                || bundle_config.force.unwrap_or(false)
                || lode::env_vars::bundle_force();

            commands::update::run(
                &gems,
                all,
                conservative,
                gemfile.as_deref(),
                jobs_merged,
                quiet,
                retry_merged,
                patch,
                minor,
                major,
                strict,
                local_merged,
                pre,
                group.as_deref(),
                source.as_deref(),
                ruby,
                bundler.as_deref(),
                redownload_merged,
                full_index,
            )
            .await
        }
        Commands::Outdated {
            lockfile,
            parseable,
            major,
            minor,
            patch,
            pre,
            group,
        } => {
            commands::outdated::run(
                &lockfile,
                parseable,
                major,
                minor,
                patch,
                pre,
                group.as_deref(),
            )
            .await
        }
        Commands::Lock {
            gemfile,
            lockfile,
            add_platform,
            remove_platform,
            update,
            print,
            verbose,
            patch,
            minor,
            major,
            strict,
            conservative,
            local,
            pre,
            bundler,
            normalize_platforms,
            add_checksums,
            full_index,
            quiet,
        } => {
            let bundle_config = lode::BundleConfig::load().unwrap_or_default();

            // Merge settings with proper priority (CLI > Config > Env > Default)
            let verbose_merged = verbose
                || bundle_config.verbose.unwrap_or(false)
                || lode::env_vars::bundle_verbose();
            let local_merged =
                local || bundle_config.local.unwrap_or(false) || lode::env_vars::bundle_local();

            commands::lock::run(
                &gemfile,
                lockfile.as_deref(),
                &add_platform,
                &remove_platform,
                &update,
                print,
                verbose_merged,
                patch,
                minor,
                major,
                strict,
                conservative,
                local_merged,
                pre,
                bundler.as_deref(),
                normalize_platforms,
                add_checksums,
                full_index,
                quiet,
            )
            .await
        }
        Commands::Install {
            gemfile,
            redownload,
            verbose,
            quiet,
            jobs,
            local,
            prefer_local,
            retry,
            no_cache,
            standalone,
            trust_policy,
            full_index,
            target_rbconfig,
        } => {
            let lockfile_path = gemfile.as_ref().map_or_else(
                || "Gemfile.lock".to_string(),
                |gemfile_path| format!("{gemfile_path}.lock"),
            );

            // Load bundle config from .bundle/config files
            // Priority: CLI flags > Local config > Env vars > Global config > Defaults
            let bundle_config = lode::BundleConfig::load().unwrap_or_default();

            // Merge settings with proper priority (CLI > Config > Env > Default)
            let jobs_merged = jobs
                .or(bundle_config.jobs)
                .or_else(lode::env_vars::bundle_jobs);
            let retry_merged = retry
                .or_else(|| bundle_config.retry.map(|v| v as usize))
                .or_else(|| lode::env_vars::bundle_retry().map(|v| v as usize));
            let local_merged =
                local || bundle_config.local.unwrap_or(false) || lode::env_vars::bundle_local();
            let prefer_local_merged = prefer_local
                || bundle_config.prefer_local.unwrap_or(false)
                || lode::env_vars::bundle_prefer_local();
            let force_merged = redownload
                || bundle_config.force.unwrap_or(false)
                || lode::env_vars::bundle_force();
            let no_cache_merged = no_cache; // No env var for this (not commonly used)
            let verbose_merged = verbose
                || bundle_config.verbose.unwrap_or(false)
                || lode::env_vars::bundle_verbose();

            // Warn if running as root (unless silenced)
            let silence_root_warning = bundle_config.silence_root_warning.unwrap_or(false)
                || lode::env_vars::bundle_silence_root_warning();
            if lode::user::is_root() && !silence_root_warning && !quiet {
                eprintln!(
                    "Warning: Running as root user. Set BUNDLE_SILENCE_ROOT_WARNING=1 to silence this warning."
                );
            }

            // Handle deployment mode: deployment = frozen + exclude dev/test
            let deployment_mode = bundle_config.deployment.unwrap_or(false);
            let frozen_merged = deployment_mode
                || bundle_config.frozen.unwrap_or(false)
                || lode::env_vars::bundle_frozen();

            // Gather group filters from config (Config > Env > Default)
            let mut without_groups_merged = bundle_config
                .without
                .clone()
                .or_else(lode::env_vars::bundle_without)
                .unwrap_or_default();
            let with_groups_merged = bundle_config
                .with
                .clone()
                .or_else(lode::env_vars::bundle_with)
                .unwrap_or_default();

            // Deployment mode automatically excludes development and test groups
            if deployment_mode {
                if !without_groups_merged.contains(&"development".to_string()) {
                    without_groups_merged.push("development".to_string());
                }
                if !without_groups_merged.contains(&"test".to_string()) {
                    without_groups_merged.push("test".to_string());
                }
            }

            // Auto-clean after install if BUNDLE_CLEAN is enabled
            let auto_clean = bundle_config.clean.unwrap_or(false) || lode::env_vars::bundle_clean();

            commands::install::run(commands::install::InstallOptions {
                lockfile_path: &lockfile_path,
                redownload: force_merged,
                verbose: verbose_merged,
                quiet,
                workers: jobs_merged,
                local: local_merged,
                prefer_local: prefer_local_merged,
                retry: retry_merged,
                no_cache: no_cache_merged,
                standalone: standalone.as_deref(),
                trust_policy: trust_policy.as_deref(),
                full_index,
                target_rbconfig: target_rbconfig.as_deref(),
                frozen: frozen_merged,
                without_groups: without_groups_merged,
                with_groups: with_groups_merged,
                auto_clean,
            })
            .await
        }
        Commands::Binstubs {
            gems,
            shebang,
            force,
            all,
            all_platforms,
        } => {
            let bundle_config = lode::BundleConfig::load().unwrap_or_default();
            let shebang_merged = shebang
                .or(bundle_config.shebang)
                .or_else(lode::env_vars::bundle_shebang);
            let force_merged =
                force || bundle_config.force.unwrap_or(false) || lode::env_vars::bundle_force();

            commands::binstubs::run(
                &gems,
                shebang_merged.as_deref(),
                force_merged,
                all,
                all_platforms,
            )
        }
        Commands::Check { gemfile, dry_run } => {
            let lockfile_path = gemfile.as_ref().map_or_else(
                || "Gemfile.lock".to_string(),
                |gemfile_path| format!("{gemfile_path}.lock"),
            );
            commands::check::run(&lockfile_path, dry_run)
        }
        Commands::List {
            name_only,
            paths,
            only_group,
            without_group,
        } => commands::list::run(
            "Gemfile.lock",
            name_only,
            paths,
            only_group.as_deref(),
            without_group.as_deref(),
        ),
        Commands::Show { gem, paths } => commands::show::run(gem.as_deref(), paths, "Gemfile.lock"),
        Commands::Info { gem, path, version } => commands::info::run(&gem, path, version).await,
        Commands::Search { query } => commands::search::run(&query).await,
        Commands::Specification { gem, version } => {
            commands::specification::run(&gem, version.as_deref()).await
        }
        Commands::Which { file } => commands::which::run(&file),
        Commands::Contents {
            gems,
            version,
            all,
            spec_dir,
            lib_only,
            prefix,
            show_install_dir,
            verbose: _,
            quiet: _,
            silent: _,
            config_file: _,
            backtrace: _,
            debug: _,
            norc: _,
        } => {
            let options = commands::contents::ContentsOptions {
                all,
                lib_only,
                prefix,
                show_install_dir,
            };
            commands::contents::run(&gems, version.as_deref(), &spec_dir, &options)
        }
        Commands::Unpack {
            gem,
            version,
            target,
            spec: _,
            trust_policy: _,
            verbose: _,
            quiet: _,
            silent: _,
            config_file: _,
            backtrace: _,
            debug: _,
            norc: _,
        } => commands::unpack::run(&gem, version.as_deref(), target.as_deref()).await,
        Commands::Env => {
            commands::env::run();
            Ok(())
        }
        Commands::Exec { command, gemfile } => {
            let lockfile_path = gemfile.as_ref().map_or_else(
                || "Gemfile.lock".to_string(),
                |gemfile_path| format!("{gemfile_path}.lock"),
            );
            commands::exec::run(&command, &lockfile_path)
        }
        Commands::Clean {
            vendor,
            dry_run,
            force,
        } => {
            let bundle_config = lode::BundleConfig::load().unwrap_or_default();
            let force_merged =
                force || bundle_config.force.unwrap_or(false) || lode::env_vars::bundle_force();

            commands::clean::run(vendor.as_deref(), dry_run, force_merged)
        }
        Commands::Cache {
            all_platforms,
            cache_path,
            gemfile,
            no_install,
            quiet,
        } => {
            let bundle_config = lode::BundleConfig::load().unwrap_or_default();

            // Merge settings with proper priority (CLI > Config > Env > Default)
            let all_platforms_merged = all_platforms
                || bundle_config.cache_all_platforms.unwrap_or(false)
                || lode::env_vars::bundle_cache_all_platforms();
            let cache_path_merged = cache_path
                .or(bundle_config.cache_path)
                .or_else(lode::env_vars::bundle_cache_path);

            commands::cache::run(
                all_platforms_merged,
                cache_path_merged.as_deref(),
                gemfile.as_deref(),
                no_install,
                quiet,
            )
            .await
        }
        Commands::Pristine {
            gems,
            lockfile,
            vendor,
            verbose: _,
            quiet: _,
            silent: _,
            config_file: _,
            backtrace: _,
            debug: _,
            norc: _,
        } => commands::pristine::run(&gems, &lockfile, vendor.as_deref()),
        Commands::Config {
            key,
            value,
            list,
            delete,
            global,
            local,
        } => commands::config::run(
            key.as_deref(),
            value.as_deref(),
            list,
            delete,
            global,
            local,
        ),
        Commands::Platform { ruby } => commands::platform::run(ruby),
        Commands::Plugin { subcommand } => match subcommand {
            PluginCommands::Install {
                plugin,
                source,
                version,
                git,
                branch,
                ref_,
                path,
            } => {
                commands::plugin::install(
                    &plugin,
                    source.as_deref(),
                    version.as_deref(),
                    git.as_deref(),
                    branch.as_deref(),
                    ref_.as_deref(),
                    path.as_deref(),
                )
                .await
            }
            PluginCommands::Uninstall { plugin, all } => {
                commands::plugin::uninstall(plugin.as_deref(), all)
            }
            PluginCommands::List => commands::plugin::list(),
        },
        Commands::Completion { shell } => commands::completion::run(shell),
        Commands::Open { gem, path } => commands::open::run(&gem, path.as_deref()),
        Commands::Doctor { gemfile, quiet } => commands::doctor::run(gemfile.as_deref(), quiet),
        Commands::Gem {
            name,
            exe,
            mit,
            no_mit,
            test,
        } => commands::gem::run(&name, exe, mit, no_mit, test.as_deref()),
        Commands::GemBuild {
            gemspec,
            platform,
            force,
            strict,
            output,
            directory,
            verbose: _,
            quiet: _,
            silent: _,
            config_file: _,
            backtrace: _,
            debug: _,
            norc: _,
        } => commands::gem_build::run_with_options(
            gemspec.as_deref(),
            platform.as_deref(),
            force,
            strict,
            output.as_deref(),
            directory.as_deref(),
        ),
        Commands::GemCert {
            build,
            add,
            list,
            list_filter,
            remove,
            sign,
            certificate,
            private_key,
            key_algorithm,
            days,
            re_sign,
            verbose: _,
            quiet: _,
            silent: _,
            config_file: _,
            backtrace: _,
            debug: _,
            norc: _,
        } => {
            let options = commands::gem_cert::CertOptions {
                build,
                add,
                list,
                list_filter,
                remove,
                sign,
                certificate,
                private_key,
                key_algorithm,
                days,
                re_sign,
            };
            commands::gem_cert::run(options)
        }
        Commands::GemCleanup {
            gems,
            dry_run,
            check_development,
            user_install,
            verbose,
            quiet,
            silent: _,
            config_file,
            backtrace: _,
            debug: _,
            norc,
        } => {
            let options = commands::gem_cleanup::CleanupOptions {
                gems,
                dry_run,
                check_development,
                user_install,
                verbose,
                quiet,
                config_file,
                norc,
            };
            commands::gem_cleanup::run(&options)
        }
        Commands::GemContents {
            gem,
            version,
            all,
            spec_dir,
            lib_only,
            prefix,
            no_prefix,
            show_install_dir,
            no_show_install_dir,
            verbose,
            quiet,
            silent,
            config_file: _,
            backtrace: _,
            debug: _,
            norc: _,
        } => {
            let opts = commands::gem_contents::ContentsOptions {
                gem_name: gem,
                version,
                all,
                spec_dir,
                lib_only,
                prefix: if no_prefix { false } else { prefix },
                show_install_dir: if no_show_install_dir {
                    false
                } else {
                    show_install_dir
                },
                verbose,
                quiet,
                silent,
            };
            commands::gem_contents::run(&opts)
        }
        Commands::GemDependency {
            gem,
            version,
            platform,
            prerelease,
            reverse_dependencies,
            pipe,
            local,
            remote,
            both,
            bulk_threshold,
            clear_sources,
            source,
            http_proxy,
            no_http_proxy: _,
            verbose,
            quiet,
            silent,
            config_file: _,
            backtrace: _,
            debug: _,
            norc: _,
        } => {
            let opts = commands::gem_dependency::DependencyOptions {
                gem_pattern: gem.clone(),
                version: version.clone(),
                platform: platform.clone(),
                prerelease,
                reverse_dependencies,
                pipe,
                local,
                remote,
                both,
                bulk_threshold,
                clear_sources,
                source: source.clone(),
                http_proxy: http_proxy.clone(),
                verbose,
                quiet,
                silent,
            };
            commands::gem_dependency::run(opts).await
        }
        Commands::GemFetch {
            gem,
            version,
            output_dir,
            platform: _,
            prerelease: _,
            suggestions: _,
            bulk_threshold: _,
            http_proxy: _,
            no_http_proxy: _,
            source: _,
            clear_sources: _,
            verbose: _,
            quiet: _,
            silent: _,
            config_file: _,
            backtrace: _,
            debug: _,
            norc: _,
        } => commands::gem_fetch::run(&gem, version.as_deref(), output_dir.as_deref()).await,
        Commands::GemHelp {
            command,
            verbose: _,
            quiet: _,
            silent: _,
            config_file: _,
            backtrace: _,
            debug: _,
            norc: _,
        } => commands::gem_help::run(command.as_deref()),
        Commands::GemEnvironment {
            variable,
            verbose,
            quiet,
            silent: _,
            config_file: _,
            backtrace: _,
            debug: _,
            norc: _,
        } => commands::gem_environment::run(commands::gem_environment::EnvironmentOptions {
            variable,
            verbose,
            quiet,
        }),
        Commands::GemInfo {
            gem,
            installed,
            no_installed: _,
            version,
            versions,
            no_versions: _,
            all,
            exact,
            prerelease,
            no_prerelease: _,
            update_sources,
            no_update_sources: _,
            local,
            remote,
            both,
            bulk_threshold,
            clear_sources,
            source,
            http_proxy,
            verbose,
            no_verbose: _,
            quiet,
            silent,
            config_file,
            backtrace,
            debug,
            norc,
            ..
        } => {
            let options = commands::gem_info::InfoOptions {
                gem,
                installed,
                version,
                versions,
                all,
                exact,
                prerelease,
                update_sources,
                local,
                remote,
                both,
                bulk_threshold,
                clear_sources,
                source,
                http_proxy,
                verbose,
                quiet,
                silent,
                config_file,
                backtrace,
                debug,
                norc,
            };
            commands::gem_info::run(options).await
        }
        Commands::GemInstall {
            gems,
            platform,
            version,
            prerelease,
            no_prerelease: _,
            update_sources,
            no_update_sources: _,
            install_dir,
            bindir,
            document,
            no_document,
            build_root,
            vendor,
            env_shebang,
            no_env_shebang: _,
            force,
            no_force: _,
            wrappers,
            no_wrappers: _,
            trust_policy,
            ignore_dependencies,
            format_executable,
            no_format_executable: _,
            user_install,
            no_user_install: _,
            development,
            development_all,
            conservative,
            minimal_deps,
            no_minimal_deps: _,
            post_install_message,
            no_post_install_message: _,
            file,
            without,
            explain,
            lock,
            suggestions,
            no_suggestions: _,
            target_rbconfig,
            default: _,
            build_flags: _,
            ruby: _,
            with_extension_lib: _,
            local,
            remote,
            both,
            bulk_threshold,
            clear_sources,
            source,
            http_proxy,
            no_http_proxy: _,
            verbose,
            quiet,
            silent,
            config_file,
            backtrace,
            debug,
            norc,
        } => {
            let options = commands::gem_install::InstallOptions {
                gems: gems.clone(),
                platform: platform.clone(),
                version: version.clone(),
                prerelease,
                update_sources,
                install_dir: install_dir.clone(),
                bindir: bindir.clone(),
                document: document.clone(),
                no_document,
                build_root: build_root.clone(),
                vendor,
                env_shebang,
                force,
                wrappers,
                trust_policy: trust_policy.clone(),
                ignore_dependencies,
                format_executable,
                user_install,
                development,
                development_all,
                conservative,
                minimal_deps,
                post_install_message,
                file: file.clone(),
                without: without.clone(),
                explain,
                lock,
                suggestions,
                target_rbconfig: target_rbconfig.clone(),
                local,
                remote,
                both,
                bulk_threshold,
                clear_sources,
                source: source.clone(),
                http_proxy: http_proxy.clone(),
                verbose,
                quiet,
                silent,
                config_file: config_file.clone(),
                backtrace,
                debug,
                norc,
            };
            commands::gem_install::run(options).await
        }
        Commands::GemList {
            pattern,
            installed,
            not_installed,
            version,
            details,
            no_details: _,
            versions,
            all,
            exact,
            prerelease,
            no_prerelease: _,
            update_sources,
            no_update_sources: _,
            local,
            remote,
            both,
            bulk_threshold,
            clear_sources,
            source,
            http_proxy,
            no_http_proxy: _,
            verbose,
            quiet,
            silent,
            config_file,
            backtrace,
            debug,
            norc,
        } => {
            let options = commands::gem_list::ListOptions {
                pattern: pattern.as_deref(),
                installed: if not_installed {
                    Some(false)
                } else if installed {
                    Some(true)
                } else {
                    None
                },
                version: version.as_deref(),
                details,
                versions,
                all,
                exact,
                prerelease,
                update_sources,
                local,
                remote,
                both,
                bulk_threshold,
                clear_sources,
                source: source.as_deref(),
                http_proxy: http_proxy.as_deref(),
                verbose,
                quiet,
                silent,
                config_file: config_file.as_deref(),
                backtrace,
                debug,
                norc,
            };
            commands::gem_list::run(options).await
        }
        Commands::GemOwner {
            gem,
            add,
            remove,
            key,
            otp,
            host,
            http_proxy,
            no_http_proxy: _,
            verbose: _,
            quiet: _,
            silent: _,
            config_file: _,
            backtrace: _,
            debug: _,
            norc: _,
        } => {
            async move {
                if add.is_empty() && remove.is_empty() {
                    return commands::gem_owner::list_owners(
                        &gem,
                        host.as_deref(),
                        key.as_deref(),
                        http_proxy.as_deref(),
                    )
                    .await;
                }

                for email in &add {
                    commands::gem_owner::run_with_options(
                        &gem,
                        email,
                        true,
                        host.as_deref(),
                        key.as_deref(),
                        otp.as_deref(),
                        http_proxy.as_deref(),
                    )
                    .await?;
                }

                for email in &remove {
                    commands::gem_owner::run_with_options(
                        &gem,
                        email,
                        false,
                        host.as_deref(),
                        key.as_deref(),
                        otp.as_deref(),
                        http_proxy.as_deref(),
                    )
                    .await?;
                }

                Ok(())
            }
            .await
        }
        Commands::GemPristine {
            gems,
            all,
            skip,
            extensions,
            only_missing_extensions,
            only_executables,
            only_plugins,
            env_shebang,
            install_dir,
            bindir,
            version,
            verbose,
            quiet,
            silent: _,
            config_file,
            backtrace: _,
            debug: _,
            norc,
        } => {
            let options = commands::gem_pristine::PristineOptions {
                gems,
                all,
                skip,
                extensions,
                only_missing_extensions,
                only_executables,
                only_plugins,
                env_shebang,
                install_dir: install_dir.map(std::path::PathBuf::from),
                bindir: bindir.map(std::path::PathBuf::from),
                version,
                verbose,
                quiet,
                config_file,
                norc,
            };
            commands::gem_pristine::run(&options)
        }
        Commands::GemPush {
            gem,
            key,
            otp,
            host,
            attestation: _,
            http_proxy: _,
            no_http_proxy: _,
            verbose: _,
            quiet: _,
            silent: _,
            config_file: _,
            backtrace: _,
            debug: _,
            norc: _,
        } => {
            commands::gem_push::run_with_options(
                &gem,
                host.as_deref(),
                key.as_deref(),
                otp.as_deref(),
            )
            .await
        }
        Commands::GemRdoc {
            gem,
            all: _,
            rdoc: _,
            no_rdoc: _,
            ri: _,
            no_ri: _,
            overwrite: _,
            no_overwrite: _,
            version: _,
            verbose: _,
            quiet: _,
            silent: _,
            config_file: _,
            backtrace: _,
            debug: _,
            norc: _,
        } => commands::gem_rdoc::run(gem.as_deref()),
        Commands::GemRebuild {
            gem,
            diff: _,
            force: _,
            strict: _,
            source: _,
            original: _,
            gemspec: _,
            working_dir: _,
            verbose: _,
            quiet: _,
            silent: _,
            config_file: _,
            backtrace: _,
            debug: _,
            norc: _,
        } => commands::gem_rebuild::run(&gem),
        Commands::GemSearch {
            query,
            installed,
            not_installed,
            version,
            details,
            no_details: _,
            versions,
            all,
            exact,
            prerelease,
            no_prerelease: _,
            update_sources,
            no_update_sources: _,
            local,
            remote,
            both,
            bulk_threshold,
            clear_sources,
            source,
            http_proxy,
            no_http_proxy: _,
            verbose,
            quiet,
            silent,
            config_file,
            backtrace,
            debug,
            norc,
        } => {
            let options = commands::gem_search::SearchOptions {
                query,
                installed: if not_installed {
                    Some(false)
                } else if installed {
                    Some(true)
                } else {
                    None
                },
                version,
                details,
                versions,
                all,
                exact,
                prerelease,
                update_sources,
                local,
                remote,
                both,
                bulk_threshold,
                clear_sources,
                source,
                http_proxy,
                verbose,
                quiet,
                silent,
                config_file,
                backtrace,
                debug,
                norc,
            };
            commands::gem_search::run(options).await
        }
        Commands::GemSignin {
            host,
            verbose: _,
            quiet: _,
            silent: _,
            config_file: _,
            backtrace: _,
            debug: _,
            norc: _,
        } => commands::gem_signin::run(host.as_deref()).await,
        Commands::GemSignout {
            verbose,
            quiet,
            silent,
            config_file: _,
            backtrace: _,
            debug: _,
            norc: _,
        } => {
            let options = commands::gem_signout::SignoutOptions {
                verbose,
                quiet,
                silent,
            };
            commands::gem_signout::run_with_options(options)
        }
        Commands::GemSources {
            add,
            append,
            prepend,
            list,
            remove,
            clear_all,
            update,
            force,
            http_proxy,
            no_http_proxy: _,
            verbose,
            quiet,
            silent,
            config_file: _,
            backtrace: _,
            debug: _,
            norc: _,
        } => {
            let options = commands::gem_sources::SourcesOptions {
                add,
                append,
                prepend,
                remove,
                clear_all,
                update,
                list,
                force,
                http_proxy,
                verbose,
                quiet,
                silent,
            };
            commands::gem_sources::run_with_options(options).await
        }

        Commands::GemStale {
            verbose,
            quiet,
            silent,
            config_file: _,
            backtrace: _,
            debug: _,
            norc: _,
        } => {
            let options = commands::gem_stale::StaleOptions {
                verbose,
                quiet,
                silent,
            };
            commands::gem_stale::run_with_options(options)
        }
        Commands::GemUninstall {
            gems,
            all,
            ignore_dependencies,
            check_development,
            executables,
            install_dir,
            bindir,
            user_install: _,
            no_user_install,
            format_executable,
            no_format_executable: _,
            force,
            no_force: _,
            abort_on_dependent,
            no_abort_on_dependent: _,
            version,
            platform,
            vendor,
            verbose: _,
            quiet: _,
            silent: _,
            config_file,
            backtrace: _,
            debug: _,
            norc,
        } => {
            // Default: --user-install is TRUE per gem 4.0.0
            // Only set to false if explicitly --no-user-install is passed
            let user_install_final = !no_user_install;

            let options = commands::gem_uninstall::UninstallOptions {
                all,
                ignore_dependencies,
                check_development,
                executables,
                install_dir,
                bindir,
                user_install: user_install_final,
                format_executable,
                force,
                abort_on_dependent,
                version,
                platform,
                vendor,
                config_file,
                norc,
            };
            commands::gem_uninstall::run(&gems, &options)
        }
        Commands::GemUpdate {
            gems,
            system,
            platform,
            prerelease,
            no_prerelease: _,
            bindir,
            build_root,
            install_dir,
            document,
            no_document,
            vendor,
            env_shebang,
            no_env_shebang: _,
            wrappers,
            no_wrappers: _,
            format_executable,
            no_format_executable: _,
            user_install,
            no_user_install: _,
            post_install_message,
            no_post_install_message: _,
            local,
            remote,
            both,
            bulk_threshold,
            clear_sources,
            source,
            force,
            no_force: _,
            ignore_dependencies,
            minimal_deps,
            no_minimal_deps: _,
            conservative,
            development,
            development_all,
            default,
            trust_policy,
            target_rbconfig,
            file,
            without,
            explain,
            lock,
            no_lock: _,
            suggestions,
            no_suggestions: _,
            http_proxy,
            no_http_proxy: _,
            verbose,
            no_verbose: _,
            quiet,
            silent,
            config_file,
            backtrace,
            debug,
            norc,
        } => {
            let options = commands::gem_update::UpdateOptions {
                gems,
                system,
                platform,
                prerelease,
                bindir,
                build_root,
                install_dir,
                document,
                no_document,
                vendor,
                env_shebang,
                force,
                wrappers,
                trust_policy,
                ignore_dependencies,
                format_executable,
                user_install,
                development,
                development_all,
                conservative,
                minimal_deps,
                post_install_message,
                file,
                without,
                explain,
                lock,
                suggestions,
                target_rbconfig,
                default,
                local,
                remote,
                both,
                bulk_threshold,
                clear_sources,
                source,
                http_proxy,
                verbose,
                quiet,
                silent,
                config_file,
                backtrace,
                debug,
                norc,
            };
            commands::gem_update::run(options).await
        }
        Commands::GemWhich {
            files,
            all,
            gems_first,
            verbose,
            quiet,
            silent,
            config_file: _,
            backtrace: _,
            debug: _,
            norc: _,
        } => {
            let options = commands::gem_which::WhichOptions {
                all,
                gems_first,
                verbose,
                quiet,
                silent,
            };
            commands::gem_which::run(&files, &options)
        }
        Commands::GemYank {
            gem,
            version,
            platform,
            otp,
            host,
            key,
            undo,
            verbose: _,
            quiet: _,
            silent: _,
            config_file: _,
            backtrace: _,
            debug: _,
            norc: _,
        } => {
            commands::gem_yank::run_with_options(
                &gem,
                &version,
                platform.as_deref(),
                host.as_deref(),
                key.as_deref(),
                otp.as_deref(),
                undo,
            )
            .await
        }
    };

    if let Err(e) = result {
        // Display error with formatting
        display_error(&e, backtrace);
        process::exit(1);
    }
}

mod commands;
