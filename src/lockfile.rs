//! Gemfile.lock parsing and generation
//!
//! Parses and generates Bundler-compatible Gemfile.lock files with support
//! for GEM, GIT, PATH sections, platforms, and dependency specifications.

use std::fmt;
use thiserror::Error;

/// Represents a gem specification from Gemfile.lock
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GemSpec {
    /// Gem name (e.g., "rails")
    pub name: String,
    /// Gem version (e.g., "7.0.8")
    pub version: String,
    /// Platform constraint (e.g., "ruby", "arm64-darwin", None for universal)
    pub platform: Option<String>,
    /// Direct dependencies of this gem
    pub dependencies: Vec<Dependency>,
    /// Groups this gem belongs to (e.g., `["default", "development"]`)
    pub groups: Vec<String>,
    /// SHA256 checksum of the gem file (optional)
    pub checksum: Option<String>,
    /// Cached full name (computed once during construction)
    full_name_cached: String,
    /// Cached full name with platform (computed once during construction)
    full_name_with_platform_cached: String,
}

impl GemSpec {
    /// Create a new `GemSpec` with pre-computed cached names
    #[must_use]
    pub fn new(
        name: String,
        version: String,
        platform: Option<String>,
        dependencies: Vec<Dependency>,
        groups: Vec<String>,
    ) -> Self {
        let full_name_cached = format!("{name}-{version}");
        let full_name_with_platform_cached = platform.as_ref().map_or_else(
            || full_name_cached.clone(),
            |p| format!("{name}-{version}-{p}"),
        );

        Self {
            name,
            version,
            platform,
            dependencies,
            groups,
            checksum: None,
            full_name_cached,
            full_name_with_platform_cached,
        }
    }

    /// Get full name with version (e.g., "rails-7.0.8").
    #[must_use]
    #[inline]
    pub fn full_name(&self) -> &str {
        &self.full_name_cached
    }

    /// Get full name with platform if present (e.g., "nokogiri-1.14.0-arm64-darwin").
    #[must_use]
    #[inline]
    pub fn full_name_with_platform(&self) -> &str {
        &self.full_name_with_platform_cached
    }
}

/// Represents a gem dependency with version constraint
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dependency {
    /// Name of the dependency
    pub name: String,
    /// Version requirement (e.g., "~> 3.0", ">= 2.0, < 4.0")
    pub requirement: String,
}

/// Represents a gem from a git source
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitGemSpec {
    pub name: String,
    pub version: String,
    pub repository: String,
    pub revision: String,
    pub branch: Option<String>,
    pub tag: Option<String>,
    pub groups: Vec<String>,
}

/// Represents a gem from a local path
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathGemSpec {
    pub name: String,
    pub version: String,
    pub path: String,
    pub groups: Vec<String>,
}

/// Complete representation of a Gemfile.lock
#[derive(Debug, Clone)]
pub struct Lockfile {
    /// Gems from rubygems.org
    pub gems: Vec<GemSpec>,
    /// Gems from git repositories
    pub git_gems: Vec<GitGemSpec>,
    /// Gems from local paths
    pub path_gems: Vec<PathGemSpec>,
    /// Supported platforms
    pub platforms: Vec<String>,
    /// Ruby version constraint
    pub ruby_version: Option<String>,
    /// Bundler version used to generate lockfile
    pub bundled_with: Option<String>,
}

impl Lockfile {
    /// Create an empty lockfile
    #[must_use]
    pub const fn new() -> Self {
        Self {
            gems: Vec::new(),
            git_gems: Vec::new(),
            path_gems: Vec::new(),
            platforms: Vec::new(),
            ruby_version: None,
            bundled_with: None,
        }
    }

    /// Parse a lockfile from string content
    ///
    /// # Errors
    ///
    /// Returns an error if the lockfile format is invalid or cannot be parsed.
    pub fn parse(content: &str) -> Result<Self, LockfileError> {
        Parser::new(content).parse()
    }
}

