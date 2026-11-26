//! Classified error types for Projeto ARGUS.

use std::path::PathBuf;
use thiserror::Error;

/// Unified result alias across ARGUS crates.
pub type ArgusResult<T> = Result<T, ArgusError>;

/// Domain and application error classification.
#[derive(Debug, Error)]
pub enum ArgusError {
    #[error("Erro de configuração: {0}")]
    ConfigurationError(String),

    #[error("Permissão negada: {0}")]
    PermissionDenied(String),

    #[error("Caminho inválido '{path}': {reason}")]
    InvalidPath { path: PathBuf, reason: String },

    #[error("Rede indisponível: {0}")]
    NetworkUnavailable(String),

    #[error("Plataforma não suportada: {0}")]
    UnsupportedPlatform(String),

    #[error("Erro de banco de dados: {0}")]
    DatabaseError(String),

    #[error("Falha na análise: {0}")]
    AnalysisFailed(String),

    #[error("Erro de repositório Git: {0}")]
    GitError(String),

    #[error("Erro ao exportar relatório: {0}")]
    ExportError(String),

    #[error("Módulo desativado na configuração: {0}")]
    ModuleDisabled(String),

    #[error("Operação cancelada pelo usuário")]
    Cancelled,

    #[error("Erro de E/S: {0}")]
    IoError(String),
}

impl ArgusError {
    /// Returns a concise, user-friendly explanation without exposing internal stack noise.
    pub fn friendly_message(&self) -> String {
        match self {
            Self::ConfigurationError(msg) => {
                format!("Verifique seu arquivo de configuração (config.toml): {msg}")
            }
            Self::PermissionDenied(msg) => {
                format!("Permissão insuficiente para concluir a operação: {msg}")
            }
            Self::InvalidPath { path, reason } => {
                format!(
                    "O caminho '{}' não pode ser usado: {reason}",
                    path.display()
                )
            }
            Self::NetworkUnavailable(msg) => {
                format!("Não foi possível acessar a rede solicitada: {msg}")
            }
            Self::UnsupportedPlatform(msg) => {
                format!("Funcionalidade indisponível nesta plataforma: {msg}")
            }
            Self::DatabaseError(msg) => {
                format!("Falha na persistência local (SQLite): {msg}")
            }
            Self::AnalysisFailed(msg) => format!("A análise não pôde ser concluída: {msg}"),
            Self::GitError(msg) => format!("Não foi possível inspecionar o repositório Git: {msg}"),
            Self::ExportError(msg) => format!("Falha ao gerar o arquivo de saída: {msg}"),
            Self::ModuleDisabled(module) => {
                format!("O módulo '{module}' está desativado em config.toml.")
            }
            Self::Cancelled => "Operação cancelada de forma segura.".to_string(),
            Self::IoError(msg) => format!("Erro de leitura/escrita no sistema de arquivos: {msg}"),
        }
    }
}

impl From<std::io::Error> for ArgusError {
    fn from(err: std::io::Error) -> Self {
        if err.kind() == std::io::ErrorKind::PermissionDenied {
            Self::PermissionDenied(err.to_string())
        } else {
            Self::IoError(err.to_string())
        }
    }
}
