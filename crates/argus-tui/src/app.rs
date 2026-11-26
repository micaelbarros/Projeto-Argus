//! Main TUI application state machine and safe terminal lifecycle runner.

use crate::events::{poll_action, UiAction};
use crate::screens;
use crate::theme::ArgusTheme;
use crate::widgets;
use argus_core::config::ArgusConfig;
use argus_core::domain::alert::Alert;
use argus_core::domain::network::{
    ConnectionInfo, NetworkInterface, NetworkMetric, TransferRate,
};
use argus_core::domain::project::{AnalysisRunRecord, ProjectAnalysis};
use argus_core::errors::ArgusResult;
use argus_core::ports::persistence::MetricsRepositoryPort;
use argus_devpilot::domain::AnalyzeRequest;
use argus_devpilot::DevPilotService;
use argus_netscope::NetScopeService;
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::Terminal;
use std::io::{self, Stdout};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio_util::sync::CancellationToken;

/// Top-level navigation screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Dashboard,
    DevPilot,
    NetScope,
    History,
    Settings,
}

/// Complete state of the interactive ARGUS TUI.
pub struct ArgusTuiApp {
    pub config: ArgusConfig,
    pub theme: ArgusTheme,
    pub target_path: PathBuf,
    pub screen: Screen,
    pub devpilot_subtab: usize,
    pub netscope_subtab: usize,
    pub scroll: usize,
    pub show_help: bool,
    pub status_message: String,
    pub analysis: Option<ProjectAnalysis>,
    pub interfaces: Vec<NetworkInterface>,
    pub rates: Vec<TransferRate>,
    pub connections: Vec<ConnectionInfo>,
    pub throughput_history: Vec<u64>,
    pub analysis_history: Vec<AnalysisRunRecord>,
    pub metric_history: Vec<NetworkMetric>,
    pub alerts: Vec<Alert>,
}

impl ArgusTuiApp {
    pub fn new(config: ArgusConfig, target_path: PathBuf) -> Self {
        let theme = ArgusTheme::from_name(&config.general.theme);
        Self {
            config,
            theme,
            target_path,
            screen: Screen::Dashboard,
            devpilot_subtab: 0,
            netscope_subtab: 0,
            scroll: 0,
            show_help: false,
            status_message: "Pronto".to_string(),
            analysis: None,
            interfaces: Vec::new(),
            rates: Vec::new(),
            connections: Vec::new(),
            throughput_history: Vec::new(),
            analysis_history: Vec::new(),
            metric_history: Vec::new(),
            alerts: Vec::new(),
        }
    }

    pub fn refresh_all(
        &mut self,
        devpilot: &DevPilotService,
        netscope: &NetScopeService,
        repo: Option<&Arc<dyn MetricsRepositoryPort>>,
        cancel: &CancellationToken,
    ) {
        if self.config.modules.devpilot_enabled {
            let req = AnalyzeRequest::new(self.target_path.clone());
            match devpilot.analyze(&req, &self.config, cancel) {
                Ok(a) => self.analysis = Some(a),
                Err(e) => self.status_message = e.friendly_message(),
            }
        }
        self.refresh_network(netscope);
        self.refresh_history(repo);
        self.status_message = format!(
            "Atualizado em {}",
            chrono::Local::now().format("%H:%M:%S")
        );
    }

    pub fn refresh_network(&mut self, netscope: &NetScopeService) {
        if !self.config.modules.netscope_enabled {
            return;
        }
        if let Ok(sample) = netscope.sample_traffic(&self.config) {
            let total_kbs: u64 = sample
                .rates
                .iter()
                .map(|r| ((r.rx_bytes_per_sec + r.tx_bytes_per_sec) / 1024.0) as u64)
                .sum();
            self.throughput_history.push(total_kbs);
            if self.throughput_history.len() > 40 {
                self.throughput_history.remove(0);
            }
            self.interfaces = sample.interfaces;
            self.rates = sample.rates;
        }
        if let Ok((conns, _)) = netscope.inspect_connections(None, &self.config) {
            self.connections = conns;
        }
    }

    pub fn refresh_history(&mut self, repo: Option<&Arc<dyn MetricsRepositoryPort>>) {
        if let Some(r) = repo {
            if let Ok(runs) = r.list_analysis_runs(25) {
                self.analysis_history = runs;
            }
            if let Ok(metrics) = r.list_network_metrics(None, 25) {
                self.metric_history = metrics;
            }
            if let Ok(alerts) = r.list_alerts(false, 25) {
                self.alerts = alerts;
            }
        }
    }
}