impl Default for Lockfile {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Error)]
pub enum LockfileError {
    #[error("failed to parse lockfile at line {line}: {message}")]
    ParseError { line: usize, message: String },

    #[error("invalid gem specification at line {line}: {message}")]
    InvalidSpec { line: usize, message: String },

    #[error("unexpected section: {0}")]
    UnexpectedSection(String),
}

/// Parser for Gemfile.lock format
struct Parser<'a> {
    lines: Vec<&'a str>,
    pos: usize,
    current_line: usize,
}

impl<'a> Parser<'a> {
    fn new(content: &'a str) -> Self {
        Self {
            lines: content.lines().collect(),
            pos: 0,
            current_line: 1,
        }
    }

    fn parse(&mut self) -> Result<Lockfile, LockfileError> {
        let mut lockfile = Lockfile::new();

        while !self.is_eof() {
            let line = self.current();

            if line.is_empty() {
                self.advance();
                continue;
            }

            // Parse sections based on header
            match line.trim() {
                "GEM" => {
                    self.advance();
                    self.parse_gem_section(&mut lockfile)?;
                }
                "GIT" => {
                    self.advance();
                    self.parse_git_section(&mut lockfile);
                }
                "PATH" => {
                    self.advance();
                    self.parse_path_section(&mut lockfile);
                }
                "PLATFORMS" => {
                    self.advance();
                    self.parse_platforms(&mut lockfile);
                }
                "DEPENDENCIES" => {
                    self.advance();
                    self.skip_until_section();
                }
                "CHECKSUMS" => {
                    self.advance();
                    self.parse_checksums(&mut lockfile);
                }
                "RUBY VERSION" => {
                    self.advance();
                    lockfile.ruby_version = self.parse_ruby_version();
                }
                "BUNDLED WITH" => {
                    self.advance();
                    lockfile.bundled_with = self.parse_bundled_with();
                }
                _ => {
                    self.advance();
                }
            }
        }

        Ok(lockfile)
    }

    fn parse_gem_section(&mut self, lockfile: &mut Lockfile) -> Result<(), LockfileError> {
        // Skip "remote:" line
        while !self.is_eof() && self.current().starts_with("  remote:") {
            self.advance();
        }

        // Parse "specs:" section
        if !self.is_eof() && self.current().trim() == "specs:" {
            self.advance();

            while !self.is_eof() {
                let line = self.current();

                // Check if we've reached a new section
                if !line.starts_with("    ") && !line.is_empty() {
                    break;
                }

                if line.starts_with("    ") && !line.starts_with("      ") {
                    // This is a gem spec line
                    let gem = self.parse_gem_spec()?;
                    lockfile.gems.push(gem);
                } else {
                    self.advance();
                }
            }
        }

        Ok(())
    }

    fn parse_gem_spec(&mut self) -> Result<GemSpec, LockfileError> {
        let line = self.current().trim();

        // Parse gem name and version: "rails (7.0.8)" or "nokogiri (1.14.0-arm64-darwin)"
        let (name, version, platform) = self.parse_gem_line(line)?;

        self.advance();

        // Parse dependencies (lines starting with 6 spaces)
        let mut dependencies = Vec::new();
        while !self.is_eof() {
            let line = self.current();

            if !line.starts_with("      ") || line.trim().is_empty() {
                break;
            }

            let dep = Self::parse_dependency(line.trim());
            dependencies.push(dep);
            self.advance();
        }

        Ok(GemSpec::new(
            name,
            version,
            platform,
            dependencies,
            Vec::new(), // Groups are enriched from Gemfile later
        ))
    }

