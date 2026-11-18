//! What the operating system says about the person's preferences and its
//! own look, and the few native things the app asks of it.
//!
//! Read on demand (startup, a light/dark switch, the window regaining
//! focus), never per frame. Light and dark mode also come from eframe
//! (`Context::system_theme`), which updates live.
//!
//! - Windows: WinRT `UISettings` (text size, animations, accent and its
//!   shades, transparency effects) and `AccessibilitySettings` (contrast
//!   themes and their colours).
//! - macOS: `NSWorkspace` accessibility settings (reduce motion and
//!   transparency, increase contrast) and `NSColor`'s semantic colours,
//!   resolved for the current appearance.
//! - Linux: defaults (no portable setting to read).
//!
//! This crate depends on no UI crate (colours are plain RGBA), so it can be
//! checked on its own for each target (`cargo clippy -p spire-platform
//! --target …`).
//!
//! It is the one crate in the workspace allowed `unsafe`, and only in the
//! native window plumbing that has no safe API: `window::mac` (the sidebar
//! material behind egui's view), `window::win` (the window subclass for
//! snap layouts) and the context-menu call in `menu`. Each such module is
//! marked `#[allow(unsafe_code)]`, every block carries a `// SAFETY:`
//! comment, and the crate denies `unsafe` everywhere else.

mod fonts;
pub mod menu;
pub mod window;

pub use fonts::{SystemFont, system_font};

/// A colour as unmultiplied RGBA.
pub type Rgba = [u8; 4];

/// The person's system-wide preferences that the looks follow.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SystemPrefs {
    /// Text size multiplier (1.0 = 100%).
    pub text_scale: f32,
    /// The person asked for less motion.
    pub reduce_motion: bool,
}

impl Default for SystemPrefs {
    fn default() -> Self {
        SystemPrefs { text_scale: 1.0, reduce_motion: false }
    }
}

/// Reads the preferences now.
pub fn prefs() -> SystemPrefs {
    imp::prefs()
}

/// The system accent and its shades (Windows reports three lighter and
/// three darker; macOS one colour).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Accent {
    pub base: Rgba,
    /// Light 1–3, lightest last.
    pub light: [Rgba; 3],
    /// Dark 1–3, darkest last.
    pub dark: [Rgba; 3],
    /// macOS "Multicolor": the app may use its own accent.
    pub multicolor: bool,
}

/// A Windows contrast theme's colours.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Contrast {
    pub window: Rgba,
    pub window_text: Rgba,
    pub highlight: Rgba,
    pub highlight_text: Rgba,
    pub gray_text: Rgba,
    pub hotlight: Rgba,
    pub button_face: Rgba,
    pub button_text: Rgba,
}

/// macOS semantic colours, resolved for the current appearance (with
/// their alpha, as the system defines them).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MacColors {
    pub window_background: Rgba,
    pub control_background: Rgba,
    pub text_background: Rgba,
    pub label: Rgba,
    pub secondary_label: Rgba,
    pub tertiary_label: Rgba,
    pub separator: Rgba,
    pub unemphasized_selection: Rgba,
    pub green: Rgba,
    pub orange: Rgba,
    pub red: Rgba,
    pub blue: Rgba,
}

/// What the system's own look is right now.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Appearance {
    /// Dark mode, if the system says.
    pub dark: Option<bool>,
    pub accent: Option<Accent>,
    /// A Windows contrast theme is on.
    pub contrast: Option<Contrast>,
    /// macOS Increase Contrast.
    pub increase_contrast: bool,
    /// Transparency effects are allowed (Windows "Transparency effects",
    /// macOS not "Reduce transparency").
    pub transparency: bool,
    pub mac: Option<MacColors>,
}

/// Reads the system's look now.
pub fn appearance() -> Appearance {
    imp::appearance()
}

#[cfg(windows)]
mod imp {
    use windows::UI::ViewManagement::{AccessibilitySettings, UIColorType, UIElementType, UISettings};

    use super::{Accent, Appearance, Contrast, Rgba};

    /// Through WinRT's `UISettings`, which is safe to call (no `unsafe`,
    /// which this workspace forbids). Any failure falls back to defaults.
    pub fn prefs() -> super::SystemPrefs {
        let Ok(ui) = UISettings::new() else { return super::SystemPrefs::default() };
        let text_scale = ui.TextScaleFactor().map_or(1.0, |f| f.clamp(1.0, 2.25) as f32);
        let reduce_motion = ui.AnimationsEnabled().is_ok_and(|on| !on);
        super::SystemPrefs { text_scale, reduce_motion }
    }

    fn rgba(c: windows::UI::Color) -> Rgba {
        [c.R, c.G, c.B, c.A]
    }

