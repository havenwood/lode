//! Gem version resolution using the `PubGrub` algorithm.

use crate::gemfile::Gemfile;
use crate::rubygems_client::{GemVersion, RubyGemsClient, RubyGemsError};
use anyhow::{Context, Result};
use pubgrub::{
    DefaultStringReporter, Dependencies, DependencyConstraints, DependencyProvider,
    PackageResolutionStatistics, Ranges, Reporter, SemanticVersion,
};
use std::collections::HashMap;
use std::convert::Infallible;
use std::sync::Arc;
use thiserror::Error;

/// Errors that can occur during dependency resolution
#[derive(Debug, Error)]
pub enum ResolverError {
    #[error("Failed to resolve dependencies: {message}")]
    ResolutionFailed { message: String },

    #[error("Gem '{gem}' not found in any source")]
    GemNotFound { gem: String },

    #[error("Invalid version constraint '{constraint}' for gem '{gem}': {reason}")]
    InvalidConstraint {
        gem: String,
        constraint: String,
        reason: String,
    },

    #[error("Circular dependency detected: {chain}")]
    CircularDependency { chain: String },

    #[error("Network error while resolving '{gem}': {source}")]
    NetworkError {
        gem: String,
        #[source]
        source: RubyGemsError,
    },
}

/// A resolved gem with its final version
///
/// Represents a single gem at a specific version chosen by the resolver
/// (similar to `bundle lock` output).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedGem {
    /// Gem name
    pub name: String,

    /// Resolved version
    pub version: String,

    /// Platform (e.g., "ruby", "x86_64-linux")
    pub platform: String,

    /// Dependencies of this resolved version
    pub dependencies: Vec<ResolvedDependency>,

    /// Ruby version requirement
    pub ruby_version: Option<String>,
}

/// A dependency of a resolved gem
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedDependency {
    /// Dependency name
    pub name: String,

    /// Version requirement
    pub requirement: String,
}

/// Dependency resolver using `PubGrub` algorithm
///
/// Uses `PubGrub` instead of Bundler's Molinillo, providing clearer error
/// messages when resolution fails.
#[derive(Debug)]
pub struct Resolver {
    /// `RubyGems` API client for fetching metadata
    client: Arc<RubyGemsClient>,

    /// Cache of version ranges parsed from gem version requirements
    range_cache: std::sync::RwLock<HashMap<String, Ranges<SemanticVersion>>>,
}

impl Resolver {
    /// Create a new resolver with the given `RubyGems` client
    #[must_use]
    pub fn new(client: RubyGemsClient) -> Self {
        Self {
            client: Arc::new(client),
            range_cache: std::sync::RwLock::new(HashMap::new()),
        }
    }

