//! Git operations for git gem support
//!
//! Handles cloning and checking out git repositories for gems. Git gems are
//! sourced from git repositories instead of RubyGems.org.

use anyhow::{Context, Result};
use git2::{Repository, build::CheckoutBuilder};
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GitError {
    #[error("Failed to clone {repo}: {source}")]
    CloneError {
        repo: String,
        #[source]
        source: git2::Error,
    },

    #[error("Failed to checkout {revision} in {repo}: {source}")]
    CheckoutError {
        repo: String,
        revision: String,
        #[source]
        source: git2::Error,
    },

    #[error("Repository not found at {path}")]
    RepositoryNotFound { path: String },
}

/// Manages git operations for git gem sources
#[derive(Debug)]
pub struct GitManager {
    /// Cache directory for git repositories
    cache_dir: PathBuf,
}

impl GitManager {
    /// Create a new git manager.
    ///
    /// # Errors
    ///
    /// Returns an error if the cache directory cannot be created.
    pub fn new(cache_dir: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&cache_dir).context("Failed to create git cache directory")?;
        Ok(Self { cache_dir })
    }

    /// Clone or update a git repository and checkout a specific revision
    ///
    /// Returns the path to the checked-out repository.
    ///
    /// # Arguments
    /// * `repository_url` - Git repository URL (https or ssh)
    /// * `revision` - Commit SHA to checkout
    ///
    /// # Errors
    ///
    /// Returns an error if cloning or checkout fails.
    pub fn clone_and_checkout(
        &self,
        repository_url: &str,
        revision: &str,
    ) -> Result<PathBuf, GitError> {
        let repo_name = Self::repo_name_from_url(repository_url);
        let repo_path = self.cache_dir.join(&repo_name);

        let repo = if repo_path.exists() {
            Repository::open(&repo_path).map_err(|e| GitError::CloneError {
                repo: repository_url.to_string(),
                source: e,
            })?
        } else {
            Repository::clone(repository_url, &repo_path).map_err(|e| GitError::CloneError {
                repo: repository_url.to_string(),
                source: e,
            })?
        };

        let mut remote = repo
            .find_remote("origin")
            .or_else(|_| repo.remote_anonymous(repository_url))
            .map_err(|e| GitError::CloneError {
                repo: repository_url.to_string(),
                source: e,
            })?;

        remote
            .fetch(&["refs/heads/*:refs/heads/*"], None, None)
            .map_err(|e| GitError::CloneError {
                repo: repository_url.to_string(),
                source: e,
            })?;

        let oid = git2::Oid::from_str(revision).map_err(|e| GitError::CheckoutError {
            repo: repository_url.to_string(),
            revision: revision.to_string(),
            source: e,
        })?;

        let commit = repo.find_commit(oid).map_err(|e| GitError::CheckoutError {
            repo: repository_url.to_string(),
            revision: revision.to_string(),
            source: e,
        })?;

        repo.checkout_tree(commit.as_object(), Some(CheckoutBuilder::new().force()))
            .map_err(|e| GitError::CheckoutError {
                repo: repository_url.to_string(),
                revision: revision.to_string(),
                source: e,
            })?;

        repo.set_head_detached(oid)
            .map_err(|e| GitError::CheckoutError {
                repo: repository_url.to_string(),
                revision: revision.to_string(),
                source: e,
            })?;

        Ok(repo_path)
    }

    /// Converts repository URL to safe directory name
    ///
    /// Example: `https://github.com/rails/rails` -> `github.com-rails-rails`
    fn repo_name_from_url(url: &str) -> String {
        url.trim_end_matches(".git")
            .replace("https://", "")
            .replace("http://", "")
            .replace("git@", "")
            .replace([':', '/'], "-")
    }

    /// Get the cache directory path
    #[must_use]
    pub fn cache_dir(&self) -> &Path {
        &self.cache_dir
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_repo_name_from_url() {
        assert_eq!(
            GitManager::repo_name_from_url("https://github.com/rails/rails"),
            "github.com-rails-rails"
        );

        assert_eq!(
            GitManager::repo_name_from_url("https://github.com/rails/rails.git"),
            "github.com-rails-rails"
        );

        assert_eq!(
            GitManager::repo_name_from_url("git@github.com:rails/rails.git"),
            "github.com-rails-rails"
        );
    }

    #[test]
    fn manager_creation() -> Result<()> {
        let temp_dir = tempfile::tempdir()?;
        let manager = GitManager::new(temp_dir.path().to_path_buf())?;
        assert!(manager.cache_dir().exists());
        Ok(())
    }
}
