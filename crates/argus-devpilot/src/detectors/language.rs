//! DP-001: Programming language detection from file extensions and canonical filenames.

use argus_core::domain::project::{FileEntry, LanguageStat};
use std::collections::BTreeMap;
use std::path::Path;

/// Maps a file path to its human-readable programming/markup language, if recognized.
pub fn detect_language(path: &Path) -> Option<&'static str> {
    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
        match name {
            "Dockerfile" | "Containerfile" => return Some("Docker"),
            "Makefile" | "GNUmakefile" => return Some("Makefile"),
            "CMakeLists.txt" => return Some("CMake"),
            "Justfile" | "justfile" => return Some("Just"),
            _ => {}
        }
        if name.starts_with("Dockerfile.") {
            return Some("Docker");
        }
    }

    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_ascii_lowercase())?;

    let lang = match ext.as_str() {
        "rs" => "Rust",
        "ts" | "mts" | "cts" => "TypeScript",
        "tsx" => "TypeScript (TSX)",
        "js" | "mjs" | "cjs" => "JavaScript",
        "jsx" => "JavaScript (JSX)",
        "py" | "pyi" => "Python",
        "go" => "Go",
        "java" => "Java",
        "kt" | "kts" => "Kotlin",
        "cs" => "C#",
        "c" | "h" => "C",
        "cpp" | "cc" | "cxx" | "hpp" | "hh" => "C++",
        "rb" => "Ruby",
        "php" => "PHP",
        "swift" => "Swift",
        "scala" => "Scala",
        "dart" => "Dart",
        "zig" => "Zig",
        "lua" => "Lua",
        "sh" | "bash" | "zsh" => "Shell",
        "ps1" | "psm1" => "PowerShell",
        "sql" => "SQL",
        "html" | "htm" => "HTML",
        "css" => "CSS",
        "scss" | "sass" => "SCSS",
        "vue" => "Vue",
        "svelte" => "Svelte",
        "json" | "jsonc" => "JSON",
        "toml" => "TOML",
        "yaml" | "yml" => "YAML",
        "xml" => "XML",
        "md" | "markdown" => "Markdown",
        "tf" | "tfvars" => "Terraform",
        "proto" => "Protocol Buffers",
        _ => return None,
    };
    Some(lang)
}

/// Aggregates language statistics across scanned files in deterministic descending order.
pub fn aggregate_languages(files: &[FileEntry]) -> Vec<LanguageStat> {
    let mut map: BTreeMap<&'static str, (usize, usize, u64)> = BTreeMap::new();
    let mut total_bytes: u64 = 0;

    for entry in files {
        if let Some(lang) = detect_language(&entry.relative_path) {
            let slot = map.entry(lang).or_insert((0, 0, 0));
            slot.0 += 1;
            slot.1 += entry.line_count.unwrap_or(0);
            slot.2 += entry.size_bytes;
            total_bytes += entry.size_bytes;
        }
    }

    let mut stats: Vec<LanguageStat> = map
        .into_iter()
        .map(|(language, (files_count, lines, bytes))| {
            let percentage = if total_bytes > 0 {
                (bytes as f64 / total_bytes as f64) * 100.0
            } else {
                0.0
            };
            LanguageStat {
                language: language.to_string(),
                files: files_count,
                lines,
                bytes,
                percentage: (percentage * 100.0).round() / 100.0,
            }
        })
        .collect();

    stats.sort_by(|a, b| {
        b.bytes
            .cmp(&a.bytes)
            .then_with(|| b.files.cmp(&a.files))
            .then_with(|| a.language.cmp(&b.language))
    });
    stats
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn detects_common_languages() {
        assert_eq!(detect_language(Path::new("src/main.rs")), Some("Rust"));
        assert_eq!(
            detect_language(Path::new("web/app.tsx")),
            Some("TypeScript (TSX)")
        );
        assert_eq!(detect_language(Path::new("Dockerfile")), Some("Docker"));
        assert_eq!(detect_language(Path::new("bin/blob.unknown")), None);
    }

    #[test]
    fn aggregates_percentages_accurately() {
        let files = vec![
            FileEntry {
                relative_path: PathBuf::from("src/lib.rs"),
                size_bytes: 750,
                extension: Some("rs".into()),
                line_count: Some(30),
                is_sensitive: false,
            },
            FileEntry {
                relative_path: PathBuf::from("README.md"),
                size_bytes: 250,
                extension: Some("md".into()),
                line_count: Some(10),
                is_sensitive: false,
            },
        ];
        let stats = aggregate_languages(&files);
        assert_eq!(stats.len(), 2);
        assert_eq!(stats[0].language, "Rust");
        assert!((stats[0].percentage - 75.0).abs() < 0.01);
        assert_eq!(stats[1].language, "Markdown");
        assert!((stats[1].percentage - 25.0).abs() < 0.01);
    }
}
