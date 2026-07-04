use chrono::{Local, NaiveDate};
use findbt_core::{CaseMetadata, HostAdapterInfo};

use crate::theme::Theme;

pub struct WizardState {
    date: String,
    name: String,
    section: String,
    user: String,
    computer_name: String,
    host_name: String,
    host_address: String,
}

pub enum WizardAction {
    None,
    Begin {
        metadata: CaseMetadata,
        host: HostAdapterInfo,
    },
}

impl WizardState {
    pub fn new(host: HostAdapterInfo) -> Self {
        let computer_name = if host.computer_name.trim().is_empty() {
            detect_computer_name()
        } else {
            host.computer_name
        };
        Self {
            date: Local::now().date_naive().to_string(),
            name: String::new(),
            section: String::new(),
            user: String::new(),
            computer_name,
            host_name: host.name,
            host_address: host.address,
        }
    }

    pub fn ui(&mut self, ui: &mut egui::Ui, theme: Theme) -> WizardAction {
        let mut action = WizardAction::None;
        egui::Panel::top("wizard-titlebar")
            .exact_size(40.0)
            .frame(egui::Frame::new().fill(theme.bg_elevated))
            .show(ui, |ui| {
                crate::titlebar::titlebar(ui, theme, false);
            });
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(theme.bg))
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(40.0);
                    logo(ui, theme, 42.0);
                    ui.add_space(16.0);
                    ui.heading(
                        egui::RichText::new("FindBT")
                            .color(theme.text)
                            .size(22.0),
                    );
                    ui.label(
                        egui::RichText::new("Find nearby Bluetooth device")
                            .color(theme.text_muted)
                            .size(12.0),
                    );
                });

                ui.add_space(30.0);
                // Center the card: content width + inner margins + border stroke.
                let card_outer = 560.0 + 24.0 * 2.0 + 2.0;
                let indent = ((ui.available_width() - card_outer) * 0.5).max(0.0);
                ui.horizontal(|ui| {
                    ui.add_space(indent);
                    egui::Frame::new()
                        .fill(theme.bg_elevated)
                        .stroke(egui::Stroke::new(1.0, theme.border))
                        .corner_radius(8.0)
                        .inner_margin(egui::Margin::same(24))
                        .show(ui, |ui| {
                            ui.set_width(560.0);
                            ui.vertical(|ui| {
                                label(ui, theme, "DATE");
                                ui.add(text_edit(&mut self.date, "YYYY-MM-DD"));
                                ui.add_space(10.0);

                                label(ui, theme, "SCAN REFERENCE");
                                ui.add(text_edit(&mut self.name, ""));
                                ui.add_space(10.0);

                                label(ui, theme, "TARGET DEVICE REFERENCE");
                                ui.add(text_edit(&mut self.section, ""));
                                ui.add_space(10.0);

                                label(ui, theme, "USER");
                                ui.add(text_edit(&mut self.user, ""));
                                ui.add_space(18.0);

                                ui.separator();
                                ui.add_space(14.0);

                                ui.label(
                                    egui::RichText::new("Host adapter")
                                        .color(theme.text)
                                        .strong(),
                                );
                                ui.add_space(6.0);
                                label(ui, theme, "COMPUTER NAME");
                                ui.add(text_edit(&mut self.computer_name, ""));
                                ui.add_space(10.0);
                                label(ui, theme, "ADAPTER NAME");
                                ui.add(text_edit(&mut self.host_name, "Detected Bluetooth adapter"));
                                ui.add_space(10.0);
                                label(ui, theme, "RADIO ADDRESS OR TAG");
                                ui.add(text_edit(
                                    &mut self.host_address,
                                    "AA:BB:CC:DD:EE:FF or label",
                                ));

                                ui.add_space(22.0);
                                let metadata = self.metadata();
                                let valid = metadata
                                    .as_ref()
                                    .map(CaseMetadata::is_complete)
                                    .unwrap_or(false);
                                if ui
                                    .add_enabled(
                                        valid,
                                        egui::Button::new(
                                            egui::RichText::new("Begin Scan")
                                                .color(theme.accent_text)
                                                .strong(),
                                        )
                                        .fill(theme.accent_main())
                                        .corner_radius(6.0)
                                        .min_size(egui::vec2(132.0, 36.0)),
                                    )
                                    .clicked()
                                {
                                    if let Some(metadata) = metadata {
                                        action = WizardAction::Begin {
                                            metadata,
                                            host: HostAdapterInfo {
                                                name: self.host_name.trim().to_string(),
                                                address: self.host_address.trim().to_string(),
                                                computer_name: self
                                                    .computer_name
                                                    .trim()
                                                    .to_string(),
                                            },
                                        };
                                    }
                                }
                            });
                        });
                });
            });
        action
    }

    fn metadata(&self) -> Option<CaseMetadata> {
        Some(CaseMetadata {
            date: NaiveDate::parse_from_str(self.date.trim(), "%Y-%m-%d").ok()?,
            name: self.name.trim().to_string(),
            section: self.section.trim().to_string(),
            user: self.user.trim().to_string(),
        })
    }
}

/// Best-effort host name detection from the environment. `COMPUTERNAME` is
/// always set on Windows; `HOSTNAME` covers most unix shells. The field stays
/// editable in the wizard, so a miss here is not fatal.
fn detect_computer_name() -> String {
    std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_default()
        .trim()
        .to_string()
}

fn text_edit<'a>(text: &'a mut String, hint: &'a str) -> egui::TextEdit<'a> {
    egui::TextEdit::singleline(text)
        .hint_text(hint)
        .desired_width(f32::INFINITY)
}

fn label(ui: &mut egui::Ui, theme: Theme, text: &str) {
    ui.label(
        egui::RichText::new(text)
            .color(theme.text_muted)
            .size(10.0)
            .strong(),
    );
}

pub fn logo(ui: &mut egui::Ui, theme: Theme, size: f32) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::hover());
    let center = rect.center();
    let painter = ui.painter();
    painter.circle_stroke(
        center,
        size * 0.42,
        egui::Stroke::new(2.0, theme.accent_main()),
    );
    painter.circle_stroke(
        center,
        size * 0.28,
        egui::Stroke::new(2.0, theme.accent_main()),
    );
    painter.circle_filled(center, size * 0.12, theme.accent_main());
}