    pub fn appearance() -> Appearance {
        let Ok(ui) = UISettings::new() else { return Appearance { transparency: true, ..Appearance::default() } };
        let color = |t: UIColorType| ui.GetColorValue(t).ok().map(rgba);
        // The documented way to tell dark mode: a light foreground.
        let dark = color(UIColorType::Foreground).map(|[r, g, b, _]| (r as u32 + g as u32 + b as u32) > 3 * 128);
        let accent = match (
            color(UIColorType::Accent),
            [color(UIColorType::AccentLight1), color(UIColorType::AccentLight2), color(UIColorType::AccentLight3)],
            [color(UIColorType::AccentDark1), color(UIColorType::AccentDark2), color(UIColorType::AccentDark3)],
        ) {
            (Some(base), [Some(l1), Some(l2), Some(l3)], [Some(d1), Some(d2), Some(d3)]) => {
                Some(Accent { base, light: [l1, l2, l3], dark: [d1, d2, d3], multicolor: false })
            }
            _ => None,
        };
        let high = AccessibilitySettings::new().and_then(|a| a.HighContrast()).unwrap_or(false);
        let element = |t: UIElementType| ui.UIElementColor(t).ok().map(rgba);
        let contrast = if high {
            match (
                element(UIElementType::Window),
                element(UIElementType::WindowText),
                element(UIElementType::Highlight),
                element(UIElementType::HighlightText),
                element(UIElementType::GrayText),
                element(UIElementType::Hotlight),
                element(UIElementType::ButtonFace),
                element(UIElementType::ButtonText),
            ) {
                (Some(w), Some(wt), Some(h), Some(ht), Some(g), Some(hl), Some(bf), Some(bt)) => Some(Contrast {
                    window: w,
                    window_text: wt,
                    highlight: h,
                    highlight_text: ht,
                    gray_text: g,
                    hotlight: hl,
                    button_face: bf,
                    button_text: bt,
                }),
                _ => None,
            }
        } else {
            None
        };
        let transparency = ui.AdvancedEffectsEnabled().unwrap_or(true);
        Appearance { dark, accent, contrast, increase_contrast: false, transparency, mac: None }
    }
}

#[cfg(target_os = "macos")]
mod imp {
    use objc2_app_kit::{NSColor, NSColorSpace, NSWorkspace};
    use objc2_foundation::{NSUserDefaults, ns_string};

    use super::{Accent, Appearance, MacColors, Rgba};

    pub fn prefs() -> super::SystemPrefs {
        let workspace = NSWorkspace::sharedWorkspace();
        super::SystemPrefs { text_scale: 1.0, reduce_motion: workspace.accessibilityDisplayShouldReduceMotion() }
    }

    /// A semantic colour resolved to sRGB for the current appearance.
    fn rgba(c: &NSColor) -> Option<Rgba> {
        let c = c.colorUsingColorSpace(&NSColorSpace::sRGBColorSpace())?;
        let byte = |v: f64| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
        Some([byte(c.redComponent()), byte(c.greenComponent()), byte(c.blueComponent()), byte(c.alphaComponent())])
    }

    pub fn appearance() -> Appearance {
        let workspace = NSWorkspace::sharedWorkspace();
        let colors = (|| {
            Some(MacColors {
                window_background: rgba(&NSColor::windowBackgroundColor())?,
                control_background: rgba(&NSColor::controlBackgroundColor())?,
                text_background: rgba(&NSColor::textBackgroundColor())?,
                label: rgba(&NSColor::labelColor())?,
                secondary_label: rgba(&NSColor::secondaryLabelColor())?,
                tertiary_label: rgba(&NSColor::tertiaryLabelColor())?,
                separator: rgba(&NSColor::separatorColor())?,
                unemphasized_selection: rgba(&NSColor::unemphasizedSelectedContentBackgroundColor())?,
                green: rgba(&NSColor::systemGreenColor())?,
                orange: rgba(&NSColor::systemOrangeColor())?,
                red: rgba(&NSColor::systemRedColor())?,
                blue: rgba(&NSColor::systemBlueColor())?,
            })
        })();
        // Dark mode: the label colour is light.
        let dark = colors.map(|c| {
            let [r, g, b, _] = c.label;
            (r as u32 + g as u32 + b as u32) > 3 * 128
        });
        // "Multicolor" leaves AppleAccentColor unset.
        let multicolor = NSUserDefaults::standardUserDefaults().stringForKey(ns_string!("AppleAccentColor")).is_none();
        let accent = rgba(&NSColor::controlAccentColor()).map(|base| Accent {
            base,
            light: [base; 3],
            dark: [base; 3],
            multicolor,
        });
        Appearance {
            dark,
            accent,
            contrast: None,
            increase_contrast: workspace.accessibilityDisplayShouldIncreaseContrast(),
            transparency: !workspace.accessibilityDisplayShouldReduceTransparency(),
            mac: colors,
        }
    }
}

#[cfg(not(any(windows, target_os = "macos")))]
mod imp {
    pub fn prefs() -> super::SystemPrefs {
        super::SystemPrefs::default()
    }

    pub fn appearance() -> super::Appearance {
        super::Appearance { transparency: true, ..super::Appearance::default() }
    }
}
