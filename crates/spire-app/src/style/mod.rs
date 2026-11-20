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

mod system;

use std::cell::{Cell, RefCell};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use eframe::egui::{self, Color32, CornerRadius, FontFamily, FontId, Shadow, Stroke, TextStyle};
use spire_core::analysis::tags::Tag;
use spire_core::catalog::CardType;
use spire_core::playbook::Role;

#[derive(Clone, Copy, Debug, PartialEq)]
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
    /// The OS's own UI font (Segoe UI Variable, SF Pro), read from the
    /// system at runtime; Inter when it can't be.
    System,
}

/// The platform a System look follows: its conventions as well as its
/// colours (navigation, Settings, dialogs, menus).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Native {
    Windows,
    Mac,
}

/// How a panel is drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PanelStyle {
    /// A border and no fill (cheap on software renderers).
    Outlined,
    /// Fluent's card: a translucent fill, a hairline stroke, rounded.
    Card,
    /// macOS grouped content: a soft fill, no stroke, rounded.
    Grouped,
}

/// Control fills by state, for looks whose controls are filled.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fills {
    pub rest: Color32,
    pub hover: Color32,
    pub pressed: Color32,
}

/// One complete visual treatment.
#[derive(Clone, Copy, Debug, PartialEq)]
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
    /// Section headings and stat captions in capitals. Only the Terminal
    /// look, where it is the style; elsewhere headings are sentence case.
    pub caps_headings: bool,
    /// Brief fades and slides (83 to 167 ms) where something changes.
    pub motion: bool,
    /// A System look: follows this platform's conventions.
    pub native: Option<Native>,
    /// Corner radius of controls and list rows, and of overlays (windows,
    /// menus, dialogs, cards). 0 keeps square corners and no feathering.
    pub radius: u8,
    pub overlay_radius: u8,
    /// Height of buttons and fields, before Larger text.
    pub control_height: f32,
    pub button_padding: (f32, f32),
    pub panel: PanelStyle,
    pub card_fill: Color32,
    pub card_stroke: Color32,
    /// The content layer drawn under the panels (Fluent's layer on Mica,
    /// macOS's window background beside the sidebar). Transparent for none.
    pub layer: Color32,
    /// Text on an accent fill; the background colour when unset.
    pub on_accent: Option<Color32>,
    /// Filled controls; outlined ones on the surface when unset.
    pub fills: Option<Fills>,
    /// Soft shadows under menus and dialogs.
    pub shadows: bool,
    /// The window may show a system material (Mica) through the content.
    pub translucent: bool,
}

const fn rgb(r: u8, g: u8, b: u8) -> Color32 {
    Color32::from_rgb(r, g, b)
}

/// Spacing tokens on a 4px grid (Fluent's rhythm). Larger text scales
/// them with the type, so the layout keeps its proportions.
pub mod space {
    fn k() -> f32 {
        if super::large() { 1.25 } else { 1.0 }
    }
    pub fn xs() -> f32 {
        4.0 * k()
    }
    pub fn s() -> f32 {
        8.0 * k()
    }
    pub fn m() -> f32 {
        12.0 * k()
    }
    pub fn l() -> f32 {
        16.0 * k()
    }
    pub fn xl() -> f32 {
        24.0 * k()
    }
}

/// `a` moved `t` of the way towards `b`.
fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let m = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgb(m(a.r(), b.r()), m(a.g(), b.g()), m(a.b(), b.b()))
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
    caps_headings: false,
    motion: true,
    native: None,
    radius: 0,
    overlay_radius: 0,
    control_height: 24.0,
    button_padding: (10.0, 4.0),
    panel: PanelStyle::Outlined,
    card_fill: Color32::TRANSPARENT,
    card_stroke: Color32::TRANSPARENT,
    layer: Color32::TRANSPARENT,
    on_accent: None,
    fills: None,
    shadows: false,
    translucent: false,
};

