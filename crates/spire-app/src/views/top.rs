//! Header: identity, run vitals and global controls.

use eframe::egui::{self, RichText, Ui};

use crate::actions::Action;
use crate::app::{UiState, ViewCtx};
use crate::style::{self, color::*};
use crate::widgets::{caption, hp_bar, slots, stat};

pub fn show(ui: &mut Ui, v: &ViewCtx, st: &mut UiState, out: &mut Vec<Action>) {
    let s = v.state;
    if crate::style::current().character_accent && s.started() {
        // Spirebound: a band in the character's colour across the top.
        let r = ui.max_rect();
        let band = egui::Rect::from_min_size(egui::pos2(r.min.x - 14.0, r.min.y), egui::vec2(r.width() + 28.0, 3.0));
        ui.painter().rect_filled(band, 0.0, identity());
    }
    let top = ui.max_rect().top();
    crate::chrome::title_bar_area(ui);
    ui.add_space(crate::style::space::s());
    let native = style::current().native;
    let toolbar = native == Some(style::Native::Mac);
    // The row is as tall as the vitals (measured last frame): they're drawn
    // in a child of their own, so the row wouldn't grow for them, and they'd
    // spill under the panels when nothing else in it is as tall (the system
    // title bar takes the tall window buttons away).
    let row = ui.ctx().data(|d| d.get_temp::<f32>(row_height_id())).unwrap_or(0.0).max(ui.spacing().interact_size.y);
    let width = ui.available_width();
    ui.allocate_ui_with_layout(egui::vec2(width, row), egui::Layout::left_to_right(egui::Align::Center), |ui| {
        ui.set_min_height(row);
        let full = ui.max_rect();
        // macOS's unified title bar: the traffic lights sit at the left
        // (in the sidebar, with the System look).
        if crate::chrome::unified() && !toolbar {
            ui.add_space(crate::chrome::TRAFFIC_LIGHTS);
        }
        // On macOS the tabs live in the sidebar; elsewhere, here.
        if !toolbar {
            tabs(ui, st, native == Some(style::Native::Windows));
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            crate::chrome::window_buttons(ui);
            // Title-bar buttons are compact on the platform looks.
            if native.is_some() {
                ui.spacing_mut().button_padding.x = 8.0;
            }
            match native {
                // macOS: Settings is in the app menu (⌘,), not the toolbar.
                Some(style::Native::Mac) => {}
                // Windows: a navigation item that opens the Settings page.
                Some(style::Native::Windows) => {
                    let label = if st.updating { "Updating data…" } else { "Settings" };
                    if nav_item(ui, label, st.show_data).clicked() {
                        out.push(Action::ToggleDataWindow);
                    }
                }
                // Settings opens a window (ellipsis) and is a subtle button:
                // the run actions beside it matter more.
                None => {
                    let data_label = if st.updating { "Updating data…" } else { "Settings…" };
                    if ui.add(egui::Button::new(data_label).frame_when_inactive(false)).clicked() {
                        out.push(Action::ToggleDataWindow);
                    }
                }
            }
            if s.started() {
                if ui
                    .add_enabled(v.can_undo, egui::Button::new("Undo"))
                    .on_hover_text(format!("Undo the last logged step ({})", crate::widgets::shortcut("Z")))
                    .clicked()
                {
                    out.push(Action::Undo);
                }
                let in_progress = s.outcome.is_none();
                if ui.button(if in_progress { "New run…" } else { "New run" }).clicked() {
                    if in_progress {
                        st.confirm_abandon = true;
                    } else {
                        out.push(Action::AbandonRun);
                    }
                }
                if in_progress
                    && ui.button("End run…").on_hover_text("Record how the run ended and file it in history").clicked()
                {
                    st.end_run_open = true;
                    st.end_encounter = s.encounters_seen.last().cloned();
                }
            }
            // What's left between the tabs and the buttons holds the vitals.
            let free = ui.available_rect_before_wrap();
            centred_vitals(ui, v, full, free);
        });
    });
    // An inactive window dims its header, as Windows and macOS do, so it's
    // clear which window takes input. One translucent fill, header only.
    if !ui.ctx().input(|i| i.viewport().focused.unwrap_or(true)) {
        let rect = ui.min_rect().expand2(egui::vec2(16.0, 0.0));
        ui.painter().rect_filled(rect, 0.0, bg().gamma_multiply(0.45));
    }
    match st.messages.last() {
        Some(msg) => {
            ui.label(caption(msg));
        }
        None => ui.add_space(crate::style::space::xs()),
    }
    crate::chrome::title_bar_done(ui, top);
}

