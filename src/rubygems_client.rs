//! HTTP client for RubyGems.org API with cached metadata lookups.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Read;
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;

/// Errors that can occur when fetching gem metadata
#[derive(Debug, Error)]
pub enum RubyGemsError {
    #[error("Gem not found: {gem}")]
    GemNotFound { gem: String },

    #[error("HTTP {status} error fetching {gem} from {url}")]
    HttpError {
        gem: String,
        status: u16,
        url: String,
    },

    #[error("Network error fetching {gem}: {source}")]
    NetworkError {
        gem: String,
        #[source]
        source: reqwest::Error,
    },

    #[error("Failed to parse response for {gem}: {source}")]
    ParseError {
        gem: String,
        #[source]
        source: serde_json::Error,
    },
}

/// Represents a gem version with its dependencies
///
/// Metadata returned by RubyGems.org for each version (similar to
/// `gem specification rails --remote`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GemVersion {
    /// Version number (e.g., "7.0.8")
    pub number: String,

    /// Platform (e.g., "ruby", "x86_64-linux")
    #[serde(default)]
    pub platform: String,

    /// Ruby version requirement (e.g., ">= 2.7.0")
    #[serde(default)]
    pub ruby_version: Option<String>,

    /// Dependencies for this version
    #[serde(default)]
    pub dependencies: Dependencies,
}

/// Dependencies grouped by type
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Dependencies {
    /// Runtime dependencies (required for the gem to work)
    #[serde(default)]
    pub runtime: Vec<DependencySpec>,

    /// Development dependencies (only needed for development)
    #[serde(default)]
    pub development: Vec<DependencySpec>,
}

/// A single dependency specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencySpec {
    /// Dependency name
    pub name: String,

    /// Version requirements
    pub requirements: String,
}

/// Bulk gem specification from specs.4.8.gz index
///
/// This represents a single entry in the bulk gem index, which contains
/// basic information about all gems available on the server.
#[derive(Debug, Clone)]
pub struct BulkGemSpec {
    /// Gem name
    pub name: String,

    /// Version string
    pub version: String,

    /// Platform (e.g., "ruby", "x86_64-linux")
    pub platform: String,
}

/// API response for gem versions endpoint
#[derive(Debug, Deserialize)]
#[allow(dead_code, reason = "Used for JSON deserialization")]
struct VersionsResponse {
    #[serde(default)]
    versions: Vec<GemVersion>,
}

/// Client for interacting with RubyGems.org API
///
/// Handles HTTP requests to fetch gem metadata. The `reqwest` client provides
/// connection pooling and automatic retry logic. Response caching reduces
/// redundant API calls during dependency resolution.
#[derive(Debug, Clone)]
pub struct RubyGemsClient {
    /// Base URL for the gem server (e.g., <https://rubygems.org>)
    base_url: String,

    /// HTTP client with connection pooling
    client: reqwest::Client,

    /// Response cache to avoid redundant API calls
    /// Key: gem name, Value: Arc-wrapped versions list (cheap to clone)
    /// Wrapped in Arc to allow cloning the client
    cache: Arc<tokio::sync::RwLock<HashMap<String, Arc<Vec<GemVersion>>>>>,

    /// Bulk gem index cache (specs.4.8.gz)
    /// Downloaded once per client lifetime for "list all" operations
    /// `Arc<Mutex>` allows thread-safe access and cloning
    bulk_index_cache: Arc<tokio::sync::Mutex<Option<Vec<BulkGemSpec>>>>,

    /// Only use cached gems, no network requests (--local mode)
    cache_only: bool,

    /// Include prerelease versions (--pre mode)
    include_prerelease: bool,
}

impl RubyGemsClient {
    /// Create a new `RubyGems` API client.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP client cannot be built.
    ///
    /// # Example
    ///
    /// ```rust
    /// use lode::rubygems_client::RubyGemsClient;
    ///
    /// let client = RubyGemsClient::new("https://rubygems.org")?;
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    pub fn new(base_url: impl Into<String>) -> Result<Self> {
        Self::new_with_proxy(base_url, None::<String>)
    }