    /// Resolve dependencies from a Gemfile.
    ///
    /// Similar to running `bundle lock`.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Dependencies cannot be resolved (conflicting version constraints)
    /// - A gem is not found
    /// - Network errors occur while fetching metadata
    pub async fn resolve(
        &self,
        gemfile: &Gemfile,
        platforms: &[&str],
        allow_prerelease: bool,
    ) -> Result<Vec<ResolvedGem>, ResolverError> {
        // Pre-fetch direct dependencies to warm the cache
        // This reduces blocking operations during PubGrub resolution
        let mut fetch_tasks = Vec::with_capacity(gemfile.gems.len());
        for gem in &gemfile.gems {
            let client = Arc::clone(&self.client);
            let gem_name = gem.name.clone();

            let task = tokio::spawn(async move {
                // Ignore errors - cache will be empty if fetch fails
                drop(client.fetch_versions(&gem_name).await);
            });

            fetch_tasks.push(task);
        }

        // Wait for all pre-fetches to complete
        for task in fetch_tasks {
            drop(task.await);
        }

        // Create dependency provider for PubGrub
        let provider = RubyGemsDependencyProvider {
            client: Arc::clone(&self.client),
            platforms: platforms
                .iter()
                .map(std::string::ToString::to_string)
                .collect(),
            allow_prerelease,
            cache: std::sync::RwLock::new(HashMap::new()),
            root_deps: std::sync::RwLock::new(HashMap::new()),
        };

        // Store root dependencies in provider
        {
            let mut root_deps_map =
                provider
                    .root_deps
                    .write()
                    .map_err(|_| ResolverError::ResolutionFailed {
                        message: "internal error: lock poisoned during initialization".to_string(),
                    })?;
            for gem in &gemfile.gems {
                let range = self
                    .parse_version_requirement(&gem.name, &gem.version_requirement)
                    .map_err(|e| ResolverError::InvalidConstraint {
                        gem: gem.name.clone(),
                        constraint: gem.version_requirement.clone(),
                        reason: e.to_string(),
                    })?;

                root_deps_map.insert(gem.name.clone(), (range, String::new()));
            }
        }

        // Run PubGrub resolution with a virtual root package
        let root_package = "___root___".to_string();
        let root_version = SemanticVersion::zero();
        let resolved =
            pubgrub::resolve(&provider, root_package.clone(), root_version).map_err(|err| {
                use pubgrub::PubGrubError;
                let message = match err {
                    PubGrubError::NoSolution(tree) => DefaultStringReporter::report(&tree),
                    PubGrubError::ErrorRetrievingDependencies {
                        package,
                        version,
                        source,
                    } => {
                        format!("Error retrieving dependencies for {package} {version}: {source:?}")
                    }
                };
                ResolverError::ResolutionFailed { message }
            })?;

        // Convert PubGrub solution to our ResolvedGem format
        let mut result = Vec::new();
        for (package, version) in resolved {
            // Skip the root package (injected by PubGrub)
            if package == root_package || version == SemanticVersion::zero() {
                continue;
            }

            // Fetch the gem version details
            let versions = provider
                .client
                .fetch_versions(&package)
                .await
                .map_err(|e| ResolverError::NetworkError {
                    gem: package.clone(),
                    source: e,
                })?;

            let version_str = version.to_string();

            // Find the matching version
            let gem_version = versions
                .iter()
                .find(|v| v.number == version_str)
                .ok_or_else(|| ResolverError::GemNotFound {
                    gem: format!("{package}-{version_str}"),
                })?;

            result.push(ResolvedGem {
                name: package,
                version: version_str,
                platform: gem_version.platform.clone(),
                dependencies: gem_version
                    .dependencies
                    .runtime
                    .iter()
                    .map(|dep| ResolvedDependency {
                        name: dep.name.clone(),
                        requirement: dep.requirements.clone(),
                    })
                    .collect(),
                ruby_version: gem_version.ruby_version.clone(),
            });
        }

        // Sort by name for consistent output
        result.sort_by(|a, b| a.name.cmp(&b.name));

        Ok(result)
    }

    /// Parse a Ruby gem version requirement into a `PubGrub` range
    ///
    /// Converts gem version constraints to `PubGrub's` `Range` type.
    ///
    /// # Supported formats
    ///
    /// - `">= 1.0.0"`, `"< 2.0.0"`
    /// - `"~> 1.2"` (pessimistic: `">= 1.2, < 2.0"`)
    /// - `">= 1.0, < 2.0"` (multiple constraints)
    /// - `""` (any version)
    ///
    /// # Errors
    ///
    /// Returns an error if the version requirement cannot be parsed
    pub fn parse_version_requirement(
        &self,
        gem_name: &str,
        requirement: &str,
    ) -> Result<Ranges<SemanticVersion>> {
        // Check cache first
        let cache_key = format!("{gem_name}:{requirement}");
        {
            let cache = self
                .range_cache
                .read()
                .map_err(|_| anyhow::anyhow!("Range cache lock poisoned"))?;
            if let Some(range) = cache.get(&cache_key) {
                return Ok(range.clone());
            }
        }

        let range = if requirement.is_empty() {
            // No constraint = any version
            Ranges::full()
        } else if requirement.starts_with("~>") {
            // Pessimistic constraint: "~> 1.2" means ">= 1.2, < 2.0"
            Self::parse_pessimistic_constraint(requirement)?
        } else if requirement.contains(',') {
            // Multiple constraints: ">= 1.0, < 2.0"
            self.parse_multiple_constraints(requirement)?
        } else if requirement.starts_with(">=") {
            // Greater than or equal
            let version_str = requirement.trim_start_matches(">=").trim();
            let version = Self::parse_semantic_version(version_str)?;
            Ranges::higher_than(version)
        } else if requirement.starts_with('>') {
            // Greater than (strict)
            let version_str = requirement.trim_start_matches('>').trim();
            let version = Self::parse_semantic_version(version_str)?;
            Ranges::strictly_higher_than(version)
        } else if requirement.starts_with("<=") {
            // Less than or equal
            let version_str = requirement.trim_start_matches("<=").trim();
            let version = Self::parse_semantic_version(version_str)?;
            Ranges::strictly_lower_than(version.bump_patch())
        } else if requirement.starts_with('<') {
            // Less than (strict)
            let version_str = requirement.trim_start_matches('<').trim();
            let version = Self::parse_semantic_version(version_str)?;
            Ranges::strictly_lower_than(version)
        } else if requirement.starts_with('=') {
            // Exact version
            let version_str = requirement.trim_start_matches('=').trim();
            let version = Self::parse_semantic_version(version_str)?;
            Ranges::singleton(version)
        } else {
            // Assume exact version if no operator
            let version = Self::parse_semantic_version(requirement.trim())?;
            Ranges::singleton(version)
        };

        // Cache the parsed range
        {
            let mut cache = self
                .range_cache
                .write()
                .map_err(|_| anyhow::anyhow!("Range cache lock poisoned"))?;
            cache.insert(cache_key, range.clone());
        }

        Ok(range)
    }