    fn parse_gem_line(
        &self,
        line: &str,
    ) -> Result<(String, String, Option<String>), LockfileError> {
        // Format: "gem-name (version)" or "gem-name (version-platform)"
        let parts: Vec<&str> = line.splitn(2, " (").collect();
        if parts.len() != 2 {
            return Err(LockfileError::InvalidSpec {
                line: self.current_line,
                message: format!("expected format 'name (version)', got: {line}"),
            });
        }

        let name = parts
            .first()
            .ok_or_else(|| LockfileError::ParseError {
                line: self.current_line,
                message: format!("missing gem name in: {line}"),
            })?
            .to_string();
        let version_part = parts
            .get(1)
            .ok_or_else(|| LockfileError::ParseError {
                line: self.current_line,
                message: format!("missing version in: {line}"),
            })?
            .trim_end_matches(')');

        // Check for platform suffix
        // Platforms look like: "arm64-darwin", "x86_64-linux", "java", "mswin32", etc.
        // We need to distinguish from version suffixes like "1.0.0-beta"
        // Strategy: Look for known platform keywords
        if let Some((version, platform)) = Self::split_version_platform(version_part) {
            Ok((name, version.to_string(), Some(platform.to_string())))
        } else {
            Ok((name, version_part.to_string(), None))
        }
    }

    fn split_version_platform(version_part: &str) -> Option<(&str, &str)> {
        // Known platform patterns
        let platform_keywords = [
            "darwin", "linux", "mingw", "mswin", "java", "jruby", "x86_64", "aarch64", "arm64",
            "x86", "i386",
        ];

        // Try to find a platform keyword in the version string
        for keyword in &platform_keywords {
            if version_part.contains(keyword) {
                // Find where the platform starts
                if let Some(dash_pos) = version_part.rfind('-') {
                    let potential_platform = &version_part[dash_pos + 1..];

                    // Check if the part after the dash contains a platform keyword
                    if platform_keywords
                        .iter()
                        .any(|k| potential_platform.contains(k))
                    {
                        // Found a platform! But we might need to go back further
                        // e.g., "1.14.0-arm64-darwin" should split to "1.14.0" and "arm64-darwin"
                        let mut split_pos = dash_pos;

                        // Look for an earlier dash that might be part of the platform
                        let before_platform = &version_part[..dash_pos];
                        if let Some(prev_dash) = before_platform.rfind('-') {
                            let middle_part = &version_part[prev_dash + 1..dash_pos];
                            // If the middle part also looks like platform (e.g., "arm64"), include it
                            if platform_keywords.iter().any(|k| middle_part.contains(k)) {
                                split_pos = prev_dash;
                            }
                        }

                        return Some((&version_part[..split_pos], &version_part[split_pos + 1..]));
                    }
                }
            }
        }

        None
    }

    fn parse_dependency(line: &str) -> Dependency {
        // Format: "rack (~> 2.0)" or "rack (>= 2.0, < 3.0)" or just "rack"
        line.find(" (").map_or_else(
            || Dependency {
                name: line.to_string(),
                requirement: ">= 0".to_string(),
            },
            |open_paren| {
                let name = line[..open_paren].to_string();
                let requirement = line[open_paren + 2..line.len() - 1].to_string();
                Dependency { name, requirement }
            },
        )
    }

