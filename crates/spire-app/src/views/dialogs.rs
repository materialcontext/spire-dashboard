//! Modal dialogs: ending a run, and confirming an abandon.

use eframe::egui::{self, RichText, Ui};
use spire_core::run::Outcome;

use crate::actions::Action;
use crate::app::{UiState, ViewCtx};
use crate::style::color::*;
use crate::widgets::{eyebrow, hairline, primary_button};

pub fn show(ui: &mut Ui, v: &ViewCtx, st: &mut UiState, out: &mut Vec<Action>) {
    let ctx = ui.ctx().clone();
    if st.end_run_open {
        let modal = egui::Modal::new(egui::Id::unique("end-run")).show(&ctx, |ui| {
            ui.set_width(380.0);
            ui.label(RichText::new("End run").heading());
            ui.label(RichText::new("The run is filed in history with this result.").small().color(text_dim()));
            hairline(ui);
            if primary_button(ui, "Victory", true).clicked() {
                out.push(Action::EndRun(Outcome::Victory));
                st.end_run_open = false;
            }
            ui.add_space(8.0);
            ui.label(eyebrow("Died to"));
            let label = st.end_encounter.as_ref().map_or("Unknown".to_string(), |e| v.catalog.encounter_name(e));
            egui::ComboBox::from_id_salt("death").selected_text(label).width(ui.available_width()).show_ui(ui, |ui| {
                ui.selectable_value(&mut st.end_encounter, None, "Unknown");
                // Fights from this run first, most recent at the top.
                for e in v.state.encounters_seen.iter().rev() {
                    ui.selectable_value(&mut st.end_encounter, Some(e.clone()), v.catalog.encounter_name(e));
                }
                if let Some(act) = &v.state.act {
                    for e in v.catalog.encounters.values().filter(|e| e.act.as_ref() == Some(act)) {
                        if !v.state.encounters_seen.contains(&e.id) {
                            ui.selectable_value(&mut st.end_encounter, Some(e.id.clone()), &e.name);
                        }
                    }
                }
            });
            if ui.add_sized([ui.available_width(), 28.0], egui::Button::new("Died")).clicked() {
                out.push(Action::EndRun(Outcome::Died { encounter: st.end_encounter.clone() }));
                st.end_run_open = false;
            }
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.button("Abandoned").clicked() {
                    out.push(Action::EndRun(Outcome::Abandoned));
                    st.end_run_open = false;
                }
                if ui.button(RichText::new("Cancel").color(text_dim())).clicked() {
                    st.end_run_open = false;
                }
            });
        });
        if modal.should_close() {
            st.end_run_open = false;
        }
    }

    if st.confirm_abandon {
        let modal = egui::Modal::new(egui::Id::unique("abandon")).show(&ctx, |ui| {
            ui.set_width(360.0);
            ui.label(RichText::new("Start a new run?").heading());
            ui.label(
                RichText::new("The current run is still in progress. It will be saved to history as Abandoned.")
                    .color(text_dim()),
            );
            ui.add_space(8.0);
            if primary_button(ui, "Abandon and start a new run", true).clicked() {
                out.push(Action::AbandonRun);
                st.confirm_abandon = false;
            }
            if ui.button(RichText::new("Keep playing").color(text_dim())).clicked() {
                st.confirm_abandon = false;
            }
        });
        if modal.should_close() {
            st.confirm_abandon = false;
        }
    }
}