pub const LOOKS: &[LookDef] = &[
    LookDef {
        name: "spirebound",
        label: "Spirebound",
        blurb: "Your character's colour marks the header; gold marks what you can act on.",
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
            // Light looks layer the other way round from dark ones: raised
            // surfaces (windows, menus) are lighter than the page, and hover
            // darkens. Faint text and warnings stay above 4.5:1 on the page.
            bg: rgb(238, 228, 206),
            surface: rgb(248, 243, 231),
            surface_hi: rgb(222, 209, 182),
            input: rgb(252, 249, 241),
            border: rgb(170, 146, 108),
            border_hi: rgb(118, 94, 62),
            text: rgb(36, 25, 16),
            text_dim: rgb(86, 66, 46),
            text_faint: rgb(104, 82, 60),
            accent: rgb(150, 26, 34),
            good: rgb(36, 108, 48),
            warn: rgb(138, 80, 0),
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
        caps_headings: true,
        motion: false,
        ..BASE
    },
];

/// The System look's id: built at runtime for the platform (see `system`).
pub const SYSTEM_LOOK: &str = "system";

/// The look new installs start with: System on Windows and macOS
/// (decision D1), Spirebound elsewhere.
pub const DEFAULT_LOOK: &str = if cfg!(any(windows, target_os = "macos")) { SYSTEM_LOOK } else { "spirebound" };

thread_local! {
    /// The look in use. Read by every colour accessor, so a thread-local
    /// cell (one load) rather than a lock. The UI runs on one thread.
    static CURRENT: Cell<&'static LookDef> = const { Cell::new(&LOOKS[0]) };
    /// What the OS last said about its look.
    static APPEARANCE: Cell<crate::platform::Appearance> = Cell::new(crate::platform::Appearance::default());
    /// System looks built so far, reused when the same one comes back
    /// (each is a few hundred bytes, kept for the life of the process).
    static BUILT: RefCell<Vec<&'static LookDef>> = const { RefCell::new(Vec::new()) };
}

/// The platform whose System look this build offers: this OS, or the one
/// named by `SPIRE_SYSTEM=windows|macos` (to preview it anywhere).
pub fn native() -> Option<Native> {
    static NATIVE: OnceLock<Option<Native>> = OnceLock::new();
    *NATIVE.get_or_init(|| match std::env::var("SPIRE_SYSTEM").as_deref() {
        Ok("windows") => Some(Native::Windows),
        Ok("macos") => Some(Native::Mac),
        _ if cfg!(windows) => Some(Native::Windows),
        _ if cfg!(target_os = "macos") => Some(Native::Mac),
        _ => None,
    })
}

/// The window shows a system material (Mica) behind transparent areas.
static MATERIAL: AtomicBool = AtomicBool::new(false);

/// Records whether the window got its material (see `chrome::material`).
pub fn set_material(on: bool) {
    MATERIAL.store(on, Ordering::Relaxed);
}
/// Current character colour (RGBA), or 0 for none; drives `character_accent`.
static CHARACTER: AtomicU32 = AtomicU32::new(0);
/// The "Larger text" readability option, applied on top of any look.
static LARGE: AtomicBool = AtomicBool::new(false);
/// The system asked for less motion.
static REDUCE_MOTION: AtomicBool = AtomicBool::new(false);

/// Follows the system's reduced-motion preference.
pub fn set_reduce_motion(ctx: &egui::Context, on: bool) {
    REDUCE_MOTION.store(on, Ordering::Relaxed);
    configure(ctx);
}

/// Whether brief animations play: the look allows them and the system
/// hasn't asked for less motion.
pub fn motion() -> bool {
    current().motion && !REDUCE_MOTION.load(Ordering::Relaxed)
}

/// Fluent's timings: fades 83 ms, moves 167 ms.
pub const FADE: f32 = 0.083;
pub const MOVE: f32 = 0.167;

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

