//! DP-002: Framework and tooling detection from configuration files and dependencies.

use argus_core::domain::project::{Dependency, DetectedFramework, FileEntry};
use std::collections::BTreeSet;

/// Detects frameworks, runtimes, and build/DevOps tooling from project files & dependencies.
pub fn detect_frameworks(
    files: &[FileEntry],
    dependencies: &[Dependency],
) -> Vec<DetectedFramework> {
    let mut results = Vec::new();
    let mut seen = BTreeSet::new();

    let mut push = |name: &str, category: &str, evidence: String| {
        if seen.insert(name.to_string()) {
            results.push(DetectedFramework {
                name: name.to_string(),
                category: category.to_string(),
                evidence,
            });
        }
    };

    for entry in files {
        let rel = entry.relative_path.to_string_lossy().replace('\\', "/");
        let file_name = entry
            .relative_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();

        match file_name {
            "Cargo.toml" => push("Cargo / Rust", "Build & Ecosystem", rel.clone()),
            "package.json" => push("Node.js", "Runtime & Ecosystem", rel.clone()),
            "tsconfig.json" => push("TypeScript", "Language Tooling", rel.clone()),
            "next.config.js" | "next.config.mjs" | "next.config.ts" => {
                push("Next.js", "Web Framework", rel.clone())
            }
            "vite.config.js" | "vite.config.ts" | "vite.config.mjs" => {
                push("Vite", "Build Tool", rel.clone())
            }
            "angular.json" => push("Angular", "Web Framework", rel.clone()),
            "nuxt.config.ts" | "nuxt.config.js" => push("Nuxt", "Web Framework", rel.clone()),
            "svelte.config.js" => push("SvelteKit", "Web Framework", rel.clone()),
            "tailwind.config.js" | "tailwind.config.ts" | "tailwind.config.cjs" => {
                push("Tailwind CSS", "Styling", rel.clone())
            }
            "pyproject.toml" | "requirements.txt" => {
                push("Python Ecosystem", "Runtime & Ecosystem", rel.clone())
            }
            "manage.py" => push("Django", "Web Framework", rel.clone()),
            "go.mod" => push("Go Modules", "Build & Ecosystem", rel.clone()),
            "pom.xml" => push("Maven / Java", "Build & Ecosystem", rel.clone()),
            "build.gradle" | "build.gradle.kts" => push("Gradle", "Build & Ecosystem", rel.clone()),
            "Dockerfile" | "Containerfile" => push("Docker", "Containerization", rel.clone()),
            "docker-compose.yml" | "docker-compose.yaml" | "compose.yml" | "compose.yaml" => {
                push("Docker Compose", "Containerization", rel.clone())
            }
            _ => {}
        }

        if rel.starts_with(".github/workflows/")
            && (rel.ends_with(".yml") || rel.ends_with(".yaml"))
        {
            push("GitHub Actions", "CI/CD", rel);
        }
    }

    for dep in dependencies {
        let name_lower = dep.name.to_ascii_lowercase();
        let evidence = format!("{} ({})", dep.name, dep.manifest);
        match name_lower.as_str() {
            "tokio" => push("Tokio", "Async Runtime", evidence),
            "ratatui" => push("Ratatui", "TUI Framework", evidence),
            "axum" => push("Axum", "Web Framework", evidence),
            "actix-web" => push("Actix Web", "Web Framework", evidence),
            "clap" => push("Clap", "CLI Framework", evidence),
            "react" => push("React", "UI Library", evidence),
            "next" => push("Next.js", "Web Framework", evidence),
            "vue" => push("Vue.js", "UI Framework", evidence),
            "express" => push("Express", "Web Framework", evidence),
            "@nestjs/core" => push("NestJS", "Backend Framework", evidence),
            "fastapi" => push("FastAPI", "Web Framework", evidence),
            "django" => push("Django", "Web Framework", evidence),
            "flask" => push("Flask", "Web Framework", evidence),
            "github.com/gin-gonic/gin" => push("Gin", "Web Framework", evidence),
            "github.com/labstack/echo/v4" => push("Echo", "Web Framework", evidence),
            _ => {}
        }
    }

    results.sort_by(|a, b| a.name.cmp(&b.name));
    results
}
