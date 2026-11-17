//! StS2 macro dashboard.

mod actions;
mod app;
mod draft;
mod views;
mod widgets;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("Spire Dashboard")
            .with_inner_size([1400.0, 900.0])
            .with_min_inner_size([900.0, 600.0]),
        ..Default::default()
    };
    eframe::run_native("spire-dashboard", options, Box::new(|cc| Ok(Box::new(app::SpireApp::new(cc)))))
}