#[inline]
pub fn current() -> &'static LookDef {
    CURRENT.with(Cell::get)
}

/// A look by id. The System look is listed only where it's offered.
pub fn find_look(name: &str) -> Option<&'static LookDef> {
    if name == SYSTEM_LOOK {
        return native().map(|n| system_look(n, current().light));
    }
    LOOKS.iter().find(|l| l.name == name)
}

/// The looks to offer: System first where there is one, then the themes.
pub fn looks() -> Vec<(&'static str, &'static str, &'static str)> {
    let system = native().map(|n| {
        let blurb = system_look(n, current().light).blurb;
        (SYSTEM_LOOK, "System", blurb)
    });
    system.into_iter().chain(LOOKS.iter().map(|l| (l.name, l.label, l.blurb))).collect()
}

/// The System look for this mode and the OS's current appearance.
fn system_look(native: Native, light: bool) -> &'static LookDef {
    let look = system::look(native, light, &APPEARANCE.with(Cell::get));
    BUILT.with(|b| {
        let mut b = b.borrow_mut();
        if let Some(found) = b.iter().find(|l| ***l == look) {
            return *found;
        }
        let leaked: &'static LookDef = Box::leak(Box::new(look));
        b.push(leaked);
        leaked
    })
}

/// Whether the System look is showing.
pub fn is_system() -> bool {
    current().native.is_some()
}

/// Takes in what the OS says about its look. Rebuilds the System look if
/// it's showing and anything changed.
pub fn set_appearance(ctx: &egui::Context, a: crate::platform::Appearance) {
    if APPEARANCE.with(|c| c.replace(a)) != a && is_system() {
        let light = current().light;
        if let Some(n) = native() {
            CURRENT.with(|c| c.set(system_look(n, light)));
            configure(ctx);
        }
    }
}

