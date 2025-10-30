//! Gem Command - Generate a new gem project skeleton
//!
//! Creates a directory with a gemspec, README, and basic project structure
//! for developing a new `RubyGem`.

use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

/// Run the gem command to create a new gem project.
pub(crate) fn run(
    gem_name_or_path: &str,
    exe: bool,
    _mit: bool,
    no_mit: bool,
    test_framework: Option<&str>,
) -> Result<()> {
    // Extract gem name from path if an absolute/relative path was provided
    let gem_dir = Path::new(gem_name_or_path);
    let gem_name = gem_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(gem_name_or_path);

    if !is_valid_gem_name(gem_name) {
        anyhow::bail!(
            "Invalid gem name '{gem_name}'. Gem names must contain only lowercase letters, numbers, underscores, and hyphens"
        );
    }

    if gem_dir.exists() {
        anyhow::bail!("Directory '{gem_name}' already exists");
    }

    println!("Creating gem '{gem_name}'...");

    fs::create_dir(gem_dir).context("Failed to create gem directory")?;
    fs::create_dir_all(gem_dir.join("lib").join(gem_name))
        .context("Failed to create lib directory")?;

    if exe {
        fs::create_dir_all(gem_dir.join("exe")).context("Failed to create exe directory")?;
    }

    let module_name = to_module_name(gem_name);
    let author =
        get_git_config("user.name").unwrap_or_else(|| String::from("TODO: Write your name"));
    let email =
        get_git_config("user.email").unwrap_or_else(|| String::from("TODO: Write your email"));

    // Determine whether to include license (default: true, unless --no-mit)
    let include_license = !no_mit;

    create_gemspec(
        gem_dir,
        gem_name,
        &module_name,
        &author,
        &email,
        exe,
        include_license,
    )?;

    create_lib_file(gem_dir, gem_name, &module_name)?;
    create_version_file(gem_dir, gem_name, &module_name)?;
    create_readme(gem_dir, gem_name)?;
    create_gemfile(gem_dir, gem_name)?;
    create_rakefile(gem_dir, test_framework)?;

    if let Some(framework) = test_framework {
        create_test_files(gem_dir, gem_name, &module_name, framework)?;
    }

    if include_license {
        create_license(gem_dir, &author)?;
    }

    create_gitignore(gem_dir)?;

    if exe {
        create_executable(gem_dir, gem_name)?;
    }

    if let Err(e) = std::process::Command::new("git")
        .args(["init", gem_dir.to_str().unwrap_or(gem_name)])
        .output()
    {
        eprintln!("Warning: Failed to initialize git repository: {e}");
    }

    println!("      create  {gem_name}/Gemfile");
    println!("      create  {gem_name}/{gem_name}.gemspec");
    println!("      create  {gem_name}/Rakefile");
    println!("      create  {gem_name}/README.md");
    if include_license {
        println!("      create  {gem_name}/LICENSE.txt");
    }
    println!("      create  {gem_name}/.gitignore");
    println!("      create  {gem_name}/lib/{gem_name}.rb");
    println!("      create  {gem_name}/lib/{gem_name}/version.rb");
    if exe {
        println!("      create  {gem_name}/exe/{gem_name}");
    }
    if let Some(framework) = test_framework {
        match framework {
            "rspec" => {
                println!("      create  {gem_name}/.rspec");
                println!("      create  {gem_name}/spec/spec_helper.rb");
                println!("      create  {gem_name}/spec/{gem_name}_spec.rb");
            }
            "minitest" => {
                println!("      create  {gem_name}/test/test_helper.rb");
                println!("      create  {gem_name}/test/{gem_name}_test.rb");
            }
            "test-unit" => {
                println!("      create  {gem_name}/test/test_helper.rb");
                println!("      create  {gem_name}/test/test_{gem_name}.rb");
            }
            _ => {}
        }
    }

    println!();
    println!("Initialized empty Git repository in {gem_name}/.git/");
    println!();
    println!("Gem '{gem_name}' was successfully created.");
    println!(
        "For more information on making a RubyGem visit https://guides.rubygems.org/make-your-own-gem"
    );

    Ok(())
}

fn is_valid_gem_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
}

fn to_module_name(gem_name: &str) -> String {
    gem_name
        .split(['-', '_'])
        .map(|part| {
            let mut chars = part.chars();
            chars.next().map_or_else(String::new, |first| {
                first.to_uppercase().chain(chars).collect()
            })
        })
        .collect()
}

fn get_git_config(key: &str) -> Option<String> {
    std::process::Command::new("git")
        .args(["config", "--get", key])
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                String::from_utf8(output.stdout)
                    .ok()
                    .map(|s| s.trim().to_string())
            } else {
                None
            }
        })
}

