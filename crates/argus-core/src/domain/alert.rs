//! Alert domain entities shared by DevPilot and NetScope.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Module originating an alert.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum AlertModule {
    Core,
    DevPilot,
    NetScope,
}

impl std::fmt::Display for AlertModule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Core => write!(f, "core"),
            Self::DevPilot => write!(f, "devpilot"),
            Self::NetScope => write!(f, "netscope"),
        }
    }
}

/// Alert severity classification.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
}

impl std::fmt::Display for AlertSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Info => write!(f, "info"),
            Self::Warning => write!(f, "warning"),
            Self::Critical => write!(f, "critical"),
        }
    }
}

/// Structured alert persisted in SQLite `alerts`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Alert {
    pub id: i64,
    pub module: AlertModule,
    pub severity: AlertSeverity,
    pub message: String,
    pub created_at: DateTime<Utc>,
    pub acknowledged: bool,
}
