//! Git2Adapter — read-only Git repository inspection via `libgit2`.

use argus_core::domain::project::{GitCommitInfo, GitFileStatus, GitSummary};
use argus_core::errors::{ArgusError, ArgusResult};
use argus_core::ports::git::GitRepositoryPort;
use chrono::{TimeZone, Utc};
use git2::{Repository, StatusOptions};
use std::collections::BTreeSet;
use std::path::Path;

/// Concrete adapter implementing `GitRepositoryPort` using `git2`.
#[derive(Debug, Default, Clone)]
pub struct Git2Adapter;

impl Git2Adapter {
    pub fn new() -> Self {
        Self
    }
}

impl GitRepositoryPort for Git2Adapter {
    fn inspect_repository(
        &self,
        path: &Path,
        max_commits: usize,
    ) -> ArgusResult<Option<GitSummary>> {
        let repo = match Repository::discover(path) {
            Ok(r) => r,
            Err(err) if err.code() == git2::ErrorCode::NotFound => return Ok(None),
            Err(err) => return Err(ArgusError::GitError(err.message().to_string())),
        };

        let branch = match repo.head() {
            Ok(head) => head
                .shorthand()
                .map(|s| s.to_string())
                .unwrap_or_else(|| "HEAD (detached)".to_string()),
            Err(_) => "main (unborn)".to_string(),
        };

        let mut status_opts = StatusOptions::new();
        status_opts
            .include_untracked(true)
            .recurse_untracked_dirs(true)
            .exclude_submodules(true);

        let mut modified_files = Vec::new();
        if let Ok(statuses) = repo.statuses(Some(&mut status_opts)) {
            for entry in statuses.iter() {
                let st = entry.status();
                let label = if st.is_wt_new() || st.is_index_new() {
                    "added/untracked"
                } else if st.is_wt_deleted() || st.is_index_deleted() {
                    "deleted"
                } else if st.is_wt_renamed() || st.is_index_renamed() {
                    "renamed"
                } else if st.is_wt_modified() || st.is_index_modified() {
                    "modified"
                } else {
                    continue;
                };
                if let Some(p) = entry.path() {
                    modified_files.push(GitFileStatus {
                        path: p.to_string(),
                        status: label.to_string(),
                    });
                }
            }
        }
        let is_dirty = !modified_files.is_empty();

        let mut recent_commits = Vec::new();
        let mut contributors = BTreeSet::new();
        let mut total_commits = 0usize;

        if let Ok(mut revwalk) = repo.revwalk() {
            if revwalk.push_head().is_ok() {
                let _ = revwalk.set_sorting(git2::Sort::TIME);
                for oid_res in revwalk {
                    let Ok(oid) = oid_res else {
                        continue;
                    };
                    total_commits += 1;
                    if let Ok(commit) = repo.find_commit(oid) {
                        let author = commit.author().name().unwrap_or("unknown").to_string();
                        contributors.insert(author.clone());
                        if recent_commits.len() < max_commits {
                            let hash = oid.to_string();
                            let short_hash = hash.chars().take(7).collect();
                            let secs = commit.time().seconds();
                            let timestamp =
                                Utc.timestamp_opt(secs, 0).single().unwrap_or_else(Utc::now);
                            let summary = commit
                                .summary()
                                .unwrap_or("(sem mensagem)")
                                .trim()
                                .to_string();
                            recent_commits.push(GitCommitInfo {
                                hash,
                                short_hash,
                                author,
                                timestamp,
                                summary,
                            });
                        }
                    }
                    // Cap full traversal at 10,000 commits for fast startup on giant repos
                    if total_commits >= 10_000 {
                        break;
                    }
                }
            }
        }

        Ok(Some(GitSummary {
            branch,
            is_dirty,
            total_commits,
            recent_commits,
            modified_files,
            contributors: contributors.into_iter().collect(),
        }))
    }
}