    /// Parse a pessimistic constraint like "~> 1.2.3"
    fn parse_pessimistic_constraint(constraint: &str) -> Result<Ranges<SemanticVersion>> {
        let version_str = constraint.trim_start_matches("~>").trim();
        let version = Self::parse_semantic_version(version_str)?;

        // "~> 1.2.3" means ">= 1.2.3, < 1.3.0"
        // "~> 1.2" means ">= 1.2.0, < 2.0.0"
        // Parse the original string to determine format
        let parts: Vec<&str> = version_str.split('.').collect();
        let upper_bound = if parts.len() >= 3 && parts.get(2).is_some_and(|&p| p != "0") {
            // Has non-zero patch, bump minor
            let major: u32 = parts
                .first()
                .ok_or_else(|| anyhow::anyhow!("Missing major version"))?
                .parse()
                .map_err(|_| anyhow::anyhow!("Invalid major version"))?;
            let minor: u32 = parts
                .get(1)
                .ok_or_else(|| anyhow::anyhow!("Missing minor version"))?
                .parse()
                .map_err(|_| anyhow::anyhow!("Invalid minor version"))?;
            SemanticVersion::new(major, minor + 1, 0)
        } else {
            // No patch or patch is 0, bump major
            let major: u32 = parts
                .first()
                .ok_or_else(|| anyhow::anyhow!("Missing major version"))?
                .parse()
                .map_err(|_| anyhow::anyhow!("Invalid major version"))?;
            SemanticVersion::new(major + 1, 0, 0)
        };

        Ok(Ranges::between(version, upper_bound))
    }

    /// Parse multiple constraints like ">= 1.0, < 2.0"
    fn parse_multiple_constraints(&self, constraints: &str) -> Result<Ranges<SemanticVersion>> {
        let parts: Vec<&str> = constraints.split(',').map(str::trim).collect();

        let mut combined = Ranges::full();
        for part in parts {
            let range = self.parse_version_requirement("", part)?;
            combined = combined.intersection(&range);
        }

        Ok(combined)
    }

    /// Parse a semantic version string
    ///
    /// # Errors
    ///
    /// Returns an error if the version string is invalid
    pub fn parse_semantic_version(version: &str) -> Result<SemanticVersion> {
        let parts: Vec<&str> = version.split('.').collect();

        let major = parts
            .first()
            .and_then(|s| s.parse::<u32>().ok())
            .context("Invalid major version")?;

        let minor = parts
            .get(1)
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(0);

        let patch = parts
            .get(2)
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(0);

        Ok(SemanticVersion::new(major, minor, patch))
    }
}

/// `PubGrub` dependency provider for `RubyGems`
///
/// This implements `PubGrub`'s `DependencyProvider` trait to fetch gem metadata
/// and provide it to the resolution algorithm.
struct RubyGemsDependencyProvider {
    client: Arc<RubyGemsClient>,
    platforms: Vec<String>,
    allow_prerelease: bool,
    #[allow(
        dead_code,
        reason = "Cache for future optimization of dependency provider"
    )]
    cache: std::sync::RwLock<HashMap<String, Vec<GemVersion>>>,
    root_deps: std::sync::RwLock<HashMap<String, (Ranges<SemanticVersion>, String)>>,
}

impl DependencyProvider for RubyGemsDependencyProvider {
    type P = String;
    type V = SemanticVersion;
    type VS = Ranges<SemanticVersion>;
    type M = String; // Metadata (we'll use empty string for now)
    type Err = Infallible;
    type Priority = usize;

