//! Path utilities for Gemfile and lockfile detection.
//! Supports both traditional (Gemfile/Gemfile.lock) and modern (gems.rb/gems.locked) naming.

use crate::env_vars;
use std::path::{Path, PathBuf};

/// Find the Gemfile in the current directory.
/// Priority: `BUNDLE_GEMFILE` env var -> gems.rb -> Gemfile (defaults to Gemfile if neither exists).
#[must_use]
pub fn find_gemfile() -> PathBuf {
    // Check BUNDLE_GEMFILE environment variable first
    if let Some(gemfile) = env_vars::bundle_gemfile() {
        return PathBuf::from(gemfile);
    }

    find_gemfile_in(".")
}

/// Find the Gemfile in `dir`, checking gems.rb first (modern) then Gemfile (traditional).
/// Defaults to "Gemfile" if neither exists.
#[must_use]
pub fn find_gemfile_in(dir: impl AsRef<Path>) -> PathBuf {
    let dir = dir.as_ref();

    // Check for gems.rb (modern convention)
    let gems_rb = dir.join("gems.rb");
    if gems_rb.exists() {
        return gems_rb;
    }

    // Check for Gemfile (traditional)
    let gemfile = dir.join("Gemfile");
    if gemfile.exists() {
        return gemfile;
    }

    // Default to Gemfile if neither exists
    gemfile
}

/// Find the lockfile in the current directory.
/// Priority: gems.locked -> Gemfile.lock (defaults to Gemfile.lock if neither exists).
#[must_use]
pub fn find_lockfile() -> PathBuf {
    find_lockfile_in(".")
}

/// Find the lockfile in `dir`, checking gems.locked first (modern) then Gemfile.lock (traditional).
/// Defaults to "Gemfile.lock" if neither exists.
#[must_use]
pub fn find_lockfile_in(dir: impl AsRef<Path>) -> PathBuf {
    let dir = dir.as_ref();

    // Check for gems.locked (modern convention)
    let gems_locked = dir.join("gems.locked");
    if gems_locked.exists() {
        return gems_locked;
    }

    // Check for Gemfile.lock (traditional)
    let gemfile_lock = dir.join("Gemfile.lock");
    if gemfile_lock.exists() {
        return gemfile_lock;
    }

    // Default to Gemfile.lock if neither exists
    gemfile_lock
}

/// Get the lockfile path for a given Gemfile.
/// Maps gems.rb -> gems.locked, otherwise appends ".lock".
#[must_use]
pub fn lockfile_for_gemfile(gemfile: &Path) -> PathBuf {
    if let Some(file_name) = gemfile.file_name()
        && file_name == "gems.rb"
    {
        return gemfile.with_file_name("gems.locked");
    }

    // Default: append .lock to the gemfile path
    let mut lockfile = gemfile.as_os_str().to_owned();
    lockfile.push(".lock");
    PathBuf::from(lockfile)
}

/// Get the Gemfile path for a given lockfile.
/// Maps gems.locked -> gems.rb, otherwise removes ".lock".
#[must_use]
pub fn gemfile_for_lockfile(lockfile: &Path) -> PathBuf {
    if let Some(file_name) = lockfile.file_name()
        && file_name == "gems.locked"
    {
        return lockfile.with_file_name("gems.rb");
    }

    // Default: remove .lock extension
    let lockfile_str = lockfile.to_string_lossy();
    if let Some(gemfile_str) = lockfile_str.strip_suffix(".lock") {
        return PathBuf::from(gemfile_str);
    }

    // Fallback: just return the lockfile as-is
    lockfile.to_path_buf()
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "Tests can panic")]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn find_gemfile_prefers_gems_rb() {
        let temp = TempDir::new().unwrap();
        fs::write(
            temp.path().join("gems.rb"),
            "source 'https://rubygems.org'\n",
        )
        .unwrap();
        fs::write(
            temp.path().join("Gemfile"),
            "source 'https://rubygems.org'\n",
        )
        .unwrap();

        let found = find_gemfile_in(temp.path());
        assert_eq!(found.file_name().unwrap(), "gems.rb");
    }

    #[test]
    fn find_gemfile_falls_back_to_gemfile() {
        let temp = TempDir::new().unwrap();
        fs::write(
            temp.path().join("Gemfile"),
            "source 'https://rubygems.org'\n",
        )
        .unwrap();

        let found = find_gemfile_in(temp.path());
        assert_eq!(found.file_name().unwrap(), "Gemfile");
    }

    #[test]
    fn find_gemfile_defaults_to_gemfile() {
        let temp = TempDir::new().unwrap();

        let found = find_gemfile_in(temp.path());
        assert_eq!(found.file_name().unwrap(), "Gemfile");
    }

    #[test]
    fn find_lockfile_prefers_gems_locked() {
        let temp = TempDir::new().unwrap();
        fs::write(temp.path().join("gems.locked"), "GEM\n").unwrap();
        fs::write(temp.path().join("Gemfile.lock"), "GEM\n").unwrap();

        let found = find_lockfile_in(temp.path());
        assert_eq!(found.file_name().unwrap(), "gems.locked");
    }

    #[test]
    fn find_lockfile_falls_back_to_gemfile_lock() {
        let temp = TempDir::new().unwrap();
        fs::write(temp.path().join("Gemfile.lock"), "GEM\n").unwrap();

        let found = find_lockfile_in(temp.path());
        assert_eq!(found.file_name().unwrap(), "Gemfile.lock");
    }

    #[test]
    fn find_lockfile_defaults_to_gemfile_lock() {
        let temp = TempDir::new().unwrap();

        let found = find_lockfile_in(temp.path());
        assert_eq!(found.file_name().unwrap(), "Gemfile.lock");
    }

    #[test]
    fn lockfile_for_gemfile_gems_rb() {
        assert_eq!(
            lockfile_for_gemfile(Path::new("gems.rb")),
            Path::new("gems.locked")
        );
    }

    #[test]
    fn lockfile_for_gemfile_traditional() {
        assert_eq!(
            lockfile_for_gemfile(Path::new("Gemfile")),
            Path::new("Gemfile.lock")
        );
    }

    #[test]
    fn lockfile_for_gemfile_with_path() {
        assert_eq!(
            lockfile_for_gemfile(Path::new("custom/gems.rb")),
            Path::new("custom/gems.locked")
        );
        assert_eq!(
            lockfile_for_gemfile(Path::new("custom/Gemfile")),
            Path::new("custom/Gemfile.lock")
        );
    }

    #[test]
    fn gemfile_for_lockfile_gems_locked() {
        assert_eq!(
            gemfile_for_lockfile(Path::new("gems.locked")),
            Path::new("gems.rb")
        );
    }

    #[test]
    fn gemfile_for_lockfile_traditional() {
        assert_eq!(
            gemfile_for_lockfile(Path::new("Gemfile.lock")),
            Path::new("Gemfile")
        );
    }

    #[test]
    fn gemfile_for_lockfile_with_path() {
        assert_eq!(
            gemfile_for_lockfile(Path::new("custom/gems.locked")),
            Path::new("custom/gems.rb")
        );
        assert_eq!(
            gemfile_for_lockfile(Path::new("custom/Gemfile.lock")),
            Path::new("custom/Gemfile")
        );
    }
}
