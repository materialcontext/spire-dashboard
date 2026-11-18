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

use std::sync::atomic::{AtomicBool, Ordering};

use eframe::egui::{self, Color32, CursorIcon, Rect, ResizeDirection, Sense, Stroke, Ui, ViewportCommand, pos2, vec2};

use crate::style::color::*;

/// Whether the app draws its own title bar (the default).
static CUSTOM: AtomicBool = AtomicBool::new(true);

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
    if !custom() {
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
    if custom() {
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
    if !custom() {
        return;
    }
    let ctx = ui.ctx().clone();
    ui.spacing_mut().item_spacing.x = 0.0;
    if button(ui, Glyph::Close, "Close").clicked() {
        ctx.send_viewport_cmd(ViewportCommand::Close);
    }
    let max = maximized(&ctx);
    let (glyph, tip) = if max { (Glyph::Restore, "Restore") } else { (Glyph::Maximize, "Maximise") };
    if button(ui, glyph, tip).clicked() {
        ctx.send_viewport_cmd(ViewportCommand::Maximized(!max));
    }
    if button(ui, Glyph::Minimize, "Minimise").clicked() {
        ctx.send_viewport_cmd(ViewportCommand::Minimized(true));
    }
    ui.spacing_mut().item_spacing.x = 8.0;
}

fn button(ui: &mut Ui, glyph: Glyph, tip: &str) -> egui::Response {
    let (rect, r) = ui.allocate_exact_size(vec2(40.0, 30.0), Sense::click());
    let hovered = r.hovered();
    let painter = ui.painter();
    if hovered {
        let fill = if matches!(glyph, Glyph::Close) { Color32::from_rgb(196, 43, 28) } else { surface_hi() };
        painter.rect_filled(rect, 0.0, fill);
    }
    let color = match (glyph, hovered) {
        (Glyph::Close, true) => Color32::WHITE,
        (_, true) => text(),
        _ => text_dim(),
    };
    let stroke = Stroke::new(1.2, color);
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
