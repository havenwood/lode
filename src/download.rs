//! Gem download and caching
//!
//! Manages parallel gem downloads from RubyGems.org with retry logic and caching.

use crate::lockfile::GemSpec;
use anyhow::{Context, Result};
use futures_util::StreamExt;
use std::path::{Path, PathBuf};
use std::time::Duration;
use thiserror::Error;
use tokio::io::AsyncWriteExt;

#[derive(Debug, Error)]
pub enum DownloadError {
    #[error("Gem not found: {gem} (searched {location})")]
    GemNotFound { gem: String, location: String },

    #[error("HTTP {status} error downloading {gem} from {url}")]
    HttpError {
        gem: String,
        status: u16,
        url: String,
    },

    #[error("Network error downloading {gem}: {source}")]
    NetworkError {
        gem: String,
        #[source]
        source: reqwest::Error,
    },

    #[error("Failed to write gem {gem} to cache: {source}")]
    IoError {
        gem: String,
        #[source]
        source: std::io::Error,
    },

    #[error("Failed to save gem {gem} to cache: {source}")]
    TempFileError {
        gem: String,
        #[source]
        source: tempfile::PersistError,
    },
}

impl DownloadError {
    /// Wrap an IO error with gem context for use in `map_err`
    pub fn wrap_io(gem_name: impl Into<String>) -> impl Fn(std::io::Error) -> Self {
        let gem = gem_name.into();
        move |source| Self::IoError {
            gem: gem.clone(),
            source,
        }
    }

    /// Wrap a network error with gem context for use in `map_err`
    pub fn wrap_network(gem_name: impl Into<String>) -> impl Fn(reqwest::Error) -> Self {
        let gem = gem_name.into();
        move |source| Self::NetworkError {
            gem: gem.clone(),
            source,
        }
    }

    /// Wrap a temp file error with gem context for use in `map_err`
    pub fn wrap_tempfile(gem_name: impl Into<String>) -> impl Fn(tempfile::PersistError) -> Self {
        let gem = gem_name.into();
        move |source| Self::TempFileError {
            gem: gem.clone(),
            source,
        }
    }
}

/// Manages gem downloads with caching
#[derive(Clone)]
pub struct DownloadManager {
    cache_dir: PathBuf,
    client: reqwest::Client,
    sources: Vec<String>,
    max_retries: usize,
    skip_cache: bool,
    local_only: bool,
}

impl std::fmt::Debug for DownloadManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DownloadManager")
            .field("cache_dir", &self.cache_dir)
            .field("sources", &self.sources)
            .field("max_retries", &self.max_retries)
            .finish_non_exhaustive()
    }
}

impl DownloadManager {
    /// Create a new download manager with default source (gems.coop)
    ///
    /// # Errors
    ///
    /// Returns an error if the cache directory cannot be created or the HTTP client cannot be built.
    pub fn new(cache_dir: PathBuf) -> Result<Self> {
        Self::with_sources_and_retry(cache_dir, vec![crate::DEFAULT_GEM_SOURCE.to_string()], 0)
    }

    /// Create a new download manager with custom gem sources
    ///
    /// Sources are tried in order until a gem is found (fallback behavior).
    ///
    /// # Arguments
    /// * `cache_dir` - Directory to cache downloaded gems
    /// * `sources` - List of gem sources to try (e.g., `["https://rubygems.org", "https://gems.contoso.com"]`)
    ///
    /// # Errors
    ///
    /// Returns an error if the cache directory cannot be created or the HTTP client cannot be built.
    pub fn with_sources(cache_dir: PathBuf, sources: Vec<String>) -> Result<Self> {
        Self::with_sources_and_retry(cache_dir, sources, 0)
    }

    /// Create a new download manager with custom gem sources and retry configuration
    ///
    /// Sources are tried in order until a gem is found (fallback behavior).
    ///
    /// # Arguments
    /// * `cache_dir` - Directory to cache downloaded gems
    /// * `sources` - List of gem sources to try (e.g., `["https://rubygems.org", "https://gems.contoso.com"]`)
    /// * `max_retries` - Number of times to retry failed downloads (0 for no retries)
    ///
    /// # Errors
    ///
    /// Returns an error if the cache directory cannot be created or the HTTP client cannot be built.
    pub fn with_sources_and_retry(
        cache_dir: PathBuf,
        sources: Vec<String>,
        max_retries: usize,
    ) -> Result<Self> {
        std::fs::create_dir_all(&cache_dir).context("Failed to create cache directory")?;

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(60))
            .user_agent(format!("lode/{}", env!("CARGO_PKG_VERSION")))
            .build()?;

        let sources = if sources.is_empty() {
            vec![crate::DEFAULT_GEM_SOURCE.to_string()]
        } else {
            sources
        };