    /// Create a new `RubyGems` API client with optional proxy override.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP client cannot be built.
    ///
    /// # Example
    ///
    /// ```rust
    /// use lode::rubygems_client::RubyGemsClient;
    ///
    /// // Use environment variable
    /// let client1 = RubyGemsClient::new_with_proxy("https://rubygems.org", None)?;
    ///
    /// // Override with specific proxy
    /// let client2 = RubyGemsClient::new_with_proxy("https://rubygems.org", Some("http://proxy:8080"))?;
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    pub fn new_with_proxy(
        base_url: impl Into<String>,
        proxy_url: Option<impl Into<String>>,
    ) -> Result<Self> {
        let timeout_secs = crate::env_vars::bundle_timeout();

        let user_agent = crate::env_vars::bundle_user_agent()
            .unwrap_or_else(|| format!("lode/{}", env!("CARGO_PKG_VERSION")));

        let mut builder = reqwest::Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .user_agent(user_agent)
            .pool_max_idle_per_host(10) // Connection pooling
            .redirect(reqwest::redirect::Policy::limited(
                crate::env_vars::bundle_redirect(),
            )); // Limit redirects for security

        // Add proxy support if configured (parameter overrides environment variable)
        let effective_proxy_url = proxy_url
            .map(Into::into)
            .or_else(crate::env_vars::http_proxy);

        if let Some(proxy_url) = effective_proxy_url {
            let mut proxy = reqwest::Proxy::all(&proxy_url)
                .with_context(|| format!("Invalid proxy URL: {proxy_url}"))?;

            // Check for HTTPS-specific credentials first, then fall back to HTTP credentials
            let proxy_user =
                crate::env_vars::https_proxy_user().or_else(crate::env_vars::http_proxy_user);
            let proxy_pass =
                crate::env_vars::https_proxy_pass().or_else(crate::env_vars::http_proxy_pass);

            if let (Some(user), Some(pass)) = (proxy_user, proxy_pass) {
                proxy = proxy.basic_auth(&user, &pass);
            }

            if let Some(no_proxy) = crate::env_vars::no_proxy() {
                proxy = proxy.no_proxy(reqwest::NoProxy::from_string(&no_proxy));
            }

            builder = builder.proxy(proxy);
        }

        if let Some(ca_cert_path) = crate::env_vars::bundle_ssl_ca_cert() {
            let cert_bytes = std::fs::read(&ca_cert_path)
                .with_context(|| format!("Failed to read SSL CA cert from {ca_cert_path}"))?;
            let cert = reqwest::Certificate::from_pem(&cert_bytes)
                .context("Failed to parse SSL CA certificate")?;
            builder = builder.add_root_certificate(cert);
        }

        if let Some(client_cert_path) = crate::env_vars::bundle_ssl_client_cert() {
            let cert_bytes = std::fs::read(&client_cert_path).with_context(|| {
                format!("Failed to read SSL client cert from {client_cert_path}")
            })?;
            let identity = reqwest::Identity::from_pem(&cert_bytes)
                .context("Failed to parse SSL client certificate")?;
            builder = builder.identity(identity);
        }

        if let Some(verify_mode) = crate::env_vars::bundle_ssl_verify_mode() {
            match verify_mode.to_lowercase().as_str() {
                "none" => {
                    builder = builder.danger_accept_invalid_certs(true);
                }
                "peer" => {}
                _ => {
                    anyhow::bail!(
                        "Invalid BUNDLE_SSL_VERIFY_MODE: {verify_mode}. Expected 'none' or 'peer'"
                    );
                }
            }
        }

        let client = builder.build().context("Failed to build HTTP client")?;

        Ok(Self {
            base_url: base_url.into(),
            client,
            cache: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            bulk_index_cache: Arc::new(tokio::sync::Mutex::new(None)),
            cache_only: false,
            include_prerelease: false,
        })
    }

