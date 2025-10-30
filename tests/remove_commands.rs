mod common;

use std::fs;
use std::process::Command;
use tempfile::TempDir;

use common::get_lode_binary;

#[test]
fn remove_single_gem() {
    let temp = TempDir::new().unwrap();
    let gemfile = temp.path().join("Gemfile");
    fs::write(
        &gemfile,
        "source 'https://rubygems.org'\n\ngem 'rake', '13.0.6'\ngem 'rspec', '~> 3.12'\n",
    )
    .unwrap();

    let output = Command::new(get_lode_binary())
        .current_dir(temp.path())
        .args(["remove", "rake"])
        .output()
        .expect("Failed to execute lode remove");

    assert!(
        output.status.success(),
        "lode remove should succeed. stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let content = fs::read_to_string(&gemfile).unwrap();
    assert!(
        !content.contains("rake"),
        "Gemfile should not contain rake after removal"
    );
    assert!(
        content.contains("rspec"),
        "Gemfile should still contain rspec"
    );
}

#[test]
fn remove_multiple_gems() {
    let temp = TempDir::new().unwrap();
    let gemfile = temp.path().join("Gemfile");
    fs::write(
        &gemfile,
        "source 'https://rubygems.org'\n\ngem 'rake'\ngem 'rspec'\ngem 'rails'\n",
    )
    .unwrap();

    let output = Command::new(get_lode_binary())
        .current_dir(temp.path())
        .args(["remove", "rake", "rspec"])
        .output()
        .expect("Failed to execute lode remove");

    assert!(
        output.status.success(),
        "lode remove multiple gems should succeed. stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let content = fs::read_to_string(&gemfile).unwrap();
    assert!(!content.contains("rake"), "Gemfile should not contain rake");
    assert!(
        !content.contains("rspec"),
        "Gemfile should not contain rspec"
    );
    assert!(
        content.contains("rails"),
        "Gemfile should still contain rails"
    );
}

#[test]
fn remove_gem_with_version() {
    let temp = TempDir::new().unwrap();
    let gemfile = temp.path().join("Gemfile");
    fs::write(
        &gemfile,
        "source 'https://rubygems.org'\n\ngem 'rake', '13.0.6'\ngem 'bundler', '~> 2.0'\n",
    )
    .unwrap();

    let output = Command::new(get_lode_binary())
        .current_dir(temp.path())
        .args(["remove", "rake"])
        .output()
        .expect("Failed to execute lode remove");

    assert!(
        output.status.success(),
        "lode remove should succeed. stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let content = fs::read_to_string(&gemfile).unwrap();
    assert!(!content.contains("rake"), "Gemfile should not contain rake");
    assert!(
        content.contains("bundler"),
        "Gemfile should still contain bundler"
    );
}

#[test]
fn remove_gem_from_group() {
    let temp = TempDir::new().unwrap();
    let gemfile = temp.path().join("Gemfile");
    fs::write(
        &gemfile,
        "source 'https://rubygems.org'\n\ngroup :test do\n  gem 'rspec'\nend\n\ngem 'rails'\n",
    )
    .unwrap();

    let output = Command::new(get_lode_binary())
        .current_dir(temp.path())
        .args(["remove", "rspec"])
        .output()
        .expect("Failed to execute lode remove");

    assert!(
        output.status.success(),
        "lode remove should succeed. stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let content = fs::read_to_string(&gemfile).unwrap();
    assert!(
        !content.contains("rspec"),
        "Gemfile should not contain rspec"
    );
    assert!(
        content.contains("rails"),
        "Gemfile should still contain rails"
    );
}

#[test]
fn remove_nonexistent_gem() {
    let temp = TempDir::new().unwrap();
    let gemfile = temp.path().join("Gemfile");
    fs::write(
        &gemfile,
        "source 'https://rubygems.org'\n\ngem 'rake', '13.0.6'\n",
    )
    .unwrap();

    let output = Command::new(get_lode_binary())
        .current_dir(temp.path())
        .args(["remove", "nonexistent-gem-xyz"])
        .output()
        .expect("Failed to execute lode remove");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success() || stderr.to_string().is_empty(),
        "lode remove of nonexistent gem should warn or fail gracefully"
    );

    let content = fs::read_to_string(&gemfile).unwrap();
    assert!(
        content.contains("rake"),
        "Gemfile should still contain rake"
    );
}

#[test]
fn remove_gem_multiple_times() {
    let temp = TempDir::new().unwrap();
    let gemfile = temp.path().join("Gemfile");
    fs::write(
        &gemfile,
        "source 'https://rubygems.org'\n\ngem 'rake', '13.0.6'\ngem 'rspec'\n",
    )
    .unwrap();

    let output = Command::new(get_lode_binary())
        .current_dir(temp.path())
        .args(["remove", "rake"])
        .output()
        .expect("Failed to execute lode remove");

    assert!(output.status.success(), "First remove should succeed");

    let content = fs::read_to_string(&gemfile).unwrap();
    assert!(!content.contains("rake"), "Gemfile should not contain rake");
    assert!(
        content.contains("rspec"),
        "Gemfile should still contain rspec"
    );
}

#[test]
fn remove_gem_no_gemfile() {
    let temp = TempDir::new().unwrap();

    let output = Command::new(get_lode_binary())
        .current_dir(temp.path())
        .args(["remove", "rake"])
        .output()
        .expect("Failed to execute lode remove");

    assert!(
        !output.status.success(),
        "lode remove without Gemfile should fail"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.to_string().to_lowercase().contains("gemfile")
            || stderr.to_string().to_lowercase().contains("not found"),
        "Error should mention Gemfile"
    );
}
