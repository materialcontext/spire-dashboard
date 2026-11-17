//! Dashboard views. Each is a function of read-only data that pushes
//! [`Action`]s; UI-only state is passed in separately.
//!
//! Layout: header across the top; the run (deck, relics, potions) on the
//! left; the floor log on the right; analysis in the middle, two columns
//! when there is room.

pub mod act;
pub mod breakdown;
pub mod data;
pub mod deck;
pub mod dialogs;
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

const GAP: f32 = 12.0;

pub fn root(ui: &mut Ui, v: &ViewCtx, draft: &mut NodeDraft, st: &mut UiState, out: &mut Vec<Action>) {
    let side = egui::Frame::new().inner_margin(egui::Margin { left: 14, right: 14, top: 0, bottom: 0 });
    egui::Panel::top("top").frame(side).show(ui, |ui| top::show(ui, v, st, out));
    dialogs::show(ui, v, st, out);

    if !v.state.started() {
        egui::CentralPanel::default().frame(side).show(ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| start::show(ui, v, st, out));
        });
        return;
    }

    egui::Panel::left("run").frame(side).resizable(true).default_size(290.0).min_size(240.0).show(ui, |ui| {
        egui::ScrollArea::vertical().show(ui, |ui| deck::show(ui, v, out));
    });
    egui::Panel::right("log").frame(side).resizable(true).default_size(330.0).min_size(280.0).show(ui, |ui| {
        egui::ScrollArea::vertical().show(ui, |ui| log::show(ui, v, draft, out));
    });
    let center = egui::Frame::new().inner_margin(egui::Margin::same(GAP as i8));
    egui::CentralPanel::default().frame(center).show(ui, |ui| {
        egui::ScrollArea::vertical().show(ui, |ui| {
            if v.state.act.is_none() {
                start::enter_act(ui, v, out);
                return;
            }
            let g = v.generation;
            if ui.available_width() > 680.0 {
                ui.columns(2, |cols| {
                    culled(&mut cols[0], ("act", g), |ui| act::show(ui, v, out));
                    cols[0].add_space(GAP);
                    culled(&mut cols[0], ("breakdown", g), |ui| breakdown::show(ui, v));
                    culled(&mut cols[1], ("threats", g, st.threat_tab), |ui| threats::show(ui, v, st));
                    cols[1].add_space(GAP);
                    events::show(&mut cols[1], v);
                });
            } else {
                act::show(ui, v, out);
                ui.add_space(GAP);
                breakdown::show(ui, v);
                ui.add_space(GAP);
                threats::show(ui, v, st);
                ui.add_space(GAP);
                events::show(ui, v);
            }
        });
    });
}