    /// Enable cache-only mode (no network requests)
    ///
    /// Mirrors Bundler's `--local` flag behavior.
    ///
    /// # Example
    ///
    /// ```rust
    /// use lode::rubygems_client::RubyGemsClient;
    ///
    /// let client = RubyGemsClient::new("https://rubygems.org")?
    ///     .with_cache_only(true);
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    #[must_use]
    pub const fn with_cache_only(mut self, cache_only: bool) -> Self {
        self.cache_only = cache_only;
        self
    }

    /// Enable prerelease versions (alpha, beta, rc, etc.)
    ///
    /// By default, prerelease versions are excluded from resolution.
    /// This mirrors Bundler's `--pre` flag behavior.
    ///
    /// # Example
    ///
    /// ```rust
    /// use lode::rubygems_client::RubyGemsClient;
    ///
    /// let client = RubyGemsClient::new("https://rubygems.org")?
    ///     .with_prerelease(true);
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    #[must_use]
    pub const fn with_prerelease(mut self, include_prerelease: bool) -> Self {
        self.include_prerelease = include_prerelease;
        self
    }

    /// Fetch all available versions of a gem
    ///
    /// Similar to running `gem list rails --remote --all`. Results are cached in
    /// memory to avoid redundant API calls during dependency resolution (which may
    /// query the same gem multiple times).
    ///
    /// # Errors
    ///
    /// Returns an error if the gem doesn't exist or the network request fails.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use lode::rubygems_client::RubyGemsClient;
    /// # async fn example() -> anyhow::Result<()> {
    /// let client = RubyGemsClient::new("https://rubygems.org")?;
    /// let versions = client.fetch_versions("rails").await?;
    ///
    /// for version in versions {
    ///     println!("rails {}", version.number);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn fetch_versions(&self, gem_name: &str) -> Result<Vec<GemVersion>, RubyGemsError> {
        // Check cache first (Arc makes this cheap)
        {
            let cache = self.cache.read().await;
            if let Some(versions) = cache.get(gem_name) {
                let mut result = (**versions).clone();

                // Filter out prerelease versions unless explicitly requested
                if !self.include_prerelease {
                    result.retain(|v| !Self::is_prerelease(&v.number));
                }

                return Ok(result);
            }
        }

        if self.cache_only {
            return Err(RubyGemsError::GemNotFound {
                gem: gem_name.to_string(),
            });
        }

        let url = format!("{}/api/v1/versions/{}.json", self.base_url, gem_name);

        let response =
            self.client
                .get(&url)
                .send()
                .await
                .map_err(|e| RubyGemsError::NetworkError {
                    gem: gem_name.to_string(),
                    source: e,
                })?;

        let status = response.status();
        if status == reqwest::StatusCode::NOT_FOUND {
            return Err(RubyGemsError::GemNotFound {
                gem: gem_name.to_string(),
            });
        }

        if !status.is_success() {
            return Err(RubyGemsError::HttpError {
                gem: gem_name.to_string(),
                status: status.as_u16(),
                url,
            });
        }

        let text = response
            .text()
            .await
            .map_err(|e| RubyGemsError::NetworkError {
                gem: gem_name.to_string(),
                source: e,
            })?;

        let versions: Vec<GemVersion> =
            serde_json::from_str(&text).map_err(|e| RubyGemsError::ParseError {
                gem: gem_name.to_string(),
                source: e,
            })?;

        // Cache the result (Arc reduces cloning overhead)
        let versions_arc = Arc::new(versions);
        {
            let mut cache = self.cache.write().await;
            cache.insert(gem_name.to_string(), Arc::clone(&versions_arc));
        }

        let mut result = (*versions_arc).clone();

        // Filter out prerelease versions unless explicitly requested
        if !self.include_prerelease {
            result.retain(|v| !Self::is_prerelease(&v.number));
        }