fn create_gemspec(
    gem_dir: &Path,
    gem_name: &str,
    module_name: &str,
    author: &str,
    email: &str,
    exe: bool,
    include_license: bool,
) -> Result<()> {
    let exe_line = if exe {
        format!("  spec.executables   = [\"{gem_name}\"]\n")
    } else {
        String::new()
    };

    let license_line = if include_license {
        "  spec.license = \"MIT\"\n"
    } else {
        ""
    };

    let content = format!(
        r#"# frozen_string_literal: true

require_relative "lib/{gem_name}/version"

Gem::Specification.new do |spec|
  spec.name = "{gem_name}"
  spec.version = {module_name}::VERSION
  spec.authors = ["{author}"]
  spec.email = ["{email}"]

  spec.summary = "TODO: Write a short summary, because RubyGems requires one."
  spec.description = "TODO: Write a longer description or delete this line."
  spec.homepage = "https://github.com/yourusername/{gem_name}"
{license_line}  spec.required_ruby_version = ">= 3.0.0"

  spec.metadata["homepage_uri"] = spec.homepage
  spec.metadata["source_code_uri"] = "https://github.com/yourusername/{gem_name}"
  spec.metadata["changelog_uri"] = "https://github.com/yourusername/{gem_name}/blob/main/CHANGELOG.md"

  # Specify which files should be added to the gem when it is released.
  spec.files = Dir.chdir(__dir__) do
    `git ls-files -z`.split("\x0").reject do |f|
      (File.expand_path(f) == __FILE__) ||
        f.start_with?(*%w[test/ spec/ features/ .git .github appveyor Gemfile])
    end
  end
  spec.bindir = "exe"
{exe_line}  spec.require_paths = ["lib"]

  # Uncomment to register a new dependency of your gem
  # spec.add_dependency "example-gem", "~> 1.0"

  # For more information and examples about making a new gem, check out our
  # guide at: https://bundler.io/guides/creating_gem.html
end
"#
    );

    fs::write(gem_dir.join(format!("{gem_name}.gemspec")), content)
        .context("Failed to create gemspec")
}

fn create_lib_file(gem_dir: &Path, gem_name: &str, module_name: &str) -> Result<()> {
    let content = format!(
        r#"# frozen_string_literal: true

require_relative "{gem_name}/version"

module {module_name}
  class Error < StandardError; end
  # Your code goes here...
end
"#
    );

    fs::write(gem_dir.join("lib").join(format!("{gem_name}.rb")), content)
        .context("Failed to create lib file")
}

fn create_version_file(gem_dir: &Path, gem_name: &str, module_name: &str) -> Result<()> {
    let content = format!(
        r#"# frozen_string_literal: true

module {module_name}
  VERSION = "0.1.0"
end
"#
    );

    fs::write(
        gem_dir.join("lib").join(gem_name).join("version.rb"),
        content,
    )
    .context("Failed to create version file")
}

