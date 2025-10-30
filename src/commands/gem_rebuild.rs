//! Rebuild command
//!
//! Rebuild native extensions for installed gems

use anyhow::Result;
use lode::extensions::{builder::ExtensionBuilder, detector::detect_extension};
use lode::gem_store::GemStore;

/// Rebuild native extensions for a gem
pub(crate) fn run(gem: &str) -> Result<()> {
    let store = GemStore::new()?;
    let gems = store.find_gem_by_name(gem)?;

    if gems.is_empty() {
        anyhow::bail!("Gem '{gem}' not found");
    }

    println!("Rebuilding extensions for {gem}...\n");

    let mut rebuilt_count = 0;

    for gem_info in gems {
        println!("Processing {} ({})...", gem_info.name, gem_info.version);

        // Detect extension type
        let ext_type = detect_extension(&gem_info.path, &gem_info.name, None);

        // Check if this gem has extensions
        if !ext_type.needs_building() {
            println!("  No extensions to build ({})", ext_type.description());
            continue;
        }

        println!("  Found: {}", ext_type.description());

        // Build the extension
        let mut builder = ExtensionBuilder::new(false, true, None); // skip=false, verbose=true, no rbconfig

        println!("  Building extension...");
        match builder.build_if_needed(&gem_info.name, &gem_info.path, None) {
            Some(result) => {
                if result.success {
                    rebuilt_count += 1;
                    println!("    Successfully rebuilt in {:?}", result.duration);
                } else {
                    eprintln!(
                        "    Failed to rebuild: {}",
                        result.error.unwrap_or_else(|| "Unknown error".to_string())
                    );
                    if !result.output.is_empty() {
                        eprintln!("    Output: {}", result.output);
                    }
                }
            }
            None => {
                println!("     No build needed (already built)");
            }
        }
    }

    if rebuilt_count > 0 {
        println!("\nRebuilt {rebuilt_count} extension(s)");
    } else {
        println!("\n No extensions were rebuilt");
    }

    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "Tests can panic")]
mod tests {
    // Tests would require a test gem directory setup
}
