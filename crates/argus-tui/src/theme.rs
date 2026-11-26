//! Accessible color themes compatible with basic and true-color terminals.

use ratatui::style::{Color, Modifier, Style};

/// Visual styling palette for ARGUS TUI.
#[derive(Debug, Clone, Copy)]
pub struct ArgusTheme {
    pub accent: Color,
    pub accent_secondary: Color,
    pub ok: Color,
    pub warn: Color,
    pub err: Color,
    pub muted: Color,
    pub text: Color,
    pub highlight_bg: Color,
}

impl ArgusTheme {
    pub fn from_name(name: &str) -> Self {
        if name.eq_ignore_ascii_case("light") {
            Self {
                accent: Color::Blue,
                accent_secondary: Color::Magenta,
                ok: Color::Green,
                warn: Color::Yellow,
                err: Color::Red,
                muted: Color::DarkGray,
                text: Color::Black,
                highlight_bg: Color::Gray,
            }
        } else {
            Self {
                accent: Color::Cyan,
                accent_secondary: Color::LightBlue,
                ok: Color::Green,
                warn: Color::Yellow,
                err: Color::Red,
                muted: Color::DarkGray,
                text: Color::White,
                highlight_bg: Color::DarkGray,
            }
        }
    }

    pub fn title_style(&self) -> Style {
        Style::default()
            .fg(self.accent)
            .add_modifier(Modifier::BOLD)
    }

    pub fn header_style(&self) -> Style {
        Style::default()
            .fg(self.accent_secondary)
            .add_modifier(Modifier::BOLD)
    }

    pub fn muted_style(&self) -> Style {
        Style::default().fg(self.muted)
    }

    pub fn selected_tab_style(&self) -> Style {
        Style::default()
            .fg(Color::Black)
            .bg(self.accent)
            .add_modifier(Modifier::BOLD)
    }
}
