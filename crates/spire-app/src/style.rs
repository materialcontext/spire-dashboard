//! Visual language and frame-cost tuning.
//!
//! **Look.** A dark, game-adjacent theme with one warm accent (gold, like the
//! game's keyword text), semantic colours for coverage (good / thin /
//! missing), and fixed colours per card type and per deck role so the same
//! thing always reads the same way. Panels are outlined rather than filled.
//!
//! **Cost.** Measured with `frame_bench` and the render bench (see
//! `docs/PERFORMANCE.md`). Each setting keeps the look and removes work:
//!
//! - **No animations.** A hover change otherwise schedules several extra
//!   full repaints while highlights fade.
//! - **No window/popup shadows.** Large blurred meshes, shaded per pixel.
//! - **Transparent panels, clear to the background.** Panels used to fill the
//!   window on top of the cleared framebuffer: a redundant full-screen pass.
//! - **Outlined, not filled, cards.** Borders cost a few pixels; fills cost
//!   their whole area on software rasterisers.
//! - **Square corners, no feathering.** Rounded corners are fans of extra
//!   triangles and feathering adds a ring per shape. Text keeps its own
//!   anti-aliasing (glyphs are pre-rasterised).

use eframe::egui::{self, Color32, CornerRadius, FontFamily, FontId, Shadow, Stroke, TextStyle};
use spire_core::catalog::CardType;
use spire_core::playbook::Role;

pub mod color {
    use eframe::egui::Color32;

    pub const BG: Color32 = Color32::from_rgb(17, 19, 24);
    pub const SURFACE: Color32 = Color32::from_rgb(26, 29, 36);
    pub const SURFACE_HI: Color32 = Color32::from_rgb(36, 40, 50);
    pub const INPUT: Color32 = Color32::from_rgb(12, 14, 18);
    pub const BORDER: Color32 = Color32::from_rgb(44, 49, 60);
    pub const BORDER_HI: Color32 = Color32::from_rgb(70, 77, 92);

    pub const TEXT: Color32 = Color32::from_rgb(222, 226, 233);
    pub const TEXT_DIM: Color32 = Color32::from_rgb(146, 154, 168);
    pub const TEXT_FAINT: Color32 = Color32::from_rgb(98, 106, 121);

    pub const ACCENT: Color32 = Color32::from_rgb(226, 176, 74);
    pub const GOOD: Color32 = Color32::from_rgb(98, 186, 124);
    pub const WARN: Color32 = Color32::from_rgb(230, 165, 64);
    pub const BAD: Color32 = Color32::from_rgb(228, 94, 94);
    pub const INFO: Color32 = Color32::from_rgb(96, 160, 224);
}

use color::*;

pub fn card_type_color(t: CardType) -> Color32 {
    match t {
        CardType::Attack => Color32::from_rgb(224, 108, 92),
        CardType::Skill => Color32::from_rgb(92, 169, 224),
        CardType::Power => ACCENT,
        CardType::Curse => Color32::from_rgb(160, 108, 213),
        CardType::Status => TEXT_FAINT,
        CardType::Quest => GOOD,
        CardType::Other => TEXT_DIM,
    }
}

pub fn role_color(r: Role) -> Color32 {
    match r {
        Role::FrontloadDamage => Color32::from_rgb(224, 108, 92),
        Role::ScalingDamage => Color32::from_rgb(232, 150, 74),
        Role::FrontloadDefense => Color32::from_rgb(92, 169, 224),
        Role::ScalingDefense => Color32::from_rgb(79, 193, 181),
        Role::Consistency => Color32::from_rgb(180, 140, 224),
    }
}

/// Accent per character, matching their in-game card colours.
pub fn character_color(id: &str) -> Color32 {
    match id {
        "IRONCLAD" => Color32::from_rgb(214, 84, 76),
        "SILENT" => Color32::from_rgb(98, 186, 124),
        "DEFECT" => Color32::from_rgb(92, 169, 224),
        "NECROBINDER" => Color32::from_rgb(170, 120, 220),
        "REGENT" => Color32::from_rgb(232, 150, 74),
        "FOOL" => Color32::from_rgb(226, 196, 92),
        _ => ACCENT,
    }
}

