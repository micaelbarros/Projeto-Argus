//! Screen 5: Active configuration overview.

use crate::app::ArgusTuiApp;
use ratatui::layout::Rect;
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

pub fn render(frame: &mut Frame<'_>, area: Rect, app: &ArgusTuiApp) {
    let cfg = &app.config;
    let lines = vec![
        Line::from("Configuração ativa (TOML)"),
        Line::from(""),
        Line::from(format!("  [general] theme = \"{}\"", cfg.general.theme)),
        Line::from(format!(
            "  [general] language = \"{}\"",
            cfg.general.language
        )),
        Line::from(format!(
            "  [general] log_level = \"{}\"",
            cfg.general.log_level
        )),
        Line::from(""),
        Line::from(format!(
            "  [modules] devpilot_enabled = {}",
            cfg.modules.devpilot_enabled
        )),
        Line::from(format!(
            "  [modules] netscope_enabled = {}",
            cfg.modules.netscope_enabled
        )),
        Line::from(""),
        Line::from(format!(
            "  [devpilot] max_file_size_mb = {}",
            cfg.devpilot.max_file_size_mb
        )),
        Line::from(format!(
            "  [devpilot] large_file_threshold_kb = {}",
            cfg.devpilot.large_file_threshold_kb
        )),
        Line::from(format!(
            "  [devpilot] ignore_hidden = {} | respect_gitignore = {}",
            cfg.devpilot.ignore_hidden, cfg.devpilot.respect_gitignore
        )),
        Line::from(""),
        Line::from(format!(
            "  [netscope] refresh_interval_ms = {}",
            cfg.netscope.refresh_interval_ms
        )),
        Line::from(format!(
            "  [netscope] latency_threshold_ms = {:.1} | packet_loss_threshold_pct = {:.1}%",
            cfg.netscope.latency_threshold_ms, cfg.netscope.packet_loss_threshold_pct
        )),
        Line::from(format!(
            "  [netscope] authorized_cidrs = {:?}",
            cfg.netscope.authorized_cidrs
        )),
        Line::from(""),
        Line::from(format!(
            "  [storage] database_enabled = {} | retention_days = {}",
            cfg.storage.database_enabled, cfg.storage.retention_days
        )),
    ];

    let widget = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Configurações do Projeto ARGUS ")
            .title_style(app.theme.header_style()),
    );
    frame.render_widget(widget, area);
}
