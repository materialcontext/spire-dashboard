//! Native pieces of the main window: Windows 11's Mica, macOS's sidebar
//! material, and snap layouts on an app-drawn Windows title bar.
//!
//! Each takes the window through `raw-window-handle`, which eframe's
//! `CreationContext` and `Frame` implement.

use raw_window_handle::HasWindowHandle;

/// Puts Windows 11's Mica behind the window (Windows 11 22H2 and later;
/// fails cleanly before), and sets the title bar's light or dark. The
/// window must be transparent for Mica to show through the content.
#[cfg_attr(not(windows), allow(unused_variables))]
pub fn apply_mica(window: &impl HasWindowHandle, dark: bool) -> bool {
    #[cfg(windows)]
    {
        window_vibrancy::apply_mica(window, Some(dark)).is_ok()
    }
    #[cfg(not(windows))]
    {
        false
    }
}

/// Puts macOS's sidebar material (`NSVisualEffectView`) behind the app's
/// content view, so the pixels the app leaves transparent show it. The
/// window must be transparent. Returns whether it was added.
#[cfg_attr(not(target_os = "macos"), allow(unused_variables))]
pub fn sidebar_material(window: &impl HasWindowHandle) -> bool {
    #[cfg(target_os = "macos")]
    {
        mac::sidebar_material(window)
    }
    #[cfg(not(target_os = "macos"))]
    {
        false
    }
}

pub use snap::{Snap, install_snap};

/// Snap layouts for an app-drawn title bar (Windows 11). Windows shows the
/// snap layout flyout when the pointer rests on whatever the window
/// reports as its maximise button (`HTMAXBUTTON`); an undecorated window
/// has none, so a window subclass reports the app's own button there.
mod snap {
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicBool, Ordering};

    use raw_window_handle::HasWindowHandle;

    /// The app's maximise button, in physical pixels from the top-left of
    /// the window's client area; `None` while it isn't drawn.
    pub(super) static BUTTON: Mutex<Option<[i32; 4]>> = Mutex::new(None);
    pub(super) static HOVER: AtomicBool = AtomicBool::new(false);
    pub(super) static PRESSED: AtomicBool = AtomicBool::new(false);
    /// Called when the hover or pressed state changes, to repaint.
    #[cfg_attr(not(windows), allow(dead_code))]
    pub(super) static WAKE: Mutex<Option<Box<dyn Fn() + Send>>> = Mutex::new(None);

    /// The snap-layout hookup for one window.
    pub struct Snap;

    impl Snap {
        /// Where the app drew its maximise button this frame (physical
        /// pixels, client area: left, top, right, bottom), or `None`.
        pub fn set_button(&self, rect: Option<[i32; 4]>) {
            if let Ok(mut b) = BUTTON.lock() {
                *b = rect;
            }
        }

        /// The pointer is over the maximise button (Windows routes it there,
        /// so the app sees no pointer of its own).
        pub fn hovered(&self) -> bool {
            HOVER.load(Ordering::Relaxed)
        }

        /// The maximise button is held down.
        pub fn pressed(&self) -> bool {
            PRESSED.load(Ordering::Relaxed)
        }
    }

    /// Hooks snap layouts into an undecorated window. `wake` repaints the
    /// app when the button's hover or pressed state changes. `None` off
    /// Windows or if the window can't be subclassed.
    #[cfg_attr(not(windows), allow(unused_variables))]
    pub fn install_snap(window: &impl HasWindowHandle, wake: impl Fn() + Send + 'static) -> Option<Snap> {
        #[cfg(windows)]
        {
            if let Ok(mut w) = WAKE.lock() {
                *w = Some(Box::new(wake));
            }
            super::win::subclass(window).then_some(Snap)
        }
        #[cfg(not(windows))]
        {
            None
        }
    }
}

#[cfg(target_os = "macos")]
#[allow(unsafe_code)]
mod mac {
    use objc2::rc::Retained;
    use objc2::{MainThreadMarker, MainThreadOnly};
    use objc2_app_kit::{
        NSAutoresizingMaskOptions, NSView, NSVisualEffectBlendingMode, NSVisualEffectMaterial, NSVisualEffectState,
        NSWindowOrderingMode,
    };
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};

    pub fn sidebar_material(window: &impl HasWindowHandle) -> bool {
        let Some(mtm) = MainThreadMarker::new() else { return false };
        let Ok(handle) = window.window_handle() else { return false };
        let RawWindowHandle::AppKit(appkit) = handle.as_raw() else { return false };
        // SAFETY: an AppKit window handle's `ns_view` is the window's live
        // `NSView` (winit's content view) for as long as the window exists,
        // and we're on the main thread (checked above). Retaining it keeps
        // it alive while we use it.
        let Some(view) = (unsafe { Retained::retain(appkit.ns_view.as_ptr().cast::<NSView>()) }) else {
            return false;
        };
        // The content view's superview is the window's frame view. The
        // material goes there, *below* the content view: a subview of the
        // content view itself would be drawn over egui's layer.
        // SAFETY: `superview` is a plain getter on a valid view, used on
        // the main thread; the frame view outlives this call.
        let Some(frame_view) = (unsafe { view.superview() }) else { return false };
        let effect = objc2_app_kit::NSVisualEffectView::initWithFrame(
            objc2_app_kit::NSVisualEffectView::alloc(mtm),
            frame_view.bounds(),
        );
        effect.setMaterial(NSVisualEffectMaterial::Sidebar);
        effect.setBlendingMode(NSVisualEffectBlendingMode::BehindWindow);
        effect.setState(NSVisualEffectState::FollowsWindowActiveState);
        effect.setAutoresizingMask(
            NSAutoresizingMaskOptions::ViewWidthSizable | NSAutoresizingMaskOptions::ViewHeightSizable,
        );
        frame_view.addSubview_positioned_relativeTo(&effect, NSWindowOrderingMode::Below, Some(&view));
        true
    }
}

