//! SQLiteMetricsAdapter — local-first transactional persistence for history, metrics, alerts, and settings.

use argus_core::domain::alert::{Alert, AlertModule, AlertSeverity};
use argus_core::domain::network::NetworkMetric;
use argus_core::domain::project::AnalysisRunRecord;
use argus_core::errors::{ArgusError, ArgusResult};
use argus_core::ports::persistence::{
    MetricsRepositoryPort, NewAlert, NewAnalysisRun, NewNetworkMetric,
};
use chrono::{DateTime, Duration, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use std::fs;
use std::path::Path;
use std::sync::Mutex;

/// SQLite-backed implementation of `MetricsRepositoryPort`.
pub struct SQLiteMetricsAdapter {
    conn: Mutex<Connection>,
}

impl SQLiteMetricsAdapter {
    /// Opens (or creates) a local SQLite database at `path` and runs schema migrations.
    pub fn open(path: &Path) -> ArgusResult<Self> {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }
        let conn = Connection::open(path)
            .map_err(|e| ArgusError::DatabaseError(format!("Falha ao abrir SQLite: {e}")))?;
        let adapter = Self {
            conn: Mutex::new(conn),
        };
        adapter.initialize_schema()?;
        Ok(adapter)
    }

    /// Creates an in-memory SQLite database (useful for tests).
    pub fn open_in_memory() -> ArgusResult<Self> {
        let conn = Connection::open_in_memory().map_err(|e| {
            ArgusError::DatabaseError(format!("Falha ao abrir SQLite em memória: {e}"))
        })?;
        let adapter = Self {
            conn: Mutex::new(conn),
        };
        adapter.initialize_schema()?;
        Ok(adapter)
    }

    fn initialize_schema(&self) -> ArgusResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| ArgusError::DatabaseError("Mutex SQLite corrompido".to_string()))?;

        conn.execute_batch(
            "
            PRAGMA journal_mode = WAL;
            PRAGMA synchronous = NORMAL;

            CREATE TABLE IF NOT EXISTS analysis_runs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                project_path TEXT NOT NULL,
                started_at TEXT NOT NULL,
                finished_at TEXT NOT NULL,
                status TEXT NOT NULL,
                score INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS network_metrics (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                interface_name TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                bytes_received INTEGER NOT NULL,
                bytes_sent INTEGER NOT NULL,
                latency_ms REAL
            );

            CREATE TABLE IF NOT EXISTS alerts (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                module TEXT NOT NULL,
                severity TEXT NOT NULL,
                message TEXT NOT NULL,
                created_at TEXT NOT NULL,
                acknowledged INTEGER NOT NULL DEFAULT 0
            );

            CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_analysis_runs_started
                ON analysis_runs(started_at DESC);
            CREATE INDEX IF NOT EXISTS idx_network_metrics_iface_ts
                ON network_metrics(interface_name, timestamp DESC);
            CREATE INDEX IF NOT EXISTS idx_alerts_created_ack
                ON alerts(acknowledged, created_at DESC);
            ",
        )
        .map_err(|e| {
            ArgusError::DatabaseError(format!("Erro ao inicializar schema SQLite: {e}"))
        })?;

        Ok(())
    }

    fn parse_dt(raw: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(raw)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now())
    }

    fn parse_module(raw: &str) -> AlertModule {
        match raw.to_ascii_lowercase().as_str() {
            "devpilot" => AlertModule::DevPilot,
            "netscope" => AlertModule::NetScope,
            _ => AlertModule::Core,
        }
    }

    fn parse_severity(raw: &str) -> AlertSeverity {
        match raw.to_ascii_lowercase().as_str() {
            "critical" => AlertSeverity::Critical,
            "warning" => AlertSeverity::Warning,
            _ => AlertSeverity::Info,
        }
    }
}

