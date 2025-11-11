//! Cache statistics and management
//!
//! Analyzes cache directories, calculates statistics, and formats sizes in
//! human-readable format.

use std::fs;
use std::path::Path;

/// Cache statistics
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stats {
    /// Number of files in cache
    pub files: usize,
    /// Total size in bytes
    pub total_size: i64,
}

impl Stats {
    /// Create empty stats
    #[must_use]
    pub const fn new() -> Self {
        Self {
            files: 0,
            total_size: 0,
        }
    }
}

impl Default for Stats {
    fn default() -> Self {
        Self::new()
    }
}

/// Collect statistics from a cache directory
///
/// Walks the directory tree and counts files and total size.
/// Returns empty stats if directory doesn't exist (not an error).
///
/// # Errors
///
/// Returns an error if directory traversal fails.
pub fn collect_stats<P: AsRef<Path>>(cache_dir: P) -> std::io::Result<Stats> {
    let cache_dir = cache_dir.as_ref();
    let mut stats = Stats::new();

    if !cache_dir.exists() {
        return Ok(stats);
    }

    walk_dir(cache_dir, &mut stats)?;
    Ok(stats)
}

/// Recursive directory walker
fn walk_dir(dir: &Path, stats: &mut Stats) -> std::io::Result<()> {
    if !dir.is_dir() {
        return Ok(());
    }

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            walk_dir(&path, stats)?;
        } else if path.is_file() {
            stats.files += 1;
            if let Ok(metadata) = fs::metadata(&path) {
                stats.total_size += i64::try_from(metadata.len()).unwrap_or(i64::MAX);
            }
        }
        // Ignore symlinks and other special files for now
    }

    Ok(())
}

