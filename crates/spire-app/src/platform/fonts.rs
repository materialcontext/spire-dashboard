//! The system UI typeface, read from the system's own font folder.
//!
//! Segoe UI Variable and SF Pro are licensed only as part of their OS, so
//! they are never bundled: the app reads the copy the OS already has, once,
//! and keeps it for the life of the process. Anything missing falls back to
//! the app's own Inter (the caller's choice).

use std::sync::OnceLock;

/// A system typeface and how to get its weights.
#[derive(Clone, Copy, Debug)]
pub enum SystemFont {
    /// One variable font: set the `wght` axis for each weight (and `opsz`
    /// where the font has it).
    Variable { data: &'static [u8], regular: f32, strong: f32, optical: Option<f32> },
    /// Separate regular and semibold files (Windows 10's Segoe UI).
    Static { regular: &'static [u8], strong: &'static [u8] },
}

#[cfg_attr(not(any(windows, target_os = "macos")), allow(dead_code))]
fn read(path: &std::path::Path) -> Option<&'static [u8]> {
    std::fs::read(path).ok().filter(|b| !b.is_empty()).map(|b| &*Box::leak(b.into_boxed_slice()))
}

/// The system UI font, read once. `None` off Windows and macOS, or when
/// the files aren't where the OS keeps them.
pub fn system_font() -> Option<SystemFont> {
    static FONT: OnceLock<Option<SystemFont>> = OnceLock::new();
    *FONT.get_or_init(load)
}

#[cfg(windows)]
fn load() -> Option<SystemFont> {
    let dir =
        std::path::PathBuf::from(std::env::var_os("WINDIR").unwrap_or_else(|| "C:\\Windows".into())).join("Fonts");
    // Windows 11: Segoe UI Variable, "Text" optical size (10.5) for UI text.
    if let Some(data) = read(&dir.join("SegUIVar.ttf")) {
        return Some(SystemFont::Variable { data, regular: 400.0, strong: 600.0, optical: Some(10.5) });
    }
    // Windows 10: Segoe UI and Segoe UI Semibold.
    Some(SystemFont::Static { regular: read(&dir.join("segoeui.ttf"))?, strong: read(&dir.join("seguisb.ttf"))? })
}

#[cfg(target_os = "macos")]
fn load() -> Option<SystemFont> {
    // SF Pro, the system font, as a variable font.
    let data = read(std::path::Path::new("/System/Library/Fonts/SFNS.ttf"))?;
    Some(SystemFont::Variable { data, regular: 400.0, strong: 590.0, optical: None })
}

#[cfg(not(any(windows, target_os = "macos")))]
fn load() -> Option<SystemFont> {
    None
}
