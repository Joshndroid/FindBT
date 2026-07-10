use std::sync::mpsc::{self, Receiver};

use chrono::Local;
use findbt_backend::{BluetoothBackend, DefaultBluetoothBackend};
use findbt_core::{
    normalize_address, pdf, report, CaptureSession, CaseMetadata, HostAdapterInfo, RawObservation,
    ScanPhase, ScanPhaseRun,
};

use crate::{
    main_screen::{KindFilter, MainScreenAction, MainScreenState},
    settings::{AppSettings, ThemeSetting},
    settings_screen::{SettingsScreenAction, SettingsScreenState, SettingsSection},
    theme::{AccentColor, Theme},
    widgets,
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
    route: MainRoute,
    settings_section: SettingsSection,
    report_error_open: bool,
    scan_console_open: bool,
    scan_console_lines: Vec<String>,
    settings: AppSettings,
    /// The real bundled app icon, uploaded to the GPU once at startup and
    /// reused everywhere the icon is shown (titlebar, wizard) so it never
    /// has to be decoded or re-uploaded per frame.
    app_icon: egui::TextureHandle,
}

/// Decode the bundled app icon PNG and upload it as a texture. Reuses
/// `eframe::icon_data::from_png_bytes`, the same decoder already used for
/// the OS window/taskbar icon in `main.rs`, so the wizard and titlebar show
/// the identical, official icon rather than a placeholder.
///
/// The source art is a flat, fully opaque 256x256 square (no alpha channel,
/// solid black outside the rounded icon graphic) and is only ever shown
/// small (20-40px) in the UI. It is down-sampled here with a box filter
/// before upload so it stays crisp instead of relying on the GPU to minify
/// a 256px texture down to a few dozen pixels on the fly, which looks
/// blurry. The remaining flat black corners are clipped off separately by
/// `logo()`, which draws the texture with a rounded `corner_radius`.
fn load_app_icon_texture(ctx: &egui::Context) -> egui::TextureHandle {
    let icon = eframe::icon_data::from_png_bytes(include_bytes!("../assets/app-icon-256.png"))
        .expect("bundled app icon must be a valid PNG");
    let width = icon.width as usize;
    let height = icon.height as usize;
    let factor = (width / 64).max(1);
    let (rgba, out_w, out_h) = downsample_rgba_box(&icon.rgba, width, height, factor);
    let image = egui::ColorImage::from_rgba_unmultiplied([out_w, out_h], &rgba);
    ctx.load_texture("findbt-app-icon", image, egui::TextureOptions::LINEAR)
}

/// Shrink an RGBA buffer by an integer `factor` using a simple box filter
/// (each output pixel is the average of the corresponding `factor x factor`
/// block of input pixels). Implemented by hand rather than pulling in an
/// image-resizing crate, since this is the only place the app needs it.
fn downsample_rgba_box(
    rgba: &[u8],
    width: usize,
    height: usize,
    factor: usize,
) -> (Vec<u8>, usize, usize) {
    let factor = factor.max(1);
    if factor == 1 {
        return (rgba.to_vec(), width, height);
    }
    let out_w = (width / factor).max(1);
    let out_h = (height / factor).max(1);
    let mut out = vec![0u8; out_w * out_h * 4];
    for oy in 0..out_h {
        for ox in 0..out_w {
            let mut sum = [0u32; 4];
            let mut count = 0u32;
            for dy in 0..factor {
                let y = oy * factor + dy;
                if y >= height {
                    continue;
                }
                for dx in 0..factor {
                    let x = ox * factor + dx;
                    if x >= width {
                        continue;
                    }
                    let idx = (y * width + x) * 4;
                    for c in 0..4 {
                        sum[c] += rgba[idx + c] as u32;
                    }
                    count += 1;
                }
            }
            let count = count.max(1);
            let out_idx = (oy * out_w + ox) * 4;
            for c in 0..4 {
                out[out_idx + c] = (sum[c] / count) as u8;
            }
        }
    }
    (out, out_w, out_h)
}

enum Screen {
    Wizard(WizardState),
    Main(CaptureSession),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MainRoute {
    Capture,
    Settings,
}

impl FindBtApp {
    pub fn new(ctx: &egui::Context) -> Self {
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
            route: MainRoute::Capture,
            settings_section: SettingsSection::Appearance,
            report_error_open: false,
            scan_console_open: false,
            scan_console_lines: vec![format!(
                "[{}] FindBT live scan log ready.",
                Local::now().format("%H:%M:%S")
            )],
            settings: AppSettings::load(),
            app_icon: load_app_icon_texture(ctx),
        }
    }

