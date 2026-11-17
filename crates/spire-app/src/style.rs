//! Visual language and frame-cost tuning.
//!
//! **Larger text** is a readability option independent of the look: +10%
//! type, roomier spacing, filled chips and brighter tertiary text, applied
//! on top of whichever look is chosen.
//!
//! **Looks.** Every visual treatment is one entry in [`LOOKS`]: a palette,
//! a typeface, sizes, border weight, and a few switches (light/dark,
//! glowing borders, an accent that follows the current character). Adding
//! or tuning a look is editing that table; views only ask for colours by
//! role (`color::text()`, `color::accent()`, …) and never hard-code them.
//! Card-type, role and character colours are fixed hues, darkened
//! automatically on light looks so they stay readable.
//!
//! **Cost.** Measured with `frame_bench` and the render bench (see
//! `docs/PERFORMANCE.md`). Every look keeps these:
//!
//! - **No animations.** A hover change otherwise schedules several extra
//!   full repaints while highlights fade.
//! - **No window/popup shadows.** Large blurred meshes, shaded per pixel.
//! - **Transparent panels, clear to the background.** Panels used to fill the
//!   window on top of the cleared framebuffer: a redundant full-screen pass.
//! - **Outlined, not filled, cards.** Borders cost a few pixels; fills cost
//!   their whole area on software rasterisers. (Glow is one extra outline.)
//! - **Square corners, no feathering.** Rounded corners are fans of extra
//!   triangles and feathering adds a ring per shape. Text keeps its own
//!   anti-aliasing (glyphs are pre-rasterised).

use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

use eframe::egui::{self, Color32, CornerRadius, FontFamily, FontId, Shadow, Stroke, TextStyle};
use spire_core::catalog::CardType;
use spire_core::playbook::Role;

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Typeface {
    /// egui's built-in Ubuntu Light.
    Ubuntu,
    /// Inter Regular with Inter SemiBold for emphasis.
    Inter,
    /// Everything in the monospace face.
    Mono,
}

/// One complete visual treatment.
pub struct LookDef {
    /// Stable id, used in settings and `SPIRE_LOOK`.
    pub name: &'static str,
    pub label: &'static str,
    pub blurb: &'static str,
    pub palette: Palette,
    pub typeface: Typeface,
    /// Heading, body, small sizes in points.
    pub sizes: (f32, f32, f32),
    pub border: f32,
    pub light: bool,
    /// A faint second outline around panels.
    pub glow: bool,
    /// Accent follows the current character's colour.
    pub character_accent: bool,
    pub tinted_chips: bool,
}

const fn rgb(r: u8, g: u8, b: u8) -> Color32 {
    Color32::from_rgb(r, g, b)
}

/// High-contrast dark base (Spirebound, and the template for new looks).
const BRIGHT: Palette = Palette {
    bg: rgb(13, 15, 19),
    surface: rgb(27, 30, 38),
    surface_hi: rgb(42, 47, 59),
    input: rgb(8, 9, 12),
    border: rgb(64, 71, 87),
    border_hi: rgb(100, 109, 128),
    text: rgb(242, 244, 248),
    text_dim: rgb(190, 197, 210),
    text_faint: rgb(146, 155, 172),
    accent: rgb(244, 194, 86),
    good: rgb(112, 210, 142),
    warn: rgb(248, 184, 76),
    bad: rgb(248, 114, 114),
    info: rgb(118, 184, 248),
};

const BASE: LookDef = LookDef {
    name: "",
    label: "",
    blurb: "",
    palette: BRIGHT,
    typeface: Typeface::Inter,
    sizes: (20.0, 15.0, 12.5),
    border: 1.0,
    light: false,
    glow: false,
    character_accent: false,
    tinted_chips: false,
};

