//! Common test utilities and helpers
//!
//! This module provides shared functionality used across integration tests:
//! - Binary path resolution (via `get_lode_binary`)
//! - Test fixture utilities (via `helpers`)

pub(crate) mod helpers;

// Re-export get_lode_binary for convenient access
pub(crate) use helpers::get_lode_binary;
