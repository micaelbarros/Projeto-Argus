//! NS-001 & NS-002: Network interface filtering and sorting utilities.

use argus_core::domain::network::{InterfaceStatus, NetworkInterface};

/// Sorts interfaces prioritizing active (`Up`) interfaces and highest traffic volume.
pub fn sort_interfaces(mut interfaces: Vec<NetworkInterface>) -> Vec<NetworkInterface> {
    interfaces.sort_by(|a, b| {
        let rank = |s: InterfaceStatus| match s {
            InterfaceStatus::Up => 0,
            InterfaceStatus::Unknown => 1,
            InterfaceStatus::Down => 2,
        };
        rank(a.status)
            .cmp(&rank(b.status))
            .then_with(|| {
                (b.bytes_received + b.bytes_sent).cmp(&(a.bytes_received + a.bytes_sent))
            })
            .then_with(|| a.name.cmp(&b.name))
    });
    interfaces
}