    fn parse_git_section(&mut self, lockfile: &mut Lockfile) {
        // Parse GIT section format:
        // GIT
        //   remote: https://github.com/user/repo
        //   revision: abc123def456...
        //   branch: main
        //   specs:
        //     gem_name (version)

        // Read the remote URL
        let mut remote = String::new();
        if !self.is_eof() && self.current().trim().starts_with("remote:") {
            remote = self
                .current()
                .trim()
                .strip_prefix("remote:")
                .unwrap_or("")
                .trim()
                .to_string();
            self.advance();
        }

        // Read the revision (commit SHA)
        let mut revision = String::new();
        if !self.is_eof() && self.current().trim().starts_with("revision:") {
            revision = self
                .current()
                .trim()
                .strip_prefix("revision:")
                .unwrap_or("")
                .trim()
                .to_string();
            self.advance();
        }

        // Read optional branch
        let mut branch = None;
        if !self.is_eof() && self.current().trim().starts_with("branch:") {
            branch = Some(
                self.current()
                    .trim()
                    .strip_prefix("branch:")
                    .unwrap_or("")
                    .trim()
                    .to_string(),
            );
            self.advance();
        }

        // Read optional tag
        let mut tag = None;
        if !self.is_eof() && self.current().trim().starts_with("tag:") {
            tag = Some(
                self.current()
                    .trim()
                    .strip_prefix("tag:")
                    .unwrap_or("")
                    .trim()
                    .to_string(),
            );
            self.advance();
        }

        // Skip to specs section
        while !self.is_eof() && !self.current().trim().starts_with("specs:") {
            self.advance();
        }

        if !self.is_eof() && self.current().trim() == "specs:" {
            self.advance();

            // Parse gem specs from this git source
            while !self.is_eof() {
                let line = self.current();

                // Check if we've reached a new section
                if !line.starts_with("    ") && !line.is_empty() {
                    break;
                }

                if line.starts_with("    ") && !line.starts_with("      ") {
                    // This is a gem spec line
                    let trimmed = line.trim();

                    // Parse gem name and version
                    if let Ok((name, version, _platform)) = self.parse_gem_line(trimmed) {
                        lockfile.git_gems.push(GitGemSpec {
                            name,
                            version,
                            repository: remote.clone(),
                            revision: revision.clone(),
                            branch: branch.clone(),
                            tag: tag.clone(),
                            groups: Vec::new(), // Groups enriched from Gemfile later
                        });
                    }

                    self.advance();

                    // Skip dependencies (we don't track them for git gems currently)
                    while !self.is_eof() && self.current().starts_with("      ") {
                        self.advance();
                    }
                } else {
                    self.advance();
                }
            }
        }
    }

    fn parse_path_section(&mut self, lockfile: &mut Lockfile) {
        // Parse PATH section format:
        // PATH
        //   remote: ../mylib
        //   specs:
        //     mylib (1.0.0)

        // Read the remote path
        let mut remote_path = String::new();
        if !self.is_eof() && self.current().trim().starts_with("remote:") {
            remote_path = self
                .current()
                .trim()
                .strip_prefix("remote:")
                .unwrap_or("")
                .trim()
                .to_string();
            self.advance();
        }

        // Skip to specs section
        while !self.is_eof() && !self.current().trim().starts_with("specs:") {
            self.advance();
        }

        if !self.is_eof() && self.current().trim() == "specs:" {
            self.advance();

            // Parse gem specs from this path source
            while !self.is_eof() {
                let line = self.current();

                // Check if we've reached a new section
                if !line.starts_with("    ") && !line.is_empty() {
                    break;
                }

                if line.starts_with("    ") && !line.starts_with("      ") {
                    // This is a gem spec line
                    let trimmed = line.trim();

                    // Parse gem name and version
                    if let Ok((name, version, _platform)) = self.parse_gem_line(trimmed) {
                        lockfile.path_gems.push(PathGemSpec {
                            name,
                            version,
                            path: remote_path.clone(),
                            groups: Vec::new(), // Groups enriched from Gemfile later
                        });
                    }

                    self.advance();

                    // Skip dependencies (we don't track them for path gems currently)
                    while !self.is_eof() && self.current().starts_with("      ") {
                        self.advance();
                    }
                } else {
                    self.advance();
                }
            }
        }
    }

    fn parse_platforms(&mut self, lockfile: &mut Lockfile) {
        while !self.is_eof() {
            let line = self.current();

            if !line.starts_with("  ") || line.is_empty() {
                break;
            }

            lockfile.platforms.push(line.trim().to_string());
            self.advance();
        }
    }

    fn parse_ruby_version(&mut self) -> Option<String> {
        if !self.is_eof() {
            let line = self.current().trim();
            if line.starts_with("ruby ") {
                let version = line.strip_prefix("ruby ").unwrap_or("").to_string();
                self.advance();
                return Some(version);
            }
        }
        None
    }

