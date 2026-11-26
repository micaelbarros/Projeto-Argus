//! FileSystemPort — safe, scoped filesystem access without symlink escape or secret leaks.

use crate::domain::project::{DirectoryNode, FileEntry};
use crate::errors::ArgusResult;
use std::path::{Path, PathBuf};
use tokio_util::sync::CancellationToken;

/// Configuration options controlling directory scanning.
#[derive(Debug, Clone)]
pub struct ScanOptions {
    pub max_file_size_bytes: u64,
    pub ignore_hidden: bool,
    pub respect_gitignore: bool,
    pub ignored_dirs: Vec<String>,
    pub max_tree_depth: usize,
}

/// Output of a scoped directory scan.
#[derive(Debug, Clone)]
pub struct ScannedWorkspace {
    pub canonical_root: PathBuf,
    pub files: Vec<FileEntry>,
    pub total_dirs: usize,
    pub directory_tree: DirectoryNode,
}

/// Port abstracting local filesystem operations for DevPilot.
pub trait FileSystemPort: Send + Sync {
    /// Scans `root` within its canonical boundary, never following symlinks outside `root`.
    fn scan_workspace(
        &self,
        root: &Path,
        options: &ScanOptions,
        cancel: &CancellationToken,
    ) -> ArgusResult<ScannedWorkspace>;

    /// Reads a UTF-8 text file safely, refusing sensitive `.env` secret files and files
    /// exceeding `max_bytes` or escaping `root`.
    fn read_text_file(
        &self,
        root: &Path,
        relative_path: &Path,
        max_bytes: u64,
    ) -> ArgusResult<String>;

    /// Writes output safely. If `overwrite` is false and the file exists, returns an error.
    fn write_file(&self, path: &Path, content: &str, overwrite: bool) -> ArgusResult<()>;

    /// Checks whether a path is considered sensitive (e.g. `.env`, `.env.local`, private keys).
    fn is_sensitive_path(&self, path: &Path) -> bool {
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            return false;
        };
        let lower = name.to_ascii_lowercase();
        if lower == ".env.example" || lower == ".env.sample" || lower == ".env.template" {
            return false;
        }
        lower == ".env"
            || lower.starts_with(".env.")
            || lower.ends_with(".pem")
            || lower.ends_with(".key")
            || lower == "id_rsa"
            || lower == "id_ed25519"
            || lower == "credentials.json"
    }
}
