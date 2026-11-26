//! Shared reusable TUI widgets (navigation header, footer status bar, help modal).

use crate::app::Screen;
use crate::theme::ArgusTheme;
use argus_core::{APP_NAME, APP_VERSION};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Tabs};
use ratatui::Frame;

/// Renders the top navigation bar with active screen highlight.
pub fn render_header(frame: &mut Frame<'_>, area: Rect, screen: Screen, theme: ArgusTheme) {
    let titles = [
        "[1] Dashboard",
        "[2] DevPilot",
        "[3] NetScope",
        "[4] Histórico",
        "[C] Configurações",
    ];
    let selected = match screen {
        Screen::Dashboard => 0,
        Screen::DevPilot => 1,
        Screen::NetScope => 2,
        Screen::History => 3,
        Screen::Settings => 4,
    };

    let tabs = Tabs::new(titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" {APP_NAME} v{APP_VERSION} "))
                .title_style(theme.title_style()),
        )
        .select(selected)
        .style(Style::default().fg(theme.text))
        .highlight_style(theme.selected_tab_style());

    frame.render_widget(tabs, area);
}

/// Renders the bottom shortcut/status footer.
pub fn render_footer(frame: &mut Frame<'_>, area: Rect, status: &str, theme: ArgusTheme) {
    let line = Line::from(vec![
        Span::styled(
            " 1-4: Telas | Tab: Sub-abas | R: Atualizar | C: Config | ?: Ajuda | Q: Sair  ",
            theme.muted_style(),
        ),
        Span::styled("│ ", theme.muted_style()),
        Span::styled(status, Style::default().fg(theme.accent)),
    ]);

    let paragraph = Paragraph::new(line).block(Block::default().borders(Borders::ALL));
    frame.render_widget(paragraph, area);
}

/// Renders a centered modal listing keyboard shortcuts.
pub fn render_help_modal(frame: &mut Frame<'_>, area: Rect, theme: ArgusTheme) {
    let popup = centered_rect(64, 58, area);
    frame.render_widget(Clear, popup);

    let lines = vec![
        Line::from(Span::styled(
            "Atalhos do Projeto ARGUS",
            theme.title_style(),
        )),
        Line::from(""),
        Line::from("  1         Dashboard Geral"),
        Line::from("  2         DevPilot (Análise, Dependências, TODOs, Git)"),
        Line::from("  3         NetScope (Interfaces, Tráfego, Conexões)"),
        Line::from("  4         Histórico SQLite e Alertas"),
        Line::from("  C         Configurações ativas"),
        Line::from("  Tab / ←→  Alternar sub-abas da tela atual"),
        Line::from("  ↑↓ / j k  Rolar tabelas/listas"),
        Line::from("  R         Atualizar métricas e reanalisar projeto"),
        Line::from("  ?         Abrir/fechar esta ajuda"),
        Line::from("  Q / Esc   Fechar ajuda ou sair"),
        Line::from(""),
        Line::from(Span::styled(
            "Pressione '?' ou 'Esc' para fechar.",
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::ITALIC),
        )),
    ];

    let widget = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Ajuda ")
            .title_style(theme.title_style()),
    );
    frame.render_widget(widget, popup);
}

pub fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1])[1]
}
