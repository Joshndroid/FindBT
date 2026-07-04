use findbt_core::{CaptureSession, DeviceKind, DeviceRecord, ScanPhase};

use crate::{
    theme::{signal_color, Theme},
    wizard::logo,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KindFilter {
    All,
    Kind(DeviceKind),
}

#[derive(Debug, Clone)]
pub enum MainScreenAction {
    Start(ScanPhase),
    Stop,
    Rescan(ScanPhase),
    SelectPhase(ScanPhase),
    GenerateReport,
    ResetCapture,
    SetKindFilter(KindFilter),
    SetFilterText(String),
    OpenSettings,
}

#[derive(Debug, Clone, Copy)]
pub struct MainScreenState<'a> {
    pub theme: Theme,
    pub active_phase: ScanPhase,
    pub scanning_phase: Option<ScanPhase>,
    pub status: &'a str,
    pub filter_text: &'a str,
    pub kind_filter: KindFilter,
}

pub fn show(
    ui: &mut egui::Ui,
    session: &CaptureSession,
    state: MainScreenState<'_>,
) -> Option<MainScreenAction> {
    let mut action = None;
    egui::Panel::top("titlebar")
        .exact_size(40.0)
        .frame(egui::Frame::new().fill(state.theme.bg_elevated))
        .show(ui, |ui| titlebar(ui, state.theme, &mut action));

    egui::Panel::left("sidebar")
        .exact_size(260.0)
        .frame(egui::Frame::new().fill(state.theme.bg_sunken))
        .show(ui, |ui| {
            sidebar(
                ui,
                session,
                state.theme,
                state.active_phase,
                state.scanning_phase,
                state.status,
                &mut action,
            )
        });

    egui::CentralPanel::default()
        .frame(egui::Frame::new().fill(state.theme.bg))
        .show(ui, |ui| main_panel(ui, session, state, &mut action));

    action
}

fn titlebar(ui: &mut egui::Ui, theme: Theme, action: &mut Option<MainScreenAction>) {
    // Register the drag area first so the buttons drawn afterwards stay on
    // top and receive their clicks. Dragging only starts on actual drag
    // motion, never on a plain press, so button clicks are not swallowed by
    // the OS window-move loop.
    let drag_response = ui.interact(
        ui.max_rect(),
        egui::Id::new("titlebar-drag"),
        egui::Sense::click_and_drag(),
    );
    if drag_response.drag_started_by(egui::PointerButton::Primary) {
        ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
    }

    ui.horizontal(|ui| {
        ui.add_space(10.0);
        logo(ui, theme, 24.0);
        ui.label(
            egui::RichText::new("Bluetooth Capture")
                .color(theme.text)
                .strong(),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("x").clicked() {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            }
            if ui.button("□").clicked() {
                let maximized = ui.input(|input| input.viewport().maximized.unwrap_or(false));
                ui.ctx()
                    .send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
            }
            if ui.button("-").clicked() {
                ui.ctx()
                    .send_viewport_cmd(egui::ViewportCommand::Minimized(true));
            }
            ui.add_space(6.0);
            if ui.button("Settings").clicked() {
                *action = Some(MainScreenAction::OpenSettings);
            }
        });
    });
}

fn sidebar(
    ui: &mut egui::Ui,
    session: &CaptureSession,
    theme: Theme,
    active_phase: ScanPhase,
    scanning_phase: Option<ScanPhase>,
    status: &str,
    action: &mut Option<MainScreenAction>,
) {
    ui.add_space(14.0);
    ui.label(
        egui::RichText::new("HOST ADAPTER")
            .color(theme.text_muted)
            .size(10.0)
            .strong(),
    );
    ui.add_space(8.0);
    panel(ui, theme, |ui| {
        ui.label(
            egui::RichText::new(&session.host.name)
                .color(theme.text)
                .strong(),
        );
        ui.label(
            egui::RichText::new(&session.host.address)
                .color(theme.text_muted)
                .monospace(),
        );
        ui.add_space(4.0);
        ui.label(
            egui::RichText::new(&session.metadata.name)
                .color(theme.text_muted)
                .size(11.0),
        );
    });
    ui.add_space(18.0);
    ui.label(
        egui::RichText::new("PHASES")
            .color(theme.text_muted)
            .size(10.0)
            .strong(),
    );
    ui.add_space(8.0);
    for phase in ScanPhase::ALL {
        let complete = session.phase_run_for(phase).is_some();
        let scanning = scanning_phase == Some(phase);
        let selected = active_phase == phase;
        let label = format!("{}  {}", phase.number(), phase.tab_title());
        let fill = if selected {
            theme.accent_soft()
        } else {
            theme.bg_sunken
        };
        let stroke = if scanning {
            egui::Stroke::new(1.4, theme.accent_main())
        } else {
            egui::Stroke::new(1.0, theme.border_soft)
        };
        let response = egui::Frame::new()
            .fill(fill)
            .stroke(stroke)
            .corner_radius(6.0)
            .inner_margin(egui::Margin::symmetric(10, 8))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    phase_badge(ui, theme, phase, complete, scanning);
                    ui.label(
                        egui::RichText::new(label)
                            .color(theme.text)
                            .size(12.0)
                            .strong(),
                    );
                });
            })
            .response;
        if response.interact(egui::Sense::click()).clicked() {
            *action = Some(MainScreenAction::SelectPhase(phase));
        }
        ui.add_space(6.0);
    }

    ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
        ui.add_space(12.0);
        if ui.button("Reset capture").clicked() {
            *action = Some(MainScreenAction::ResetCapture);
        }
        ui.label(
            egui::RichText::new(status)
                .color(theme.text_muted)
                .size(11.0),
        );
    });
}

