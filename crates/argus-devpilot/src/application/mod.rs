//! DevPilot application use cases orchestrating scanners, detectors, analyzers, Git, and persistence.

use crate::analyzers::circular_deps::detect_circular_dependencies;
use crate::analyzers::docs::generate_project_markdown;
use crate::analyzers::health::compute_health_report;
use crate::analyzers::large_files::detect_large_files;
use crate::analyzers::todo::scan_todos;
use crate::analyzers::{AnalyzerContext, AnalyzerRegistry};
use crate::detectors::dependency::inspect_dependencies;
use crate::detectors::framework::detect_frameworks;
use crate::detectors::language::aggregate_languages;
use crate::domain::AnalyzeRequest;
use argus_core::config::ArgusConfig;
use argus_core::domain::project::{GitSummary, Project, ProjectAnalysis};
use argus_core::errors::{ArgusError, ArgusResult};
use argus_core::ports::filesystem::{FileSystemPort, ScanOptions};
use argus_core::ports::git::GitRepositoryPort;
use argus_core::ports::persistence::{MetricsRepositoryPort, NewAnalysisRun};
use chrono::Utc;
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

/// Application service for all DevPilot use cases.
pub struct DevPilotService {
    fs: Arc<dyn FileSystemPort>,
    git: Arc<dyn GitRepositoryPort>,
    repo: Option<Arc<dyn MetricsRepositoryPort>>,
    registry: AnalyzerRegistry,
}

impl DevPilotService {
    pub fn new(
        fs: Arc<dyn FileSystemPort>,
        git: Arc<dyn GitRepositoryPort>,
        repo: Option<Arc<dyn MetricsRepositoryPort>>,
    ) -> Self {
        Self {
            fs,
            git,
            repo,
            registry: AnalyzerRegistry::with_defaults(),
        }
    }

    /// Registers an additional analyzer into the service registry.
    pub fn with_analyzer(mut self, analyzer: Box<dyn crate::analyzers::ProjectAnalyzer>) -> Self {
        self.registry.register(analyzer);
        self
    }

    /// Executes full project analysis (scan, languages, frameworks, deps, TODOs, cycles, health, Git).
    pub fn analyze(
        &self,
        request: &AnalyzeRequest,
        config: &ArgusConfig,
        cancel: &CancellationToken,
    ) -> ArgusResult<ProjectAnalysis> {
        if !config.modules.devpilot_enabled {
            return Err(ArgusError::ModuleDisabled("devpilot".to_string()));
        }
        if cancel.is_cancelled() {
            return Err(ArgusError::Cancelled);
        }

        let started_at = Utc::now();
        let timer = Instant::now();
        let max_file_bytes = config.devpilot.max_file_size_mb.saturating_mul(1024 * 1024);

        let scan_opts = ScanOptions {
            max_file_size_bytes: max_file_bytes,
            ignore_hidden: config.devpilot.ignore_hidden,
            respect_gitignore: config.devpilot.respect_gitignore,
            ignored_dirs: config.devpilot.ignored_dirs.clone(),
            max_tree_depth: config.devpilot.max_tree_depth,
        };

        info!(path = %request.path.display(), "Starting DevPilot analysis");
        let scanned = self.fs.scan_workspace(&request.path, &scan_opts, cancel)?;
        if cancel.is_cancelled() {
            return Err(ArgusError::Cancelled);
        }

        let languages = aggregate_languages(&scanned.files);
        let (dependency_files, dependencies) =
            inspect_dependencies(&scanned.canonical_root, &scanned.files, self.fs.as_ref());
        let frameworks = detect_frameworks(&scanned.files, &dependencies);
        let todos = scan_todos(
            &scanned.canonical_root,
            &scanned.files,
            max_file_bytes,
            self.fs.as_ref(),
            cancel,
        );
        if cancel.is_cancelled() {
            return Err(ArgusError::Cancelled);
        }

        let large_files =
            detect_large_files(&scanned.files, config.devpilot.large_file_threshold_kb);
        let circular_dependencies = detect_circular_dependencies(
            &scanned.canonical_root,
            &scanned.files,
            max_file_bytes,
            self.fs.as_ref(),
        );

        let git = if request.include_git {
            match self
                .git
                .inspect_repository(&scanned.canonical_root, request.max_commits)
            {
                Ok(summary) => summary,
                Err(err) => {
                    warn!(error = %err, "Git inspection skipped");
                    None
                }
            }
        } else {
            None
        };

        let health = compute_health_report(
            &scanned.files,
            &todos,
            &large_files,
            &circular_dependencies,
            git.as_ref(),
        );

        let ctx = AnalyzerContext {
            root: &scanned.canonical_root,
            files: &scanned.files,
            languages: &languages,
            frameworks: &frameworks,
            dependencies: &dependencies,
            dependency_files: &dependency_files,
        };
        let custom_findings = self.registry.run_all(&ctx)?;

        let total_bytes: u64 = scanned.files.iter().map(|f| f.size_bytes).sum();
        let total_lines: usize = scanned.files.iter().filter_map(|f| f.line_count).sum();
        let project_name = scanned
            .canonical_root
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("project")
            .to_string();

        let finished_at = Utc::now();
        let duration_ms = timer.elapsed().as_millis() as u64;

        let analysis = ProjectAnalysis {
            project: Project {
                name: project_name,
                root_path: scanned.canonical_root.clone(),
                total_files: scanned.files.len(),
                total_dirs: scanned.total_dirs,
                total_bytes,
                total_lines,
            },
            analyzed_at: finished_at,
            duration_ms,
            languages,
            frameworks,
            dependency_files,
            dependencies,
            todos,
            large_files,
            circular_dependencies,
            directory_tree: scanned.directory_tree,
            health,
            git,
            custom_findings,
        };

        if let Some(repo) = &self.repo {
            if config.storage.database_enabled {
                let record = NewAnalysisRun {
                    project_path: analysis.project.root_path.display().to_string(),
                    started_at,
                    finished_at,
                    status: "completed".to_string(),
                    score: i32::from(analysis.health.score),
                };
                if let Err(err) = repo.save_analysis_run(&record) {
                    warn!(error = %err, "Failed to persist DevPilot analysis run");
                }
            }
        }

        Ok(analysis)
    }

    /// Inspects Git repository details directly.
    pub fn inspect_git(
        &self,
        path: &Path,
        max_commits: usize,
        config: &ArgusConfig,
    ) -> ArgusResult<GitSummary> {
        if !config.modules.devpilot_enabled {
            return Err(ArgusError::ModuleDisabled("devpilot".to_string()));
        }
        self.git
            .inspect_repository(path, max_commits)?
            .ok_or_else(|| {
                ArgusError::GitError(format!(
                    "Nenhum repositório Git encontrado em '{}'.",
                    path.display()
                ))
            })
    }

    /// Generates Markdown documentation for a project without overwriting existing files unless allowed.
    pub fn generate_docs(
        &self,
        path: &Path,
        config: &ArgusConfig,
        cancel: &CancellationToken,
    ) -> ArgusResult<String> {
        let req = AnalyzeRequest::new(path.to_path_buf());
        let analysis = self.analyze(&req, config, cancel)?;
        Ok(generate_project_markdown(&analysis))
    }
}
