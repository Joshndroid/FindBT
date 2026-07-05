//! Shared, theme-driven building blocks used by every screen (wizard,
//! titlebar, main screen, settings). Centralizing them here is what keeps
//! the app's look consistent end to end: one accent-filled "primary" button
//! style, one neutral "secondary" button style, one boxed input style, one
//! muted caption style, one card container style.
//!
//! The overall aesthetic mixes two references on purpose: containers
//! (`card_frame`) use the generous, soft-rounded corners of a macOS
//! dialog/sheet, while controls (`primary_button`, `secondary_button`,
//! `text_field`) use the tighter, rectangular corners and flat fills of a
//! Windows 11 setup wizard. Neither platform's chrome dominates, which is
//! the point for a cross-platform app.

use crate::theme::{hex, Theme};

/// The strong call-to-action button: solid accent fill, light text, tight
/// control-radius corners. There should be at most one of these visible per
/// screen (e.g. "Begin Scan", "Generate Report").
pub fn primary_button(ui: &mut egui::Ui, theme: Theme, text: &str) -> egui::Response {
    primary_button_enabled(ui, theme, text, true)
}

/// Same as [`primary_button`], but can be disabled (e.g. while required
/// fields are incomplete). Disabled buttons keep their shape and position so
/// the layout does not jump once they become active.
pub fn primary_button_enabled(
    ui: &mut egui::Ui,
    theme: Theme,
    text: &str,
    enabled: bool,
) -> egui::Response {
    ui.add_enabled(
        enabled,
        egui::Button::new(egui::RichText::new(text).color(theme.accent_text).strong())
            .fill(theme.accent_main())
            .corner_radius(theme.control_radius)
            .min_size(egui::vec2(120.0, 34.0)),
    )
}

/// The lower-emphasis button that sits alongside a primary action, or stands
/// alone for routine actions (e.g. "Settings", "Reset capture"). Neutral
/// fill, thin border, same control-radius and height as the primary button
/// so the two always line up.
pub fn secondary_button(ui: &mut egui::Ui, theme: Theme, text: &str) -> egui::Response {
    ui.add(
        egui::Button::new(egui::RichText::new(text).color(theme.text))
            .fill(theme.bg_elevated)
            .stroke(egui::Stroke::new(1.0, theme.border))
            .corner_radius(theme.control_radius)
            .min_size(egui::vec2(96.0, 34.0)),
    )
}

/// Full-width sidebar footer button. Used for screen navigation actions so
/// capture/settings behave consistently from the same bottom-left location.
pub fn sidebar_button(ui: &mut egui::Ui, theme: Theme, text: &str) -> egui::Response {
    ui.add_sized(
        egui::vec2(ui.available_width(), 34.0),
        egui::Button::new(egui::RichText::new(text).color(theme.text).strong())
            .fill(theme.bg_elevated)
            .stroke(egui::Stroke::new(1.0, theme.border))
            .corner_radius(theme.control_radius),
    )
}

/// Extra width a `text_field` box adds beyond its `width` argument: inner
/// margin on both sides plus the border stroke. Callers that need to center
/// the box themselves (see `wizard::field`) use this to compute the box's
/// true on-screen footprint.
pub const TEXT_FIELD_PADDING: f32 = 22.0;

/// A single-line text input drawn as its own boxed control: flat fill, thin
/// border, control-radius corners. The `TextEdit`'s own frame is turned off
/// so this box is the only chrome the user sees, rather than two borders
/// stacked on top of each other. Text (and the hint) is centered
/// horizontally, matching the centered caption above it and the centered
/// box itself.
///
/// `width` is the fixed display width of the box; its full on-screen width
/// is `width + TEXT_FIELD_PADDING`. Callers that want the box centered
/// rather than stretched edge-to-edge should indent by half the difference
/// between the available width and that total.
pub fn text_field(
    ui: &mut egui::Ui,
    theme: Theme,
    value: &mut String,
    hint: &str,
    width: f32,
) -> egui::Response {
    egui::Frame::new()
        .fill(theme.bg)
        .stroke(egui::Stroke::new(1.0, theme.border))
        .corner_radius(theme.control_radius)
        .inner_margin(egui::Margin::symmetric(10, 6))
        .show(ui, |ui| {
            ui.add(
                egui::TextEdit::singleline(value)
                    .hint_text(hint)
                    .desired_width(width)
                    .horizontal_align(egui::Align::Center)
                    .frame(egui::Frame::NONE),
            )
        })
        .inner
}

/// Small muted, uppercase caption used above fields and above grouped
/// sections ("DATE", "HOST ADAPTER", "PHASES", table headers, ...). One
/// helper for every screen keeps this label style from drifting.
pub fn caption(ui: &mut egui::Ui, theme: Theme, text: &str) {
    ui.label(
        egui::RichText::new(text.to_uppercase())
            .color(theme.text_muted)
            .size(10.0)
            .strong(),
    );
}

/// The rounded, bordered container used for the wizard card and other
/// elevated surfaces. Uses the softer `card_radius` (macOS-like) rather than
/// the tighter `control_radius` used for buttons and inputs inside it.
pub fn card_frame(theme: Theme) -> egui::Frame {
    egui::Frame::new()
        .fill(theme.bg_elevated)
        .stroke(egui::Stroke::new(1.0, theme.border))
        .corner_radius(theme.card_radius)
}

/// Which window-caption action a `titlebar_button` represents. Icons are
/// hand-drawn with the painter (a line, a square outline, an X) rather than
/// rendered as text glyphs, because the app's UI font ("Hack", a
/// programming monospace font) doesn't reliably include glyphs for the
/// Unicode minimize/maximize/close symbols and was falling back to a
/// generic missing-character box for all three, making them indistinguishable.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum CaptionIcon {
    Minimize,
    Maximize,
    Close,
}

/// A Windows-style window-caption button: no border or fill at rest, a flat
/// rectangular hover highlight, and (for the close button) a red hover
/// highlight with a light icon. Used for the custom titlebar's minimize /
/// maximize / close controls on every screen.
pub fn titlebar_button(
    ui: &mut egui::Ui,
    theme: Theme,
    icon: CaptionIcon,
    danger: bool,
) -> egui::Response {
    let size = egui::vec2(44.0, 32.0);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
    let hovered = response.hovered();
    if hovered {
        let bg = if danger {
            hex(0xc42b1c)
        } else {
            theme.bg_sunken
        };
        ui.painter().rect_filled(rect, 0.0, bg);
    }
    let color = if hovered && danger {
        egui::Color32::WHITE
    } else {
        theme.text
    };
    let stroke = egui::Stroke::new(1.2, color);
    let center = rect.center();
    let half = 4.5;
    match icon {
        CaptionIcon::Minimize => {
            ui.painter().line_segment(
                [
                    egui::pos2(center.x - half, center.y),
                    egui::pos2(center.x + half, center.y),
                ],
                stroke,
            );
        }
        CaptionIcon::Maximize => {
            let square = egui::Rect::from_center_size(center, egui::vec2(half * 2.0, half * 2.0));
            ui.painter()
                .rect_stroke(square, 0.0, stroke, egui::StrokeKind::Middle);
        }
        CaptionIcon::Close => {
            ui.painter().line_segment(
                [
                    egui::pos2(center.x - half, center.y - half),
                    egui::pos2(center.x + half, center.y + half),
                ],
                stroke,
            );
            ui.painter().line_segment(
                [
                    egui::pos2(center.x - half, center.y + half),
                    egui::pos2(center.x + half, center.y - half),
                ],
                stroke,
            );
        }
    }
    response
}
