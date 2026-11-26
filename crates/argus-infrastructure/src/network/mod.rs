//! SystemNetworkAdapter — cross-platform network observation without arbitrary shell commands.

use argus_core::application::services::compute_latency_stats;
use argus_core::domain::network::{
    ConnectionInfo, DiscoveredHost, InterfaceStatus, LatencyReport, LatencySample,
    NetworkInterface, PortCheckResult, ProbeMethod, TransportProtocol,
};
use argus_core::errors::{ArgusError, ArgusResult};
use argus_core::ports::network::NetworkMonitorPort;
use chrono::Utc;
use netstat2::{get_sockets_info, AddressFamilyFlags, ProtocolFlags, ProtocolSocketInfo, TcpState};
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream, ToSocketAddrs};
use std::thread;
use std::time::{Duration, Instant};
use sysinfo::{Networks, ProcessesToUpdate, System};
use tokio_util::sync::CancellationToken;

/// Concrete adapter implementing `NetworkMonitorPort` via native OS APIs (`sysinfo`, `netstat2`, sockets).
#[derive(Debug, Default, Clone)]
pub struct SystemNetworkAdapter;

impl SystemNetworkAdapter {
    pub fn new() -> Self {
        Self
    }

    fn resolve_target_addr(target: &str, port: u16) -> ArgusResult<SocketAddr> {
        if let Ok(ip) = target.parse::<IpAddr>() {
            return Ok(SocketAddr::new(ip, port));
        }
        let lookup = format!("{target}:{port}");
        let mut addrs = lookup.to_socket_addrs().map_err(|e| {
            ArgusError::NetworkUnavailable(format!(
                "Não foi possível resolver o host '{target}': {e}"
            ))
        })?;
        addrs.next().ok_or_else(|| {
            ArgusError::NetworkUnavailable(format!("Nenhum endereço IP retornado para '{target}'."))
        })
    }

    fn tcp_state_label(state: TcpState) -> String {
        format!("{state:?}").to_ascii_uppercase()
    }

    fn parse_cidr_hosts(cidr: &str) -> ArgusResult<Vec<Ipv4Addr>> {
        let (ip_str, prefix_str) = cidr
            .split_once('/')
            .ok_or_else(|| ArgusError::ConfigurationError(format!("CIDR inválido: '{cidr}'")))?;
        let base: Ipv4Addr = ip_str.parse().map_err(|_| {
            ArgusError::ConfigurationError(format!("IPv4 inválido no CIDR: '{cidr}'"))
        })?;
        let prefix: u8 = prefix_str.parse().map_err(|_| {
            ArgusError::ConfigurationError(format!("Prefixo inválido no CIDR: '{cidr}'"))
        })?;
        if !(24..=32).contains(&prefix) {
            return Err(ArgusError::PermissionDenied(
                "Descoberta restrita a blocos entre /24 e /32.".to_string(),
            ));
        }

        let mask = if prefix == 0 {
            0
        } else {
            !0u32 << (32 - prefix)
        };
        let network = u32::from(base) & mask;
        let host_bits = 32 - prefix;
        let count = 1u32 << host_bits;

        if prefix == 32 {
            return Ok(vec![base]);
        }
        if prefix == 31 {
            return Ok(vec![Ipv4Addr::from(network), Ipv4Addr::from(network + 1)]);
        }

        let mut hosts = Vec::with_capacity(count.saturating_sub(2) as usize);
        for offset in 1..(count - 1) {
            hosts.push(Ipv4Addr::from(network + offset));
        }
        Ok(hosts)
    }
}

