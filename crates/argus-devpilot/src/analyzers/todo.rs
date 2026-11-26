//! DP-007: TODO / FIXME / HACK marker detector.

use argus_core::domain::project::{FileEntry, TodoMarker};
use argus_core::ports::filesystem::FileSystemPort;
use regex::Regex;
use std::path::Path;
use std::sync::OnceLock;
use tokio_util::sync::CancellationToken;

fn marker_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?i)\b(TODO|FIXME|HACK|XXX)\b\s*[:\-]?\s*(.*)").expect("valid marker regex")
    })
}

/// Scans non-sensitive source files for TODO/FIXME/HACK/XXX markers.
pub fn scan_todos(
    root: &Path,
    files: &[FileEntry],
    max_file_bytes: u64,
    fs: &dyn FileSystemPort,
    cancel: &CancellationToken,
) -> Vec<TodoMarker> {
    let re = marker_regex();
    let mut markers = Vec::new();

    for entry in files {
        if cancel.is_cancelled() {
            break;
        }
        if entry.is_sensitive || entry.size_bytes > max_file_bytes {
            continue;
        }
        if !is_scannable_text_ext(entry.extension.as_deref()) {
            continue;
        }
        let Ok(content) = fs.read_text_file(root, &entry.relative_path, max_file_bytes) else {
            continue;
        };

        for (idx, line) in content.lines().enumerate() {
            if let Some(caps) = re.captures(line) {
                let kind = caps
                    .get(1)
                    .map(|m| m.as_str().to_ascii_uppercase())
                    .unwrap_or_else(|| "TODO".to_string());
                let raw_text = caps.get(2).map(|m| m.as_str().trim()).unwrap_or_default();
                // Trim overly long comments cleanly
                let text: String = raw_text.chars().take(140).collect();
                markers.push(TodoMarker {
                    kind,
                    file: entry.relative_path.clone(),
                    line: idx + 1,
                    text,
                });
            }
        }
    }

    markers.sort_by(|a, b| a.file.cmp(&b.file).then_with(|| a.line.cmp(&b.line)));
    markers
}

fn is_scannable_text_ext(ext: Option<&str>) -> bool {
    let Some(e) = ext else {
        return false;
    };
    matches!(
        e.to_ascii_lowercase().as_str(),
        "rs" | "ts"
            | "tsx"
            | "js"
            | "jsx"
            | "py"
            | "go"
            | "java"
            | "kt"
            | "cs"
            | "c"
            | "h"
            | "cpp"
            | "hpp"
            | "rb"
            | "php"
            | "swift"
            | "sh"
            | "ps1"
            | "sql"
            | "toml"
            | "yaml"
            | "yml"
    )
}