    fn prioritize(
        &self,
        _package: &Self::P,
        _range: &Self::VS,
        _conflicts_counts: &PackageResolutionStatistics,
    ) -> Self::Priority {
        // Simple strategy: return 0 for all packages (no prioritization)
        0
    }

    fn choose_version(
        &self,
        package: &Self::P,
        range: &Self::VS,
    ) -> Result<Option<Self::V>, Self::Err> {
        // Handle root package specially - it only has version 0.0.0
        if package == "___root___" {
            return Ok(Some(SemanticVersion::zero()));
        }

        // Fetch versions using block_in_place to bridge sync trait with async client
        // Note: Direct dependencies are pre-fetched and cached, so this is typically fast.
        // Only transitive dependencies will require blocking network calls.
        let Ok(versions) = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current()
                .block_on(async { self.client.fetch_versions(package).await })
        }) else {
            return Ok(None);
        };

        // Filter by platform
        let compatible_versions: Vec<_> = versions
            .into_iter()
            .filter(|v| {
                self.platforms.is_empty()
                    || v.platform.is_empty()
                    || v.platform == "ruby"
                    || self.platforms.contains(&v.platform)
            })
            .collect();

        // Find the highest version that matches the range
        let mut matching_versions: Vec<SemanticVersion> = compatible_versions
            .iter()
            .filter_map(|v| {
                // Filter out prereleases unless explicitly allowed
                if !self.allow_prerelease && is_prerelease(&v.number) {
                    return None;
                }

                let parts: Vec<&str> = v.number.split('.').collect();
                let major = parts.first()?.parse::<u32>().ok()?;
                let minor = parts.get(1)?.parse::<u32>().ok().unwrap_or(0);
                let patch = parts.get(2)?.parse::<u32>().ok().unwrap_or(0);

                let sem_ver = SemanticVersion::new(major, minor, patch);
                if range.contains(&sem_ver) {
                    Some(sem_ver)
                } else {
                    None
                }
            })
            .collect();

        matching_versions.sort();
        Ok(matching_versions.last().copied())
    }

    fn get_dependencies(
        &self,
        package: &Self::P,
        version: &Self::V,
    ) -> Result<Dependencies<Self::P, Self::VS, Self::M>, Self::Err> {
        // Handle root package specially
        if package == "___root___" {
            let mut deps = DependencyConstraints::default();
            {
                let Ok(root_deps) = self.root_deps.read() else {
                    return Ok(Dependencies::Unavailable(
                        "internal error: lock poisoned".to_string(),
                    ));
                };
                for (name, (range, _)) in root_deps.iter() {
                    deps.insert(name.clone(), range.clone());
                }
            }
            return Ok(Dependencies::Available(deps));
        }

        // Fetch gem metadata using block_in_place to bridge sync trait with async client
        // Pre-fetching reduces the number of blocking calls needed here
        let versions = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current()
                .block_on(async { self.client.fetch_versions(package).await })
        })
        .ok();

        let Some(versions) = versions else {
            return Ok(Dependencies::Unavailable(
                "Failed to fetch gem versions".to_string(),
            ));
        };

        let version_str = version.to_string();

        // Find the specific version
        let gem_version = versions.iter().find(|v| v.number == version_str);

        let Some(gem_version) = gem_version else {
            return Ok(Dependencies::Unavailable(format!(
                "Version {version_str} not found for {package}"
            )));
        };

        // Convert runtime dependencies to PubGrub format
        let mut deps = DependencyConstraints::default();
        for dep in &gem_version.dependencies.runtime {
            // Parse version requirement
            let range = Self::parse_requirement(&dep.requirements).ok();
            if let Some(range) = range {
                deps.insert(dep.name.clone(), range);
            }
        }

        Ok(Dependencies::Available(deps))
    }
}