        Ok(result)
    }

    /// Check if a version string is a prerelease
    ///
    /// Prerelease versions contain a hyphen (e.g., "1.0.0-alpha", "1.0.0-beta.1")
    fn is_prerelease(version: &str) -> bool {
        version.contains('-')
    }

    /// Fetch metadata for a specific version of a gem
    ///
    /// More detailed than `fetch_versions` but slower. Use `fetch_versions` for
    /// dependency resolution and this only when you need detailed metadata.
    pub async fn fetch_gem_info(
        &self,
        gem_name: &str,
        version: &str,
    ) -> Result<GemMetadata, RubyGemsError> {
        let url = format!(
            "{}/api/v2/rubygems/{}/versions/{}.json",
            self.base_url, gem_name, version
        );

        let response =
            self.client
                .get(&url)
                .send()
                .await
                .map_err(|e| RubyGemsError::NetworkError {
                    gem: gem_name.to_string(),
                    source: e,
                })?;

        let status = response.status();
        if status == reqwest::StatusCode::NOT_FOUND {
            return Err(RubyGemsError::GemNotFound {
                gem: format!("{gem_name}-{version}"),
            });
        }

        if !status.is_success() {
            return Err(RubyGemsError::HttpError {
                gem: gem_name.to_string(),
                status: status.as_u16(),
                url,
            });
        }

        let text = response
            .text()
            .await
            .map_err(|e| RubyGemsError::NetworkError {
                gem: gem_name.to_string(),
                source: e,
            })?;

        // If response is empty or just whitespace, treat as not found
        if text.trim().is_empty() {
            return Err(RubyGemsError::GemNotFound {
                gem: format!("{gem_name}-{version}"),
            });
        }

        serde_json::from_str(&text).map_err(|e| RubyGemsError::ParseError {
            gem: gem_name.to_string(),
            source: e,
        })
    }

    /// Fetch the bulk gem index (`specs.4.8.gz` or `prerelease_specs.4.8.gz`).
    ///
    /// This downloads and parses the complete gem index, which contains basic
    /// information (name, version, platform) for all gems on the server.
    /// The index is cached for the lifetime of the client.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The download fails
    /// - The file cannot be decompressed
    /// - The Marshal data cannot be parsed
    ///
    /// # Performance
    ///
    /// The compressed file is ~5.6MB and decompresses to ~40MB. Downloading and parsing
    /// takes a few seconds on typical connections. Results are cached in memory.
    pub async fn fetch_bulk_index(&self, include_prerelease: bool) -> Result<Vec<BulkGemSpec>> {
        // Check cache first
        {
            let cache_guard = self.bulk_index_cache.lock().await;
            if let Some(cached) = cache_guard.as_ref() {
                return Ok(cached.clone());
            }
        }

        let index_file = if include_prerelease {
            "prerelease_specs.4.8.gz"
        } else {
            "specs.4.8.gz"
        };

        let url = format!("{}/{}", self.base_url, index_file);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to download bulk gem index")?;

        if !response.status().is_success() {
            anyhow::bail!("Failed to download bulk index: HTTP {}", response.status());
        }

        // Get the compressed bytes
        let compressed_bytes = response
            .bytes()
            .await
            .context("Failed to read bulk index response")?;

        // Decompress with flate2
        let mut decoder = flate2::read::GzDecoder::new(&compressed_bytes[..]);
        let mut decompressed = Vec::new();
        decoder
            .read_to_end(&mut decompressed)
            .context("Failed to decompress bulk gem index")?;

        // Parse Marshal data
        let marshal_value = alox_48::from_bytes(&decompressed)
            .map_err(|e| anyhow::anyhow!("Failed to parse Marshal data: {e}"))?;

        // Convert Marshal array to Vec<BulkGemSpec>
        let specs = Self::parse_marshal_specs(&marshal_value)
            .context("Failed to parse gem specifications from Marshal data")?;

        // Cache the results
        {
            let mut cache_guard = self.bulk_index_cache.lock().await;
            *cache_guard = Some(specs.clone());
        }

        Ok(specs)
    }

    /// Parse Marshal array of gem specifications
    ///
    /// The Marshal data is an array of [name, version, platform] tuples.
    /// Example: `[["rails", "7.0.8", "ruby"], ["rack", "3.0.0", "ruby"], ...]`
    fn parse_marshal_specs(value: &alox_48::Value) -> Result<Vec<BulkGemSpec>> {
        // The top level should be an array
        let specs_array = value
            .as_array()
            .context("Expected Marshal array at top level")?;

        let mut result = Vec::with_capacity(specs_array.len());

        for spec_value in specs_array {
            // Each spec is an array: [name, version, platform]
            let Some(spec_parts) = spec_value.as_array() else {
                continue; // Skip malformed entries
            };

            if spec_parts.len() < 3 {
                continue; // Skip incomplete entries
            }

            // Extract name (String)
            let Some(name) = spec_parts
                .first()
                .and_then(|v| v.as_string())
                .map(|rb_str| String::from_utf8(rb_str.data.clone()))
                .transpose()
                .context("Invalid UTF-8 in gem name")?
            else {
                continue;
            };

            // Extract version (could be String or Array for Gem::Version)
            let Some(version_value) = spec_parts.get(1) else {
                continue;
            };

            let version = if let Some(rb_str) = version_value.as_string() {
                String::from_utf8(rb_str.data.clone()).context("Invalid UTF-8 in version field")?
            } else if let Some(arr) = version_value.as_array() {
                // Gem::Version is represented as an array with version string as first element
                let Some(version_str) = arr
                    .first()
                    .and_then(|v| v.as_string())
                    .map(|rb_str| String::from_utf8(rb_str.data.clone()))
                    .transpose()
                    .context("Invalid UTF-8 in version array")?
                else {
                    continue;
                };
                version_str
            } else if let Some(obj) = version_value.as_object() {
                // Try common field names for version objects
                let keys = ["__value", "@version", "version"];
                let Some(version_str) = keys
                    .iter()
                    .find_map(|&key| {
                        let symbol = alox_48::Symbol::from(key.to_string());
                        obj.fields.get(&symbol).and_then(|v| v.as_string())
                    })
                    .map(|rb_str| String::from_utf8(rb_str.data.clone()))
                    .transpose()
                    .context("Invalid UTF-8 in version object")?
                else {
                    continue;
                };
                version_str
            } else {
                continue;
            };

            // Extract platform (String)
            let platform = spec_parts
                .get(2)
                .and_then(|v| v.as_string())
                .map(|rb_str| String::from_utf8(rb_str.data.clone()))
                .transpose()
                .context("Invalid UTF-8 in platform field")?
                .unwrap_or_else(|| "ruby".to_string());

            result.push(BulkGemSpec {
                name,
                version,
                platform,
            });
        }

        Ok(result)
    }

    /// Search the bulk index for gems matching a pattern.
    ///
    /// Convenience method that fetches the bulk index if needed, then filters it
    /// based on the provided pattern. Returns all gems whose names start with the pattern.
    ///
    /// # Errors
    ///
    /// Returns an error if the bulk index cannot be downloaded or parsed.
    pub async fn search_bulk_index(
        &self,
        pattern: &str,
        include_prerelease: bool,
    ) -> Result<Vec<BulkGemSpec>> {
        let index = self.fetch_bulk_index(include_prerelease).await?;

        // Filter by pattern (case-insensitive prefix match)
        let pattern_lower = pattern.to_lowercase();
        let results: Vec<BulkGemSpec> = index
            .into_iter()
            .filter(|spec| spec.name.to_lowercase().starts_with(&pattern_lower))
            .collect();

        Ok(results)
    }

    /// Clear the response cache
    ///
    /// Useful for forcing fresh API calls, for example after a long-running operation.
    pub async fn clear_cache(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }

    /// Get cache statistics
    #[must_use]
    pub async fn cache_stats(&self) -> CacheStats {
        let cache = self.cache.read().await;
        CacheStats {
            entries: cache.len(),
            gems_cached: cache.keys().cloned().collect(),
        }
    }
}

