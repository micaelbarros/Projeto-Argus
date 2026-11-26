//! NS-010 & NS-011: Authorized host discovery and port verification guardrails.

use crate::domain::AuthorizationPolicy;
use argus_core::config::NetScopeConfig;
use argus_core::errors::{ArgusError, ArgusResult};

const MAX_PORTS_PER_CHECK: usize = 64;

/// Ensures port list is bounded and non-empty before probing an authorized host.
pub fn validate_port_request(
    host: &str,
    ports: &[u16],
    cfg: &NetScopeConfig,
) -> ArgusResult<String> {
    if ports.is_empty() {
        return Err(ArgusError::ConfigurationError(
            "Informe ao menos uma porta para verificação.".to_string(),
        ));
    }
    if ports.len() > MAX_PORTS_PER_CHECK {
        return Err(ArgusError::PermissionDenied(format!(
            "Máximo de {MAX_PORTS_PER_CHECK} portas por verificação para evitar sondagens agressivas."
        )));
    }
    let policy = AuthorizationPolicy::from_cidrs(&cfg.authorized_cidrs)?;
    let ip = policy.validate_host(host)?;
    Ok(ip.to_string())
}

/// Ensures CIDR discovery request is authorized in configuration.
pub fn validate_discovery_request(cidr: &str, cfg: &NetScopeConfig) -> ArgusResult<()> {
    let policy = AuthorizationPolicy::from_cidrs(&cfg.authorized_cidrs)?;
    let _ = policy.validate_discovery_cidr(cidr)?;
    Ok(())
}
