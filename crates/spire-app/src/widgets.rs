//! Reusable UI components. All paint with the palette in `style::color`.

use eframe::egui::{self, Color32, Rect, RichText, Sense, Stroke, StrokeKind, Ui, Vec2, vec2};
use spire_core::analysis::Coverage;
use spire_core::playbook::{Opportunity, contains_ci};

use crate::style::color::*;

/// The frame a panel is drawn in, by the look's panel style: outlined
/// (border, no fill: cheap on software renderers), a Fluent card
/// (translucent fill, hairline, rounded) or macOS grouped content (soft
/// fill, rounded, no outline).
pub fn panel_frame() -> egui::Frame {
    let look = crate::style::current();
    let margin = egui::Margin { left: 16, right: 16, top: 12, bottom: 12 };
    let frame = egui::Frame::new().inner_margin(margin);
    match look.panel {
        crate::style::PanelStyle::Outlined => {
            frame.stroke(Stroke::new(look.border, border())).corner_radius(look.overlay_radius)
        }
        crate::style::PanelStyle::Card => {
            frame.fill(look.card_fill).stroke(Stroke::new(1.0, look.card_stroke)).corner_radius(look.overlay_radius)
        }
        crate::style::PanelStyle::Grouped => frame.fill(look.card_fill).corner_radius(look.overlay_radius),
    }
}

/// A panel with a title, an optional right-side note, and a hairline under
/// the header, drawn in the look's panel style ([`panel_frame`]).
pub fn panel<R>(ui: &mut Ui, title: &str, note: Option<&str>, add: impl FnOnce(&mut Ui) -> R) -> R {
    let look = crate::style::current();
    let shown = panel_frame().show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            ui.label(eyebrow(title));
            if let Some(note) = note {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(caption(note));
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

/// A panel that asks for attention: a heavier outline and title in the
/// accent colour. For a step the player must take before anything else
/// (choosing the act or its boss). Still outlined, not filled.
pub fn callout<R>(ui: &mut Ui, title: &str, note: Option<&str>, add: impl FnOnce(&mut Ui) -> R) -> R {
    let look = crate::style::current();
    egui::Frame::new()
        .fill(look.card_fill)
        .corner_radius(look.overlay_radius)
        .stroke(Stroke::new(2.0, accent()))
        .inner_margin(egui::Margin { left: 16, right: 16, top: 12, bottom: 16 })
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(self::title(title).family(crate::style::strong_family()).color(accent()));
            if let Some(note) = note {
                ui.label(RichText::new(note).color(text_dim()));
            }
            ui.add_space(crate::style::space::s());
            add(ui)
        })
        .inner
}

/// A choice button for a callout: larger, outlined in the accent colour.
pub fn choice_button(ui: &mut Ui, label: impl Into<String>) -> egui::Response {
    let text = RichText::new(label.into()).color(text());
    ui.add(egui::Button::new(text).stroke(Stroke::new(1.0, accent())).min_size(vec2(0.0, 30.0)))
}

/// A page or pane title (Subtitle in the ramp), with the slight negative
/// tracking large sizes need.
pub fn title(text: impl Into<String>) -> RichText {
    let size = egui::TextStyle::Heading;
    RichText::new(text).text_style(size).extra_letter_spacing(-0.3).color(crate::style::color::text())
}

/// A label that links somewhere: pointer cursor and underline on hover.
pub fn link(ui: &mut Ui, text: RichText) -> egui::Response {
    let r = ui.add(egui::Label::new(text).sense(Sense::click()));
    if r.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        let y = r.rect.max.y - 1.0;
        ui.painter().hline(r.rect.x_range(), y, Stroke::new(1.0, text_faint()));
    }
    r
}

/// A [`link`] cut short with "…" past `max_width` (the full text shows on
/// hover), for rows with columns to their right.
pub fn link_within(ui: &mut Ui, text: RichText, max_width: f32) -> egui::Response {
    ui.scope(|ui| {
        ui.set_max_width(max_width.max(40.0));
        let r = ui.add(egui::Label::new(text).sense(Sense::click()).truncate());
        if r.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            ui.painter().hline(r.rect.x_range(), r.rect.max.y - 1.0, Stroke::new(1.0, text_faint()));
        }
        r
    })
    .inner
}

/// Small, dimmed text: secondary lines that still carry information.
pub fn secondary(text: impl Into<String>) -> RichText {
    RichText::new(text).small().color(text_dim())
}

/// Small, faint text: hints, notes and asides.
pub fn caption(text: impl Into<String>) -> RichText {
    RichText::new(text).small().color(text_faint())
}