/// The tab strip. Windows' System look follows NavigationView's top mode:
/// text items with a 16×3 accent pill under the selected one. Other looks
/// underline the selected tab.
fn tabs(ui: &mut Ui, st: &mut UiState, fluent: bool) {
    let mut chosen = None;
    for (n, tab) in crate::views::Tab::ALL.into_iter().enumerate() {
        // On the Windows Settings page no tab is selected.
        let active = st.tab == tab && !(fluent && st.show_data);
        let r = if fluent {
            nav_item(ui, tab.label(), active)
        } else {
            let text = RichText::new(tab.label()).family(crate::style::strong_family());
            ui.add(
                egui::Button::new(text.color(if active { crate::style::color::text() } else { text_dim() }))
                    .frame(false),
            )
        };
        if active && !fluent {
            chosen = Some(r.rect);
        }
        let r = r.on_hover_text(format!("{} ({})", tab.label(), crate::widgets::shortcut(&(n + 1).to_string())));
        if r.clicked() {
            st.tab = tab;
            st.show_data = false;
        }
    }
    // The underline slides to a newly chosen tab (167 ms) when motion is on.
    if let Some(rect) = chosen {
        let (left, right) = if crate::style::motion() {
            let ctx = ui.ctx().clone();
            let id = egui::Id::unique("tab-underline");
            (
                ctx.animate_value_with_time(id.with("l"), rect.left(), crate::style::MOVE),
                ctx.animate_value_with_time(id.with("r"), rect.right(), crate::style::MOVE),
            )
        } else {
            (rect.left(), rect.right())
        };
        ui.painter().hline(left..=right, rect.max.y + 2.0, egui::Stroke::new(2.0, crate::style::color::accent()));
    }
}

/// A Fluent top navigation item: text on a subtle backplate when hovered,
/// a 16×3 accent pill under it when selected (sliding between items when
/// motion is on).
fn nav_item(ui: &mut Ui, label: &str, selected: bool) -> egui::Response {
    let font = egui::FontId::new(egui::TextStyle::Body.resolve(ui.style()).size, crate::style::strong_family());
    let galley = ui.painter().layout_no_wrap(label.to_string(), font, crate::style::color::text());
    let size = egui::vec2(galley.size().x + 20.0, 40.0);
    let (rect, r) = ui.allocate_exact_size(size, egui::Sense::click());
    r.widget_info(|| egui::WidgetInfo::selected(egui::Role::Tab, true, selected, label));
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        if r.hovered() || r.is_pointer_button_down_on() {
            let fill = if r.is_pointer_button_down_on() { surface_hi().gamma_multiply(0.6) } else { surface_hi() };
            painter.rect_filled(rect.shrink2(egui::vec2(2.0, 4.0)), style::radius(), fill);
        }
        let color = if r.is_pointer_button_down_on() { text_dim() } else { crate::style::color::text() };
        painter.galley(rect.center() - galley.size() / 2.0, galley, color);
    }
    if selected {
        let target = rect.center().x;
        let x = if crate::style::motion() {
            ui.ctx().animate_value_with_time(egui::Id::unique("nav-pill"), target, crate::style::MOVE)
        } else {
            target
        };
        let pill = egui::Rect::from_center_size(egui::pos2(x, rect.bottom() - 3.0), egui::vec2(16.0, 3.0));
        ui.painter().rect_filled(pill, 1.5, crate::style::color::accent());
    }
    r
}

