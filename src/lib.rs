//! Lode CLI internal library code

/// Default gem source URL (CDN/mirror)
pub const DEFAULT_GEM_SOURCE: &str = "https://rubygems.org";

/// Official RubyGems.org URL (for API operations like push/yank/signin)
pub const RUBYGEMS_ORG_URL: &str = "https://rubygems.org";

/// Get the gem source URL to use for fetching gems.
/// Priority: `GEM_SOURCE` env var -> `DEFAULT_GEM_SOURCE` constant.
#[must_use]
pub fn gem_source_url() -> String {
    env_vars::gem_source().unwrap_or_else(|| DEFAULT_GEM_SOURCE.to_string())
}

pub mod cache;
pub mod config;
pub mod debug;
pub mod download;
pub mod env_vars;
pub mod extensions;
pub mod full_index;
pub mod gem_store;
pub mod gem_utils;
pub mod gemfile;
pub mod gemfile_writer;
pub mod git;
pub mod install;
pub mod lockfile;
pub mod paths;
pub mod platform;
pub mod resolver;
pub mod ruby;
pub mod rubygems_client;
pub mod standalone;
pub mod trust_policy;
pub mod user;

// Re-export common types for convenience
pub use cache::{Stats as CacheDirStats, collect_stats, human_bytes};
pub use config::{BundleConfig, Config};
pub use debug::{debug_log, debug_logf, init_debug, is_debug_enabled};
pub use download::DownloadManager;
pub use extensions::{
    BinstubGenerator, BuildResult, CExtensionBuilder, ExtensionBuilder, ExtensionType,
    build_extensions, generate_binstubs,
};
pub use full_index::{FullIndex, IndexGemSpec};
pub use gem_utils::parse_gem_name;
pub use gemfile::{GemDependency, Gemfile, GemfileError};
pub use gemfile_writer::GemfileWriter;
pub use git::{GitError, GitManager};
pub use install::InstallReport;
pub use lockfile::{Dependency, GemSpec, GitGemSpec, Lockfile, LockfileError, PathGemSpec};
pub use paths::{
    find_gemfile, find_gemfile_in, find_lockfile, find_lockfile_in, gemfile_for_lockfile,
    lockfile_for_gemfile,
};
pub use platform::{detect_current_platform, platform_matches};
pub use resolver::{ResolvedDependency, ResolvedGem, Resolver, ResolverError};
pub use ruby::{
    RubyEngine, detect_engine, detect_engine_from_platform, detect_ruby_version,
    detect_ruby_version_from_lockfile, get_standard_gem_paths, get_system_gem_dir,
    normalize_ruby_version, to_major_minor,
};
pub use rubygems_client::{
    CacheStats, Dependencies, DependencySpec, GemMetadata, GemVersion, RubyGemsClient,
    RubyGemsError,
};
pub use standalone::{StandaloneBundle, StandaloneGem, StandaloneOptions};
pub use trust_policy::{GemVerifier, TrustPolicy, VerificationError};
