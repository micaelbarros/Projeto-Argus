//! Command execution handlers wiring hexagonal ports, adapters, and output formatting.

use crate::args::{Cli, Commands, DevPilotCommand, NetScopeCommand};
use argus_core::application::services::format_bytes;
use argus_core::config::ArgusConfig;
use argus_core::domain::project::{GitSummary, HealthReport, ProjectAnalysis};
use argus_core::domain::report::{ArgusReport, DiagnosticCheck, DoctorReport, ReportFormat};
use argus_core::errors::{ArgusError, ArgusResult};
use argus_core::ports::exporter::{to_json_value, ReportExporterPort};
use argus_core::ports::filesystem::FileSystemPort;
use argus_core::ports::persistence::MetricsRepositoryPort;
use argus_core::{APP_NAME, APP_VERSION};
use argus_devpilot::domain::AnalyzeRequest;
use argus_devpilot::DevPilotService;
use argus_infrastructure::{
    DefaultReportExporter, Git2Adapter, LocalFileSystemAdapter, SQLiteMetricsAdapter,
    SystemNetworkAdapter,
};
use argus_netscope::NetScopeService;
use chrono::Utc;
use std::fmt::Write as _;
use std::sync::Arc;
use std::time::Duration;
use tokio_util::sync::CancellationToken;
use tracing::warn;

/// Holds composed application dependencies (Dependency Injection root).
pub struct AppContainer {
    pub config: ArgusConfig,
    pub fs: Arc<LocalFileSystemAdapter>,
    pub exporter: Arc<DefaultReportExporter>,
    pub repo: Option<Arc<dyn MetricsRepositoryPort>>,
    pub devpilot: DevPilotService,
    pub netscope: NetScopeService,
}

impl AppContainer {
    pub fn build(config: ArgusConfig) -> Self {
        let fs = Arc::new(LocalFileSystemAdapter::new());
        let git = Arc::new(Git2Adapter::new());
        let net = Arc::new(SystemNetworkAdapter::new());
        let exporter = Arc::new(DefaultReportExporter::new());

        let repo: Option<Arc<dyn MetricsRepositoryPort>> = if config.storage.database_enabled {
            match config
                .resolve_database_path()
                .and_then(|p| SQLiteMetricsAdapter::open(&p))
            {
                Ok(db) => {
                    let _ = db.prune_older_than(config.storage.retention_days);
                    Some(Arc::new(db))
                }
                Err(err) => {
                    warn!(error = %err, "SQLite indisponível; continuando sem persistência.");
                    None
                }
            }
        } else {
            None
        };

        let devpilot = DevPilotService::new(fs.clone(), git, repo.clone());
        let netscope = NetScopeService::new(net, repo.clone());

        Self {
            config,
            fs,
            exporter,
            repo,
            devpilot,
            netscope,
        }
    }
}

