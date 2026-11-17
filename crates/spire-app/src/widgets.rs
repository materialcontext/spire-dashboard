//! Reusable UI components. All paint with the palette in `style::color`.

use eframe::egui::{self, Color32, Rect, RichText, Sense, Stroke, StrokeKind, Ui, Vec2, vec2};
use spire_core::analysis::Coverage;
use spire_core::playbook::{Opportunity, contains_ci};

use crate::style::color::*;

/// An outlined panel with an uppercase title, an optional right-side note,
/// and a hairline under the header. Outlined rather than filled: borders
/// cost a few pixels to draw, fills cost their whole area.
pub fn panel<R>(ui: &mut Ui, title: &str, note: Option<&str>, add: impl FnOnce(&mut Ui) -> R) -> R {
    let look = crate::style::current();
    let shown = egui::Frame::new()
        .stroke(Stroke::new(look.border, border()))
        .inner_margin(egui::Margin { left: 14, right: 14, top: 10, bottom: 12 })
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(eyebrow(title));
                if let Some(note) = note {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(RichText::new(note).small().color(text_faint()));
                    });
                }
            });
            hairline(ui);
            add(ui)
        });
    if look.glow {
        // One faint outer outline: reads as a glow for a few hundred pixels.
        let rect = shown.response.rect.expand(2.0);
        ui.painter().rect_stroke(rect, 0.0, Stroke::new(2.0, border_hi().gamma_multiply(0.35)), StrokeKind::Outside);
    }
    shown.inner
}

/// Uppercase, small, dimmed label used for section titles.
pub fn eyebrow(text: &str) -> RichText {
    RichText::new(text.to_uppercase()).small().family(crate::style::strong_family()).color(text_dim())
}

/// A horizontal 1px rule across the available width.
pub fn hairline(ui: &mut Ui) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 1.0), Sense::hover());
    ui.painter().hline(rect.x_range(), rect.center().y, Stroke::new(1.0, border()));
    ui.add_space(4.0);
}

/// A small outlined label. Painted directly (one galley, one rect) rather
/// than as a `Frame` with a child `Ui`: chips are the most numerous widget.
pub fn chip(ui: &mut Ui, text: impl Into<String>, color: Color32) -> egui::Response {
    const PAD: Vec2 = vec2(6.0, 2.0);
    let font = egui::TextStyle::Small.resolve(ui.style());
    let galley = ui.painter().layout_no_wrap(text.into(), font, color);
    let (rect, response) = ui.allocate_exact_size(galley.size() + 2.0 * PAD, Sense::hover());
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        if crate::style::tinted_chips() {
            painter.rect_filled(rect, 0.0, color.gamma_multiply(0.18));
        }
        painter.rect_stroke(rect, 0.0, Stroke::new(1.0, color.gamma_multiply(0.7)), StrokeKind::Inside);
        painter.galley(rect.min + PAD, galley, color);
    }
    response
}

/// A number with a small caption, for header stats ("FLOOR 12").
pub fn stat(ui: &mut Ui, caption: &str, value: impl Into<String>, value_color: Color32) {
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 0.0;
        ui.label(RichText::new(caption.to_uppercase()).small().color(text_faint()));
        ui.label(RichText::new(value).family(crate::style::strong_family()).color(value_color));
    });
}

/// HP bar coloured by health, with the numbers inside.
pub fn hp_bar(ui: &mut Ui, hp: i32, max: i32, width: f32) {
    let fraction = if max > 0 { (hp as f32 / max as f32).clamp(0.0, 1.0) } else { 0.0 };
    let (rect, _) = ui.allocate_exact_size(vec2(width, 20.0), Sense::hover());
    let painter = ui.painter();
    painter.rect_stroke(rect, 0.0, Stroke::new(1.0, border_hi()), StrokeKind::Inside);
    let fill = Rect::from_min_size(rect.min + vec2(2.0, 2.0), vec2((width - 4.0) * fraction, rect.height() - 4.0));
    painter.rect_filled(fill, 0.0, crate::style::hp_color(fraction).gamma_multiply(0.75));
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        format!("{hp} / {max}"),
        egui::TextStyle::Small.resolve(ui.style()),
        text(),
    );
}

/// Potion slots as squares: filled for each potion held.
pub fn slots(ui: &mut Ui, held: usize, total: u32) {
    let size = 10.0;
    for i in 0..total as usize {
        let (rect, _) = ui.allocate_exact_size(vec2(size, size), Sense::hover());
        if i < held {
            ui.painter().rect_filled(rect, 0.0, info());
        } else {
            ui.painter().rect_stroke(rect, 0.0, Stroke::new(1.0, border_hi()), StrokeKind::Inside);
        }
    }
}

