//! NetScope domain policies and aggregated monitoring DTOs.

use argus_core::domain::network::{NetworkInterface, TransferRate};
use argus_core::errors::{ArgusError, ArgusResult};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::net::Ipv4Addr;

/// Combined traffic snapshot and calculated transfer rates.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrafficSampleReport {
    pub sampled_at: DateTime<Utc>,
    pub interfaces: Vec<NetworkInterface>,
    pub rates: Vec<TransferRate>,
}

/// Aggregated TCP/UDP connection statistics.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConnectionSummary {
    pub total: usize,
    pub tcp_count: usize,
    pub udp_count: usize,
    pub established_count: usize,
    pub listening_count: usize,
}

/// Security policy ensuring active network discovery and port verification only target
/// explicitly authorized IPv4 CIDRs configured in `config.toml` (Security §8.3).
#[derive(Debug, Clone)]
pub struct AuthorizationPolicy {
    allowed_cidrs: Vec<(Ipv4Addr, u8)>,
}

impl AuthorizationPolicy {
    /// Parses CIDR strings such as `127.0.0.1/32` or `192.168.1.0/24`.
    pub fn from_cidrs(cidrs: &[String]) -> ArgusResult<Self> {
        let mut parsed = Vec::new();
        for raw in cidrs {
            parsed.push(parse_ipv4_cidr(raw)?);
        }
        Ok(Self {
            allowed_cidrs: parsed,
        })
    }

    /// Returns `true` if `ip` falls inside at least one authorized CIDR.
    pub fn is_ip_authorized(&self, ip: Ipv4Addr) -> bool {
        let target = u32::from(ip);
        self.allowed_cidrs.iter().any(|&(net, prefix)| {
            let mask = prefix_to_mask(prefix);
            (target & mask) == (u32::from(net) & mask)
        })
    }

    /// Validates that a requested CIDR is completely contained within an authorized CIDR
    /// and is bounded to at most `/24` (254 hosts) to avoid aggressive network scans.
    pub fn validate_discovery_cidr(&self, cidr: &str) -> ArgusResult<(Ipv4Addr, u8)> {
        let (net, prefix) = parse_ipv4_cidr(cidr)?;
        if prefix < 24 {
            return Err(ArgusError::PermissionDenied(format!(
                "Prefixo /{prefix} excede o limite seguro de descoberta (/24 ou menor)."
            )));
        }
        let authorized = self.allowed_cidrs.iter().any(|&(allowed_net, allowed_prefix)| {
            if prefix < allowed_prefix {
                return false;
            }
            let mask = prefix_to_mask(allowed_prefix);
            (u32::from(net) & mask) == (u32::from(allowed_net) & mask)
        });
        if !authorized {
            return Err(ArgusError::PermissionDenied(format!(
                "Alvo '{cidr}' não está autorizado em netscope.authorized_cidrs."
            )));
        }
        Ok((net, prefix))
    }

    /// Validates that a single IPv4 host string is authorized for port checks.
    pub fn validate_host(&self, host: &str) -> ArgusResult<Ipv4Addr> {
        let ip = if host.eq_ignore_ascii_case("localhost") {
            Ipv4Addr::LOCALHOST
        } else {
            host.parse::<Ipv4Addr>().map_err(|_| {
                ArgusError::PermissionDenied(format!(
                    "Verificação de portas exige IPv4 explícito autorizado (recebido: '{host}')."
                ))
            })?
        };
        if !self.is_ip_authorized(ip) {
            return Err(ArgusError::PermissionDenied(format!(
                "Host '{ip}' não está na lista netscope.authorized_cidrs."
            )));
        }
        Ok(ip)
    }
}

/// Parses an IPv4 CIDR string (`a.b.c.d/prefix`).
pub fn parse_ipv4_cidr(cidr: &str) -> ArgusResult<(Ipv4Addr, u8)> {
    let (ip_part, prefix_part) = cidr.split_once('/').ok_or_else(|| {
        ArgusError::ConfigurationError(format!("CIDR inválido (esperado IP/prefixo): '{cidr}'"))
    })?;
    let ip: Ipv4Addr = ip_part.parse().map_err(|_| {
        ArgusError::ConfigurationError(format!("Endereço IPv4 inválido no CIDR: '{cidr}'"))
    })?;
    let prefix: u8 = prefix_part.parse().map_err(|_| {
        ArgusError::ConfigurationError(format!("Prefixo inválido no CIDR: '{cidr}'"))
    })?;
    if prefix > 32 {
        return Err(ArgusError::ConfigurationError(format!(
            "Prefixo CIDR fora do intervalo 0..=32: '{cidr}'"
        )));
    }
    Ok((ip, prefix))
}

fn prefix_to_mask(prefix: u8) -> u32 {
    if prefix == 0 {
        0
    } else {
        !0u32 << (32 - prefix)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enforces_authorized_cidrs() {
        let policy = AuthorizationPolicy::from_cidrs(&[
            "127.0.0.1/32".to_string(),
            "192.168.10.0/24".to_string(),
        ])
        .unwrap();

        assert!(policy.validate_host("127.0.0.1").is_ok());
        assert!(policy.validate_host("localhost").is_ok());
        assert!(policy.validate_host("192.168.10.42").is_ok());
        assert!(policy.validate_host("8.8.8.8").is_err());

        assert!(policy.validate_discovery_cidr("192.168.10.0/28").is_ok());
        assert!(policy.validate_discovery_cidr("10.0.0.0/24").is_err());
        assert!(policy.validate_discovery_cidr("192.168.0.0/16").is_err());
    }
}
