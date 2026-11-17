//! Starting a run and entering an act.

use eframe::egui::{self, RichText, Ui};

use crate::actions::Action;
use crate::app::{UiState, ViewCtx};
use crate::widgets::section;

pub fn show(ui: &mut Ui, v: &ViewCtx, st: &mut UiState, out: &mut Vec<Action>) {
    ui.add_space(24.0);
    ui.vertical_centered(|ui| {
        ui.set_max_width(520.0);
        section(ui, "New run", |ui| {
            ui.label("Pick a character. After each node, log what changed on the right.");
            ui.add_space(6.0);
            for c in v.catalog.characters.values() {
                let picked = st.new_character.as_ref() == Some(&c.id);
                let text = format!("{}  ({} HP, {} cards)", c.name, c.max_hp, c.starting_deck.len());
                if ui.selectable_label(picked, text).on_hover_text(&c.description).clicked() {
                    st.new_character = Some(c.id.clone());
                }
            }
            ui.horizontal(|ui| {
                ui.label("Ascension");
                ui.add(egui::DragValue::new(&mut st.new_ascension).range(0..=20));
            });
            ui.add_space(6.0);
            let ready = st.new_character.is_some();
            if ui.add_enabled(ready, egui::Button::new("Start run")).clicked()
                && let Some(character) = st.new_character.clone()
            {
                out.push(Action::NewRun { character, ascension: st.new_ascension });
            }
        });
    });
}

/// Act picker: shown when the run has no current act (start, or after a boss).
pub fn enter_act(ui: &mut Ui, v: &ViewCtx, out: &mut Vec<Action>) {
    let s = v.state;
    let next = if s.nodes.is_empty() { 0 } else { s.act_index + 1 };
    act_picker(ui, v, next, out);
}

pub fn act_picker(ui: &mut Ui, v: &ViewCtx, index: u8, out: &mut Vec<Action>) {
    let acts: Vec<_> = v.catalog.acts_at(index).collect();
    section(ui, &format!("Enter Act {}", index + 1), |ui| {
        if acts.is_empty() {
            ui.label("No act data for this act. Update data from the Data window.");
            return;
        }
        ui.label("Choose the act and, once the map shows it, its boss.");
        for act in acts {
            ui.add_space(4.0);
            ui.label(RichText::new(&act.name).strong());
            ui.horizontal_wrapped(|ui| {
                if ui.button("Boss unknown").clicked() {
                    out.push(Action::EnterAct { act: act.id.clone(), index, boss: None });
                }
                for boss in &act.bosses {
                    if ui.button(v.catalog.encounter_name(boss)).clicked() {
                        out.push(Action::EnterAct { act: act.id.clone(), index, boss: Some(boss.clone()) });
                    }
                }
            });
        }
    });
}
