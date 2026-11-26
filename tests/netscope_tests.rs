//! Integration tests for NetScope interfaces, rates, authorization guardrails, and persistence.

use argus::core::config::ArgusConfig;
use argus::core::ports::persistence::MetricsRepositoryPort;
use argus::infrastructure::{SQLiteMetricsAdapter, SystemNetworkAdapter};
use argus::netscope::NetScopeService;
use std::net::TcpListener;
use std::sync::Arc;
use std::time::Duration;
use tokio_util::sync::CancellationToken;

#[test]
fn lists_interfaces_and_computes_transfer_rates_with_persistence() {
    let adapter = Arc::new(SystemNetworkAdapter::new());
    let db = Arc::new(SQLiteMetricsAdapter::open_in_memory().unwrap());
    let repo: Arc<dyn MetricsRepositoryPort> = db.clone();

    let service = NetScopeService::new(adapter, Some(repo));
    let config = ArgusConfig::default();

    let sample1 = service.sample_traffic(&config).unwrap();
    assert!(!sample1.interfaces.is_empty());
    // First sample establishes baseline
    assert!(sample1.rates.is_empty());

    std::thread::sleep(Duration::from_millis(50));
    let sample2 = service.sample_traffic(&config).unwrap();
    assert_eq!(sample2.rates.len(), sample2.interfaces.len());
    for rate in &sample2.rates {
        assert!(rate.rx_bytes_per_sec >= 0.0);
        assert!(rate.tx_bytes_per_sec >= 0.0);
    }

    let history = service.metric_history(None, 100).unwrap();
    assert!(!history.is_empty());
}

#[test]
fn enforces_authorized_host_policy_and_checks_local_listener_port() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let open_port = listener.local_addr().unwrap().port();

    let adapter = Arc::new(SystemNetworkAdapter::new());
    let service = NetScopeService::new(adapter, None);
    let config = ArgusConfig::default(); // default authorizes 127.0.0.1/32 only
    let cancel = CancellationToken::new();

    // Unauthorized IP must be rejected
    let denied = service.check_authorized_ports(
        "8.8.8.8",
        &[443],
        Duration::from_millis(200),
        &config,
        &cancel,
    );
    assert!(denied.is_err());

    // Authorized localhost port must be detected as open
    let results = service
        .check_authorized_ports(
            "127.0.0.1",
            &[open_port],
            Duration::from_millis(500),
            &config,
            &cancel,
        )
        .unwrap();
    assert_eq!(results.len(), 1);
    assert!(results[0].open);
    assert!(results[0].latency_ms.is_some());

    // Latency probe against local bound port succeeds with 0% packet loss
    let latency = service
        .measure_latency(
            "127.0.0.1",
            2,
            Duration::from_millis(500),
            Some(open_port),
            &config,
            &cancel,
        )
        .unwrap();
    assert_eq!(latency.sent, 2);
    assert_eq!(latency.received, 2);
    assert!((latency.packet_loss_pct - 0.0).abs() < 1e-6);
    assert!(latency.avg_ms.is_some());
}