/// Uppercase, small, dimmed label used for section titles.
pub fn eyebrow(text: &str) -> RichText {
    // Sentence-case Body Strong headings read faster than small capitals;
    // the Terminal look keeps capitals as part of its style.
    if crate::style::current().caps_headings {
        RichText::new(text.to_uppercase()).small().family(crate::style::strong_family()).color(text_dim())
    } else {
        RichText::new(text).family(crate::style::strong_family()).color(crate::style::color::text())
    }
}

/// A horizontal 1px rule across the available width.
pub fn hairline(ui: &mut Ui) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 1.0), Sense::hover());
    ui.painter().hline(rect.x_range(), rect.center().y, Stroke::new(1.0, border()));
    ui.add_space(crate::style::space::xs());
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
            painter.rect_filled(rect, crate::style::radius(), color.gamma_multiply(0.18));
        }
        let r = crate::style::radius();
        painter.rect_stroke(rect, r, Stroke::new(1.0, color.gamma_multiply(0.7)), StrokeKind::Inside);
        painter.galley(rect.min + PAD, galley, color);
    }
    response
}

/// A number with a small caption, for header stats ("FLOOR 12").
pub fn stat(ui: &mut Ui, title: &str, value: impl Into<String>, value_color: Color32) -> egui::Response {
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 0.0;
        let title = if crate::style::current().caps_headings { title.to_uppercase() } else { title.to_string() };
        ui.label(caption(title));
        ui.label(RichText::new(value).family(crate::style::strong_family()).color(value_color));
    })
    .response
}

/// One series key for the row of keys above a chart: a colour, a name and
/// whether its line is dashed. Kept outside the plot so it never covers data.
pub struct Key<'a> {
    pub color: Color32,
    pub name: &'a str,
    pub dashed: bool,
}

pub fn keys(ui: &mut Ui, keys: &[Key<'_>]) {
    ui.horizontal_wrapped(|ui| {
        for k in keys {
            let (rect, _) = ui.allocate_exact_size(vec2(18.0, 10.0), Sense::hover());
            let (a, b) = (egui::pos2(rect.left(), rect.center().y), egui::pos2(rect.right(), rect.center().y));
            let stroke = Stroke::new(2.0, k.color);
            if k.dashed {
                ui.painter().extend(egui::Shape::dashed_line(&[a, b], stroke, 4.0, 3.0));
            } else {
                ui.painter().line_segment([a, b], stroke);
            }
            ui.label(secondary(k.name));
            ui.add_space(crate::style::space::s());
        }
    });
}

/// A tiny painted line chart (no axes), with an optional dashed target.
/// Painted directly: five of these cost less than one plot widget.
pub fn sparkline(ui: &mut Ui, values: &[f32], target: Option<f32>, color: Color32, size: Vec2) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
    if !ui.is_rect_visible(rect) {
        return response;
    }
    let painter = ui.painter();
    painter.hline(rect.x_range(), rect.bottom(), Stroke::new(1.0, border()));
    let top = values.iter().copied().fold(target.unwrap_or(0.0), f32::max).max(1.0);
    let y = |v: f32| rect.bottom() - (v / top) * (rect.height() - 2.0);
    if let Some(t) = target {
        let dash = egui::Shape::dashed_line(
            &[egui::pos2(rect.left(), y(t)), egui::pos2(rect.right(), y(t))],
            Stroke::new(1.0, text_faint()),
            4.0,
            3.0,
        );
        painter.extend(dash);
    }
    if values.len() >= 2 {
        let step = rect.width() / (values.len() - 1) as f32;
        let points: Vec<egui::Pos2> =
            values.iter().enumerate().map(|(i, v)| egui::pos2(rect.left() + i as f32 * step, y(*v))).collect();
        painter.add(egui::Shape::line(points, Stroke::new(2.0, color)));
    }
    response
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
            let text = RichText::new(*label).color(if selected { on_accent() } else { text() });
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
    let button = egui::Button::new(label.color(if enabled { on_accent() } else { text_faint() }))
        .fill(if enabled { accent() } else { Color32::TRANSPARENT })
        .stroke(if enabled { Stroke::NONE } else { Stroke::new(1.0, border()) })
        .min_size(vec2(ui.available_width(), 32.0));
    ui.add_enabled(enabled, button)
}

/// A button for an action that can't be taken back: outlined and labelled
/// in the danger colour, never filled, so it never looks like the
/// recommended choice.
pub fn destructive_button(ui: &mut Ui, text: &str) -> egui::Response {
    let label = RichText::new(text).family(crate::style::strong_family()).color(bad());
    ui.add(
        egui::Button::new(label).fill(Color32::TRANSPARENT).stroke(Stroke::new(1.0, bad())).min_size(vec2(0.0, 32.0)),
    )
}

