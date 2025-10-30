//! Binstubs Command
//!
//! Generates executable wrappers (binstubs) for specific gems.
//! Useful when you want to regenerate binstubs or create them for gems
//! that weren't installed with `lode install`.

use anyhow::{Context, Result};
use lode::{BinstubGenerator, Config, Lockfile, config};
use std::fs;
use std::path::Path;

/// Options for binstubs generation
#[derive(Debug)]
struct BinstubsOptions<'a> {
    gems: &'a [String],
    shebang: Option<&'a str>,
    force: bool,
    _all: bool,
    _all_platforms: bool,
    lockfile_path_override: Option<&'a str>,
    gems_dir_override: Option<&'a Path>,
    bin_dir_override: Option<&'a Path>,
}

/// Generate binstubs for specific gems.
#[cfg(not(test))]
pub(crate) fn run(
    gems: &[String],
    shebang: Option<&str>,
    force: bool,
    all: bool,
    all_platforms: bool,
) -> Result<()> {
    run_impl(&BinstubsOptions {
        gems,
        shebang,
        force,
        _all: all,
        _all_platforms: all_platforms,
        lockfile_path_override: None,
        gems_dir_override: None,
        bin_dir_override: None,
    })
}

/// Test version with optional path overrides
#[cfg(test)]
pub(crate) fn run(
    gems: &[String],
    shebang: Option<&str>,
    force: bool,
    all: bool,
    all_platforms: bool,
) -> Result<()> {
    run_impl(&BinstubsOptions {
        gems,
        shebang,
        force,
        _all: all,
        _all_platforms: all_platforms,
        lockfile_path_override: None,
        gems_dir_override: None,
        bin_dir_override: None,
    })
}