/// Detailed gem metadata (for gem info command)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GemMetadata {
    pub name: String,
    /// Note: API returns "number" field, but we alias it to "version" via #[serde(alias)]
    #[serde(alias = "number")]
    pub version: String,
    pub platform: String,
    /// Authors as a single string (comma-separated if multiple)
    pub authors: String,
    pub description: Option<String>,
    pub summary: Option<String>,
    /// Homepage URL (API uses both "`homepage_uri`" and "homepage")
    #[serde(alias = "homepage_uri")]
    pub homepage: Option<String>,
    pub licenses: Vec<String>,
    pub dependencies: Dependencies,
    /// Post-install message (displayed after gem installation)
    #[serde(alias = "post_install_message")]
    pub post_install_message: Option<String>,
}

/// Cache statistics
#[derive(Debug)]
pub struct CacheStats {
    /// Number of cached entries
    pub entries: usize,

    /// List of gem names in cache
    pub gems_cached: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_creation() {
        let client = RubyGemsClient::new("https://rubygems.org")
            .expect("should create rubygems client for test");
        assert_eq!(client.base_url, "https://rubygems.org");
    }

    #[tokio::test]
    async fn test_cache_stats() {
        let client = RubyGemsClient::new("https://rubygems.org")
            .expect("should create rubygems client for test");
        let stats = client.cache_stats().await;
        assert_eq!(stats.entries, 0);
    }

