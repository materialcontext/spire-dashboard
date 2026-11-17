//! Look-and-feel tuned for cheap frames on weak hardware.
//!
//! Measured with `frame_bench` and the in-window render bench (see
//! `docs/PERFORMANCE.md`). Each setting keeps the look and removes work:
//!
//! - **No animations.** A hover change otherwise schedules several extra
//!   full repaints while highlights fade; on software rendering each repaint
//!   costs a whole frame.
//! - **No window/popup shadows.** Shadows are large blurred meshes that the
//!   rasteriser must shade pixel by pixel.
//! - **Transparent panels, clear to the panel colour.** Panels used to fill
//!   the whole window on top of the cleared framebuffer: one redundant
//!   full-screen blend pass per frame.
//! - **Square corners, no feathering.** Every rounded corner is a fan of
//!   extra triangles and anti-aliasing feathers add a ring around each
//!   shape; software rasterisers pay per triangle. Text keeps its own
//!   anti-aliasing (glyphs are pre-rasterised).

use eframe::egui::{self, Color32, Shadow};

/// `SPIRE_STYLE=default` keeps egui's stock style (for A/B benchmarks).
pub fn tuned() -> bool {
    std::env::var("SPIRE_STYLE").map_or(true, |v| v != "default")
}

pub fn configure(ctx: &egui::Context) {
    if !tuned() {
        return;
    }
    ctx.all_styles_mut(|style| {
        style.animation_time = 0.0;
        style.visuals.window_shadow = Shadow::NONE;
        style.visuals.popup_shadow = Shadow::NONE;
        style.visuals.panel_fill = Color32::TRANSPARENT;
        let w = &mut style.visuals.widgets;
        for v in [&mut w.noninteractive, &mut w.inactive, &mut w.hovered, &mut w.active, &mut w.open] {
            v.corner_radius = egui::CornerRadius::ZERO;
        }
        style.visuals.menu_corner_radius = egui::CornerRadius::ZERO;
        style.visuals.window_corner_radius = egui::CornerRadius::ZERO;
    });
    // With square corners, anti-aliasing feathers only the odd diagonal
    // (checkmarks, arrows); text is pre-antialiased. Feathering would
    // double the triangles of every other shape.
    ctx.tessellation_options_mut(|o| o.feathering = false);
}

/// The colour the framebuffer is cleared to: the stock panel colour when
/// panels are transparent.
pub fn clear_color(visuals: &egui::Visuals) -> [f32; 4] {
    if !tuned() {
        return Color32::from_rgba_unmultiplied(12, 12, 12, 180).to_normalized_gamma_f32();
    }
    let stock = if visuals.dark_mode { egui::Visuals::dark() } else { egui::Visuals::light() };
    stock.panel_fill.to_normalized_gamma_f32()
}
