//! Current act, its boss(es), and macro priorities.

use eframe::egui::{self, RichText, Ui};
use spire_core::analysis::Severity;
use spire_core::run::{NodeKind, RunEvent};

use crate::actions::Action;
use crate::app::ViewCtx;
use crate::style::color::*;
use crate::views::{start, threats};
use crate::widgets::{eyebrow, panel, stripe};

pub fn show(ui: &mut Ui, v: &ViewCtx, out: &mut Vec<Action>) {
    let s = v.state;
    let act = s.act.as_ref().and_then(|a| v.catalog.acts.get(a));

    if s.nodes.last() == Some(&NodeKind::Boss) && s.act_index < v.catalog.max_act_index() {
        start::act_picker(ui, v, s.act_index + 1, out);
        ui.add_space(12.0);
    }

    let double = v.catalog.rules.double_boss(s.ascension, s.act_index);
    let title = format!("Act {} · {}", s.act_index + 1, act.map_or("?", |a| a.name.as_str()));
    let note = if double { Some("Two bosses at this ascension") } else { None };
    panel(ui, &title, note, |ui| {
        for boss in &v.dash.bosses {
            if crate::widgets::link(ui, RichText::new(&boss.name).heading().color(text()))
                .on_hover_text("Open in Enemies")
                .clicked()
            {
                out.push(Action::Open(crate::actions::Link::Encounter(boss.encounter.clone())));
            }
            threats::preview_body(ui, boss);
            ui.add_space(6.0);
        }
        if let Some(act) = act {
            let pick = |ui: &mut Ui, label: &str, second: bool, out: &mut Vec<Action>| {
                ui.label(RichText::new(label).color(text_dim()));
                ui.horizontal_wrapped(|ui| {
                    for b in act.bosses.iter().filter(|b| Some(*b) != s.boss.as_ref()) {
                        if ui.button(v.catalog.encounter_name(b)).clicked() {
                            let e =
                                if second { RunEvent::SetSecondBoss(b.clone()) } else { RunEvent::SetBoss(b.clone()) };
                            out.push(Action::Record(e));
                        }
                    }
                });
            };
            if s.boss.is_none() {
                pick(ui, "Boss not set. Pick it once the map shows it:", false, out);
            } else if double && s.second_boss.is_none() {
                pick(ui, "Second boss:", true, out);
            }
        }
    });

    ui.add_space(12.0);
    panel(ui, "Priorities", None, |ui| {
        if v.dash.advice.is_empty() {
            ui.label(RichText::new("Nothing pressing.").color(text_dim()));
        }
        for a in &v.dash.advice {
            let (color, text_color) = match a.severity {
                Severity::Urgent => (bad(), text()),
                Severity::Watch => (warn(), text()),
                Severity::Info => (border_hi(), text_dim()),
            };
            ui.horizontal(|ui| {
                stripe(ui, color, 18.0);
                ui.add(egui::Label::new(RichText::new(&a.text).color(text_color)).wrap());
            });
        }
        if !s.encounters_seen.is_empty() {
            ui.add_space(6.0);
            egui::CollapsingHeader::new(eyebrow(&format!("Fought this run · {}", s.encounters_seen.len())))
                .id_salt("fought")
                .default_open(false)
                .show(ui, |ui| {
                    for e in &s.encounters_seen {
                        ui.label(RichText::new(v.catalog.encounter_name(e)).color(text_dim()));
                    }
                });
        }
    });
}
