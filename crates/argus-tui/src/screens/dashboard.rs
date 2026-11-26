//! Screen 1: Unified Overview Dashboard combining DevPilot and NetScope indicators.

use crate::app::ArgusTuiApp;
use argus_core::application::services::format_bytes;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Gauge, List, ListItem, Paragraph, Sparkline};
use ratatui::Frame;

pub fn render(frame: &mut Frame<'_>, area: Rect, app: &ArgusTuiApp) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),
            Constraint::Min(8),
            Constraint::Length(7),
        ])
        .split(area);

    let health_score = app
        .analysis
        .as_ref()
        .map(|a| a.health.score)
        .unwrap_or(0);
    let health_label = app
        .analysis
        .as_ref()
        .map(|a| {
            format!(
                "Projeto: {} | Saúde: {}/100 ({}) | Arquivos: {} | Linhas: {}",
                a.project.name,
                a.health.score,
                a.health.grade,
                a.project.total_files,
                a.project.total_lines
            )
        })
        .unwrap_or_else(|| "DevPilot: aguardando análise (pressione R)".to_string());

    let gauge_color = match health_score {
        80..=100 => app.theme.ok,
        60..=79 => app.theme.warn,
        _ => app.theme.err,
    };

    let gauge = Gauge::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Saúde do Projeto (DevPilot) ")
                .title_style(app.theme.header_style()),
        )
        .gauge_style(Style::default().fg(gauge_color))
        .percent(u16::from(health_score))
        .label(health_label);
    frame.render_widget(gauge, rows[0]);

    let mid_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(rows[1]);

    // Left panel: DevPilot snapshot
    let mut dp_lines = Vec::new();
    if let Some(a) = &app.analysis {
        dp_lines.push(Line::from(format!(
            "Diretório: {}",
            a.project.root_path.display()
        )));
        dp_lines.push(Line::from(format!(
            "Dependências: {} | TODO/FIXME: {} | Ciclos: {}",
            a.dependencies.len(),
            a.todos.len(),
            a.circular_dependencies.len()
        )));
        if let Some(git) = &a.git {
            dp_lines.push(Line::from(format!(
                "Git: branch '{}' | {} | {} commit(s)",
                git.branch,
                if git.is_dirty { "alterado" } else { "limpo" },
                git.total_commits
            )));
        }
        dp_lines.push(Line::from(""));
        dp_lines.push(Line::from(Span::styled(
            "Principais linguagens:",
            app.theme.header_style(),
        )));
        for lang in a.languages.iter().take(5) {
            dp_lines.push(Line::from(format!(
                "  • {:<16} {:>4} arq  {:>6.2}%",
                lang.language, lang.files, lang.percentage
            )));
        }
    } else {
        dp_lines.push(Line::from("Nenhuma análise carregada."));
    }
    let dp_card = Paragraph::new(dp_lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" DevPilot — Resumo ")
            .title_style(app.theme.header_style()),
    );
    frame.render_widget(dp_card, mid_cols[0]);

    // Right panel: NetScope snapshot + Sparkline
    let ns_split = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(5), Constraint::Length(4)])
        .split(mid_cols[1]);

    let iface_items: Vec<ListItem<'_>> = app
        .interfaces
        .iter()
        .take(6)
        .map(|iface| {
            ListItem::new(format!(
                "{:<14} [{}]  RX: {:<10} TX: {}",
                iface.name,
                iface.status,
                format_bytes(iface.bytes_received),
                format_bytes(iface.bytes_sent)
            ))
        })
        .collect();
    let ns_list = List::new(iface_items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(
                " NetScope — Interfaces ({}) | Conexões ({}) ",
                app.interfaces.len(),
                app.connections.len()
            ))
            .title_style(app.theme.header_style()),
    );
    frame.render_widget(ns_list, ns_split[0]);

    let spark_data: Vec<u64> = if app.throughput_history.is_empty() {
        vec![0]
    } else {
        app.throughput_history.clone()
    };
    let sparkline = Sparkline::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Tráfego Combinado (KB/s) "),
        )
        .data(&spark_data)
        .style(Style::default().fg(app.theme.accent));
    frame.render_widget(sparkline, ns_split[1]);

    // Bottom panel: Recent Alerts
    let alert_items: Vec<ListItem<'_>> = if app.alerts.is_empty() {
        vec![ListItem::new("Nenhum alerta ativo registrado.")]
    } else {
        app.alerts
            .iter()
            .take(4)
            .map(|al| {
                ListItem::new(format!(
                    "[{} / {}] {} ({})",
                    al.module,
                    al.severity,
                    al.message,
                    al.created_at.format("%H:%M:%S")
                ))
            })
            .collect()
    };
    let alerts_widget = List::new(alert_items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Alertas Recentes ")
            .title_style(app.theme.header_style()),
    );
    frame.render_widget(alerts_widget, rows[2]);
}
