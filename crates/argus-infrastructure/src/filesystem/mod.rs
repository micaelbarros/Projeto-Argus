//! LocalFileSystemAdapter — scoped, symlink-safe, secret-safe filesystem adapter.

use argus_core::domain::project::{DirectoryNode, FileEntry};
use argus_core::errors::{ArgusError, ArgusResult};
use argus_core::ports::filesystem::{FileSystemPort, ScanOptions, ScannedWorkspace};
use ignore::WalkBuilder;
use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::path::{Component, Path, PathBuf};
use tokio_util::sync::CancellationToken;

/// Concrete adapter implementing `FileSystemPort` over the local OS filesystem.
#[derive(Debug, Default, Clone)]
pub struct LocalFileSystemAdapter;

impl LocalFileSystemAdapter {
    pub fn new() -> Self {
        Self
    }

    fn resolve_within_root(&self, root: &Path, relative_path: &Path) -> ArgusResult<PathBuf> {
        for comp in relative_path.components() {
            if matches!(
                comp,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            ) {
                return Err(ArgusError::InvalidPath {
                    path: relative_path.to_path_buf(),
                    reason: "Caminho relativo inválido fora do diretório autorizado.".to_string(),
                });
            }
        }
        let canonical_root = fs::canonicalize(root).map_err(|e| ArgusError::InvalidPath {
            path: root.to_path_buf(),
            reason: e.to_string(),
        })?;
        let candidate = canonical_root.join(relative_path);
        let canonical_target =
            fs::canonicalize(&candidate).map_err(|e| ArgusError::InvalidPath {
                path: candidate,
                reason: e.to_string(),
            })?;
        if !canonical_target.starts_with(&canonical_root) {
            return Err(ArgusError::PermissionDenied(format!(
                "Acesso negado fora do diretório raiz autorizado: {}",
                relative_path.display()
            )));
        }
        Ok(canonical_target)
    }

    fn count_lines_safe(path: &Path, size: u64, max_bytes: u64) -> Option<usize> {
        if size > max_bytes || size == 0 {
            return Some(0);
        }
        let mut file = fs::File::open(path).ok()?;
        let mut buf = Vec::with_capacity(size.min(64 * 1024) as usize);
        file.read_to_end(&mut buf).ok()?;
        // Skip binary files containing NUL bytes
        if buf.iter().take(4096).any(|&b| b == 0) {
            return None;
        }
        let newlines = buf.iter().filter(|&&b| b == b'\n').count();
        Some(if buf.last() == Some(&b'\n') {
            newlines
        } else {
            newlines + 1
        })
    }
}

impl FileSystemPort for LocalFileSystemAdapter {
    fn scan_workspace(
        &self,
        root: &Path,
        options: &ScanOptions,
        cancel: &CancellationToken,
    ) -> ArgusResult<ScannedWorkspace> {
        if !root.exists() {
            return Err(ArgusError::InvalidPath {
                path: root.to_path_buf(),
                reason: "Diretório não encontrado.".to_string(),
            });
        }
        if !root.is_dir() {
            return Err(ArgusError::InvalidPath {
                path: root.to_path_buf(),
                reason: "O caminho informado não é um diretório.".to_string(),
            });
        }

        let canonical_root = fs::canonicalize(root).map_err(|e| ArgusError::InvalidPath {
            path: root.to_path_buf(),
            reason: e.to_string(),
        })?;

        let ignored_dirs = options.ignored_dirs.clone();
        let mut builder = WalkBuilder::new(&canonical_root);
        builder
            .follow_links(false)
            .hidden(options.ignore_hidden)
            .git_ignore(options.respect_gitignore)
            .git_global(options.respect_gitignore)
            .git_exclude(options.respect_gitignore)
            .parents(false)
            .filter_entry(move |entry| {
                if entry.depth() == 0 {
                    return true;
                }
                if entry.file_type().is_some_and(|ft| ft.is_symlink()) {
                    return false;
                }
                if let Some(name) = entry.file_name().to_str() {
                    if ignored_dirs.iter().any(|ig| ig == name) {
                        return false;
                    }
                }
                true
            });

        let mut files = Vec::new();
        let mut total_dirs = 0usize;

        for result in builder.build() {
            if cancel.is_cancelled() {
                return Err(ArgusError::Cancelled);
            }
            let Ok(entry) = result else {
                continue;
            };
            if entry.depth() == 0 {
                continue;
            }
            let path = entry.path();
            if !path.starts_with(&canonical_root) {
                continue;
            }
            let Ok(rel) = path.strip_prefix(&canonical_root) else {
                continue;
            };

            let Some(ft) = entry.file_type() else {
                continue;
            };
            if ft.is_symlink() {
                continue;
            }
            if ft.is_dir() {
                total_dirs += 1;
                continue;
            }
            if !ft.is_file() {
                continue;
            }

            let size_bytes = entry.metadata().map(|m| m.len()).unwrap_or(0);
            let is_sensitive = self.is_sensitive_path(rel);
            let line_count = if is_sensitive {
                None
            } else {
                Self::count_lines_safe(path, size_bytes, options.max_file_size_bytes)
            };
            let extension = rel
                .extension()
                .and_then(|e| e.to_str())
                .map(|s| s.to_ascii_lowercase());

            files.push(FileEntry {
                relative_path: rel.to_path_buf(),
                size_bytes,
                extension,
                line_count,
                is_sensitive,
            });
        }

        files.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));

        let root_name = canonical_root
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("root")
            .to_string();
        let directory_tree = build_directory_tree(&root_name, &files, options.max_tree_depth);

        Ok(ScannedWorkspace {
            canonical_root,
            files,
            total_dirs,
            directory_tree,
        })
    }

    fn read_text_file(
        &self,
        root: &Path,
        relative_path: &Path,
        max_bytes: u64,
    ) -> ArgusResult<String> {
        if self.is_sensitive_path(relative_path) {
            return Err(ArgusError::PermissionDenied(format!(
                "Leitura bloqueada por segurança para arquivo sensível: {}",
                relative_path.display()
            )));
        }
        let resolved = self.resolve_within_root(root, relative_path)?;
        let metadata = fs::metadata(&resolved)?;
        if metadata.len() > max_bytes {
            return Err(ArgusError::AnalysisFailed(format!(
                "Arquivo '{}' excede o limite de {} bytes.",
                relative_path.display(),
                max_bytes
            )));
        }
        let bytes = fs::read(&resolved)?;
        String::from_utf8(bytes).map_err(|_| {
            ArgusError::AnalysisFailed(format!(
                "Arquivo '{}' não contém texto UTF-8 válido.",
                relative_path.display()
            ))
        })
    }

    fn write_file(&self, path: &Path, content: &str, overwrite: bool) -> ArgusResult<()> {
        if path.exists() && !overwrite {
            return Err(ArgusError::ExportError(format!(
                "O arquivo '{}' já existe. Use --force para confirmar a sobrescrita.",
                path.display()
            )));
        }
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }
        fs::write(path, content)?;
        Ok(())
    }
}