    #[tokio::test]
    async fn test_clear_cache() {
        let client = RubyGemsClient::new("https://rubygems.org")
            .expect("should create rubygems client for test");
        client.clear_cache().await;
        let stats = client.cache_stats().await;
        assert_eq!(stats.entries, 0);
    }

    // Integration test with real RubyGems.org (network required)
    #[tokio::test]
    #[ignore = "requires network and downloads large file"]
    async fn test_fetch_bulk_index() {
        let client = RubyGemsClient::new("https://rubygems.org")
            .expect("should create rubygems client for test");

        // Fetch bulk index (this downloads ~5.6MB compressed)
        let index = client
            .fetch_bulk_index(false)
            .await
            .expect("should download and parse bulk index");

        // Verify we got a substantial number of gems
        assert!(
            index.len() > 100_000,
            "Expected >100k gems, got {}",
            index.len()
        );

        // Verify structure by checking a well-known gem (rails should exist)
        assert!(
            index.iter().any(|spec| spec.name == "rails"),
            "Expected to find 'rails' in bulk index"
        );

        // Verify the cache works (second call should be instant)
        let index2 = client
            .fetch_bulk_index(false)
            .await
            .expect("should get cached bulk index");
        assert_eq!(index.len(), index2.len(), "Cache should return same data");
    }

    // Test search functionality
    #[tokio::test]
    #[ignore = "requires network"]
    async fn test_search_bulk_index() {
        let client = RubyGemsClient::new("https://rubygems.org")
            .expect("should create rubygems client for test");

        // Search for gems starting with "rack"
        let results = client
            .search_bulk_index("rack", false)
            .await
            .expect("should search bulk index");

        // Should find multiple rack-related gems
        assert!(
            results.len() > 10,
            "Expected >10 gems matching 'rack', got {}",
            results.len()
        );

        // Verify results match the pattern
        for spec in &results {
            assert!(
                spec.name.to_lowercase().starts_with("rack"),
                "Result '{}' should start with 'rack'",
                spec.name
            );
        }

        // Verify "rack" itself is in the results
        assert!(
            results.iter().any(|spec| spec.name == "rack"),
            "Expected to find 'rack' gem in search results"
        );
    }

