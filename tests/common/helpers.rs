//! Shared test helpers and utilities

use std::fmt::Write;
use std::fs;
use tempfile::TempDir;

/// Get the path to the lode binary (target/debug/lode)
///
/// This is shared across all integration tests to avoid duplication.
pub(crate) fn get_lode_binary() -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    std::path::Path::new(manifest_dir)
        .join("target/debug/lode")
        .to_string_lossy()
        .to_string()
}

/// Create a temporary Gemfile with the given gems
///
/// # Arguments
/// * `temp_dir` - The temporary directory to create the Gemfile in
/// * `gems` - Slice of (name, version) tuples
///
/// # Returns
/// The path to the created Gemfile
#[allow(dead_code)]
pub(crate) fn create_test_gemfile(temp_dir: &TempDir, gems: &[(&str, &str)]) -> String {
    let gemfile_path = temp_dir.path().join("Gemfile");

    let mut content = String::from("source 'https://rubygems.org'\n\n");
    for (name, version) in gems {
        writeln!(&mut content, "gem '{name}', '{version}'").unwrap();
    }

    fs::write(&gemfile_path, content).expect("Failed to write Gemfile");
    gemfile_path.to_string_lossy().to_string()
}

/// Create a test Gemfile.lock with the given gems
///
/// # Arguments
/// * `temp_dir` - The temporary directory to create the lockfile in
/// * `gems` - Slice of (name, version) tuples
///
/// # Returns
/// The path to the created Gemfile.lock
#[allow(dead_code)]
pub(crate) fn create_test_lockfile(temp_dir: &TempDir, gems: &[(&str, &str)]) -> String {
    let lockfile_path = temp_dir.path().join("Gemfile.lock");
    let mut content = String::from("GEM\n  remote: https://rubygems.org/\n  specs:\n");
    for (name, version) in gems {
        writeln!(&mut content, "    {name} ({version})").unwrap();
    }
    content.push_str("\nPLATFORMS\n  ruby\n\nDEPENDENCIES\n");
    for (name, _version) in gems {
        writeln!(&mut content, "  {name}").unwrap();
    }
    content.push_str("\nRUBY VERSION\n   ruby 3.2.0\n\nBUNDLED WITH\n   2.4.6\n");
    fs::write(&lockfile_path, content).expect("Failed to write lockfile");
    lockfile_path.to_string_lossy().to_string()
}