    fn parse_bundled_with(&mut self) -> Option<String> {
        if !self.is_eof() {
            let line = self.current().trim();
            let version = line.to_string();
            self.advance();
            return Some(version);
        }
        None
    }

    fn parse_checksums(&mut self, lockfile: &mut Lockfile) {
        while !self.is_eof() {
            let line = self.current();
            if !line.starts_with(' ') && !line.is_empty() {
                break;
            }

            let trimmed = line.trim();
            if trimmed.is_empty() {
                self.advance();
                continue;
            }

            // Parse checksum line: "gem_name (version) sha256=checksum"
            // or "gem_name (version-platform) sha256=checksum"
            if let Some((gem_info, checksum_part)) = trimmed.split_once(" sha256=")
                && let Some((name, version_part)) = gem_info.split_once(" (")
                && let Some(version_str) = version_part.strip_suffix(')')
            {
                // Check if version includes platform (e.g., "1.0.0-x86_64-linux")
                let (version, _platform) = if let Some((v, p)) = version_str.rsplit_once('-') {
                    // Could be version-platform or just a version with dash
                    // Heuristic: if last part looks like a platform, treat it as such
                    if p.contains("linux")
                        || p.contains("darwin")
                        || p.contains("mingw")
                        || p.contains("java")
                    {
                        (v.to_string(), Some(p.to_string()))
                    } else {
                        (version_str.to_string(), None)
                    }
                } else {
                    (version_str.to_string(), None)
                };

                // Find the gem in lockfile and set its checksum
                for gem in &mut lockfile.gems {
                    if gem.name == name && gem.version == version {
                        gem.checksum = Some(checksum_part.to_string());
                        break;
                    }
                }
            }

            self.advance();
        }
    }

    fn skip_until_section(&mut self) {
        while !self.is_eof() {
            let line = self.current();
            if !line.starts_with(' ') && !line.is_empty() {
                break;
            }
            self.advance();
        }
    }

    fn current(&self) -> &str {
        self.lines.get(self.pos).map_or("", |line| *line)
    }

    const fn advance(&mut self) {
        if self.pos < self.lines.len() {
            self.pos += 1;
            self.current_line += 1;
        }
    }

    const fn is_eof(&self) -> bool {
        self.pos >= self.lines.len()
    }
}

impl fmt::Display for GemSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.name, self.version)?;
        if let Some(ref platform) = self.platform {
            write!(f, "-{platform}")?;
        }
        Ok(())
    }
}

impl fmt::Display for Lockfile {
    /// Format Lockfile as Bundler-compatible Gemfile.lock
    ///
    /// Generates the exact format that Bundler expects. The order matters:
    /// GEM, GIT, PATH, PLATFORMS, DEPENDENCIES, RUBY VERSION, BUNDLED WITH
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // GEM section
        if !self.gems.is_empty() {
            writeln!(f, "GEM")?;

            // Group gems by source (for now, assume all from gems.coop)
            writeln!(f, "  remote: {}/", crate::DEFAULT_GEM_SOURCE)?;
            writeln!(f, "  specs:")?;

            // Sort gems alphabetically
            let mut sorted_gems = self.gems.clone();
            sorted_gems.sort_by(|a, b| a.name.cmp(&b.name));

            for gem in &sorted_gems {
                // Write gem line with platform if present
                if let Some(ref platform) = gem.platform {
                    writeln!(f, "    {} ({}-{})", gem.name, gem.version, platform)?;
                } else {
                    writeln!(f, "    {} ({})", gem.name, gem.version)?;
                }

                // Write dependencies (indented with 6 spaces)
                for dep in &gem.dependencies {
                    if dep.requirement.is_empty() || dep.requirement == ">= 0" {
                        writeln!(f, "      {}", dep.name)?;
                    } else {
                        writeln!(f, "      {} ({})", dep.name, dep.requirement)?;
                    }
                }
            }
            writeln!(f)?;
        }

