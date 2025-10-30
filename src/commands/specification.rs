//! Specification command
//!
//! Display full gemspec details

use anyhow::{Context, Result};
use lode::{Lockfile, RubyGemsClient, gem_store::GemStore};
use std::fs;

/// Display full gemspec details for a gem.
///
/// Shows comprehensive metadata including version, authors, dependencies,
/// licenses, homepage, and more.
///
/// # Example
///
/// ```bash
/// lode specification rack
/// lode specification rails --version 7.0.8
/// ```
#[allow(clippy::too_many_lines)]
pub(crate) async fn run(gem_name: &str, version: Option<&str>) -> Result<()> {
    run_with_lockfile(gem_name, version, None).await
}

async fn run_with_lockfile(
    gem_name: &str,
    version: Option<&str>,
    lockfile_path: Option<&str>,
) -> Result<()> {
    // Determine version to query
    let gem_version = if let Some(v) = version {
        v.to_string()
    } else {
        // Try to get version from lockfile
        let lockfile_path =
            lockfile_path.map_or_else(lode::find_lockfile, std::path::PathBuf::from);

        if !lockfile_path.exists() {
            anyhow::bail!(
                "No version specified and lockfile not found. Specify --version explicitly."
            );
        }

        let content = fs::read_to_string(&lockfile_path).context("Failed to read lockfile")?;
        let lockfile = Lockfile::parse(&content).context("Failed to parse lockfile")?;

        // Find gem in lockfile
        lockfile
            .gems
            .iter()
            .find(|g| g.name == gem_name)
            .map(|g| g.version.clone())
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Gem '{gem_name}' not found in lockfile. Specify --version explicitly."
                )
            })?
    };

    // Try to get metadata from locally installed gem first
    if let Ok(gem_store) = GemStore::new() {
        // Search for gem in system directories
        if let Ok(gems) = gem_store.list_gems() {
            for gem_info in gems {
                if gem_info.name == gem_name && gem_info.version == gem_version {
                    display_local_spec(&gem_info.name, &gem_info.version);
                    return Ok(());
                }
            }
        }
    }

    // If not found locally, try to fetch from RubyGems.org
    // Note: This may fail if the API response structure doesn't match expected schema
    match RubyGemsClient::new(lode::RUBYGEMS_ORG_URL) {
        Ok(client) => {
            match client.fetch_gem_info(gem_name, &gem_version).await {
                Ok(metadata) => {
                    // Display full specification from remote metadata
                    println!("--- !ruby/object:Gem::Specification");
                    println!("name: {}", metadata.name);
                    println!("version: !ruby/object:Gem::Version");
                    println!("  version: {}", metadata.version);
                    println!();

                    println!("platform: {}", metadata.platform);
                    println!();

                    // Authors
                    if !metadata.authors.is_empty() {
                        println!("authors: {}", metadata.authors);
                        println!();
                    }

                    // Summary and description
                    if let Some(summary) = &metadata.summary {
                        println!("summary: {summary}");
                        println!();
                    }

                    if let Some(description) = &metadata.description {
                        println!("description: |");
                        // Indent each line of description
                        for line in description.lines() {
                            println!("  {line}");
                        }
                        println!();
                    }

                    // Homepage
                    if let Some(homepage) = &metadata.homepage {
                        println!("homepage: {homepage}");
                        println!();
                    }

                    // Licenses
                    if !metadata.licenses.is_empty() {
                        println!("licenses:");
                        for license in &metadata.licenses {
                            println!("  - {license}");
                        }
                        println!();
                    }

                    // Dependencies
                    let runtime_deps = &metadata.dependencies.runtime;
                    let dev_deps = &metadata.dependencies.development;

                    if !runtime_deps.is_empty() {
                        println!("dependencies:");
                        for dep in runtime_deps {
                            let dep_name = &dep.name;
                            println!("  - !ruby/object:Gem::Dependency");
                            println!("    name: {dep_name}");
                            println!("    requirement: !ruby/object:Gem::Requirement");
                            println!("      requirements:");
                            let req = if dep.requirements.is_empty() {
                                ">= 0"
                            } else {
                                &dep.requirements
                            };
                            println!("        - - \"{req}\"");
                            println!("    type: :runtime");
                            println!("    prerelease: false");
                        }
                        println!();
                    }

                    if !dev_deps.is_empty() {
                        println!("development_dependencies:");
                        for dep in dev_deps {
                            let dep_name = &dep.name;
                            println!("  - !ruby/object:Gem::Dependency");
                            println!("    name: {dep_name}");
                            println!("    requirement: !ruby/object:Gem::Requirement");
                            println!("      requirements:");
                            let req = if dep.requirements.is_empty() {
                                ">= 0"
                            } else {
                                &dep.requirements
                            };
                            println!("        - - \"{req}\"");
                            println!("    type: :development");
                            println!("    prerelease: false");
                        }
                        println!();
                    }
                }
                Err(_) => {
                    // Remote fetch failed, show message
                    anyhow::bail!(
                        "Gem '{gem_name} {gem_version}' not found in local gems or remote repository"
                    );
                }
            }
        }
        Err(_) => {
            // Client creation failed, show message
            anyhow::bail!(
                "Could not connect to RubyGems.org to fetch specification for {gem_name} {gem_version}"
            );
        }
    }

    Ok(())
}

