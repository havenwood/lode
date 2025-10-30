//! Gem installation and extraction
//!
//! Handles extracting .gem files, copying path gems, building git gems,
//! and installing gems to vendor directories.

use crate::lockfile::{GemSpec, GitGemSpec, PathGemSpec};
use anyhow::Result;
use flate2::read::GzDecoder;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tar::Archive;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum InstallError {
    #[error("Failed to extract {gem}: {source}")]
    ExtractionError {
        gem: String,
        #[source]
        source: std::io::Error,
    },

    #[error("Invalid gem archive for {gem}: {reason}")]
    InvalidArchive { gem: String, reason: String },

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

/// Extract a .gem file to a destination directory
///
/// Extracts gem contents and metadata to appropriate directories.
///
/// # Errors
///
/// Returns an error if the gem file cannot be read, is corrupted, or extraction fails.
pub fn extract_gem(
    gem_path: &Path,
    dest_dir: &Path,
    gem_name: &str,
    spec_path: &Path,
) -> Result<(), InstallError> {
    let file = fs::File::open(gem_path).map_err(|e| InstallError::ExtractionError {
        gem: gem_name.to_string(),
        source: e,
    })?;

    let mut archive = Archive::new(file);
    let mut found_data = false;
    let mut found_metadata = false;

    for entry_result in archive
        .entries()
        .map_err(|e| InstallError::ExtractionError {
            gem: gem_name.to_string(),
            source: e,
        })?
    {
        let entry = entry_result.map_err(|e| InstallError::ExtractionError {
            gem: gem_name.to_string(),
            source: e,
        })?;

        let path = entry.path().map_err(|e| InstallError::ExtractionError {
            gem: gem_name.to_string(),
            source: e,
        })?;

        match path.to_str() {
            Some("data.tar.gz") => {
                found_data = true;

                // Decompress and extract gem files
                let gz = GzDecoder::new(entry);
                let mut data_archive = Archive::new(gz);

                data_archive
                    .unpack(dest_dir)
                    .map_err(|e| InstallError::ExtractionError {
                        gem: gem_name.to_string(),
                        source: e,
                    })?;
            }
            Some("metadata.gz") => {
                found_metadata = true;

                // Extract gemspec for Bundler compatibility
                let mut gz = GzDecoder::new(entry);
                let mut metadata = Vec::new();
                std::io::Read::read_to_end(&mut gz, &mut metadata).map_err(|e| {
                    InstallError::ExtractionError {
                        gem: gem_name.to_string(),
                        source: e,
                    }
                })?;

                // Ensure specifications directory exists
                if let Some(parent) = spec_path.parent() {
                    fs::create_dir_all(parent)?;
                }

                // Write gemspec file
                fs::write(spec_path, metadata)?;
            }
            _ => {}
        }

        // Exit early if we've found both
        if found_data && found_metadata {
            break;
        }
    }

    if !found_data {
        return Err(InstallError::InvalidArchive {
            gem: gem_name.to_string(),
            reason: "data.tar.gz not found in gem archive".to_string(),
        });
    }

    Ok(())
}

/// Install a gem from cache to vendor directory
///
/// Creates standard `RubyGems` directory structure.
///
/// # Errors
///
/// Returns an error if gem extraction or installation fails.
pub fn install_gem(
    gem_spec: &GemSpec,
    cache_path: &Path,
    vendor_dir: &Path,
    ruby_version: &str,
) -> Result<(), InstallError> {
    // Build installation paths
    let ruby_dir = vendor_dir.join("ruby").join(ruby_version);
    let gem_install_dir = ruby_dir.join("gems").join(gem_spec.full_name());
    let spec_path = ruby_dir
        .join("specifications")
        .join(format!("{}.gemspec", gem_spec.full_name()));

    // Skip if already installed
    if gem_install_dir.exists() {
        return Ok(());
    }

    // Create parent directories
    if let Some(parent) = gem_install_dir.parent() {
        fs::create_dir_all(parent)?;
    }

    // Create gem directory
    fs::create_dir_all(&gem_install_dir)?;

    // Extract gem files and gemspec
    extract_gem(cache_path, &gem_install_dir, &gem_spec.name, &spec_path)?;

    Ok(())
}

/// Install a gem from a local path to vendor directory
///
/// Copies the gem directory directly without archive extraction.
///
/// # Errors
///
/// Returns an error if the path doesn't exist or copying fails.
pub fn install_path_gem(
    path_spec: &PathGemSpec,
    vendor_dir: &Path,
    ruby_version: &str,
) -> Result<(), InstallError> {
    // Build installation paths
    let ruby_dir = vendor_dir.join("ruby").join(ruby_version);
    let gem_full_name = format!("{}-{}", path_spec.name, path_spec.version);
    let gem_install_dir = ruby_dir.join("gems").join(&gem_full_name);

    // Skip if already installed
    if gem_install_dir.exists() {
        return Ok(());
    }

    // Resolve path (relative to current directory)
    let source_path = PathBuf::from(&path_spec.path);
    let source_path = if source_path.is_absolute() {
        source_path
    } else {
        std::env::current_dir()?.join(&source_path)
    };

    // Verify source path exists
    if !source_path.exists() {
        return Err(InstallError::InvalidArchive {
            gem: path_spec.name.clone(),
            reason: format!("Path gem source not found: {}", source_path.display()),
        });
    }

    // Create parent directories
    if let Some(parent) = gem_install_dir.parent() {
        fs::create_dir_all(parent)?;
    }

    // Copy gem directory
    copy_dir_recursive(&source_path, &gem_install_dir)?;

    // Create gemspec stub if needed (for Bundler compatibility)
    let spec_path = ruby_dir
        .join("specifications")
        .join(format!("{gem_full_name}.gemspec"));

    if !spec_path.exists() {
        if let Some(parent) = spec_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Look for .gemspec file in source directory
        if let Some(gemspec_file) = find_gemspec(&source_path) {
            fs::copy(gemspec_file, &spec_path)?;
        } else {
            // Create minimal gemspec stub
            let stub_content = format!(
                "# -*- encoding: utf-8 -*-\n\
                Gem::Specification.new do |s|\n\
                  s.name = \"{}\"\n\
                  s.version = \"{}\"\n\
                end\n",
                path_spec.name, path_spec.version
            );
            fs::write(&spec_path, stub_content)?;
        }
    }

    Ok(())
}

/// Recursively copy directory contents
fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), InstallError> {
    fs::create_dir_all(dst)?;

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            // Skip .git, .bundle, vendor directories
            let dir_name = entry.file_name();
            if dir_name == ".git" || dir_name == ".bundle" || dir_name == "vendor" {
                continue;
            }
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }

    Ok(())
}

