use crate::{
    app::ReportFormat,
    settings::{AppSettings, ThemeSetting},
    theme::Theme,
    titlebar::titlebar,
    widgets,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsScreenAction {
    BackToCapture,
    SelectSection(SettingsSection),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsSection {
    Appearance,
    ReportGeneration,
}

pub struct SettingsScreenState<'a> {
    pub theme: Theme,
    pub active_section: SettingsSection,
    pub settings: &'a mut AppSettings,
    pub report_format: &'a mut ReportFormat,
    pub app_icon: &'a egui::TextureHandle,
}

pub fn show(ui: &mut egui::Ui, state: SettingsScreenState<'_>) -> Option<SettingsScreenAction> {
    let mut action = None;
    let theme = state.theme;

    egui::Panel::top("settings-titlebar")
        .exact_size(40.0)
        .frame(egui::Frame::new().fill(theme.bg_elevated))
        .show(ui, |ui| {
            let _ = titlebar(ui, theme, false, state.app_icon);
        });

    egui::Panel::left("settings-sidebar")
        .exact_size(260.0)
        .frame(egui::Frame::new().fill(theme.bg_sunken))
        .show(ui, |ui| {
            settings_sidebar(ui, theme, state.active_section, &mut action);
        });

    egui::CentralPanel::default()
        .frame(egui::Frame::new().fill(theme.bg))
        .show(ui, |ui| {
            settings_content(
                ui,
                theme,
                state.active_section,
                state.settings,
                state.report_format,
            );
        });

    action
}

fn settings_sidebar(
    ui: &mut egui::Ui,
    theme: Theme,
    active_section: SettingsSection,
    action: &mut Option<SettingsScreenAction>,
) {
    ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
    ui.set_width(ui.available_width());
    ui.add_space(18.0);
    ui.horizontal(|ui| {
        ui.add_space(18.0);
        ui.vertical(|ui| {
            ui.set_width(224.0);
            ui.heading(egui::RichText::new("Settings").color(theme.text).size(18.0));

            ui.add_space(22.0);
            widgets::caption(ui, theme, "Sections");
            ui.add_space(8.0);
            if nav_item(
                ui,
                theme,
                "Appearance",
                active_section == SettingsSection::Appearance,
            )
            .clicked()
            {
                *action = Some(SettingsScreenAction::SelectSection(
                    SettingsSection::Appearance,
                ));
            }
            ui.add_space(6.0);
            if nav_item(
                ui,
                theme,
                "Report generation",
                active_section == SettingsSection::ReportGeneration,
            )
            .clicked()
            {
                *action = Some(SettingsScreenAction::SelectSection(
                    SettingsSection::ReportGeneration,
                ));
            }
        });
    });

    ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
        ui.add_space(20.0);
        ui.horizontal(|ui| {
            ui.add_space(18.0);
            ui.vertical(|ui| {
                ui.set_width(224.0);
                if widgets::sidebar_button(ui, theme, "Back to Capture").clicked() {
                    *action = Some(SettingsScreenAction::BackToCapture);
                }
            });
        });
    });
}

fn settings_content(
    ui: &mut egui::Ui,
    theme: Theme,
    active_section: SettingsSection,
    settings: &mut AppSettings,
    report_format: &mut ReportFormat,
) {
    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.add_space(22.0);
        ui.horizontal(|ui| {
            ui.add_space(24.0);
            ui.vertical(|ui| {
                ui.set_width((ui.available_width() - 24.0).max(0.0));
                ui.heading(
                    egui::RichText::new("App settings")
                        .color(theme.text)
                        .size(22.0),
                );
                ui.label(
                    egui::RichText::new("Changes apply immediately.")
                        .color(theme.text_muted)
                        .size(12.0),
                );

                ui.add_space(22.0);
                match active_section {
                    SettingsSection::Appearance => appearance_section(ui, theme, settings),
                    SettingsSection::ReportGeneration => report_section(ui, theme, report_format),
                }
                ui.add_space(22.0);
            });
        });
    });
}

