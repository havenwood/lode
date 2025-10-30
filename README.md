# Lode

> **A Ruby package manager written in Rust.** All Bundler commands, all RubyGems commands, one fast tool.

[![Bundler](https://img.shields.io/badge/bundler-30%20commands-purple)](https://github.com/havenwood/lode)
[![RubyGems](https://img.shields.io/badge/rubygems-27%20commands-orange)](https://github.com/havenwood/lode)

Lode targets Bundler 4 & RubyGems 4 APIs.

## Why Lode?

Implementing `bundle` and `gem` commands in a single binary seemed like a nice step towards exploring unification.

Ruby developers typically juggle two separate tools:
- `bundle` for managing project dependencies
- `gem` for installing system gems

System gem commands use a `gem-` prefix for now to avoid conflicts.

```bash
# `bundle` commands work as-is
lode install
lode update rails --patch
lode lock

# `gem` commands get a `gem-` prefix
lode gem-search falcon
lode gem-list
lode gem-install falcon
```

## Commands

Lode implements all Bundler 4 & RubyGems 4 commands with full option flag parity (400+ flags).

```bash
$ lode --help
A Ruby package manager

Usage: lode <COMMAND>

Commands:
  install         Install gems from Gemfile.lock
  update          Update gems to their latest versions within constraints
  cache           Inspect or prune the lode gem cache
  exec            Run commands with lode-managed environment
  config          Get and set Bundler configuration options
  add             Add gems to Gemfile
  binstubs        Generate binstubs for gem executables
  check           Verify all gems are installed
  show            Show the source location of a gem
  outdated        List gems with newer versions available
  open            Open a gem's source code in your editor
  lock            Regenerate Gemfile.lock from Gemfile
  init            Create a new Gemfile
  gem             Generate a new gem project skeleton
  platform        Display platform compatibility information
  plugin          Manage Bundler plugins
  clean           Remove unused gems from vendor directory
  doctor          Diagnose common Bundler problems
  remove          Remove gems from Gemfile
  list            List all gems in the current bundle
  info            Show detailed information about a gem
  search          Search for gems on RubyGems.org
  specification   Display full gemspec metadata
  which           Find the location of a required library file
  contents        List all files in an installed gem
  unpack          Extract gem source to current directory
  env             Show environment information
  pristine        Restore gems to pristine condition
  completion      Generate shell completion scripts
  gem-install     Install a gem
  gem-uninstall   Uninstall a gem
  gem-update      Update installed gems
  gem-list        List installed gems
  gem-search      Search for gems on RubyGems.org
  gem-build       Build a gem from a gemspec
  gem-push        Push a gem to `RubyGems`
  gem-yank        Yank a gem version from `RubyGems`
  gem-owner       Manage gem ownership
  gem-signin      Sign in to `RubyGems`
  gem-signout     Sign out from `RubyGems`
  gem-info        Show gem information
  gem-contents    List files in an installed gem
  gem-dependency  Show gem dependencies
  gem-which       Find the installation path of a gem
  gem-fetch       Download a gem without installing it
  gem-stale       List stale gems
  gem-cleanup     Clean up gem cache
  gem-pristine    Restore original files in gem install
  gem-rebuild     Rebuild installed gems
  gem-sources     Manage gem sources
  gem-cert        Manage gem certificates
  gem-rdoc        Build `RDoc` for installed gems
  gem-server      Serve gems from a gem server
  gem-mirror      Mirror gem repositories
  gem-environment Display `RubyGems` environment information
  gem-help        Show help for gem commands
  help            Print this message or the help of the given subcommand(s)

Options:
  -v, --version  Print version
  -h, --help     Print help
```

## Installation

Manually clone and `cargo build --release` for now, then put the binary somewhere in your `$PATH`.
Binary releases coming soon.

```bash
git clone https://github.com/havenwood/lode.git
cd lode
cargo build --release
./target/release/lode --version
#>> lode 0.1.0
```

### Deprecated RubyGems Commands

Some RubyGems commands have been deprecated/moved in modern versions:

- **`gem server`** - Moved to separate `rubygems-server` gem. Lode implements full HTTP server with Marshal API support.
- **`gem mirror`** - Moved to separate `rubygems-mirror` gem. Lode implements basic mirror management.

We might consider removing deprecated commands or separating them out to another tool like RubyGems has.

## Environment Variables

Lode supports Bundler & RubyGems environment variables:

**Core Bundler:**
- `BUNDLE_GEMFILE`, `BUNDLE_PATH`, `BUNDLE_WITHOUT`, `BUNDLE_WITH`, `BUNDLE_JOBS`, `BUNDLE_FROZEN`, `BUNDLE_DEPLOYMENT`, `BUNDLE_RETRY`, `BUNDLE_APP_CONFIG`, `BUNDLE_USER_HOME`

**Network & Enterprise:**
- `HTTP_PROXY`, `HTTPS_PROXY`, `NO_PROXY`, `BUNDLE_TIMEOUT`, `RUBYGEMS_HOST`, `BUNDLE_SSL_CA_CERT`, `BUNDLE_SSL_CLIENT_CERT`, `BUNDLE_SSL_VERIFY_MODE`

**Build Tools:**
- `CC`, `CXX`, `CFLAGS`, `CXXFLAGS`, `LDFLAGS`, `MAKE` - Cross-compilation and custom toolchain

**RubyGems & Auth:**
- `HTTP_PROXY_USER`, `HTTP_PROXY_PASS`, `HTTPS_PROXY_USER`, `HTTPS_PROXY_PASS`, `RUBYGEMS_API_KEY`, `GEM_HOST_API_KEY`, `GEM_SOURCE`, `GEM_SKIP`, and more

**Cache, Security, Behavior & Advanced:**
- `BUNDLE_CACHE_ALL`, `BUNDLE_CACHE_PATH`, `BUNDLE_DISABLE_CHECKSUM_VALIDATION`, `BUNDLE_PREFER_PATCH`, `BUNDLE_REDIRECT`, `BUNDLE_IGNORE_CONFIG`, `BUNDLE_AUTO_INSTALL`, `BUNDLE_ALLOW_OFFLINE_INSTALL`, `BUNDLE_USER_CACHE`, `BUNDLE_BIN`, `BUNDLE_ONLY`, `BUNDLE_VERBOSE`, `BUNDLE_FORCE`, `BUNDLE_LOCAL`, `BUNDLE_PREFER_LOCAL`, and more

All environment variables follow standard priority: CLI args > env vars > config file > defaults

### Shell Completion
```bash
# Generate completion script for your shell
lode completion bash > ~/.local/share/bash-completion/completions/lode

# Zsh
lode completion zsh > ~/.zsh/completions/_lode
# Add to ~/.zshrc: fpath=(~/.zsh/completions $fpath)

# Fish
lode completion fish > ~/.config/fish/completions/lode.fish

# PowerShell
lode completion powershell > lode.ps1
```

Tab completion works for all commands, subcommands and flags.
