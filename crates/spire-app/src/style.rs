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

/// A complete visual treatment: palette, type, spacing and chip style.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Look {
    /// The original look: Ubuntu Light, compact.
    Standard,
    /// Higher-contrast palette and larger type; same font.
    Bright,
    /// Bright, set in Inter with semibold headings.
    Clear,
    /// Clear at a larger size with roomier spacing and tinted chips.
    Large,
}

impl Look {
    pub const ALL: [Look; 4] = [Look::Standard, Look::Bright, Look::Clear, Look::Large];

    pub fn name(self) -> &'static str {
        match self {
            Look::Standard => "standard",
            Look::Bright => "bright",
            Look::Clear => "clear",
            Look::Large => "large",
        }
    }

    pub fn parse(s: &str) -> Option<Look> {
        Look::ALL.into_iter().find(|l| l.name() == s)
    }

    fn palette(self) -> &'static Palette {
        match self {
            Look::Standard => &STANDARD,
            _ => &BRIGHT,
        }
    }

    /// Heading, body, small sizes.
    fn sizes(self) -> (f32, f32, f32) {
        match self {
            Look::Standard => (19.0, 14.0, 11.5),
            Look::Bright | Look::Clear => (20.0, 15.0, 12.5),
            Look::Large => (22.0, 16.5, 13.5),
        }
    }

    fn inter(self) -> bool {
        matches!(self, Look::Clear | Look::Large)
    }

    pub fn tinted_chips(self) -> bool {
        self == Look::Large
    }
}

pub struct Palette {
    pub bg: Color32,
    pub surface: Color32,
    pub surface_hi: Color32,
    pub input: Color32,
    pub border: Color32,
    pub border_hi: Color32,
    pub text: Color32,
    pub text_dim: Color32,
    pub text_faint: Color32,
    pub accent: Color32,
    pub good: Color32,
    pub warn: Color32,
    pub bad: Color32,
    pub info: Color32,
}

const STANDARD: Palette = Palette {
    bg: Color32::from_rgb(17, 19, 24),
    surface: Color32::from_rgb(26, 29, 36),
    surface_hi: Color32::from_rgb(36, 40, 50),
    input: Color32::from_rgb(12, 14, 18),
    border: Color32::from_rgb(44, 49, 60),
    border_hi: Color32::from_rgb(70, 77, 92),
    text: Color32::from_rgb(222, 226, 233),
    text_dim: Color32::from_rgb(146, 154, 168),
    text_faint: Color32::from_rgb(98, 106, 121),
    accent: Color32::from_rgb(226, 176, 74),
    good: Color32::from_rgb(98, 186, 124),
    warn: Color32::from_rgb(230, 165, 64),
    bad: Color32::from_rgb(228, 94, 94),
    info: Color32::from_rgb(96, 160, 224),
};

/// Text at least 7:1 against the background for body and dimmed text,
/// 4.5:1 for faint; brighter borders so panels read at a glance.
const BRIGHT: Palette = Palette {
    bg: Color32::from_rgb(13, 15, 19),
    surface: Color32::from_rgb(27, 30, 38),
    surface_hi: Color32::from_rgb(42, 47, 59),
    input: Color32::from_rgb(8, 9, 12),
    border: Color32::from_rgb(64, 71, 87),
    border_hi: Color32::from_rgb(100, 109, 128),
    text: Color32::from_rgb(242, 244, 248),
    text_dim: Color32::from_rgb(190, 197, 210),
    text_faint: Color32::from_rgb(146, 155, 172),
    accent: Color32::from_rgb(244, 194, 86),
    good: Color32::from_rgb(112, 210, 142),
    warn: Color32::from_rgb(248, 184, 76),
    bad: Color32::from_rgb(248, 114, 114),
    info: Color32::from_rgb(118, 184, 248),
};

static LOOK: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);

pub fn look() -> Look {
    Look::ALL[usize::from(LOOK.load(std::sync::atomic::Ordering::Relaxed)).min(Look::ALL.len() - 1)]
}

fn palette() -> &'static Palette {
    look().palette()
}

/// Palette accessors. Functions rather than constants so the look can
/// change at runtime; each is a relaxed atomic load and a field read.
pub mod color {
    use eframe::egui::Color32;

    macro_rules! accessors {
        ($($name:ident),*) => {$(
            #[inline]
            pub fn $name() -> Color32 {
                super::palette().$name
            }
        )*};
    }
    accessors!(
        bg, surface, surface_hi, input, border, border_hi, text, text_dim, text_faint, accent, good, warn, bad, info
    );
}

use color::*;

/// The family for emphasis (headings, titles, names): a real heavier
/// weight with Inter; egui's `strong()` only changes colour.
pub fn strong_family() -> FontFamily {
    FontFamily::Name("strong".into())
}

const INTER: &[u8] = include_bytes!("../assets/fonts/Inter-Regular.otf");
const INTER_SEMIBOLD: &[u8] = include_bytes!("../assets/fonts/Inter-SemiBold.otf");