/// Find .gemspec file in a directory
fn find_gemspec(dir: &Path) -> Option<PathBuf> {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("gemspec") {
                return Some(path);
            }
        }
    }
    None
}

/// Build a gem from source (run `gem build <gemspec>`)
///
/// Returns the path to the built .gem file.
///
/// # Errors
///
/// Returns an error if no gemspec is found, gem build fails, or Ruby is not available.
pub fn build_gem_from_source(
    git_spec: &GitGemSpec,
    source_dir: &Path,
    build_dir: &Path,
) -> Result<PathBuf, InstallError> {
    // Find gemspec file
    let gemspec_path = find_gemspec(source_dir).ok_or_else(|| InstallError::InvalidArchive {
        gem: git_spec.name.clone(),
        reason: format!("No .gemspec file found in {}", source_dir.display()),
    })?;

    // Create build directory
    fs::create_dir_all(build_dir)?;

    // Run gem build <gemspec>
    let output = Command::new("gem")
        .arg("build")
        .arg(&gemspec_path)
        .arg("--output")
        .arg(build_dir.join(format!("{}-{}.gem", git_spec.name, git_spec.version)))
        .current_dir(source_dir)
        .output()
        .map_err(|e| InstallError::InvalidArchive {
            gem: git_spec.name.clone(),
            reason: format!("Failed to run gem build: {e}"),
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(InstallError::InvalidArchive {
            gem: git_spec.name.clone(),
            reason: format!("gem build failed: {stderr}"),
        });
    }

    // Return path to built gem
    let gem_path = build_dir.join(format!("{}-{}.gem", git_spec.name, git_spec.version));

    if !gem_path.exists() {
        return Err(InstallError::InvalidArchive {
            gem: git_spec.name.clone(),
            reason: format!(
                "gem build succeeded but .gem file not found at {}",
                gem_path.display()
            ),
        });
    }

    Ok(gem_path)
}

/// Install a gem from a git source
///
/// Builds the gem from source and then installs it.
///
/// # Errors
///
/// Returns an error if the build or installation fails.
pub fn install_git_gem(
    git_spec: &GitGemSpec,
    source_dir: &Path,
    vendor_dir: &Path,
    ruby_version: &str,
) -> Result<(), InstallError> {
    // Build gem from source
    let build_dir = source_dir.join("pkg");
    let gem_path = build_gem_from_source(git_spec, source_dir, &build_dir)?;

    // Create a GemSpec for installation
    let gem_spec = GemSpec::new(
        git_spec.name.clone(),
        git_spec.version.clone(),
        None,
        vec![],
        vec![],
    );

    // Install the built gem
    install_gem(&gem_spec, &gem_path, vendor_dir, ruby_version)?;

    Ok(())
}

/// Install report statistics
#[derive(Debug, Default, Copy, Clone)]
pub struct InstallReport {
    pub installed: usize,
    pub skipped: usize,
    pub failed: usize,
}

impl InstallReport {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub const fn record_installed(&mut self) {
        self.installed += 1;
    }

    pub const fn record_skipped(&mut self) {
        self.skipped += 1;
    }

    pub const fn record_failed(&mut self) {
        self.failed += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn install_report() {
        let mut report = InstallReport::new();
        assert_eq!(report.installed, 0);

        report.record_installed();
        assert_eq!(report.installed, 1);

        report.record_skipped();
        assert_eq!(report.skipped, 1);
    }
}
