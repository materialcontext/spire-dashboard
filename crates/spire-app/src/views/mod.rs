//! Dashboard views. Each is a function of read-only data that pushes
//! [`Action`]s; UI-only state is passed in separately.

pub mod act;
pub mod breakdown;
pub mod data;
pub mod deck;
pub mod events;
pub mod log;
pub mod start;
pub mod threats;
pub mod top;

use eframe::egui::{self, Ui};

use crate::actions::Action;
use crate::app::{UiState, ViewCtx};
use crate::draft::NodeDraft;
use crate::widgets::culled;

pub fn root(ui: &mut Ui, v: &ViewCtx, draft: &mut NodeDraft, st: &mut UiState, out: &mut Vec<Action>) {
    egui::Panel::top("top").show(ui, |ui| top::show(ui, v, st, out));

    if !v.state.started() {
        egui::CentralPanel::default().show(ui, |ui| start::show(ui, v, st, out));
        return;
    }

    egui::Panel::left("deck").resizable(true).default_size(300.0).show(ui, |ui| {
        egui::ScrollArea::vertical().show(ui, |ui| deck::show(ui, v, out));
    });
    egui::Panel::right("log").resizable(true).default_size(340.0).show(ui, |ui| {
        egui::ScrollArea::vertical().show(ui, |ui| log::show(ui, v, draft, out));
    });
    egui::CentralPanel::default().show(ui, |ui| {
        egui::ScrollArea::vertical().show(ui, |ui| {
            if v.state.act.is_none() {
                start::enter_act(ui, v, out);
                return;
            }
            let wide = ui.available_width() > 680.0;
            if wide {
                ui.columns(2, |cols| {
                    let g = v.generation;
                    culled(&mut cols[0], ("act", g), |ui| act::show(ui, v, out));
                    cols[0].add_space(8.0);
                    culled(&mut cols[0], ("breakdown", g), |ui| breakdown::show(ui, v));
                    culled(&mut cols[1], ("threats", g, st.threat_tab), |ui| threats::show(ui, v, st));
                    cols[1].add_space(8.0);
                    events::show(&mut cols[1], v);
                });
            } else {
                act::show(ui, v, out);
                breakdown::show(ui, v);
                threats::show(ui, v, st);
                events::show(ui, v);
            }
        });
    });
}
