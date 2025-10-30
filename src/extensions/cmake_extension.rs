//! `CMake` extension building
//!
//! Builds `CMake`-based extensions. Some gems use `CMake` instead of
//! `extconf.rb` for more complex build setups.
//! Examples: Some versions of `nokogiri`, `libxml-ruby`
//!
//! Build process:
//! ```bash
//! mkdir -p build
//! cd build
//! cmake ..
//! cmake --build .
//! cmake --install .
//! ```

use super::types::BuildResult;
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

/// `CMake` extension builder
///
/// Handles `CMake`-based extensions. `CMake` is used for more complex build
/// configurations than `extconf.rb`.
#[derive(Debug)]
pub struct CMakeExtensionBuilder {
    /// Path to `CMake` executable
    cmake_path: PathBuf,
    /// Enable verbose output
    verbose: bool,
}

impl CMakeExtensionBuilder {
    /// Create a new `CMake` extension builder
    ///
    /// Finds the `CMake` executable automatically.
    /// Priority order:
    /// 1. `CMAKE` environment variable
    /// 2. `cmake` in `PATH`
    /// 3. Error if not found
    pub fn new(verbose: bool) -> Result<Self> {
        let cmake_path = Self::find_cmake_executable().context(
            "CMake executable not found. CMake extensions require CMake to be installed.",
        )?;

        Ok(Self {
            cmake_path,
            verbose,
        })
    }

    /// Find `CMake` executable on the system
    fn find_cmake_executable() -> Result<PathBuf> {
        // Check CMAKE environment variable
        if let Ok(cmake_env) = std::env::var("CMAKE") {
            let path = PathBuf::from(cmake_env);
            if path.exists() {
                return Ok(path);
            }
        }

        // Check for `cmake` in PATH
        if let Ok(output) = Command::new("which").arg("cmake").output()
            && output.status.success()
        {
            let path_str = String::from_utf8_lossy(&output.stdout);
            let path = PathBuf::from(path_str.trim());
            if path.exists() {
                return Ok(path);
            }
        }

        anyhow::bail!("CMake executable not found. Install CMake from https://cmake.org")
    }

    /// Build a `CMake` extension.
    ///
    /// # Returns
    /// Result with build status and output
    pub fn build(&self, gem_name: &str, ext_dir: &Path, gem_dir: &Path) -> Result<BuildResult> {
        let start_time = Instant::now();
        let mut output_buffer = Vec::new();

        if self.verbose {
            output_buffer.extend_from_slice(
                format!("Building CMake extension for {gem_name}...\n").as_bytes(),
            );
        }

        // Create build directory
        let build_dir = ext_dir.join("build");
        std::fs::create_dir_all(&build_dir).context("Failed to create build directory")?;

        // Step 1: Run cmake to configure
        let mut cmd = Command::new(&self.cmake_path);
        cmd.arg("..")
            .arg(format!("-DCMAKE_INSTALL_PREFIX={}", gem_dir.display()))
            .current_dir(&build_dir);

        // Pass build tool environment variables to CMake
        // CMake respects both CMAKE_* and standard compiler variables
        if let Some(cc) = crate::env_vars::cc() {
            cmd.env("CC", &cc);
            cmd.arg(format!("-DCMAKE_C_COMPILER={cc}"));
        }
        if let Some(cxx) = crate::env_vars::cxx() {
            cmd.env("CXX", &cxx);
            cmd.arg(format!("-DCMAKE_CXX_COMPILER={cxx}"));
        }
        if let Some(cflags) = crate::env_vars::cflags() {
            cmd.env("CFLAGS", &cflags);
            cmd.arg(format!("-DCMAKE_C_FLAGS={cflags}"));
        }
        if let Some(cxxflags) = crate::env_vars::cxxflags() {
            cmd.env("CXXFLAGS", &cxxflags);
            cmd.arg(format!("-DCMAKE_CXX_FLAGS={cxxflags}"));
        }
        if let Some(ldflags) = crate::env_vars::ldflags() {
            cmd.env("LDFLAGS", &ldflags);
            cmd.arg(format!("-DCMAKE_EXE_LINKER_FLAGS={ldflags}"));
        }

        let configure_output = cmd.output().context("Failed to execute cmake configure")?;

        output_buffer.extend_from_slice(&configure_output.stdout);
        output_buffer.extend_from_slice(&configure_output.stderr);

        if !configure_output.status.success() {
            return Ok(BuildResult::failure(
                gem_name.to_string(),
                start_time.elapsed(),
                "CMake configuration failed".to_string(),
                String::from_utf8_lossy(&output_buffer).to_string(),
            ));
        }

        // Step 2: Run cmake --build to compile
        let build_output = Command::new(&self.cmake_path)
            .arg("--build")
            .arg(".")
            .current_dir(&build_dir)
            .output()
            .context("Failed to execute cmake build")?;

        output_buffer.extend_from_slice(&build_output.stdout);
        output_buffer.extend_from_slice(&build_output.stderr);

        if !build_output.status.success() {
            return Ok(BuildResult::failure(
                gem_name.to_string(),
                start_time.elapsed(),
                "CMake build failed".to_string(),
                String::from_utf8_lossy(&output_buffer).to_string(),
            ));
        }

        // Step 3: Run cmake --install to install
        let install_output = Command::new(&self.cmake_path)
            .arg("--install")
            .arg(".")
            .current_dir(&build_dir)
            .output()
            .context("Failed to execute cmake install")?;

        output_buffer.extend_from_slice(&install_output.stdout);
        output_buffer.extend_from_slice(&install_output.stderr);

        if !install_output.status.success() {
            return Ok(BuildResult::failure(
                gem_name.to_string(),
                start_time.elapsed(),
                "CMake install failed".to_string(),
                String::from_utf8_lossy(&output_buffer).to_string(),
            ));
        }

        Ok(BuildResult::success(
            gem_name.to_string(),
            start_time.elapsed(),
            String::from_utf8_lossy(&output_buffer).to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_cmake() {
        // This test will pass if CMake is installed, fail otherwise
        let result = CMakeExtensionBuilder::find_cmake_executable();

        // Just check that it either finds cmake or errors appropriately
        match result {
            Ok(path) => {
                assert!(path.exists(), "CMake path exists");
            }
            Err(e) => {
                assert!(e.to_string().contains("CMake executable not found"));
            }
        }
    }

    #[test]
    fn cmake_builder_creation() {
        // Test that we can create a builder (or get appropriate error)
        let result = CMakeExtensionBuilder::new(false);

        match result {
            Ok(_builder) => {
                // Builder created successfully
            }
            Err(e) => {
                // Expected error if CMake not installed
                assert!(e.to_string().contains("CMake"));
            }
        }
    }
}