/// Green above 60% HP, amber above 30%, red below.
pub fn hp_color(fraction: f32) -> Color32 {
    if fraction > 0.6 {
        GOOD
    } else if fraction > 0.3 {
        WARN
    } else {
        BAD
    }
}

/// `SPIRE_STYLE=default` keeps egui's stock style (for A/B benchmarks).
pub fn tuned() -> bool {
    std::env::var("SPIRE_STYLE").map_or(true, |v| v != "default")
}

pub fn configure(ctx: &egui::Context) {
    if !tuned() {
        return;
    }
    ctx.set_theme(egui::Theme::Dark);
    ctx.all_styles_mut(|style| {
        use FontFamily::{Monospace, Proportional};
        style.text_styles = [
            (TextStyle::Heading, FontId::new(19.0, Proportional)),
            (TextStyle::Body, FontId::new(14.0, Proportional)),
            (TextStyle::Button, FontId::new(14.0, Proportional)),
            (TextStyle::Small, FontId::new(11.5, Proportional)),
            (TextStyle::Monospace, FontId::new(13.0, Monospace)),
        ]
        .into();
        style.spacing.item_spacing = egui::vec2(8.0, 6.0);
        style.spacing.button_padding = egui::vec2(10.0, 4.0);
        style.spacing.interact_size.y = 24.0;
        style.animation_time = 0.0;

        let v = &mut style.visuals;
        v.dark_mode = true;
        v.panel_fill = Color32::TRANSPARENT;
        v.window_fill = SURFACE;
        v.window_stroke = Stroke::new(1.0, BORDER_HI);
        v.window_shadow = Shadow::NONE;
        v.popup_shadow = Shadow::NONE;
        v.window_corner_radius = CornerRadius::ZERO;
        v.menu_corner_radius = CornerRadius::ZERO;
        v.extreme_bg_color = INPUT;
        v.faint_bg_color = SURFACE;
        v.code_bg_color = INPUT;
        v.hyperlink_color = INFO;
        v.warn_fg_color = WARN;
        v.error_fg_color = BAD;
        v.override_text_color = None;
        v.selection.bg_fill = ACCENT.gamma_multiply(0.35);
        v.selection.stroke = Stroke::new(1.0, ACCENT);

        let w = &mut v.widgets;
        w.noninteractive.bg_fill = SURFACE;
        w.noninteractive.weak_bg_fill = SURFACE;
        w.noninteractive.bg_stroke = Stroke::new(1.0, BORDER);
        w.noninteractive.fg_stroke = Stroke::new(1.0, TEXT);
        w.inactive.bg_fill = SURFACE;
        w.inactive.weak_bg_fill = SURFACE;
        w.inactive.bg_stroke = Stroke::new(1.0, BORDER);
        w.inactive.fg_stroke = Stroke::new(1.0, TEXT);
        w.hovered.bg_fill = SURFACE_HI;
        w.hovered.weak_bg_fill = SURFACE_HI;
        w.hovered.bg_stroke = Stroke::new(1.0, BORDER_HI);
        w.hovered.fg_stroke = Stroke::new(1.0, Color32::WHITE);
        w.active.bg_fill = SURFACE_HI;
        w.active.weak_bg_fill = SURFACE_HI;
        w.active.bg_stroke = Stroke::new(1.0, ACCENT);
        w.active.fg_stroke = Stroke::new(1.0, Color32::WHITE);
        w.open = w.hovered;
        for s in [&mut w.noninteractive, &mut w.inactive, &mut w.hovered, &mut w.active, &mut w.open] {
            s.corner_radius = CornerRadius::ZERO;
            s.expansion = 0.0;
        }
    });
    // With square corners, anti-aliasing feathers only the odd diagonal
    // (checkmarks, arrows); text is pre-antialiased.
    ctx.tessellation_options_mut(|o| o.feathering = false);
}

/// The colour the framebuffer is cleared to (panels are transparent).
pub fn clear_color(_visuals: &egui::Visuals) -> [f32; 4] {
    if !tuned() {
        return Color32::from_rgba_unmultiplied(12, 12, 12, 180).to_normalized_gamma_f32();
    }
    BG.to_normalized_gamma_f32()
}