impl NetworkMonitorPort for SystemNetworkAdapter {
    fn list_interfaces(&self) -> ArgusResult<Vec<NetworkInterface>> {
        let networks = Networks::new_with_refreshed_list();
        let mut out = Vec::new();

        for (name, data) in &networks {
            let ips: Vec<String> = data
                .ip_networks()
                .iter()
                .map(|net| format!("{}/{}", net.addr, net.prefix))
                .collect();
            let mac = data.mac_address().to_string();
            let mac_opt = if mac == "00:00:00:00:00:00" || mac.is_empty() {
                None
            } else {
                Some(mac)
            };

            let status =
                if !ips.is_empty() || data.total_received() > 0 || data.total_transmitted() > 0 {
                    InterfaceStatus::Up
                } else {
                    InterfaceStatus::Down
                };

            out.push(NetworkInterface {
                name: name.to_string(),
                status,
                mac_address: mac_opt,
                ip_addresses: ips,
                bytes_received: data.total_received(),
                bytes_sent: data.total_transmitted(),
                packets_received: data.total_packets_received(),
                packets_sent: data.total_packets_transmitted(),
                errors_in: data.total_errors_on_received(),
                errors_out: data.total_errors_on_transmitted(),
            });
        }

        Ok(out)
    }

    fn list_connections(&self) -> ArgusResult<Vec<ConnectionInfo>> {
        let af_flags = AddressFamilyFlags::IPV4 | AddressFamilyFlags::IPV6;
        let proto_flags = ProtocolFlags::TCP | ProtocolFlags::UDP;
        let sockets = get_sockets_info(af_flags, proto_flags).map_err(|e| {
            ArgusError::PermissionDenied(format!(
                "Não foi possível consultar sockets do sistema: {e}"
            ))
        })?;

        let mut sys = System::new();
        sys.refresh_processes(ProcessesToUpdate::All, true);
        let pid_map: HashMap<u32, String> = sys
            .processes()
            .iter()
            .map(|(pid, proc_info)| {
                (
                    pid.as_u32(),
                    proc_info.name().to_string_lossy().into_owned(),
                )
            })
            .collect();

        let mut out = Vec::with_capacity(sockets.len());
        for si in sockets {
            let pid = si.associated_pids.first().copied();
            let process_name = pid.and_then(|p| pid_map.get(&p).cloned());

            match si.protocol_socket_info {
                ProtocolSocketInfo::Tcp(tcp) => {
                    out.push(ConnectionInfo {
                        protocol: TransportProtocol::Tcp,
                        local_address: tcp.local_addr.to_string(),
                        local_port: tcp.local_port,
                        remote_address: Some(tcp.remote_addr.to_string()),
                        remote_port: Some(tcp.remote_port),
                        state: Self::tcp_state_label(tcp.state),
                        pid,
                        process_name,
                    });
                }
                ProtocolSocketInfo::Udp(udp) => {
                    out.push(ConnectionInfo {
                        protocol: TransportProtocol::Udp,
                        local_address: udp.local_addr.to_string(),
                        local_port: udp.local_port,
                        remote_address: None,
                        remote_port: None,
                        state: "UNCONNECTED".to_string(),
                        pid,
                        process_name,
                    });
                }
            }
        }

        Ok(out)
    }

    fn measure_latency(
        &self,
        target: &str,
        count: u32,
        timeout: Duration,
        port: Option<u16>,
        cancel: &CancellationToken,
    ) -> ArgusResult<LatencyReport> {
        let probe_port = port.unwrap_or(443);
        let resolved = Self::resolve_target_addr(target, probe_port)?;
        let resolved_ip = resolved.ip().to_string();

        let mut samples = Vec::with_capacity(count as usize);
        for seq in 1..=count {
            if cancel.is_cancelled() {
                return Err(ArgusError::Cancelled);
            }
            let start = Instant::now();
            match TcpStream::connect_timeout(&resolved, timeout) {
                Ok(_stream) => {
                    let rtt = start.elapsed().as_secs_f64() * 1000.0;
                    samples.push(LatencySample {
                        sequence: seq,
                        rtt_ms: Some((rtt * 100.0).round() / 100.0),
                        error: None,
                    });
                }
                Err(err) => {
                    // On localhost/LAN, ConnectionRefused still proves host reachability & RTT
                    if err.kind() == std::io::ErrorKind::ConnectionRefused && port.is_none() {
                        let rtt = start.elapsed().as_secs_f64() * 1000.0;
                        samples.push(LatencySample {
                            sequence: seq,
                            rtt_ms: Some((rtt * 100.0).round() / 100.0),
                            error: Some("TCP RST recebido (host acessível)".to_string()),
                        });
                    } else {
                        samples.push(LatencySample {
                            sequence: seq,
                            rtt_ms: None,
                            error: Some(err.to_string()),
                        });
                    }
                }
            }

            if seq < count && !cancel.is_cancelled() {
                thread::sleep(Duration::from_millis(200));
            }
        }

        let stats = compute_latency_stats(&samples);
        Ok(LatencyReport {
            target: target.to_string(),
            resolved_ip: Some(resolved_ip),
            method: ProbeMethod::TcpConnect(probe_port),
            sent: stats.sent,
            received: stats.received,
            packet_loss_pct: stats.packet_loss_pct,
            min_ms: stats.min_ms,
            avg_ms: stats.avg_ms,
            max_ms: stats.max_ms,
            samples,
            privilege_note: Some(
                "Sondagem TCP sem privilégios administrativos (princípio do menor privilégio)."
                    .to_string(),
            ),
            measured_at: Utc::now(),
        })
    }

