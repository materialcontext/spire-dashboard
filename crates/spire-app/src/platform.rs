//! What the operating system says about the person's preferences.
//!
//! Read once at startup (cheap system calls, never per frame). Light and
//! dark mode come from eframe instead (`Context::system_theme`), which
//! updates live.
//!
//! - Windows: Settings > Accessibility > Text size and "Animation effects",
//!   from WinRT `UISettings` (`TextScaleFactor`, `AnimationsEnabled`).
//! - macOS: Reduce Motion (`NSWorkspace.accessibilityDisplayShouldReduceMotion`).
//!   macOS has no system text size for apps.
//! - Linux: defaults (no portable setting to read).

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

#[cfg(windows)]
mod imp {
    use windows::UI::ViewManagement::UISettings;

    /// Through WinRT's `UISettings`, which is safe to call (no `unsafe`,
    /// which this workspace forbids). Any failure falls back to defaults.
    pub fn prefs() -> super::SystemPrefs {
        let Ok(ui) = UISettings::new() else { return super::SystemPrefs::default() };
        let text_scale = ui.TextScaleFactor().map_or(1.0, |f| f.clamp(1.0, 2.25) as f32);
        let reduce_motion = ui.AnimationsEnabled().is_ok_and(|on| !on);
        super::SystemPrefs { text_scale, reduce_motion }
    }
}

#[cfg(target_os = "macos")]
mod imp {
    pub fn prefs() -> super::SystemPrefs {
        let workspace = objc2_app_kit::NSWorkspace::sharedWorkspace();
        super::SystemPrefs { text_scale: 1.0, reduce_motion: workspace.accessibilityDisplayShouldReduceMotion() }
    }
}

#[cfg(not(any(windows, target_os = "macos")))]
mod imp {
    pub fn prefs() -> super::SystemPrefs {
        super::SystemPrefs::default()
    }
}