    /// Push the persisted theme preference into egui and resolve the palette
    /// for this frame. With `System`, egui tracks OS light/dark changes live.
    fn apply_theme(&mut self, ctx: &egui::Context) {
        ctx.set_theme(match self.settings.theme {
            ThemeSetting::System => egui::ThemePreference::System,
            ThemeSetting::Light => egui::ThemePreference::Light,
            ThemeSetting::Dark => egui::ThemePreference::Dark,
        });
        self.theme = match ctx.theme() {
            egui::Theme::Dark => Theme::dark(AccentColor::Blue),
            egui::Theme::Light => Theme::light(AccentColor::Blue),
        };
    }

    fn begin_session(&mut self, metadata: CaseMetadata, host: HostAdapterInfo) {
        let mut session = CaptureSession::new(metadata, host);
        let local = normalize_address(&session.host.address);
        session.registry.apply_local_radio_tag(&local);
        self.screen = Screen::Main(session);
        self.route = MainRoute::Capture;
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
                self.push_scan_console_line(format!(
                    "[{}] START {}",
                    Local::now().format("%H:%M:%S"),
                    phase.report_label()
                ));
                self.status = format!("{} running.", phase.tab_title());
            }
            Err(err) => {
                self.push_scan_console_line(format!(
                    "[{}] ERROR scan could not start: {err}",
                    Local::now().format("%H:%M:%S")
                ));
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
            self.push_scan_console_line(format!(
                "[{}] STOP {} | {reason}",
                Local::now().format("%H:%M:%S"),
                phase.report_label()
            ));
            self.status = format!("{} stopped.", phase.tab_title());
        }
    }

    fn drain_observations(&mut self) {
        let Some(receiver) = &self.receiver else {
            return;
        };
        let mut observations = Vec::new();
        while let Ok(observation) = receiver.try_recv() {
            observations.push(observation);
        }
        if observations.is_empty() {
            return;
        }
        for observation in &observations {
            self.push_scan_console_observation(observation);
        }
        if let Screen::Main(session) = &mut self.screen {
            for observation in observations {
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

    fn report_ready(&self) -> bool {
        matches!(
            &self.screen,
            Screen::Main(session) if session.phase_run_for(ScanPhase::Verification).is_some()
        )
    }

    fn report_not_ready_popup(&mut self, ctx: &egui::Context) {
        if !self.report_error_open {
            return;
        }
        let mut open = self.report_error_open;
        let mut close_requested = false;
        egui::Window::new("Report unavailable")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.set_width(320.0);
                ui.label(
                    egui::RichText::new(
                        "All three scan phases must be completed before a report can be generated.",
                    )
                    .color(self.theme.text),
                );
                ui.add_space(12.0);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if crate::widgets::secondary_button(ui, self.theme, "OK").clicked() {
                        close_requested = true;
                    }
                });
            });
        if close_requested {
            open = false;
        }
        self.report_error_open = open;
    }

    fn apply_action(&mut self, action: MainScreenAction) {
        match action {
            MainScreenAction::Start(phase) => self.start_scan(phase),
            MainScreenAction::Stop => self.stop_scan("Stopped by operator"),
            MainScreenAction::Rescan(phase) => self.start_scan(phase),
            MainScreenAction::SelectPhase(phase) => self.active_phase = phase,
            MainScreenAction::GenerateReport => {
                if self.report_ready() {
                    self.save_report();
                } else {
                    self.report_error_open = true;
                    self.status =
                        "Complete all three scan phases before generating a report.".to_string();
                }
            }
            MainScreenAction::ResetCapture => {
                self.backend.stop();
                self.receiver = None;
                self.scanning_phase = None;
                self.current_run = None;
                self.report_error_open = false;
                self.scan_console_lines.clear();
                self.push_scan_console_line(format!(
                    "[{}] Capture reset.",
                    Local::now().format("%H:%M:%S")
                ));
                if let Screen::Main(session) = &mut self.screen {
                    session.reset_capture();
                }
                self.active_phase = ScanPhase::Baseline;
                self.status = "Capture reset.".to_string();
            }
            MainScreenAction::SetKindFilter(kind) => self.kind_filter = kind,
            MainScreenAction::SetFilterText(text) => self.filter_text = text,
            MainScreenAction::OpenScanConsole => self.scan_console_open = true,
            MainScreenAction::OpenSettings => self.route = MainRoute::Settings,
        }
    }

    fn apply_settings_action(&mut self, action: SettingsScreenAction) {
        match action {
            SettingsScreenAction::BackToCapture => self.route = MainRoute::Capture,
            SettingsScreenAction::SelectSection(section) => self.settings_section = section,
        }
    }
}

