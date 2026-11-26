//! Integration tests for DevPilot end-to-end analysis, Git, health, cycles, and persistence.

use argus::core::config::ArgusConfig;
use argus::core::ports::persistence::MetricsRepositoryPort;
use argus::devpilot::domain::AnalyzeRequest;
use argus::devpilot::{generate_project_markdown, DevPilotService};
use argus::infrastructure::{Git2Adapter, LocalFileSystemAdapter, SQLiteMetricsAdapter};
use std::fs;
use std::sync::Arc;
use tempfile::tempdir;
use tokio_util::sync::CancellationToken;

#[test]
fn analyzes_full_temporary_project_with_git_deps_todos_and_cycles() {
    let dir = tempdir().unwrap();
    let root = dir.path();

    // Create sample Rust + JS files, Cargo.toml, README, LICENSE, .gitignore, tests, and a JS cycle
    fs::create_dir_all(root.join("src")).unwrap();
    fs::create_dir_all(root.join("tests")).unwrap();
    fs::create_dir_all(root.join(".github/workflows")).unwrap();
    fs::create_dir_all(root.join("node_modules/pkg")).unwrap();

    fs::write(root.join("README.md"), "# Sample\n").unwrap();
    fs::write(root.join("LICENSE"), "MIT\n").unwrap();
    fs::write(root.join(".gitignore"), "/target\nnode_modules/\n").unwrap();
    fs::write(root.join(".github/workflows/ci.yml"), "name: CI\n").unwrap();
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"sample\"\nversion = \"0.1.0\"\n\n[dependencies]\nserde = \"1.0\"\ntokio = \"1.40\"\n",
    )
    .unwrap();
    fs::write(
        root.join("src/main.rs"),
        "// TODO: refactor startup\nfn main() {\n    // FIXME: handle edge case\n    println!(\"hi\");\n}\n",
    )
    .unwrap();
    fs::write(
        root.join("src/a.ts"),
        "import { b } from './b';\nexport const a = b;\n",
    )
    .unwrap();
    fs::write(
        root.join("src/b.ts"),
        "import { a } from './a';\nexport const b = a;\n",
    )
    .unwrap();
    fs::write(root.join("tests/basic.rs"), "#[test]\nfn ok() {}\n").unwrap();
    // Should be ignored by ignored_dirs
    fs::write(root.join("node_modules/pkg/index.js"), "// TODO: ignored\n").unwrap();
    // Sensitive file must NOT be read for TODOs
    fs::write(root.join(".env"), "API_KEY=super_secret_todo_TODO_value\n").unwrap();

    // Initialize a Git repository with one commit
    let git_repo = git2::Repository::init(root).unwrap();
    let mut index = git_repo.index().unwrap();
    index.add_path(std::path::Path::new("README.md")).unwrap();
    index.write().unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = git_repo.find_tree(tree_id).unwrap();
    let sig = git2::Signature::now("Tester", "tester@example.com").unwrap();
    git_repo
        .commit(Some("HEAD"), &sig, &sig, "feat: initial commit", &tree, &[])
        .unwrap();

    let fs_adapter = Arc::new(LocalFileSystemAdapter::new());
    let git_adapter = Arc::new(Git2Adapter::new());
    let db = Arc::new(SQLiteMetricsAdapter::open_in_memory().unwrap());
    let repo_port: Arc<dyn MetricsRepositoryPort> = db.clone();

    let service = DevPilotService::new(fs_adapter, git_adapter, Some(repo_port));
    let mut config = ArgusConfig::default();
    config.devpilot.ignore_hidden = false; // allow inspecting .github and detecting .env sensitivity
    let cancel = CancellationToken::new();

    let analysis = service
        .analyze(&AnalyzeRequest::new(root.to_path_buf()), &config, &cancel)
        .unwrap();

    // Verify language detection & ignored directory exclusion
    assert!(analysis.languages.iter().any(|l| l.language == "Rust"));
    assert!(analysis
        .languages
        .iter()
        .any(|l| l.language == "TypeScript"));
    assert!(!analysis
        .todos
        .iter()
        .any(|t| t.file.to_string_lossy().contains("node_modules")));
    // Verify .env content was NEVER scanned
    assert!(!analysis
        .todos
        .iter()
        .any(|t| t.text.contains("super_secret")));
    // Verify TODO / FIXME found in src/main.rs
    assert_eq!(analysis.todos.len(), 2);
    // Verify dependencies & frameworks detected
    assert!(analysis.dependencies.iter().any(|d| d.name == "serde"));
    assert!(analysis.frameworks.iter().any(|f| f.name == "Tokio"));
    assert!(analysis
        .frameworks
        .iter()
        .any(|f| f.name == "GitHub Actions"));
    // Verify circular dependency detected between src/a and src/b
    assert_eq!(analysis.circular_dependencies.len(), 1);
    // Verify Git summary populated
    let git = analysis.git.as_ref().expect("git summary expected");
    assert_eq!(git.total_commits, 1);
    assert_eq!(git.recent_commits[0].summary, "feat: initial commit");
    // Verify Markdown doc generation works
    let md = generate_project_markdown(&analysis);
    assert!(md.contains("Linguagens Detectadas"));
    assert!(md.contains("Projeto ARGUS"));
    // Verify SQLite persistence recorded the run
    let runs = db.list_analysis_runs(10).unwrap();
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].score, i32::from(analysis.health.score));
}

#[test]
fn respects_cooperative_cancellation() {
    let dir = tempdir().unwrap();
    let fs_adapter = Arc::new(LocalFileSystemAdapter::new());
    let git_adapter = Arc::new(Git2Adapter::new());
    let service = DevPilotService::new(fs_adapter, git_adapter, None);

    let cancel = CancellationToken::new();
    cancel.cancel();

    let res = service.analyze(
        &AnalyzeRequest::new(dir.path().to_path_buf()),
        &ArgusConfig::default(),
        &cancel,
    );
    assert!(res.is_err());
}