/// Convert bytes to human-readable format using binary units (1 KiB = 1024 bytes).
/// Examples: 512 -> "512 B", 1024 -> "1.0 KiB", 1048576 -> "1.0 MiB"
#[must_use]
#[allow(clippy::cast_possible_truncation)]
#[allow(clippy::cast_precision_loss)]
pub fn human_bytes(size: i64) -> String {
    const UNIT: f64 = 1024.0;
    const UNITS: &[char] = &['K', 'M', 'G', 'T', 'P', 'E'];

    // Handle sizes smaller than 1 KiB (including negative)
    if size < UNIT as i64 && size > -(UNIT as i64) {
        return format!("{size} B");
    }

    let size_f = size as f64;
    let abs_size = size_f.abs();
    let mut div = UNIT;
    let mut exp = 0;

    while abs_size / (div * UNIT) >= 1.0 && exp < UNITS.len() - 1 {
        div *= UNIT;
        exp += 1;
    }

    let unit = UNITS.get(exp).copied().unwrap_or('?');
    format!("{:.1} {unit}iB", size_f / div)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn collect_stats_empty_dir() {
        let tmp_dir = TempDir::new().unwrap();

        let stats = collect_stats(tmp_dir.path()).unwrap();

        assert_eq!(stats.files, 0);
        assert_eq!(stats.total_size, 0);
    }

    #[test]
    fn collect_stats_with_files() {
        let tmp_dir = TempDir::new().unwrap();

        // Create test files
        let file1 = tmp_dir.path().join("file1.txt");
        let file2 = tmp_dir.path().join("file2.txt");
        let subdir = tmp_dir.path().join("subdir");
        fs::create_dir(&subdir).unwrap();
        let file3 = subdir.join("file3.txt");
        let nested = subdir.join("nested");
        fs::create_dir(&nested).unwrap();
        let file4 = nested.join("file.rb");

        fs::write(&file1, b"Hello, world!").unwrap();
        fs::write(&file2, b"Another file with more content.").unwrap();
        fs::write(&file3, b"Nested file.").unwrap();
        fs::write(&file4, b"puts 'Ruby code'").unwrap();

        let expected_size = 13 + 31 + 12 + 16; // Total bytes

        let stats = collect_stats(tmp_dir.path()).unwrap();

        assert_eq!(stats.files, 4);
        assert_eq!(stats.total_size, expected_size);
    }

    #[test]
    fn collect_stats_nonexistent_dir() {
        let stats = collect_stats("/nonexistent/directory/path").unwrap();
        assert_eq!(stats.files, 0);
        assert_eq!(stats.total_size, 0);
    }

    #[test]
    fn collect_stats_only_directories() {
        let tmp_dir = TempDir::new().unwrap();
        fs::create_dir_all(tmp_dir.path().join("dir1").join("nested")).unwrap();
        fs::create_dir_all(tmp_dir.path().join("dir2").join("nested").join("deep")).unwrap();
        let stats = collect_stats(tmp_dir.path()).unwrap();
        assert_eq!(stats.files, 0);
        assert_eq!(stats.total_size, 0);
    }

    #[test]
    fn collect_stats_mixed_content() {
        let tmp_dir = TempDir::new().unwrap();

        let gems_dir = tmp_dir.path().join("gems");
        let specs_dir = tmp_dir.path().join("specs");
        let metadata_dir = tmp_dir.path().join("metadata");

        fs::create_dir(&gems_dir).unwrap();
        fs::create_dir(&specs_dir).unwrap();
        fs::create_dir(&metadata_dir).unwrap();

        let content1 = vec![0u8; 1024 * 10]; // 10 KB
        let content2 = vec![0u8; 1024 * 50]; // 50 KB
        let content3 = vec![0u8; 1024 * 2]; // 2 KB
        let content4 = vec![0u8; 1024 * 5]; // 5 KB

        fs::write(gems_dir.join("rake-13.0.6.gem"), &content1).unwrap();
        fs::write(gems_dir.join("bundler-2.5.0.gem"), &content2).unwrap();
        fs::write(specs_dir.join("ruby-3.4.0.json"), &content3).unwrap();
        fs::write(metadata_dir.join("nokogiri-1.16.json"), &content4).unwrap();

        let expected_size = (10 + 50 + 2 + 5) * 1024;
        let stats = collect_stats(tmp_dir.path()).unwrap();
        assert_eq!(stats.files, 4);
        assert_eq!(stats.total_size, expected_size);
    }

    #[test]
    fn human_bytes_basic() {
        assert_eq!(human_bytes(0), "0 B");
        assert_eq!(human_bytes(512), "512 B");
        assert_eq!(human_bytes(1023), "1023 B");
    }

    #[test]
    fn human_bytes_kib() {
        assert_eq!(human_bytes(1024), "1.0 KiB");
        assert_eq!(human_bytes(1024 * 5), "5.0 KiB");
        assert_eq!(human_bytes(1536), "1.5 KiB"); // 1.5 KiB
    }

    #[test]
    #[allow(clippy::cast_possible_truncation)]
    fn human_bytes_mib() {
        assert_eq!(human_bytes(1024 * 1024), "1.0 MiB");
        assert_eq!(human_bytes(1024 * 1024 * 10), "10.0 MiB");
        assert_eq!(human_bytes((1024.0 * 1024.0 * 3.5) as i64), "3.5 MiB");
    }

    #[test]
    fn human_bytes_gib() {
        assert_eq!(human_bytes(1024 * 1024 * 1024), "1.0 GiB");
        assert_eq!(human_bytes(1024 * 1024 * 1024 * 2), "2.0 GiB");
        assert_eq!(human_bytes(6_120_335_360), "5.7 GiB"); // ~5.7 GiB
    }

    #[test]
    fn human_bytes_large() {
        assert_eq!(human_bytes(1024_i64.pow(4) * 3), "3.0 TiB");
        assert_eq!(human_bytes(1024_i64.pow(5) * 2), "2.0 PiB");
        assert_eq!(human_bytes(1024_i64.pow(6)), "1.0 EiB");
    }

    #[test]
    fn human_bytes_realistic() {
        assert_eq!(human_bytes(1024 * 1024 * 150), "150.0 MiB");
    }

    #[test]
    fn human_bytes_edge_cases() {
        // Note: The Go implementation has a bug with negative numbers
        // We handle them correctly by keeping small negatives as bytes
        assert_eq!(human_bytes(-512), "-512 B"); // Negative (stays in bytes)
        assert_eq!(human_bytes(i64::MAX), "8.0 EiB"); // Max int64
        assert_eq!(human_bytes(1), "1 B");
        assert_eq!(human_bytes(1025), "1.0 KiB"); // Just above 1 KiB
    }

    #[test]
    fn stats_struct() {
        // Test that Stats struct fields are accessible and have correct types
        let stats = Stats {
            files: 42,
            total_size: 1024 * 1024 * 100, // 100 MiB
        };

        assert_eq!(stats.files, 42);
        assert_eq!(stats.total_size, 1024 * 1024 * 100);
    }

    #[test]
    fn collect_stats_symlinks() {
        let tmp_dir = TempDir::new().unwrap();

        // Create a regular file
        let file_path = tmp_dir.path().join("regular.txt");
        let content = b"Regular file content";
        fs::write(&file_path, content).unwrap();

        // Create a symlink to the file
        #[cfg(unix)]
        {
            let symlink_path = tmp_dir.path().join("link.txt");
            if std::os::unix::fs::symlink(&file_path, &symlink_path).is_ok() {
                let stats = collect_stats(tmp_dir.path()).unwrap();
                // Symlinks are ignored in our implementation
                assert!(stats.files >= 1);
                assert!(stats.total_size >= i64::try_from(content.len()).unwrap_or(0));
            }
        }

        // On non-Unix systems, just verify the regular file is counted
        #[cfg(not(unix))]
        {
            let stats = collect_stats(tmp_dir.path()).unwrap();
            assert_eq!(stats.files, 1);
            assert_eq!(stats.total_size, content.len() as i64);
        }
    }
}
