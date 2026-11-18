//! Window chrome drawn by the app.
//!
//! The header row doubles as the title bar, and the system frame is turned
//! off. Desktop themes and tools that restyle native title bars (Mica For
//! Everyone, blur themes) then have nothing of ours to paint over.
//!
//! - Drag the header to move; double-click it to maximise or restore.
//! - Window buttons sit at the right of the header.
//! - Drag any window edge or corner to resize.
//!
//! Settings can bring back the system title bar.
//!
//! On macOS with the System look the window keeps its real title bar,
//! made transparent over full-size content ("unified"): the traffic
//! lights stay where the system puts them and the app's toolbar row sits
//! beside them. On Windows the System look can put Mica behind the window.

use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};

use eframe::egui::{self, Color32, CursorIcon, Rect, ResizeDirection, Sense, Stroke, Ui, ViewportCommand, pos2, vec2};

use crate::style::color::*;

/// Whether the app draws its own title bar (the default).
static CUSTOM: AtomicBool = AtomicBool::new(true);
/// macOS unified title bar: the system's traffic lights over our content.
static UNIFIED: AtomicBool = AtomicBool::new(false);
/// The light/dark the window material was last set for (0 unset, 1 light, 2 dark).
static MATERIAL_MODE: AtomicU8 = AtomicU8::new(0);

/// Width the macOS traffic lights take at the left of the title bar,
/// with their margin.
pub const TRAFFIC_LIGHTS: f32 = 78.0;

/// The window keeps macOS's title bar, transparent over the content.
pub fn unified() -> bool {
    UNIFIED.load(Ordering::Relaxed)
}

/// Sets up the macOS unified title bar (startup only; see `main.rs`).
pub fn init_unified(on: bool) {
    UNIFIED.store(on, Ordering::Relaxed);
    if on {
        CUSTOM.store(false, Ordering::Relaxed);
    }
}

/// Puts Windows 11's Mica behind the window, matching light or dark. Call
/// at startup and whenever the mode changes; cheap when nothing changed.
/// The window must have been created transparent. Records whether it
/// worked, so the app clears to transparent only then.
pub fn material(window: Window, dark: bool) {
    let mode = if dark { 2 } else { 1 };
    if MATERIAL_MODE.swap(mode, Ordering::Relaxed) != mode {
        apply_mica(window, dark);
    }
}

/// Sets the system backdrop (Mica, Windows 11 22H2 and later) and the dark
/// title bar; fails cleanly on older Windows and does nothing elsewhere.
fn apply_mica(window: Window, dark: bool) {
    if !cfg!(windows) {
        return;
    }
    let ok = match window {
        Window::Creation(cc) => crate::platform::window::apply_mica(cc, dark),
        Window::Frame(f) => crate::platform::window::apply_mica(f, dark),
    };
    crate::style::set_material(ok);
}

/// macOS: puts the sidebar material behind the window's content (the
/// window must have been created transparent). Startup only.
pub fn sidebar_material(cc: &eframe::CreationContext) {
    if crate::platform::window::sidebar_material(cc) {
        crate::style::set_material(true);
    }
}

/// Windows: snap layouts on the app-drawn maximise button.
static SNAP: std::sync::OnceLock<crate::platform::window::Snap> = std::sync::OnceLock::new();

/// Hooks up snap layouts for the app-drawn title bar (Windows 11). Startup
/// only; harmless while the system title bar is in use.
pub fn install_snap(cc: &eframe::CreationContext) {
    let ctx = cc.egui_ctx.clone();
    if let Some(snap) = crate::platform::window::install_snap(cc, move || ctx.request_repaint()) {
        let _ = SNAP.set(snap);
    }
}

/// The native window, as eframe hands it over at startup or each frame.
pub enum Window<'a> {
    Creation(&'a eframe::CreationContext<'a>),
    Frame(&'a eframe::Frame),
}

/// Whether the window should be created transparent so Mica can show:
/// the System look on Windows 11, in the system frame, with transparency
/// effects on and no contrast theme. Decided before the window exists.
///
/// Only with the system frame: Mica behind an undecorated window isn't
/// documented, and if it didn't take, the transparent window would show
/// the desktop through it. The app-drawn title bar gets Mica's own solid
/// fallback colour instead, as Windows shows when Mica is off.
pub fn wants_material(system_look: bool, system_frame: bool, a: &crate::platform::Appearance) -> bool {
    cfg!(windows) && system_look && system_frame && a.transparency && a.contrast.is_none()
}

/// Pixels along each window edge that start a resize.
const EDGE: f32 = 5.0;

pub fn custom() -> bool {
    CUSTOM.load(Ordering::Relaxed)
}

/// Sets the startup choice; the window is created to match (see `main.rs`).
pub fn init(custom: bool) {
    CUSTOM.store(custom, Ordering::Relaxed);
}