        // GIT section
        if !self.git_gems.is_empty() {
            writeln!(f, "GIT")?;
            // Group by repository
            let mut repos: std::collections::HashMap<String, Vec<&GitGemSpec>> =
                std::collections::HashMap::new();
            for git_gem in &self.git_gems {
                repos
                    .entry(git_gem.repository.clone())
                    .or_default()
                    .push(git_gem);
            }

            for (repo, gems) in repos {
                writeln!(f, "  remote: {repo}")?;
                if let Some(first_gem) = gems.first() {
                    writeln!(f, "  revision: {}", first_gem.revision)?;
                    if let Some(ref branch) = first_gem.branch {
                        writeln!(f, "  branch: {branch}")?;
                    }
                    if let Some(ref tag) = first_gem.tag {
                        writeln!(f, "  tag: {tag}")?;
                    }
                }
                writeln!(f, "  specs:")?;

                for gem in gems {
                    writeln!(f, "    {} ({})", gem.name, gem.version)?;
                }
            }
            writeln!(f)?;
        }

        // PATH section
        if !self.path_gems.is_empty() {
            for path_gem in &self.path_gems {
                writeln!(f, "PATH")?;
                writeln!(f, "  remote: {}", path_gem.path)?;
                writeln!(f, "  specs:")?;
                writeln!(f, "    {} ({})", path_gem.name, path_gem.version)?;
                writeln!(f)?;
            }
        }

        // PLATFORMS section
        if !self.platforms.is_empty() {
            writeln!(f, "PLATFORMS")?;
            for platform in &self.platforms {
                writeln!(f, "  {platform}")?;
            }
            writeln!(f)?;
        }

        // DEPENDENCIES section (simplified - would need Gemfile reference to be accurate)
        // For now, we skip this as it requires tracking which gems are direct dependencies

        // CHECKSUMS section
        let gems_with_checksums: Vec<_> = self
            .gems
            .iter()
            .filter(|gem| gem.checksum.is_some())
            .collect();

        if !gems_with_checksums.is_empty() {
            writeln!(f, "CHECKSUMS")?;
            for gem in gems_with_checksums {
                if let Some(ref checksum) = gem.checksum {
                    if let Some(ref platform) = gem.platform {
                        writeln!(
                            f,
                            "  {} ({}-{}) sha256={}",
                            gem.name, gem.version, platform, checksum
                        )?;
                    } else {
                        writeln!(f, "  {} ({}) sha256={}", gem.name, gem.version, checksum)?;
                    }
                }
            }
            writeln!(f)?;
        }

        // RUBY VERSION section
        if let Some(ref ruby_version) = self.ruby_version {
            writeln!(f, "RUBY VERSION")?;
            writeln!(f, "   {ruby_version}")?;
            writeln!(f)?;
        }