    #[test]
    fn base_url_validation() {
        let client =
            RubyGemsClient::new("https://rubygems.org").expect("should create with https url");
        assert_eq!(client.base_url, "https://rubygems.org");
    }

    #[test]
    fn base_url_with_trailing_slash() {
        let client = RubyGemsClient::new("https://rubygems.org/")
            .expect("should create with trailing slash");
        assert!(!client.base_url.is_empty());
    }

    #[tokio::test]
    async fn cache_stats_empty() {
        let client = RubyGemsClient::new("https://rubygems.org").expect("should create client");
        let stats = client.cache_stats().await;
        assert_eq!(stats.entries, 0);
        assert!(stats.gems_cached.is_empty());
    }

    #[tokio::test]
    async fn cache_clear_idempotent() {
        let client = RubyGemsClient::new("https://rubygems.org").expect("should create client");

        client.clear_cache().await;
        let stats = client.cache_stats().await;
        assert_eq!(stats.entries, 0);

        client.clear_cache().await;
        let stats = client.cache_stats().await;
        assert_eq!(stats.entries, 0);
    }

    #[test]
    fn gem_metadata_creates_valid() {
        let metadata = GemMetadata {
            name: "test".to_string(),
            version: "1.0.0".to_string(),
            platform: "ruby".to_string(),
            authors: "Author".to_string(),
            description: Some("Test gem".to_string()),
            summary: Some("A test".to_string()),
            homepage: Some("https://example.com".to_string()),
            licenses: vec!["MIT".to_string()],
            dependencies: Dependencies {
                runtime: vec![],
                development: vec![],
            },
            post_install_message: None,
        };
        assert_eq!(metadata.name, "test");
        assert_eq!(metadata.licenses.len(), 1);
    }

    #[test]
    fn gem_metadata_optional_fields() {
        let metadata = GemMetadata {
            name: "minimal".to_string(),
            version: "0.1.0".to_string(),
            platform: "ruby".to_string(),
            authors: String::new(),
            description: None,
            summary: None,
            homepage: None,
            licenses: vec![],
            dependencies: Dependencies {
                runtime: vec![],
                development: vec![],
            },
            post_install_message: None,
        };
        assert!(metadata.description.is_none());
        assert!(metadata.homepage.is_none());
        assert!(metadata.licenses.is_empty());
    }

    #[test]
    fn cache_stats_operations() {
        let stats = CacheStats {
            entries: 5,
            gems_cached: vec!["rack".to_string(), "rails".to_string()],
        };
        assert_eq!(stats.entries, 5);
        assert_eq!(stats.gems_cached.len(), 2);
        assert!(stats.gems_cached.contains(&"rack".to_string()));
    }

    #[test]
    fn cache_stats_empty_struct() {
        let stats = CacheStats {
            entries: 0,
            gems_cached: vec![],
        };
        assert_eq!(stats.entries, 0);
        assert!(stats.gems_cached.is_empty());
    }

    #[test]
    fn test_invalid_utf8_in_marshal_raises_error() {
        // Marshal array: [["rails", <invalid-utf8>, "ruby"]]
        let marshal_bytes = vec![
            0x04, 0x08, // Marshal version 4.8
            0x5b, 0x06, // Array with 1 element
            0x5b, 0x08, // Nested array with 3 elements
            0x22, 0x0a, b'r', b'a', b'i', b'l', b's', // "rails"
            0x22, 0x09, b'1', b'.', 0xFF, 0xFE, // Invalid UTF-8
            0x22, 0x09, b'r', b'u', b'b', b'y', // "ruby"
        ];

        if let Ok(value) = alox_48::from_bytes(&marshal_bytes) {
            let parse_result = RubyGemsClient::parse_marshal_specs(&value);
            assert!(parse_result.is_err(), "Expected error for invalid UTF-8");
            let err_msg = format!("{:?}", parse_result.unwrap_err());
            assert!(
                err_msg.contains("Invalid UTF-8"),
                "Error should mention UTF-8"
            );
        }
    }
}