        Ok(Self {
            cache_dir,
            client,
            sources,
            max_retries,
            skip_cache: false,
            local_only: false,
        })
    }

    /// Set whether to skip cache (always fetch fresh)
    #[must_use]
    pub const fn with_skip_cache(mut self, skip_cache: bool) -> Self {
        self.skip_cache = skip_cache;
        self
    }

    /// Set whether to use local cache only (don't download from remote)
    #[must_use]
    pub const fn with_local_only(mut self, local_only: bool) -> Self {
        self.local_only = local_only;
        self
    }

    /// Download a gem to the cache.
    ///
    /// Returns the cached gem path. Reuses existing cached files.
    ///
    /// Tries all configured sources with retry logic on network errors.
    ///
    /// # Errors
    ///
    /// Returns an error if the download fails, the network is unavailable, or the gem cannot be found on any source.
    pub async fn download_gem(&self, spec: &GemSpec) -> Result<PathBuf, DownloadError> {
        let filename = format!("{}.gem", spec.full_name_with_platform());
        let cache_path = self.cache_dir.join(&filename);

        // Check if already cached (unless skip_cache is enabled)
        if !self.skip_cache && cache_path.exists() {
            return Ok(cache_path);
        }

        // If local_only is set and gem not in cache, return error
        if self.local_only {
            return Err(DownloadError::GemNotFound {
                gem: spec.full_name_with_platform().to_string(),
                location: "local cache".to_string(),
            });
        }

        // Try each source in order
        let mut last_error = None;
        for source in &self.sources {
            let url = format!("{source}/downloads/{filename}");

            // Attempt download with retry
            let mut network_error = None;
            for attempt in 0..=self.max_retries {
                match self.client.get(&url).send().await {
                    Ok(response) => {
                        let status = response.status();

                        // Check for 404 - try next source
                        if status.as_u16() == 404 {
                            last_error = Some(DownloadError::GemNotFound {
                                gem: spec.full_name_with_platform().to_string(),
                                location: source.clone(),
                            });
                            break; // Break retry loop, try next source
                        }

                        // Other HTTP errors fail immediately
                        if !status.is_success() {
                            return Err(DownloadError::HttpError {
                                gem: spec.name.clone(),
                                status: status.as_u16(),
                                url,
                            });
                        }

                        // Success! Download the gem
                        return self
                            .download_from_response(response, spec, cache_path.clone())
                            .await;
                    }
                    Err(e) => {
                        network_error = Some(e);
                        if attempt < self.max_retries {
                            // Wait before retrying (exponential backoff)
                            let delay = Duration::from_millis(100 * 2_u64.pow(attempt as u32));
                            tokio::time::sleep(delay).await;
                        }
                    }
                }
            }

            // If we had a network error after all retries, return it
            if let Some(e) = network_error {
                return Err(DownloadError::NetworkError {
                    gem: spec.name.clone(),
                    source: e,
                });
            }
        }

        // All sources exhausted
        Err(last_error.unwrap_or_else(|| DownloadError::GemNotFound {
            gem: spec.full_name_with_platform().to_string(),
            location: "No gem sources configured".to_string(),
        }))
    }

    /// Download gem from a successful HTTP response
    async fn download_from_response(
        &self,
        response: reqwest::Response,
        spec: &GemSpec,
        cache_path: PathBuf,
    ) -> Result<PathBuf, DownloadError> {
        // Stream to temporary file
        let temp_file = tempfile::NamedTempFile::new_in(&self.cache_dir)
            .map_err(DownloadError::wrap_io(&spec.name))?;

        {
            let file_std = temp_file
                .as_file()
                .try_clone()
                .map_err(DownloadError::wrap_io(&spec.name))?;
            let mut file = tokio::fs::File::from_std(file_std);

            let mut stream = response.bytes_stream();
            while let Some(chunk_result) = stream.next().await {
                let chunk = chunk_result.map_err(DownloadError::wrap_network(&spec.name))?;
                file.write_all(&chunk)
                    .await
                    .map_err(DownloadError::wrap_io(&spec.name))?;
            }

            file.flush()
                .await
                .map_err(DownloadError::wrap_io(&spec.name))?;
        } // File is closed here

        // Atomic rename
        temp_file
            .persist(&cache_path)
            .map_err(DownloadError::wrap_tempfile(&spec.name))?;

        Ok(cache_path)
    }

    /// Get the cache directory path
    #[must_use]
    pub fn cache_dir(&self) -> &Path {
        &self.cache_dir
    }

    /// Compute SHA256 checksum of a gem file
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or hashed
    pub fn compute_checksum(gem_path: &Path) -> Result<String> {
        use sha2::{Digest, Sha256};
        use std::io::Read;

        let mut file = std::fs::File::open(gem_path).with_context(|| {
            format!(
                "Failed to open gem file for checksum: {}",
                gem_path.display()
            )
        })?;

        let mut hasher = Sha256::new();
        let mut buffer = [0; 8192];

        loop {
            let count = file.read(&mut buffer).with_context(|| {
                format!(
                    "Failed to read gem file for checksum: {}",
                    gem_path.display()
                )
            })?;
            if count == 0 {
                break;
            }
            hasher.update(buffer.get(..count).unwrap_or(&[]));
        }

        let result = hasher.finalize();
        Ok(format!("{result:x}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn download_manager_creation() -> Result<()> {
        let temp_dir = tempfile::tempdir().context("Failed to create temp dir")?;
        let dm = DownloadManager::new(temp_dir.path().to_path_buf())?;
        assert!(dm.cache_dir().exists());
        Ok(())
    }

    #[test]
    fn test_compute_checksum() -> Result<()> {
        use std::io::Write;

        let temp_dir = tempfile::tempdir().context("Failed to create temp dir")?;
        let test_file = temp_dir.path().join("test.gem");

        let mut file = std::fs::File::create(&test_file)?;
        file.write_all(b"test content")?;
        file.sync_all()?;
        drop(file);

        let checksum = DownloadManager::compute_checksum(&test_file)?;

        assert_eq!(
            checksum,
            "6ae8a75555209fd6c44157c0aed8016e763ff435a19cf186f76863140143ff72"
        );

        Ok(())
    }

    #[test]
    fn compute_checksum_empty_file() -> Result<()> {
        let temp_dir = tempfile::tempdir().context("Failed to create temp dir")?;
        let test_file = temp_dir.path().join("empty.gem");

        std::fs::File::create(&test_file)?;

        let checksum = DownloadManager::compute_checksum(&test_file)?;

        assert_eq!(
            checksum,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );

        Ok(())
    }
}
