//! GitRepositoryPort — read-only Git repository inspection.

use crate::domain::project::GitSummary;
use crate::errors::ArgusResult;
use std::path::Path;

/// Port abstracting Git repository inspection (implemented via `git2`).
pub trait GitRepositoryPort: Send + Sync {
    /// Inspects repository metadata, working tree status, and up to `max_commits` history items.
    /// Returns `Ok(None)` if `path` is not inside a Git repository.
    fn inspect_repository(&self, path: &Path, max_commits: usize) -> ArgusResult<Option<GitSummary>>;
}
