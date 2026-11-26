//! Pure domain calculations shared across modules (transfer rate, latency stats, alerts).

use crate::config::NetScopeConfig;
use crate::domain::alert::{AlertModule, AlertSeverity};
use crate::domain::network::{LatencyReport, LatencySample, NetworkInterface, TransferRate};
use crate::ports::persistence::NewAlert;
use chrono::Utc;

/// Computes non-negative transfer rates between two snapshots of the same interface.
/// Counter resets/wraps are handled safely via saturating subtraction.
pub fn calculate_transfer_rate(
    previous: &NetworkInterface,
    current: &NetworkInterface,
    interval_secs: f64,
) -> TransferRate {
    let dt = if interval_secs > 0.0 && interval_secs.is_finite() {
        interval_secs
    } else {
        1.0
    };
    let rx_delta = current
        .bytes_received
        .saturating_sub(previous.bytes_received) as f64;
    let tx_delta = current.bytes_sent.saturating_sub(previous.bytes_sent) as f64;
    let rx_bps = (rx_delta / dt).max(0.0);
    let tx_bps = (tx_delta / dt).max(0.0);

    TransferRate {
        interface_name: current.name.clone(),
        rx_bytes_per_sec: rx_bps,
        tx_bytes_per_sec: tx_bps,
        rx_mbps: (rx_bps * 8.0) / 1_000_000.0,
        tx_mbps: (tx_bps * 8.0) / 1_000_000.0,
        interval_secs: dt,
    }
}

/// Summary statistics for a slice of RTT samples in milliseconds.
#[derive(Debug, Clone, PartialEq)]
pub struct LatencyStats {
    pub sent: u32,
    pub received: u32,
    pub packet_loss_pct: f64,
    pub min_ms: Option<f64>,
    pub avg_ms: Option<f64>,
    pub max_ms: Option<f64>,
}

/// Computes sent/received counts, packet loss %, min, avg, and max latency.
pub fn compute_latency_stats(samples: &[LatencySample]) -> LatencyStats {
    let sent = samples.len() as u32;
    if sent == 0 {
        return LatencyStats {
            sent: 0,
            received: 0,
            packet_loss_pct: 0.0,
            min_ms: None,
            avg_ms: None,
            max_ms: None,
        };
    }

    let mut min_v = f64::INFINITY;
    let mut max_v = f64::NEG_INFINITY;
    let mut sum = 0.0_f64;
    let mut received = 0u32;

    for sample in samples {
        if let Some(rtt) = sample.rtt_ms {
            if rtt.is_finite() && rtt >= 0.0 {
                received += 1;
                sum += rtt;
                if rtt < min_v {
                    min_v = rtt;
                }
                if rtt > max_v {
                    max_v = rtt;
                }
            }
        }
    }

    let lost = sent.saturating_sub(received);
    let packet_loss_pct = (f64::from(lost) / f64::from(sent)) * 100.0;

    if received == 0 {
        LatencyStats {
            sent,
            received: 0,
            packet_loss_pct,
            min_ms: None,
            avg_ms: None,
            max_ms: None,
        }
    } else {
        let avg = (sum / f64::from(received)).clamp(min_v, max_v);
        LatencyStats {
            sent,
            received,
            packet_loss_pct,
            min_ms: Some(min_v),
            avg_ms: Some(avg),
            max_ms: Some(max_v),
        }
    }
}

/// Evaluates NetScope latency thresholds and emits alerts when limits are exceeded.
pub fn evaluate_latency_alerts(report: &LatencyReport, cfg: &NetScopeConfig) -> Vec<NewAlert> {
    let mut alerts = Vec::new();
    let now = Utc::now();

    if report.packet_loss_pct > cfg.packet_loss_threshold_pct {
        let severity = if report.packet_loss_pct >= 50.0 {
            AlertSeverity::Critical
        } else {
            AlertSeverity::Warning
        };
        alerts.push(NewAlert {
            module: AlertModule::NetScope,
            severity,
            message: format!(
                "Perda de pacotes elevada para {}: {:.1}% (limite: {:.1}%)",
                report.target, report.packet_loss_pct, cfg.packet_loss_threshold_pct
            ),
            created_at: now,
        });
    }

    if let Some(avg) = report.avg_ms {
        if avg > cfg.latency_threshold_ms {
            let severity = if avg > cfg.latency_threshold_ms * 2.0 {
                AlertSeverity::Critical
            } else {
                AlertSeverity::Warning
            };
            alerts.push(NewAlert {
                module: AlertModule::NetScope,
                severity,
                message: format!(
                    "Latência média elevada para {}: {:.2} ms (limite: {:.1} ms)",
                    report.target, avg, cfg.latency_threshold_ms
                ),
                created_at: now,
            });
        }
    }

    alerts
}

