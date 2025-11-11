//! Create standalone bundles that work without `Bundler` or `RubyGems`.

use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

/// Configuration options for standalone bundle generation
#[derive(Debug, Clone)]
pub struct StandaloneOptions {
    /// Path where bundle/ directory will be created (default: ./bundle)
    pub bundle_path: PathBuf,

    /// Groups to include (empty = all groups)
    pub groups: Vec<String>,
}

impl Default for StandaloneOptions {
    fn default() -> Self {
        Self {
            bundle_path: PathBuf::from("./bundle"),
            groups: Vec::new(),
        }
    }
}

/// Represents a gem to be installed in the standalone bundle
#[derive(Debug, Clone)]
pub struct StandaloneGem {
    /// Gem name (e.g., "rack")
    pub name: String,

    /// Gem version (e.g., "3.0.8")
    pub version: String,

    /// Platform (e.g., "ruby", "arm64-darwin")
    pub platform: Option<String>,

    /// Path to extracted gem directory
    pub extracted_path: PathBuf,

    /// Path to built extension directory (if any)
    pub extension_path: Option<PathBuf>,

    /// Whether this gem has native extensions
    pub has_extensions: bool,
}

impl StandaloneGem {
    /// Returns the full gem name with version (e.g., "rack-3.0.8")
    #[must_use]
    pub fn full_name(&self) -> String {
        if let Some(ref platform) = self.platform
            && platform != "ruby"
        {
            return format!("{}-{}-{}", self.name, self.version, platform);
        }
        format!("{}-{}", self.name, self.version)
    }
}

/// Standalone bundle directory structure
#[derive(Debug)]
pub struct StandaloneBundle {
    /// Root path (./bundle)
    root: PathBuf,

    /// Ruby version-specific path (./bundle/ruby/{version})
    pub ruby_path: PathBuf,

    /// Gems directory (./bundle/ruby/{version}/gems)
    gems_path: PathBuf,

    /// Extensions directory (./bundle/ruby/{version}/extensions/{platform}/{version})
    extensions_path: PathBuf,

    /// Cache directory (./bundle/ruby/{version}/cache)
    cache_path: PathBuf,

    /// Specifications directory (./bundle/ruby/{version}/specifications)
    specifications_path: PathBuf,

    /// Bin directory (./bundle/ruby/{version}/bin)
    bin_path: PathBuf,

    /// Ruby version (e.g., "3.3.0")
    ruby_version: String,

    /// Ruby engine (e.g., "ruby", "jruby")
    ruby_engine: String,

    /// Platform (e.g., "arm64-darwin-25")
    platform: String,
}

impl StandaloneBundle {
    /// Create a new standalone bundle structure.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use lode::standalone::{StandaloneBundle, StandaloneOptions};
    /// let options = StandaloneOptions::default();
    /// let bundle = StandaloneBundle::new(options, "3.3.0", "ruby")?;
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if bundle initialization fails.
    pub fn new(options: StandaloneOptions, ruby_version: &str, ruby_engine: &str) -> Result<Self> {
        let root = options.bundle_path;
        let platform = crate::platform::detect_current_platform();

        // Ruby version-specific directory
        let ruby_path = root.join(ruby_engine).join(ruby_version);

        Ok(Self {
            gems_path: ruby_path.join("gems"),
            extensions_path: ruby_path
                .join("extensions")
                .join(&platform)
                .join(ruby_version),
            cache_path: ruby_path.join("cache"),
            specifications_path: ruby_path.join("specifications"),
            bin_path: ruby_path.join("bin"),
            ruby_path,
            root,
            ruby_version: ruby_version.to_string(),
            ruby_engine: ruby_engine.to_string(),
            platform,
        })
    }

    /// Create all necessary directories for the standalone bundle
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use lode::standalone::{StandaloneBundle, StandaloneOptions};
    /// let bundle = StandaloneBundle::new(StandaloneOptions::default(), "3.3.0", "ruby")?;
    /// bundle.create_directories()?;
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if directory creation fails.
    pub fn create_directories(&self) -> Result<()> {
        fs::create_dir_all(&self.root)
            .with_context(|| format!("Failed to create bundle root: {}", self.root.display()))?;

        fs::create_dir_all(&self.gems_path).with_context(|| {
            format!(
                "Failed to create gems directory: {}",
                self.gems_path.display()
            )
        })?;

        fs::create_dir_all(&self.extensions_path).with_context(|| {
            format!(
                "Failed to create extensions directory: {}",
                self.extensions_path.display()
            )
        })?;

        fs::create_dir_all(&self.cache_path).with_context(|| {
            format!(
                "Failed to create cache directory: {}",
                self.cache_path.display()
            )
        })?;

        fs::create_dir_all(&self.specifications_path).with_context(|| {
            format!(
                "Failed to create specifications directory: {}",
                self.specifications_path.display()
            )
        })?;

        fs::create_dir_all(&self.bin_path).with_context(|| {
            format!(
                "Failed to create bin directory: {}",
                self.bin_path.display()
            )
        })?;

        // Create bundler/ directory for setup.rb
        let bundler_dir = self.root.join("bundler");
        fs::create_dir_all(&bundler_dir).with_context(|| {
            format!(
                "Failed to create bundler directory: {}",
                bundler_dir.display()
            )
        })?;

        Ok(())
    }

