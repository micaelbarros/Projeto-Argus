//! Screen 4: Local SQLite history of analysis runs, network metrics, and alerts.

use crate::app::ArgusTuiApp;
use argus_core::application::services::format_bytes;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::widgets::{Block, Borders, Row, Table};
use ratatui::Frame;

pub fn render(frame: &mut Frame<'_>, area: Rect, app: &ArgusTuiApp) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    let run_rows: Vec<Row<'_>> = app
        .analysis_history
        .iter()
        .take(20)
        .map(|r| {
            Row::new(vec![
                r.id.to_string(),
                r.started_at.format("%Y-%m-%d %H:%M:%S").to_string(),
                r.status.clone(),
                format!("{}/100", r.score),
                r.project_path.clone(),
            ])
        })
        .collect();
    let run_table = Table::new(
        run_rows,
        [
            Constraint::Length(6),
            Constraint::Length(20),
            Constraint::Length(12),
            Constraint::Length(9),
            Constraint::Min(20),
        ],
    )
    .header(Row::new(["ID", "Início (UTC)", "Status", "Saúde", "Projeto"]).style(app.theme.header_style()))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(" Histórico de Análises DevPilot ({}) ", app.analysis_history.len())),
    );
    frame.render_widget(run_table, rows[0]);

    let metric_rows: Vec<Row<'_>> = app
        .metric_history
        .iter()
        .take(20)
        .map(|m| {
            Row::new(vec![
                m.timestamp.format("%H:%M:%S").to_string(),
                m.interface_name.clone(),
                format_bytes(m.bytes_received),
                format_bytes(m.bytes_sent),
                m.latency_ms
                    .map(|v| format!("{v:.2} ms"))
                    .unwrap_or_else(|| "-".to_string()),
            ])
        })
        .collect();
    let metric_table = Table::new(
        metric_rows,
        [
            Constraint::Length(10),
            Constraint::Percentage(28),
            Constraint::Percentage(20),
            Constraint::Percentage(20),
            Constraint::Percentage(22),
        ],
    )
    .header(
        Row::new(["Hora", "Interface / Alvo", "RX", "TX", "Latência"]).style(app.theme.header_style()),
    )
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(" Histórico de Métricas NetScope ({}) ", app.metric_history.len())),
    );
    frame.render_widget(metric_table, rows[1]);
}
