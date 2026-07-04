use chrono::{Local, NaiveDate};
use findbt_core::{CaseMetadata, HostAdapterInfo};

use crate::theme::Theme;

pub struct WizardState {
    date: String,
    name: String,
    section: String,
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
        Self {
            date: Local::now().date_naive().to_string(),
            name: String::new(),
            section: String::new(),
            host_name: host.name,
            host_address: host.address,
        }
    }

    pub fn ui(&mut self, ui: &mut egui::Ui, theme: Theme) -> WizardAction {
        let mut action = WizardAction::None;
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(theme.bg))
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(40.0);
                    logo(ui, theme, 42.0);
                    ui.add_space(16.0);
                    ui.heading(
                        egui::RichText::new("Bluetooth Capture")
                            .color(theme.text)
                            .size(22.0),
                    );
                    ui.label(
                        egui::RichText::new("Create a capture session")
                            .color(theme.text_muted)
                            .size(12.0),
                    );
                });

                ui.add_space(30.0);
                egui::Frame::new()
                    .fill(theme.bg_elevated)
                    .stroke(egui::Stroke::new(1.0, theme.border))
                    .corner_radius(8.0)
                    .inner_margin(egui::Margin::same(24))
                    .show(ui, |ui| {
                        ui.set_max_width(560.0);
                        ui.vertical(|ui| {
                            label(ui, theme, "CASE DATE");
                            ui.add(text_edit(&mut self.date, "YYYY-MM-DD"));
                            ui.add_space(10.0);

                            label(ui, theme, "NAME");
                            ui.add(text_edit(
                                &mut self.name,
                                "Person, device owner, or case name",
                            ));
                            ui.add_space(10.0);

                            label(ui, theme, "SECTION");
                            ui.add(text_edit(
                                &mut self.section,
                                "Team, section, or exhibit reference",
                            ));
                            ui.add_space(18.0);

                            ui.separator();
                            ui.add_space(14.0);

                            ui.label(
                                egui::RichText::new("Host adapter")
                                    .color(theme.text)
                                    .strong(),
                            );
                            ui.add_space(6.0);
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
                                        },
                                    };
                                }
                            }
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
        })
    }
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
