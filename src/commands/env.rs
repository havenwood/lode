//! Env Command
//!
//! Displays environment information useful for debugging gem issues.
//! Similar to `bundle env`, shows Ruby version, `RubyGems` version,
//! Bundler version, platform, and environment variables.

use std::env;
use std::process::Command;

/// Display environment information
pub(crate) fn run() {
    println!("## Environment");
    println!();

    // Lode version
    println!("Lode       {}", env!("CARGO_PKG_VERSION"));
    println!();

    // Ruby version
    if let Ok(output) = Command::new("ruby").arg("--version").output() {
        if output.status.success()
            && let Ok(version) = String::from_utf8(output.stdout)
        {
            println!("Ruby       {}", version.trim());
        }
    } else {
        println!("Ruby       not found");
    }
    println!();

    // RubyGems version
    if let Ok(output) = Command::new("gem").arg("--version").output() {
        if output.status.success()
            && let Ok(version) = String::from_utf8(output.stdout)
        {
            println!("RubyGems   {}", version.trim());
        }
    } else {
        println!("RubyGems   not found");
    }
    println!();

    // Bundler version (if available)
    if let Ok(output) = Command::new("bundle").arg("--version").output()
        && output.status.success()
        && let Ok(version) = String::from_utf8(output.stdout)
    {
        println!("{}", version.trim());
    }
    println!();

    // Platform
    println!("## Platform");
    println!();
    println!("OS         {}", env::consts::OS);
    println!("Arch       {}", env::consts::ARCH);
    println!("Family     {}", env::consts::FAMILY);
    println!();

    // Relevant environment variables
    println!("## Environment Variables");
    println!();

    let env_vars = [
        "GEM_HOME",
        "GEM_PATH",
        "BUNDLE_PATH",
        "BUNDLE_GEMFILE",
        "BUNDLE_APP_CONFIG",
        "RUBY_VERSION",
        "RUBYGEMS_GEMDEPS",
        "PATH",
    ];

    for var in &env_vars {
        if let Ok(value) = env::var(var) {
            println!("{var:<20} {value}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn env_run() {
        // Just verify it doesn't crash
        run();
    }
}
