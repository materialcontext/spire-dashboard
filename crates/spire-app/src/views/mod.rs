//! Dashboard views. Each is a function of read-only data that pushes
//! [`Action`]s; UI-only state is passed in separately.
//!
//! Layout: header across the top; the run (deck, relics, potions) on the
//! left; the floor log on the right; analysis in the middle, two columns
//! when there is room.

pub mod act;
pub mod analytics;
pub mod breakdown;
pub mod cards;
pub mod data;
pub mod deck;
pub mod dialogs;
pub mod enemies;
pub mod events;
pub mod history;
pub mod log;
pub mod relics;
pub mod start;
pub mod threats;
pub mod top;

use eframe::egui::{self, Ui};

use crate::actions::Action;
use crate::app::{UiState, ViewCtx};
use crate::draft::NodeDraft;
use crate::widgets::culled;

const GAP: f32 = 12.0;

/// Top-level navigation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tab {
    Run,
    Analytics,
    History,
    Cards,
    Relics,
    Enemies,
}

impl Tab {
    pub const ALL: [Tab; 6] = [Tab::Run, Tab::Analytics, Tab::History, Tab::Cards, Tab::Relics, Tab::Enemies];

    pub fn label(self) -> &'static str {
        match self {
            Tab::Run => "Run",
            Tab::Analytics => "Analytics",
            Tab::History => "History",
            Tab::Cards => "Cards",
            Tab::Relics => "Relics",
            Tab::Enemies => "Enemies",
        }
    }
}

pub fn root(ui: &mut Ui, v: &ViewCtx, draft: &mut NodeDraft, st: &mut UiState, out: &mut Vec<Action>) {
    let side = egui::Frame::new().inner_margin(egui::Margin { left: 14, right: 14, top: 0, bottom: 0 });
    egui::Panel::top("top").frame(side).show(ui, |ui| top::show(ui, v, st, out));
    dialogs::show(ui, v, st, out);
    toast(ui, st, out);

    let center = egui::Frame::new().inner_margin(egui::Margin::same(GAP as i8));
    match st.tab {
        Tab::Run => {}
        Tab::Analytics => {
            egui::CentralPanel::default().frame(center).show(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| match v.analytics {
                    Some(a) => analytics::show(ui, v, a, st),
                    None => {
                        ui.label("Preparing analytics…");
                    }
                });
            });
            return;
        }
        Tab::History => {
            egui::Panel::left("history-list").frame(side).resizable(true).default_size(300.0).show(ui, |ui| {
                history::list(ui, v, &mut st.history);
            });
            egui::CentralPanel::default().frame(center).show(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| history::detail(ui, v, &mut st.history));
            });
            return;
        }
        Tab::Cards => {
            egui::Panel::left("card-filters").frame(side).resizable(true).default_size(260.0).show(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| cards::filters(ui, v, &mut st.cards));
            });
            egui::Panel::right("card-detail").frame(side).resizable(true).default_size(420.0).show(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| cards::detail(ui, v, &mut st.cards, out));
            });
            egui::CentralPanel::default()
                .frame(egui::Frame::new().inner_margin(egui::Margin::same(GAP as i8)))
                .show(ui, |ui| cards::list(ui, v, &mut st.cards));
            return;
        }
        Tab::Relics => {
            egui::Panel::left("relic-filters").frame(side).resizable(true).default_size(260.0).show(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| relics::filters(ui, v, &mut st.relics));
            });
            egui::Panel::right("relic-detail").frame(side).resizable(true).default_size(420.0).show(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| relics::detail(ui, v, &mut st.relics));
            });
            egui::CentralPanel::default()
                .frame(egui::Frame::new().inner_margin(egui::Margin::same(GAP as i8)))
                .show(ui, |ui| relics::list(ui, v, &mut st.relics));
            return;
        }
        Tab::Enemies => {
            egui::Panel::left("enemy-filters").frame(side).resizable(true).default_size(260.0).show(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| enemies::filters(ui, v, &mut st.enemies));
            });
            egui::Panel::right("enemy-detail").frame(side).resizable(true).default_size(460.0).show(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| enemies::detail(ui, v, &st.enemies));
            });
            egui::CentralPanel::default()
                .frame(egui::Frame::new().inner_margin(egui::Margin::same(GAP as i8)))
                .show(ui, |ui| enemies::list(ui, v, &mut st.enemies));
            return;
        }
    }

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
                    culled(&mut cols[1], ("threats", g, st.threat_tab), |ui| threats::show(ui, v, st, out));
                    cols[1].add_space(GAP);
                    events::show(&mut cols[1], v);
                });
            } else {
                act::show(ui, v, out);
                ui.add_space(GAP);
                breakdown::show(ui, v);
                ui.add_space(GAP);
                threats::show(ui, v, st, out);
                ui.add_space(GAP);
                events::show(ui, v);
            }
        });
    });
}

/// The commit notice, bottom centre, with Undo.
fn toast(ui: &mut Ui, st: &UiState, out: &mut Vec<Action>) {
    use crate::style::color::*;
    let Some(t) = &st.toast else { return };
    egui::Area::new(egui::Id::unique("toast"))
        .anchor(egui::Align2::CENTER_BOTTOM, egui::vec2(0.0, -24.0))
        .order(egui::Order::Foreground)
        .show(ui.ctx(), |ui| {
            egui::Frame::new()
                .fill(surface_hi())
                .stroke(egui::Stroke::new(1.0, border_hi()))
                .inner_margin(egui::Margin::symmetric(14, 8))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(&t.text).color(text()));
                        ui.add_space(12.0);
                        if ui.button(egui::RichText::new("Undo").color(accent())).clicked() {
                            out.push(Action::Undo);
                            out.push(Action::DismissToast);
                        }
                        if ui.add(egui::Button::new(egui::RichText::new("x").color(text_dim())).frame(false)).clicked()
                        {
                            out.push(Action::DismissToast);
                        }
                    });
                });
        });
}