impl eframe::App for FindBtApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.drain_observations();
        ui.request_repaint_after(std::time::Duration::from_millis(100));
        let ctx = ui.ctx().clone();
        self.apply_theme(&ctx);

        match &mut self.screen {
            Screen::Wizard(state) => match state.ui(ui, self.theme, &self.app_icon) {
                WizardAction::None => {}
                WizardAction::Begin { metadata, host } => self.begin_session(metadata, host),
            },
            Screen::Main(session) => match self.route {
                MainRoute::Capture => {
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
                            app_icon: &self.app_icon,
                        },
                    );
                    if let Some(action) = action {
                        self.apply_action(action);
                    }
                }
                MainRoute::Settings => {
                    let action = crate::settings_screen::show(
                        ui,
                        SettingsScreenState {
                            theme: self.theme,
                            active_section: self.settings_section,
                            settings: &mut self.settings,
                            report_format: &mut self.report_format,
                            app_icon: &self.app_icon,
                        },
                    );
                    if let Some(action) = action {
                        self.apply_settings_action(action);
                    }
                }
            },
        }

        let ctx = ui.ctx().clone();
        self.report_not_ready_popup(&ctx);
        self.scan_console_window(&ctx);
    }
}

impl FindBtApp {
    fn push_scan_console_observation(&mut self, observation: &RawObservation) {
        let endpoint = if observation.address.trim().is_empty() {
            observation.device_id.as_str()
        } else {
            observation.address.as_str()
        };
        let rssi = observation
            .rssi
            .map(|value| format!("{value} dBm"))
            .unwrap_or_else(|| "RSSI n/a".to_string());
        let paired = if observation.is_paired {
            "paired"
        } else {
            "not paired"
        };
        self.push_scan_console_line(format!(
            "[{}] phase={} | {} | {} | {} | {} | {} | {}",
            observation.observed_at.format("%H:%M:%S%.3f"),
            observation.phase.number(),
            observation.kind.label(),
            observation.name,
            endpoint,
            rssi,
            paired,
            observation.properties_summary
        ));
    }

    fn push_scan_console_line(&mut self, line: String) {
        const MAX_CONSOLE_LINES: usize = 2_000;
        self.scan_console_lines.push(line);
        if self.scan_console_lines.len() > MAX_CONSOLE_LINES {
            let overflow = self.scan_console_lines.len() - MAX_CONSOLE_LINES;
            self.scan_console_lines.drain(0..overflow);
        }
    }

    fn scan_console_window(&mut self, ctx: &egui::Context) {
        if !self.scan_console_open {
            return;
        }
        let mut open = self.scan_console_open;
        egui::Window::new("Live Scan Log")
            .open(&mut open)
            .default_width(780.0)
            .default_height(420.0)
            .resizable(true)
            .collapsible(false)
            .frame(
                egui::Frame::new()
                    .fill(self.theme.bg_sunken)
                    .stroke(egui::Stroke::new(1.0, self.theme.border))
                    .corner_radius(8.0)
                    .inner_margin(egui::Margin::same(12)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(format!(
                            "{} raw event{}",
                            self.scan_console_lines.len(),
                            if self.scan_console_lines.len() == 1 {
                                ""
                            } else {
                                "s"
                            }
                        ))
                        .color(self.theme.text_muted)
                        .size(11.0),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if widgets::secondary_button(ui, self.theme, "Clear").clicked() {
                            self.scan_console_lines.clear();
                            self.push_scan_console_line(format!(
                                "[{}] Live scan log cleared.",
                                Local::now().format("%H:%M:%S")
                            ));
                        }
                    });
                });
                ui.add_space(8.0);
                egui::Frame::new()
                    .fill(self.theme.bg)
                    .stroke(egui::Stroke::new(1.0, self.theme.border_soft))
                    .corner_radius(6.0)
                    .inner_margin(egui::Margin::same(10))
                    .show(ui, |ui| {
                        egui::ScrollArea::vertical()
                            .stick_to_bottom(true)
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                ui.set_min_width(ui.available_width());
                                for line in &self.scan_console_lines {
                                    ui.label(
                                        egui::RichText::new(line)
                                            .monospace()
                                            .color(self.theme.text)
                                            .size(11.0),
                                    );
                                }
                            });
                    });
            });
        self.scan_console_open = open;
    }
}
