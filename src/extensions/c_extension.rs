//! C Extension Building
//!
//! Builds C extensions using the extconf.rb + make workflow.
//! It's the equivalent of what happens when you run:
//! ```bash
//! cd ext/gem_name
//! ruby extconf.rb   # Generate Makefile
//! make              # Compile C code
//! make install      # Copy .so/.bundle to lib/
//! ```

use super::types::BuildResult;
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

/// C extension builder
///
/// Handles the standard C extension build process:
/// 1. Find Ruby executable (from PATH or environment)
/// 2. Run `ruby extconf.rb` to generate Makefile
/// 3. Run `make` to compile
/// 4. Copy compiled extension (.so/.bundle) to lib/
#[derive(Debug)]
pub struct CExtensionBuilder {
    /// Path to Ruby executable
    ruby_path: PathBuf,
    /// Enable verbose output
    verbose: bool,
}

impl CExtensionBuilder {
    /// Create a new C extension builder
    ///
    /// Finds the Ruby executable automatically.
    /// Priority order:
    /// 1. RUBY environment variable
    /// 2. `ruby` in PATH
    /// 3. Error if not found
    ///
    /// # Errors
    ///
    /// Returns an error if Ruby executable cannot be found.
    pub fn new(verbose: bool) -> Result<Self> {
        let ruby_path = Self::find_ruby_executable()
            .context("Ruby executable not found. C extensions require Ruby to be installed.")?;

        Ok(Self { ruby_path, verbose })
    }

    /// Find Ruby executable on the system
    ///
    /// Checks RUBY env var first, then PATH
    fn find_ruby_executable() -> Result<PathBuf> {
        // Check RUBY environment variable
        if let Ok(ruby_env) = std::env::var("RUBY") {
            let path = PathBuf::from(ruby_env);
            if path.exists() {
                return Ok(path);
            }
        }

        // Check for `ruby` in PATH
        if let Ok(output) = Command::new("which").arg("ruby").output()
            && output.status.success()
        {
            let path_str = String::from_utf8_lossy(&output.stdout);
            let path = PathBuf::from(path_str.trim());
            if path.exists() {
                return Ok(path);
            }
        }

        anyhow::bail!("Ruby executable not found in PATH or RUBY environment variable")
    }

