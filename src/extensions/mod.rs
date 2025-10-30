//! Native extension building
//!
//! Handles compilation of native extensions. Extensions are compiled code
//! (C, Rust, etc.) that must be built during gem installation (similar to
//! what `bundle install` does).
//!
//! Supported extension types:
//! - C extensions (`extconf.rb` + `make`)
//! - Rust extensions (`Cargo.toml`)
//! - `CMake` extensions (`CMakeLists.txt`)
//! - Precompiled (no build needed)

pub mod binstubs;
pub mod builder;
pub mod c_extension;
pub mod cmake_extension;
pub mod detector;
pub mod rust_extension;
pub mod types;

pub use binstubs::{BinstubGenerator, generate_binstubs};
pub use builder::{ExtensionBuilder, build_extensions};
pub use c_extension::CExtensionBuilder;
pub use cmake_extension::CMakeExtensionBuilder;
pub use detector::{detect_extension, has_platform_suffix};
pub use rust_extension::RustExtensionBuilder;
pub use types::{BuildResult, ExtensionType};
