//! Core crate for Projeto ARGUS.
//!
//! Contains pure domain models, hexagonal port definitions, configuration management,
//! error classification, and shared application utilities. Zero dependency on TUI,
//! SQLite, Git2, or OS-specific network APIs.

pub mod application;
pub mod config;
pub mod domain;
pub mod errors;
pub mod ports;

pub const APP_NAME: &str = "Projeto ARGUS";
pub const APP_BIN: &str = "argus";
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