        // BUNDLED WITH section
        if let Some(ref bundled_with) = self.bundled_with {
            writeln!(f, "BUNDLED WITH")?;
            writeln!(f, "   {bundled_with}")?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod parsing {
        use super::*;

        #[test]
        fn simple_lockfile() -> Result<(), LockfileError> {
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

            let lockfile = Lockfile::parse(content)?;
            assert_eq!(lockfile.gems.len(), 2);

            let first_gem = lockfile.gems.first().unwrap();
            assert_eq!(first_gem.name, "rack");
            assert_eq!(first_gem.version, "3.0.8");

            let second_gem = lockfile.gems.get(1).expect("should have second gem");
            assert_eq!(second_gem.name, "rails");
            assert_eq!(second_gem.dependencies.len(), 2);
            assert_eq!(lockfile.platforms.len(), 2);
            assert_eq!(lockfile.ruby_version, Some("3.3.0p0".to_string()));
            assert_eq!(lockfile.bundled_with, Some("2.5.3".to_string()));
            Ok(())
        }

        #[test]
        fn gem_with_platform() -> Result<(), LockfileError> {
            let content = r"
GEM
  remote: https://rubygems.org/
  specs:
    nokogiri (1.14.0-arm64-darwin)
      racc (~> 1.4)
";

            let lockfile = Lockfile::parse(content)?;
            let gem = lockfile.gems.first().expect("should have gem");
            assert_eq!(gem.name, "nokogiri");
            assert_eq!(gem.version, "1.14.0");
            assert_eq!(gem.platform, Some("arm64-darwin".to_string()));
            Ok(())
        }

        #[test]
        fn empty_lockfile() {
            let lockfile = Lockfile::parse("").unwrap();
            assert_eq!(lockfile.gems.len(), 0);
        }

        #[test]
        fn with_dependencies() -> Result<(), LockfileError> {
            let content = r"
GEM
  specs:
    rails (7.0.8)
      actionpack (= 7.0.8)
      rack (~> 2.0, >= 2.2.0)
";

            let lockfile = Lockfile::parse(content)?;
            let gem = lockfile.gems.first().expect("should have gem");
            assert_eq!(gem.dependencies.len(), 2);
            assert_eq!(
                gem.dependencies.first().expect("should have dep").name,
                "actionpack"
            );
            assert_eq!(
                gem.dependencies
                    .get(1)
                    .expect("should have second dep")
                    .requirement,
                "~> 2.0, >= 2.2.0"
            );
            Ok(())
        }

        #[test]
        fn multiple_platforms() -> Result<(), LockfileError> {
            let content = r"
PLATFORMS
  ruby
  x86_64-linux
  arm64-darwin
";

            let lockfile = Lockfile::parse(content)?;
            assert_eq!(lockfile.platforms.len(), 3);
            assert!(lockfile.platforms.contains(&"ruby".to_string()));
            Ok(())
        }

        #[test]
        fn path_gem() -> Result<(), LockfileError> {
            let content = r"
PATH
  remote: ../mylib
  specs:
    mylib (1.0.0)

PLATFORMS
  ruby
";

            let lockfile = Lockfile::parse(content)?;
            let path_gem = lockfile.path_gems.first().expect("should have path gem");
            assert_eq!(path_gem.name, "mylib");
            assert_eq!(path_gem.version, "1.0.0");
            assert_eq!(path_gem.path, "../mylib");
            Ok(())
        }

        #[test]
        fn multiple_path_gems() -> Result<(), LockfileError> {
            let content = r"
PATH
  remote: ../lib1
  specs:
    lib1 (2.0.0)

PATH
  remote: /absolute/path/lib2
  specs:
    lib2 (3.0.0)

PLATFORMS
  ruby
";

            let lockfile = Lockfile::parse(content)?;
            assert_eq!(lockfile.path_gems.len(), 2);
            assert_eq!(
                lockfile
                    .path_gems
                    .first()
                    .expect("should have first path gem")
                    .name,
                "lib1"
            );
            assert_eq!(
                lockfile
                    .path_gems
                    .first()
                    .expect("should have first path gem")
                    .path,
                "../lib1"
            );
            assert_eq!(
                lockfile
                    .path_gems
                    .get(1)
                    .expect("should have second path gem")
                    .name,
                "lib2"
            );
            assert_eq!(
                lockfile
                    .path_gems
                    .get(1)
                    .expect("should have second path gem")
                    .path,
                "/absolute/path/lib2"
            );
            Ok(())
        }

        #[test]
        fn git_gem() -> Result<(), LockfileError> {
            let content = r"
GIT
  remote: https://github.com/rails/rails
  revision: abc123def456
  branch: main
  specs:
    rails (7.1.0.beta)

PLATFORMS
  ruby
";

            let lockfile = Lockfile::parse(content)?;
            let git_gem = lockfile.git_gems.first().expect("should have git gem");
            assert_eq!(git_gem.name, "rails");
            assert_eq!(git_gem.version, "7.1.0.beta");
            assert_eq!(git_gem.repository, "https://github.com/rails/rails");
            assert_eq!(git_gem.revision, "abc123def456");
            assert_eq!(git_gem.branch, Some("main".to_string()));
            assert_eq!(git_gem.tag, None);
            Ok(())
        }

        #[test]
        fn git_gem_with_tag() -> Result<(), LockfileError> {
            let content = r"
GIT
  remote: https://github.com/user/repo
  revision: xyz789
  tag: v2.0.0
  specs:
    mygem (2.0.0)

PLATFORMS
  ruby
";

            let lockfile = Lockfile::parse(content)?;
            let git_gem = lockfile.git_gems.first().expect("should have git gem");
            assert_eq!(git_gem.name, "mygem");
            assert_eq!(git_gem.tag, Some("v2.0.0".to_string()));
            assert_eq!(git_gem.branch, None);
            Ok(())
        }
    }

    mod gem_spec {
        use super::*;

        #[test]
        fn full_name() {
            let spec = GemSpec::new(
                "rails".to_string(),
                "7.0.8".to_string(),
                None,
                vec![],
                vec![],
            );
            assert_eq!(spec.full_name(), "rails-7.0.8");
        }

        #[test]
        fn full_name_with_platform() {
            let spec = GemSpec::new(
                "nokogiri".to_string(),
                "1.14.0".to_string(),
                Some("arm64-darwin".to_string()),
                vec![],
                vec![],
            );
            assert_eq!(
                spec.full_name_with_platform(),
                "nokogiri-1.14.0-arm64-darwin"
            );
        }

        #[test]
        fn display_format() {
            let spec = GemSpec::new(
                "rails".to_string(),
                "7.0.8".to_string(),
                None,
                vec![],
                vec![],
            );
            assert_eq!(format!("{spec}"), "rails (7.0.8)");
        }
    }

    mod lockfile {
        use super::*;

        #[test]
        fn new_creates_empty() {
            let lockfile = Lockfile::new();
            assert_eq!(lockfile.gems.len(), 0);
            assert_eq!(lockfile.platforms.len(), 0);
            assert!(lockfile.ruby_version.is_none());
        }

        #[test]
        fn display_format() {
            let mut lockfile = Lockfile::new();

            lockfile.gems.push(GemSpec::new(
                "rack".to_string(),
                "3.0.8".to_string(),
                None,
                vec![],
                vec![],
            ));

            lockfile.gems.push(GemSpec::new(
                "rails".to_string(),
                "7.0.8".to_string(),
                None,
                vec![
                    Dependency {
                        name: "actionpack".to_string(),
                        requirement: "= 7.0.8".to_string(),
                    },
                    Dependency {
                        name: "activesupport".to_string(),
                        requirement: "= 7.0.8".to_string(),
                    },
                ],
                vec![],
            ));

            lockfile.platforms.push("ruby".to_string());
            lockfile.platforms.push("arm64-darwin".to_string());
            lockfile.ruby_version = Some("ruby 3.3.0p0".to_string());
            lockfile.bundled_with = Some("2.5.3".to_string());

            let output = lockfile.to_string();

            assert!(output.contains("GEM"));
            assert!(output.contains("remote: https://rubygems.org/"));
            assert!(output.contains("specs:"));
            assert!(output.contains("rack (3.0.8)"));
            assert!(output.contains("rails (7.0.8)"));
            assert!(output.contains("actionpack (= 7.0.8)"));
            assert!(output.contains("PLATFORMS"));
            assert!(output.contains("ruby"));
            assert!(output.contains("arm64-darwin"));
            assert!(output.contains("RUBY VERSION"));
            assert!(output.contains("ruby 3.3.0p0"));
            assert!(output.contains("BUNDLED WITH"));
            assert!(output.contains("2.5.3"));
        }
    }
}