impl RubyGemsDependencyProvider {
    /// Parse a Ruby gem version requirement
    ///
    /// Simplified wrapper around the full requirement parser.
    /// Delegates to the comprehensive parser used by Resolver.
    fn parse_requirement(requirement: &str) -> Result<Ranges<SemanticVersion>, String> {
        // Use the full parsing logic from Resolver (without caching)
        if requirement.is_empty() || requirement == ">= 0" {
            return Ok(Ranges::full());
        }

        if requirement.starts_with("~>") {
            // Pessimistic constraint: "~> 1.2" means ">= 1.2, < 2.0"
            Resolver::parse_pessimistic_constraint(requirement).map_err(|e| e.to_string())
        } else if requirement.starts_with(">=") {
            // Greater than or equal
            let version_str = requirement.trim_start_matches(">=").trim();
            let version = Resolver::parse_semantic_version(version_str)
                .map_err(|e| format!("Invalid version in '{requirement}': {e}"))?;
            Ok(Ranges::higher_than(version))
        } else if requirement.starts_with('>') {
            // Greater than (strict)
            let version_str = requirement.trim_start_matches('>').trim();
            let version = Resolver::parse_semantic_version(version_str)
                .map_err(|e| format!("Invalid version in '{requirement}': {e}"))?;
            Ok(Ranges::strictly_higher_than(version))
        } else if requirement.starts_with("<=") {
            // Less than or equal
            let version_str = requirement.trim_start_matches("<=").trim();
            let version = Resolver::parse_semantic_version(version_str)
                .map_err(|e| format!("Invalid version in '{requirement}': {e}"))?;
            Ok(Ranges::strictly_lower_than(version.bump_patch()))
        } else if requirement.starts_with('<') {
            // Less than (strict)
            let version_str = requirement.trim_start_matches('<').trim();
            let version = Resolver::parse_semantic_version(version_str)
                .map_err(|e| format!("Invalid version in '{requirement}': {e}"))?;
            Ok(Ranges::strictly_lower_than(version))
        } else if requirement.starts_with('=') {
            // Exact version
            let version_str = requirement.trim_start_matches('=').trim();
            let version = Resolver::parse_semantic_version(version_str)
                .map_err(|e| format!("Invalid version in '{requirement}': {e}"))?;
            Ok(Ranges::singleton(version))
        } else if requirement.contains(',') {
            // Multiple constraints not fully supported here, fallback
            Ok(Ranges::full())
        } else {
            // Assume exact version if no operator
            let version = Resolver::parse_semantic_version(requirement.trim())
                .map_err(|e| format!("Invalid version '{requirement}': {e}"))?;
            Ok(Ranges::singleton(version))
        }
    }
}

/// Check if a version string indicates a prerelease version
///
/// Prerelease versions typically contain: alpha, beta, rc, pre, dev
fn is_prerelease(version: &str) -> bool {
    let version_lower = version.to_lowercase();
    version_lower.contains("alpha")
        || version_lower.contains("beta")
        || version_lower.contains("rc")
        || version_lower.contains("pre")
        || version_lower.contains("dev")
}

#[cfg(test)]
mod tests {
    use super::*;

    mod version_parsing {
        use super::*;

        #[test]
        fn semantic_version() {
            let version = Resolver::parse_semantic_version("1.2.3").unwrap();
            assert_eq!(version.to_string(), "1.2.3");

            let version = Resolver::parse_semantic_version("2.0").unwrap();
            assert_eq!(version.to_string(), "2.0.0");
        }

        #[test]
        fn semantic_version_with_prerelease() {
            let version = Resolver::parse_semantic_version("1.0.0.alpha").unwrap();
            assert!(!version.to_string().is_empty());
        }

        #[test]
        fn semantic_version_single_digit() {
            let version = Resolver::parse_semantic_version("3").unwrap();
            assert_eq!(version.to_string(), "3.0.0");
        }

        #[test]
        fn empty_constraint() {
            let resolver = Resolver::new(RubyGemsClient::new("https://rubygems.org").unwrap());
            let range = resolver.parse_version_requirement("test", "").unwrap();
            assert!(range.contains(&SemanticVersion::new(1, 0, 0)));
            assert!(range.contains(&SemanticVersion::new(999, 0, 0)));
        }

        #[test]
        fn gte_constraint() {
            let resolver = Resolver::new(RubyGemsClient::new("https://rubygems.org").unwrap());
            let range = resolver
                .parse_version_requirement("test", ">= 1.0.0")
                .unwrap();
            assert!(range.contains(&SemanticVersion::new(1, 0, 0)));
            assert!(range.contains(&SemanticVersion::new(2, 0, 0)));
            assert!(!range.contains(&SemanticVersion::new(0, 9, 0)));
        }

        #[test]
        fn gt_constraint() {
            let resolver = Resolver::new(RubyGemsClient::new("https://rubygems.org").unwrap());
            let range = resolver
                .parse_version_requirement("test", "> 1.0.0")
                .unwrap();
            assert!(!range.contains(&SemanticVersion::new(1, 0, 0)));
            assert!(range.contains(&SemanticVersion::new(1, 0, 1)));
            assert!(range.contains(&SemanticVersion::new(2, 0, 0)));
        }

