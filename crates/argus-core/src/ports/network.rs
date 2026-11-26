//! NetworkMonitorPort — cross-platform network observation and diagnostics.

use crate::domain::network::{
    ConnectionInfo, DiscoveredHost, LatencyReport, NetworkInterface, PortCheckResult,
};
use crate::errors::ArgusResult;
use std::time::Duration;
use tokio_util::sync::CancellationToken;

/// Port abstracting OS network interfaces, connections, latency probes, and authorized scans.
pub trait NetworkMonitorPort: Send + Sync {
    /// Lists currently available network interfaces and cumulative traffic counters.
    fn list_interfaces(&self) -> ArgusResult<Vec<NetworkInterface>>;

    /// Queries active TCP/UDP sockets and, where permitted by OS privileges, owning processes.
    fn list_connections(&self) -> ArgusResult<Vec<ConnectionInfo>>;

    /// Executes `count` latency probes against `target` with per-probe `timeout`.
    fn measure_latency(
        &self,
        target: &str,
        count: u32,
        timeout: Duration,
        port: Option<u16>,
        cancel: &CancellationToken,
    ) -> ArgusResult<LatencyReport>;

    /// Checks a bounded list of TCP ports on an authorized host with rate limiting.
    fn check_ports(
        &self,
        host: &str,
        ports: &[u16],
        timeout: Duration,
        rate_limit: Duration,
        cancel: &CancellationToken,
    ) -> ArgusResult<Vec<PortCheckResult>>;

    /// Discovers reachable hosts within an authorized IPv4 CIDR block with rate limiting.
    fn discover_hosts(
        &self,
        cidr: &str,
        timeout: Duration,
        rate_limit: Duration,
        cancel: &CancellationToken,
    ) -> ArgusResult<Vec<DiscoveredHost>>;
}
