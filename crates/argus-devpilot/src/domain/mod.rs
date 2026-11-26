//! DevPilot-specific domain abstractions and options.

use argus_core::domain::project::{Dependency, DetectedFramework, FileEntry, LanguageStat};
use std::path::PathBuf;

/// Read-only context provided to every registered `ProjectAnalyzer`.
pub struct AnalyzerContext<'a> {
    pub root: &'a std::path::Path,
    pub files: &'a [FileEntry],
    pub languages: &'a [LanguageStat],
    pub frameworks: &'a [DetectedFramework],
    pub dependencies: &'a [Dependency],
    pub dependency_files: &'a [PathBuf],
}

/// Request parameters for executing a DevPilot project analysis.
#[derive(Debug, Clone)]
pub struct AnalyzeRequest {
    pub path: PathBuf,
    pub include_git: bool,
    pub max_commits: usize,
}

impl AnalyzeRequest {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            include_git: true,
            max_commits: 20,
        }
    }
}
