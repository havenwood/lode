//! Which command
//!
//! Find the location of a library file

use anyhow::{Context, Result};
use std::path::PathBuf;
use std::process::Command;

/// Options for the gem which command
#[derive(Debug)]
pub(crate) struct WhichOptions {
    /// Show all matching files (not just the first)
    pub all: bool,
    /// Search gems before non-gems
    pub gems_first: bool,
    /// Verbose output
    pub verbose: bool,
    /// Quiet output (suppress progress)
    pub quiet: bool,
    /// Silent output (suppress all output)
    pub silent: bool,
}

/// Find and display the location of library files
pub(crate) fn run(files: &[String], options: &WhichOptions) -> Result<()> {
    if files.is_empty() {
        anyhow::bail!("Please specify at least one file to find");
    }

    // Get Ruby's load path
    let load_path = get_ruby_load_path()?;

    if options.verbose && !options.quiet && !options.silent {
        println!("Searching in {} directories", load_path.len());
    }

    let mut found_any = false;

    for file in files {
        let matches = find_file_in_load_path(file, &load_path, options);

        if matches.is_empty() {
            if !options.silent {
                eprintln!("Can't find Ruby library file or shared library {file}");
            }
        } else {
            found_any = true;

            // Only print results if not silent
            if !options.silent {
                if options.all {
                    // Show all matches
                    for path in matches {
                        println!("{}", path.display());
                    }
                } else if let Some(first) = matches.first() {
                    // Show only the first match
                    println!("{}", first.display());
                }
            }
        }
    }

    // Exit with code 1 if no files were found
    if !found_any {
        std::process::exit(1);
    }

    Ok(())
}

/// Get Ruby's load path ($`LOAD_PATH`) and all gem library paths
fn get_ruby_load_path() -> Result<Vec<PathBuf>> {
    // Get both $LOAD_PATH and all gem lib directories using Gem.find_files approach
    let ruby_code = r"
require 'rubygems'
# Get all gem lib directories
gem_paths = Gem::Specification.map { |spec| spec.full_require_paths }.flatten.uniq
# Combine with $LOAD_PATH (which includes non-gem dirs)
all_paths = ($LOAD_PATH + gem_paths).uniq
puts all_paths
";

    let output = Command::new("ruby")
        .args(["-e", ruby_code])
        .output()
        .context("Failed to execute ruby command to get load path")?;

    if !output.status.success() {
        anyhow::bail!(
            "Ruby command failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let paths = String::from_utf8(output.stdout)
        .context("Invalid UTF-8 in Ruby output")?
        .lines()
        .map(|line| PathBuf::from(line.trim()))
        .filter(|path| path.exists())
        .collect();

    Ok(paths)
}

/// Find a file in the Ruby load path
fn find_file_in_load_path(
    file: &str,
    load_path: &[PathBuf],
    options: &WhichOptions,
) -> Vec<PathBuf> {
    let mut matches = Vec::new();

    // Normalize the file path - remove leading .rb if present for searching
    let file_base = file.strip_suffix(".rb").unwrap_or(file);

    if options.gems_first {
        // Search gems first, then non-gems
        let (gem_paths, non_gem_paths) = split_gem_and_non_gem_paths(load_path);

        search_paths(&gem_paths, file, file_base, &mut matches, options);
        if !options.all && !matches.is_empty() {
            return matches;
        }

        search_paths(&non_gem_paths, file, file_base, &mut matches, options);
    } else {
        // Search all paths in order
        search_paths(load_path, file, file_base, &mut matches, options);
    }

    matches
}

/// Split paths into gem and non-gem paths
fn split_gem_and_non_gem_paths(paths: &[PathBuf]) -> (Vec<PathBuf>, Vec<PathBuf>) {
    let mut gem_paths = Vec::new();
    let mut non_gem_paths = Vec::new();

    for path in paths {
        let path_str = path.to_string_lossy();
        if path_str.contains("/gems/") || path_str.contains("/bundler/gems/") {
            gem_paths.push(path.clone());
        } else {
            non_gem_paths.push(path.clone());
        }
    }

    (gem_paths, non_gem_paths)
}

/// Search for a file in the given paths
fn search_paths(
    paths: &[PathBuf],
    original_file: &str,
    file_base: &str,
    matches: &mut Vec<PathBuf>,
    options: &WhichOptions,
) {
    for dir in paths {
        // Try exact match first (if user specified .rb)
        if std::path::Path::new(original_file)
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("rb"))
        {
            let candidate = dir.join(original_file);
            if candidate.exists() && candidate.is_file() {
                matches.push(candidate);
                if !options.all {
                    return;
                }
            }
        }

        // Try with .rb extension
        let candidate_rb = dir.join(format!("{file_base}.rb"));
        if candidate_rb.exists() && candidate_rb.is_file() {
            matches.push(candidate_rb);
            if !options.all {
                return;
            }
        }

        // Try as directory with file_base.rb inside
        let candidate_dir = dir.join(file_base).join(format!(
            "{}.rb",
            file_base.split('/').next_back().unwrap_or(file_base)
        ));
        if candidate_dir.exists() && candidate_dir.is_file() {
            matches.push(candidate_dir);
            if !options.all {
                return;
            }
        }

        // Try shared libraries (.so, .bundle, .dylib)
        for ext in &["so", "bundle", "dylib"] {
            let candidate_so = dir.join(format!("{file_base}.{ext}"));
            if candidate_so.exists() && candidate_so.is_file() {
                matches.push(candidate_so);
                if !options.all {
                    return;
                }
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "Tests can panic")]
mod tests {
    use super::*;

    /// Test that run requires at least one file
    #[test]
    fn test_run_requires_files() {
        let options = WhichOptions {
            all: false,
            gems_first: false,
            verbose: false,
            quiet: true,
            silent: true,
        };
        let result = run(&[], &options);
        assert!(result.is_err(), "Should error with no files");
    }

    /// Test that `get_ruby_load_path` returns paths
    #[test]
    fn test_get_ruby_load_path() {
        let load_path = get_ruby_load_path();
        assert!(load_path.is_ok(), "Should get Ruby load path");
        let paths = load_path.unwrap();
        assert!(!paths.is_empty(), "Load path should not be empty");
    }

    /// Test `split_gem_and_non_gem_paths`
    #[test]
    fn test_split_gem_and_non_gem_paths() {
        let paths = vec![
            PathBuf::from("/usr/lib/ruby/3.5.0"),
            PathBuf::from("/home/user/.gem/ruby/3.5.0/gems/rake-13.0.0/lib"),
            PathBuf::from("/usr/lib/ruby/site_ruby"),
            PathBuf::from("/home/user/.gem/ruby/3.5.0/gems/json-2.7.0/lib"),
        ];

        let (gem_paths, non_gem_paths) = split_gem_and_non_gem_paths(&paths);

        assert_eq!(gem_paths.len(), 2, "Should find 2 gem paths");
        assert_eq!(non_gem_paths.len(), 2, "Should find 2 non-gem paths");
    }

    /// Test `find_file_in_load_path` with empty load path
    #[test]
    fn test_find_file_empty_load_path() {
        let options = WhichOptions {
            all: false,
            gems_first: false,
            verbose: false,
            quiet: true,
            silent: true,
        };
        let result = find_file_in_load_path("rake", &[], &options);
        assert!(
            result.is_empty(),
            "Should find no matches with empty load path"
        );
    }
}
