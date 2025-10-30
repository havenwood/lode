//! Extension type definitions
//!
//! Ruby gems can include native extensions written in C, Rust, or other languages.
//! These must be compiled during installation. This module defines the types of
//! extensions we support.

use std::path::PathBuf;

/// Types of native extensions a gem can have
///
/// Determined by checking the gem's `ext/` directory and build files:
/// - C extensions use extconf.rb (most common: nokogiri, pg, mysql2)
/// - Rust extensions use Cargo.toml (newer: magnus-based gems)
/// - Precompiled means the gem includes platform-specific binaries
/// - None means pure Ruby (no compilation needed)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExtensionType {
    /// C extension using extconf.rb and mkmf
    CExtension {
        /// Path to the ext/ directory containing extconf.rb
        ext_dir: PathBuf,
        /// Path to the extconf.rb file
        extconf_path: PathBuf,
    },

    /// Rust extension using Cargo
    RustExtension {
        /// Path to Cargo.toml
        cargo_toml: PathBuf,
    },

    /// CMake-based extension
    CMakeExtension {
        /// Path to CMakeLists.txt
        cmake_lists: PathBuf,
    },

    /// Precompiled extension (platform-specific gem)
    Precompiled,

    /// No extension (pure Ruby gem)
    None,
}

impl ExtensionType {
    /// Check if this gem requires building
    #[must_use]
    #[inline]
    pub const fn needs_building(&self) -> bool {
        !matches!(self, Self::Precompiled | Self::None)
    }

    /// Get a human-readable description
    #[must_use]
    #[inline]
    pub const fn description(&self) -> &str {
        match self {
            Self::CExtension { .. } => "C extension",
            Self::RustExtension { .. } => "Rust extension",
            Self::CMakeExtension { .. } => "CMake extension",
            Self::Precompiled => "precompiled",
            Self::None => "pure Ruby",
        }
    }
}

/// Result of building an extension
#[derive(Debug)]
pub struct BuildResult {
    /// Gem name
    pub gem_name: String,

    /// Whether the build succeeded
    pub success: bool,

    /// Build duration
    pub duration: std::time::Duration,

    /// Error message if failed
    pub error: Option<String>,

    /// Build output (stdout + stderr)
    pub output: String,
}

impl BuildResult {
    /// Create a successful build result
    #[must_use]
    pub const fn success(gem_name: String, duration: std::time::Duration, output: String) -> Self {
        Self {
            gem_name,
            success: true,
            duration,
            error: None,
            output,
        }
    }

    /// Create a failed build result
    #[must_use]
    pub const fn failure(
        gem_name: String,
        duration: std::time::Duration,
        error: String,
        output: String,
    ) -> Self {
        Self {
            gem_name,
            success: false,
            duration,
            error: Some(error),
            output,
        }
    }
}