/// Internal implementation with optional path overrides for testing
fn run_impl(options: &BinstubsOptions<'_>) -> Result<()> {
    let lockfile_path = options.lockfile_path_override.unwrap_or("Gemfile.lock");

    // Read lockfile
    let lockfile_content = fs::read_to_string(lockfile_path)
        .with_context(|| format!("Failed to read lockfile: {lockfile_path}"))?;

    let lockfile = Lockfile::parse(&lockfile_content)
        .with_context(|| format!("Failed to parse lockfile: {lockfile_path}"))?;

    // Determine paths
    let cfg = Config::load().unwrap_or_default();

    let install_path = config::vendor_dir(Some(&cfg)).map_or_else(
        |_| std::env::var("GEM_HOME").unwrap_or_else(|_| String::from("vendor/bundle")),
        |p| p.to_string_lossy().to_string(),
    );

    let gemfile_path = lockfile_path.trim_end_matches(".lock");
    let ruby_version = lode::config::ruby_version_with_gemfile(
        lockfile.ruby_version.as_deref(),
        Some(gemfile_path),
    );
    let base_path = Path::new(&install_path);
    let default_gems_dir = base_path.join("ruby").join(&ruby_version).join("gems");
    let gems_dir = options.gems_dir_override.unwrap_or(&default_gems_dir);

    // Determine bin directory
    let default_binstub_dir = Path::new("bin");
    let binstub_dir = options.bin_dir_override.unwrap_or(default_binstub_dir);

    // Determine Gemfile path from lockfile (supports both Gemfile/gems.rb naming)
    let gemfile_pathbuf = lode::gemfile_for_lockfile(Path::new(lockfile_path));
    let gemfile_path = gemfile_pathbuf.to_str().unwrap_or("Gemfile");

    // Create binstub generator
    let generator = BinstubGenerator::new(
        Path::new(binstub_dir).to_path_buf(),
        Path::new(gemfile_path).to_path_buf(),
        options.shebang.map(String::from),
        options.force,
    );

    // Filter gems from lockfile
    let target_gems: Vec<_> = if options.gems.is_empty() {
        // If no gems specified, generate for all gems with executables
        lockfile.gems.iter().collect()
    } else {
        // Only generate for specified gems
        lockfile
            .gems
            .iter()
            .filter(|gem| options.gems.contains(&gem.name))
            .collect()
    };

    if target_gems.is_empty() {
        if options.gems.is_empty() {
            println!("No gems with executables found in {lockfile_path}");
        } else {
            eprintln!("Error: None of the specified gems were found in {lockfile_path}");
            return Ok(());
        }
    }

    // Generate binstubs
    let mut total_binstubs = 0;
    let mut gems_with_binstubs = 0;

    for gem in target_gems {
        let gem_dir = gems_dir.join(format!("{}-{}", gem.name, gem.version));

        if !gem_dir.exists() {
            eprintln!(
                "Warning: {name} ({version}) is not installed",
                name = gem.name,
                version = gem.version
            );
            continue;
        }

        match generator.generate(&gem.name, &gem_dir) {
            Ok(count) => {
                if count > 0 {
                    total_binstubs += count;
                    gems_with_binstubs += 1;
                }
            }
            Err(e) => {
                eprintln!(
                    "Warning: Failed to generate binstubs for {name}: {e}",
                    name = gem.name
                );
            }
        }
    }

    if total_binstubs > 0 {
        println!(
            "Generated {total_binstubs} binstub{} for {gems_with_binstubs} gem{} in {}",
            if total_binstubs == 1 { "" } else { "s" },
            if gems_with_binstubs == 1 { "" } else { "s" },
            binstub_dir.display(),
        );
    } else if !options.gems.is_empty() {
        println!("No executables found in the specified gems");
    }

    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "Tests can panic")]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    /// Test helper that allows specifying custom paths
    #[allow(clippy::too_many_arguments, reason = "Test helper function")]
    fn run_with_paths(
        gems: &[String],
        shebang: Option<&str>,
        force: bool,
        all: bool,
        all_platforms: bool,
        lockfile_path: &str,
        gems_dir: &Path,
        bin_dir: &Path,
    ) -> Result<()> {
        run_impl(&BinstubsOptions {
            gems,
            shebang,
            force,
            _all: all,
            _all_platforms: all_platforms,
            lockfile_path_override: Some(lockfile_path),
            gems_dir_override: Some(gems_dir),
            bin_dir_override: Some(bin_dir),
        })
    }

    fn create_test_lockfile(dir: &Path) -> String {
        let lockfile_path = dir.join("Gemfile.lock");
        let content = r"GEM
  remote: https://rubygems.org/
  specs:
    rake (13.0.6)
    rails (7.0.8)

PLATFORMS
  ruby

DEPENDENCIES
  rake (~> 13.0)
  rails (~> 7.0)

BUNDLED WITH
   2.4.10
";
        fs::write(&lockfile_path, content).unwrap();
        lockfile_path.to_string_lossy().to_string()
    }

    fn create_test_gem(dir: &Path, name: &str, version: &str, executables: &[&str]) {
        let gem_dir = dir.join(format!("{name}-{version}"));
        let exe_dir = gem_dir.join("exe");

        fs::create_dir_all(&exe_dir).unwrap();

        for exe in executables {
            let exe_path = exe_dir.join(exe);
            fs::write(&exe_path, "#!/usr/bin/env ruby\nputs 'Hello'").unwrap();

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = fs::metadata(&exe_path).unwrap().permissions();
                perms.set_mode(0o755);
                fs::set_permissions(&exe_path, perms).unwrap();
            }
        }
    }

    #[test]
    fn binstubs_run_with_specific_gems() {
        let temp = TempDir::new().unwrap();
        let lockfile = create_test_lockfile(temp.path());

        // Create test gems directory
        let gems_dir = temp.path().join("gems");
        fs::create_dir_all(&gems_dir).unwrap();
        create_test_gem(&gems_dir, "rake", "13.0.6", &["rake"]);
        create_test_gem(&gems_dir, "rails", "7.0.8", &["rails"]);

        // Create Gemfile
        let gemfile = temp.path().join("Gemfile");
        fs::write(&gemfile, "source 'https://rubygems.org'").unwrap();

        let bin_dir = temp.path().join("test_bin");
        fs::create_dir_all(&bin_dir).unwrap();

        let result = run_with_paths(
            &[String::from("rake")],
            None,  // shebang
            false, // force
            false, // all
            false, // all_platforms
            &lockfile,
            &gems_dir,
            &bin_dir,
        );

        assert!(result.is_ok());

        let binstub = bin_dir.join("rake");
        assert!(binstub.exists());
    }

    #[test]
    fn binstubs_with_nonexistent_gem() {
        let temp = TempDir::new().unwrap();
        let lockfile = create_test_lockfile(temp.path());

        let gems_dir = temp.path().join("gems");
        fs::create_dir_all(&gems_dir).unwrap();

        let bin_dir = temp.path().join("test_bin");
        fs::create_dir_all(&bin_dir).unwrap();

        let result = run_with_paths(
            &[String::from("nonexistent")],
            None,  // shebang
            false, // force
            false, // all
            false, // all_platforms
            &lockfile,
            &gems_dir,
            &bin_dir,
        );

        // Should succeed but print a message
        assert!(result.is_ok());
    }

    #[test]
    fn binstubs_missing_lockfile() {
        let temp = TempDir::new().unwrap();
        let nonexistent_lockfile = temp.path().join("nonexistent.lock");

        let gems_dir = temp.path().join("gems");
        fs::create_dir_all(&gems_dir).unwrap();

        let bin_dir = temp.path().join("test_bin");
        fs::create_dir_all(&bin_dir).unwrap();

        let result = run_with_paths(
            &[String::from("rake")],
            None,  // shebang
            false, // force
            false, // all
            false, // all_platforms
            nonexistent_lockfile.to_str().unwrap(),
            &gems_dir,
            &bin_dir,
        );

        assert!(result.is_err());
    }
}