/// A row of mutually exclusive options; returns true when changed.
pub fn segmented<T: PartialEq + Clone>(ui: &mut Ui, value: &mut T, options: &[(T, &str)]) -> bool {
    let mut changed = false;
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        for (opt, label) in options {
            let selected = *value == *opt;
            let text = RichText::new(*label).color(if selected { bg() } else { text() });
            let button = egui::Button::new(text)
                .fill(if selected { accent() } else { Color32::TRANSPARENT })
                .stroke(Stroke::new(1.0, if selected { accent() } else { border() }));
            if ui.add(button).clicked() && !selected {
                *value = opt.clone();
                changed = true;
            }
        }
    });
    changed
}

/// The single most important action in a panel.
pub fn primary_button(ui: &mut Ui, text: &str, enabled: bool) -> egui::Response {
    let label = RichText::new(text).family(crate::style::strong_family());
    let button = egui::Button::new(label.color(if enabled { bg() } else { text_faint() }))
        .fill(if enabled { accent() } else { Color32::TRANSPARENT })
        .stroke(if enabled { Stroke::NONE } else { Stroke::new(1.0, border()) })
        .min_size(vec2(ui.available_width(), 32.0));
    ui.add_enabled(enabled, button)
}

/// A labelled horizontal bar: `value` out of `max`, a faded `extra`
/// segment after it, and an optional target tick.
pub fn bar(ui: &mut Ui, label: &str, value: u32, extra: u32, max: u32, target: Option<u32>, color: Color32) {
    ui.horizontal(|ui| {
        ui.add_sized([150.0, 18.0], egui::Label::new(RichText::new(label).color(text_dim())).truncate());
        let width = (ui.available_width() - 64.0).max(60.0);
        let (rect, _) = ui.allocate_exact_size(vec2(width, 10.0), Sense::hover());
        let painter = ui.painter_at(rect.expand(3.0));
        let max = max.max(1) as f32;
        painter.rect_filled(rect, 0.0, surface());
        let frac = |n: u32| (n as f32 / max).min(1.0) * rect.width();
        let main = Rect::from_min_size(rect.min, vec2(frac(value), rect.height()));
        painter.rect_filled(main, 0.0, color);
        if extra > 0 {
            let faded = Rect::from_min_size(
                egui::pos2(main.max.x, rect.min.y),
                vec2(frac(value + extra) - frac(value), rect.height()),
            );
            painter.rect_filled(faded, 0.0, color.gamma_multiply(0.3));
        }
        if let Some(t) = target {
            let x = rect.min.x + frac(t);
            painter.vline(x, (rect.min.y - 3.0)..=(rect.max.y + 3.0), Stroke::new(2.0, text()));
        }
        let text = match (extra, target) {
            (0, Some(t)) => format!("{value} / {t}"),
            (0, None) => value.to_string(),
            (e, Some(t)) => format!("{value} +{e} / {t}"),
            (e, None) => format!("{value} +{e}"),
        };
        let met = target.is_none_or(|t| value >= t);
        ui.label(RichText::new(text).small().color(if met { crate::style::color::text() } else { warn() }));
    });
}

/// A thin coloured mark, e.g. the card-type stripe on deck rows.
pub fn stripe(ui: &mut Ui, color: Color32, height: f32) {
    let (rect, _) = ui.allocate_exact_size(vec2(3.0, height), Sense::hover());
    ui.painter().rect_filled(rect, 0.0, color);
}

pub fn coverage_color(c: Coverage) -> Color32 {
    match c {
        Coverage::Missing => bad(),
        Coverage::Thin => warn(),
        Coverage::Covered => good(),
    }
}

pub fn opportunity_color(o: Opportunity) -> Color32 {
    if o.is_risk() { warn() } else { info() }
}

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
    egui::Frame::new().stroke(Stroke::new(1.0, border())).inner_margin(4).show(ui, |ui| {
        ui.set_width(ui.available_width());
        let mut any = false;
        for (id, name, detail) in matches {
            any = true;
            if ui.selectable_label(false, name).on_hover_text(detail).clicked() {
                picked = Some(id.to_string());
            }
        }
        if !any {
            ui.label(RichText::new("No matches").small().color(text_faint()));
        }
    });
    if picked.is_some() {
        query.clear();
    }
    picked
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
    if let Some(size) = ui.ctx().data(|d| d.get_temp::<Vec2>(id)) {
        let rect = Rect::from_min_size(ui.cursor().min, size);
        if !ui.is_rect_visible(rect) {
            ui.allocate_space(size);
            return None;
        }
    }
    let shown = ui.scope(add);
    ui.ctx().data_mut(|d| d.insert_temp(id, shown.response.rect.size()));
    Some(shown.inner)
}
