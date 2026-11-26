//! CLI argument and subcommand definitions using `clap`.

use argus_core::domain::network::TransportProtocol;
use argus_core::domain::report::ReportFormat;
use clap::{Args, Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

/// Projeto ARGUS — Produtividade de Engenharia (DevPilot) e Observabilidade de Redes (NetScope).
#[derive(Debug, Parser)]
#[command(
    name = "argus",
    version,
    about = "Projeto ARGUS — Ferramenta local-first CLI + TUI para análise de projetos e diagnóstico de redes.",
    long_about = None
)]
pub struct Cli {
    /// Saída estruturada em JSON.
    #[arg(long, global = true)]
    pub json: bool,

    /// Habilita logs detalhados de depuração.
    #[arg(short, long, global = true, conflicts_with = "quiet")]
    pub verbose: bool,

    /// Suprime mensagens informativas extras.
    #[arg(short, long, global = true)]
    pub quiet: bool,

    /// Caminho para arquivo de configuração TOML personalizado.
    #[arg(long, global = true, value_name = "FILE")]
    pub config: Option<PathBuf>,

    /// Salva a saída em um arquivo ao invés de exibir apenas no terminal.
    #[arg(short, long, global = true, value_name = "FILE")]
    pub output: Option<PathBuf>,

    /// Permite sobrescrever arquivos existentes ao usar --output ou init.
    #[arg(long, global = true)]
    pub force: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Inicializa o arquivo de configuração padrão (config.toml) e o banco SQLite local.
    Init,

    /// Executa diagnóstico do ambiente de execução do Projeto ARGUS.
    Doctor,

    /// Abre o dashboard interativo no terminal (TUI).
    Tui {
        /// Diretório do projeto a ser inspecionado no DevPilot.
        #[arg(default_value = ".")]
        path: PathBuf,
    },

    /// Módulo DevPilot — análise de repositórios, dependências, saúde e Git.
    Devpilot(DevPilotArgs),

    /// Módulo NetScope — interfaces, tráfego, conexões, latência e diagnóstico de rede.
    Netscope(NetScopeArgs),

    /// Consulta o histórico local salvo no SQLite (análises, métricas e alertas).
    History {
        /// Número máximo de registros por seção.
        #[arg(long, default_value_t = 15)]
        limit: usize,
    },

    /// Gera um relatório técnico consolidado (JSON ou Markdown).
    Report {
        /// Diretório do projeto para incluir no relatório (opcional).
        #[arg(long, default_value = ".")]
        path: PathBuf,

        /// Formato de exportação do relatório.
        #[arg(long, value_enum, default_value_t = CliReportFormat::Json)]
        format: CliReportFormat,

        /// Executa um teste de latência opcional para incluir no relatório.
        #[arg(long)]
        ping: Option<String>,
    },
}

#[derive(Debug, Args)]
pub struct DevPilotArgs {
    #[command(subcommand)]
    pub command: DevPilotCommand,
}

#[derive(Debug, Subcommand)]
pub enum DevPilotCommand {
    /// Analisa estrutura, linguagens, frameworks, dependências, TODOs e saúde do projeto.
    Analyze {
        #[arg(default_value = ".")]
        path: PathBuf,
    },

    /// Gera o relatório de saúde e recomendações do projeto.
    Health {
        #[arg(default_value = ".")]
        path: PathBuf,
    },

    /// Inspeciona branch, status e histórico recente do repositório Git.
    Git {
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Quantidade máxima de commits recentes a exibir.
        #[arg(long, default_value_t = 15)]
        commits: usize,
    },

    /// Gera documentação técnica inicial em Markdown.
    Docs {
        #[arg(default_value = ".")]
        path: PathBuf,
    },
}

#[derive(Debug, Args)]
pub struct NetScopeArgs {
    #[command(subcommand)]
    pub command: NetScopeCommand,
}

#[derive(Debug, Subcommand)]
pub enum NetScopeCommand {
    /// Lista interfaces de rede disponíveis, estado e contadores acumulados.
    Interfaces,

    /// Monitora taxas de transferência (RX/TX) ao longo de múltiplas amostras.
    Monitor {
        /// Número de amostras (0 = contínuo até Ctrl+C).
        #[arg(long, default_value_t = 5)]
        samples: u32,

        /// Intervalo entre amostras em milissegundos.
        #[arg(long)]
        interval_ms: Option<u64>,
    },

    /// Executa teste de latência (mínimo, média, máximo e perda de pacotes).
    Ping {
        /// Hostname ou endereço IP de destino.
        target: String,

        /// Quantidade de sondagens (1..=20).
        #[arg(short = 'c', long, default_value_t = 4)]
        count: u32,

        /// Timeout por sondagem em milissegundos.
        #[arg(long, default_value_t = 2000)]
        timeout_ms: u64,

        /// Porta TCP opcional para sondagem sem privilégios (padrão: 443).
        #[arg(long)]
        port: Option<u16>,
    },

    /// Lista conexões TCP/UDP ativas e processos associados quando permitido.
    Connections {
        /// Filtra por protocolo (tcp ou udp).
        #[arg(long, value_enum)]
        protocol: Option<CliProtocol>,
    },

    /// Verifica portas TCP em um host explicitamente autorizado em config.toml.
    Ports {
        /// IPv4 ou localhost autorizado em `netscope.authorized_cidrs`.
        host: String,

        /// Lista de portas separadas por vírgula (ex.: 80,443,8080).
        #[arg(long, value_delimiter = ',', required = true)]
        ports: Vec<u16>,

        /// Timeout por porta em milissegundos.
        #[arg(long, default_value_t = 500)]
        timeout_ms: u64,
    },

    /// Descobre hosts ativos em um bloco CIDR IPv4 autorizado em config.toml.
    Discover {
        /// Bloco CIDR autorizado (ex.: 127.0.0.1/32).
        cidr: String,

        /// Timeout por sondagem em milissegundos.
        #[arg(long, default_value_t = 300)]
        timeout_ms: u64,
    },

    /// Lista alertas gerados pelo NetScope.
    Alerts {
        /// Inclui alertas já reconhecidos.
        #[arg(long)]
        all: bool,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum CliReportFormat {
    Json,
    Markdown,
}

impl From<CliReportFormat> for ReportFormat {
    fn from(value: CliReportFormat) -> Self {
        match value {
            CliReportFormat::Json => ReportFormat::Json,
            CliReportFormat::Markdown => ReportFormat::Markdown,
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum CliProtocol {
    Tcp,
    Udp,
}

impl From<CliProtocol> for TransportProtocol {
    fn from(value: CliProtocol) -> Self {
        match value {
            CliProtocol::Tcp => TransportProtocol::Tcp,
            CliProtocol::Udp => TransportProtocol::Udp,
        }
    }
}
