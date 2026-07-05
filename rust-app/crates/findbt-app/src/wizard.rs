use chrono::{Local, NaiveDate};
use findbt_core::{CaseMetadata, HostAdapterInfo};

use crate::theme::Theme;
use crate::widgets;

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

    pub fn ui(
        &mut self,
        ui: &mut egui::Ui,
        theme: Theme,
        icon: &egui::TextureHandle,
    ) -> WizardAction {
        let mut action = WizardAction::None;
        egui::Panel::top("wizard-titlebar")
            .exact_size(40.0)
            .frame(egui::Frame::new().fill(theme.bg_elevated))
            .show(ui, |ui| {
                crate::titlebar::titlebar(ui, theme, false, icon);
            });
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(theme.bg))
            .show(ui, |ui| {
                // Scrollable so the wizard is always reachable even on a
                // small window, but the spacing below is tuned to fit the
                // default window size without needing to scroll.
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        // Header block: centered app icon, name, and
                        // subtitle, in the style of a macOS "open document"
                        // welcome screen.
                        ui.vertical_centered(|ui| {
                            ui.add_space(18.0);
                            logo(ui, icon, 40.0);
                            ui.add_space(8.0);
                            ui.heading(
                                egui::RichText::new("FindBT")
                                    .color(theme.text)
                                    .size(20.0)
                                    .strong(),
                            );
                            ui.add_space(2.0);
                            ui.label(
                                egui::RichText::new("Find nearby Bluetooth devices")
                                    .color(theme.text_muted)
                                    .size(12.0),
                            );
                        });

                        ui.add_space(14.0);

                        // The card itself: soft, generous corners
                        // (macOS-like), holding rectangular, flat-bordered
                        // controls (Windows-like). Fields are narrower than
                        // the card and centered within it, rather than
                        // stretched edge to edge.
                        let card_width = 520.0;
                        let card_padding = 20.0;
                        // Sized for the longest real content any field will
                        // hold ("Detected Bluetooth adapter"), not stretched
                        // out to fill the card.
                        let field_width = 230.0;
                        let card_outer = card_width + card_padding * 2.0 + 2.0;
                        let indent = ((ui.available_width() - card_outer) * 0.5).max(0.0);
                        ui.horizontal(|ui| {
                            ui.add_space(indent);
                            widgets::card_frame(theme)
                                .inner_margin(egui::Margin::same(20))
                                .show(ui, |ui| {
                                    ui.set_width(card_width);
                                    ui.vertical(|ui| {
                                        ui.vertical_centered(|ui| {
                                            widgets::caption(ui, theme, "Case details");
                                        });
                                        ui.add_space(6.0);

                                        field(
                                            ui,
                                            theme,
                                            "Date",
                                            &mut self.date,
                                            "YYYY-MM-DD",
                                            field_width,
                                        );
                                        field(
                                            ui,
                                            theme,
                                            "Scan reference",
                                            &mut self.name,
                                            "",
                                            field_width,
                                        );
                                        field(
                                            ui,
                                            theme,
                                            "Target device reference",
                                            &mut self.section,
                                            "",
                                            field_width,
                                        );
                                        field(ui, theme, "User", &mut self.user, "", field_width);

                                        ui.add_space(4.0);
                                        ui.separator();
                                        ui.add_space(10.0);

                                        ui.vertical_centered(|ui| {
                                            widgets::caption(ui, theme, "Host adapter");
                                        });
                                        ui.add_space(6.0);

                                        field(
                                            ui,
                                            theme,
                                            "Computer name",
                                            &mut self.computer_name,
                                            "",
                                            field_width,
                                        );
                                        field(
                                            ui,
                                            theme,
                                            "Adapter name",
                                            &mut self.host_name,
                                            "Detected Bluetooth adapter",
                                            field_width,
                                        );
                                        field(
                                            ui,
                                            theme,
                                            "Radio address or tag",
                                            &mut self.host_address,
                                            "AA:BB:CC:DD:EE:FF or label",
                                            field_width,
                                        );

                                        ui.add_space(4.0);
                                        ui.separator();
                                        ui.add_space(10.0);

                                        // Primary action is centered, matching
                                        // every other centered element in the
                                        // wizard.
                                        let metadata = self.metadata();
                                        let valid = metadata
                                            .as_ref()
                                            .map(CaseMetadata::is_complete)
                                            .unwrap_or(false);
                                        ui.vertical_centered(|ui| {
                                            if widgets::primary_button_enabled(
                                                ui,
                                                theme,
                                                "Begin Scan",
                                                valid,
                                            )
                                            .clicked()
                                            {
                                                if let Some(metadata) = metadata {
                                                    action = WizardAction::Begin {
                                                        metadata,
                                                        host: HostAdapterInfo {
                                                            name: self.host_name.trim().to_string(),
                                                            address: self
                                                                .host_address
                                                                .trim()
                                                                .to_string(),
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
                        ui.add_space(16.0);
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

/// One labeled field: a muted caption above a boxed input, both centered
/// within the available width (rather than stretched edge to edge), with
/// the trailing spacing baked in so call sites can just list fields in
/// order.
///
/// `vertical_centered` alone isn't enough for the box: a `Frame`-wrapped
/// `TextEdit` still claims the full available width unless something
/// narrower is placed around it, so the box is indented by hand here using
/// the same technique already used to center the card itself.
fn field(
    ui: &mut egui::Ui,
    theme: Theme,
    caption: &str,
    value: &mut String,
    hint: &str,
    width: f32,
) {
    ui.vertical_centered(|ui| {
        widgets::caption(ui, theme, caption);
    });
    ui.add_space(3.0);
    let box_width = width + widgets::TEXT_FIELD_PADDING;
    let indent = ((ui.available_width() - box_width) * 0.5).max(0.0);
    ui.horizontal(|ui| {
        ui.add_space(indent);
        widgets::text_field(ui, theme, value, hint, width);
    });
    ui.add_space(6.0);
}

/// Draw the real, bundled app icon at the given size. `icon` is loaded once
/// at startup (see `app.rs`) and reused everywhere the icon appears, so the
/// titlebar and wizard always show the same official artwork.
///
/// The source art has no transparency: it's a flat opaque square with solid
/// black outside the rounded icon graphic (meant for OS icon slots that
/// apply their own mask). A generous `corner_radius` clips that flat black
/// square down to just the rounded icon so it doesn't show as a hard black
/// box inline in the UI.
pub fn logo(ui: &mut egui::Ui, icon: &egui::TextureHandle, size: f32) {
    ui.add(
        egui::Image::from_texture(icon)
            .fit_to_exact_size(egui::vec2(size, size))
            .corner_radius(size * 0.22),
    );
}