/// Dispatches the parsed CLI command with cooperative cancellation support.
pub async fn execute(cli: Cli, cancel: CancellationToken) -> ArgusResult<()> {
    // Handle `init` before loading config so users can bootstrap cleanly
    if matches!(cli.command, Commands::Init) {
        return handle_init(&cli);
    }

    let config = ArgusConfig::load(cli.config.as_deref())?;
    let container = AppContainer::build(config);

    match &cli.command {
        Commands::Init => unreachable!(),
        Commands::Doctor => handle_doctor(&cli, &container),
        Commands::Tui { path } => argus_tui::run_tui(
            container.config.clone(),
            path.clone(),
            &container.devpilot,
            &container.netscope,
            container.repo.clone(),
            cancel,
        ),
        Commands::Devpilot(dp) => match &dp.command {
            DevPilotCommand::Analyze { path } => {
                let req = AnalyzeRequest::new(path.clone());
                let analysis = container
                    .devpilot
                    .analyze(&req, &container.config, &cancel)?;
                if cli.json {
                    emit_json(&cli, &container, &analysis)
                } else {
                    emit_text(&cli, &container, &format_analysis_text(&analysis))
                }
            }
            DevPilotCommand::Health { path } => {
                let req = AnalyzeRequest::new(path.clone());
                let analysis = container
                    .devpilot
                    .analyze(&req, &container.config, &cancel)?;
                if cli.json {
                    emit_json(&cli, &container, &analysis.health)
                } else {
                    emit_text(
                        &cli,
                        &container,
                        &format_health_text(&analysis.project.name, &analysis.health),
                    )
                }
            }
            DevPilotCommand::Git { path, commits } => {
                let summary =
                    container
                        .devpilot
                        .inspect_git(path, *commits, &container.config)?;
                if cli.json {
                    emit_json(&cli, &container, &summary)
                } else {
                    emit_text(&cli, &container, &format_git_text(&summary))
                }
            }
            DevPilotCommand::Docs { path } => {
                let markdown =
                    container
                        .devpilot
                        .generate_docs(path, &container.config, &cancel)?;
                if cli.json {
                    emit_json(
                        &cli,
                        &container,
                        &serde_json::json!({ "path": path, "markdown": markdown }),
                    )
                } else {
                    emit_text(&cli, &container, &markdown)
                }
            }
        },
        Commands::Netscope(ns) => match &ns.command {
            NetScopeCommand::Interfaces => {
                let ifaces = container.netscope.list_interfaces(&container.config)?;
                if cli.json {
                    emit_json(&cli, &container, &ifaces)
                } else {
                    let mut out = String::new();
                    let _ = writeln!(out, "Interfaces de Rede ({}):", ifaces.len());
                    for i in &ifaces {
                        let ips = if i.ip_addresses.is_empty() {
                            "-".to_string()
                        } else {
                            i.ip_addresses.join(", ")
                        };
                        let _ = writeln!(
                            out,
                            "  - {:<18} [{}]  RX: {:<10}  TX: {:<10}  IPs: {}",
                            i.name,
                            i.status,
                            format_bytes(i.bytes_received),
                            format_bytes(i.bytes_sent),
                            ips
                        );
                    }
                    emit_text(&cli, &container, &out)
                }
            }
            NetScopeCommand::Monitor {
                samples,
                interval_ms,
            } => {
                let interval = Duration::from_millis(
                    interval_ms
                        .unwrap_or(container.config.netscope.refresh_interval_ms)
                        .max(100),
                );
                let mut collected = Vec::new();
                let mut idx = 0u32;

                loop {
                    if cancel.is_cancelled() {
                        break;
                    }
                    let report = container.netscope.sample_traffic(&container.config)?;
                    if !cli.json && !cli.quiet {
                        if idx == 0 {
                            println!(
                                "[amostra 1] Coletando baseline de {} interface(s)...",
                                report.interfaces.len()
                            );
                        } else {
                            println!(
                                "[amostra {} - {}]",
                                idx + 1,
                                report.sampled_at.format("%H:%M:%S")
                            );
                            for r in report.rates.iter().take(6) {
                                println!(
                                    "  {:<18} RX: {:>8.2} KB/s ({:.3} Mbps) | TX: {:>8.2} KB/s ({:.3} Mbps)",
                                    r.interface_name,
                                    r.rx_bytes_per_sec / 1024.0,
                                    r.rx_mbps,
                                    r.tx_bytes_per_sec / 1024.0,
                                    r.tx_mbps
                                );
                            }
                        }
                    }
                    collected.push(report);
                    idx += 1;
                    if *samples > 0 && idx >= *samples {
                        break;
                    }

                    tokio::select! {
                        _ = tokio::time::sleep(interval) => {}
                        _ = cancel.cancelled() => break,
                    }
                }

                if cli.json {
                    emit_json(&cli, &container, &collected)?;
                } else if let Some(out_path) = &cli.output {
                    let rendered = serde_json::to_string_pretty(&collected)
                        .map_err(|e| ArgusError::ExportError(e.to_string()))?;
                    container.fs.write_file(out_path, &rendered, cli.force)?;
                }
                Ok(())
            }
            NetScopeCommand::Ping {
                target,
                count,
                timeout_ms,
                port,
            } => {
                let report = container.netscope.measure_latency(
                    target,
                    *count,
                    Duration::from_millis(*timeout_ms),
                    *port,
                    &container.config,
                    &cancel,
                )?;
                if cli.json {
                    emit_json(&cli, &container, &report)
                } else {
                    let mut out = String::new();
                    let _ = writeln!(
                        out,
                        "Teste de Latência — {} ({}) [{}]",
                        report.target,
                        report.resolved_ip.as_deref().unwrap_or("?"),
                        report.method
                    );
                    for s in &report.samples {
                        match s.rtt_ms {
                            Some(ms) => {
                                let note = s
                                    .error
                                    .as_deref()
                                    .map(|e| format!(" ({e})"))
                                    .unwrap_or_default();
                                let _ = writeln!(out, "  seq={} rtt={:.2} ms{}", s.sequence, ms, note);
                            }
                            None => {
                                let _ = writeln!(
                                    out,
                                    "  seq={} falha: {}",
                                    s.sequence,
                                    s.error.as_deref().unwrap_or("timeout")
                                );
                            }
                        }
                    }
                    let _ = writeln!(
                        out,
                        "Resumo: {}/{} recebidos ({:.1}% perda)",
                        report.received, report.sent, report.packet_loss_pct
                    );
                    if let (Some(min), Some(avg), Some(max)) =
                        (report.min_ms, report.avg_ms, report.max_ms)
                    {
                        let _ = writeln!(
                            out,
                            "RTT (ms): min={min:.2} | média={avg:.2} | máx={max:.2}"
                        );
                    }
                    if let Some(note) = &report.privilege_note {
                        let _ = writeln!(out, "Nota: {note}");
                    }
                    emit_text(&cli, &container, &out)
                }
            }
            NetScopeCommand::Connections { protocol } => {
                let proto = protocol.map(Into::into);
                let (conns, summary) =
                    container
                        .netscope
                        .inspect_connections(proto, &container.config)?;
                if cli.json {
                    emit_json(
                        &cli,
                        &container,
                        &serde_json::json!({ "summary": summary, "connections": conns }),
                    )
                } else {
                    let mut out = String::new();
                    let _ = writeln!(
                        out,
                        "Conexões Ativas: {} total (TCP: {}, UDP: {}, ESTABLISHED: {}, LISTEN: {})",
                        summary.total,
                        summary.tcp_count,
                        summary.udp_count,
                        summary.established_count,
                        summary.listening_count
                    );
                    for c in conns.iter().take(50) {
                        let remote = match (&c.remote_address, c.remote_port) {
                            (Some(a), Some(p)) => format!("{a}:{p}"),
                            _ => "-".to_string(),
                        };
                        let proc_label = c
                            .process_name
                            .as_deref()
                            .unwrap_or("-");
                        let _ = writeln!(
                            out,
                            "  {:<4} {:<24} -> {:<24} {:<12} {}",
                            c.protocol,
                            format!("{}:{}", c.local_address, c.local_port),
                            remote,
                            c.state,
                            proc_label
                        );
                    }
                    if conns.len() > 50 {
                        let _ = writeln!(out, "  ... e mais {} conexão(ões) (use --json para saída completa).", conns.len() - 50);
                    }
                    emit_text(&cli, &container, &out)
                }
            }
            NetScopeCommand::Ports {
                host,
                ports,
                timeout_ms,
            } => {
                let results = container.netscope.check_authorized_ports(
                    host,
                    ports,
                    Duration::from_millis(*timeout_ms),
                    &container.config,
                    &cancel,
                )?;
                if cli.json {
                    emit_json(&cli, &container, &results)
                } else {
                    let mut out = String::new();
                    let _ = writeln!(out, "Verificação de Portas Autorizada — {host}:");
                    for r in &results {
                        let status = if r.open { "ABERTA" } else { "fechada" };
                        let rtt = r
                            .latency_ms
                            .map(|ms| format!(" ({ms:.2} ms)"))
                            .unwrap_or_default();
                        let _ = writeln!(out, "  porta {:<5} : {}{}", r.port, status, rtt);
                    }
                    emit_text(&cli, &container, &out)
                }
            }
            NetScopeCommand::Discover { cidr, timeout_ms } => {
                let hosts = container.netscope.discover_authorized_network(
                    cidr,
                    Duration::from_millis(*timeout_ms),
                    &container.config,
                    &cancel,
                )?;
                if cli.json {
                    emit_json(&cli, &container, &hosts)
                } else {
                    let mut out = String::new();
                    let _ = writeln!(out, "Descoberta Autorizada ({cidr}) — {} host(s) ativo(s):", hosts.len());
                    for h in &hosts {
                        let _ = writeln!(
                            out,
                            "  - {} | RTT: {} | Portas abertas: {:?}",
                            h.ip,
                            h.rtt_ms
                                .map(|v| format!("{v:.2} ms"))
                                .unwrap_or_else(|| "-".to_string()),
                            h.open_ports
                        );
                    }
                    emit_text(&cli, &container, &out)
                }
            }
            NetScopeCommand::Alerts { all } => {
                let alerts = container.netscope.list_alerts(!*all, 50)?;
                if cli.json {
                    emit_json(&cli, &container, &alerts)
                } else {
                    let mut out = String::new();
                    if alerts.is_empty() {
                        let _ = writeln!(out, "Nenhum alerta encontrado.");
                    } else {
                        let _ = writeln!(out, "Alertas ({}):", alerts.len());
                        for a in &alerts {
                            let _ = writeln!(
                                out,
                                "  #{} [{} / {}] {} ({})",
                                a.id,
                                a.module,
                                a.severity,
                                a.message,
                                a.created_at.format("%Y-%m-%d %H:%M UTC")
                            );
                        }
                    }
                    emit_text(&cli, &container, &out)
                }
            }
        },
        Commands::History { limit } => {
            let runs = container
                .repo
                .as_ref()
                .map(|r| r.list_analysis_runs(*limit))
                .transpose()?
                .unwrap_or_default();
            let metrics = container.netscope.metric_history(None, *limit)?;
            let alerts = container.netscope.list_alerts(false, *limit)?;
            if cli.json {
                emit_json(
                    &cli,
                    &container,
                    &serde_json::json!({
                        "analysis_runs": runs,
                        "network_metrics": metrics,
                        "alerts": alerts
                    }),
                )
            } else {
                let mut out = String::new();
                let _ = writeln!(out, "Histórico DevPilot ({} execução(ões)):", runs.len());
                for r in &runs {
                    let _ = writeln!(
                        out,
                        "  #{} | {} | saúde {}/100 | {}",
                        r.id,
                        r.started_at.format("%Y-%m-%d %H:%M UTC"),
                        r.score,
                        r.project_path
                    );
                }
                let _ = writeln!(out, "\nHistórico NetScope ({} métrica(s)):", metrics.len());
                for m in &metrics {
                    let _ = writeln!(
                        out,
                        "  #{} | {} | {} | RX={} TX={} | lat={}",
                        m.id,
                        m.timestamp.format("%Y-%m-%d %H:%M UTC"),
                        m.interface_name,
                        format_bytes(m.bytes_received),
                        format_bytes(m.bytes_sent),
                        m.latency_ms
                            .map(|v| format!("{v:.2} ms"))
                            .unwrap_or_else(|| "-".to_string())
                    );
                }
                let _ = writeln!(out, "\nAlertas ({}):", alerts.len());
                for a in &alerts {
                    let _ = writeln!(out, "  #{} [{}] {}", a.id, a.severity, a.message);
                }
                emit_text(&cli, &container, &out)
            }
        }
        Commands::Report { path, format, ping } => {
            let project_analysis = if container.config.modules.devpilot_enabled && path.exists() {
                let req = AnalyzeRequest::new(path.clone());
                Some(container.devpilot.analyze(&req, &container.config, &cancel)?)
            } else {
                None
            };
            let network_interfaces = if container.config.modules.netscope_enabled {
                container.netscope.list_interfaces(&container.config)?
            } else {
                Vec::new()
            };
            let latency = if let Some(target) = ping {
                Some(container.netscope.measure_latency(
                    target,
                    3,
                    Duration::from_millis(2000),
                    None,
                    &container.config,
                    &cancel,
                )?)
            } else {
                None
            };
            let alerts = container.netscope.list_alerts(false, 20)?;

            let report = ArgusReport {
                generator: APP_NAME.to_string(),
                version: APP_VERSION.to_string(),
                generated_at: Utc::now(),
                project_analysis,
                network_interfaces,
                latency,
                alerts,
            };

            let effective_format = if cli.json {
                ReportFormat::Json
            } else {
                (*format).into()
            };
            let rendered = container.exporter.render_report(&report, effective_format)?;
            emit_text(&cli, &container, &rendered)
        }
    }
}

