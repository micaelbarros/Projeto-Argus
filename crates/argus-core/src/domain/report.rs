//! Unified reporting and environment diagnostic domain types.

use crate::domain::alert::Alert;
use crate::domain::network::{LatencyReport, NetworkInterface};
use crate::domain::project::ProjectAnalysis;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Supported report export formats.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ReportFormat {
    Json,
    Markdown,
}

/// Combined technical report exported by `argus report`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ArgusReport {
    pub generator: String,
    pub version: String,
    pub generated_at: DateTime<Utc>,
    pub project_analysis: Option<ProjectAnalysis>,
    pub network_interfaces: Vec<NetworkInterface>,
    pub latency: Option<LatencyReport>,
    pub alerts: Vec<Alert>,
}

/// Individual check produced by `argus doctor`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DiagnosticCheck {
    pub name: String,
    pub ok: bool,
    pub detail: String,
}

/// Environment diagnostic summary.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DoctorReport {
    pub app: String,
    pub version: String,
    pub os: String,
    pub arch: String,
    pub checked_at: DateTime<Utc>,
    pub checks: Vec<DiagnosticCheck>,
}
