//! DevPilot module — repository intelligence, dependency parsing, health scoring, and docs.

pub mod analyzers;
pub mod application;
pub mod detectors;
pub mod domain;

pub use analyzers::docs::generate_project_markdown;
pub use analyzers::{AnalyzerContext, AnalyzerRegistry, ProjectAnalyzer};
pub use application::DevPilotService;