/// Evaluates transfer-rate thresholds and emits alerts when throughput exceeds configured ceiling.
pub fn evaluate_throughput_alerts(rate: &TransferRate, cfg: &NetScopeConfig) -> Vec<NewAlert> {
    let mut alerts = Vec::new();
    let total_mbps = rate.rx_mbps + rate.tx_mbps;
    if total_mbps > cfg.throughput_alert_mbps {
        alerts.push(NewAlert {
            module: AlertModule::NetScope,
            severity: AlertSeverity::Warning,
            message: format!(
                "Taxa de transferência alta na interface {}: {:.2} Mbps (limite: {:.1} Mbps)",
                rate.interface_name, total_mbps, cfg.throughput_alert_mbps
            ),
            created_at: Utc::now(),
        });
    }
    alerts
}

/// Formats a byte count into a human-readable IEC/SI string.
pub fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut value = bytes as f64;
    let mut unit_idx = 0usize;
    while value >= 1024.0 && unit_idx < UNITS.len() - 1 {
        value /= 1024.0;
        unit_idx += 1;
    }
    if unit_idx == 0 {
        format!("{bytes} B")
    } else {
        format!("{value:.2} {}", UNITS[unit_idx])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::network::InterfaceStatus;
    use proptest::prelude::*;

    fn sample_iface(rx: u64, tx: u64) -> NetworkInterface {
        NetworkInterface {
            name: "eth0".to_string(),
            status: InterfaceStatus::Up,
            mac_address: None,
            ip_addresses: vec!["127.0.0.1".to_string()],
            bytes_received: rx,
            bytes_sent: tx,
            packets_received: 0,
            packets_sent: 0,
            errors_in: 0,
            errors_out: 0,
        }
    }

    #[test]
    fn computes_transfer_rate_correctly() {
        let prev = sample_iface(1_000, 500);
        let curr = sample_iface(3_000, 1_500);
        let rate = calculate_transfer_rate(&prev, &curr, 2.0);
        assert!((rate.rx_bytes_per_sec - 1_000.0).abs() < 1e-6);
        assert!((rate.tx_bytes_per_sec - 500.0).abs() < 1e-6);
    }

    #[test]
    fn handles_counter_reset_without_negative_rate() {
        let prev = sample_iface(10_000, 5_000);
        let curr = sample_iface(200, 100);
        let rate = calculate_transfer_rate(&prev, &curr, 1.0);
        assert_eq!(rate.rx_bytes_per_sec, 0.0);
        assert_eq!(rate.tx_bytes_per_sec, 0.0);
    }

    #[test]
    fn computes_latency_stats_with_packet_loss() {
        let samples = vec![
            LatencySample {
                sequence: 1,
                rtt_ms: Some(10.0),
                error: None,
            },
            LatencySample {
                sequence: 2,
                rtt_ms: None,
                error: Some("timeout".to_string()),
            },
            LatencySample {
                sequence: 3,
                rtt_ms: Some(30.0),
                error: None,
            },
        ];
        let stats = compute_latency_stats(&samples);
        assert_eq!(stats.sent, 3);
        assert_eq!(stats.received, 2);
        assert!((stats.packet_loss_pct - 33.333333).abs() < 0.01);
        assert_eq!(stats.min_ms, Some(10.0));
        assert_eq!(stats.avg_ms, Some(20.0));
        assert_eq!(stats.max_ms, Some(30.0));
    }

    proptest! {
        #[test]
        fn transfer_rate_is_never_negative(
            prev_rx in any::<u64>(),
            curr_rx in any::<u64>(),
            prev_tx in any::<u64>(),
            curr_tx in any::<u64>(),
            dt in 0.001f64..3600.0,
        ) {
            let prev = sample_iface(prev_rx, prev_tx);
            let curr = sample_iface(curr_rx, curr_tx);
            let rate = calculate_transfer_rate(&prev, &curr, dt);
            prop_assert!(rate.rx_bytes_per_sec >= 0.0);
            prop_assert!(rate.tx_bytes_per_sec >= 0.0);
            prop_assert!(rate.rx_mbps >= 0.0);
            prop_assert!(rate.tx_mbps >= 0.0);
        }

        #[test]
        fn latency_avg_always_between_min_and_max(
            rtts in prop::collection::vec(0.0f64..10_000.0, 1..50),
        ) {
            let samples: Vec<LatencySample> = rtts
                .iter()
                .enumerate()
                .map(|(idx, &v)| LatencySample {
                    sequence: idx as u32 + 1,
                    rtt_ms: Some(v),
                    error: None,
                })
                .collect();
            let stats = compute_latency_stats(&samples);
            let min = stats.min_ms.unwrap();
            let avg = stats.avg_ms.unwrap();
            let max = stats.max_ms.unwrap();
            prop_assert!(avg >= min);
            prop_assert!(avg <= max);
            prop_assert!((stats.packet_loss_pct - 0.0).abs() < 1e-9);
        }
    }
}
