//! Screen 3: NetScope interactive observability (Interfaces/Rates and Connections).

use crate::app::ArgusTuiApp;
use argus_core::application::services::format_bytes;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::widgets::{Block, Borders, Row, Table, Tabs};
use ratatui::Frame;

pub fn render(frame: &mut Frame<'_>, area: Rect, app: &ArgusTuiApp) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(6)])
        .split(area);

    let sub_titles = ["Interfaces & Taxas", "Conexões TCP / UDP"];
    let sub_idx = app.netscope_subtab % sub_titles.len();
    let sub_tabs = Tabs::new(sub_titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" NetScope (Tab para alternar) "),
        )
        .select(sub_idx)
        .highlight_style(app.theme.selected_tab_style());
    frame.render_widget(sub_tabs, chunks[0]);

    match sub_idx {
        0 => {
            let rows: Vec<Row<'_>> = app
                .interfaces
                .iter()
                .skip(app.scroll)
                .take(40)
                .map(|iface| {
                    let rate = app.rates.iter().find(|r| r.interface_name == iface.name);
                    let rate_str = rate
                        .map(|r| {
                            format!(
                                "{:.1} / {:.1} KB/s",
                                r.rx_bytes_per_sec / 1024.0,
                                r.tx_bytes_per_sec / 1024.0
                            )
                        })
                        .unwrap_or_else(|| "0.0 / 0.0 KB/s".to_string());
                    let ips = if iface.ip_addresses.is_empty() {
                        "-".to_string()
                    } else {
                        iface.ip_addresses.join(", ")
                    };
                    Row::new(vec![
                        iface.name.clone(),
                        iface.status.to_string(),
                        format_bytes(iface.bytes_received),
                        format_bytes(iface.bytes_sent),
                        rate_str,
                        ips,
                    ])
                })
                .collect();

            let table = Table::new(
                rows,
                [
                    Constraint::Percentage(20),
                    Constraint::Length(8),
                    Constraint::Percentage(13),
                    Constraint::Percentage(13),
                    Constraint::Percentage(18),
                    Constraint::Percentage(28),
                ],
            )
            .header(
                Row::new([
                    "Interface",
                    "Estado",
                    "RX Total",
                    "TX Total",
                    "RX/TX Taxa",
                    "IPs",
                ])
                .style(app.theme.header_style()),
            )
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!(" Interfaces ({}) ", app.interfaces.len())),
            );
            frame.render_widget(table, chunks[1]);
        }
        _ => {
            let rows: Vec<Row<'_>> = app
                .connections
                .iter()
                .skip(app.scroll)
                .take(40)
                .map(|c| {
                    let remote = match (&c.remote_address, c.remote_port) {
                        (Some(addr), Some(port)) => format!("{addr}:{port}"),
                        _ => "-".to_string(),
                    };
                    let proc_str = match (c.pid, &c.process_name) {
                        (Some(pid), Some(name)) => format!("{name} ({pid})"),
                        (Some(pid), None) => format!("PID {pid}"),
                        _ => "-".to_string(),
                    };
                    Row::new(vec![
                        c.protocol.to_string(),
                        format!("{}:{}", c.local_address, c.local_port),
                        remote,
                        c.state.clone(),
                        proc_str,
                    ])
                })
                .collect();

            let table = Table::new(
                rows,
                [
                    Constraint::Length(6),
                    Constraint::Percentage(26),
                    Constraint::Percentage(26),
                    Constraint::Percentage(16),
                    Constraint::Percentage(26),
                ],
            )
            .header(
                Row::new(["Proto", "Local", "Remoto", "Estado", "Processo"])
                    .style(app.theme.header_style()),
            )
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!(" Conexões Ativas ({}) ", app.connections.len())),
            );
            frame.render_widget(table, chunks[1]);
        }
    }
}
