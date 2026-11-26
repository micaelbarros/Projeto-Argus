//! Extensible analyzer registry and built-in code/health/documentation analyzers.

pub mod circular_deps;
pub mod docs;
pub mod health;
pub mod large_files;
pub mod todo;

pub use crate::domain::AnalyzerContext;
use argus_core::domain::project::CustomFinding;
use argus_core::errors::ArgusResult;

/// DP-014: Extensible analyzer trait for internal Rust plugins.
pub trait ProjectAnalyzer: Send + Sync {
    /// Unique identifier of the analyzer.
    fn name(&self) -> &'static str;

    /// Executes the analyzer against the read-only project context.
    fn analyze(&self, ctx: &AnalyzerContext<'_>) -> ArgusResult<Vec<CustomFinding>>;
}

/// Registry storing active analyzers executed during `DevPilotService::analyze`.
#[derive(Default)]
pub struct AnalyzerRegistry {
    analyzers: Vec<Box<dyn ProjectAnalyzer>>,
}

impl AnalyzerRegistry {
    /// Creates an empty registry.
    pub fn new() -> Self {
        Self {
            analyzers: Vec::new(),
        }
    }

    /// Creates a registry populated with ARGUS default built-in analyzers.
    pub fn with_defaults() -> Self {
        let mut registry = Self::new();
        registry.register(Box::new(SecurityHygieneAnalyzer));
        registry
    }

    /// Registers a new analyzer.
    pub fn register(&mut self, analyzer: Box<dyn ProjectAnalyzer>) {
        self.analyzers.push(analyzer);
    }

    /// Runs all registered analyzers in deterministic registration order.
    pub fn run_all(&self, ctx: &AnalyzerContext<'_>) -> ArgusResult<Vec<CustomFinding>> {
        let mut findings = Vec::new();
        for analyzer in &self.analyzers {
            findings.extend(analyzer.analyze(ctx)?);
        }
        Ok(findings)
    }
}

/// Built-in analyzer checking lockfile reproducibility and `.env` hygiene.
pub struct SecurityHygieneAnalyzer;

impl ProjectAnalyzer for SecurityHygieneAnalyzer {
    fn name(&self) -> &'static str {
        "security-hygiene"
    }

    fn analyze(&self, ctx: &AnalyzerContext<'_>) -> ArgusResult<Vec<CustomFinding>> {
        let mut details = Vec::new();

        let sensitive_count = ctx.files.iter().filter(|f| f.is_sensitive).count();
        if sensitive_count > 0 {
            details.push(format!(
                "Encontrados {sensitive_count} arquivo(s) sensível(is) (.env/chaves); conteúdo nunca é exposto pelo ARGUS."
            ));
        }

        let has_cargo = ctx
            .dependency_files
            .iter()
            .any(|p| p.file_name().and_then(|n| n.to_str()) == Some("Cargo.toml"));
        let has_cargo_lock = ctx
            .files
            .iter()
            .any(|f| f.relative_path.file_name().and_then(|n| n.to_str()) == Some("Cargo.lock"));
        if has_cargo && !has_cargo_lock {
            details.push("Cargo.toml detectado sem Cargo.lock versionado.".to_string());
        }

        if details.is_empty() {
            Ok(Vec::new())
        } else {
            Ok(vec![CustomFinding {
                analyzer: self.name().to_string(),
                category: "Hygiene".to_string(),
                summary: format!("{} observação(ões) de higiene do repositório", details.len()),
                details,
            }])
        }
    }
}