impl MetricsRepositoryPort for SQLiteMetricsAdapter {
    fn save_analysis_run(&self, run: &NewAnalysisRun) -> ArgusResult<i64> {
        let mut conn = self
            .conn
            .lock()
            .map_err(|_| ArgusError::DatabaseError("Mutex SQLite corrompido".to_string()))?;
        let tx = conn
            .transaction()
            .map_err(|e| ArgusError::DatabaseError(e.to_string()))?;
        tx.execute(
            "INSERT INTO analysis_runs (project_path, started_at, finished_at, status, score)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                run.project_path,
                run.started_at.to_rfc3339(),
                run.finished_at.to_rfc3339(),
                run.status,
                run.score
            ],
        )
        .map_err(|e| ArgusError::DatabaseError(e.to_string()))?;
        let id = tx.last_insert_rowid();
        tx.commit()
            .map_err(|e| ArgusError::DatabaseError(e.to_string()))?;
        Ok(id)
    }

    fn list_analysis_runs(&self, limit: usize) -> ArgusResult<Vec<AnalysisRunRecord>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| ArgusError::DatabaseError("Mutex SQLite corrompido".to_string()))?;
        let mut stmt = conn
            .prepare(
                "SELECT id, project_path, started_at, finished_at, status, score
                 FROM analysis_runs ORDER BY id DESC LIMIT ?1",
            )
            .map_err(|e| ArgusError::DatabaseError(e.to_string()))?;

        let rows = stmt
            .query_map(params![limit as i64], |row| {
                let started: String = row.get(2)?;
                let finished: String = row.get(3)?;
                Ok(AnalysisRunRecord {
                    id: row.get(0)?,
                    project_path: row.get(1)?,
                    started_at: Self::parse_dt(&started),
                    finished_at: Self::parse_dt(&finished),
                    status: row.get(4)?,
                    score: row.get(5)?,
                })
            })
            .map_err(|e| ArgusError::DatabaseError(e.to_string()))?;

        let mut out = Vec::new();
        for item in rows {
            out.push(item.map_err(|e| ArgusError::DatabaseError(e.to_string()))?);
        }
        Ok(out)
    }

    fn save_network_metric(&self, metric: &NewNetworkMetric) -> ArgusResult<i64> {
        let mut conn = self
            .conn
            .lock()
            .map_err(|_| ArgusError::DatabaseError("Mutex SQLite corrompido".to_string()))?;
        let tx = conn
            .transaction()
            .map_err(|e| ArgusError::DatabaseError(e.to_string()))?;
        tx.execute(
            "INSERT INTO network_metrics (interface_name, timestamp, bytes_received, bytes_sent, latency_ms)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                metric.interface_name,
                metric.timestamp.to_rfc3339(),
                metric.bytes_received as i64,
                metric.bytes_sent as i64,
                metric.latency_ms
            ],
        )
        .map_err(|e| ArgusError::DatabaseError(e.to_string()))?;
        let id = tx.last_insert_rowid();
        tx.commit()
            .map_err(|e| ArgusError::DatabaseError(e.to_string()))?;
        Ok(id)
    }

    fn list_network_metrics(
        &self,
        interface_name: Option<&str>,
        limit: usize,
    ) -> ArgusResult<Vec<NetworkMetric>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| ArgusError::DatabaseError("Mutex SQLite corrompido".to_string()))?;

        let mut out = Vec::new();
        if let Some(iface) = interface_name {
            let mut stmt = conn
                .prepare(
                    "SELECT id, interface_name, timestamp, bytes_received, bytes_sent, latency_ms
                     FROM network_metrics WHERE interface_name = ?1 ORDER BY id DESC LIMIT ?2",
                )
                .map_err(|e| ArgusError::DatabaseError(e.to_string()))?;
            let rows = stmt
                .query_map(params![iface, limit as i64], |row| {
                    let ts: String = row.get(2)?;
                    Ok(NetworkMetric {
                        id: row.get(0)?,
                        interface_name: row.get(1)?,
                        timestamp: Self::parse_dt(&ts),
                        bytes_received: row.get::<_, i64>(3)?.max(0) as u64,
                        bytes_sent: row.get::<_, i64>(4)?.max(0) as u64,
                        latency_ms: row.get(5)?,
                    })
                })
                .map_err(|e| ArgusError::DatabaseError(e.to_string()))?;
            for item in rows {
                out.push(item.map_err(|e| ArgusError::DatabaseError(e.to_string()))?);
            }
        } else {
            let mut stmt = conn
                .prepare(
                    "SELECT id, interface_name, timestamp, bytes_received, bytes_sent, latency_ms
                     FROM network_metrics ORDER BY id DESC LIMIT ?1",
                )
                .map_err(|e| ArgusError::DatabaseError(e.to_string()))?;
            let rows = stmt
                .query_map(params![limit as i64], |row| {
                    let ts: String = row.get(2)?;
                    Ok(NetworkMetric {
                        id: row.get(0)?,
                        interface_name: row.get(1)?,
                        timestamp: Self::parse_dt(&ts),
                        bytes_received: row.get::<_, i64>(3)?.max(0) as u64,
                        bytes_sent: row.get::<_, i64>(4)?.max(0) as u64,
                        latency_ms: row.get(5)?,
                    })
                })
                .map_err(|e| ArgusError::DatabaseError(e.to_string()))?;
            for item in rows {
                out.push(item.map_err(|e| ArgusError::DatabaseError(e.to_string()))?);
            }
        }
        Ok(out)
    }

    fn save_alert(&self, alert: &NewAlert) -> ArgusResult<i64> {
        let mut conn = self
            .conn
            .lock()
            .map_err(|_| ArgusError::DatabaseError("Mutex SQLite corrompido".to_string()))?;
        let tx = conn
            .transaction()
            .map_err(|e| ArgusError::DatabaseError(e.to_string()))?;
        tx.execute(
            "INSERT INTO alerts (module, severity, message, created_at, acknowledged)
             VALUES (?1, ?2, ?3, ?4, 0)",
            params![
                alert.module.to_string(),
                alert.severity.to_string(),
                alert.message,
                alert.created_at.to_rfc3339()
            ],
        )
        .map_err(|e| ArgusError::DatabaseError(e.to_string()))?;
        let id = tx.last_insert_rowid();
        tx.commit()
            .map_err(|e| ArgusError::DatabaseError(e.to_string()))?;
        Ok(id)
    }

    fn list_alerts(&self, only_unacknowledged: bool, limit: usize) -> ArgusResult<Vec<Alert>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| ArgusError::DatabaseError("Mutex SQLite corrompido".to_string()))?;
        let sql = if only_unacknowledged {
            "SELECT id, module, severity, message, created_at, acknowledged
             FROM alerts WHERE acknowledged = 0 ORDER BY id DESC LIMIT ?1"
        } else {
            "SELECT id, module, severity, message, created_at, acknowledged
             FROM alerts ORDER BY id DESC LIMIT ?1"
        };
        let mut stmt = conn
            .prepare(sql)
            .map_err(|e| ArgusError::DatabaseError(e.to_string()))?;
        let rows = stmt
            .query_map(params![limit as i64], |row| {
                let module: String = row.get(1)?;
                let severity: String = row.get(2)?;
                let created: String = row.get(4)?;
                let ack: i64 = row.get(5)?;
                Ok(Alert {
                    id: row.get(0)?,
                    module: Self::parse_module(&module),
                    severity: Self::parse_severity(&severity),
                    message: row.get(3)?,
                    created_at: Self::parse_dt(&created),
                    acknowledged: ack != 0,
                })
            })
            .map_err(|e| ArgusError::DatabaseError(e.to_string()))?;

        let mut out = Vec::new();
        for item in rows {
            out.push(item.map_err(|e| ArgusError::DatabaseError(e.to_string()))?);
        }
        Ok(out)
    }

    fn acknowledge_alert(&self, id: i64) -> ArgusResult<bool> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| ArgusError::DatabaseError("Mutex SQLite corrompido".to_string()))?;
        let updated = conn
            .execute(
                "UPDATE alerts SET acknowledged = 1 WHERE id = ?1",
                params![id],
            )
            .map_err(|e| ArgusError::DatabaseError(e.to_string()))?;
        Ok(updated > 0)
    }

    fn set_setting(&self, key: &str, value: &str) -> ArgusResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| ArgusError::DatabaseError("Mutex SQLite corrompido".to_string()))?;
        conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )
        .map_err(|e| ArgusError::DatabaseError(e.to_string()))?;
        Ok(())
    }

    fn get_setting(&self, key: &str) -> ArgusResult<Option<String>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| ArgusError::DatabaseError("Mutex SQLite corrompido".to_string()))?;
        conn.query_row(
            "SELECT value FROM settings WHERE key = ?1",
            params![key],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| ArgusError::DatabaseError(e.to_string()))
    }

    fn prune_older_than(&self, retention_days: u32) -> ArgusResult<usize> {
        let cutoff = (Utc::now() - Duration::days(i64::from(retention_days))).to_rfc3339();
        let mut conn = self
            .conn
            .lock()
            .map_err(|_| ArgusError::DatabaseError("Mutex SQLite corrompido".to_string()))?;
        let tx = conn
            .transaction()
            .map_err(|e| ArgusError::DatabaseError(e.to_string()))?;
        let a = tx
            .execute(
                "DELETE FROM analysis_runs WHERE started_at < ?1",
                params![cutoff],
            )
            .map_err(|e| ArgusError::DatabaseError(e.to_string()))?;
        let n = tx
            .execute(
                "DELETE FROM network_metrics WHERE timestamp < ?1",
                params![cutoff],
            )
            .map_err(|e| ArgusError::DatabaseError(e.to_string()))?;
        tx.commit()
            .map_err(|e| ArgusError::DatabaseError(e.to_string()))?;
        Ok(a + n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn persists_and_queries_all_tables() {
        let db = SQLiteMetricsAdapter::open_in_memory().unwrap();
        let now = Utc::now();

        let run_id = db
            .save_analysis_run(&NewAnalysisRun {
                project_path: "/tmp/demo".to_string(),
                started_at: now,
                finished_at: now,
                status: "completed".to_string(),
                score: 92,
            })
            .unwrap();
        assert!(run_id > 0);
        let runs = db.list_analysis_runs(10).unwrap();
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].score, 92);

        db.save_network_metric(&NewNetworkMetric {
            interface_name: "eth0".to_string(),
            timestamp: now,
            bytes_received: 2048,
            bytes_sent: 1024,
            latency_ms: Some(12.5),
        })
        .unwrap();
        let metrics = db.list_network_metrics(Some("eth0"), 10).unwrap();
        assert_eq!(metrics.len(), 1);
        assert_eq!(metrics[0].bytes_received, 2048);

        let alert_id = db
            .save_alert(&NewAlert {
                module: AlertModule::NetScope,
                severity: AlertSeverity::Warning,
                message: "High latency".to_string(),
                created_at: now,
            })
            .unwrap();
        assert_eq!(db.list_alerts(true, 10).unwrap().len(), 1);
        assert!(db.acknowledge_alert(alert_id).unwrap());
        assert_eq!(db.list_alerts(true, 10).unwrap().len(), 0);

        db.set_setting("theme", "dark").unwrap();
        assert_eq!(db.get_setting("theme").unwrap().as_deref(), Some("dark"));
    }
}
