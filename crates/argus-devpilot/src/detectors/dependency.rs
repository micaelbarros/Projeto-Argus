//! DP-005 & DP-006: Dependency manifest discovery and declared dependency parsing.

use argus_core::domain::project::{Dependency, FileEntry};
use argus_core::ports::filesystem::FileSystemPort;
use std::path::{Path, PathBuf};

const MAX_MANIFEST_BYTES: u64 = 2 * 1024 * 1024;

/// Known dependency manifest filenames across major ecosystems.
pub fn is_dependency_manifest(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return false;
    };
    matches!(
        name,
        "Cargo.toml"
            | "package.json"
            | "requirements.txt"
            | "pyproject.toml"
            | "go.mod"
            | "pom.xml"
            | "composer.json"
            | "Gemfile"
    )
}

/// Discovers manifest files and parses declared dependencies safely without executing code.
pub fn inspect_dependencies(
    root: &Path,
    files: &[FileEntry],
    fs: &dyn FileSystemPort,
) -> (Vec<PathBuf>, Vec<Dependency>) {
    let mut manifests = Vec::new();
    let mut deps = Vec::new();

    for entry in files {
        if !is_dependency_manifest(&entry.relative_path) {
            continue;
        }
        manifests.push(entry.relative_path.clone());

        let Ok(content) = fs.read_text_file(root, &entry.relative_path, MAX_MANIFEST_BYTES) else {
            continue;
        };
        let manifest_label = entry.relative_path.to_string_lossy().replace('\\', "/");
        let file_name = entry
            .relative_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();

        match file_name {
            "Cargo.toml" => deps.extend(parse_cargo_toml(&content, &manifest_label)),
            "package.json" => deps.extend(parse_package_json(&content, &manifest_label)),
            "requirements.txt" => deps.extend(parse_requirements_txt(&content, &manifest_label)),
            "pyproject.toml" => deps.extend(parse_pyproject_toml(&content, &manifest_label)),
            "go.mod" => deps.extend(parse_go_mod(&content, &manifest_label)),
            _ => {}
        }
    }

    manifests.sort();
    deps.sort_by(|a, b| {
        a.ecosystem
            .cmp(&b.ecosystem)
            .then_with(|| a.name.cmp(&b.name))
            .then_with(|| a.manifest.cmp(&b.manifest))
    });
    (manifests, deps)
}

fn parse_cargo_toml(content: &str, manifest: &str) -> Vec<Dependency> {
    let Ok(val) = content.parse::<toml::Value>() else {
        return Vec::new();
    };
    let mut out = Vec::new();

    let extract_table = |table: &toml::Table, is_dev: bool, target: &mut Vec<Dependency>| {
        for (name, spec) in table {
            let version = match spec {
                toml::Value::String(s) => s.clone(),
                toml::Value::Table(t) => {
                    if let Some(v) = t.get("version").and_then(|v| v.as_str()) {
                        v.to_string()
                    } else if t.get("workspace").and_then(|v| v.as_bool()) == Some(true) {
                        "workspace".to_string()
                    } else if let Some(p) = t.get("path").and_then(|v| v.as_str()) {
                        format!("path:{p}")
                    } else {
                        "*".to_string()
                    }
                }
                _ => "*".to_string(),
            };
            target.push(Dependency {
                name: name.clone(),
                version,
                ecosystem: "cargo".to_string(),
                manifest: manifest.to_string(),
                is_dev,
            });
        }
    };

    if let Some(t) = val.get("dependencies").and_then(|v| v.as_table()) {
        extract_table(t, false, &mut out);
    }
    if let Some(t) = val.get("dev-dependencies").and_then(|v| v.as_table()) {
        extract_table(t, true, &mut out);
    }
    if let Some(ws) = val
        .get("workspace")
        .and_then(|v| v.get("dependencies"))
        .and_then(|v| v.as_table())
    {
        extract_table(ws, false, &mut out);
    }

    out
}

