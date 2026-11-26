//! Cross-platform TOML configuration management for Projeto ARGUS.

use crate::errors::{ArgusError, ArgusResult};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Top-level ARGUS configuration stored in TOML.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct ArgusConfig {
    pub general: GeneralConfig,
    pub modules: ModulesConfig,
    pub devpilot: DevPilotConfig,
    pub netscope: NetScopeConfig,
    pub storage: StorageConfig,
}

impl Default for ArgusConfig {
    fn default() -> Self {
        Self {
            general: GeneralConfig::default(),
            modules: ModulesConfig::default(),
            devpilot: DevPilotConfig::default(),
            netscope: NetScopeConfig::default(),
            storage: StorageConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct GeneralConfig {
    pub theme: String,
    pub language: String,
    pub log_level: String,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            theme: "dark".to_string(),
            language: "pt-BR".to_string(),
            log_level: "info".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct ModulesConfig {
    pub devpilot_enabled: bool,
    pub netscope_enabled: bool,
}

impl Default for ModulesConfig {
    fn default() -> Self {
        Self {
            devpilot_enabled: true,
            netscope_enabled: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct DevPilotConfig {
    pub max_file_size_mb: u64,
    pub large_file_threshold_kb: u64,
    pub ignore_hidden: bool,
    pub respect_gitignore: bool,
    pub ignored_dirs: Vec<String>,
    pub max_tree_depth: usize,
}

impl Default for DevPilotConfig {
    fn default() -> Self {
        Self {
            max_file_size_mb: 10,
            large_file_threshold_kb: 512,
            ignore_hidden: true,
            respect_gitignore: true,
            ignored_dirs: vec![
                ".git".to_string(),
                "target".to_string(),
                "node_modules".to_string(),
                "dist".to_string(),
                "build".to_string(),
                ".venv".to_string(),
                "__pycache__".to_string(),
            ],
            max_tree_depth: 5,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct NetScopeConfig {
    pub refresh_interval_ms: u64,
    pub history_enabled: bool,
    pub latency_threshold_ms: f64,
    pub packet_loss_threshold_pct: f64,
    pub throughput_alert_mbps: f64,
    pub authorized_cidrs: Vec<String>,
    pub probe_rate_limit_ms: u64,
}

impl Default for NetScopeConfig {
    fn default() -> Self {
        Self {
            refresh_interval_ms: 1000,
            history_enabled: true,
            latency_threshold_ms: 150.0,
            packet_loss_threshold_pct: 5.0,
            throughput_alert_mbps: 500.0,
            authorized_cidrs: vec!["127.0.0.1/32".to_string()],
            probe_rate_limit_ms: 50,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct StorageConfig {
    pub database_enabled: bool,
    pub database_path: Option<PathBuf>,
    pub retention_days: u32,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            database_enabled: true,
            database_path: None,
            retention_days: 30,
        }
    }
}

impl ArgusConfig {
    /// Resolves the default cross-platform configuration path (`.../argus/config.toml`).
    pub fn default_config_path() -> ArgusResult<PathBuf> {
        let dirs = ProjectDirs::from("io", "argus", "argus").ok_or_else(|| {
            ArgusError::ConfigurationError(
                "Não foi possível resolver o diretório padrão de configuração do usuário."
                    .to_string(),
            )
        })?;
        Ok(dirs.config_dir().join("config.toml"))
    }

    /// Resolves the SQLite database path from config or platform data directory (`argus.db`).
    pub fn resolve_database_path(&self) -> ArgusResult<PathBuf> {
        if let Some(custom) = &self.storage.database_path {
            return Ok(custom.clone());
        }
        let dirs = ProjectDirs::from("io", "argus", "argus").ok_or_else(|| {
            ArgusError::ConfigurationError(
                "Não foi possível resolver o diretório padrão de dados do usuário.".to_string(),
            )
        })?;
        Ok(dirs.data_local_dir().join("argus.db"))
    }

    /// Loads configuration from `path` (or default path if `None`).
    /// If the default config file does not exist yet, returns defaults cleanly.
    pub fn load(path: Option<&Path>) -> ArgusResult<Self> {
        let target = match path {
            Some(p) => {
                if !p.exists() {
                    return Err(ArgusError::ConfigurationError(format!(
                        "Arquivo de configuração não encontrado: {}",
                        p.display()
                    )));
                }
                p.to_path_buf()
            }
            None => {
                let default_path = Self::default_config_path()?;
                if !default_path.exists() {
                    return Ok(Self::default());
                }
                default_path
            }
        };

        let raw = fs::read_to_string(&target)?;
        let cfg: Self = toml::from_str(&raw).map_err(|e| {
            ArgusError::ConfigurationError(format!("TOML inválido em {}: {e}", target.display()))
        })?;
        cfg.validate()?;
        Ok(cfg)
    }

    /// Saves the configuration to `path` (creating parent directories if needed).
    pub fn save(&self, path: &Path) -> ArgusResult<()> {
        self.validate()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let serialized = toml::to_string_pretty(self).map_err(|e| {
            ArgusError::ConfigurationError(format!("Falha ao serializar configuração: {e}"))
        })?;
        fs::write(path, serialized)?;
        Ok(())
    }

    /// Validates business invariants on configuration values.
    pub fn validate(&self) -> ArgusResult<()> {
        if self.devpilot.max_file_size_mb == 0 {
            return Err(ArgusError::ConfigurationError(
                "devpilot.max_file_size_mb deve ser maior que zero.".to_string(),
            ));
        }
        if self.netscope.refresh_interval_ms < 100 {
            return Err(ArgusError::ConfigurationError(
                "netscope.refresh_interval_ms deve ser >= 100 ms.".to_string(),
            ));
        }
        if !(0.0..=100.0).contains(&self.netscope.packet_loss_threshold_pct) {
            return Err(ArgusError::ConfigurationError(
                "netscope.packet_loss_threshold_pct deve estar entre 0 e 100.".to_string(),
            ));
        }
        if self.netscope.latency_threshold_ms <= 0.0 {
            return Err(ArgusError::ConfigurationError(
                "netscope.latency_threshold_ms deve ser positivo.".to_string(),
            ));
        }
        if !matches!(
            self.general.log_level.as_str(),
            "trace" | "debug" | "info" | "warn" | "error"
        ) {
            return Err(ArgusError::ConfigurationError(format!(
                "general.log_level inválido: '{}'",
                self.general.log_level
            )));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use tempfile::tempdir;

    #[test]
    fn default_config_is_valid() {
        let cfg = ArgusConfig::default();
        assert!(cfg.validate().is_ok());
    }

    #[test]
    fn save_and_load_roundtrip() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("config.toml");
        let cfg = ArgusConfig::default();
        cfg.save(&file).unwrap();
        let loaded = ArgusConfig::load(Some(&file)).unwrap();
        assert_eq!(cfg, loaded);
    }

    #[test]
    fn rejects_invalid_refresh_interval() {
        let mut cfg = ArgusConfig::default();
        cfg.netscope.refresh_interval_ms = 10;
        assert!(cfg.validate().is_err());
    }

    proptest! {
        #[test]
        fn toml_roundtrip_preserves_valid_values(
            max_mb in 1u64..1024,
            refresh_ms in 100u64..10_000,
            retention in 1u32..365,
            latency_ms in 1.0f64..5000.0,
            loss_pct in 0.0f64..100.0,
        ) {
            let mut cfg = ArgusConfig::default();
            cfg.devpilot.max_file_size_mb = max_mb;
            cfg.netscope.refresh_interval_ms = refresh_ms;
            cfg.storage.retention_days = retention;
            cfg.netscope.latency_threshold_ms = latency_ms;
            cfg.netscope.packet_loss_threshold_pct = loss_pct;

            let serialized = toml::to_string(&cfg).unwrap();
            let parsed: ArgusConfig = toml::from_str(&serialized).unwrap();
            prop_assert_eq!(parsed.devpilot.max_file_size_mb, max_mb);
            prop_assert_eq!(parsed.netscope.refresh_interval_ms, refresh_ms);
            prop_assert_eq!(parsed.storage.retention_days, retention);
            prop_assert!((parsed.netscope.latency_threshold_ms - latency_ms).abs() < 1e-6);
            prop_assert!((parsed.netscope.packet_loss_threshold_pct - loss_pct).abs() < 1e-6);
        }
    }
}