#[derive(Default)]
struct TreeBuilderNode {
    is_dir: bool,
    size_bytes: u64,
    children: BTreeMap<String, TreeBuilderNode>,
}

fn build_directory_tree(root_name: &str, files: &[FileEntry], max_depth: usize) -> DirectoryNode {
    let mut root = TreeBuilderNode {
        is_dir: true,
        size_bytes: 0,
        children: BTreeMap::new(),
    };

    for entry in files {
        root.size_bytes = root.size_bytes.saturating_add(entry.size_bytes);
        let components: Vec<String> = entry
            .relative_path
            .components()
            .filter_map(|c| match c {
                Component::Normal(s) => Some(s.to_string_lossy().into_owned()),
                _ => None,
            })
            .collect();

        insert_components(&mut root, &components, entry.size_bytes, 0, max_depth);
    }

    convert_tree_node(root_name.to_string(), root)
}

fn insert_components(
    node: &mut TreeBuilderNode,
    components: &[String],
    size_bytes: u64,
    depth: usize,
    max_depth: usize,
) {
    if components.is_empty() || depth >= max_depth {
        return;
    }
    let head = &components[0];
    let is_leaf = components.len() == 1;
    let child = node
        .children
        .entry(head.clone())
        .or_insert_with(|| TreeBuilderNode {
            is_dir: !is_leaf,
            size_bytes: 0,
            children: BTreeMap::new(),
        });
    child.size_bytes = child.size_bytes.saturating_add(size_bytes);
    if !is_leaf {
        child.is_dir = true;
        insert_components(child, &components[1..], size_bytes, depth + 1, max_depth);
    }
}

fn convert_tree_node(name: String, node: TreeBuilderNode) -> DirectoryNode {
    let mut children: Vec<DirectoryNode> = node
        .children
        .into_iter()
        .map(|(k, v)| convert_tree_node(k, v))
        .collect();
    children.sort_by(|a, b| b.is_dir.cmp(&a.is_dir).then_with(|| a.name.cmp(&b.name)));

    DirectoryNode {
        name,
        is_dir: node.is_dir,
        size_bytes: node.size_bytes,
        children,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn refuses_reading_env_secrets_and_path_traversal() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join(".env"), "SECRET_KEY=topsecret").unwrap();
        fs::write(dir.path().join(".env.example"), "SECRET_KEY=changeme").unwrap();

        let adapter = LocalFileSystemAdapter::new();
        assert!(adapter
            .read_text_file(dir.path(), Path::new(".env"), 1024)
            .is_err());
        assert!(adapter
            .read_text_file(dir.path(), Path::new(".env.example"), 1024)
            .is_ok());
        assert!(adapter
            .read_text_file(dir.path(), Path::new("../outside.txt"), 1024)
            .is_err());
    }

    #[test]
    fn refuses_overwrite_without_confirmation() {
        let dir = tempdir().unwrap();
        let out = dir.path().join("report.md");
        let adapter = LocalFileSystemAdapter::new();
        adapter.write_file(&out, "first", false).unwrap();
        assert!(adapter.write_file(&out, "second", false).is_err());
        adapter.write_file(&out, "second", true).unwrap();
        assert_eq!(fs::read_to_string(&out).unwrap(), "second");
    }
}