#[cfg(windows)]
#[allow(unsafe_code)]
mod win {
    use std::sync::atomic::Ordering;

    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM};
    use windows::Win32::Graphics::Gdi::ScreenToClient;
    use windows::Win32::UI::Shell::{DefSubclassProc, SetWindowSubclass};
    use windows::Win32::UI::WindowsAndMessaging::{
        HTMAXBUTTON, IsZoomed, SW_MAXIMIZE, SW_RESTORE, ShowWindow, WM_MOUSEMOVE, WM_NCHITTEST, WM_NCLBUTTONDOWN,
        WM_NCLBUTTONUP, WM_NCMOUSELEAVE, WM_NCMOUSEMOVE,
    };

    use super::snap::{BUTTON, HOVER, PRESSED, WAKE};

    /// The subclass's id (any constant unique to this app).
    const ID: usize = 0x5350_4952; // "SPIR"

    pub fn subclass(window: &impl HasWindowHandle) -> bool {
        let Ok(handle) = window.window_handle() else { return false };
        let RawWindowHandle::Win32(w) = handle.as_raw() else { return false };
        let hwnd = HWND(w.hwnd.get() as *mut core::ffi::c_void);
        // SAFETY: `hwnd` is the live window eframe created, and this runs
        // on the thread that owns it (eframe's UI thread), as
        // SetWindowSubclass requires. `proc` matches SUBCLASSPROC and only
        // reads the shared state above; the subclass lives as long as the
        // window.
        unsafe { SetWindowSubclass(hwnd, Some(proc), ID, 0).as_bool() }
    }

    fn set(flag: &std::sync::atomic::AtomicBool, on: bool) {
        if flag.swap(on, Ordering::Relaxed) != on
            && let Ok(w) = WAKE.lock()
            && let Some(w) = w.as_ref()
        {
            w();
        }
    }

    /// Whether a screen point (from an NC message's lParam) is on the
    /// app's maximise button.
    fn on_button(hwnd: HWND, lparam: LPARAM) -> bool {
        let Ok(button) = BUTTON.lock() else { return false };
        let Some([l, t, r, b]) = *button else { return false };
        let raw = lparam.0 as u32;
        let mut p = POINT { x: (raw & 0xFFFF) as i16 as i32, y: (raw >> 16) as i16 as i32 };
        // SAFETY: `hwnd` is this subclass's own (live) window and `p` is a
        // valid, writable POINT.
        if !unsafe { ScreenToClient(hwnd, &mut p) }.as_bool() {
            return false;
        }
        p.x >= l && p.x < r && p.y >= t && p.y < b
    }

    unsafe extern "system" fn proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
        _id: usize,
        _data: usize,
    ) -> LRESULT {
        let max = HTMAXBUTTON as usize;
        match msg {
            WM_NCHITTEST if on_button(hwnd, lparam) => return LRESULT(max as isize),
            WM_NCMOUSEMOVE => set(&HOVER, wparam.0 == max),
            WM_NCMOUSELEAVE | WM_MOUSEMOVE => {
                set(&HOVER, false);
                set(&PRESSED, false);
            }
            // The app draws the button, so it handles the click: swallow the
            // press (no system maximise box to track) and act on release.
            WM_NCLBUTTONDOWN if wparam.0 == max => {
                set(&PRESSED, true);
                return LRESULT(0);
            }
            WM_NCLBUTTONUP if wparam.0 == max => {
                set(&PRESSED, false);
                // SAFETY: `hwnd` is this subclass's own live window.
                unsafe {
                    let cmd = if IsZoomed(hwnd).as_bool() { SW_RESTORE } else { SW_MAXIMIZE };
                    let _ = ShowWindow(hwnd, cmd);
                }
                return LRESULT(0);
            }
            _ => {}
        }
        // SAFETY: forwarding the message unchanged to the next procedure in
        // the chain, from within the subclass procedure, as required.
        unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) }
    }
}