    /// Build a C extension.
    ///
    /// Equivalent to what `bundle install` does when it encounters a gem with
    /// an extconf.rb file.
    ///
    /// # Returns
    /// `BuildResult` with build status, duration, and output
    #[must_use]
    #[allow(clippy::too_many_lines)]
    pub fn build(
        &self,
        gem_name: &str,
        ext_dir: &Path,
        extconf_path: &Path,
        gem_dir: &Path,
        rbconfig_path: Option<&str>,
    ) -> BuildResult {
        let start_time = Instant::now();
        let mut output = String::new();

        if self.verbose {
            println!("Building C extension for {gem_name}");
            println!("  ext_dir: {}", ext_dir.display());
            println!("  extconf: {}", extconf_path.display());
        }

        // Step 1: Run ruby extconf.rb
        let mut cmd = Command::new(&self.ruby_path);

        // Add --with-rbconfig if cross-compiling
        if let Some(rbconfig) = rbconfig_path {
            cmd.arg(format!("--with-rbconfig={rbconfig}"));
            if self.verbose {
                println!(
                    "  Running: {} --with-rbconfig={} extconf.rb",
                    self.ruby_path.display(),
                    rbconfig
                );
            }
        } else if self.verbose {
            println!("  Running: {} extconf.rb", self.ruby_path.display());
        }

        cmd.arg("extconf.rb");
        cmd.current_dir(ext_dir);

        // Pass build tool environment variables to extconf.rb
        // These affect how mkmf generates the Makefile
        if let Some(cc) = crate::env_vars::cc() {
            cmd.env("CC", cc);
        }
        if let Some(cxx) = crate::env_vars::cxx() {
            cmd.env("CXX", cxx);
        }
        if let Some(cflags) = crate::env_vars::cflags() {
            cmd.env("CFLAGS", cflags);
        }
        if let Some(cxxflags) = crate::env_vars::cxxflags() {
            cmd.env("CXXFLAGS", cxxflags);
        }
        if let Some(ldflags) = crate::env_vars::ldflags() {
            cmd.env("LDFLAGS", ldflags);
        }

        let extconf_result = cmd.output();

        let extconf_output = match extconf_result {
            Ok(out) => out,
            Err(e) => {
                return BuildResult::failure(
                    gem_name.to_string(),
                    start_time.elapsed(),
                    format!("Failed to run ruby extconf.rb: {e}"),
                    output,
                );
            }
        };

        output.push_str(&String::from_utf8_lossy(&extconf_output.stdout));
        output.push_str(&String::from_utf8_lossy(&extconf_output.stderr));

        if !extconf_output.status.success() {
            return BuildResult::failure(
                gem_name.to_string(),
                start_time.elapsed(),
                format!(
                    "extconf.rb failed with exit code: {}",
                    extconf_output
                        .status
                        .code()
                        .map_or_else(|| "unknown".to_string(), |c| c.to_string())
                ),
                output,
            );
        }

        // Step 2: Run make
        let make_cmd = crate::env_vars::make_command().unwrap_or_else(|| "make".to_string());

        if self.verbose {
            println!("  Running: {make_cmd}");
        }

        let mut cmd = Command::new(&make_cmd);
        cmd.current_dir(ext_dir);

        // Pass build tool environment variables to make
        // These override what's in the Makefile if needed
        if let Some(cc) = crate::env_vars::cc() {
            cmd.env("CC", cc);
        }
        if let Some(cxx) = crate::env_vars::cxx() {
            cmd.env("CXX", cxx);
        }
        if let Some(cflags) = crate::env_vars::cflags() {
            cmd.env("CFLAGS", cflags);
        }
        if let Some(cxxflags) = crate::env_vars::cxxflags() {
            cmd.env("CXXFLAGS", cxxflags);
        }
        if let Some(ldflags) = crate::env_vars::ldflags() {
            cmd.env("LDFLAGS", ldflags);
        }

        let make_result = cmd.output();

        let make_output = match make_result {
            Ok(out) => out,
            Err(e) => {
                return BuildResult::failure(
                    gem_name.to_string(),
                    start_time.elapsed(),
                    format!("Failed to run make: {e}"),
                    output,
                );
            }
        };

        output.push_str(&String::from_utf8_lossy(&make_output.stdout));
        output.push_str(&String::from_utf8_lossy(&make_output.stderr));

        if !make_output.status.success() {
            return BuildResult::failure(
                gem_name.to_string(),
                start_time.elapsed(),
                format!(
                    "make failed with exit code: {}",
                    make_output
                        .status
                        .code()
                        .map_or_else(|| "unknown".to_string(), |c| c.to_string())
                ),
                output,
            );
        }

        // Step 3: Find and copy compiled extension to lib/
        match self.copy_extension(gem_name, ext_dir, gem_dir) {
            Ok(copy_output) => {
                output.push_str(&copy_output);
                BuildResult::success(gem_name.to_string(), start_time.elapsed(), output)
            }
            Err(e) => BuildResult::failure(
                gem_name.to_string(),
                start_time.elapsed(),
                format!("Failed to copy extension: {e}"),
                output,
            ),
        }
    }