/// Character, act, floor, HP, gold, potions and deck: the run at a glance.
fn vitals(ui: &mut Ui, v: &ViewCtx, tight: bool, minimal: bool) {
    let gap = if tight { 6.0 } else { 10.0 };
    let s = v.state;
    let Some(c) = s.character.as_ref().and_then(|c| v.catalog.characters.get(c)) else { return };
    let accent = style::character_color(c.id.as_str());
    ui.label(RichText::new(c.name.trim_start_matches("The ")).heading().color(accent));
    let asc = v.catalog.ascensions.get(&s.ascension).map_or(String::new(), |a| a.name.clone());
    ui.label(RichText::new(format!("A{}", s.ascension)).strong().color(if s.ascension > 0 {
        text()
    } else {
        text_dim()
    }))
    .on_hover_text(asc);
    ui.add_space(gap + 6.0);

    // Short of room, the act shows its number only and potions drop out
    // (both are in the panels below); the rest always shows.
    let act = s.act.as_ref().and_then(|a| v.catalog.acts.get(a));
    if minimal {
        stat(ui, "Act", (s.act_index + 1).to_string(), text())
            .on_hover_text(act.map_or("Not started", |a| a.name.as_str()));
    } else {
        stat(ui, &format!("Act {}", s.act_index + 1), act.map_or("Not started", |a| a.name.as_str()), text());
    }
    ui.add_space(gap);
    stat(ui, "Floor", s.floor.to_string(), text());
    ui.add_space(gap);
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 2.0;
        ui.label(caption("HP"));
        hp_bar(ui, s.hp, s.max_hp, if tight { 110.0 } else { 140.0 });
    });
    ui.add_space(gap);
    // Gold in gold: the thing's own colour, not the interactive accent.
    stat(ui, "Gold", s.gold.to_string(), style::gold());
    if !minimal {
        ui.add_space(gap);
        potions(ui, v);
    }
    ui.add_space(gap);
    stat(ui, "Deck", s.deck.len().to_string(), text());
}

fn potions(ui: &mut Ui, v: &ViewCtx) {
    let s = v.state;
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 4.0;
        let title = if style::current().caps_headings { "POTIONS" } else { "Potions" };
        ui.label(caption(title));
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 3.0;
            slots(ui, s.potions.len(), s.potion_slots);
        });
    });
}

fn row_height_id() -> egui::Id {
    egui::Id::unique("header-row-height")
}

/// Draws `vitals` centred on the whole header (`full`), kept clear of the
/// tabs and buttons (`free`). egui lays out in one pass, so the width is
/// measured each frame and used for the next; a change triggers one repaint.
fn centred_vitals(ui: &mut Ui, v: &ViewCtx, full: egui::Rect, free: egui::Rect) {
    let id = egui::Id::unique("header-vitals-width");
    let width: f32 = ui.ctx().data(|d| d.get_temp(id)).unwrap_or(free.width());
    let left = (full.center().x - width / 2.0).clamp(free.left(), (free.right() - width).max(free.left()));
    let rect = egui::Rect::from_min_max(egui::pos2(left, free.top()), egui::pos2(free.right(), free.bottom()));
    let mut child =
        ui.new_child(egui::UiBuilder::new().max_rect(rect).layout(egui::Layout::left_to_right(egui::Align::Center)));
    // Labels never wrap: a squeezed header would otherwise stack "Deck" letter by letter.
    child.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
    // Tighter spacing when the space between tabs and buttons is short, or
    // the vitals didn't fit last frame; never drawn over the buttons.
    let tight = free.width() < 700.0 || width > free.width();
    child.set_clip_rect(free.intersect(child.clip_rect()));
    child.spacing_mut().item_spacing.x = if tight { 6.0 } else { 8.0 };
    // Narrower still: drop what the panels below already show, once the
    // full set (measured when last shown) no longer fits.
    let full_id = egui::Id::unique("header-vitals-full-width");
    let full_width: f32 = ui.ctx().data(|d| d.get_temp(full_id)).unwrap_or(0.0);
    let minimal = full_width > free.width();
    vitals(&mut child, v, tight, minimal);
    let measured = child.min_rect().width();
    if !minimal && (measured - full_width).abs() > 0.5 {
        ui.ctx().data_mut(|d| d.insert_temp(full_id, measured));
    }
    if (measured - width).abs() > 0.5 {
        ui.ctx().data_mut(|d| d.insert_temp(id, measured));
        ui.ctx().request_repaint();
    }
    let height = child.min_rect().height();
    let row: f32 = ui.ctx().data(|d| d.get_temp(row_height_id())).unwrap_or(0.0);
    if (height - row).abs() > 0.5 {
        ui.ctx().data_mut(|d| d.insert_temp(row_height_id(), height));
        ui.ctx().request_repaint();
    }
}