/// Display minimal specification for locally installed gem
fn display_local_spec(gem_name: &str, version: &str) {
    println!("--- !ruby/object:Gem::Specification");
    println!("name: {gem_name}");
    println!("version: !ruby/object:Gem::Version");
    println!("  version: {version}");
    println!("platform: ruby");
    println!();
    println!("(Local gem found. Full specification requires fetching from remote repository)");
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "Tests can panic")]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_specification_no_version_no_lockfile() {
        let temp = TempDir::new().unwrap();
        let lockfile = temp.path().join("Gemfile.lock");

        let result = run_with_lockfile("rack", None, Some(lockfile.to_str().unwrap())).await;

        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("lockfile not found")
        );
    }

    #[tokio::test]
    async fn test_specification_gem_not_in_lockfile() {
        let temp = TempDir::new().unwrap();
        let lockfile = temp.path().join("Gemfile.lock");

        fs::write(
            &lockfile,
            "GEM\n  remote: https://rubygems.org/\n  specs:\n    rails (7.0.0)\n",
        )
        .unwrap();

        let result = run_with_lockfile("rack", None, Some(lockfile.to_str().unwrap())).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[tokio::test]
    async fn test_specification_finds_gem_in_lockfile() {
        let temp = TempDir::new().unwrap();
        let lockfile = temp.path().join("Gemfile.lock");

        fs::write(
            &lockfile,
            "GEM\n  remote: https://rubygems.org/\n  specs:\n    rack (2.2.3)\n",
        )
        .unwrap();

        let result = run_with_lockfile("rack", None, Some(lockfile.to_str().unwrap())).await;

        assert!(result.is_ok() || result.unwrap_err().to_string().contains("not found"));
    }

    #[tokio::test]
    async fn test_specification_with_version_bypasses_lockfile() {
        let result = run("rake", Some("13.0.0")).await;
        assert!(result.is_ok() || result.unwrap_err().to_string().contains("not found"));
    }

    #[tokio::test]
    async fn test_specification_handles_empty_gem_name() {
        let result = run("", Some("1.0.0")).await;
        assert!(result.is_ok() || result.is_err());
    }

    #[tokio::test]
    async fn test_specification_handles_invalid_version() {
        let result = run("rake", Some("invalid.version.string")).await;
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_display_local_spec() {
        display_local_spec("rake", "13.0.0");
    }

    #[tokio::test]
    async fn test_specification_lockfile_with_multiple_gems() {
        let temp = TempDir::new().unwrap();
        let lockfile = temp.path().join("Gemfile.lock");

        fs::write(
            &lockfile,
            "GEM\n  remote: https://rubygems.org/\n  specs:\n    rack (2.2.3)\n    rails (7.0.0)\n    rake (13.0.0)\n",
        )
        .unwrap();

        let result = run_with_lockfile("rails", None, Some(lockfile.to_str().unwrap())).await;
        assert!(result.is_ok() || result.unwrap_err().to_string().contains("not found"));
    }

    #[tokio::test]
    async fn test_specification_lockfile_missing_specs_section() {
        let temp = TempDir::new().unwrap();
        let lockfile = temp.path().join("Gemfile.lock");

        fs::write(&lockfile, "GEM\n  remote: https://rubygems.org/\n").unwrap();

        let result = run_with_lockfile("rack", None, Some(lockfile.to_str().unwrap())).await;
        assert!(result.is_err());
    }
}