pub const LOOKS: &[LookDef] = &[
    LookDef {
        name: "spirebound",
        label: "Spirebound",
        blurb: "Accent follows your character.",
        character_accent: true,
        tinted_chips: true,
        ..BASE
    },
    LookDef {
        name: "forge",
        label: "Forge",
        blurb: "Charcoal, soot and molten embers.",
        palette: Palette {
            bg: rgb(17, 11, 9),
            surface: rgb(34, 22, 17),
            surface_hi: rgb(54, 34, 25),
            input: rgb(10, 6, 5),
            border: rgb(86, 50, 34),
            border_hi: rgb(150, 82, 48),
            text: rgb(250, 236, 222),
            text_dim: rgb(214, 182, 156),
            text_faint: rgb(160, 126, 102),
            accent: rgb(255, 128, 48),
            good: rgb(156, 214, 104),
            warn: rgb(255, 196, 70),
            bad: rgb(255, 84, 72),
            info: rgb(120, 186, 236),
        },
        tinted_chips: true,
        ..BASE
    },
    LookDef {
        name: "neon",
        label: "Neon Spire",
        blurb: "Synthwave: hot pink and cyan, glowing edges.",
        palette: Palette {
            bg: rgb(14, 7, 26),
            surface: rgb(30, 15, 50),
            surface_hi: rgb(48, 24, 78),
            input: rgb(8, 3, 16),
            border: rgb(110, 44, 160),
            border_hi: rgb(186, 84, 240),
            text: rgb(248, 238, 255),
            text_dim: rgb(206, 180, 236),
            text_faint: rgb(150, 118, 190),
            accent: rgb(255, 60, 172),
            good: rgb(54, 244, 206),
            warn: rgb(255, 204, 64),
            bad: rgb(255, 76, 112),
            info: rgb(76, 206, 255),
        },
        border: 1.5,
        glow: true,
        tinted_chips: true,
        ..BASE
    },
    LookDef {
        name: "grimoire",
        label: "Grimoire",
        blurb: "Parchment, ink and crimson rubrics. Light.",
        palette: Palette {
            bg: rgb(238, 228, 206),
            surface: rgb(228, 214, 186),
            surface_hi: rgb(214, 196, 162),
            input: rgb(248, 242, 228),
            border: rgb(170, 146, 108),
            border_hi: rgb(118, 94, 62),
            text: rgb(36, 25, 16),
            text_dim: rgb(86, 66, 46),
            text_faint: rgb(124, 102, 78),
            accent: rgb(150, 26, 34),
            good: rgb(36, 108, 48),
            warn: rgb(156, 92, 6),
            bad: rgb(166, 28, 28),
            info: rgb(28, 76, 148),
        },
        light: true,
        ..BASE
    },
    LookDef {
        name: "terminal",
        label: "Terminal",
        blurb: "Phosphor CRT run log. All monospace.",
        palette: Palette {
            bg: rgb(3, 9, 4),
            surface: rgb(8, 22, 10),
            surface_hi: rgb(14, 38, 18),
            input: rgb(1, 5, 2),
            border: rgb(26, 84, 36),
            border_hi: rgb(50, 150, 66),
            text: rgb(150, 255, 160),
            text_dim: rgb(96, 200, 110),
            text_faint: rgb(62, 140, 74),
            accent: rgb(255, 196, 64),
            good: rgb(120, 255, 140),
            warn: rgb(255, 196, 64),
            bad: rgb(255, 92, 80),
            info: rgb(96, 226, 226),
        },
        typeface: Typeface::Mono,
        sizes: (18.0, 14.0, 12.0),
        ..BASE
    },
];

/// The look new installs start with.
pub const DEFAULT_LOOK: &str = "spirebound";

static LOOK: AtomicUsize = AtomicUsize::new(0);
/// Current character colour (RGBA), or 0 for none; drives `character_accent`.
static CHARACTER: AtomicU32 = AtomicU32::new(0);
/// The "Larger text" readability option, applied on top of any look.
static LARGE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Whether "Larger text" is on.
pub fn large() -> bool {
    LARGE.load(Ordering::Relaxed)
}

/// Turns the "Larger text" option on or off and re-applies the look.
pub fn set_large(ctx: &egui::Context, on: bool) {
    LARGE.store(on, Ordering::Relaxed);
    configure(ctx);
}

/// Chips get a tinted fill with the look's own setting or Larger text.
pub fn tinted_chips() -> bool {
    current().tinted_chips || large()
}

pub fn current() -> &'static LookDef {
    &LOOKS[LOOK.load(Ordering::Relaxed).min(LOOKS.len() - 1)]
}

pub fn find_look(name: &str) -> Option<&'static LookDef> {
    LOOKS.iter().find(|l| l.name == name)
}

