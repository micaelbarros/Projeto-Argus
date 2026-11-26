//! NetScope module — network interfaces, traffic rates, TCP/UDP connections, latency, and authorized discovery.

pub mod application;
pub mod connections;
pub mod discovery;
pub mod domain;
pub mod interfaces;
pub mod latency;
pub mod traffic;

pub use application::NetScopeService;
pub use domain::{AuthorizationPolicy, ConnectionSummary, TrafficSampleReport};