fn create_readme(gem_dir: &Path, gem_name: &str) -> Result<()> {
    let module_name = to_module_name(gem_name);
    let content = format!(
        "# {module_name}

TODO: Delete this and the text below, and describe your gem

Welcome to your new gem! In this directory, you'll find the files you need to be able to package up your Ruby library into a gem. Put your Ruby code in the file `lib/{gem_name}`. To experiment with that code, run `bin/console` for an interactive prompt.

## Installation

TODO: Replace `UPDATE_WITH_YOUR_GEM_NAME_IMMEDIATELY_AFTER_RELEASE_TO_RUBYGEMS_ORG` with your gem name right after releasing it to RubyGems.org. Please do not do it earlier due to security reasons. Alternatively, replace this section with instructions to install your gem from git if you don't plan to release to RubyGems.org.

Install the gem and add to the application's Gemfile by executing:

    $ bundle add UPDATE_WITH_YOUR_GEM_NAME_IMMEDIATELY_AFTER_RELEASE_TO_RUBYGEMS_ORG

If bundler is not being used to manage dependencies, install the gem by executing:

    $ gem install UPDATE_WITH_YOUR_GEM_NAME_IMMEDIATELY_AFTER_RELEASE_TO_RUBYGEMS_ORG

## Usage

TODO: Write usage instructions here

## Development

After checking out the repo, run `bin/setup` to install dependencies. You can also run `bin/console` for an interactive prompt that will allow you to experiment.

To install this gem onto your local machine, run `bundle exec rake install`. To release a new version, update the version number in `version.rb`, and then run `bundle exec rake release`, which will create a git tag for the version, push git commits and the created tag, and push the `.gem` file to [rubygems.org](https://rubygems.org).

## Contributing

Bug reports and pull requests are welcome on GitHub at https://github.com/yourusername/{gem_name}.

## License

The gem is available as open source under the terms of the [MIT License](https://opensource.org/licenses/MIT).
"
    );

    fs::write(gem_dir.join("README.md"), content).context("Failed to create README")
}

fn create_gemfile(gem_dir: &Path, gem_name: &str) -> Result<()> {
    let content = format!(
        r#"# frozen_string_literal: true

source "{source}"

# Specify your gem's dependencies in {gem_name}.gemspec
gemspec

gem "rake", "~> 13.0"
"#,
        source = lode::DEFAULT_GEM_SOURCE
    );

    fs::write(gem_dir.join("Gemfile"), content).context("Failed to create Gemfile")
}

fn create_rakefile(gem_dir: &Path, test_framework: Option<&str>) -> Result<()> {
    let test_task = match test_framework {
        Some("rspec") => {
            r#"
require "rspec/core/rake_task"

RSpec::Core::RakeTask.new(:spec)

task default: %i[spec]
"#
        }
        Some("minitest") => {
            r#"
require "rake/testtask"

Rake::TestTask.new(:test) do |t|
  t.libs << "test"
  t.libs << "lib"
  t.test_files = FileList["test/**/*_test.rb"]
end

task default: %i[test]
"#
        }
        Some("test-unit") => {
            r#"
require "rake/testtask"

Rake::TestTask.new(:test) do |t|
  t.libs << "test"
  t.libs << "lib"
  t.test_files = FileList["test/**/test_*.rb"]
end

task default: %i[test]
"#
        }
        _ => "task default: %i[]\n",
    };

    let content = format!(
        r#"# frozen_string_literal: true

require "bundler/gem_tasks"
{test_task}"#
    );

    fs::write(gem_dir.join("Rakefile"), content).context("Failed to create Rakefile")
}

fn create_license(gem_dir: &Path, author: &str) -> Result<()> {
    let year = chrono::Local::now().format("%Y");
    let content = format!(
        r#"The MIT License (MIT)

Copyright (c) {year} {author}

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in
all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
THE SOFTWARE.
"#
    );

    fs::write(gem_dir.join("LICENSE.txt"), content).context("Failed to create LICENSE")
}

fn create_gitignore(gem_dir: &Path) -> Result<()> {
    let content = "/.bundle/
/.yardoc
/_yardoc/
/coverage/
/doc/
/pkg/
/spec/reports/
/tmp/

# rspec failure tracking
.rspec_status

# Environment normalization:
/.bundle/
/vendor/bundle
/lib/bundler/man/

# Used by dotenv library to load environment variables.
.env

# Ignore Byebug command history file.
.byebug_history

## Specific to RubyMotion:
.dat*
.repl_history
build/
*.bridgesupport
build-iPhoneOS/
build-iPhoneSimulator/

## Documentation cache and generated files:
/.yardoc/
/_yardoc/
/doc/
/rdoc/

## Environment normalization:
/.bundle/
/vendor/bundle
/lib/bundler/man/

## Test coverage output
/coverage/

## Gem output
*.gem

## Ignore IDE files
.idea/
.vscode/
";

    fs::write(gem_dir.join(".gitignore"), content).context("Failed to create .gitignore")
}

fn create_test_files(
    gem_dir: &Path,
    gem_name: &str,
    module_name: &str,
    framework: &str,
) -> Result<()> {
    match framework {
        "rspec" => {
            fs::create_dir_all(gem_dir.join("spec"))?;

            let spec_helper = format!(
                r#"# frozen_string_literal: true

require "{gem_name}"

RSpec.configure do |config|
  # Enable flags like --only-failures and --next-failure
  config.example_status_persistence_file_path = ".rspec_status"

  # Disable RSpec exposing methods globally on `Module` and `main`
  config.disable_monkey_patching!

  config.expect_with :rspec do |c|
    c.syntax = :expect
  end
end
"#
            );
            fs::write(gem_dir.join("spec/spec_helper.rb"), spec_helper)
                .context("Failed to create spec_helper.rb")?;

            let example_spec = format!(
                r#"# frozen_string_literal: true

RSpec.describe {module_name} do
  it "has a version number" do
    expect({module_name}::VERSION).not_to be_nil
  end

  it "does something useful" do
    expect(false).to eq(true)
  end
end
"#
            );
            fs::write(
                gem_dir.join(format!("spec/{gem_name}_spec.rb")),
                example_spec,
            )
            .context("Failed to create spec file")?;

            let rspec_config = "--require spec_helper\n--color\n--format documentation\n";
            fs::write(gem_dir.join(".rspec"), rspec_config).context("Failed to create .rspec")?;
        }
        "minitest" | "test-unit" => {
            fs::create_dir_all(gem_dir.join("test"))?;

            let test_helper = format!(
                r#"# frozen_string_literal: true

$LOAD_PATH.unshift File.expand_path("../lib", __dir__)
require "{gem_name}"

require "minitest/autorun"
"#
            );
            fs::write(gem_dir.join("test/test_helper.rb"), test_helper)
                .context("Failed to create test_helper.rb")?;

            let file_name = if framework == "test-unit" {
                format!("test/test_{gem_name}.rb")
            } else {
                format!("test/{gem_name}_test.rb")
            };

            let example_test = format!(
                r#"# frozen_string_literal: true

require "test_helper"

class Test{module_name} < Minitest::Test
  def test_that_it_has_a_version_number
    refute_nil {module_name}::VERSION
  end

  def test_it_does_something_useful
    assert false
  end
end
"#
            );
            fs::write(gem_dir.join(file_name), example_test)
                .context("Failed to create test file")?;
        }
        _ => {
            anyhow::bail!(
                "Unsupported test framework: {framework}. Supported: rspec, minitest, test-unit"
            );
        }
    }

    Ok(())
}

fn create_executable(gem_dir: &Path, gem_name: &str) -> Result<()> {
    let module_name = to_module_name(gem_name);
    let content = format!(
        r#"#!/usr/bin/env ruby
# frozen_string_literal: true

require "{gem_name}"

# Your CLI code goes here...
puts "{module_name}::VERSION"
"#
    );

    let exe_path = gem_dir.join("exe").join(gem_name);
    fs::write(&exe_path, content).context("Failed to create executable")?;

    // Make executable on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&exe_path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&exe_path, perms)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_is_valid_gem_name() {
        assert!(is_valid_gem_name("my_gem"));
        assert!(is_valid_gem_name("my-gem"));
        assert!(is_valid_gem_name("mygem123"));
        assert!(!is_valid_gem_name("MyGem"));
        assert!(!is_valid_gem_name("my gem"));
        assert!(!is_valid_gem_name(""));
    }

    #[test]
    fn test_to_module_name() {
        assert_eq!(to_module_name("my_gem"), "MyGem");
        assert_eq!(to_module_name("my-gem"), "MyGem");
        assert_eq!(to_module_name("active_record"), "ActiveRecord");
    }

    #[test]
    fn create_gem_basic() {
        let temp = TempDir::new().unwrap();
        let gem_path = temp.path().join("test_gem_basic");

        let result = run(gem_path.to_str().unwrap(), false, false, false, None);

        assert!(result.is_ok(), "Error: {:?}", result.err());

        // Verify files were created using absolute paths
        assert!(gem_path.join("test_gem_basic.gemspec").exists());
        assert!(gem_path.join("lib/test_gem_basic.rb").exists());
        assert!(gem_path.join("lib/test_gem_basic/version.rb").exists());
        assert!(gem_path.join("README.md").exists());
        assert!(gem_path.join("Gemfile").exists());
        assert!(gem_path.join("Rakefile").exists());
        assert!(gem_path.join("LICENSE.txt").exists());
        assert!(gem_path.join(".gitignore").exists());
    }

    #[test]
    fn create_gem_with_exe() {
        let temp = TempDir::new().unwrap();
        let gem_path = temp.path().join("test_gem_exe");

        let result = run(gem_path.to_str().unwrap(), true, false, false, None);

        assert!(result.is_ok(), "Error: {:?}", result.err());

        // Verify executable was created using absolute path
        assert!(gem_path.join("exe/test_gem_exe").exists());
    }

    #[test]
    fn create_gem_existing_directory() {
        let temp = TempDir::new().unwrap();
        let gem_path = temp.path().join("test_gem_exists");

        fs::create_dir(&gem_path).unwrap();
        let result = run(gem_path.to_str().unwrap(), false, false, false, None);
        assert!(result.is_err());
    }

    #[test]
    fn create_gem_invalid_name() {
        let temp = TempDir::new().unwrap();
        let gem_path = temp.path().join("Test Gem");

        let result = run(gem_path.to_str().unwrap(), false, false, false, None);
        assert!(result.is_err());
    }

    #[test]
    fn create_gem_without_license() {
        let temp = TempDir::new().unwrap();
        let gem_path = temp.path().join("test_gem_no_license");

        let result = run(gem_path.to_str().unwrap(), false, false, true, None);

        assert!(result.is_ok(), "Error: {:?}", result.err());

        // Verify license file was NOT created
        assert!(!gem_path.join("LICENSE.txt").exists());

        // Verify gemspec does not contain license field
        let gemspec_content = fs::read_to_string(gem_path.join("test_gem_no_license.gemspec"))
            .expect("should read gemspec");
        assert!(!gemspec_content.contains("spec.license"));
    }
}
