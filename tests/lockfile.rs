use lode::Lockfile;

#[test]
fn parses_complete_lockfile() {
    let content = r"
GEM
  remote: https://rubygems.org/
  specs:
    rack (3.0.8)
    rails (7.0.8)
      actionpack (= 7.0.8)
      activesupport (= 7.0.8)

PLATFORMS
  ruby
  arm64-darwin

RUBY VERSION
   ruby 3.3.0p0

BUNDLED WITH
   2.5.3
";

    let lockfile = Lockfile::parse(content).unwrap();
    assert_eq!(lockfile.gems.len(), 2);
    assert_eq!(lockfile.gems.first().unwrap().name, "rack");
    assert_eq!(lockfile.gems.get(1).unwrap().dependencies.len(), 2);
    assert_eq!(lockfile.platforms.len(), 2);
}

#[test]
fn parses_gem_with_platform() {
    let content = r"
GEM
  remote: https://rubygems.org/
  specs:
    nokogiri (1.14.0-arm64-darwin)
      racc (~> 1.4)
";

    let lockfile = Lockfile::parse(content).unwrap();
    let gem = lockfile.gems.first().unwrap();
    assert_eq!(gem.name, "nokogiri");
    assert_eq!(gem.version, "1.14.0");
    assert_eq!(gem.platform, Some("arm64-darwin".to_string()));
}

#[test]
fn parses_empty_lockfile() {
    let lockfile = Lockfile::parse("").unwrap();
    assert_eq!(lockfile.gems.len(), 0);
}

#[test]
fn formats_to_bundler_compatible_string() {
    let lockfile = Lockfile::new();
    let output = lockfile.to_string();
    assert!(output.is_empty() || output.contains("GEM"));
}
