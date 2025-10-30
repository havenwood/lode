//! Gem utilities
//!
//! Shared utility functions used across multiple gem commands.

/// Parse gem directory into (name, version) by finding last dash before digit.
/// Handles multiple dashes in name (e.g., "mini-mime-1.1.5") and platform suffixes.
///
/// # Examples
///
/// ```
/// use lode::gem_utils::parse_gem_name;
///
/// assert_eq!(parse_gem_name("rake-13.0.0"), Some(("rake", "13.0.0")));
/// assert_eq!(parse_gem_name("mini-mime-1.1.5"), Some(("mini-mime", "1.1.5")));
/// assert_eq!(parse_gem_name("nogem"), None);
/// ```
#[must_use]
pub fn parse_gem_name(dir_name: &str) -> Option<(&str, &str)> {
    let mut last_dash_before_digit = None;
    let chars: Vec<char> = dir_name.chars().collect();

    for (index, window) in chars.windows(2).enumerate() {
        if let ['-', c] = window
            && c.is_ascii_digit()
        {
            last_dash_before_digit = Some(index);
        }
    }

    last_dash_before_digit.and_then(|pos| {
        let name = &dir_name[..pos];
        let version = &dir_name[pos + 1..];
        if name.is_empty() {
            None
        } else {
            Some((name, version))
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_gem_name() {
        assert_eq!(parse_gem_name("rack-3.0.8"), Some(("rack", "3.0.8")));
        assert_eq!(parse_gem_name("rails-7.0.8"), Some(("rails", "7.0.8")));
        assert_eq!(
            parse_gem_name("mini-mime-1.1.5"),
            Some(("mini-mime", "1.1.5"))
        );
        assert_eq!(parse_gem_name("nogems"), None);
    }

    #[test]
    fn test_parse_gem_name_edge_cases() {
        // Gem with platform
        assert_eq!(
            parse_gem_name("nokogiri-1.16.0-x86_64-linux"),
            Some(("nokogiri", "1.16.0-x86_64-linux"))
        );

        // Gem with multiple dashes before version
        assert_eq!(
            parse_gem_name("activemodel-serializers-xml-1.0.2"),
            Some(("activemodel-serializers-xml", "1.0.2"))
        );

        // Gem with version starting with zero
        assert_eq!(parse_gem_name("sinatra-0.9.4"), Some(("sinatra", "0.9.4")));

        // Edge cases from other test suites
        assert_eq!(parse_gem_name(""), None);
        assert_eq!(parse_gem_name("-"), None);
        assert_eq!(parse_gem_name("-1.0.0"), None);
        assert_eq!(parse_gem_name("just-a-name"), None);
    }
}