    /// Find compiled extension and copy to lib/
    ///
    /// Extensions are compiled as .so (Linux/BSD), .bundle (macOS), or .dll (Windows).
    /// They need to be copied to the lib/ directory so Ruby can require them.
    fn copy_extension(&self, _gem_name: &str, ext_dir: &Path, gem_dir: &Path) -> Result<String> {
        let mut output = String::new();

        // Find the compiled extension file
        // Common extensions: .so (Linux), .bundle (macOS), .dll (Windows)
        let extensions = ["so", "bundle", "dll"];

        let mut found_extension: Option<PathBuf> = None;

        for entry in std::fs::read_dir(ext_dir)
            .with_context(|| format!("Failed to read extension directory: {}", ext_dir.display()))?
        {
            let entry = entry?;
            let path = entry.path();

            if path.is_file()
                && let Some(ext) = path.extension()
                && extensions.contains(&ext.to_string_lossy().as_ref())
            {
                found_extension = Some(path);
                break;
            }
        }

        let extension_file = found_extension
            .ok_or_else(|| anyhow::anyhow!("No compiled extension found (.so/.bundle/.dll)"))?;

        // Determine target directory (lib/)
        let lib_dir = gem_dir.join("lib");
        std::fs::create_dir_all(&lib_dir)
            .with_context(|| format!("Failed to create lib directory: {}", lib_dir.display()))?;

        // Copy extension to lib/
        let target_path = lib_dir.join(
            extension_file
                .file_name()
                .ok_or_else(|| anyhow::anyhow!("Extension file has no name"))?,
        );

        std::fs::copy(&extension_file, &target_path).with_context(|| {
            format!(
                "Failed to copy {} to {}",
                extension_file.display(),
                target_path.display()
            )
        })?;

        if self.verbose {
            let msg = format!(
                "  Copied extension: {} -> {}\n",
                extension_file.display(),
                target_path.display()
            );
            output.push_str(&msg);
        }

        Ok(output)
    }

    /// Get the Ruby version being used
    ///
    /// Runs `ruby -v` to get version information. Useful for verifying Ruby
    /// compatibility.
    ///
    /// # Errors
    ///
    /// Returns an error if Ruby version command fails.
    pub fn ruby_version(&self) -> Result<String> {
        let output = Command::new(&self.ruby_path)
            .arg("-v")
            .output()
            .context("Failed to get Ruby version")?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            anyhow::bail!("Failed to get Ruby version")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_find_ruby_executable() {
        // Should find ruby in PATH (assuming it exists)
        // This test will pass if Ruby is installed
        let result = CExtensionBuilder::find_ruby_executable();

        // We can't guarantee Ruby is installed in CI, so just check the logic
        if let Ok(path) = result {
            assert!(path.to_string_lossy().contains("ruby"));
        } else {
            // Ruby not installed - expected in some environments
        }
    }

    #[test]
    fn builder_creation() {
        // Test builder creation (may fail if Ruby not installed)
        let result = CExtensionBuilder::new(false);

        if let Ok(builder) = result {
            // Verify ruby_path is set
            assert!(!builder.ruby_path.as_os_str().is_empty());
        } else {
            // Ruby not installed - expected in some environments
        }
    }

    #[test]
    fn copy_extension_missing_source() {
        // Create a fake extension directory with no compiled extensions
        let temp = TempDir::new().unwrap();
        let ext_dir = temp.path().join("ext");
        let gem_dir = temp.path();

        fs::create_dir_all(&ext_dir).unwrap();

        // Try to create a builder (may fail if Ruby not installed)
        if let Ok(builder) = CExtensionBuilder::new(false) {
            // Should fail because no .so/.bundle/.dll exists
            let result = builder.copy_extension("test_gem", &ext_dir, gem_dir);
            assert!(result.is_err());
            assert!(
                result
                    .unwrap_err()
                    .to_string()
                    .contains("No compiled extension found")
            );
        }
    }

    #[test]
    fn copy_extension_success() {
        // Create a fake extension directory with a .so file
        let temp = TempDir::new().unwrap();
        let ext_dir = temp.path().join("ext");
        let gem_dir = temp.path();

        fs::create_dir_all(&ext_dir).unwrap();

        // Create a fake compiled extension
        let fake_extension = ext_dir.join("test.so");
        fs::write(&fake_extension, b"fake compiled code").unwrap();

        // Try to create a builder (may fail if Ruby not installed)
        if let Ok(builder) = CExtensionBuilder::new(false) {
            // Should succeed in copying
            let result = builder.copy_extension("test_gem", &ext_dir, gem_dir);
            assert!(result.is_ok());

            // Verify file was copied to lib/
            let lib_dir = gem_dir.join("lib");
            let target = lib_dir.join("test.so");
            assert!(target.exists());
        }
    }

    #[test]
    fn test_ruby_version() {
        // Test getting Ruby version (may fail if Ruby not installed)
        if let Ok(builder) = CExtensionBuilder::new(false) {
            let result = builder.ruby_version();
            if let Ok(version) = result {
                assert!(version.contains("ruby"));
            } else {
                // May fail in some environments
            }
        }
    }
}
