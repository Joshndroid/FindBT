// Hide the console window on Windows in all builds. Without this the exe is
// a console-subsystem binary and Windows opens a terminal alongside the GUI.
#![windows_subsystem = "windows"]

mod app;
mod main_screen;
mod settings;
mod theme;
mod titlebar;
mod wizard;

use app::FindBtApp;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1180.0, 760.0])
            .with_min_inner_size([900.0, 620.0])
            .with_icon(load_app_icon())
            .with_decorations(false),
        ..Default::default()
    };

    eframe::run_native(
        "FindBT",
        options,
        Box::new(|cc| {
            configure_fonts(&cc.egui_ctx);
            Ok(Box::new(FindBtApp::new()))
        }),
    )
}

fn load_app_icon() -> egui::IconData {
    eframe::icon_data::from_png_bytes(include_bytes!("../assets/app-icon-256.png"))
        .expect("bundled app icon must be a valid PNG")
}

fn configure_fonts(ctx: &egui::Context) {
    // Prefer egui's bundled monospace font ("Hack") for proportional text so
    // the whole UI uses the same technical look. "Hack" is a font-data key in
    // egui's FontDefinitions::default(); family names like "Monospace" are
    // not valid here.
    let mut fonts = egui::FontDefinitions::default();
    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .insert(0, "Hack".to_string());
    ctx.set_fonts(fonts);
}
