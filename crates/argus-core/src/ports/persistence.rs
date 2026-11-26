//! MetricsRepositoryPort — optional local persistence for history, metrics, alerts, and settings.

use crate::domain::alert::{Alert, AlertModule, AlertSeverity};
use crate::domain::network::NetworkMetric;
use crate::domain::project::AnalysisRunRecord;
use crate::errors::ArgusResult;
use chrono::{DateTime, Utc};

/// Input payload to persist an analysis run.
#[derive(Debug, Clone)]
pub struct NewAnalysisRun {
    pub project_path: String,
    pub started_at: DateTime<Utc>,
    pub finished_at: DateTime<Utc>,
    pub status: String,
    pub score: i32,
}

/// Input payload to persist a network metric sample.
#[derive(Debug, Clone)]
pub struct NewNetworkMetric {
    pub interface_name: String,
    pub timestamp: DateTime<Utc>,
    pub bytes_received: u64,
    pub bytes_sent: u64,
    pub latency_ms: Option<f64>,
}

/// Input payload to persist an alert.
#[derive(Debug, Clone)]
pub struct NewAlert {
    pub module: AlertModule,
    pub severity: AlertSeverity,
    pub message: String,
    pub created_at: DateTime<Utc>,
}

/// Port abstracting local persistence (implemented via SQLite or no-op adapter).
pub trait MetricsRepositoryPort: Send + Sync {
    fn save_analysis_run(&self, run: &NewAnalysisRun) -> ArgusResult<i64>;
    fn list_analysis_runs(&self, limit: usize) -> ArgusResult<Vec<AnalysisRunRecord>>;

    fn save_network_metric(&self, metric: &NewNetworkMetric) -> ArgusResult<i64>;
    fn list_network_metrics(
        &self,
        interface_name: Option<&str>,
        limit: usize,
    ) -> ArgusResult<Vec<NetworkMetric>>;

    fn save_alert(&self, alert: &NewAlert) -> ArgusResult<i64>;
    fn list_alerts(&self, only_unacknowledged: bool, limit: usize) -> ArgusResult<Vec<Alert>>;
    fn acknowledge_alert(&self, id: i64) -> ArgusResult<bool>;

    fn set_setting(&self, key: &str, value: &str) -> ArgusResult<()>;
    fn get_setting(&self, key: &str) -> ArgusResult<Option<String>>;

    fn prune_older_than(&self, retention_days: u32) -> ArgusResult<usize>;
}
