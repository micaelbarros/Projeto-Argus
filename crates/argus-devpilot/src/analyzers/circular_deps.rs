//! DP-011: Circular dependency detection across internal relative imports/modules.

use argus_core::domain::project::{CircularDependency, FileEntry};
use argus_core::ports::filesystem::FileSystemPort;
use regex::Regex;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path, PathBuf};
use std::sync::OnceLock;

fn js_import_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"(?:import\s+.*?\s+from\s+|require\()\s*['"](\.\.?/[^'"]+)['"]"#)
            .expect("valid js import regex")
    })
}

fn rust_use_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"\buse\s+crate::([a-zA-Z0-9_]+)").expect("valid rust use regex")
    })
}

fn py_import_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"^\s*from\s+\.([a-zA-Z0-9_\.]+)\s+import\b").expect("valid py import regex")
    })
}

/// Builds a lightweight internal import graph and detects cycles via DFS.
pub fn detect_circular_dependencies(
    root: &Path,
    files: &[FileEntry],
    max_file_bytes: u64,
    fs: &dyn FileSystemPort,
) -> Vec<CircularDependency> {
    let mut known_nodes: BTreeMap<String, PathBuf> = BTreeMap::new();
    for entry in files {
        if entry.is_sensitive {
            continue;
        }
        if let Some(key) = normalize_module_key(&entry.relative_path) {
            known_nodes.insert(key, entry.relative_path.clone());
        }
    }

    let mut graph: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();

    for (node_key, rel_path) in &known_nodes {
        let Ok(content) = fs.read_text_file(root, rel_path, max_file_bytes) else {
            continue;
        };
        let ext = rel_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();

        let mut targets = BTreeSet::new();
        match ext.as_str() {
            "js" | "jsx" | "ts" | "tsx" | "mjs" | "cjs" => {
                let parent = rel_path.parent().unwrap_or_else(|| Path::new(""));
                for cap in js_import_re().captures_iter(&content) {
                    if let Some(rel_imp) = cap.get(1) {
                        let resolved = normalize_rel_path(&parent.join(rel_imp.as_str()));
                        if let Some(target_key) = resolve_known_node(&resolved, &known_nodes) {
                            if target_key != *node_key {
                                targets.insert(target_key);
                            }
                        }
                    }
                }
            }
            "rs" => {
                // Scope Rust `use crate::foo` to the same crate root (`.../src/`)
                let crate_prefix = rust_crate_src_prefix(rel_path);
                for cap in rust_use_re().captures_iter(&content) {
                    if let Some(module_name) = cap.get(1) {
                        let candidate = format!("{crate_prefix}{}", module_name.as_str());
                        if let Some(target_key) = resolve_known_rust(&candidate, &known_nodes) {
                            if target_key != *node_key {
                                targets.insert(target_key);
                            }
                        }
                    }
                }
            }
            "py" => {
                let parent = rel_path.parent().unwrap_or_else(|| Path::new(""));
                for line in content.lines() {
                    if let Some(cap) = py_import_re().captures(line) {
                        if let Some(mod_path) = cap.get(1) {
                            let rel_mod = mod_path.as_str().replace('.', "/");
                            let resolved = normalize_rel_path(&parent.join(rel_mod));
                            if let Some(target_key) = resolve_known_node(&resolved, &known_nodes) {
                                if target_key != *node_key {
                                    targets.insert(target_key);
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }

        if !targets.is_empty() {
            graph.insert(node_key.clone(), targets);
        }
    }

    find_cycles(&graph)
}

fn normalize_module_key(path: &Path) -> Option<String> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    if !matches!(
        ext.as_str(),
        "rs" | "js" | "jsx" | "ts" | "tsx" | "mjs" | "cjs" | "py"
    ) {
        return None;
    }
    let without_ext = path.with_extension("");
    let raw = without_ext.to_string_lossy().replace('\\', "/");
    let stripped = raw
        .strip_suffix("/mod")
        .or_else(|| raw.strip_suffix("/index"))
        .or_else(|| raw.strip_suffix("/__init__"))
        .unwrap_or(&raw);
    Some(stripped.to_string())
}

fn normalize_rel_path(path: &Path) -> String {
    let mut stack: Vec<String> = Vec::new();
    for comp in path.components() {
        match comp {
            Component::CurDir => {}
            Component::ParentDir => {
                stack.pop();
            }
            Component::Normal(seg) => stack.push(seg.to_string_lossy().into_owned()),
            _ => {}
        }
    }
    stack.join("/")
}

fn resolve_known_node(
    candidate: &str,
    known: &BTreeMap<String, PathBuf>,
) -> Option<String> {
    let stripped = candidate
        .strip_suffix(".ts")
        .or_else(|| candidate.strip_suffix(".tsx"))
        .or_else(|| candidate.strip_suffix(".js"))
        .or_else(|| candidate.strip_suffix(".jsx"))
        .or_else(|| candidate.strip_suffix(".py"))
        .unwrap_or(candidate);
    if known.contains_key(stripped) {
        Some(stripped.to_string())
    } else {
        None
    }
}

fn rust_crate_src_prefix(rel_path: &Path) -> String {
    let s = rel_path.to_string_lossy().replace('\\', "/");
    if let Some(idx) = s.find("src/") {
        s[..idx + 4].to_string()
    } else {
        String::new()
    }
}

fn resolve_known_rust(candidate: &str, known: &BTreeMap<String, PathBuf>) -> Option<String> {
    if known.contains_key(candidate) {
        Some(candidate.to_string())
    } else {
        None
    }
}

fn find_cycles(graph: &BTreeMap<String, BTreeSet<String>>) -> Vec<CircularDependency> {
    let mut visited = BTreeSet::new();
    let mut stack = Vec::new();
    let mut on_stack = BTreeSet::new();
    let mut unique_cycles: BTreeSet<Vec<String>> = BTreeSet::new();

    for node in graph.keys() {
        if !visited.contains(node) {
            dfs(
                node,
                graph,
                &mut visited,
                &mut stack,
                &mut on_stack,
                &mut unique_cycles,
            );
        }
    }

    unique_cycles
        .into_iter()
        .map(|cycle| CircularDependency { cycle })
        .collect()
}

fn dfs(
    node: &str,
    graph: &BTreeMap<String, BTreeSet<String>>,
    visited: &mut BTreeSet<String>,
    stack: &mut Vec<String>,
    on_stack: &mut BTreeSet<String>,
    cycles: &mut BTreeSet<Vec<String>>,
) {
    visited.insert(node.to_string());
    stack.push(node.to_string());
    on_stack.insert(node.to_string());

    if let Some(neighbors) = graph.get(node) {
        for next in neighbors {
            if on_stack.contains(next) {
                if let Some(pos) = stack.iter().position(|n| n == next) {
                    let mut cycle = stack[pos..].to_vec();
                    // Canonicalize rotation so A->B->A and B->A->B deduplicate
                    if let Some((min_idx, _)) =
                        cycle.iter().enumerate().min_by(|a, b| a.1.cmp(b.1))
                    {
                        cycle.rotate_left(min_idx);
                    }
                    cycle.push(cycle[0].clone());
                    cycles.insert(cycle);
                }
            } else if !visited.contains(next) && graph.contains_key(next) {
                dfs(next, graph, visited, stack, on_stack, cycles);
            }
        }
    }

    stack.pop();
    on_stack.remove(node);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_simple_cycle_and_deduplicates() {
        let mut graph = BTreeMap::new();
        graph.insert("src/a".to_string(), BTreeSet::from(["src/b".to_string()]));
        graph.insert("src/b".to_string(), BTreeSet::from(["src/a".to_string()]));
        let cycles = find_cycles(&graph);
        assert_eq!(cycles.len(), 1);
        assert_eq!(cycles[0].cycle, vec!["src/a", "src/b", "src/a"]);
    }
}
