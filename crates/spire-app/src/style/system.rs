//! The System look: built at runtime from the platform's design tokens and
//! what the OS reports (accent, contrast theme, semantic colours).
//!
//! - **Windows 11:** Fluent's theme resources (WinUI `Common_themeresources`),
//!   values in docs/NATIVE_LOOK.md §4.3. Text and fills are translucent
//!   layers in Fluent; here the ones drawn as text are composited over the
//!   base colour, and the surfaces that sit on Mica stay translucent.
//! - **macOS:** `NSColor` semantic colours read for the current appearance,
//!   with Apple's published defaults as the fallback (used on other systems
//!   when previewing with `SPIRE_SYSTEM=macos`).
//!
//! The accent is the system's (decision D2). Data colours (card types,
//! roles, characters) stay as they are in every look.

use eframe::egui::Color32;

use super::{BASE, Fills, LookDef, Native, Palette, PanelStyle, Typeface};
use crate::platform::{Accent, Appearance, Contrast, MacColors, Rgba};

const fn argb(v: u32) -> Color32 {
    let [a, r, g, b] = v.to_be_bytes();
    Color32::from_rgba_unmultiplied_const(r, g, b, a)
}

fn rgba([r, g, b, a]: Rgba) -> Color32 {
    Color32::from_rgba_unmultiplied(r, g, b, a)
}

/// `top` (with its alpha) drawn over the opaque `base`.
pub fn over(base: Color32, top: Color32) -> Color32 {
    let a = top.a() as f32 / 255.0;
    let m = |b: u8, t: u8| (b as f32 + (t as f32 - b as f32) * a).round() as u8;
    Color32::from_rgb(m(base.r(), top.r()), m(base.g(), top.g()), m(base.b(), top.b()))
}

/// Windows 11's default accent shades, for when the OS doesn't report one.
const WIN_ACCENT: Accent = Accent {
    base: [0, 120, 212, 255],
    light: [[0, 145, 248, 255], [76, 194, 255, 255], [153, 235, 255, 255]],
    dark: [[0, 103, 192, 255], [0, 62, 146, 255], [0, 26, 104, 255]],
};

/// Builds the System look for `native` in light or dark mode.
pub fn look(native: Native, light: bool, a: &Appearance) -> LookDef {
    match native {
        Native::Windows => match a.contrast {
            Some(c) => windows_contrast(&c),
            None => windows(light, a),
        },
        Native::Mac => mac(light, a),
    }
}

fn windows(light: bool, a: &Appearance) -> LookDef {
    let accent = a.accent.unwrap_or(WIN_ACCENT);
    // SolidBackgroundFillColorBase: the window under Mica, and Mica's fallback.
    let base = if light { argb(0xFFF3F3F3) } else { argb(0xFF202020) };
    let text = |v_light: u32, v_dark: u32| over(base, argb(if light { v_light } else { v_dark }));
    let palette = Palette {
        bg: base,
        // Flyouts and dialogs: Acrylic's solid fallback.
        surface: if light { argb(0xFFF9F9F9) } else { argb(0xFF2C2C2C) },
        // Subtle hover backplate (SubtleFillColorSecondary) on the layer.
        surface_hi: text(0x0A000000, 0x15FFFFFF),
        // ControlFillColorInputActive.
        input: if light { argb(0xFFFFFFFF) } else { over(base, argb(0xB31E1E1E)) },
        // ControlStrokeColorDefault, and ControlStrongStrokeColorDefault.
        border: text(0x14000000, 0x18FFFFFF),
        border_hi: text(0x72000000, 0x8BFFFFFF),
        // TextFillColorPrimary, Secondary, Tertiary.
        text: text(0xE4000000, 0xFFFFFFFF),
        text_dim: text(0x9E000000, 0xC5FFFFFF),
        text_faint: text(0x72000000, 0x87FFFFFF),
        // Accent fills: AccentDark1 on light, AccentLight2 on dark.
        accent: rgba(if light { accent.dark[0] } else { accent.light[1] }),
        // SystemFillColorSuccess, Caution, Critical.
        good: if light { argb(0xFF0F7B0F) } else { argb(0xFF6CCB5F) },
        warn: if light { argb(0xFF9D5D00) } else { argb(0xFFFCE100) },
        bad: if light { argb(0xFFC42B1C) } else { argb(0xFFFF99A4) },
        // Accent text: AccentDark2 on light, AccentLight3 on dark.
        info: rgba(if light { accent.dark[1] } else { accent.light[2] }),
    };
    LookDef {
        name: super::SYSTEM_LOOK,
        label: "System",
        blurb: "Windows 11: Segoe UI, your accent colour, Mica and Fluent controls.",
        palette,
        typeface: Typeface::System,
        // Subtitle, Body, Caption.
        sizes: (20.0, 14.0, 12.0),
        light,
        native: Some(Native::Windows),
        radius: 4,
        overlay_radius: 8,
        control_height: 32.0,
        button_padding: (11.0, 5.0),
        panel: PanelStyle::Card,
        // CardBackgroundFillColorDefault and CardStrokeColorDefault.
        card_fill: if light { argb(0xB3FFFFFF) } else { argb(0x0DFFFFFF) },
        card_stroke: if light { argb(0x0F000000) } else { argb(0x19000000) },
        // LayerFillColorDefault: the content layer on Mica.
        layer: if light { argb(0x80FFFFFF) } else { argb(0x4C3A3A3A) },
        // TextOnAccentFillColorPrimary.
        on_accent: Some(if light { Color32::WHITE } else { Color32::BLACK }),
        // ControlFillColorDefault, Secondary (hover), Tertiary (pressed).
        fills: Some(Fills {
            rest: if light { argb(0xB3FFFFFF) } else { argb(0x0FFFFFFF) },
            hover: if light { argb(0x80F9F9F9) } else { argb(0x15FFFFFF) },
            pressed: if light { argb(0x4DF9F9F9) } else { argb(0x08FFFFFF) },
        }),
        shadows: true,
        translucent: a.transparency,
        ..BASE
    }
}