fn main_panel(
    ui: &mut egui::Ui,
    session: &CaptureSession,
    state: MainScreenState<'_>,
    action: &mut Option<MainScreenAction>,
) {
    let theme = state.theme;
    let active_phase = state.active_phase;
    let scanning_phase = state.scanning_phase;
    let filter_text = state.filter_text;
    let kind_filter = state.kind_filter;

    ui.add_space(18.0);
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.heading(
                egui::RichText::new(active_phase.tab_title())
                    .color(theme.text)
                    .size(18.0),
            );
            ui.label(
                egui::RichText::new(active_phase.description())
                    .color(theme.text_muted)
                    .size(12.0),
            );
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .add(
                    egui::Button::new(egui::RichText::new("Generate Report").strong())
                        .corner_radius(20.0),
                )
                .clicked()
            {
                *action = Some(MainScreenAction::GenerateReport);
            }
        });
    });

    ui.add_space(18.0);
    ui.horizontal(|ui| {
        let button_text = if scanning_phase == Some(active_phase) {
            "Stop Scan"
        } else if session.phase_run_for(active_phase).is_some() {
            "Rescan"
        } else {
            "Start Scan"
        };
        let clicked = ui
            .add(
                egui::Button::new(
                    egui::RichText::new(button_text)
                        .color(theme.accent_text)
                        .strong(),
                )
                .fill(theme.accent_main())
                .corner_radius(6.0)
                .min_size(egui::vec2(112.0, 34.0)),
            )
            .clicked();
        if clicked {
            *action = Some(if scanning_phase == Some(active_phase) {
                MainScreenAction::Stop
            } else if session.phase_run_for(active_phase).is_some() {
                MainScreenAction::Rescan(active_phase)
            } else {
                MainScreenAction::Start(active_phase)
            });
        }
        if scanning_phase == Some(active_phase) {
            ui.colored_label(theme.accent_main(), "● scanning");
        }
    });

    ui.add_space(14.0);
    ui.horizontal(|ui| {
        let mut next_filter = filter_text.to_string();
        if ui
            .add(
                egui::TextEdit::singleline(&mut next_filter)
                    .hint_text("Filter devices")
                    .desired_width(260.0),
            )
            .changed()
        {
            *action = Some(MainScreenAction::SetFilterText(next_filter));
        }
        filter_chip(
            ui,
            theme,
            "All",
            kind_filter == KindFilter::All,
            MainScreenAction::SetKindFilter(KindFilter::All),
            action,
        );
        for kind in DeviceKind::ALL {
            filter_chip(
                ui,
                theme,
                kind.label(),
                kind_filter == KindFilter::Kind(kind),
                MainScreenAction::SetKindFilter(KindFilter::Kind(kind)),
                action,
            );
        }
    });

    ui.add_space(10.0);
    ui.horizontal(|ui| {
        marker(ui, theme, MarkerState::Seen, 1.0);
        ui.label(
            egui::RichText::new("seen")
                .color(theme.text_muted)
                .size(10.0),
        );
        marker(ui, theme, MarkerState::Missed, 1.0);
        ui.label(
            egui::RichText::new("phase ran, not seen")
                .color(theme.text_muted)
                .size(10.0),
        );
        marker(ui, theme, MarkerState::Pending, 1.0);
        ui.label(
            egui::RichText::new("phase pending")
                .color(theme.text_muted)
                .size(10.0),
        );
    });

    ui.add_space(10.0);
    egui::ScrollArea::vertical().show(ui, |ui| {
        egui::Grid::new("device-table")
            .num_columns(7)
            .spacing([16.0, 10.0])
            .striped(true)
            .show(ui, |ui| {
                header(ui, theme, "Signal");
                header(ui, theme, "Device");
                header(ui, theme, "Kind");
                header(ui, theme, "RSSI");
                header(ui, theme, "Baseline");
                header(ui, theme, "Target");
                header(ui, theme, "Verification");
                ui.end_row();

                for device in filtered_devices(session, filter_text, kind_filter) {
                    device_row(ui, session, theme, device, active_phase);
                    ui.end_row();
                }
            });
    });
}

fn filtered_devices<'a>(
    session: &'a CaptureSession,
    filter_text: &str,
    kind_filter: KindFilter,
) -> impl Iterator<Item = &'a DeviceRecord> {
    let needle = filter_text.trim().to_lowercase();
    session.registry.devices().filter(move |device| {
        let kind_ok = match kind_filter {
            KindFilter::All => true,
            KindFilter::Kind(kind) => device.kind == kind,
        };
        let text_ok = needle.is_empty()
            || device.name.to_lowercase().contains(&needle)
            || device.address.to_lowercase().contains(&needle)
            || device.device_id.to_lowercase().contains(&needle);
        kind_ok && text_ok
    })
}