/// Light or dark for the System look: the OS's mode as eframe sees it,
/// else as the platform module read it.
fn system_light(ctx: &egui::Context) -> bool {
    // SPIRE_THEME=light|dark pins the mode (previews and screenshots).
    match std::env::var("SPIRE_THEME").as_deref() {
        Ok("light") => return true,
        Ok("dark") => return false,
        _ => {}
    }
    match ctx.system_theme() {
        Some(t) => t == egui::Theme::Light,
        None => APPEARANCE.with(Cell::get).dark.is_some_and(|d| !d),
    }
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

    /// The look's accent. It means one thing: you can act on this, or it
    /// is selected (primary buttons, selected tabs, focus, links, callouts
    /// that need a choice). Data emphasis uses weight, not this colour.
    #[inline]
    pub fn accent() -> Color32 {
        super::current().palette.accent
    }

    /// Text on an accent fill (selected chips, primary buttons).
    #[inline]
    pub fn on_accent() -> Color32 {
        let look = super::current();
        look.on_accent.unwrap_or(look.palette.bg)
    }

    /// The identity colour: the current character's on looks that follow
    /// the character (the header band), else the accent. Never used to
    /// mean "interactive".
    #[inline]
    pub fn identity() -> Color32 {
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

/// Stars (the Regent's resource): a pale star-white, apart from Scaling
/// Damage's gold.
pub fn star_color() -> Color32 {
    adapt(rgb(238, 228, 160))
}

/// Gold's own colour (the in-game coin), for gold amounts.
pub fn gold() -> Color32 {
    adapt(rgb(232, 186, 72))
}

pub fn role_color(r: Role) -> Color32 {
    adapt(match r {
        Role::Mitigation => rgb(90, 140, 240),
        Role::Survival => rgb(80, 200, 188),
        Role::FastDamage => rgb(232, 112, 96),
        Role::ScalingDamage => rgb(224, 180, 60),
        Role::Aoe => rgb(240, 150, 70),
        Role::Chip => rgb(226, 118, 178),
    })
}

/// What a card does beside its roles (draw, energy): a tag, not a role.
pub fn tag_color() -> Color32 {
    adapt(rgb(186, 146, 232))
}

/// One tag's colour: Energy has its own, so it reads apart from Draw where
/// the two sit side by side; every other tag shares [`tag_color`].
pub fn tag_color_of(t: Tag) -> Color32 {
    match t {
        Tag::Energy => adapt(rgb(164, 212, 84)),
        _ => tag_color(),
    }
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

const INTER: &[u8] = include_bytes!("../../assets/fonts/Inter-Regular.otf");
const INTER_SEMIBOLD: &[u8] = include_bytes!("../../assets/fonts/Inter-SemiBold.otf");

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
        Typeface::System => match crate::platform::system_font() {
            Some(font) => {
                let (regular, strong) = system_fonts(font);
                defs.font_data.insert("System".into(), Arc::new(regular));
                defs.font_data.insert("System-Strong".into(), Arc::new(strong));
                defs.families.insert(FontFamily::Proportional, with(&["System"], &proportional));
                defs.families.insert(strong_family(), with(&["System-Strong", "System"], &proportional));
            }
            // No system font to read: Inter, as the other looks use.
            None => return fonts(Typeface::Inter),
        },
    }
    defs
}

/// Regular and emphasis faces from the system font: one variable font at
/// two weights, or two files.
fn system_fonts(font: crate::platform::SystemFont) -> (egui::FontData, egui::FontData) {
    use crate::platform::SystemFont;
    use egui::epaint::text::{FontTweak, VariationCoords};
    match font {
        SystemFont::Variable { data, regular, strong, optical } => {
            let at = |weight: f32| {
                let coords = match optical {
                    Some(o) => VariationCoords::new([(b"wght", weight), (b"opsz", o)]),
                    None => VariationCoords::new([(b"wght", weight)]),
                };
                egui::FontData::from_static(data).tweak(FontTweak { coords, ..FontTweak::default() })
            };
            (at(regular), at(strong))
        }
        SystemFont::Static { regular, strong } => {
            (egui::FontData::from_static(regular), egui::FontData::from_static(strong))
        }
    }
}

/// `SPIRE_STYLE=default` keeps egui's stock style (for A/B benchmarks).
pub fn tuned() -> bool {
    std::env::var("SPIRE_STYLE").map_or(true, |v| v != "default")
}

/// Applies the named look (unknown names fall back to the default). The
/// System look follows the OS's light or dark mode.
pub fn set_look(ctx: &egui::Context, name: &str) {
    let look = match (name, native()) {
        (SYSTEM_LOOK, Some(n)) => system_look(n, system_light(ctx)),
        _ => LOOKS
            .iter()
            .find(|l| l.name == name)
            .or_else(|| LOOKS.iter().find(|l| l.name == DEFAULT_LOOK))
            .unwrap_or(&LOOKS[0]),
    };
    CURRENT.with(|c| c.set(look));
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
        let (pad_x, pad_y) = look.button_padding;
        style.spacing.button_padding = egui::vec2(pad_x, if roomy { pad_y + 1.0 } else { pad_y });
        style.spacing.interact_size.y = if roomy { look.control_height + 4.0 } else { look.control_height };
        // Popups and collapsing sections fade briefly; nothing animates while idle.
        style.animation_time = if motion() { FADE } else { 0.0 };

        let strong_fg = if look.light { Color32::BLACK } else { Color32::WHITE };
        // Pressed is one step past hover: lighter on dark looks, darker on light ones.
        let pressed = mix(surface_hi(), strong_fg, 0.1);
        let line = look.border;
        let control = CornerRadius::same(look.radius);
        let overlay = CornerRadius::same(look.overlay_radius);
        let v = &mut style.visuals;
        v.dark_mode = !look.light;
        v.panel_fill = Color32::TRANSPARENT;
        v.window_fill = surface();
        v.window_stroke = Stroke::new(line, if look.native.is_some() { border() } else { border_hi() });
        // Fluent's flyout and dialog elevation, softened: one blurred shape each.
        let shade = |alpha: u8| Color32::from_black_alpha(if look.light { alpha / 2 } else { alpha });
        v.popup_shadow =
            if look.shadows { Shadow { offset: [0, 8], blur: 16, spread: 0, color: shade(70) } } else { Shadow::NONE };
        v.window_shadow =
            if look.shadows { Shadow { offset: [0, 16], blur: 32, spread: 0, color: shade(90) } } else { Shadow::NONE };
        v.window_corner_radius = overlay;
        v.menu_corner_radius = overlay;
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
        match look.fills {
            // Filled controls (Fluent, AppKit): a fill per state, a hairline
            // stroke, the accent only on the focused or pressed edge.
            Some(f) => {
                for (state, fill) in [(&mut w.inactive, f.rest), (&mut w.hovered, f.hover), (&mut w.active, f.pressed)]
                {
                    state.bg_fill = fill;
                    state.weak_bg_fill = fill;
                    state.bg_stroke = Stroke::new(1.0, border());
                }
                w.inactive.fg_stroke = Stroke::new(1.0, text());
                w.hovered.fg_stroke = Stroke::new(1.0, text());
                // Fluent: pressed text goes secondary.
                w.active.fg_stroke = Stroke::new(1.0, text_dim());
            }
            None => {
                w.inactive.bg_fill = surface();
                w.inactive.weak_bg_fill = surface();
                // Controls are outlined at 3:1 or better (WCAG non-text contrast);
                // decorative panel outlines keep the softer border.
                w.inactive.bg_stroke = Stroke::new(line, border_hi());
                w.inactive.fg_stroke = Stroke::new(1.0, text());
                w.hovered.bg_fill = surface_hi();
                w.hovered.weak_bg_fill = surface_hi();
                w.hovered.bg_stroke = Stroke::new(line, text_dim());
                w.hovered.fg_stroke = Stroke::new(1.0, strong_fg);
                w.active.bg_fill = pressed;
                w.active.weak_bg_fill = pressed;
                w.active.bg_stroke = Stroke::new(line, accent());
                w.active.fg_stroke = Stroke::new(1.0, text_dim());
            }
        }
        w.open = w.hovered;
        // Disabled controls fade to about 40%, a tier of their own (Fluent
        // uses 36%), so they no longer look like faint hint text.
        v.disabled_alpha = 0.4;
        for s in [&mut w.noninteractive, &mut w.inactive, &mut w.hovered, &mut w.active, &mut w.open] {
            s.corner_radius = control;
            s.expansion = 0.0;
        }
    });
    // With square corners, anti-aliasing feathers only the odd diagonal
    // (checkmarks, arrows); text is pre-antialiased. Rounded looks need it.
    let rounded = look.radius > 0 || look.overlay_radius > 0;
    ctx.tessellation_options_mut(|o| o.feathering = rounded);
}

/// The colour the framebuffer is cleared to (panels are transparent).
/// Clear (transparent) where the window shows a system material: Mica
/// then fills every pixel the app leaves unpainted.
pub fn clear_color(_visuals: &egui::Visuals) -> [f32; 4] {
    if !tuned() {
        return Color32::from_rgba_unmultiplied(12, 12, 12, 180).to_normalized_gamma_f32();
    }
    if material() {
        return [0.0; 4];
    }
    bg().to_normalized_gamma_f32()
}

/// A system material shows through: the window has one and the look
/// allows it.
pub fn material() -> bool {
    MATERIAL.load(Ordering::Relaxed) && current().translucent
}

/// Corner radius for controls and list rows in the current look.
pub fn radius() -> f32 {
    current().radius as f32
}

/// Corner radius for overlays and cards in the current look.
pub fn overlay_radius() -> f32 {
    current().overlay_radius as f32
}
