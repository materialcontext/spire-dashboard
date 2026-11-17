//! Potential next threats by node type.

use eframe::egui::{RichText, Ui};
use spire_core::analysis::ThreatPreview;
use spire_core::run::NodeKind;

use crate::app::{UiState, ViewCtx};
use crate::widgets::{chip, coverage_color, section};

/// Stats and demand chips for one encounter.
pub fn preview_body(ui: &mut Ui, t: &ThreatPreview) {
    let p = &t.profile;
    let hp = if p.hp_max > p.hp_min { format!("{}–{}", p.hp_min, p.hp_max) } else { p.hp_min.to_string() };
    let mut facts = vec![format!("{hp} HP"), format!("worst turn {}", p.worst_turn)];
    if p.group || p.enemies > 1 {
        facts.push("multiple enemies".into());
    }
    if p.buffs {
        facts.push("buffs".into());
    }
    if p.debuffs {
        facts.push("debuffs".into());
    }
    if p.statuses {
        facts.push("statuses".into());
    }
    ui.label(RichText::new(facts.join(" · ")).small());
    ui.horizontal_wrapped(|ui| {
        for c in &t.checks {
            let color = coverage_color(ui, c.coverage);
            chip(ui, format!("{} ({})", c.demand.capability.label(), c.coverage.label()), color)
                .on_hover_text(format!("{}. You have {}.", c.demand.reason, c.have));
        }
    });
}

pub fn show(ui: &mut Ui, v: &ViewCtx, st: &mut UiState) {
    section(ui, "Next threats", |ui| {
        ui.horizontal(|ui| {
            for k in [NodeKind::Monster, NodeKind::Elite, NodeKind::Boss] {
                ui.selectable_value(&mut st.threat_tab, k, k.label());
            }
        });
        let list =
            v.dash.threats.iter().find(|(k, _)| *k == st.threat_tab).map(|(_, l)| l.as_slice()).unwrap_or_default();
        if st.threat_tab == NodeKind::Monster {
            let weak = v.playbook.act(v.state.act_index).is_some_and(|p| v.state.hallway_fights_in_act < p.weak_fights);
            ui.label(RichText::new(if weak { "Early-act (weak) pool" } else { "Normal hallway pool" }).small().weak());
        }
        if list.is_empty() {
            ui.label("Nothing known for this act.");
        }
        for t in list {
            ui.add_space(4.0);
            // Name colored by the worst-covered demand; dimmed once seen.
            let mut title = RichText::new(&t.name).strong();
            if let Some(c) = t.worst() {
                title = title.color(coverage_color(ui, c));
            }
            if t.seen {
                title = title.weak();
            }
            ui.horizontal(|ui| {
                ui.label(title);
                if t.seen {
                    ui.label(RichText::new("seen").small().weak());
                }
            });
            preview_body(ui, t);
        }
        ui.add_space(4.0);
        ui.label(RichText::new("HP counts each monster type once; groups can have more.").small().weak())
            .on_hover_text("The data source lists distinct monsters per encounter, not their count.");
    });
}