    /// Install a gem into the standalone bundle.
    ///
    /// Copies the gem's files to the appropriate locations in the bundle directory.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use lode::standalone::{StandaloneBundle, StandaloneOptions, StandaloneGem};
    /// # use std::path::PathBuf;
    /// let bundle = StandaloneBundle::new(StandaloneOptions::default(), "3.3.0", "ruby")?;
    /// bundle.create_directories()?;
    ///
    /// let gem = StandaloneGem {
    ///     name: "rack".to_string(),
    ///     version: "3.0.8".to_string(),
    ///     platform: Some("ruby".to_string()),
    ///     extracted_path: PathBuf::from("/path/to/rack-3.0.8"),
    ///     extension_path: None,
    ///     has_extensions: false,
    /// };
    ///
    /// bundle.install_gem(&gem)?;
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if gem installation fails.
    pub fn install_gem(&self, gem: &StandaloneGem) -> Result<()> {
        let gem_name = gem.full_name();

        let dest_gem_path = self.gems_path.join(&gem_name);
        copy_dir_recursive(&gem.extracted_path, &dest_gem_path).with_context(|| {
            format!(
                "Failed to copy gem {} to {}",
                gem_name,
                dest_gem_path.display()
            )
        })?;

        if let Some(ref ext_path) = gem.extension_path
            && ext_path.exists()
        {
            let dest_ext_path = self.extensions_path.join(&gem_name);
            copy_dir_recursive(ext_path, &dest_ext_path).with_context(|| {
                format!(
                    "Failed to copy extensions for {} to {}",
                    gem_name,
                    dest_ext_path.display()
                )
            })?;
        }

        Ok(())
    }

    /// Generate bundle/bundler/setup.rb
    ///
    /// This file manipulates Ruby's `$LOAD_PATH` to make gems available
    /// without requiring Bundler or `RubyGems`.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use lode::standalone::{StandaloneBundle, StandaloneOptions, StandaloneGem};
    /// # use std::path::PathBuf;
    /// let bundle = StandaloneBundle::new(StandaloneOptions::default(), "3.3.0", "ruby")?;
    /// let gems = vec![
    ///     StandaloneGem {
    ///         name: "rack".to_string(),
    ///         version: "3.0.8".to_string(),
    ///         platform: Some("ruby".to_string()),
    ///         extracted_path: PathBuf::from("/tmp/rack"),
    ///         extension_path: None,
    ///         has_extensions: false,
    ///     }
    /// ];
    /// bundle.generate_setup_rb(&gems)?;
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if setup.rb generation fails.
    pub fn generate_setup_rb(&self, gems: &[StandaloneGem]) -> Result<()> {
        use std::fmt::Write;

        let mut setup = String::new();

        // Header: Disable RubyGems and setup minimal Gem module
        setup.push_str(SETUP_HEADER);

        // Add load path entries for each gem
        for gem in gems {
            let gem_name = gem.full_name();

            // Add extension path if gem has extensions
            if gem.has_extensions {
                writeln!(
                    &mut setup,
                    "$:.unshift File.expand_path(\"#{{__dir__}}/../{}/{}/extensions/{}/{}/{}\")",
                    self.ruby_engine, self.ruby_version, self.platform, self.ruby_version, gem_name
                )
                .expect("writing to string should not fail");
            }

            // Add lib path for the gem
            writeln!(
                &mut setup,
                "$:.unshift File.expand_path(\"#{{__dir__}}/../{}/{}/gems/{}/lib\")",
                self.ruby_engine, self.ruby_version, gem_name
            )
            .expect("writing to string should not fail");
        }

        let setup_path = self.root.join("bundler").join("setup.rb");
        fs::write(&setup_path, setup)
            .with_context(|| format!("Failed to write setup.rb to {}", setup_path.display()))?;

        Ok(())
    }
}

