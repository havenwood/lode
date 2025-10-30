//! Completion command
//!
//! Generate shell completion scripts

use anyhow::Result;
use clap::CommandFactory;
use clap_complete::{Shell, generate};
use std::io;

/// Generate shell completion scripts
///
/// Outputs completion script for the specified shell to stdout.
/// Users can save this to their shell's completion directory.
///
/// # Examples
///
/// ```bash
/// # Bash
/// lode completion bash > /usr/local/share/bash-completion/completions/lode
///
/// # Zsh
/// lode completion zsh > /usr/local/share/zsh/site-functions/_lode
///
/// # Fish
/// lode completion fish > ~/.config/fish/completions/lode.fish
/// ```
#[allow(
    clippy::unnecessary_wraps,
    reason = "Result type maintained for consistency with command signature pattern"
)]
pub(crate) fn run(shell: Shell) -> Result<()> {
    // We need to get the Cli command definition from main.rs
    // This requires that Cli implements CommandFactory from clap's derive
    let mut cmd = crate::Cli::command();

    generate(shell, &mut cmd, "lode", &mut io::stdout());

    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "Tests can panic")]
mod tests {
    use super::*;

    // Note: These tests require large stack due to 55+ command CLI structure
    // Run with: RUST_MIN_STACK=4194304 cargo test -- --ignored
    // Or: RUST_MIN_STACK=4194304 cargo nextest run --run-ignored ignored-only

    #[test]
    #[ignore = "requires large stack (run with RUST_MIN_STACK=4194304)"]
    fn completion_bash() {
        // Just verify it doesn't panic
        let result = run(Shell::Bash);
        assert!(result.is_ok());
    }

    #[test]
    #[ignore = "requires large stack (run with RUST_MIN_STACK=4194304)"]
    fn completion_zsh() {
        let result = run(Shell::Zsh);
        assert!(result.is_ok());
    }

    #[test]
    #[ignore = "requires large stack (run with RUST_MIN_STACK=4194304)"]
    fn completion_fish() {
        let result = run(Shell::Fish);
        assert!(result.is_ok());
    }

    #[test]
    #[ignore = "requires large stack (run with RUST_MIN_STACK=4194304)"]
    fn completion_powershell() {
        let result = run(Shell::PowerShell);
        assert!(result.is_ok());
    }
}
