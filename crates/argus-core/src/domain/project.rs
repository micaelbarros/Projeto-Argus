//! Project and DevPilot domain models.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Root metadata of an analyzed software project.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Project {
    pub name: String,
    pub root_path: PathBuf,
    pub total_files: usize,
    pub total_dirs: usize,
    pub total_bytes: u64,
    pub total_lines: usize,
}

/// Entry representing a scanned file within the project boundary.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FileEntry {
    pub relative_path: PathBuf,
    pub size_bytes: u64,
    pub extension: Option<String>,
    pub line_count: Option<usize>,
    pub is_sensitive: bool,
}

/// Recursive directory tree node.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DirectoryNode {
    pub name: String,
    pub is_dir: bool,
    pub size_bytes: u64,
    pub children: Vec<DirectoryNode>,
}

/// Language usage statistics.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LanguageStat {
    pub language: String,
    pub files: usize,
    pub lines: usize,
    pub bytes: u64,
    pub percentage: f64,
}

/// Detected framework or tooling via configuration files.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DetectedFramework {
    pub name: String,
    pub category: String,
    pub evidence: String,
}

/// Declared project dependency parsed from manifest files.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Dependency {
    pub name: String,
    pub version: String,
    pub ecosystem: String,
    pub manifest: String,
    pub is_dev: bool,
}

/// TODO / FIXME / HACK marker discovered in source files.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TodoMarker {
    pub kind: String,
    pub file: PathBuf,
    pub line: usize,
    pub text: String,
}

/// Finding for files exceeding configured size limits.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LargeFileFinding {
    pub path: PathBuf,
    pub size_bytes: u64,
    pub threshold_bytes: u64,
}

/// Detected circular dependency cycle across internal modules/imports.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CircularDependency {
    pub cycle: Vec<String>,
}

/// Severity classification for health findings.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum HealthSeverity {
    Info,
    Warning,
    Critical,
}

/// Single diagnostic issue contributing to the project health report.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HealthIssue {
    pub code: String,
    pub severity: HealthSeverity,
    pub title: String,
    pub detail: String,
    pub recommendation: String,
}

/// Aggregated project health score and recommendations.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HealthReport {
    pub score: u8,
    pub grade: String,
    pub passed_checks: Vec<String>,
    pub issues: Vec<HealthIssue>,
}

/// Single Git commit metadata.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GitCommitInfo {
    pub hash: String,
    pub short_hash: String,
    pub author: String,
    pub timestamp: DateTime<Utc>,
    pub summary: String,
}

/// Working tree file change status.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GitFileStatus {
    pub path: String,
    pub status: String,
}

/// Git repository summary for a project.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GitSummary {
    pub branch: String,
    pub is_dirty: bool,
    pub total_commits: usize,
    pub recent_commits: Vec<GitCommitInfo>,
    pub modified_files: Vec<GitFileStatus>,
    pub contributors: Vec<String>,
}

/// Extra key-value finding produced by extensible analyzers.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CustomFinding {
    pub analyzer: String,
    pub category: String,
    pub summary: String,
    pub details: Vec<String>,
}

/// Complete output of a DevPilot analysis run.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProjectAnalysis {
    pub project: Project,
    pub analyzed_at: DateTime<Utc>,
    pub duration_ms: u64,
    pub languages: Vec<LanguageStat>,
    pub frameworks: Vec<DetectedFramework>,
    pub dependency_files: Vec<PathBuf>,
    pub dependencies: Vec<Dependency>,
    pub todos: Vec<TodoMarker>,
    pub large_files: Vec<LargeFileFinding>,
    pub circular_dependencies: Vec<CircularDependency>,
    pub directory_tree: DirectoryNode,
    pub health: HealthReport,
    pub git: Option<GitSummary>,
    pub custom_findings: Vec<CustomFinding>,
}

/// Persistent summary row stored in SQLite `analysis_runs`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AnalysisRunRecord {
    pub id: i64,
    pub project_path: String,
    pub started_at: DateTime<Utc>,
    pub finished_at: DateTime<Utc>,
    pub status: String,
    pub score: i32,
}