/// Switches between the app's title bar and the system one, live.
pub fn set_custom(ctx: &egui::Context, on: bool) {
    CUSTOM.store(on, Ordering::Relaxed);
    ctx.send_viewport_cmd(ViewportCommand::Decorations(!on));
}

fn maximized(ctx: &egui::Context) -> bool {
    ctx.input(|i| i.viewport().maximized.unwrap_or(false))
}

/// Makes the header draggable. Call before anything else in the header:
/// widgets added later sit on top, so tabs and buttons keep their clicks.
/// The header's height is measured each frame and used for the next.
pub fn title_bar_area(ui: &mut Ui) {
    if !custom() && !unified() {
        return;
    }
    let id = egui::Id::unique("title-bar-height");
    let height: f32 = ui.ctx().data(|d| d.get_temp(id)).unwrap_or(48.0);
    let screen = ui.ctx().content_rect();
    let top = ui.max_rect().top();
    let rect = Rect::from_min_max(pos2(screen.left(), top), pos2(screen.right(), top + height));
    let r = ui.interact(rect, egui::Id::unique("title-bar-drag"), Sense::click_and_drag());
    if r.double_clicked() {
        let max = maximized(ui.ctx());
        ui.ctx().send_viewport_cmd(ViewportCommand::Maximized(!max));
    } else if r.drag_started_by(egui::PointerButton::Primary) {
        ui.ctx().send_viewport_cmd(ViewportCommand::StartDrag);
    }
}

/// Records the header's height for [`title_bar_area`] on the next frame.
pub fn title_bar_done(ui: &Ui, top: f32) {
    if custom() || unified() {
        let height = ui.min_rect().bottom() - top;
        ui.ctx().data_mut(|d| d.insert_temp(egui::Id::unique("title-bar-height"), height));
    }
}

#[derive(Clone, Copy)]
enum Glyph {
    Minimize,
    Maximize,
    Restore,
    Close,
}

/// Minimise, maximise/restore and close, right to left. Call inside a
/// right-to-left layout. Icons are painted, so they never depend on fonts.
pub fn window_buttons(ui: &mut Ui) {
    let snap = SNAP.get();
    if !custom() {
        if let Some(snap) = snap {
            snap.set_button(None);
        }
        return;
    }
    let ctx = ui.ctx().clone();
    ui.spacing_mut().item_spacing.x = 0.0;
    if button(ui, Glyph::Close, "Close", None).clicked() {
        ctx.send_viewport_cmd(ViewportCommand::Close);
    }
    let max = maximized(&ctx);
    let (glyph, tip) = if max { (Glyph::Restore, "Restore") } else { (Glyph::Maximize, "Maximise") };
    let r = button(ui, glyph, tip, snap);
    match snap {
        // Windows routes the pointer on this button to the system (that's
        // what brings up snap layouts) and maximises on click itself.
        Some(snap) => {
            let ppp = ctx.pixels_per_point();
            let px = |v: f32| (v * ppp).round() as i32;
            let rect = r.rect;
            snap.set_button(Some([px(rect.left()), px(rect.top()), px(rect.right()), px(rect.bottom())]));
        }
        None => {
            if r.clicked() {
                ctx.send_viewport_cmd(ViewportCommand::Maximized(!max));
            }
        }
    }
    if button(ui, Glyph::Minimize, "Minimise", None).clicked() {
        ctx.send_viewport_cmd(ViewportCommand::Minimized(true));
    }
    ui.spacing_mut().item_spacing.x = 8.0;
}

/// One caption button. `snap` is set for the maximise button when Windows
/// handles its pointer (snap layouts): its hover and press come from there.
fn button(ui: &mut Ui, glyph: Glyph, tip: &str, snap: Option<&crate::platform::window::Snap>) -> egui::Response {
    // Windows 11 caption buttons are 46 wide; other looks keep them compact.
    let fluent = crate::style::current().native == Some(crate::style::Native::Windows);
    let size = if fluent { vec2(46.0, 32.0) } else { vec2(40.0, 30.0) };
    let (rect, r) = ui.allocate_exact_size(size, Sense::click());
    r.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, tip));
    let hovered = r.hovered() || snap.is_some_and(|s| s.hovered());
    let down = r.is_pointer_button_down_on() || snap.is_some_and(|s| s.pressed());
    let painter = ui.painter();
    if hovered {
        // Close turns red (Fluent's #C42B1C); the others take the subtle
        // hover fill, pressed one step further.
        let fill = match (glyph, down) {
            (Glyph::Close, false) => Color32::from_rgb(196, 43, 28),
            (Glyph::Close, true) => Color32::from_rgb(200, 64, 49),
            (_, false) => surface_hi(),
            (_, true) => border(),
        };
        painter.rect_filled(rect, 0.0, fill);
    }
    let color = match (glyph, hovered) {
        (Glyph::Close, true) => Color32::WHITE,
        (_, true) => text(),
        _ => text_dim(),
    };
    let stroke = Stroke::new(if fluent { 1.0 } else { 1.2 }, color);
    let c = rect.center();
    let h = 5.0;
    match glyph {
        Glyph::Minimize => {
            painter.hline(c.x - h..=c.x + h, c.y, stroke);
        }
        Glyph::Maximize => {
            let sq = Rect::from_center_size(c, vec2(2.0 * h, 2.0 * h));
            painter.rect_stroke(sq, 0.0, stroke, egui::StrokeKind::Inside);
        }
        Glyph::Restore => {
            let back = Rect::from_center_size(c + vec2(2.0, -2.0), vec2(8.0, 8.0));
            let front = Rect::from_center_size(c + vec2(-1.0, 1.0), vec2(8.0, 8.0));
            painter.rect_stroke(back, 0.0, stroke, egui::StrokeKind::Inside);
            painter.rect_filled(front, 0.0, if hovered { surface_hi() } else { bg() });
            painter.rect_stroke(front, 0.0, stroke, egui::StrokeKind::Inside);
        }
        Glyph::Close => {
            painter.line_segment([c + vec2(-h, -h), c + vec2(h, h)], stroke);
            painter.line_segment([c + vec2(-h, h), c + vec2(h, -h)], stroke);
        }
    }
    r.on_hover_text(tip)
}

