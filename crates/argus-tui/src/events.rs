//! Keyboard and tick event handling for the ARGUS TUI.

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use std::time::Duration;

/// High-level UI actions mapped from terminal input events.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiAction {
    Tick,
    Quit,
    ShowDashboard,
    ShowDevPilot,
    ShowNetScope,
    ShowHistory,
    ShowSettings,
    NextSubTab,
    PrevSubTab,
    Refresh,
    ToggleHelp,
    ScrollDown,
    ScrollUp,
}

/// Polls terminal events up to `tick_rate`, translating keyboard shortcuts into `UiAction`.
pub fn poll_action(tick_rate: Duration) -> std::io::Result<UiAction> {
    if event::poll(tick_rate)? {
        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                return Ok(map_key(key));
            }
        }
    }
    Ok(UiAction::Tick)
}

pub fn map_key(key: KeyEvent) -> UiAction {
    match key.code {
        KeyCode::Char('q') | KeyCode::Char('Q') => UiAction::Quit,
        KeyCode::Esc => UiAction::Quit,
        KeyCode::Char('1') => UiAction::ShowDashboard,
        KeyCode::Char('2') => UiAction::ShowDevPilot,
        KeyCode::Char('3') => UiAction::ShowNetScope,
        KeyCode::Char('4') => UiAction::ShowHistory,
        KeyCode::Char('c') | KeyCode::Char('C') => UiAction::ShowSettings,
        KeyCode::Char('r') | KeyCode::Char('R') => UiAction::Refresh,
        KeyCode::Char('?') => UiAction::ToggleHelp,
        KeyCode::Tab | KeyCode::Right | KeyCode::Char('l') => UiAction::NextSubTab,
        KeyCode::BackTab | KeyCode::Left | KeyCode::Char('h') => UiAction::PrevSubTab,
        KeyCode::Down | KeyCode::Char('j') => UiAction::ScrollDown,
        KeyCode::Up | KeyCode::Char('k') => UiAction::ScrollUp,
        _ => UiAction::Tick,
    }
}