/// A standard dialog button.
pub fn dialog_button(ui: &mut Ui, label: &str) -> egui::Response {
    ui.add(egui::Button::new(RichText::new(label).color(text())).min_size(vec2(0.0, 32.0)))
}

/// Lays out a "do it" button and a safe button in the platform's order:
/// on macOS the safe (Cancel) button leads and the action trails; on
/// Windows and Linux the action leads and the safe button trails. Returns
/// (do it clicked, safe clicked).
pub fn dialog_row(
    ui: &mut Ui,
    action: impl FnOnce(&mut Ui) -> egui::Response,
    safe: impl FnOnce(&mut Ui) -> egui::Response,
) -> (bool, bool) {
    ui.horizontal(|ui| {
        if cfg!(target_os = "macos") {
            let s = safe(ui).clicked();
            (action(ui).clicked(), s)
        } else {
            let a = action(ui).clicked();
            (a, safe(ui).clicked())
        }
    })
    .inner
}

/// The keyboard shortcut prefix for this platform ("Ctrl+" or "Cmd+").
pub fn shortcut(key: &str) -> String {
    if cfg!(target_os = "macos") { format!("Cmd+{key}") } else { format!("Ctrl+{key}") }
}

/// A heading and content with no box around it: for sub-sections, so only
/// the few top-level groups carry outlines. Same margins as [`panel`], so
/// text lines up.
pub fn section<R>(ui: &mut Ui, title: &str, note: Option<&str>, add: impl FnOnce(&mut Ui) -> R) -> R {
    egui::Frame::new()
        .inner_margin(egui::Margin { left: 16, right: 16, top: 4, bottom: 4 })
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(eyebrow(title));
                if let Some(note) = note {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(caption(note));
                    });
                }
            });
            hairline(ui);
            add(ui)
        })
        .inner
}

/// A labelled horizontal bar: `value` out of `max`, a faded `extra`
/// segment after it, and an optional target tick.
/// Width of a [`bar`]'s label column, to line notes up under the bar.
pub const BAR_LABEL_WIDTH: f32 = 150.0;

pub fn bar(
    ui: &mut Ui,
    label: &str,
    value: u32,
    extra: u32,
    max: u32,
    target: Option<u32>,
    color: Color32,
) -> egui::Response {
    ui.horizontal(|ui| {
        ui.add_sized([BAR_LABEL_WIDTH, 18.0], egui::Label::new(RichText::new(label).color(text_dim())).truncate());
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
    })
    .response
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
/// Most results a search box lists; the list scrolls past a few rows.
const SEARCH_RESULTS: usize = 40;

/// Search box with a result list. Arrow keys move through the results
/// (the list scrolls to follow), Enter picks, and a click picks too. Focus
/// stays in the box after a pick, so several items can be added in a row.
pub fn search_pick<'a>(
    ui: &mut Ui,
    query: &mut String,
    hint: &str,
    items: impl Iterator<Item = (&'a str, &'a str, &'a str)>,
) -> Option<String> {
    let edit_id = egui::Id::unique(("search-pick", hint));
    let state_id = edit_id.with("highlight");
    let focused = ui.ctx().memory(|m| m.has_focus(edit_id));
    // Keys are taken before the text field sees them, and only while it has focus.
    let (down, up, enter) = if focused && !query.trim().is_empty() {
        ui.input_mut(|i| {
            (
                i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown),
                i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp),
                i.consume_key(egui::Modifiers::NONE, egui::Key::Enter),
            )
        })
    } else {
        (false, false, false)
    };
    ui.add(egui::TextEdit::singleline(query).id(edit_id).hint_text(hint).desired_width(f32::INFINITY));
    let q = query.trim().to_string();
    if q.is_empty() {
        return None;
    }
    let q = q.as_str();
    // (query, highlighted row): the highlight resets when the query changes.
    let (last, mut highlight): (String, usize) =
        ui.ctx().data(|d| d.get_temp(state_id)).unwrap_or_else(|| (String::new(), 0));
    if last != q {
        highlight = 0;
    }
    let matches: Vec<(&str, &str, &str)> =
        items.filter(|(_, name, _)| contains_ci(name, q)).take(SEARCH_RESULTS).collect();
    if !matches.is_empty() {
        if down {
            highlight = (highlight + 1).min(matches.len() - 1);
        }
        if up {
            highlight = highlight.saturating_sub(1);
        }
        highlight = highlight.min(matches.len() - 1);
    }
    let mut picked = None;
    egui::Frame::new().stroke(Stroke::new(1.0, border())).inner_margin(4).show(ui, |ui| {
        ui.set_width(ui.available_width());
        if matches.is_empty() {
            ui.label(caption("No matches"));
            return;
        }
        egui::ScrollArea::vertical().id_salt(state_id).max_height(220.0).show(ui, |ui| {
            for (i, (id, name, detail)) in matches.iter().enumerate() {
                let r = ui.selectable_label(i == highlight, *name).on_hover_text(*detail);
                if i == highlight && (down || up) {
                    r.scroll_to_me(Some(egui::Align::Center));
                }
                if r.clicked() {
                    picked = Some(id.to_string());
                }
            }
        });
    });
    if enter && let Some((id, _, _)) = matches.get(highlight) {
        picked = Some(id.to_string());
    }
    ui.ctx().data_mut(|d| d.insert_temp(state_id, (q.to_string(), highlight)));
    if picked.is_some() {
        query.clear();
        ui.ctx().memory_mut(|m| m.request_focus(edit_id));
    }
    picked
}

