//! Download and parse the complete `RubyGems` index (specs.4.8.gz).

use alox_48::{Value, from_bytes};
use anyhow::{Context, Result};
use flate2::read::GzDecoder;
use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};

/// A gem specification from the full index
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct IndexGemSpec {
    /// Gem name (e.g., "rack")
    pub name: String,

    /// Version string (e.g., "3.0.8")
    pub version: String,

    /// Platform (e.g., "ruby", "x86_64-linux")
    pub platform: String,
}

impl IndexGemSpec {
    /// Create a new index gem spec
    #[must_use]
    pub const fn new(name: String, version: String, platform: String) -> Self {
        Self {
            name,
            version,
            platform,
        }
    }

    /// Get the full gem name with version
    #[must_use]
    pub fn full_name(&self) -> String {
        if self.platform == "ruby" {
            format!("{}-{}", self.name, self.version)
        } else {
            format!("{}-{}-{}", self.name, self.version, self.platform)
        }
    }
}

/// Full `RubyGems` index
#[derive(Debug)]
pub struct FullIndex {
    /// Map of gem name to list of available versions
    specs: HashMap<String, Vec<IndexGemSpec>>,

    /// Total number of gem specs in the index
    total_count: usize,
}

impl FullIndex {
    /// Download and parse the full `RubyGems` index
    ///
    /// Downloads from `https://rubygems.org/specs.4.8.gz` by default.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Network request fails
    /// - Decompression fails
    /// - Marshal parsing fails
    pub async fn download_and_parse(base_url: &str) -> Result<Self> {
        let url = if base_url.ends_with('/') {
            format!("{base_url}specs.4.8.gz")
        } else {
            format!("{base_url}/specs.4.8.gz")
        };

        // Download compressed index
        let response = reqwest::get(&url)
            .await
            .with_context(|| format!("Failed to download full index from {url}"))?;

        let compressed_data = response
            .bytes()
            .await
            .context("Failed to read response body")?;

        // Decompress gzip data
        let mut decoder = GzDecoder::new(&compressed_data[..]);
        let mut marshal_data = Vec::new();
        decoder
            .read_to_end(&mut marshal_data)
            .context("Failed to decompress gzip data")?;

        // Parse Marshal format
        Self::parse(&marshal_data)
    }

    /// Parse Marshal data into full index
    ///
    /// # Errors
    ///
    /// Returns an error if Marshal parsing fails or data format is invalid
    pub fn parse(marshal_data: &[u8]) -> Result<Self> {
        // Parse Marshal format using alox-48
        let value: Value = from_bytes(marshal_data).context("Failed to parse Marshal data")?;

        // Extract array of specs
        let array = value
            .as_array()
            .context("Expected Marshal data to contain an array")?;

        // Parse each spec: [name, version, platform]
        let mut specs: HashMap<String, Vec<IndexGemSpec>> = HashMap::new();
        let mut total_count = 0;

        for entry in array {
            let spec = Self::parse_spec_entry(entry)?;
            specs.entry(spec.name.clone()).or_default().push(spec);
            total_count += 1;
        }

        Ok(Self { specs, total_count })
    }

    /// Parse a single spec entry from Marshal data
    ///
    /// Format: [name, version, platform]
    fn parse_spec_entry(entry: &Value) -> Result<IndexGemSpec> {
        let array = entry
            .as_array()
            .context("Expected spec entry to be an array")?;

        if array.len() != 3 {
            anyhow::bail!(
                "Expected spec entry to have 3 elements, got {}",
                array.len()
            );
        }

        // Extract name, version, platform
        let name = Self::extract_string(array.first().context("Missing name element")?, "name")?;
        let version =
            Self::extract_string(array.get(1).context("Missing version element")?, "version")?;
        let platform = Self::extract_string(
            array.get(2).context("Missing platform element")?,
            "platform",
        )?;

        Ok(IndexGemSpec::new(name, version, platform))
    }

    /// Extract string from Marshal Value
    fn extract_string(value: &Value, field_name: &str) -> Result<String> {
        // Try direct string first
        if let Some(rb_string) = value.as_string() {
            return String::from_utf8(rb_string.data.clone())
                .with_context(|| format!("Invalid UTF-8 in {field_name}"));
        }

        // Try as array (for Gem::Version objects which contain [version_string])
        if let Some(arr) = value.as_array()
            && let Some(first) = arr.first()
            && let Some(rb_string) = first.as_string()
        {
            return String::from_utf8(rb_string.data.clone())
                .with_context(|| format!("Invalid UTF-8 in {field_name}"));
        }

        // Try as object (for other wrapped values)
        if let Some(obj) = value.as_object() {
            // Try common field names
            for key in &["__value", "version", "@version", "v", "@v"] {
                let symbol = alox_48::Symbol::from(key.to_string());
                if let Some(field) = obj.fields.get(&symbol)
                    && let Some(rb_string) = field.as_string()
                {
                    return String::from_utf8(rb_string.data.clone())
                        .with_context(|| format!("Invalid UTF-8 in {field_name}"));
                }
            }
        }

        anyhow::bail!("Unable to extract string from {field_name}: unexpected format")
    }