/// Palette accessors. Functions rather than constants so the look can
/// change at runtime; each is a relaxed atomic load and a field read.
pub mod color {
    use eframe::egui::Color32;

    macro_rules! accessors {
        ($($name:ident),*) => {$(
            #[inline]
            pub fn $name() -> Color32 {
                super::current().palette.$name
            }
        )*};
    }
    accessors!(bg, surface, surface_hi, input, border, border_hi, text, text_dim, good, warn, bad, info);

    /// Faint (tertiary) text. Larger text lifts it to the dimmed level so
    /// nothing on screen is low-contrast.
    #[inline]
    pub fn text_faint() -> Color32 {
        let p = &super::current().palette;
        if super::large() { p.text_dim } else { p.text_faint }
    }

    /// The look's accent, or the current character's colour on looks that
    /// follow the character.
    #[inline]
    pub fn accent() -> Color32 {
        let look = super::current();
        match super::CHARACTER.load(std::sync::atomic::Ordering::Relaxed) {
            c if look.character_accent && c != 0 => {
                let [r, g, b, _] = c.to_be_bytes();
                Color32::from_rgb(r, g, b)
            }
            _ => look.palette.accent,
        }
    }
}

use color::*;

/// Darkens fixed hues on light looks so they keep contrast on parchment.
fn adapt(c: Color32) -> Color32 {
    if current().light { Color32::from_rgb(c.r() / 2 + 10, c.g() / 2 + 10, c.b() / 2 + 10) } else { c }
}

pub fn card_type_color(t: CardType) -> Color32 {
    adapt(match t {
        CardType::Attack => rgb(224, 108, 92),
        CardType::Skill => rgb(92, 169, 224),
        CardType::Power => rgb(236, 186, 80),
        CardType::Curse => rgb(170, 116, 224),
        CardType::Status => rgb(150, 158, 172),
        CardType::Quest => rgb(110, 205, 140),
        CardType::Other => rgb(170, 178, 192),
    })
}

pub fn role_color(r: Role) -> Color32 {
    adapt(match r {
        Role::FrontloadDamage => rgb(232, 112, 96),
        Role::ScalingDamage => rgb(240, 156, 76),
        Role::FrontloadDefense => rgb(96, 172, 232),
        Role::ScalingDefense => rgb(80, 200, 188),
        Role::Consistency => rgb(186, 146, 232),
    })
}

/// Accent per character, matching their in-game card colours.
pub fn character_color(id: &str) -> Color32 {
    adapt(raw_character_color(id))
}