/// Draws a ring around the focused widget, but only while the keyboard is
/// driving (after Tab or an arrow key, until the next click), as Windows
/// and macOS do: 2px outer ring in the text colour, 1px inner ring in the
/// background colour, 2px clear of the widget.
pub fn focus_ring(ctx: &egui::Context) {
    let id = egui::Id::unique("keyboard-driving");
    let (keys, clicked) = ctx.input(|i| {
        let keys = i.events.iter().any(|e| {
            matches!(
                e,
                egui::Event::Key {
                    key: egui::Key::Tab
                        | egui::Key::ArrowDown
                        | egui::Key::ArrowUp
                        | egui::Key::ArrowLeft
                        | egui::Key::ArrowRight,
                    pressed: true,
                    ..
                }
            )
        });
        (keys, i.pointer.any_pressed())
    });
    let mut driving: bool = ctx.data(|d| d.get_temp(id)).unwrap_or(false);
    if keys {
        driving = true;
    }
    if clicked {
        driving = false;
    }
    ctx.data_mut(|d| d.insert_temp(id, driving));
    if !driving {
        return;
    }
    let Some(focused) = ctx.memory(|m| m.focused()) else { return };
    let Some(r) = ctx.read_response(focused) else { return };
    let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Tooltip, egui::Id::unique("focus-ring")));
    let outer = r.rect.expand(2.0);
    painter.rect_stroke(outer, 0.0, Stroke::new(2.0, text()), egui::StrokeKind::Outside);
    painter.rect_stroke(outer, 0.0, Stroke::new(1.0, bg()), egui::StrokeKind::Inside);
}

/// Edge and corner resizing, and a hairline round the window so its edge
/// shows against the desktop. Call after the views, so the resize cursor
/// wins over whatever widget sits under the edge.
pub fn window_edges(ctx: &egui::Context) {
    if !custom() || maximized(ctx) || ctx.input(|i| i.viewport().fullscreen.unwrap_or(false)) {
        return;
    }
    let screen = ctx.content_rect();
    ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::unique("window-border"))).rect_stroke(
        screen,
        0.0,
        Stroke::new(1.0, border_hi()),
        egui::StrokeKind::Inside,
    );
    let Some(p) = ctx.input(|i| i.pointer.hover_pos()) else { return };
    let (w, e) = (p.x - screen.left() < EDGE, screen.right() - p.x < EDGE);
    let (n, s) = (p.y - screen.top() < EDGE, screen.bottom() - p.y < EDGE);
    let (dir, icon) = match (n, s, w, e) {
        (true, _, true, _) => (ResizeDirection::NorthWest, CursorIcon::ResizeNorthWest),
        (true, _, _, true) => (ResizeDirection::NorthEast, CursorIcon::ResizeNorthEast),
        (_, true, true, _) => (ResizeDirection::SouthWest, CursorIcon::ResizeSouthWest),
        (_, true, _, true) => (ResizeDirection::SouthEast, CursorIcon::ResizeSouthEast),
        (true, ..) => (ResizeDirection::North, CursorIcon::ResizeNorth),
        (_, true, ..) => (ResizeDirection::South, CursorIcon::ResizeSouth),
        (_, _, true, _) => (ResizeDirection::West, CursorIcon::ResizeWest),
        (_, _, _, true) => (ResizeDirection::East, CursorIcon::ResizeEast),
        _ => return,
    };
    ctx.set_cursor_icon(icon);
    if ctx.input(|i| i.pointer.primary_pressed()) {
        ctx.send_viewport_cmd(ViewportCommand::BeginResize(dir));
    }
}
