//! Debug logging utilities
//!
//! Provides debug logging that respects the global --debug flag.
//! When debug mode is disabled, all debug logging has zero cost.

use std::sync::OnceLock;

static DEBUG_ENABLED: OnceLock<bool> = OnceLock::new();

/// Initialize debug mode from command-line flag
pub fn init_debug(enabled: bool) {
    let _ = DEBUG_ENABLED.set(enabled);
}

/// Check if debug mode is enabled
pub fn is_debug_enabled() -> bool {
    DEBUG_ENABLED.get().copied().unwrap_or(false)
}

/// Print a debug message if debug mode is enabled
pub fn debug_log(message: &str) {
    if is_debug_enabled() {
        eprintln!("[DEBUG] {message}");
    }
}

/// Print formatted debug message if debug mode is enabled
pub fn debug_logf(args: std::fmt::Arguments<'_>) {
    if is_debug_enabled() {
        eprintln!("[DEBUG] {args}");
    }
}

/// Macro for convenient debug logging
///
/// Usage: `debug!("message with {}", variable)`
#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => {
        if $crate::debug::is_debug_enabled() {
            eprintln!("[DEBUG] {}", format_args!($($arg)*));
        }
    };
}
