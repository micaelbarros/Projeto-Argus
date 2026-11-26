//! Screen 2: DevPilot interactive inspection (Overview, Dependencies, TODOs, Git).

use crate::app::ArgusTuiApp;
use argus_core::application::services::format_bytes;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph, Row, Table, Tabs};
use ratatui::Frame;

pub fn render(frame: &mut Frame<'_>, area: Rect, app: &ArgusTuiApp) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(6)])
        .split(area);

    let sub_titles = [
        "Visão Geral & Saúde",
        "Dependências",
        "TODO / FIXME",
        "Git Insights",
    ];
    let sub_idx = app.devpilot_subtab % sub_titles.len();
    let sub_tabs = Tabs::new(sub_titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" DevPilot (Tab para alternar) "),
        )
        .select(sub_idx)
        .highlight_style(app.theme.selected_tab_style());
    frame.render_widget(sub_tabs, chunks[0]);

    let Some(analysis) = &app.analysis else {
        let empty =
            Paragraph::new("Nenhuma análise disponível. Pressione R para analisar o projeto.")
                .block(Block::default().borders(Borders::ALL));
        frame.render_widget(empty, chunks[1]);
        return;
    };

    match sub_idx {
        0 => {
            let mut lines = vec![
                Line::from(format!(
                    "Projeto: {} ({})",
                    analysis.project.name,
                    analysis.project.root_path.display()
                )),
                Line::from(format!(
                    "Saúde: {}/100 ({}) | Arquivos: {} | Diretórios: {} | Tamanho: {}",
                    analysis.health.score,
                    analysis.health.grade,
                    analysis.project.total_files,
                    analysis.project.total_dirs,
                    format_bytes(analysis.project.total_bytes)
                )),
                Line::from(""),
                Line::from("Frameworks detectados:"),
            ];
            if analysis.frameworks.is_empty() {
                lines.push(Line::from("  (nenhum framework identificado)"));
            } else {
                for fw in &analysis.frameworks {
                    lines.push(Line::from(format!(
                        "  • {} ({}) — {}",
                        fw.name, fw.category, fw.evidence
                    )));
                }
            }
            lines.push(Line::from(""));
            lines.push(Line::from("Pontos de atenção:"));
            if analysis.health.issues.is_empty() {
                lines.push(Line::from("  ✓ Nenhum problema crítico encontrado."));
            } else {
                for issue in &analysis.health.issues {
                    lines.push(Line::from(format!(
                        "  [{}] {} — {}",
                        issue.code, issue.title, issue.recommendation
                    )));
                }
            }
            let p = Paragraph::new(lines).scroll((app.scroll as u16, 0)).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Saúde & Estrutura "),
            );
            frame.render_widget(p, chunks[1]);
        }
        1 => {
            let rows: Vec<Row<'_>> = analysis
                .dependencies
                .iter()
                .skip(app.scroll)
                .take(40)
                .map(|d| {
                    Row::new(vec![
                        d.name.clone(),
                        d.version.clone(),
                        d.ecosystem.clone(),
                        if d.is_dev {
                            "dev".to_string()
                        } else {
                            "runtime".to_string()
                        },
                        d.manifest.clone(),
                    ])
                })
                .collect();
            let table = Table::new(
                rows,
                [
                    Constraint::Percentage(26),
                    Constraint::Percentage(16),
                    Constraint::Percentage(12),
                    Constraint::Percentage(10),
                    Constraint::Percentage(36),
                ],
            )
            .header(
                Row::new(["Pacote", "Versão", "Ecossistema", "Tipo", "Manifesto"])
                    .style(app.theme.header_style()),
            )
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!(" Dependências ({}) ", analysis.dependencies.len())),
            );
            frame.render_widget(table, chunks[1]);
        }
        2 => {
            let rows: Vec<Row<'_>> = analysis
                .todos
                .iter()
                .skip(app.scroll)
                .take(40)
                .map(|t| {
                    Row::new(vec![
                        t.kind.clone(),
                        format!("{}:{}", t.file.display(), t.line),
                        t.text.clone(),
                    ])
                })
                .collect();
            let table = Table::new(
                rows,
                [
                    Constraint::Length(8),
                    Constraint::Percentage(35),
                    Constraint::Percentage(57),
                ],
            )
            .header(Row::new(["Tipo", "Local", "Descrição"]).style(app.theme.header_style()))
            .block(Block::default().borders(Borders::ALL).title(format!(
                " Marcadores TODO / FIXME ({}) ",
                analysis.todos.len()
            )));
            frame.render_widget(table, chunks[1]);
        }
        _ => {
            let mut lines = Vec::new();
            if let Some(git) = &analysis.git {
                lines.push(Line::from(format!("Branch atual: {}", git.branch)));
                lines.push(Line::from(format!(
                    "Estado: {} | Commits analisados: {} | Contribuidores: {}",
                    if git.is_dirty {
                        "Working tree com alterações"
                    } else {
                        "Limpo"
                    },
                    git.total_commits,
                    git.contributors.len()
                )));
                lines.push(Line::from(""));
                lines.push(Line::from("Commits recentes:"));
                for c in &git.recent_commits {
                    lines.push(Line::from(format!(
                        "  {} | {} | {} — {}",
                        c.short_hash,
                        c.timestamp.format("%Y-%m-%d %H:%M"),
                        c.author,
                        c.summary
                    )));
                }
            } else {
                lines.push(Line::from(
                    "Este diretório não faz parte de um repositório Git.",
                ));
            }
            let p = Paragraph::new(lines)
                .style(Style::default().fg(app.theme.text))
                .scroll((app.scroll as u16, 0))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(" Git Insights "),
                );
            frame.render_widget(p, chunks[1]);
        }
    }
}