fn handle_init(cli: &Cli) -> ArgusResult<()> {
    let target = match &cli.config {
        Some(p) => p.clone(),
        None => ArgusConfig::default_config_path()?,
    };
    if target.exists() && !cli.force {
        return Err(ArgusError::ConfigurationError(format!(
            "Arquivo de configuração já existe em '{}'. Use --force para recriar.",
            target.display()
        )));
    }
    let cfg = ArgusConfig::default();
    cfg.save(&target)?;

    let db_path = if cfg.storage.database_enabled {
        let p = cfg.resolve_database_path()?;
        let _ = SQLiteMetricsAdapter::open(&p)?;
        Some(p)
    } else {
        None
    };

    if cli.json {
        let payload = serde_json::json!({
            "status": "initialized",
            "config_path": target,
            "database_path": db_path
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&payload)
                .map_err(|e| ArgusError::ExportError(e.to_string()))?
        );
    } else if !cli.quiet {
        println!("{APP_NAME} v{APP_VERSION} inicializado com sucesso.");
        println!("  Configuração: {}", target.display());
        if let Some(db) = db_path {
            println!("  Banco SQLite: {}", db.display());
        }
    }
    Ok(())
}

fn handle_doctor(cli: &Cli, container: &AppContainer) -> ArgusResult<()> {
    let mut checks = Vec::new();

    let cfg_path = cli
        .config
        .clone()
        .or_else(|| ArgusConfig::default_config_path().ok());
    checks.push(DiagnosticCheck {
        name: "Configuração TOML".to_string(),
        ok: true,
        detail: cfg_path
            .map(|p| {
                if p.exists() {
                    format!("Carregada de {}", p.display())
                } else {
                    format!("Usando defaults (crie com `argus init` em {})", p.display())
                }
            })
            .unwrap_or_else(|| "Usando configuração padrão".to_string()),
    });

    checks.push(DiagnosticCheck {
        name: "Persistência SQLite".to_string(),
        ok: container.repo.is_some() || !container.config.storage.database_enabled,
        detail: if !container.config.storage.database_enabled {
            "Desativada por configuração".to_string()
        } else if container.repo.is_some() {
            format!(
                "Ativa ({})",
                container
                    .config
                    .resolve_database_path()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|_| "argus.db".to_string())
            )
        } else {
            "Indisponível (modo degradado sem histórico)".to_string()
        },
    });

    match container.netscope.list_interfaces(&container.config) {
        Ok(ifaces) => checks.push(DiagnosticCheck {
            name: "NetScope Interfaces".to_string(),
            ok: !ifaces.is_empty(),
            detail: format!("{} interface(s) detectada(s)", ifaces.len()),
        }),
        Err(err) => checks.push(DiagnosticCheck {
            name: "NetScope Interfaces".to_string(),
            ok: false,
            detail: err.friendly_message(),
        }),
    }

    checks.push(DiagnosticCheck {
        name: "Módulos Ativos".to_string(),
        ok: true,
        detail: format!(
            "DevPilot={} | NetScope={}",
            container.config.modules.devpilot_enabled, container.config.modules.netscope_enabled
        ),
    });

    let report = DoctorReport {
        app: APP_NAME.to_string(),
        version: APP_VERSION.to_string(),
        os: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        checked_at: Utc::now(),
        checks,
    };

    if cli.json {
        emit_json(cli, container, &report)
    } else {
        let mut out = String::new();
        let _ = writeln!(
            out,
            "{} v{} — Doctor ({}/{})",
            report.app, report.version, report.os, report.arch
        );
        for c in &report.checks {
            let icon = if c.ok { "[OK]" } else { "[WARN]" };
            let _ = writeln!(out, "  {icon:<6} {}: {}", c.name, c.detail);
        }
        emit_text(cli, container, &out)
    }
}

