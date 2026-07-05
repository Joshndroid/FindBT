use findbt_core::{CaptureSession, DeviceKind, DeviceRecord, ScanPhase};

use crate::{
    theme::{signal_color, Theme},
    titlebar::titlebar,
    widgets,
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

#[derive(Clone, Copy)]
pub struct MainScreenState<'a> {
    pub theme: Theme,
    pub active_phase: ScanPhase,
    pub scanning_phase: Option<ScanPhase>,
    pub status: &'a str,
    pub filter_text: &'a str,
    pub kind_filter: KindFilter,
    pub app_icon: &'a egui::TextureHandle,
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
        .show(ui, |ui| {
            let _ = titlebar(ui, state.theme, false, state.app_icon);
        });

    egui::Panel::left("sidebar")
        .exact_size(260.0)
        .frame(egui::Frame::new().fill(state.theme.bg_sunken))
        .show(ui, |ui| {
            ui.allocate_ui_with_layout(
                ui.available_size(),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    ui.set_width(ui.available_width());
                    sidebar(
                        ui,
                        session,
                        state.theme,
                        state.active_phase,
                        state.scanning_phase,
                        state.status,
                        &mut action,
                    )
                },
            );
        });

    egui::CentralPanel::default()
        .frame(egui::Frame::new().fill(state.theme.bg))
        .show(ui, |ui| main_panel(ui, session, state, &mut action));

    action
}

fn sidebar(
    ui: &mut egui::Ui,
    session: &CaptureSession,
    theme: Theme,
    active_phase: ScanPhase,
    scanning_phase: Option<ScanPhase>,
    _status: &str,
    action: &mut Option<MainScreenAction>,
) {
    ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
    ui.set_width(ui.available_width());
    ui.add_space(18.0);
    ui.horizontal(|ui| {
        ui.add_space(18.0);
        ui.vertical(|ui| {
            ui.set_width(224.0);
            sidebar_content(ui, session, theme, active_phase, scanning_phase, action);
        });
    });

    ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
        ui.add_space(32.0);
        sidebar_footer_button(
            ui,
            theme,
            "Settings",
            MainScreenAction::OpenSettings,
            action,
        );
        ui.add_space(8.0);
        sidebar_footer_button(
            ui,
            theme,
            "Generate Report",
            MainScreenAction::GenerateReport,
            action,
        );
    });
}

