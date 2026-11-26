//! Network and NetScope domain models.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Operational state of a network interface.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum InterfaceStatus {
    Up,
    Down,
    Unknown,
}

impl std::fmt::Display for InterfaceStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Up => write!(f, "UP"),
            Self::Down => write!(f, "DOWN"),
            Self::Unknown => write!(f, "UNKNOWN"),
        }
    }
}

/// Network interface snapshot with cumulative traffic counters.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NetworkInterface {
    pub name: String,
    pub status: InterfaceStatus,
    pub mac_address: Option<String>,
    pub ip_addresses: Vec<String>,
    pub bytes_received: u64,
    pub bytes_sent: u64,
    pub packets_received: u64,
    pub packets_sent: u64,
    pub errors_in: u64,
    pub errors_out: u64,
}

/// Calculated transfer rate between two traffic snapshots.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TransferRate {
    pub interface_name: String,
    pub rx_bytes_per_sec: f64,
    pub tx_bytes_per_sec: f64,
    pub rx_mbps: f64,
    pub tx_mbps: f64,
    pub interval_secs: f64,
}

/// Persisted network metric point for SQLite `network_metrics`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NetworkMetric {
    pub id: i64,
    pub interface_name: String,
    pub timestamp: DateTime<Utc>,
    pub bytes_received: u64,
    pub bytes_sent: u64,
    pub latency_ms: Option<f64>,
}

/// Transport layer protocol.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum TransportProtocol {
    Tcp,
    Udp,
}

impl std::fmt::Display for TransportProtocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Tcp => write!(f, "TCP"),
            Self::Udp => write!(f, "UDP"),
        }
    }
}

/// Active TCP/UDP socket entry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConnectionInfo {
    pub protocol: TransportProtocol,
    pub local_address: String,
    pub local_port: u16,
    pub remote_address: Option<String>,
    pub remote_port: Option<u16>,
    pub state: String,
    pub pid: Option<u32>,
    pub process_name: Option<String>,
}

/// Method used to measure host latency.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ProbeMethod {
    Icmp,
    TcpConnect(u16),
}

impl std::fmt::Display for ProbeMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Icmp => write!(f, "ICMP"),
            Self::TcpConnect(port) => write!(f, "TCP:{port}"),
        }
    }
}

/// Single latency probe result.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LatencySample {
    pub sequence: u32,
    pub rtt_ms: Option<f64>,
    pub error: Option<String>,
}

/// Aggregated latency test report (min/avg/max + packet loss).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LatencyReport {
    pub target: String,
    pub resolved_ip: Option<String>,
    pub method: ProbeMethod,
    pub sent: u32,
    pub received: u32,
    pub packet_loss_pct: f64,
    pub min_ms: Option<f64>,
    pub avg_ms: Option<f64>,
    pub max_ms: Option<f64>,
    pub samples: Vec<LatencySample>,
    pub privilege_note: Option<String>,
    pub measured_at: DateTime<Utc>,
}

/// Host discovered in an authorized CIDR range.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DiscoveredHost {
    pub ip: String,
    pub reachable: bool,
    pub rtt_ms: Option<f64>,
    pub open_ports: Vec<u16>,
}

/// Single port check outcome on an authorized host.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PortCheckResult {
    pub host: String,
    pub port: u16,
    pub open: bool,
    pub latency_ms: Option<f64>,
}