fn emit_json<T: serde::Serialize>(
    cli: &Cli,
    container: &AppContainer,
    value: &T,
) -> ArgusResult<()> {
    let json_val = to_json_value(value)?;
    let rendered = container.exporter.render_json_value(&json_val)?;
    emit_text(cli, container, &rendered)
}

fn emit_text(cli: &Cli, container: &AppContainer, text: &str) -> ArgusResult<()> {
    if let Some(out_path) = &cli.output {
        container.fs.write_file(out_path, text, cli.force)?;
        if !cli.quiet {
            println!("Saída salva em {}", out_path.display());
        }
    } else if !cli.quiet || cli.json {
        println!("{text}");
    }
    Ok(())
}

fn format_analysis_text(a: &ProjectAnalysis) -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "DevPilot — Análise de '{}' ({})",
        a.project.name,
        a.project.root_path.display()
    );
    let _ = writeln!(
        out,
        "  Arquivos: {} | Diretórios: {} | Linhas: {} | Tamanho: {} | Tempo: {} ms",
        a.project.total_files,
        a.project.total_dirs,
        a.project.total_lines,
        format_bytes(a.project.total_bytes),
        a.duration_ms
    );
    let _ = writeln!(
        out,
        "  Saúde: {}/100 ({}) | Dependências: {} | TODO/FIXME: {} | Arquivos grandes: {} | Ciclos: {}",
        a.health.score,
        a.health.grade,
        a.dependencies.len(),
        a.todos.len(),
        a.large_files.len(),
        a.circular_dependencies.len()
    );

    if !a.languages.is_empty() {
        let _ = writeln!(out, "\nLinguagens:");
        for l in a.languages.iter().take(8) {
            let _ = writeln!(
                out,
                "  - {:<18} {:>4} arq | {:>7} linhas | {:>6.2}%",
                l.language, l.files, l.lines, l.percentage
            );
        }
    }

    if !a.frameworks.is_empty() {
        let _ = writeln!(out, "\nFrameworks e Ferramentas:");
        for fw in &a.frameworks {
            let _ = writeln!(out, "  - {} ({}) [{}]", fw.name, fw.category, fw.evidence);
        }
    }

    if let Some(git) = &a.git {
        let _ = writeln!(
            out,
            "\nGit: branch '{}' | {} | {} commit(s)",
            git.branch,
            if git.is_dirty { "alterado" } else { "limpo" },
            git.total_commits
        );
    }

    out
}