fn sidebar_content(
    ui: &mut egui::Ui,
    session: &CaptureSession,
    theme: Theme,
    active_phase: ScanPhase,
    scanning_phase: Option<ScanPhase>,
    action: &mut Option<MainScreenAction>,
) {
    widgets::caption(ui, theme, "Host adapter");
    ui.add_space(8.0);
    panel(ui, theme, |ui| {
        ui.set_min_width(ui.available_width());
        ui.label(
            egui::RichText::new(&session.host.name)
                .color(theme.text)
                .size(13.0)
                .strong(),
        );
        ui.label(
            egui::RichText::new(&session.host.address)
                .color(theme.text_muted)
                .monospace()
                .size(11.0),
        );
        ui.add_space(6.0);
        ui.label(
            egui::RichText::new(&session.metadata.name)
                .color(theme.text_muted)
                .size(11.0),
        );
    });
    ui.add_space(18.0);
    widgets::caption(ui, theme, "Scan action");
    ui.add_space(8.0);
    scan_action_button(ui, session, theme, active_phase, scanning_phase, action);
    ui.add_space(8.0);
    if widgets::sidebar_button(ui, theme, "Reset capture").clicked() {
        *action = Some(MainScreenAction::ResetCapture);
    }
    if scanning_phase == Some(active_phase) {
        ui.add_space(8.0);
        ui.colored_label(theme.accent_main(), "scanning");
    }
    ui.add_space(18.0);
    widgets::caption(ui, theme, "Phases");
    ui.add_space(8.0);
    for phase in ScanPhase::ALL {
        let complete = session.phase_run_for(phase).is_some();
        let scanning = scanning_phase == Some(phase);
        let selected = active_phase == phase;
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
                ui.set_min_width(ui.available_width());
                ui.horizontal(|ui| {
                    phase_badge(ui, theme, phase, complete, scanning);
                    ui.add_space(8.0);
                    ui.label(
                        egui::RichText::new(phase.tab_title())
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
}

fn sidebar_footer_button(
    ui: &mut egui::Ui,
    theme: Theme,
    label: &str,
    button_action: MainScreenAction,
    action: &mut Option<MainScreenAction>,
) {
    ui.horizontal(|ui| {
        ui.add_space(18.0);
        ui.vertical(|ui| {
            ui.set_width(224.0);
            if widgets::sidebar_button(ui, theme, label).clicked() {
                *action = Some(button_action);
            }
        });
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
    let filter_text = state.filter_text;
    let kind_filter = state.kind_filter;

    ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
    ui.add_space(22.0);
    ui.horizontal(|ui| {
        ui.add_space(24.0);
        ui.vertical(|ui| {
            ui.set_width((ui.available_width() - 24.0).max(0.0));
            page_heading(ui, theme, active_phase);
            ui.add_space(18.0);
            device_registry(
                ui,
                session,
                theme,
                active_phase,
                filter_text,
                kind_filter,
                action,
            );
        });
    });
}

fn page_heading(ui: &mut egui::Ui, theme: Theme, active_phase: ScanPhase) {
    ui.heading(
        egui::RichText::new(active_phase.tab_title())
            .color(theme.text)
            .size(22.0),
    );
    ui.add_space(4.0);
    ui.label(
        egui::RichText::new(active_phase.description())
            .color(theme.text_muted)
            .size(12.0),
    );
}

fn scan_action_button(
    ui: &mut egui::Ui,
    session: &CaptureSession,
    theme: Theme,
    active_phase: ScanPhase,
    scanning_phase: Option<ScanPhase>,
    action: &mut Option<MainScreenAction>,
) {
    let button_text = if scanning_phase == Some(active_phase) {
        "Stop Scan"
    } else if session.phase_run_for(active_phase).is_some() {
        "Rescan"
    } else {
        "Start Scan"
    };
    if widgets::sidebar_button(ui, theme, button_text).clicked() {
        *action = Some(if scanning_phase == Some(active_phase) {
            MainScreenAction::Stop
        } else if session.phase_run_for(active_phase).is_some() {
            MainScreenAction::Rescan(active_phase)
        } else {
            MainScreenAction::Start(active_phase)
        });
    }
}

fn device_registry(
    ui: &mut egui::Ui,
    session: &CaptureSession,
    theme: Theme,
    active_phase: ScanPhase,
    filter_text: &str,
    kind_filter: KindFilter,
    action: &mut Option<MainScreenAction>,
) {
    panel(ui, theme, |ui| {
        ui.set_min_width(ui.available_width());
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(
                    egui::RichText::new("Device registry")
                        .color(theme.text)
                        .strong()
                        .size(14.0),
                );
                ui.label(
                    egui::RichText::new("Devices seen across the capture phases")
                        .color(theme.text_muted)
                        .size(11.0),
                );
            });
        });

        ui.add_space(14.0);
        registry_toolbar(ui, theme, filter_text, kind_filter, action);
        ui.add_space(12.0);
        registry_legend(ui, theme);
        ui.add_space(12.0);
        device_table(ui, session, theme, active_phase, filter_text, kind_filter);
    });
}

fn registry_toolbar(
    ui: &mut egui::Ui,
    theme: Theme,
    filter_text: &str,
    kind_filter: KindFilter,
    action: &mut Option<MainScreenAction>,
) {
    ui.horizontal_wrapped(|ui| {
        let mut next_filter = filter_text.to_string();
        let response = egui::Frame::new()
            .fill(theme.bg)
            .stroke(egui::Stroke::new(1.0, theme.border_soft))
            .corner_radius(theme.control_radius)
            .inner_margin(egui::Margin::symmetric(10, 6))
            .show(ui, |ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut next_filter)
                        .hint_text("Filter devices")
                        .desired_width(280.0)
                        .frame(egui::Frame::NONE),
                )
            })
            .inner;
        if response.changed() {
            *action = Some(MainScreenAction::SetFilterText(next_filter));
        }
        ui.add_space(10.0);
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
}

fn registry_legend(ui: &mut egui::Ui, theme: Theme) {
    ui.horizontal_wrapped(|ui| {
        legend_item(ui, theme, MarkerState::Seen, "seen");
        ui.add_space(12.0);
        legend_item(ui, theme, MarkerState::Missed, "phase ran, not seen");
        ui.add_space(12.0);
        legend_item(ui, theme, MarkerState::Pending, "phase pending");
    });
}

fn legend_item(ui: &mut egui::Ui, theme: Theme, state: MarkerState, label: &str) {
    ui.horizontal(|ui| {
        marker(ui, theme, state, 1.0);
        ui.label(
            egui::RichText::new(label)
                .color(theme.text_muted)
                .size(10.0),
        );
    });
}

fn device_table(
    ui: &mut egui::Ui,
    session: &CaptureSession,
    theme: Theme,
    active_phase: ScanPhase,
    filter_text: &str,
    kind_filter: KindFilter,
) {
    let devices: Vec<&DeviceRecord> = filtered_devices(session, filter_text, kind_filter).collect();
    if devices.is_empty() {
        empty_state(ui, theme, filter_text);
        return;
    }

    egui::ScrollArea::vertical()
        .max_height(420.0)
        .show(ui, |ui| {
            egui::Grid::new("device-table")
                .num_columns(7)
                .min_col_width(68.0)
                .spacing([18.0, 12.0])
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

                    for device in devices {
                        device_row(ui, session, theme, device, active_phase);
                        ui.end_row();
                    }
                });
        });
}

fn empty_state(ui: &mut egui::Ui, theme: Theme, filter_text: &str) {
    egui::Frame::new()
        .fill(theme.bg)
        .stroke(egui::Stroke::new(1.0, theme.border_soft))
        .corner_radius(6.0)
        .inner_margin(egui::Margin::symmetric(18, 22))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.vertical_centered(|ui| {
                let title = if filter_text.trim().is_empty() {
                    "No devices recorded yet"
                } else {
                    "No devices match the current filter"
                };
                let detail = if filter_text.trim().is_empty() {
                    "Start the active scan phase to populate this registry."
                } else {
                    "Adjust the search text or device type filters to widen the results."
                };
                ui.label(
                    egui::RichText::new(title)
                        .color(theme.text)
                        .strong()
                        .size(13.0),
                );
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new(detail)
                        .color(theme.text_muted)
                        .size(11.0),
                );
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
    widgets::caption(ui, theme, text);
}