        #[test]
        fn lte_constraint() {
            let resolver = Resolver::new(RubyGemsClient::new("https://rubygems.org").unwrap());
            let range = resolver
                .parse_version_requirement("test", "<= 2.0.0")
                .unwrap();
            assert!(range.contains(&SemanticVersion::new(1, 0, 0)));
            assert!(range.contains(&SemanticVersion::new(2, 0, 0)));
            assert!(!range.contains(&SemanticVersion::new(2, 0, 1)));
        }

        #[test]
        fn lt_constraint() {
            let resolver = Resolver::new(RubyGemsClient::new("https://rubygems.org").unwrap());
            let range = resolver
                .parse_version_requirement("test", "< 2.0.0")
                .unwrap();
            assert!(range.contains(&SemanticVersion::new(1, 9, 9)));
            assert!(!range.contains(&SemanticVersion::new(2, 0, 0)));
            assert!(!range.contains(&SemanticVersion::new(2, 0, 1)));
        }

        #[test]
        fn eq_constraint() {
            let resolver = Resolver::new(RubyGemsClient::new("https://rubygems.org").unwrap());
            let range = resolver
                .parse_version_requirement("test", "= 1.5.0")
                .unwrap();
            assert!(range.contains(&SemanticVersion::new(1, 5, 0)));
            assert!(!range.contains(&SemanticVersion::new(1, 5, 1)));
            assert!(!range.contains(&SemanticVersion::new(1, 4, 9)));
        }

        #[test]
        fn pessimistic_constraint_three_segments() {
            let range = Resolver::parse_pessimistic_constraint("~> 1.2.3").unwrap();
            assert!(range.contains(&SemanticVersion::new(1, 2, 3)));
            assert!(range.contains(&SemanticVersion::new(1, 2, 9)));
            assert!(!range.contains(&SemanticVersion::new(1, 3, 0)));
            assert!(!range.contains(&SemanticVersion::new(2, 0, 0)));
        }

        #[test]
        fn pessimistic_constraint_two_segments() {
            let range = Resolver::parse_pessimistic_constraint("~> 1.2").unwrap();
            assert!(range.contains(&SemanticVersion::new(1, 2, 0)));
            assert!(range.contains(&SemanticVersion::new(1, 9, 9)));
            assert!(!range.contains(&SemanticVersion::new(2, 0, 0)));
        }

        #[test]
        fn complex_constraint_with_spaces() {
            let resolver = Resolver::new(RubyGemsClient::new("https://rubygems.org").unwrap());
            let range = resolver
                .parse_version_requirement("test", ">= 1.0.0, < 2.0.0")
                .unwrap();
            assert!(range.contains(&SemanticVersion::new(1, 5, 0)));
            assert!(!range.contains(&SemanticVersion::new(0, 9, 9)));
            assert!(!range.contains(&SemanticVersion::new(2, 0, 0)));
        }

        #[test]
        fn no_equals_constraints() {
            let resolver = Resolver::new(RubyGemsClient::new("https://rubygems.org").unwrap());
            assert!(resolver.parse_version_requirement("test", ">= 1.0").is_ok());
            assert!(resolver.parse_version_requirement("test", "~> 1.0").is_ok());
        }
    }

    mod semantic_version {
        use super::*;

        #[test]
        fn display_format() {
            let v = SemanticVersion::new(2, 5, 8);
            assert_eq!(v.to_string(), "2.5.8");
        }

        #[test]
        fn comparison_ordering() {
            let v1 = SemanticVersion::new(1, 0, 0);
            let v2 = SemanticVersion::new(2, 0, 0);
            assert!(v1 < v2);
        }

        #[test]
        fn equality() {
            let v1 = SemanticVersion::new(1, 2, 3);
            let v2 = SemanticVersion::new(1, 2, 3);
            assert_eq!(v1, v2);
        }

        #[test]
        fn ordering_minor_version() {
            let v1 = SemanticVersion::new(1, 0, 0);
            let v2 = SemanticVersion::new(1, 1, 0);
            assert!(v1 < v2);
        }

        #[test]
        fn ordering_patch_version() {
            let v1 = SemanticVersion::new(1, 2, 0);
            let v2 = SemanticVersion::new(1, 2, 1);
            assert!(v1 < v2);
        }
    }
}