fn fonts(look: Look) -> egui::FontDefinitions {
    use std::sync::Arc;
    let mut defs = egui::FontDefinitions::default();
    let proportional = defs.families.get(&FontFamily::Proportional).cloned().unwrap_or_default();
    if look.inter() {
        defs.font_data.insert("Inter".into(), Arc::new(egui::FontData::from_static(INTER)));
        defs.font_data.insert("Inter-SemiBold".into(), Arc::new(egui::FontData::from_static(INTER_SEMIBOLD)));
        let with = |first: &[&str]| first.iter().map(|s| s.to_string()).chain(proportional.iter().cloned()).collect();
        defs.families.insert(FontFamily::Proportional, with(&["Inter"]));
        defs.families.insert(strong_family(), with(&["Inter-SemiBold", "Inter"]));
    } else {
        defs.families.insert(strong_family(), proportional);
    }
    defs
}

pub fn card_type_color(t: CardType) -> Color32 {
    match t {
        CardType::Attack => Color32::from_rgb(224, 108, 92),
        CardType::Skill => Color32::from_rgb(92, 169, 224),
        CardType::Power => accent(),
        CardType::Curse => Color32::from_rgb(160, 108, 213),
        CardType::Status => text_faint(),
        CardType::Quest => good(),
        CardType::Other => text_dim(),
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
        _ => accent(),
    }
}

/// Green above 60% HP, amber above 30%, red below.
pub fn hp_color(fraction: f32) -> Color32 {
    if fraction > 0.6 {
        good()
    } else if fraction > 0.3 {
        warn()
    } else {
        bad()
    }
}

/// `SPIRE_STYLE=default` keeps egui's stock style (for A/B benchmarks).
pub fn tuned() -> bool {
    std::env::var("SPIRE_STYLE").map_or(true, |v| v != "default")
}

/// Applies `look`; call once at startup and again when the look changes.
pub fn set_look(ctx: &egui::Context, look: Look) {
    let index = Look::ALL.iter().position(|l| *l == look).unwrap_or(0);
    LOOK.store(index as u8, std::sync::atomic::Ordering::Relaxed);
    configure(ctx);
}

pub fn configure(ctx: &egui::Context) {
    if !tuned() {
        return;
    }
    let look = look();
    ctx.set_fonts(fonts(look));
    ctx.set_theme(egui::Theme::Dark);
    ctx.all_styles_mut(|style| {
        use FontFamily::{Monospace, Proportional};
        let (heading, body, small) = look.sizes();
        style.text_styles = [
            (TextStyle::Heading, FontId::new(heading, strong_family())),
            (TextStyle::Body, FontId::new(body, Proportional)),
            (TextStyle::Button, FontId::new(body, Proportional)),
            (TextStyle::Small, FontId::new(small, Proportional)),
            (TextStyle::Monospace, FontId::new(body - 1.0, Monospace)),
        ]
        .into();
        let roomy = look == Look::Large;
        style.spacing.item_spacing = if roomy { egui::vec2(10.0, 8.0) } else { egui::vec2(8.0, 6.0) };
        style.spacing.button_padding = egui::vec2(10.0, if roomy { 5.0 } else { 4.0 });
        style.spacing.interact_size.y = if roomy { 28.0 } else { 24.0 };
        style.animation_time = 0.0;

        let v = &mut style.visuals;
        v.dark_mode = true;
        v.panel_fill = Color32::TRANSPARENT;
        v.window_fill = surface();
        v.window_stroke = Stroke::new(1.0, border_hi());
        v.window_shadow = Shadow::NONE;
        v.popup_shadow = Shadow::NONE;
        v.window_corner_radius = CornerRadius::ZERO;
        v.menu_corner_radius = CornerRadius::ZERO;
        v.extreme_bg_color = input();
        v.faint_bg_color = surface();
        v.code_bg_color = input();
        v.hyperlink_color = info();
        v.warn_fg_color = warn();
        v.error_fg_color = bad();
        v.override_text_color = None;
        v.selection.bg_fill = accent().gamma_multiply(0.35);
        v.selection.stroke = Stroke::new(1.0, accent());

        let w = &mut v.widgets;
        w.noninteractive.bg_fill = surface();
        w.noninteractive.weak_bg_fill = surface();
        w.noninteractive.bg_stroke = Stroke::new(1.0, border());
        w.noninteractive.fg_stroke = Stroke::new(1.0, text());
        w.inactive.bg_fill = surface();
        w.inactive.weak_bg_fill = surface();
        w.inactive.bg_stroke = Stroke::new(1.0, border());
        w.inactive.fg_stroke = Stroke::new(1.0, text());
        w.hovered.bg_fill = surface_hi();
        w.hovered.weak_bg_fill = surface_hi();
        w.hovered.bg_stroke = Stroke::new(1.0, border_hi());
        w.hovered.fg_stroke = Stroke::new(1.0, Color32::WHITE);
        w.active.bg_fill = surface_hi();
        w.active.weak_bg_fill = surface_hi();
        w.active.bg_stroke = Stroke::new(1.0, accent());
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
    bg().to_normalized_gamma_f32()
}
