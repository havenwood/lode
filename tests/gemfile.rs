use lode::Gemfile;
use std::fs;
use tempfile::TempDir;

#[test]
fn parses_from_file() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("Gemfile");
    fs::write(&path, "gem 'rack'").unwrap();

    let gemfile = Gemfile::parse_file(&path).unwrap();
    assert!(!gemfile.gems.is_empty());
}

#[test]
fn parses_empty_gemfile() {
    let gemfile = Gemfile::parse("").unwrap();
    assert_eq!(gemfile.gems.len(), 0);
}

#[test]
fn parses_source() {
    let gemfile = Gemfile::parse(r#"source "https://rubygems.org""#).unwrap();
    assert_eq!(gemfile.source, "https://rubygems.org");
}

#[test]
fn parses_gem_with_version() {
    let gemfile = Gemfile::parse(r#"gem "rails", "~> 7.0""#).unwrap();
    let gem = gemfile.gems.first().unwrap();
    assert_eq!(gem.name, "rails");
    assert_eq!(gem.version_requirement, "~> 7.0");
}

#[test]
fn parses_git_gem() {
    let gemfile = Gemfile::parse(r#"gem "rails", git: "https://github.com/rails/rails""#).unwrap();
    let gem = gemfile.gems.first().unwrap();
    assert!(gem.is_git());
    assert_eq!(gem.git, Some("https://github.com/rails/rails".to_string()));
}
