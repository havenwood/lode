//! Search command
//!
//! Search for gems

use anyhow::{Context, Result};
use serde::Deserialize;

/// Search result from RubyGems.org API
#[derive(Debug, Deserialize)]
struct SearchResult {
    name: String,
    #[serde(default)]
    downloads: u64,
    version: String,
    #[serde(default)]
    info: String,
}

/// Search for gems on RubyGems.org
pub(crate) async fn run(query: &str) -> Result<()> {
    if query.is_empty() {
        anyhow::bail!("Search query cannot be empty");
    }

    let limit = 10; // Default limit

    // Build search URL with query parameter using reqwest's query builder
    // This ensures proper URL encoding for special characters and spaces
    let host = lode::env_vars::rubygems_host();
    let url = format!("{host}/api/v1/search.json");
    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .query(&[("query", query)])
        .send()
        .await
        .with_context(|| format!("Failed to search for: {query}"))?;

    if !response.status().is_success() {
        anyhow::bail!("Search failed with status: {}", response.status());
    }

    let mut results: Vec<SearchResult> = response
        .json()
        .await
        .with_context(|| "Failed to parse search results")?;

    if results.is_empty() {
        println!("No gems found matching '{query}'");
        return Ok(());
    }

    // Sort by downloads (descending) to show most popular first
    results.sort_by(|a, b| b.downloads.cmp(&a.downloads));

    // Limit results
    let display_count = results.len().min(limit);
    results.truncate(display_count);

    println!("Gems matching '{query}' ({display_count} results):\n");

    for result in &results {
        println!("{} ({})", result.name, result.version);

        if !result.info.is_empty() {
            // Truncate long descriptions
            let info = if result.info.len() > 100 {
                format!("{}...", &result.info[..97])
            } else {
                result.info.clone()
            };
            println!("   {info}");
        }

        if result.downloads > 0 {
            println!("   {} downloads", format_downloads(result.downloads));
        }

        println!();
    }

    if results.len() < limit {
        println!("Showing all {} matching gems", results.len());
    } else {
        println!(
            "Showing top {} results (sorted by downloads)",
            results.len()
        );
    }

    Ok(())
}

/// Format download count with commas for readability
fn format_downloads(count: u64) -> String {
    let s = count.to_string();
    let mut result = String::new();
    let chars: Vec<char> = s.chars().rev().collect();

    for (i, c) in chars.iter().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(*c);
    }

    result.chars().rev().collect()
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "Tests can panic")]
mod tests {
    use super::*;

    #[test]
    fn test_format_downloads() {
        assert_eq!(format_downloads(0), "0");
        assert_eq!(format_downloads(123), "123");
        assert_eq!(format_downloads(1234), "1,234");
        assert_eq!(format_downloads(12_345), "12,345");
        assert_eq!(format_downloads(123_456), "123,456");
        assert_eq!(format_downloads(1_234_567), "1,234,567");
        assert_eq!(format_downloads(12_345_678), "12,345,678");
    }

    #[tokio::test]
    #[ignore = "Requires network access to rubygems.org"]
    async fn test_search_rack() {
        let result = run("rack").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_search_empty_query() {
        let result = run("").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cannot be empty"));
    }

    #[tokio::test]
    #[ignore = "Requires network access to rubygems.org"]
    async fn test_search_no_results() {
        let result = run("this-gem-absolutely-does-not-exist-xyz12345").await;
        assert!(result.is_ok()); // Should succeed but show no results
    }
}
