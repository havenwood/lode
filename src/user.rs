//! User and permission utilities.

use std::process::Command;

/// Check if the current process is running as root.
///
/// On Unix systems, this checks if the effective user ID is 0 by running `id -u`.
/// On Windows, this always returns false (root detection not implemented).
///
/// # Examples
///
/// ```
/// if lode::user::is_root() {
///     eprintln!("Warning: Running as root user");
/// }
/// ```
#[must_use]
pub fn is_root() -> bool {
    #[cfg(unix)]
    {
        // On Unix, root user has UID 0
        // Use `id -u` command to get effective user ID
        Command::new("id")
            .arg("-u")
            .output()
            .ok()
            .and_then(|output| String::from_utf8(output.stdout).ok())
            .and_then(|uid| uid.trim().parse::<u32>().ok())
            .is_some_and(|uid| uid == 0)
    }

    #[cfg(not(unix))]
    {
        // On Windows, we don't check for admin privileges yet
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_root() {
        // This test just ensures the function doesn't panic
        // The actual value depends on who's running the test
        let _ = is_root();
    }
}