    /// Find all versions of a gem
    #[must_use]
    pub fn find_gem(&self, name: &str) -> Option<&Vec<IndexGemSpec>> {
        self.specs.get(name)
    }

    /// Get total number of gem specs in the index
    #[must_use]
    pub const fn total_count(&self) -> usize {
        self.total_count
    }

    /// Get number of unique gems
    #[must_use]
    pub fn gem_count(&self) -> usize {
        self.specs.len()
    }

    /// Save parsed index to cache file
    ///
    /// # Errors
    ///
    /// Returns an error if file operations fail
    pub fn save_to_cache(&self, cache_path: &Path) -> Result<()> {
        let serialized =
            serde_json::to_vec(&self.specs).context("Failed to serialize index to JSON")?;

        std::fs::write(cache_path, serialized)
            .with_context(|| format!("Failed to write cache to {}", cache_path.display()))?;

        Ok(())
    }

    /// Load index from cache file
    ///
    /// # Errors
    ///
    /// Returns an error if file operations fail or JSON is invalid
    pub fn load_from_cache(cache_path: &Path) -> Result<Self> {
        let data = std::fs::read(cache_path)
            .with_context(|| format!("Failed to read cache from {}", cache_path.display()))?;

        let specs: HashMap<String, Vec<IndexGemSpec>> =
            serde_json::from_slice(&data).context("Failed to deserialize cache JSON")?;

        let total_count = specs.values().map(Vec::len).sum();

        Ok(Self { specs, total_count })
    }

    /// Get cache file path for full index
    #[must_use]
    pub fn cache_path(cache_dir: &Path) -> PathBuf {
        cache_dir.join("full_index.json")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn index_gem_spec() {
        let spec = IndexGemSpec::new("rack".to_string(), "3.0.8".to_string(), "ruby".to_string());

        assert_eq!(spec.name, "rack");
        assert_eq!(spec.version, "3.0.8");
        assert_eq!(spec.platform, "ruby");
        assert_eq!(spec.full_name(), "rack-3.0.8");
    }

    #[test]
    fn index_gem_spec_with_platform() {
        let spec = IndexGemSpec::new(
            "json".to_string(),
            "2.6.0".to_string(),
            "x86_64-linux".to_string(),
        );

        assert_eq!(spec.full_name(), "json-2.6.0-x86_64-linux");
    }

    #[test]
    fn full_index_find_gem() {
        let mut specs = HashMap::new();
        specs.insert(
            "rack".to_string(),
            vec![
                IndexGemSpec::new("rack".to_string(), "3.0.8".to_string(), "ruby".to_string()),
                IndexGemSpec::new("rack".to_string(), "3.0.7".to_string(), "ruby".to_string()),
            ],
        );

        let index = FullIndex {
            specs,
            total_count: 2,
        };

        let found = index.find_gem("rack");
        assert!(found.is_some());
        assert_eq!(found.unwrap().len(), 2);

        let not_found = index.find_gem("rails");
        assert!(not_found.is_none());
    }

    #[test]
    fn full_index_counts() {
        let mut specs = HashMap::new();
        specs.insert(
            "rack".to_string(),
            vec![IndexGemSpec::new(
                "rack".to_string(),
                "3.0.8".to_string(),
                "ruby".to_string(),
            )],
        );
        specs.insert(
            "rails".to_string(),
            vec![
                IndexGemSpec::new("rails".to_string(), "7.0.8".to_string(), "ruby".to_string()),
                IndexGemSpec::new("rails".to_string(), "7.0.7".to_string(), "ruby".to_string()),
            ],
        );

        let index = FullIndex {
            specs,
            total_count: 3,
        };

        assert_eq!(index.gem_count(), 2); // 2 unique gems
        assert_eq!(index.total_count(), 3); // 3 total specs
    }

    // NOTE: Regression tests for extract_string() are difficult to write because
    // alox_48::Value requires proper Marshal serialization. The function is tested
    // indirectly through the integration with real Marshal data from RubyGems.org.
    // Key behavior tested in production:
    // - Direct strings (gem names)
    // - Arrays with string first element (Gem::Version objects like ["1.0.0"])
    // - Objects with field access (older marshal format)
}
