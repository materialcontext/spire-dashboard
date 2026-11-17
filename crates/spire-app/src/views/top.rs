//! Top bar: run vitals and global controls.

use eframe::egui::{self, RichText, Ui};

use crate::actions::Action;
use crate::app::{UiState, ViewCtx};

pub fn show(ui: &mut Ui, v: &ViewCtx, st: &UiState, out: &mut Vec<Action>) {
    let s = v.state;
    ui.horizontal(|ui| {
        ui.label(RichText::new("Spire Dashboard").strong());
        ui.separator();
        if let Some(c) = s.character.as_ref().and_then(|c| v.catalog.characters.get(c)) {
            ui.label(RichText::new(&c.name).strong());
            if s.ascension > 0 {
                ui.label(format!("A{}", s.ascension));
            }
            ui.separator();
            let act = s.act.as_ref().and_then(|a| v.catalog.acts.get(a));
            ui.label(format!("Act {} {}", s.act_index + 1, act.map_or(String::new(), |a| format!("({})", a.name))));
            ui.label(format!("Floor {}", s.floor));
            ui.separator();
            ui.label("HP");
            ui.add(egui::ProgressBar::new(s.hp_fraction()).desired_width(140.0).text(format!("{}/{}", s.hp, s.max_hp)));
            ui.label(format!("Gold {}", s.gold));
            ui.label(format!("Potions {}/{}", s.potions.len(), s.potion_slots));
            ui.label(format!("Deck {}", s.deck.len()));
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button(if st.updating { "Data… (updating)" } else { "Data…" }).clicked() {
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
                if ui.button("New run").clicked() {
                    out.push(Action::AbandonRun);
                }
            }
        });
    });
    if let Some(msg) = st.messages.last() {
        ui.label(RichText::new(msg).small().weak());
    }
}
