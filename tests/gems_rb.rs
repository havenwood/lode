//! gems.rb/gems.locked support (modern naming convention)

use lode::{find_gemfile, find_gemfile_in, find_lockfile, find_lockfile_in};
use lode::{gemfile_for_lockfile, lockfile_for_gemfile};
use std::fs;
use std::path::Path;
use tempfile::TempDir;

#[test]
fn finds_gems_rb_when_present() {
    let temp = TempDir::new().expect("should create temp dir");
    fs::write(
        temp.path().join("gems.rb"),
        "source 'https://rubygems.org'\n",
    )
    .expect("should write gems.rb");

    let found = find_gemfile_in(temp.path());
    assert_eq!(found.file_name().expect("should have file name"), "gems.rb");
}

#[test]
fn finds_gemfile_when_no_gems_rb() {
    let temp = TempDir::new().expect("should create temp dir");
    fs::write(
        temp.path().join("Gemfile"),
        "source 'https://rubygems.org'\n",
    )
    .expect("should write Gemfile");

    let found = find_gemfile_in(temp.path());
    assert_eq!(found.file_name().expect("should have file name"), "Gemfile");
}

#[test]
fn prefers_gems_rb_over_gemfile() {
    let temp = TempDir::new().expect("should create temp dir");
    fs::write(
        temp.path().join("gems.rb"),
        "source 'https://rubygems.org'\n",
    )
    .expect("should write gems.rb");
    fs::write(
        temp.path().join("Gemfile"),
        "source 'https://rubygems.org'\n",
    )
    .expect("should write Gemfile");

    let found = find_gemfile_in(temp.path());
    assert_eq!(found.file_name().expect("should have file name"), "gems.rb");
}

#[test]
fn finds_gems_locked_when_present() {
    let temp = TempDir::new().expect("should create temp dir");
    fs::write(temp.path().join("gems.locked"), "GEM\n  specs:\n")
        .expect("should write gems.locked");

    let found = find_lockfile_in(temp.path());
    assert_eq!(
        found.file_name().expect("should have file name"),
        "gems.locked"
    );
}

#[test]
fn finds_gemfile_lock_when_no_gems_locked() {
    let temp = TempDir::new().expect("should create temp dir");
    fs::write(temp.path().join("Gemfile.lock"), "GEM\n  specs:\n")
        .expect("should write Gemfile.lock");

    let found = find_lockfile_in(temp.path());
    assert_eq!(
        found.file_name().expect("should have file name"),
        "Gemfile.lock"
    );
}

#[test]
fn prefers_gems_locked_over_gemfile_lock() {
    let temp = TempDir::new().expect("should create temp dir");
    fs::write(temp.path().join("gems.locked"), "GEM\n  specs:\n")
        .expect("should write gems.locked");
    fs::write(temp.path().join("Gemfile.lock"), "GEM\n  specs:\n")
        .expect("should write Gemfile.lock");

    let found = find_lockfile_in(temp.path());
    assert_eq!(
        found.file_name().expect("should have file name"),
        "gems.locked"
    );
}

#[test]
fn returns_lockfile_for_gems_rb() {
    let lockfile = lockfile_for_gemfile(Path::new("gems.rb"));
    assert_eq!(lockfile, Path::new("gems.locked"));
}

#[test]
fn returns_lockfile_for_gemfile() {
    let lockfile = lockfile_for_gemfile(Path::new("Gemfile"));
    assert_eq!(lockfile, Path::new("Gemfile.lock"));
}

#[test]
fn returns_gemfile_for_gems_locked() {
    let gemfile = gemfile_for_lockfile(Path::new("gems.locked"));
    assert_eq!(gemfile, Path::new("gems.rb"));
}

#[test]
fn returns_gemfile_for_gemfile_lock() {
    let gemfile = gemfile_for_lockfile(Path::new("Gemfile.lock"));
    assert_eq!(gemfile, Path::new("Gemfile"));
}

#[test]
fn finds_current_directory_gems_rb() {
    let temp = TempDir::new().expect("should create temp dir");
    let original_dir = std::env::current_dir().expect("should get current dir");

    fs::write(
        temp.path().join("gems.rb"),
        "source 'https://rubygems.org'\n",
    )
    .expect("should write gems.rb");

    std::env::set_current_dir(temp.path()).expect("should change dir");
    let found = find_gemfile();
    assert_eq!(found.file_name().expect("should have file name"), "gems.rb");
    std::env::set_current_dir(original_dir).expect("should restore dir");
}

#[test]
fn finds_current_directory_gems_locked() {
    let temp = TempDir::new().expect("should create temp dir");
    let original_dir = std::env::current_dir().expect("should get current dir");

    fs::write(temp.path().join("gems.locked"), "GEM\n  specs:\n")
        .expect("should write gems.locked");

    std::env::set_current_dir(temp.path()).expect("should change dir");
    let found = find_lockfile();
    assert_eq!(
        found.file_name().expect("should have file name"),
        "gems.locked"
    );
    std::env::set_current_dir(original_dir).expect("should restore dir");
}
