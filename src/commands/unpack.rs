//! Unpack command
//!
//! Extract gem source to current directory

use anyhow::{Context, Result};
use flate2::read::GzDecoder;
use lode::{Config, DownloadManager, GemSpec, Lockfile, config};
use std::fs;
use std::path::{Path, PathBuf};
use tar::Archive;

/// Unpack a gem to the current directory.
///
/// Downloads the gem if needed, then extracts it to `./<gem-name>-<version>/`
pub(crate) async fn run(
    gem_name: &str,
    version: Option<&str>,
    target_dir: Option<&str>,
) -> Result<()> {
    // Load configuration
    let config = Config::load().context("Failed to load configuration")?;
    let cache_dir = config::cache_dir(Some(&config))?;

    // Determine version to unpack
    let gem_version = if let Some(v) = version {
        v.to_string()
    } else {
        // Try to get version from lockfile
        if Path::new("Gemfile.lock").exists() {
            let lockfile_content =
                fs::read_to_string("Gemfile.lock").context("Failed to read Gemfile.lock")?;
            let lockfile =
                Lockfile::parse(&lockfile_content).context("Failed to parse Gemfile.lock")?;

            // Find gem in lockfile
            if let Some(gem) = lockfile.gems.iter().find(|g| g.name == gem_name) {
                gem.version.clone()
            } else {
                anyhow::bail!(
                    "Gem '{gem_name}' not found in lockfile. Specify --version explicitly."
                );
            }
        } else {
            anyhow::bail!("No Gemfile.lock found. Specify --version explicitly.");
        }
    };

    println!("Unpacking {gem_name} {gem_version}...");

    // Create gem spec for download
    let gem_spec = GemSpec::new(
        gem_name.to_string(),
        gem_version.clone(),
        None,
        Vec::new(),
        Vec::new(),
    );

    // Download gem (or use cached) - supports both Gemfile and gems.rb
    let gemfile_path = lode::paths::find_gemfile();
    let sources = if gemfile_path.exists() {
        if let Ok(gemfile) = lode::Gemfile::parse_file(&gemfile_path) {
            let mut all_sources = vec![gemfile.source];
            all_sources.extend(gemfile.sources);
            all_sources
        } else {
            vec![lode::DEFAULT_GEM_SOURCE.to_string()]
        }
    } else {
        vec![lode::DEFAULT_GEM_SOURCE.to_string()]
    };

    let dm = DownloadManager::with_sources(cache_dir, sources)
        .context("Failed to create download manager")?;

    let gem_path = dm
        .download_gem(&gem_spec)
        .await
        .context("Failed to download gem")?;

    println!("Fetched gem to {}", gem_path.display());

    // Determine target directory
    let target = target_dir.map_or_else(|| PathBuf::from("."), PathBuf::from);

    // Extract gem
    extract_gem(&gem_path, &target, gem_name, &gem_version)?;

    let output_dir = target.join(format!("{gem_name}-{gem_version}"));
    println!("Unpacked gem to {}", output_dir.display());

    Ok(())
}

/// Extract a .gem file to a directory
///
/// A .gem file is a tar.gz archive containing:
/// - metadata.gz
/// - data.tar.gz (the actual gem contents)
/// - checksums.yaml.gz
///
/// We need to extract data.tar.gz and then extract its contents.
fn extract_gem(
    gem_path: &Path,
    target_dir: &Path,
    gem_name: &str,
    gem_version: &str,
) -> Result<()> {
    // Read the .gem file (it's a tar archive)
    let gem_file = fs::File::open(gem_path)
        .with_context(|| format!("Failed to open gem file: {}", gem_path.display()))?;

    let mut gem_archive = Archive::new(gem_file);

    // Extract data.tar.gz from the gem
    let mut data_tar_gz = None;

    for entry in gem_archive.entries()? {
        let mut entry = entry?;
        let path = entry.path()?;

        if path.to_str() == Some("data.tar.gz") {
            // Read data.tar.gz into memory
            let mut buffer = Vec::new();
            std::io::copy(&mut entry, &mut buffer)?;
            data_tar_gz = Some(buffer);
            break;
        }
    }

    let data_tar_gz = data_tar_gz.context("data.tar.gz not found in gem file")?;

    // Decompress and extract data.tar.gz
    let decoder = GzDecoder::new(&data_tar_gz[..]);
    let mut data_archive = Archive::new(decoder);

    // Create output directory
    let output_dir = target_dir.join(format!("{gem_name}-{gem_version}"));
    fs::create_dir_all(&output_dir)
        .with_context(|| format!("Failed to create directory: {}", output_dir.display()))?;

    // Extract all files from data.tar.gz
    data_archive
        .unpack(&output_dir)
        .with_context(|| format!("Failed to unpack gem to: {}", output_dir.display()))?;

    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "Tests can panic")]
mod tests {
    use super::*;

    #[test]
    fn version_from_lockfile_parsing() {
        // Test that we can parse version from lockfile format
        let lockfile_content = r"GEM
  remote: https://rubygems.org/
  specs:
    rack (3.0.8)
    rails (7.0.8)

PLATFORMS
  ruby

DEPENDENCIES
  rack
  rails
";

        let lockfile = Lockfile::parse(lockfile_content).unwrap();
        let rack = lockfile.gems.iter().find(|g| g.name == "rack").unwrap();
        assert_eq!(rack.version, "3.0.8");
    }

    #[test]
    fn gem_spec_creation() {
        let spec = GemSpec::new(
            "rack".to_string(),
            "3.0.8".to_string(),
            None,
            Vec::new(),
            Vec::new(),
        );

        assert_eq!(spec.name, "rack");
        assert_eq!(spec.version, "3.0.8");
        assert_eq!(spec.full_name(), "rack-3.0.8");
    }
}
