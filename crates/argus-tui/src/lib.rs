//! Interactive terminal dashboard (TUI) for Projeto ARGUS.

pub mod app;
pub mod events;
pub mod screens;
pub mod theme;
pub mod widgets;

pub use app::{run_tui, ArgusTuiApp};
