use crate::theme::Theme;
use crate::widgets;
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
pub fn titlebar(
    ui: &mut egui::Ui,
    theme: Theme,
    with_settings: bool,
    icon: &egui::TextureHandle,
) -> TitlebarAction {
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

    // Explicit height, matching the titlebar panel's own height, so the
    // logo and label are guaranteed to sit vertically centered in the strip
    // rather than top-aligned within whatever height egui infers from the
    // row's content (which was clipping the top of the icon).
    let row_size = egui::vec2(ui.available_width(), 40.0);
    ui.allocate_ui_with_layout(
        row_size,
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            ui.add_space(12.0);
            logo(ui, icon, 20.0);
            ui.add_space(6.0);
            ui.label(
                egui::RichText::new("FindBT")
                    .color(theme.text)
                    .strong()
                    .size(13.0),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Windows caption-button order, left to right: minimize,
                // maximize, close. Building right-to-left, close goes first
                // so it lands furthest right.
                if widgets::titlebar_button(ui, theme, widgets::CaptionIcon::Close, true)
                    .clicked()
                {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                }
                if widgets::titlebar_button(ui, theme, widgets::CaptionIcon::Maximize, false)
                    .clicked()
                {
                    let maximized =
                        ui.input(|input| input.viewport().maximized.unwrap_or(false));
                    ui.ctx()
                        .send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
                }
                if widgets::titlebar_button(ui, theme, widgets::CaptionIcon::Minimize, false)
                    .clicked()
                {
                    ui.ctx()
                        .send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                }
                if with_settings {
                    ui.add_space(10.0);
                    if widgets::secondary_button(ui, theme, "Settings").clicked() {
                        action = TitlebarAction::OpenSettings;
                    }
                }
            });
        },
    );

    action
}