fn parse_package_json(content: &str, manifest: &str) -> Vec<Dependency> {
    let Ok(val) = serde_json::from_str::<serde_json::Value>(content) else {
        return Vec::new();
    };
    let mut out = Vec::new();

    for (section, is_dev) in [("dependencies", false), ("devDependencies", true)] {
        if let Some(obj) = val.get(section).and_then(|v| v.as_object()) {
            for (name, ver) in obj {
                out.push(Dependency {
                    name: name.clone(),
                    version: ver.as_str().unwrap_or("*").to_string(),
                    ecosystem: "npm".to_string(),
                    manifest: manifest.to_string(),
                    is_dev,
                });
            }
        }
    }
    out
}

fn parse_requirements_txt(content: &str, manifest: &str) -> Vec<Dependency> {
    let mut out = Vec::new();
    for raw_line in content.lines() {
        let line = raw_line.split('#').next().unwrap_or_default().trim();
        if line.is_empty() || line.starts_with('-') {
            continue;
        }
        let separators = ["==", ">=", "<=", "~=", "!=", ">", "<"];
        let mut parsed = None;
        for sep in separators {
            if let Some((n, v)) = line.split_once(sep) {
                parsed = Some((n.trim().to_string(), format!("{sep}{}", v.trim())));
                break;
            }
        }
        let (name, version) = parsed.unwrap_or_else(|| (line.to_string(), "*".to_string()));
        if !name.is_empty() {
            out.push(Dependency {
                name,
                version,
                ecosystem: "pip".to_string(),
                manifest: manifest.to_string(),
                is_dev: false,
            });
        }
    }
    out
}

fn parse_pyproject_toml(content: &str, manifest: &str) -> Vec<Dependency> {
    let Ok(val) = content.parse::<toml::Value>() else {
        return Vec::new();
    };
    let mut out = Vec::new();
    if let Some(arr) = val
        .get("project")
        .and_then(|p| p.get("dependencies"))
        .and_then(|d| d.as_array())
    {
        for item in arr.iter().filter_map(|v| v.as_str()) {
            out.extend(parse_requirements_txt(item, manifest));
        }
    }
    out
}

fn parse_go_mod(content: &str, manifest: &str) -> Vec<Dependency> {
    let mut out = Vec::new();
    let mut in_block = false;
    for raw_line in content.lines() {
        let line = raw_line.split("//").next().unwrap_or_default().trim();
        if line.starts_with("require (") || line == "require (" {
            in_block = true;
            continue;
        }
        if in_block && line == ")" {
            in_block = false;
            continue;
        }
        let candidate = if in_block {
            line
        } else if let Some(rest) = line.strip_prefix("require ") {
            rest.trim()
        } else {
            continue;
        };
        let mut parts = candidate.split_whitespace();
        if let (Some(name), Some(version)) = (parts.next(), parts.next()) {
            out.push(Dependency {
                name: name.to_string(),
                version: version.to_string(),
                ecosystem: "go".to_string(),
                manifest: manifest.to_string(),
                is_dev: false,
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_cargo_and_npm_and_pip_and_go() {
        let cargo = r#"
[dependencies]
serde = "1.0"
tokio = { version = "1.40", features = ["full"] }

[dev-dependencies]
tempfile = "3.15"
"#;
        let parsed = parse_cargo_toml(cargo, "Cargo.toml");
        assert_eq!(parsed.len(), 3);
        assert!(parsed.iter().any(|d| d.name == "serde" && !d.is_dev));
        assert!(parsed.iter().any(|d| d.name == "tempfile" && d.is_dev));

        let pkg = r#"{"dependencies":{"react":"^19.0.0"},"devDependencies":{"vitest":"^3.0.0"}}"#;
        let npm = parse_package_json(pkg, "package.json");
        assert_eq!(npm.len(), 2);

        let req = "fastapi==0.115.0\n# comment\nuvicorn>=0.30\n";
        let pip = parse_requirements_txt(req, "requirements.txt");
        assert_eq!(pip.len(), 2);

        let gomod = "module example.com/app\nrequire (\n\tgithub.com/gin-gonic/gin v1.10.0\n)\n";
        let go = parse_go_mod(gomod, "go.mod");
        assert_eq!(go.len(), 1);
        assert_eq!(go[0].name, "github.com/gin-gonic/gin");
    }
}