/// A Windows contrast theme: every colour from the theme, no materials,
/// translucency or shadows (Microsoft's contrast-theme guidance).
fn windows_contrast(c: &Contrast) -> LookDef {
    let window = rgba(c.window);
    let text = rgba(c.window_text);
    let light = (window.r() as u32 + window.g() as u32 + window.b() as u32) > 3 * 128;
    let palette = Palette {
        bg: window,
        surface: rgba(c.button_face),
        surface_hi: rgba(c.highlight),
        input: window,
        border: text,
        border_hi: rgba(c.button_text),
        text,
        text_dim: text,
        text_faint: text,
        accent: rgba(c.highlight),
        good: text,
        warn: text,
        bad: text,
        info: rgba(c.hotlight),
    };
    LookDef {
        name: super::SYSTEM_LOOK,
        label: "System",
        blurb: "Your Windows contrast theme.",
        palette,
        typeface: Typeface::System,
        sizes: (20.0, 14.0, 12.0),
        light,
        native: Some(Native::Windows),
        radius: 4,
        overlay_radius: 8,
        control_height: 32.0,
        button_padding: (11.0, 5.0),
        panel: PanelStyle::Outlined,
        on_accent: Some(rgba(c.highlight_text)),
        border: 1.0,
        ..BASE
    }
}

/// Apple's published defaults, for when the colours can't be read.
fn mac_defaults(light: bool) -> MacColors {
    if light {
        MacColors {
            window_background: [236, 236, 236, 255],
            text_background: [255, 255, 255, 255],
            label: [0, 0, 0, 217],
            secondary_label: [0, 0, 0, 128],
            tertiary_label: [0, 0, 0, 66],
            separator: [0, 0, 0, 26],
            unemphasized_selection: [220, 220, 220, 255],
            green: [40, 205, 65, 255],
            orange: [255, 149, 0, 255],
            red: [255, 59, 48, 255],
            blue: [0, 122, 255, 255],
        }
    } else {
        MacColors {
            window_background: [50, 50, 50, 255],
            text_background: [30, 30, 30, 255],
            label: [255, 255, 255, 217],
            secondary_label: [255, 255, 255, 140],
            tertiary_label: [255, 255, 255, 64],
            separator: [255, 255, 255, 26],
            unemphasized_selection: [70, 70, 70, 255],
            green: [50, 215, 75, 255],
            orange: [255, 159, 10, 255],
            red: [255, 69, 58, 255],
            blue: [10, 132, 255, 255],
        }
    }
}

fn mac(light: bool, a: &Appearance) -> LookDef {
    // The OS colours are for its own mode; use them only when it matches.
    let m = a.mac.filter(|_| a.dark.is_none_or(|d| d != light)).unwrap_or_else(|| mac_defaults(light));
    let bg = rgba(m.window_background);
    let on_bg = |c: Rgba| over(bg, rgba(c));
    let text = on_bg(m.label);
    let dim = on_bg(m.secondary_label);
    let accent = a.accent.map_or(rgba(m.blue), |x| rgba(x.base));
    let palette = Palette {
        bg,
        // Menus and popovers sit a little above the window.
        surface: if light { Color32::WHITE } else { over(bg, Color32::from_white_alpha(18)) },
        surface_hi: on_bg(m.unemphasized_selection),
        input: rgba(m.text_background),
        border: on_bg(m.separator),
        border_hi: on_bg(m.tertiary_label),
        text,
        text_dim: dim,
        // Tertiary labels are faint by design; captions here carry
        // information, so they sit between secondary and tertiary.
        text_faint: super::mix(dim, on_bg(m.tertiary_label), 0.5),
        accent,
        good: rgba(m.green),
        warn: rgba(m.orange),
        bad: rgba(m.red),
        info: rgba(m.blue),
    };
    let contrast = a.increase_contrast;
    LookDef {
        name: super::SYSTEM_LOOK,
        label: "System",
        blurb: "macOS: SF Pro, your accent colour, a sidebar and the menu bar.",
        palette,
        typeface: Typeface::System,
        // Title 3, Body, Subheadline.
        sizes: (15.0, 13.0, 11.0),
        light,
        native: Some(Native::Mac),
        radius: 6,
        overlay_radius: 10,
        control_height: 24.0,
        button_padding: (10.0, 3.0),
        // Grouped content: a soft fill, no outline (outlined only with
        // Increase Contrast).
        panel: if contrast { PanelStyle::Outlined } else { PanelStyle::Grouped },
        card_fill: if light { Color32::from_white_alpha(150) } else { Color32::from_white_alpha(12) },
        card_stroke: on_bg(m.separator),
        layer: bg,
        on_accent: Some(Color32::WHITE),
        fills: Some(Fills {
            rest: if light { Color32::WHITE } else { Color32::from_white_alpha(40) },
            hover: if light { argb(0xFFF6F6F6) } else { Color32::from_white_alpha(52) },
            pressed: if light { argb(0xFFE6E6E6) } else { Color32::from_white_alpha(64) },
        }),
        shadows: true,
        // The sidebar material shows where the app leaves the window clear.
        translucent: a.transparency,
        ..BASE
    }
}