/// RAII guard restoring terminal mode even on panic or early return.
struct TerminalGuard {
    terminal: Terminal<CrosstermBackend<Stdout>>,
}

impl TerminalGuard {
    fn init() -> io::Result<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;
        Ok(Self { terminal })
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(self.terminal.backend_mut(), LeaveAlternateScreen);
        let _ = self.terminal.show_cursor();
    }
}

/// Launches the interactive TUI event loop.
pub fn run_tui(
    config: ArgusConfig,
    target_path: PathBuf,
    devpilot: &DevPilotService,
    netscope: &NetScopeService,
    repo: Option<Arc<dyn MetricsRepositoryPort>>,
    cancel: CancellationToken,
) -> ArgusResult<()> {
    let mut guard = TerminalGuard::init()?;
    let refresh_interval = Duration::from_millis(config.netscope.refresh_interval_ms.max(250));
    let mut app = ArgusTuiApp::new(config, target_path);
    app.refresh_all(devpilot, netscope, repo.as_ref(), &cancel);

    let mut last_net_tick = Instant::now();
    let poll_rate = Duration::from_millis(200);

    loop {
        if cancel.is_cancelled() {
            break;
        }

        guard.terminal.draw(|frame| {
            let area = frame.area();
            let layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Min(8),
                    Constraint::Length(3),
                ])
                .split(area);

            widgets::render_header(frame, layout[0], app.screen, app.theme);

            match app.screen {
                Screen::Dashboard => screens::dashboard::render(frame, layout[1], &app),
                Screen::DevPilot => screens::devpilot::render(frame, layout[1], &app),
                Screen::NetScope => screens::netscope::render(frame, layout[1], &app),
                Screen::History => screens::history::render(frame, layout[1], &app),
                Screen::Settings => screens::settings::render(frame, layout[1], &app),
            }

            widgets::render_footer(frame, layout[2], &app.status_message, app.theme);

            if app.show_help {
                widgets::render_help_modal(frame, area, app.theme);
            }
        })?;

        match poll_action(poll_rate)? {
            UiAction::Quit => {
                if app.show_help {
                    app.show_help = false;
                } else {
                    break;
                }
            }
            UiAction::ShowDashboard => {
                app.screen = Screen::Dashboard;
                app.scroll = 0;
            }
            UiAction::ShowDevPilot => {
                app.screen = Screen::DevPilot;
                app.scroll = 0;
            }
            UiAction::ShowNetScope => {
                app.screen = Screen::NetScope;
                app.scroll = 0;
            }
            UiAction::ShowHistory => {
                app.screen = Screen::History;
                app.scroll = 0;
                app.refresh_history(repo.as_ref());
            }
            UiAction::ShowSettings => {
                app.screen = Screen::Settings;
                app.scroll = 0;
            }
            UiAction::NextSubTab => {
                app.scroll = 0;
                match app.screen {
                    Screen::DevPilot => app.devpilot_subtab = (app.devpilot_subtab + 1) % 4,
                    Screen::NetScope => app.netscope_subtab = (app.netscope_subtab + 1) % 2,
                    _ => {}
                }
            }
            UiAction::PrevSubTab => {
                app.scroll = 0;
                match app.screen {
                    Screen::DevPilot => app.devpilot_subtab = (app.devpilot_subtab + 3) % 4,
                    Screen::NetScope => app.netscope_subtab = (app.netscope_subtab + 1) % 2,
                    _ => {}
                }
            }
            UiAction::Refresh => {
                app.refresh_all(devpilot, netscope, repo.as_ref(), &cancel);
                last_net_tick = Instant::now();
            }
            UiAction::ToggleHelp => {
                app.show_help = !app.show_help;
            }
            UiAction::ScrollDown => {
                app.scroll = app.scroll.saturating_add(1);
            }
            UiAction::ScrollUp => {
                app.scroll = app.scroll.saturating_sub(1);
            }
            UiAction::Tick => {}
        }

        if last_net_tick.elapsed() >= refresh_interval {
            app.refresh_network(netscope);
            last_net_tick = Instant::now();
        }
    }

    Ok(())
}
