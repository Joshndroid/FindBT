use std::sync::mpsc::{self, Receiver};

use chrono::Local;
use findbt_backend::{BluetoothBackend, DefaultBluetoothBackend};
use findbt_core::{
    normalize_address, pdf, report, CaptureSession, CaseMetadata, HostAdapterInfo, RawObservation,
    ScanPhase, ScanPhaseRun,
};

use crate::{
    main_screen::{KindFilter, MainScreenAction, MainScreenState},
    theme::{AccentColor, Theme},
    wizard::{WizardAction, WizardState},
};

/// Output format for the generated report, chosen in the Settings view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportFormat {
    Html,
    Pdf,
}

pub struct FindBtApp {
    backend: DefaultBluetoothBackend,
    receiver: Option<Receiver<RawObservation>>,
    screen: Screen,
    theme: Theme,
    active_phase: ScanPhase,
    scanning_phase: Option<ScanPhase>,
    current_run: Option<ScanPhaseRun>,
    status: String,
    filter_text: String,
    kind_filter: KindFilter,
    report_format: ReportFormat,
    show_settings: bool,
}

enum Screen {
    Wizard(WizardState),
    Main(CaptureSession),
}

impl FindBtApp {
    pub fn new() -> Self {
        let backend = DefaultBluetoothBackend::new();
        let detected_host = backend.default_adapter().unwrap_or_default();
        Self {
            backend,
            receiver: None,
            screen: Screen::Wizard(WizardState::new(detected_host)),
            theme: Theme::light(AccentColor::Blue),
            active_phase: ScanPhase::Baseline,
            scanning_phase: None,
            current_run: None,
            status: "Ready".to_string(),
            filter_text: String::new(),
            kind_filter: KindFilter::All,
            report_format: ReportFormat::Html,
            show_settings: false,
        }
    }

    fn begin_session(&mut self, metadata: CaseMetadata, host: HostAdapterInfo) {
        let mut session = CaptureSession::new(metadata, host);
        let local = normalize_address(&session.host.address);
        session.registry.apply_local_radio_tag(&local);
        self.screen = Screen::Main(session);
        self.active_phase = ScanPhase::Baseline;
        self.status = "Session ready. Start the baseline scan.".to_string();
    }

    fn start_scan(&mut self, phase: ScanPhase) {
        if self.scanning_phase.is_some() {
            return;
        }
        let (tx, rx) = mpsc::channel();
        match self.backend.start(tx, phase) {
            Ok(()) => {
                self.receiver = Some(rx);
                self.scanning_phase = Some(phase);
                self.current_run = Some(ScanPhaseRun {
                    phase,
                    started_at: Local::now(),
                    stopped_at: None,
                    stop_reason: String::new(),
                });
                self.status = format!("{} running.", phase.tab_title());
            }
            Err(err) => {
                self.status = format!("Scan could not start: {err}");
            }
        }
    }

    fn stop_scan(&mut self, reason: &str) {
        self.backend.stop();
        self.receiver = None;
        if let Some(mut run) = self.current_run.take() {
            run.stopped_at = Some(Local::now());
            run.stop_reason = reason.to_string();
            if let Screen::Main(session) = &mut self.screen {
                session.phase_runs.push(run);
            }
        }
        if let Some(phase) = self.scanning_phase.take() {
            self.status = format!("{} stopped.", phase.tab_title());
        }
    }

    fn drain_observations(&mut self) {
        let Some(receiver) = &self.receiver else {
            return;
        };
        if let Screen::Main(session) = &mut self.screen {
            while let Ok(observation) = receiver.try_recv() {
                session.record(observation);
            }
            let local = normalize_address(&session.host.address);
            session.registry.apply_local_radio_tag(&local);
        }
    }

    fn save_report(&mut self) {
        let Screen::Main(session) = &self.screen else {
            return;
        };
        let (filter_label, extension, default_name) = match self.report_format {
            ReportFormat::Html => ("HTML report", "html", "findbt-report.html"),
            ReportFormat::Pdf => ("PDF report", "pdf", "findbt-report.pdf"),
        };
        let Some(path) = rfd::FileDialog::new()
            .set_title("Generate FindBT Report")
            .add_filter(filter_label, &[extension])
            .set_file_name(default_name)
            .save_file()
        else {
            return;
        };
        let result = match self.report_format {
            ReportFormat::Html => std::fs::write(&path, report::generate_html(session)),
            ReportFormat::Pdf => std::fs::write(&path, pdf::generate_pdf(session)),
        };
        match result {
            Ok(()) => self.status = format!("Report saved to {}", path.display()),
            Err(err) => self.status = format!("Report save failed: {err}"),
        }
    }

    fn settings_window(&mut self, ctx: &egui::Context) {
        if !self.show_settings {
            return;
        }
        let mut open = self.show_settings;
        egui::Window::new("Settings")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.label(egui::RichText::new("Report generation").strong());
                ui.add_space(6.0);
                ui.radio_value(
                    &mut self.report_format,
                    ReportFormat::Html,
                    "HTML export (.html) - standalone web page, opens in any browser",
                );
                ui.radio_value(
                    &mut self.report_format,
                    ReportFormat::Pdf,
                    "PDF export (.pdf) - fixed-layout printable document",
                );
                ui.add_space(8.0);
                ui.label(
                    egui::RichText::new(
                        "Both formats contain the same phase runs, phase summary, device \
                         registry, and raw audit log.",
                    )
                    .size(11.0),
                );
            });
        self.show_settings = open;
    }

    fn apply_action(&mut self, action: MainScreenAction) {
        match action {
            MainScreenAction::Start(phase) => self.start_scan(phase),
            MainScreenAction::Stop => self.stop_scan("Stopped by operator"),
            MainScreenAction::Rescan(phase) => self.start_scan(phase),
            MainScreenAction::SelectPhase(phase) => self.active_phase = phase,
            MainScreenAction::GenerateReport => self.save_report(),
            MainScreenAction::ResetCapture => {
                self.backend.stop();
                self.receiver = None;
                self.scanning_phase = None;
                self.current_run = None;
                if let Screen::Main(session) = &mut self.screen {
                    session.reset_capture();
                }
                self.active_phase = ScanPhase::Baseline;
                self.status = "Capture reset.".to_string();
            }
            MainScreenAction::SetKindFilter(kind) => self.kind_filter = kind,
            MainScreenAction::SetFilterText(text) => self.filter_text = text,
            MainScreenAction::OpenSettings => self.show_settings = true,
        }
    }
}

impl eframe::App for FindBtApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.drain_observations();
        ui.request_repaint_after(std::time::Duration::from_millis(100));

        match &mut self.screen {
            Screen::Wizard(state) => match state.ui(ui, self.theme) {
                WizardAction::None => {}
                WizardAction::Begin { metadata, host } => self.begin_session(metadata, host),
            },
            Screen::Main(session) => {
                let action = crate::main_screen::show(
                    ui,
                    session,
                    MainScreenState {
                        theme: self.theme,
                        active_phase: self.active_phase,
                        scanning_phase: self.scanning_phase,
                        status: &self.status,
                        filter_text: &self.filter_text,
                        kind_filter: self.kind_filter,
                    },
                );
                if let Some(action) = action {
                    self.apply_action(action);
                }
            }
        }

        let ctx = ui.ctx().clone();
        self.settings_window(&ctx);
    }
}
