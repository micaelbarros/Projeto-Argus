//! NS-003 & NS-004: Traffic counters and transfer-rate tracking across intervals.

use argus_core::application::services::calculate_transfer_rate;
use argus_core::domain::network::{NetworkInterface, TransferRate};
use std::collections::HashMap;
use std::time::Instant;

/// Stateful tracker computing live RX/TX transfer rates between consecutive interface snapshots.
#[derive(Debug, Default)]
pub struct TrafficTracker {
    last_snapshot: HashMap<String, NetworkInterface>,
    last_instant: Option<Instant>,
}

impl TrafficTracker {
    pub fn new() -> Self {
        Self::default()
    }

    /// Updates internal snapshot and returns computed rates since the previous call.
    pub fn record(&mut self, current: &[NetworkInterface]) -> Vec<TransferRate> {
        let now = Instant::now();
        let interval_secs = self
            .last_instant
            .map(|prev| now.duration_since(prev).as_secs_f64().max(0.001))
            .unwrap_or(1.0);

        let mut rates = Vec::new();
        if self.last_instant.is_some() {
            for iface in current {
                if let Some(prev) = self.last_snapshot.get(&iface.name) {
                    rates.push(calculate_transfer_rate(prev, iface, interval_secs));
                }
            }
        }

        self.last_snapshot.clear();
        for iface in current {
            self.last_snapshot.insert(iface.name.clone(), iface.clone());
        }
        self.last_instant = Some(now);

        rates.sort_by(|a, b| {
            (b.rx_bytes_per_sec + b.tx_bytes_per_sec)
                .partial_cmp(&(a.rx_bytes_per_sec + a.tx_bytes_per_sec))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        rates
    }
}
