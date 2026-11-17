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
        ui.painter().rect_filled(band, 0.0, accent());
    }
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui.label(RichText::new("SPIRE").strong().color(accent()));
        ui.label(RichText::new("DASHBOARD").color(text_dim()));
        ui.add_space(16.0);

        if let Some(c) = s.character.as_ref().and_then(|c| v.catalog.characters.get(c)) {
            let accent = style::character_color(c.id.as_str());
            ui.label(RichText::new(c.name.trim_start_matches("The ")).heading().color(accent));
            let asc = v.catalog.ascensions.get(&s.ascension).map_or(String::new(), |a| a.name.clone());
            ui.label(RichText::new(format!("A{}", s.ascension)).strong().color(if s.ascension > 0 {
                crate::style::color::accent()
            } else {
                text_dim()
            }))
            .on_hover_text(asc);
            ui.add_space(18.0);

            let act = s.act.as_ref().and_then(|a| v.catalog.acts.get(a));
            stat(ui, &format!("Act {}", s.act_index + 1), act.map_or("Not started", |a| a.name.as_str()), text());
            ui.add_space(10.0);
            stat(ui, "Floor", s.floor.to_string(), text());
            ui.add_space(10.0);
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 2.0;
                ui.label(RichText::new("HP").small().color(text_faint()));
                hp_bar(ui, s.hp, s.max_hp, 140.0);
            });
            ui.add_space(10.0);
            stat(ui, "Gold", s.gold.to_string(), crate::style::color::accent());
            ui.add_space(10.0);
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 4.0;
                ui.label(RichText::new("POTIONS").small().color(text_faint()));
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 3.0;
                    slots(ui, s.potions.len(), s.potion_slots);
                });
            });
            ui.add_space(10.0);
            stat(ui, "Deck", s.deck.len().to_string(), text());
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let data_label = if st.updating { "Updating data…" } else { "Settings" };
            if ui.button(data_label).clicked() {
                out.push(Action::ToggleDataWindow);
            }
            if s.started() {
                if ui
                    .add_enabled(v.can_undo, egui::Button::new("Undo"))
                    .on_hover_text("Undo the last logged step")
                    .clicked()
                {
                    out.push(Action::Undo);
                }
                let in_progress = s.outcome.is_none();
                if ui.button("New run").clicked() {
                    if in_progress {
                        st.confirm_abandon = true;
                    } else {
                        out.push(Action::AbandonRun);
                    }
                }
                if in_progress
                    && ui.button("End run").on_hover_text("Record how the run ended and file it in history").clicked()
                {
                    st.end_run_open = true;
                    st.end_encounter = s.encounters_seen.last().cloned();
                }
            }
            if let Some(version) = v.catalog.layers.last().and_then(|l| l.game_version.as_ref()) {
                ui.label(RichText::new(version).small().color(text_faint()));
            }
        });
    });
    match st.messages.last() {
        Some(msg) => {
            ui.label(RichText::new(msg).small().color(text_faint()));
        }
        None => ui.add_space(4.0),
    }
}