/// A dropdown whose list follows the arrow keys: Up and Down move the
/// highlight (the list scrolls to keep it in view), Enter picks it, and a
/// click picks too. Returns true when the value changed. `options` is
/// only called while the list is open, so closed dropdowns cost nothing.
pub fn combo_pick<T: Clone + PartialEq>(
    ui: &mut Ui,
    id_salt: &str,
    selected_text: impl Into<egui::WidgetText>,
    width: f32,
    value: &mut T,
    options: impl FnOnce() -> Vec<(T, String)>,
) -> bool {
    let state_id = egui::Id::unique(("combo-pick", id_salt));
    let before = value.clone();
    let shown = egui::ComboBox::from_id_salt(id_salt).selected_text(selected_text).width(width).height(280.0).show_ui(
        ui,
        |ui| {
            let options = options();
            if options.is_empty() {
                return 0;
            }
            // Opening starts the highlight on the current value.
            let mut h: usize = ui
                .ctx()
                .data(|d| d.get_temp(state_id))
                .unwrap_or_else(|| options.iter().position(|(v, _)| v == value).unwrap_or(0));
            let (down, up, enter) = ui.input_mut(|i| {
                (
                    i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown),
                    i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp),
                    i.consume_key(egui::Modifiers::NONE, egui::Key::Enter),
                )
            });
            if down {
                h = (h + 1).min(options.len() - 1);
            }
            if up {
                h = h.saturating_sub(1);
            }
            for (i, (v, label)) in options.iter().enumerate() {
                let r = ui.selectable_label(i == h, label);
                if i == h && (down || up) {
                    r.scroll_to_me(Some(egui::Align::Center));
                }
                if r.clicked() {
                    *value = v.clone();
                    ui.close();
                }
            }
            if enter {
                *value = options[h].0.clone();
                ui.close();
            }
            h
        },
    );
    match shown.inner {
        Some(h) => ui.ctx().data_mut(|d| {
            d.insert_temp(state_id, h);
        }),
        None => ui.ctx().data_mut(|d| d.remove::<usize>(state_id)),
    }
    *value != before
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

/// The backplate of a custom-painted list row, by platform: Fluent's
/// rounded hover fill with a 3×16 accent pill on the selected row; macOS's
/// rounded selection (rows don't light up on hover); flat fills elsewhere.
/// Also tells screen readers the row's name and whether it's selected.
pub fn row_backplate(ui: &Ui, resp: &egui::Response, name: &str, selected: bool) {
    resp.widget_info(|| egui::WidgetInfo::selected(egui::Role::ListBoxOption, true, selected, name));
    let rect = resp.rect;
    let painter = ui.painter();
    let hovered = resp.hovered();
    let r = crate::style::radius();
    match crate::style::current().native {
        Some(crate::style::Native::Windows) => {
            if selected || hovered {
                painter.rect_filled(rect.shrink2(vec2(0.0, 1.0)), r, surface_hi());
            }
            if selected {
                let pill = Rect::from_center_size(egui::pos2(rect.min.x + 7.0, rect.center().y), vec2(3.0, 16.0));
                painter.rect_filled(pill, 1.5, accent());
            }
        }
        Some(crate::style::Native::Mac) => {
            if selected {
                painter.rect_filled(rect.shrink2(vec2(0.0, 1.0)), r, surface_hi());
            }
        }
        None => {
            if selected || hovered {
                painter.rect_filled(rect, 0.0, if selected { surface_hi() } else { surface() });
            }
        }
    }
}

/// A modal dialog in the platform's style: Fluent's smoke behind it
/// (#4D000000), a lighter dim on macOS, egui's default elsewhere.
pub fn modal(id: &'static str) -> egui::Modal {
    let m = egui::Modal::new(egui::Id::unique(id));
    match crate::style::current().native {
        Some(crate::style::Native::Windows) => m.backdrop_color(Color32::from_black_alpha(0x4D)),
        Some(crate::style::Native::Mac) => m.backdrop_color(Color32::from_black_alpha(0x26)),
        None => m,
    }
}
