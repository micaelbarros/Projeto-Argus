//! DefaultReportExporter — JSON and Markdown report formatting adapter.

use argus_core::application::services::format_bytes;
use argus_core::domain::report::{ArgusReport, ReportFormat};
use argus_core::errors::{ArgusError, ArgusResult};
use argus_core::ports::exporter::ReportExporterPort;
use std::fmt::Write;

/// Concrete exporter rendering `ArgusReport` to JSON or Markdown.
#[derive(Debug, Default, Clone)]
pub struct DefaultReportExporter;

impl DefaultReportExporter {
    pub fn new() -> Self {
        Self
    }

    fn render_markdown(report: &ArgusReport) -> String {
        let mut out = String::new();
        let _ = writeln!(out, "# Relatório Técnico — {}\n", report.generator);
        let _ = writeln!(out, "- **Versão:** {}", report.version);
        let _ = writeln!(
            out,
            "- **Gerado em:** {}\n",
            report.generated_at.format("%Y-%m-%d %H:%M:%S UTC")
        );

        if let Some(analysis) = &report.project_analysis {
            let _ = writeln!(out, "## DevPilot — Análise de Projeto\n");
            let _ = writeln!(out, "- **Projeto:** `{}`", analysis.project.name);
            let _ = writeln!(
                out,
                "- **Saúde:** **{}/100 ({})**",
                analysis.health.score, analysis.health.grade
            );
            let _ = writeln!(
                out,
                "- **Arquivos:** {} ({} linhas, {})",
                analysis.project.total_files,
                analysis.project.total_lines,
                format_bytes(analysis.project.total_bytes)
            );
            let _ = writeln!(
                out,
                "- **Dependências:** {} | **TODO/FIXME:** {} | **Ciclos:** {}\n",
                analysis.dependencies.len(),
                analysis.todos.len(),
                analysis.circular_dependencies.len()
            );

            if !analysis.languages.is_empty() {
                let _ = writeln!(out, "### Linguagens\n");
                let _ = writeln!(out, "| Linguagem | Arquivos | Linhas | % |");
                let _ = writeln!(out, "|---|---:|---:|---:|");
                for lang in &analysis.languages {
                    let _ = writeln!(
                        out,
                        "| {} | {} | {} | {:.2}% |",
                        lang.language, lang.files, lang.lines, lang.percentage
                    );
                }
                let _ = writeln!(out);
            }
        }

        if !report.network_interfaces.is_empty() {
            let _ = writeln!(out, "## NetScope — Interfaces de Rede\n");
            let _ = writeln!(out, "| Interface | Estado | RX | TX | IPs |");
            let _ = writeln!(out, "|---|---|---:|---:|---|");
            for iface in &report.network_interfaces {
                let ips = if iface.ip_addresses.is_empty() {
                    "-".to_string()
                } else {
                    iface.ip_addresses.join(", ")
                };
                let _ = writeln!(
                    out,
                    "| `{}` | {} | {} | {} | {} |",
                    iface.name,
                    iface.status,
                    format_bytes(iface.bytes_received),
                    format_bytes(iface.bytes_sent),
                    ips
                );
            }
            let _ = writeln!(out);
        }

        if let Some(lat) = &report.latency {
            let _ = writeln!(out, "## NetScope — Teste de Latência\n");
            let _ = writeln!(out, "- **Alvo:** `{}` ({})", lat.target, lat.method);
            let _ = writeln!(
                out,
                "- **Pacotes:** {}/{} recebidos ({:.1}% perda)",
                lat.received, lat.sent, lat.packet_loss_pct
            );
            if let (Some(min), Some(avg), Some(max)) = (lat.min_ms, lat.avg_ms, lat.max_ms) {
                let _ = writeln!(
                    out,
                    "- **RTT (ms):** min `{min:.2}` | média `{avg:.2}` | máx `{max:.2}`\n"
                );
            }
        }

        if !report.alerts.is_empty() {
            let _ = writeln!(out, "## Alertas Recentes\n");
            for alert in &report.alerts {
                let _ = writeln!(
                    out,
                    "- **[{} / {}]** {} ({})",
                    alert.module,
                    alert.severity,
                    alert.message,
                    alert.created_at.format("%Y-%m-%d %H:%M UTC")
                );
            }
            let _ = writeln!(out);
        }

        out
    }
}

impl ReportExporterPort for DefaultReportExporter {
    fn render_report(&self, report: &ArgusReport, format: ReportFormat) -> ArgusResult<String> {
        match format {
            ReportFormat::Json => serde_json::to_string_pretty(report)
                .map_err(|e| ArgusError::ExportError(e.to_string())),
            ReportFormat::Markdown => Ok(Self::render_markdown(report)),
        }
    }

    fn render_json_value(&self, value: &serde_json::Value) -> ArgusResult<String> {
        serde_json::to_string_pretty(value).map_err(|e| ArgusError::ExportError(e.to_string()))
    }
}