/// Header template for bundle/bundler/setup.rb
///
/// This disables `RubyGems` and sets up a minimal Gem module for version detection.
const SETUP_HEADER: &str = r##"require 'rbconfig'
module Kernel
  remove_method(:gem) if private_method_defined?(:gem)

  def gem(*)
  end

  private :gem
end
unless defined?(Gem)
  module Gem
    def self.ruby_api_version
      RbConfig::CONFIG["ruby_version"]
    end

    def self.extension_api_version
      if 'no' == RbConfig::CONFIG['ENABLE_SHARED']
        "#{ruby_api_version}-static"
      else
        ruby_api_version
      end
    end
  end
end
if Gem.respond_to?(:discover_gems_on_require=)
  Gem.discover_gems_on_require = false
else
  [::Kernel.singleton_class, ::Kernel].each do |k|
    if k.private_method_defined?(:gem_original_require)
      private_require = k.private_method_defined?(:require)
      k.send(:remove_method, :require)
      k.send(:define_method, :require, k.instance_method(:gem_original_require))
      k.send(:private, :require) if private_require
    end
  end
end
"##;

/// Recursively copy a directory and all its contents
fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    if !src.exists() {
        anyhow::bail!("Source directory does not exist: {}", src.display());
    }

    fs::create_dir_all(dst)
        .with_context(|| format!("Failed to create destination directory: {}", dst.display()))?;

    for entry in fs::read_dir(src)
        .with_context(|| format!("Failed to read source directory: {}", src.display()))?
    {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let src_path = entry.path();
        let file_name = entry.file_name();
        let dst_path = dst.join(&file_name);

        if file_type.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else if file_type.is_file() {
            fs::copy(&src_path, &dst_path).with_context(|| {
                format!(
                    "Failed to copy {} to {}",
                    src_path.display(),
                    dst_path.display()
                )
            })?;
        }
        // Skip symlinks for now
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standalone_gem_full_name() {
        let gem = StandaloneGem {
            name: "rack".to_string(),
            version: "3.0.8".to_string(),
            platform: Some("ruby".to_string()),
            extracted_path: PathBuf::from("/tmp/rack"),
            extension_path: None,
            has_extensions: false,
        };
        assert_eq!(gem.full_name(), "rack-3.0.8");

        let platform_gem = StandaloneGem {
            name: "json".to_string(),
            version: "2.6.0".to_string(),
            platform: Some("x86_64-linux".to_string()),
            extracted_path: PathBuf::from("/tmp/json"),
            extension_path: None,
            has_extensions: true,
        };
        assert_eq!(platform_gem.full_name(), "json-2.6.0-x86_64-linux");
    }

    #[test]
    fn standalone_bundle_paths() {
        let options = StandaloneOptions {
            bundle_path: PathBuf::from("/tmp/test_bundle"),
            groups: vec![],
        };

        let bundle = StandaloneBundle::new(options, "3.3.0", "ruby").unwrap();

        assert_eq!(bundle.root, PathBuf::from("/tmp/test_bundle"));
        assert_eq!(
            bundle.ruby_path,
            PathBuf::from("/tmp/test_bundle/ruby/3.3.0")
        );
        assert_eq!(
            bundle.gems_path,
            PathBuf::from("/tmp/test_bundle/ruby/3.3.0/gems")
        );
    }

    #[test]
    fn setup_rb_generation() -> Result<()> {
        let temp_dir = tempfile::tempdir()?;
        let options = StandaloneOptions {
            bundle_path: temp_dir.path().to_path_buf(),
            groups: vec![],
        };

        let bundle = StandaloneBundle::new(options, "3.3.0", "ruby")?;
        bundle.create_directories()?;

        let gems = vec![
            StandaloneGem {
                name: "rack".to_string(),
                version: "3.0.8".to_string(),
                platform: Some("ruby".to_string()),
                extracted_path: PathBuf::from("/tmp/rack"),
                extension_path: None,
                has_extensions: false,
            },
            StandaloneGem {
                name: "json".to_string(),
                version: "2.6.0".to_string(),
                platform: Some("ruby".to_string()),
                extracted_path: PathBuf::from("/tmp/json"),
                extension_path: Some(PathBuf::from("/tmp/json_ext")),
                has_extensions: true,
            },
        ];

        bundle.generate_setup_rb(&gems)?;

        let setup_path = temp_dir.path().join("bundler/setup.rb");
        assert!(setup_path.exists());

        let content = fs::read_to_string(&setup_path)?;
        assert!(content.contains("require 'rbconfig'"));
        assert!(content.contains("rack-3.0.8/lib"));
        assert!(content.contains("json-2.6.0/lib"));
        assert!(content.contains("json-2.6.0")); // Extension path for json

        Ok(())
    }
}
