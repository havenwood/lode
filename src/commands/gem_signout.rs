//! Signout command
//!
//! Sign out from RubyGems.org and remove credentials

use anyhow::{Context, Result};
use std::env;
use std::fs;
use std::path::PathBuf;

/// Options for gem-signout command
#[derive(Debug, Copy, Clone)]
pub(crate) struct SignoutOptions {
    pub verbose: bool,
    pub quiet: bool,
    pub silent: bool,
}

/// Sign out from all `RubyGems` sessions
pub(crate) fn run_with_options(options: SignoutOptions) -> Result<()> {
    let credentials_path = get_credentials_path()?;

    if credentials_path.exists() {
        if options.verbose && !options.silent && !options.quiet {
            println!(
                "Removing credentials file: {path}",
                path = credentials_path.display()
            );
        }

        fs::remove_file(&credentials_path).with_context(|| {
            format!(
                "Failed to remove credentials file: {path}",
                path = credentials_path.display()
            )
        })?;

        if !options.silent && !options.quiet {
            println!("You have successfully signed out from all sessions.");
        } else if options.verbose && !options.silent {
            println!("Credentials file removed successfully.");
        }
    } else if !options.silent && !options.quiet {
        println!("You are not currently signed in.");
    } else if options.verbose && !options.silent {
        println!(
            "Credentials file not found at: {path}",
            path = credentials_path.display()
        );
    }

    Ok(())
}

/// Get the path to the `RubyGems` credentials file
fn get_credentials_path() -> Result<PathBuf> {
    let home = env::var("HOME")
        .or_else(|_| env::var("USERPROFILE"))
        .context("Could not determine home directory")?;

    Ok(PathBuf::from(home).join(".gem").join("credentials"))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "Tests can panic")]
mod tests {
    use super::*;

    #[test]
    fn test_get_credentials_path() {
        let path = get_credentials_path();
        assert!(path.is_ok());

        let path = path.unwrap();
        assert!(path.to_string_lossy().contains(".gem"));
        assert!(path.to_string_lossy().ends_with("credentials"));
    }

    #[test]
    fn test_signout_options_default() {
        let options = SignoutOptions {
            verbose: false,
            quiet: false,
            silent: false,
        };
        assert!(!options.verbose);
        assert!(!options.quiet);
        assert!(!options.silent);
    }

    #[test]
    fn test_signout_options_verbose() {
        let options = SignoutOptions {
            verbose: true,
            quiet: false,
            silent: false,
        };
        assert!(options.verbose);
        assert!(!options.quiet);
        assert!(!options.silent);
    }

    #[test]
    fn test_signout_options_quiet() {
        let options = SignoutOptions {
            verbose: false,
            quiet: true,
            silent: false,
        };
        assert!(!options.verbose);
        assert!(options.quiet);
        assert!(!options.silent);
    }

    #[test]
    fn test_signout_options_silent() {
        let options = SignoutOptions {
            verbose: false,
            quiet: false,
            silent: true,
        };
        assert!(!options.verbose);
        assert!(!options.quiet);
        assert!(options.silent);
    }
}