fn device_row(
    ui: &mut egui::Ui,
    session: &CaptureSession,
    theme: Theme,
    device: &DeviceRecord,
    active_phase: ScanPhase,
) {
    let active_observation = device.seen_in(active_phase);
    let opacity = if active_observation.is_some() {
        1.0
    } else {
        0.5
    };
    signal_bars(
        ui,
        theme,
        active_observation.and_then(|obs| obs.rssi),
        opacity,
    );
    ui.vertical(|ui| {
        ui.label(egui::RichText::new(&device.name).color(theme.text).strong());
        ui.label(
            egui::RichText::new(&device.address)
                .color(theme.text_muted)
                .monospace()
                .size(10.0),
        );
    });
    ui.label(
        egui::RichText::new(device.kind.label())
            .color(theme.text)
            .size(11.0),
    );
    let rssi = active_observation.and_then(|obs| obs.rssi);
    ui.colored_label(
        signal_color(theme, rssi),
        rssi.map(|v| format!("{v} dBm"))
            .unwrap_or_else(|| "not seen".to_string()),
    );
    for phase in ScanPhase::ALL {
        let phase_ran = session.phase_run_for(phase).is_some();
        let state = if device.seen_in(phase).is_some() {
            MarkerState::Seen
        } else if phase_ran {
            MarkerState::Missed
        } else {
            MarkerState::Pending
        };
        marker(ui, theme, state, opacity);
    }
}

fn panel<R>(ui: &mut egui::Ui, theme: Theme, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    egui::Frame::new()
        .fill(theme.bg_elevated)
        .stroke(egui::Stroke::new(1.0, theme.border_soft))
        .corner_radius(6.0)
        .inner_margin(egui::Margin::same(12))
        .show(ui, add)
        .inner
}

fn phase_badge(ui: &mut egui::Ui, theme: Theme, phase: ScanPhase, complete: bool, scanning: bool) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(24.0, 24.0), egui::Sense::hover());
    let color = if complete || scanning {
        theme.accent_main()
    } else {
        theme.border
    };
    if complete {
        ui.painter().circle_filled(rect.center(), 12.0, color);
    } else {
        ui.painter()
            .circle_stroke(rect.center(), 11.0, egui::Stroke::new(1.5, color));
    }
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        phase.number().to_string(),
        egui::FontId::monospace(11.0),
        if complete {
            theme.accent_text
        } else {
            theme.text
        },
    );
}

fn signal_bars(ui: &mut egui::Ui, theme: Theme, rssi: Option<i32>, opacity: f32) {
    let strength = findbt_core::SignalStrength::from_rssi(rssi);
    let bars = strength.bars();
    let (rect, _) = ui.allocate_exact_size(egui::vec2(36.0, 24.0), egui::Sense::hover());
    let painter = ui.painter();
    for i in 0..4 {
        let height = 5.0 + i as f32 * 4.0;
        let x = rect.left() + i as f32 * 8.0;
        let y = rect.bottom() - height;
        let color = if i < bars {
            signal_color(theme, rssi).linear_multiply(opacity)
        } else {
            theme.border.linear_multiply(opacity)
        };
        painter.rect_filled(
            egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(5.0, height)),
            1.0,
            color,
        );
    }
}

#[derive(Debug, Clone, Copy)]
enum MarkerState {
    Seen,
    Missed,
    Pending,
}

fn marker(ui: &mut egui::Ui, theme: Theme, state: MarkerState, opacity: f32) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(20.0, 20.0), egui::Sense::hover());
    let center = rect.center();
    match state {
        MarkerState::Seen => {
            ui.painter()
                .circle_filled(center, 8.0, theme.accent_main().linear_multiply(opacity));
        }
        MarkerState::Missed => {
            ui.painter().circle_stroke(
                center,
                8.0,
                egui::Stroke::new(1.2, theme.border.linear_multiply(opacity)),
            );
        }
        MarkerState::Pending => {
            ui.painter().circle_stroke(
                center,
                8.0,
                egui::Stroke::new(1.2, theme.text_muted.linear_multiply(0.5 * opacity)),
            );
        }
    }
}

fn filter_chip(
    ui: &mut egui::Ui,
    theme: Theme,
    label: &str,
    selected: bool,
    chip_action: MainScreenAction,
    action: &mut Option<MainScreenAction>,
) {
    let fill = if selected {
        theme.accent_soft()
    } else {
        theme.bg_elevated
    };
    if ui
        .add(
            egui::Button::new(egui::RichText::new(label).color(theme.text).size(11.0))
                .fill(fill)
                .corner_radius(12.0),
        )
        .clicked()
    {
        *action = Some(chip_action);
    }
}

fn header(ui: &mut egui::Ui, theme: Theme, text: &str) {
    ui.label(
        egui::RichText::new(text)
            .color(theme.text_muted)
            .size(10.0)
            .strong(),
    );
}