    fn check_ports(
        &self,
        host: &str,
        ports: &[u16],
        timeout: Duration,
        rate_limit: Duration,
        cancel: &CancellationToken,
    ) -> ArgusResult<Vec<PortCheckResult>> {
        let ip: IpAddr = host.parse().map_err(|_| {
            ArgusError::NetworkUnavailable(format!("IP inválido para verificação: '{host}'"))
        })?;
        let mut results = Vec::with_capacity(ports.len());

        for (idx, &port) in ports.iter().enumerate() {
            if cancel.is_cancelled() {
                return Err(ArgusError::Cancelled);
            }
            let addr = SocketAddr::new(ip, port);
            let start = Instant::now();
            let open = TcpStream::connect_timeout(&addr, timeout).is_ok();
            let latency_ms = if open {
                Some(((start.elapsed().as_secs_f64() * 1000.0) * 100.0).round() / 100.0)
            } else {
                None
            };
            results.push(PortCheckResult {
                host: host.to_string(),
                port,
                open,
                latency_ms,
            });
            if idx + 1 < ports.len() && !rate_limit.is_zero() {
                thread::sleep(rate_limit);
            }
        }

        Ok(results)
    }

    fn discover_hosts(
        &self,
        cidr: &str,
        timeout: Duration,
        rate_limit: Duration,
        cancel: &CancellationToken,
    ) -> ArgusResult<Vec<DiscoveredHost>> {
        let ips = Self::parse_cidr_hosts(cidr)?;
        let common_ports: [u16; 3] = [80, 443, 22];
        let mut discovered = Vec::new();

        for (idx, ip) in ips.iter().enumerate() {
            if cancel.is_cancelled() {
                return Err(ArgusError::Cancelled);
            }
            let mut open_ports = Vec::new();
            let mut best_rtt: Option<f64> = None;
            let mut reachable = false;

            for &port in &common_ports {
                let addr = SocketAddr::new(IpAddr::V4(*ip), port);
                let start = Instant::now();
                match TcpStream::connect_timeout(&addr, timeout) {
                    Ok(_) => {
                        let rtt = (start.elapsed().as_secs_f64() * 1000.0 * 100.0).round() / 100.0;
                        reachable = true;
                        open_ports.push(port);
                        best_rtt = Some(best_rtt.map_or(rtt, |prev| prev.min(rtt)));
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::ConnectionRefused => {
                        let rtt = (start.elapsed().as_secs_f64() * 1000.0 * 100.0).round() / 100.0;
                        reachable = true;
                        best_rtt = Some(best_rtt.map_or(rtt, |prev| prev.min(rtt)));
                    }
                    Err(_) => {}
                }
            }

            if reachable {
                discovered.push(DiscoveredHost {
                    ip: ip.to_string(),
                    reachable: true,
                    rtt_ms: best_rtt,
                    open_ports,
                });
            }

            if idx + 1 < ips.len() && !rate_limit.is_zero() {
                thread::sleep(rate_limit);
            }
        }

        Ok(discovered)
    }
}