fn raw_character_color(id: &str) -> Color32 {
    match id {
        "IRONCLAD" => rgb(232, 88, 76),
        "SILENT" => rgb(104, 200, 128),
        "DEFECT" => rgb(96, 176, 236),
        "NECROBINDER" => rgb(182, 128, 236),
        "REGENT" => rgb(244, 156, 76),
        "FOOL" => rgb(236, 204, 92),
        _ => rgb(244, 194, 86),
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

/// The family for emphasis (headings, titles, names): a real heavier
/// weight with Inter; egui's `strong()` only changes colour.
pub fn strong_family() -> FontFamily {
    FontFamily::Name("strong".into())
}

const INTER: &[u8] = include_bytes!("../assets/fonts/Inter-Regular.otf");
const INTER_SEMIBOLD: &[u8] = include_bytes!("../assets/fonts/Inter-SemiBold.otf");

fn fonts(typeface: Typeface) -> egui::FontDefinitions {
    use std::sync::Arc;
    let mut defs = egui::FontDefinitions::default();
    let proportional = defs.families.get(&FontFamily::Proportional).cloned().unwrap_or_default();
    let monospace = defs.families.get(&FontFamily::Monospace).cloned().unwrap_or_default();
    let with = |first: &[&str], rest: &[String]| -> Vec<String> {
        first.iter().map(|s| s.to_string()).chain(rest.iter().cloned()).collect()
    };
    match typeface {
        Typeface::Inter => {
            defs.font_data.insert("Inter".into(), Arc::new(egui::FontData::from_static(INTER)));
            defs.font_data.insert("Inter-SemiBold".into(), Arc::new(egui::FontData::from_static(INTER_SEMIBOLD)));
            defs.families.insert(FontFamily::Proportional, with(&["Inter"], &proportional));
            defs.families.insert(strong_family(), with(&["Inter-SemiBold", "Inter"], &proportional));
        }
        Typeface::Mono => {
            defs.families.insert(FontFamily::Proportional, monospace.clone());
            defs.families.insert(strong_family(), monospace);
        }
        Typeface::Ubuntu => {
            defs.families.insert(strong_family(), proportional);
        }
    }
    defs
}

/// `SPIRE_STYLE=default` keeps egui's stock style (for A/B benchmarks).
pub fn tuned() -> bool {
    std::env::var("SPIRE_STYLE").map_or(true, |v| v != "default")
}

/// Applies the named look (unknown names fall back to the default).
pub fn set_look(ctx: &egui::Context, name: &str) {
    let index = LOOKS.iter().position(|l| l.name == name).or_else(|| LOOKS.iter().position(|l| l.name == DEFAULT_LOOK));
    LOOK.store(index.unwrap_or(0), Ordering::Relaxed);
    configure(ctx);
}

/// Tells character-reactive looks who is playing. Reconfigures only when
/// the character changes, so it is cheap to call every frame.
pub fn set_character(ctx: &egui::Context, id: Option<&str>) {
    let packed = id.map_or(0, |id| {
        let c = raw_character_color(id);
        u32::from_be_bytes([c.r(), c.g(), c.b(), 255])
    });
    if CHARACTER.swap(packed, Ordering::Relaxed) != packed && current().character_accent {
        configure(ctx);
    }
}

pub fn configure(ctx: &egui::Context) {
    if !tuned() {
        return;
    }
    let look = current();
    ctx.set_fonts(fonts(look.typeface));
    ctx.set_theme(if look.light { egui::Theme::Light } else { egui::Theme::Dark });
    ctx.all_styles_mut(|style| {
        use FontFamily::{Monospace, Proportional};
        // Larger text: every size up 10% (rounded to half points).
        let scale = |v: f32| if large() { (v * 1.1 * 2.0).round() / 2.0 } else { v };
        let (heading, body, small) = (scale(look.sizes.0), scale(look.sizes.1), scale(look.sizes.2));
        let roomy = large();
        style.text_styles = [
            (TextStyle::Heading, FontId::new(heading, strong_family())),
            (TextStyle::Body, FontId::new(body, Proportional)),
            (TextStyle::Button, FontId::new(body, Proportional)),
            (TextStyle::Small, FontId::new(small, Proportional)),
            (TextStyle::Monospace, FontId::new(body - 1.0, Monospace)),
        ]
        .into();
        style.spacing.item_spacing = if roomy { egui::vec2(10.0, 8.0) } else { egui::vec2(8.0, 6.0) };
        style.spacing.button_padding = egui::vec2(10.0, if roomy { 5.0 } else { 4.0 });
        style.spacing.interact_size.y = if roomy { 28.0 } else { 24.0 };
        style.animation_time = 0.0;

        let strong_fg = if look.light { Color32::BLACK } else { Color32::WHITE };
        let line = look.border;
        let v = &mut style.visuals;
        v.dark_mode = !look.light;
        v.panel_fill = Color32::TRANSPARENT;
        v.window_fill = surface();
        v.window_stroke = Stroke::new(line, border_hi());
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
        w.noninteractive.bg_stroke = Stroke::new(line, border());
        w.noninteractive.fg_stroke = Stroke::new(1.0, text());
        w.inactive.bg_fill = surface();
        w.inactive.weak_bg_fill = surface();
        w.inactive.bg_stroke = Stroke::new(line, border());
        w.inactive.fg_stroke = Stroke::new(1.0, text());
        w.hovered.bg_fill = surface_hi();
        w.hovered.weak_bg_fill = surface_hi();
        w.hovered.bg_stroke = Stroke::new(line, border_hi());
        w.hovered.fg_stroke = Stroke::new(1.0, strong_fg);
        w.active.bg_fill = surface_hi();
        w.active.weak_bg_fill = surface_hi();
        w.active.bg_stroke = Stroke::new(line, accent());
        w.active.fg_stroke = Stroke::new(1.0, strong_fg);
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
