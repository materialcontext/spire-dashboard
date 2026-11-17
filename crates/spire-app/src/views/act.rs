//! Current act, its boss, and macro advice.

use eframe::egui::{self, RichText, Ui};
use spire_core::analysis::Severity;
use spire_core::run::{NodeKind, RunEvent};

use crate::actions::Action;
use crate::app::ViewCtx;
use crate::views::{start, threats};
use crate::widgets::section;

pub fn show(ui: &mut Ui, v: &ViewCtx, out: &mut Vec<Action>) {
    let s = v.state;
    let act = s.act.as_ref().and_then(|a| v.catalog.acts.get(a));

    if s.nodes.last() == Some(&NodeKind::Boss) && s.act_index < v.catalog.max_act_index() {
        start::act_picker(ui, v, s.act_index + 1, out);
        ui.add_space(8.0);
    }

    section(ui, &format!("Act {}: {}", s.act_index + 1, act.map_or("?", |a| a.name.as_str())), |ui| {
        match (&v.dash.boss, act) {
            (Some(boss), _) => {
                ui.label(RichText::new(format!("Boss: {}", boss.name)).strong());
                threats::preview_body(ui, boss);
            }
            (None, Some(act)) => {
                ui.label("Boss not set. When the map shows it:");
                ui.horizontal_wrapped(|ui| {
                    for b in &act.bosses {
                        if ui.button(v.catalog.encounter_name(b)).clicked() {
                            out.push(Action::Record(RunEvent::SetBoss(b.clone())));
                        }
                    }
                });
            }
            (None, None) => {}
        }

        ui.add_space(6.0);
        ui.label(RichText::new("Macro").strong());
        for a in &v.dash.advice {
            let color = match a.severity {
                Severity::Urgent => ui.visuals().error_fg_color,
                Severity::Watch => ui.visuals().warn_fg_color,
                Severity::Info => ui.visuals().text_color(),
            };
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("•").color(color));
                ui.label(RichText::new(&a.text).color(color));
            });
        }

        if !s.encounters_seen.is_empty() {
            ui.add_space(6.0);
            egui::CollapsingHeader::new(format!("Enemies seen ({})", s.encounters_seen.len()))
                .default_open(false)
                .show(ui, |ui| {
                    for e in &s.encounters_seen {
                        ui.label(v.catalog.encounter_name(e));
                    }
                });
        }
    });
}