fn format_health_text(name: &str, h: &HealthReport) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "DevPilot — Saúde de '{name}': {}/100 (Nota {})", h.score, h.grade);
    let _ = writeln!(out, "\nVerificações aprovadas:");
    for p in &h.passed_checks {
        let _ = writeln!(out, "  [OK] {p}");
    }
    if !h.issues.is_empty() {
        let _ = writeln!(out, "\nAchados e Recomendações:");
        for i in &h.issues {
            let _ = writeln!(out, "  [{:?} / {}] {}", i.severity, i.code, i.title);
            let _ = writeln!(out, "    -> {}", i.recommendation);
        }
    }
    out
}

fn format_git_text(g: &GitSummary) -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "DevPilot — Git Insights\n  Branch: {}\n  Estado: {}\n  Commits analisados: {}\n  Contribuidores: {}",
        g.branch,
        if g.is_dirty { "Alterações não commitadas" } else { "Limpo" },
        g.total_commits,
        g.contributors.join(", ")
    );
    if !g.modified_files.is_empty() {
        let _ = writeln!(out, "\nArquivos modificados:");
        for f in g.modified_files.iter().take(20) {
            let _ = writeln!(out, "  [{:<15}] {}", f.status, f.path);
        }
    }
    if !g.recent_commits.is_empty() {
        let _ = writeln!(out, "\nCommits recentes:");
        for c in &g.recent_commits {
            let _ = writeln!(
                out,
                "  {} | {} | {} — {}",
                c.short_hash,
                c.timestamp.format("%Y-%m-%d %H:%M"),
                c.author,
                c.summary
            );
        }
    }
    out
}
