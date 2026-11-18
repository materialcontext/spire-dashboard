//! Header: identity, run vitals and global controls.

use eframe::egui::{self, RichText, Ui};

use crate::actions::Action;
use crate::app::{UiState, ViewCtx};
use crate::style::{self, color::*};
use crate::widgets::{hp_bar, slots, stat};

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
    ui.horizontal(|ui| {
        let full = ui.max_rect();
        // Tabs: the active one is underlined in the accent colour.
        let mut underline = None;
        for (n, tab) in crate::views::Tab::ALL.into_iter().enumerate() {
            let active = st.tab == tab;
            let text = RichText::new(tab.label()).family(crate::style::strong_family());
            let r = ui.add(
                egui::Button::new(text.color(if active { crate::style::color::text() } else { text_dim() }))
                    .frame(false),
            );
            if active {
                underline = Some(r.rect);
            }
            let r = r.on_hover_text(format!("{} ({})", tab.label(), crate::widgets::shortcut(&(n + 1).to_string())));
            if r.clicked() {
                st.tab = tab;
            }
        }
        // The underline slides to a newly chosen tab (167 ms) when motion is on.
        if let Some(rect) = underline {
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
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            crate::chrome::window_buttons(ui);
            // Settings opens a window (ellipsis) and is a subtle button: the
            // run actions beside it matter more.
            let data_label = if st.updating { "Updating data…" } else { "Settings…" };
            if ui.add(egui::Button::new(data_label).frame_when_inactive(false)).clicked() {
                out.push(Action::ToggleDataWindow);
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
            ui.label(RichText::new(msg).small().color(text_faint()));
        }
        None => ui.add_space(crate::style::space::xs()),
    }
    crate::chrome::title_bar_done(ui, top);
}

/// Character, act, floor, HP, gold, potions and deck: the run at a glance.
fn vitals(ui: &mut Ui, v: &ViewCtx, tight: bool) {
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

    let act = s.act.as_ref().and_then(|a| v.catalog.acts.get(a));
    stat(ui, &format!("Act {}", s.act_index + 1), act.map_or("Not started", |a| a.name.as_str()), text());
    ui.add_space(gap);
    stat(ui, "Floor", s.floor.to_string(), text());
    ui.add_space(gap);
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 2.0;
        ui.label(RichText::new("HP").small().color(text_faint()));
        hp_bar(ui, s.hp, s.max_hp, if tight { 110.0 } else { 140.0 });
    });
    ui.add_space(gap);
    // Gold in gold: the thing's own colour, not the interactive accent.
    stat(ui, "Gold", s.gold.to_string(), style::gold());
    ui.add_space(gap);
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 4.0;
        let caption = if style::current().caps_headings { "POTIONS" } else { "Potions" };
        ui.label(RichText::new(caption).small().color(text_faint()));
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 3.0;
            slots(ui, s.potions.len(), s.potion_slots);
        });
    });
    ui.add_space(gap);
    stat(ui, "Deck", s.deck.len().to_string(), text());
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
    // Tighter spacing when the space between tabs and buttons is short.
    let tight = free.width() < 700.0;
    child.spacing_mut().item_spacing.x = if tight { 6.0 } else { 8.0 };
    vitals(&mut child, v, tight);
    let measured = child.min_rect().width();
    if (measured - width).abs() > 0.5 {
        ui.ctx().data_mut(|d| d.insert_temp(id, measured));
        ui.ctx().request_repaint();
    }
}
