//! What the operating system says about the person's preferences.
//!
//! Read once at startup (cheap system calls, never per frame). Light and
//! dark mode come from eframe instead (`Context::system_theme`), which
//! updates live.
//!
//! - Windows: Settings > Accessibility > Text size (`TextScaleFactor`,
//!   100–225%), and "Animation effects" (`SPI_GETCLIENTAREAANIMATION`).
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
    use windows_sys::Win32::System::Registry::{HKEY_CURRENT_USER, RRF_RT_REG_DWORD, RegGetValueW};
    use windows_sys::Win32::UI::WindowsAndMessaging::{SPI_GETCLIENTAREAANIMATION, SystemParametersInfoW};

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    pub fn prefs() -> super::SystemPrefs {
        let mut scale: u32 = 100;
        let mut size = std::mem::size_of::<u32>() as u32;
        let key = wide("Software\\Microsoft\\Accessibility");
        let value = wide("TextScaleFactor");
        // SAFETY: valid null-terminated wide strings and an out-buffer of the declared size.
        let read = unsafe {
            RegGetValueW(
                HKEY_CURRENT_USER,
                key.as_ptr(),
                value.as_ptr(),
                RRF_RT_REG_DWORD,
                std::ptr::null_mut(),
                (&mut scale as *mut u32).cast(),
                &mut size,
            )
        };
        let text_scale = if read == 0 { (scale.clamp(100, 225) as f32) / 100.0 } else { 1.0 };
        let mut animate: i32 = 1;
        // SAFETY: SPI_GETCLIENTAREAANIMATION writes a BOOL to the pointer.
        let ok = unsafe { SystemParametersInfoW(SPI_GETCLIENTAREAANIMATION, 0, (&mut animate as *mut i32).cast(), 0) };
        super::SystemPrefs { text_scale, reduce_motion: ok != 0 && animate == 0 }
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
