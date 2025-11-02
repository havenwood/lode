//! Help command
//!
//! Show help for gem commands

use anyhow::Result;

/// Show help for gem commands
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn run(command: Option<&str>) -> Result<()> {
    if let Some(cmd) = command {
        show_command_help(cmd);
    } else {
        show_all_commands();
    }

    Ok(())
}

/// Show help for all gem commands
fn show_all_commands() {
    println!("RubyGems commands available via lode:\n");

    let commands = vec![
        ("gem-build", "Build a gem from a gemspec"),
        ("gem-cert", "Manage gem certificates"),
        ("gem-cleanup", "Clean up old gem versions"),
        ("gem-contents", "List files in an installed gem"),
        ("gem-dependency", "Show gem dependencies"),
        ("gem-fetch", "Download a gem without installing it"),
        ("gem-help", "Show help for gem commands"),
        ("gem-info", "Show gem information"),
        ("gem-install", "Install a gem"),
        ("gem-list", "List installed gems"),
        ("gem-owner", "Manage gem owners"),
        ("gem-pristine", "Restore gems to pristine condition"),
        ("gem-push", "Push a gem to RubyGems.org"),
        ("gem-rdoc", "Generate RDoc for a gem"),
        ("gem-rebuild", "Rebuild native extensions"),
        ("gem-search", "Search for gems on RubyGems.org"),
        ("gem-signin", "Sign in to RubyGems.org"),
        ("gem-signout", "Sign out from RubyGems.org"),
        ("gem-sources", "Manage gem sources"),
        ("gem-stale", "List stale gems"),
        ("gem-uninstall", "Uninstall a gem"),
        ("gem-update", "Update installed gems"),
        ("gem-which", "Find the installation path of a gem"),
        ("gem-yank", "Yank a gem version from RubyGems.org"),
    ];

    for (name, description) in commands {
        println!("  {name:<20} {description}");
    }

    println!("\nFor command-specific help, use:");
    println!("  lode gem-help COMMAND");
    println!("  lode COMMAND --help");
}

/// Show help for a specific command
fn show_command_help(command: &str) {
    let help_text = match command {
        "build" | "gem-build" => {
            "gem-build [GEMSPEC]\n\n\
            Build a gem from a gemspec file.\n\n\
            Options:\n  \
              [GEMSPEC]  Gemspec file to build (looks for *.gemspec if not specified)"
        }
        "cert" | "gem-cert" => {
            "gem-cert\n\n\
            Manage gem certificates for signing and verification."
        }
        "cleanup" | "gem-cleanup" => {
            "gem-cleanup\n\n\
            Remove old versions of installed gems, keeping only the latest version."
        }
        "contents" | "gem-contents" => {
            "gem-contents <GEM> [OPTIONS]\n\n\
            List all files in an installed gem.\n\n\
            Options:\n  \
              --version VERSION  Specific version (uses latest if not specified)\n  \
              --prefix           Show full file paths"
        }
        "dependency" | "gem-dependency" => {
            "gem-dependency <GEM>\n\n\
            Show gem dependencies."
        }
        "fetch" | "gem-fetch" => {
            "gem-fetch <GEM> [OPTIONS]\n\n\
            Download a gem without installing it.\n\n\
            Options:\n  \
              --version VERSION        Gem version to fetch\n  \
              --output-dir DIRECTORY   Download to specific directory"
        }
        "help" | "gem-help" => {
            "gem-help [COMMAND]\n\n\
            Show help for gem commands.\n\n\
            Usage:\n  \
              gem-help           Show all commands\n  \
              gem-help COMMAND   Show help for specific command"
        }
        "info" | "gem-info" => {
            "gem-info <GEM> [OPTIONS]\n\n\
            Show gem information from RubyGems.org."
        }
        "install" | "gem-install" => {
            "gem-install <GEM> [OPTIONS]\n\n\
            Install a gem from RubyGems.org.\n\n\
            Options:\n  \
              --version VERSION  Specific version to install"
        }
        "list" | "gem-list" => {
            "gem-list [PATTERN]\n\n\
            List installed gems, optionally filtered by pattern."
        }
        "uninstall" | "gem-uninstall" => {
            "gem-uninstall <GEM>\n\n\
            Uninstall a gem."
        }
        "update" | "gem-update" => {
            "gem-update\n\n\
            Update all installed gems to their latest versions."
        }
        "which" | "gem-which" => {
            "gem-which <GEM>\n\n\
            Find the installation path of a gem."
        }
        _ => {
            eprintln!("Unknown command: {command}");
            eprintln!("Use 'lode gem-help' to see all available commands.");
            return;
        }
    };

    println!("{help_text}");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_run_without_command() {
        // Should not panic
        assert!(run(None).is_ok());
    }

    #[test]
    fn test_run_with_known_command() {
        // Should not panic for known commands
        assert!(run(Some("build")).is_ok());
        assert!(run(Some("gem-build")).is_ok());
        assert!(run(Some("list")).is_ok());
        assert!(run(Some("install")).is_ok());
    }

    #[test]
    fn test_run_with_unknown_command() {
        // Should not panic even for unknown commands
        assert!(run(Some("nonexistent")).is_ok());
    }
}
