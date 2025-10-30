//! Rust Extension Building
//!
//! Builds Rust extensions using Cargo.
//! Many modern Ruby gems use Rust for performance-critical parts via:
//! - Magnus (most common)
//! - Rutie (older)
//! - rb-sys (newer)
//!
//! Build process:
//! ```bash
//! cargo build --release --target-dir target
//! # Compiled .so/.dylib is automatically placed in correct location
//! ```

use super::types::BuildResult;
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

/// Rust extension builder
///
/// Handles Rust-based Ruby extensions (e.g., helix gems, magnus-based gems).
#[derive(Debug)]
pub struct RustExtensionBuilder {
    /// Path to Cargo executable
    cargo_path: PathBuf,
    /// Enable verbose output
    verbose: bool,
}

impl RustExtensionBuilder {
    /// Create a new Rust extension builder
    ///
    /// Finds the Cargo executable automatically.
    /// Priority order:
    /// 1. CARGO environment variable
    /// 2. `cargo` in PATH
    /// 3. ~/.cargo/bin/cargo
    /// 4. Error if not found
    pub fn new(verbose: bool) -> Result<Self> {
        let cargo_path = Self::find_cargo_executable().context(
            "Cargo executable not found. Rust extensions require Cargo to be installed.",
        )?;

        Ok(Self {
            cargo_path,
            verbose,
        })
    }

    /// Find Cargo executable on the system
    fn find_cargo_executable() -> Result<PathBuf> {
        // Check CARGO environment variable
        if let Ok(cargo_env) = std::env::var("CARGO") {
            let path = PathBuf::from(cargo_env);
            if path.exists() {
                return Ok(path);
            }
        }

        // Check for `cargo` in PATH
        if let Ok(output) = Command::new("which").arg("cargo").output()
            && output.status.success()
        {
            let path_str = String::from_utf8_lossy(&output.stdout);
            let path = PathBuf::from(path_str.trim());
            if path.exists() {
                return Ok(path);
            }
        }

        // Check ~/.cargo/bin/cargo
        if let Some(home) = dirs::home_dir() {
            let cargo_path = home.join(".cargo").join("bin").join("cargo");
            if cargo_path.exists() {
                return Ok(cargo_path);
            }
        }

        anyhow::bail!("Cargo executable not found. Install Rust from https://rustup.rs")
    }

    /// Build a Rust extension.
    ///
    /// # Returns
    /// Result with build status and output
    pub fn build(&self, gem_name: &str, gem_dir: &Path, _cargo_toml: &Path) -> Result<BuildResult> {
        let start_time = Instant::now();
        let mut output_buffer = Vec::new();

        if self.verbose {
            output_buffer.extend_from_slice(
                format!("Building Rust extension for {gem_name}...\n").as_bytes(),
            );
        }

        // Step 1: Run cargo build --release
        let mut cmd = Command::new(&self.cargo_path);
        cmd.arg("build").arg("--release").current_dir(gem_dir);

        // Pass build tool environment variables to Cargo
        // Cargo uses these when compiling C/C++ dependencies
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

        let build_output = cmd.output().context("Failed to execute cargo build")?;

        output_buffer.extend_from_slice(&build_output.stdout);
        output_buffer.extend_from_slice(&build_output.stderr);

        if !build_output.status.success() {
            return Ok(BuildResult::failure(
                gem_name.to_string(),
                start_time.elapsed(),
                "cargo build failed".to_string(),
                String::from_utf8_lossy(&output_buffer).to_string(),
            ));
        }

        // Rust extensions typically set up their own lib/ paths via build scripts
        // No manual copying needed like with C extensions

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
    fn find_cargo() {
        // This test will pass if Cargo is installed, fail otherwise
        let result = RustExtensionBuilder::find_cargo_executable();

        // Just check that it either finds cargo or errors appropriately
        match result {
            Ok(path) => {
                assert!(path.exists(), "Cargo path exists");
            }
            Err(e) => {
                assert!(e.to_string().contains("Cargo executable not found"));
            }
        }
    }

    #[test]
    fn rust_builder_creation() {
        // Test that we can create a builder (or get appropriate error)
        let result = RustExtensionBuilder::new(false);

        match result {
            Ok(_builder) => {
                // Builder created successfully
            }
            Err(e) => {
                // Expected error if Cargo not installed
                assert!(e.to_string().contains("Cargo"));
            }
        }
    }
}
