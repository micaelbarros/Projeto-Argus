//! NS-005 & NS-006: TCP/UDP connection summarization and filtering.

use crate::domain::ConnectionSummary;
use argus_core::domain::network::{ConnectionInfo, TransportProtocol};

/// Summarizes active TCP and UDP sockets.
pub fn summarize_connections(connections: &[ConnectionInfo]) -> ConnectionSummary {
    let mut tcp_count = 0usize;
    let mut udp_count = 0usize;
    let mut established_count = 0usize;
    let mut listening_count = 0usize;

    for conn in connections {
        match conn.protocol {
            TransportProtocol::Tcp => tcp_count += 1,
            TransportProtocol::Udp => udp_count += 1,
        }
        let state_upper = conn.state.to_ascii_uppercase();
        if state_upper.contains("ESTAB") {
            established_count += 1;
        } else if state_upper.contains("LISTEN") {
            listening_count += 1;
        }
    }

    ConnectionSummary {
        total: connections.len(),
        tcp_count,
        udp_count,
        established_count,
        listening_count,
    }
}

/// Filters connections by optional transport protocol.
pub fn filter_connections(
    connections: Vec<ConnectionInfo>,
    protocol: Option<TransportProtocol>,
) -> Vec<ConnectionInfo> {
    let mut filtered: Vec<ConnectionInfo> = match protocol {
        Some(proto) => connections
            .into_iter()
            .filter(|c| c.protocol == proto)
            .collect(),
        None => connections,
    };
    filtered.sort_by(|a, b| {
        a.local_port
            .cmp(&b.local_port)
            .then_with(|| a.local_address.cmp(&b.local_address))
    });
    filtered
}