fn appearance_section(ui: &mut egui::Ui, theme: Theme, settings: &mut AppSettings) {
    section(ui, theme, "Appearance", |ui| {
        ui.label(
            egui::RichText::new("Theme")
                .color(theme.text)
                .strong()
                .size(13.0),
        );
        ui.add_space(8.0);

        let previous_theme = settings.theme;
        if option_row(
            ui,
            theme,
            settings.theme == ThemeSetting::System,
            "Follow system",
            "Use the operating system light or dark appearance.",
        )
        .clicked()
        {
            settings.theme = ThemeSetting::System;
        }
        if option_row(
            ui,
            theme,
            settings.theme == ThemeSetting::Light,
            "Light",
            "Use FindBT's bright capture workspace.",
        )
        .clicked()
        {
            settings.theme = ThemeSetting::Light;
        }
        if option_row(
            ui,
            theme,
            settings.theme == ThemeSetting::Dark,
            "Dark",
            "Use FindBT's low-light capture workspace.",
        )
        .clicked()
        {
            settings.theme = ThemeSetting::Dark;
        }

        if settings.theme != previous_theme {
            settings.save();
        }
    });
}

fn report_section(ui: &mut egui::Ui, theme: Theme, report_format: &mut ReportFormat) {
    section(ui, theme, "Report generation", |ui| {
        ui.label(
            egui::RichText::new("Export format")
                .color(theme.text)
                .strong()
                .size(13.0),
        );
        ui.add_space(8.0);

        if option_row(
            ui,
            theme,
            *report_format == ReportFormat::Html,
            "HTML report",
            "Standalone web page that opens in any browser.",
        )
        .clicked()
        {
            *report_format = ReportFormat::Html;
        }
        if option_row(
            ui,
            theme,
            *report_format == ReportFormat::Pdf,
            "PDF report",
            "Fixed-layout printable document.",
        )
        .clicked()
        {
            *report_format = ReportFormat::Pdf;
        }

        ui.add_space(10.0);
        ui.label(
            egui::RichText::new(
                "Both formats contain the same phase runs, phase summary, device registry, and raw audit log.",
            )
            .color(theme.text_muted)
            .size(11.0),
        );
    });
}

fn section<R>(
    ui: &mut egui::Ui,
    theme: Theme,
    title: &str,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    let width = ui.available_width().min(680.0);
    ui.set_width(width);
    widgets::caption(ui, theme, title);
    ui.add_space(8.0);
    egui::Frame::new()
        .fill(theme.bg_elevated)
        .stroke(egui::Stroke::new(1.0, theme.border_soft))
        .corner_radius(6.0)
        .inner_margin(egui::Margin::same(14))
        .show(ui, add_contents)
        .inner
}

fn nav_item(ui: &mut egui::Ui, theme: Theme, label: &str, selected: bool) -> egui::Response {
    let fill = if selected {
        theme.accent_soft()
    } else {
        theme.bg_sunken
    };
    egui::Frame::new()
        .fill(fill)
        .stroke(egui::Stroke::new(1.0, theme.border_soft))
        .corner_radius(6.0)
        .inner_margin(egui::Margin::symmetric(10, 8))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.label(
                egui::RichText::new(label)
                    .color(theme.text)
                    .strong()
                    .size(12.0),
            );
        })
        .response
        .interact(egui::Sense::click())
}

fn option_row(
    ui: &mut egui::Ui,
    theme: Theme,
    selected: bool,
    title: &str,
    description: &str,
) -> egui::Response {
    let fill = if selected {
        theme.accent_soft()
    } else {
        theme.bg
    };
    let response = egui::Frame::new()
        .fill(fill)
        .stroke(egui::Stroke::new(1.0, theme.border))
        .corner_radius(6.0)
        .inner_margin(egui::Margin::symmetric(12, 10))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                radio_mark(ui, theme, selected);
                ui.add_space(8.0);
                ui.vertical(|ui| {
                    ui.label(
                        egui::RichText::new(title)
                            .color(theme.text)
                            .strong()
                            .size(12.0),
                    );
                    ui.label(
                        egui::RichText::new(description)
                            .color(theme.text_muted)
                            .size(11.0),
                    );
                });
            });
        })
        .response;
    let response = response.interact(egui::Sense::click());
    ui.add_space(8.0);
    response
}

fn radio_mark(ui: &mut egui::Ui, theme: Theme, selected: bool) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(18.0, 18.0), egui::Sense::hover());
    let center = rect.center();
    ui.painter()
        .circle_stroke(center, 8.0, egui::Stroke::new(1.4, theme.accent_main()));
    if selected {
        ui.painter().circle_filled(center, 4.5, theme.accent_main());
    }
}
