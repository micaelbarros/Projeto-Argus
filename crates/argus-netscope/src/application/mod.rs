//! NetScope application service coordinating interfaces, rates, connections, latency, and alerts.

use crate::connections::{filter_connections, summarize_connections};
use crate::discovery::{validate_discovery_request, validate_port_request};
use crate::domain::{ConnectionSummary, TrafficSampleReport};
use crate::interfaces::sort_interfaces;
use crate::latency::validate_latency_target;
use crate::traffic::TrafficTracker;
use argus_core::application::services::{evaluate_latency_alerts, evaluate_throughput_alerts};
use argus_core::config::ArgusConfig;
use argus_core::domain::alert::Alert;
use argus_core::domain::network::{
    ConnectionInfo, DiscoveredHost, LatencyReport, NetworkInterface, NetworkMetric,
    PortCheckResult, TransportProtocol,
};
use argus_core::errors::{ArgusError, ArgusResult};
use argus_core::ports::network::NetworkMonitorPort;
use argus_core::ports::persistence::{MetricsRepositoryPort, NewNetworkMetric};
use chrono::Utc;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

/// Application service for all NetScope observability use cases.
pub struct NetScopeService {
    monitor: Arc<dyn NetworkMonitorPort>,
    repo: Option<Arc<dyn MetricsRepositoryPort>>,
    tracker: Mutex<TrafficTracker>,
}

impl NetScopeService {
    pub fn new(
        monitor: Arc<dyn NetworkMonitorPort>,
        repo: Option<Arc<dyn MetricsRepositoryPort>>,
    ) -> Self {
        Self {
            monitor,
            repo,
            tracker: Mutex::new(TrafficTracker::new()),
        }
    }

    fn ensure_enabled(&self, config: &ArgusConfig) -> ArgusResult<()> {
        if !config.modules.netscope_enabled {
            return Err(ArgusError::ModuleDisabled("netscope".to_string()));
        }
        Ok(())
    }

    /// Lists all detected interfaces sorted by status and traffic.
    pub fn list_interfaces(&self, config: &ArgusConfig) -> ArgusResult<Vec<NetworkInterface>> {
        self.ensure_enabled(config)?;
        let ifaces = self.monitor.list_interfaces()?;
        Ok(sort_interfaces(ifaces))
    }

    /// Captures a traffic sample, computes transfer rates vs. previous sample, evaluates alerts,
    /// and optionally persists metrics into SQLite.
    pub fn sample_traffic(&self, config: &ArgusConfig) -> ArgusResult<TrafficSampleReport> {
        self.ensure_enabled(config)?;
        let interfaces = sort_interfaces(self.monitor.list_interfaces()?);
        let rates = {
            let mut tracker = self
                .tracker
                .lock()
                .map_err(|_| ArgusError::AnalysisFailed("Tracker lock poisoned".to_string()))?;
            tracker.record(&interfaces)
        };
        let sampled_at = Utc::now();

        if let Some(repo) = &self.repo {
            if config.storage.database_enabled && config.netscope.history_enabled {
                for iface in &interfaces {
                    let metric = NewNetworkMetric {
                        interface_name: iface.name.clone(),
                        timestamp: sampled_at,
                        bytes_received: iface.bytes_received,
                        bytes_sent: iface.bytes_sent,
                        latency_ms: None,
                    };
                    if let Err(err) = repo.save_network_metric(&metric) {
                        warn!(error = %err, "Failed to persist network metric");
                    }
                }
                for rate in &rates {
                    for alert in evaluate_throughput_alerts(rate, &config.netscope) {
                        let _ = repo.save_alert(&alert);
                    }
                }
            }
        }

        Ok(TrafficSampleReport {
            sampled_at,
            interfaces,
            rates,
        })
    }

    /// Queries TCP/UDP connections and computes a summary.
    pub fn inspect_connections(
        &self,
        protocol: Option<TransportProtocol>,
        config: &ArgusConfig,
    ) -> ArgusResult<(Vec<ConnectionInfo>, ConnectionSummary)> {
        self.ensure_enabled(config)?;
        let raw = self.monitor.list_connections()?;
        let filtered = filter_connections(raw, protocol);
        let summary = summarize_connections(&filtered);
        Ok((filtered, summary))
    }

    /// Executes latency test against `target`, evaluates threshold alerts, and records metric.
    pub fn measure_latency(
        &self,
        target: &str,
        count: u32,
        timeout: Duration,
        port: Option<u16>,
        config: &ArgusConfig,
        cancel: &CancellationToken,
    ) -> ArgusResult<LatencyReport> {
        self.ensure_enabled(config)?;
        let sanitized = validate_latency_target(target)?;
        let probe_count = count.clamp(1, 20);
        info!(target = %sanitized, count = probe_count, "Measuring latency");

        let report = self
            .monitor
            .measure_latency(&sanitized, probe_count, timeout, port, cancel)?;

        if let Some(repo) = &self.repo {
            if config.storage.database_enabled && config.netscope.history_enabled {
                let metric = NewNetworkMetric {
                    interface_name: format!("probe:{}", report.target),
                    timestamp: report.measured_at,
                    bytes_received: 0,
                    bytes_sent: 0,
                    latency_ms: report.avg_ms,
                };
                let _ = repo.save_network_metric(&metric);

                for alert in evaluate_latency_alerts(&report, &config.netscope) {
                    let _ = repo.save_alert(&alert);
                }
            }
        }

        Ok(report)
    }

    /// Checks specified TCP ports on an explicitly authorized IPv4 host.
    pub fn check_authorized_ports(
        &self,
        host: &str,
        ports: &[u16],
        timeout: Duration,
        config: &ArgusConfig,
        cancel: &CancellationToken,
    ) -> ArgusResult<Vec<PortCheckResult>> {
        self.ensure_enabled(config)?;
        let authorized_ip = validate_port_request(host, ports, &config.netscope)?;
        let rate_limit = Duration::from_millis(config.netscope.probe_rate_limit_ms);
        self.monitor
            .check_ports(&authorized_ip, ports, timeout, rate_limit, cancel)
    }

    /// Discovers reachable hosts in an explicitly authorized CIDR block.
    pub fn discover_authorized_network(
        &self,
        cidr: &str,
        timeout: Duration,
        config: &ArgusConfig,
        cancel: &CancellationToken,
    ) -> ArgusResult<Vec<DiscoveredHost>> {
        self.ensure_enabled(config)?;
        validate_discovery_request(cidr, &config.netscope)?;
        let rate_limit = Duration::from_millis(config.netscope.probe_rate_limit_ms);
        self.monitor
            .discover_hosts(cidr, timeout, rate_limit, cancel)
    }

    /// Lists historical network metrics from SQLite if persistence is enabled.
    pub fn metric_history(
        &self,
        interface: Option<&str>,
        limit: usize,
    ) -> ArgusResult<Vec<NetworkMetric>> {
        match &self.repo {
            Some(repo) => repo.list_network_metrics(interface, limit),
            None => Ok(Vec::new()),
        }
    }

    /// Lists recorded alerts.
    pub fn list_alerts(&self, only_unack: bool, limit: usize) -> ArgusResult<Vec<Alert>> {
        match &self.repo {
            Some(repo) => repo.list_alerts(only_unack, limit),
            None => Ok(Vec::new()),
        }
    }
}
