//! Modal dialogs: ending a run, and confirming an abandon.

use eframe::egui::{self, RichText, Ui};
use spire_core::run::Outcome;

use crate::actions::Action;
use crate::app::{UiState, ViewCtx};
use crate::style::color::*;
use crate::widgets::{eyebrow, hairline, primary_button, secondary};

pub fn show(ui: &mut Ui, v: &ViewCtx, st: &mut UiState, out: &mut Vec<Action>) {
    let ctx = ui.ctx().clone();
    if st.end_run_open {
        let modal = crate::widgets::modal("end-run").show(&ctx, |ui| {
            ui.set_width(380.0);
            ui.label(crate::widgets::title("End run"));
            // At 0 HP the run was lost: Died leads and is the default.
            let dead = v.state.hp <= 0;
            let note = if dead {
                "You're at 0 HP. The run is filed in history with this result."
            } else {
                "The run is filed in history with this result."
            };
            ui.label(secondary(note));
            hairline(ui);
            if !dead && primary_button(ui, "Victory", true).clicked() {
                out.push(Action::EndRun(Outcome::Victory));
                st.end_run_open = false;
            }
            if !dead {
                ui.add_space(crate::style::space::s());
            }
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
            let died = if dead {
                primary_button(ui, "Died", true)
            } else {
                ui.add_sized([ui.available_width(), 28.0], egui::Button::new("Died"))
            };
            if died.clicked() {
                out.push(Action::EndRun(Outcome::Died { encounter: st.end_encounter.clone() }));
                st.end_run_open = false;
            }
            if dead {
                ui.add_space(crate::style::space::s());
                if ui.button("Victory").clicked() {
                    out.push(Action::EndRun(Outcome::Victory));
                    st.end_run_open = false;
                }
            }
            ui.add_space(crate::style::space::s());
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
        let modal = crate::widgets::modal("abandon").show(&ctx, |ui| {
            ui.set_width(360.0);
            ui.label(crate::widgets::title("Start a new run?"));
            ui.label(
                RichText::new("The current run is still in progress. It will be saved to history as Abandoned.")
                    .color(text_dim()),
            );
            ui.add_space(crate::style::space::m());
            // The safe choice is the default: Enter and Esc keep playing.
            let enter = ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter));
            let (abandon, keep) = crate::widgets::dialog_row(
                ui,
                |ui| crate::widgets::destructive_button(ui, "Abandon run"),
                |ui| crate::widgets::dialog_button(ui, "Keep playing"),
            );
            if abandon {
                out.push(Action::AbandonRun);
                st.confirm_abandon = false;
            } else if keep || enter {
                st.confirm_abandon = false;
            }
        });
        if modal.should_close() {
            st.confirm_abandon = false;
        }
    }
}
