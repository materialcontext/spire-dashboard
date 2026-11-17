//! Small reusable widgets.

use eframe::egui::{self, Color32, RichText, Ui};
use spire_core::analysis::Coverage;
use spire_core::playbook::{Opportunity, contains_ci};

/// A search box with a short list of matches. Returns the picked id.
pub fn search_pick<'a>(
    ui: &mut Ui,
    query: &mut String,
    hint: &str,
    items: impl Iterator<Item = (&'a str, &'a str, &'a str)>,
) -> Option<String> {
    ui.add(egui::TextEdit::singleline(query).hint_text(hint).desired_width(f32::INFINITY));
    let q = query.trim();
    if q.is_empty() {
        return None;
    }
    let mut picked = None;
    // Allocation-free match; stops scanning after the eighth hit.
    let matches = items.filter(|(_, name, _)| contains_ci(name, q)).take(8);
    for (id, name, detail) in matches {
        if ui.selectable_label(false, name).on_hover_text(detail).clicked() {
            picked = Some(id.to_string());
        }
    }
    if picked.is_some() {
        query.clear();
    }
    picked
}

pub fn coverage_color(ui: &Ui, c: Coverage) -> Color32 {
    match c {
        Coverage::Missing => ui.visuals().error_fg_color,
        Coverage::Thin => ui.visuals().warn_fg_color,
        Coverage::Covered => Color32::from_rgb(90, 170, 100),
    }
}

/// A small outlined label. Painted directly (one galley, one rect) rather
/// than as a `Frame` with a child `Ui`: chips are the most numerous widget.
pub fn chip(ui: &mut Ui, text: impl Into<String>, color: Color32) -> egui::Response {
    const PAD: egui::Vec2 = egui::vec2(5.0, 1.0);
    let font = egui::TextStyle::Small.resolve(ui.style());
    let galley = ui.painter().layout_no_wrap(text.into(), font, color);
    let (rect, response) = ui.allocate_exact_size(galley.size() + 2.0 * PAD, egui::Sense::hover());
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        painter.rect_stroke(rect, 0.0, egui::Stroke::new(1.0, color), egui::StrokeKind::Inside);
        painter.galley(rect.min + PAD, galley, color);
    }
    response
}

pub fn opportunity_color(ui: &Ui, o: Opportunity) -> Color32 {
    if o.is_risk() { ui.visuals().warn_fg_color } else { ui.visuals().hyperlink_color }
}

/// A labelled horizontal bar: `value` out of `max`, with an optional target tick.
pub fn bar(ui: &mut Ui, label: &str, value: u32, extra: u32, max: u32, target: Option<u32>, color: Color32) {
    ui.horizontal(|ui| {
        ui.add_sized([150.0, 16.0], egui::Label::new(label).truncate());
        let width = (ui.available_width() - 70.0).max(60.0);
        let (rect, _) = ui.allocate_exact_size(egui::vec2(width, 14.0), egui::Sense::hover());
        let painter = ui.painter_at(rect);
        let max = max.max(1) as f32;
        painter.rect_filled(rect, 0.0, ui.visuals().extreme_bg_color);
        let frac = |n: u32| (n as f32 / max).min(1.0) * rect.width();
        let main = egui::Rect::from_min_size(rect.min, egui::vec2(frac(value), rect.height()));
        painter.rect_filled(main, 0.0, color);
        if extra > 0 {
            let starter = egui::Rect::from_min_size(
                egui::pos2(main.max.x, rect.min.y),
                egui::vec2(frac(value + extra) - frac(value), rect.height()),
            );
            painter.rect_filled(starter, 0.0, color.gamma_multiply(0.35));
        }
        if let Some(t) = target {
            let x = rect.min.x + frac(t);
            painter.line_segment(
                [egui::pos2(x, rect.min.y - 2.0), egui::pos2(x, rect.max.y + 2.0)],
                egui::Stroke::new(2.0, ui.visuals().strong_text_color()),
            );
        }
        let text = match (extra, target) {
            (0, Some(t)) => format!("{value}/{t}"),
            (0, None) => value.to_string(),
            (e, Some(t)) => format!("{value}+{e}/{t}"),
            (e, None) => format!("{value}+{e}"),
        };
        ui.label(text);
    });
}

/// A titled panel section.
pub fn section<R>(ui: &mut Ui, title: &str, add: impl FnOnce(&mut Ui) -> R) -> R {
    egui::Frame::group(ui.style())
        .inner_margin(egui::Margin::same(10))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new(title).heading().strong());
            ui.add_space(4.0);
            add(ui)
        })
        .inner
}

/// Lays out `add` only when it is (or may be) visible. Content scrolled out
/// of view reserves the size it had when last laid out instead of being laid
/// out again: egui otherwise builds every widget in a scroll area each frame
/// and only discards the off-screen ones at paint time.
///
/// `key` must identify the content and change whenever its size may change
/// (pass the dashboard generation); the available width is added here.
pub fn culled<R>(
    ui: &mut Ui,
    key: impl std::hash::Hash + std::fmt::Debug,
    add: impl FnOnce(&mut Ui) -> R,
) -> Option<R> {
    let id = egui::Id::unique(("culled", key, ui.available_width() as i32));
    if let Some(size) = ui.ctx().data(|d| d.get_temp::<egui::Vec2>(id)) {
        let rect = egui::Rect::from_min_size(ui.cursor().min, size);
        if !ui.is_rect_visible(rect) {
            ui.allocate_space(size);
            return None;
        }
    }
    let shown = ui.scope(add);
    ui.ctx().data_mut(|d| d.insert_temp(id, shown.response.rect.size()));
    Some(shown.inner)
}
