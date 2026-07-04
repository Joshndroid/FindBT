use crate::theme::Theme;
use crate::wizard::logo;

/// Actions a title bar can emit beyond the standard window controls, which
/// are handled internally via viewport commands.
pub enum TitlebarAction {
    None,
    OpenSettings,
}

/// Shared custom title bar: app logo, app name, drag-to-move area, and
/// minimize / maximize / close controls. `with_settings` adds the Settings
/// button used on the main screen.
pub fn titlebar(ui: &mut egui::Ui, theme: Theme, with_settings: bool) -> TitlebarAction {
    let mut action = TitlebarAction::None;

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
        ui.label(egui::RichText::new("FindBT").color(theme.text).strong());
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
            if with_settings {
                ui.add_space(6.0);
                if ui.button("Settings").clicked() {
                    action = TitlebarAction::OpenSettings;
                }
            }
        });
    });

    action
}
