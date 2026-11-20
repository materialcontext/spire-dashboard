//! Potential next fights by room type.

use eframe::egui::{self, RichText, Ui};
use spire_core::analysis::survival::Outlook;
use spire_core::analysis::{Coverage, ThreatPreview};
use spire_core::catalog::RoomType;
use spire_core::run::NodeKind;

use crate::actions::{Action, Link};
use crate::app::{UiState, ViewCtx};
use crate::style::color::*;
use crate::widgets::{caption, chip, coverage_color, culled, link, panel, secondary, segmented};

/// Lineups shown before collapsing the rest into "+N more".
const LINEUPS_SHOWN: usize = 3;

/// Lineup, stats and demand chips for one encounter.
/// `key` identifies this preview's content for caching its labels: pass the
/// dashboard generation (and anything else it depends on).
pub fn preview_body(
    ui: &mut Ui,
    key: impl std::hash::Hash + std::fmt::Debug + Copy,
    t: &ThreatPreview,
    outlook: Option<&Outlook>,
) {
    match t.lineups.as_slice() {
        [] => {}
        // A single monster's lineup would just repeat the encounter name.
        [_] if t.profile.enemies <= 1 => {}
        [(only, _)] => {
            ui.label(RichText::new(only).color(text_dim()));
        }
        many => {
            for (label, share) in many.iter().take(LINEUPS_SHOWN) {
                ui.label(secondary(format!("{:>3.0}%  {label}", share * 100.0)));
            }
            if many.len() > LINEUPS_SHOWN {
                let rest: Vec<String> =
                    many[LINEUPS_SHOWN..].iter().map(|(l, s)| format!("{:.0}%  {l}", s * 100.0)).collect();
                ui.label(caption(format!("+{} more lineups", rest.len()))).on_hover_text(rest.join("\n"));
            }
        }
    }
    if !t.spawns.is_empty() {
        ui.label(secondary(format!("Joins mid-fight: {}", t.spawns.join(", "))));
    }

    let p = &t.profile;
    // Some bosses (multi-phase) have no single HP value in the data.
    let hp = match (p.hp_min, p.hp_max) {
        (0, 0) => "HP varies".to_string(),
        (min, max) if max > min => format!("{min}–{max} HP"),
        (min, _) => format!("{min} HP"),
    };
    let mut facts = vec![hp];
    if p.phases > 1 {
        facts.push(format!("{} phases", p.phases));
    }
    for (on, word) in [(p.buffs, "buffs"), (p.debuffs, "debuffs"), (p.statuses, "statuses")] {
        if on {
            facts.push(word.into());
        }
    }
    ui.label(secondary(facts.join("  ·  ")));
    // Turns where runs end are marked for elites and bosses only.
    let boss_or_elite = t.room != RoomType::Monster;
    crate::views::turns::strip(ui, (key, &t.encounter), &t.plans, t.stats.as_ref().filter(|_| boss_or_elite));
    // Elites and bosses: when runs end here.
    if boss_or_elite {
        // Once monsters join mid-fight, a turn number no longer says whose
        // move it is, so the deadly turns go unnamed.
        let named: &[spire_core::analysis::threats::MonsterPlan] = if t.spawns.is_empty() { &t.plans } else { &[] };
        crate::views::turns::deaths(ui, (key, &t.encounter), named, t.stats.as_ref());
    }
    // Whether this deck is on track: elites and bosses, and the fight being
    // played (a hallway fight gets its outlook then).
    if let Some(o) = outlook {
        crate::views::turns::outlook(ui, o, t.stats.as_ref());
    }
    if let Some(s) = &t.stats {
        ui.label(caption(format!(
            "Players lose {:.1} HP over {:.1} turns on average",
            s.avg_damage_taken, s.avg_turns
        )))
        .on_hover_ui(|ui| {
            ui.label(format!("From {} community-submitted fights (Spire Codex).", s.fights));
        });
    }
    ui.horizontal_wrapped(|ui| {
        for c in &t.checks {
            chip(ui, format!("{} · {}", c.demand.role.label(), c.coverage.label()), coverage_color(c.coverage))
                .on_hover_ui(|ui| {
                    ui.label(format!("{}. You have {}.", c.demand.reason, c.have));
                });
        }
    });
}

fn status_word(c: Option<Coverage>) -> (&'static str, egui::Color32) {
    match c {
        Some(Coverage::Missing) => ("Gap", coverage_color(Coverage::Missing)),
        Some(Coverage::Thin) => ("Thin", coverage_color(Coverage::Thin)),
        Some(Coverage::Covered) => ("Ready", coverage_color(Coverage::Covered)),
        None => ("No demands", text_faint()),
    }
}

pub fn show(ui: &mut Ui, v: &ViewCtx, st: &mut UiState, out: &mut Vec<Action>) {
    let weak =
        v.state.hallway_fights_in_act < spire_core::analysis::threats::weak_fights(v.state, v.catalog, v.playbook);
    let note = match st.threat_tab {
        NodeKind::Monster if weak => "Early (weak) pool",
        NodeKind::Monster => "Normal pool",
        _ => "",
    };
    panel(ui, "Next fight", Some(note), |ui| {
        let tabs = [(NodeKind::Monster, "Enemy"), (NodeKind::Elite, "Elite"), (NodeKind::Boss, "Boss")];
        segmented(ui, &mut st.threat_tab, &tabs);
        ui.add_space(crate::style::space::xs());
        let list =
            v.dash.threats.iter().find(|(k, _)| *k == st.threat_tab).map(|(_, l)| l.as_slice()).unwrap_or_default();
        if list.is_empty() {
            ui.label(RichText::new("Nothing known for this act.").color(text_dim()));
        }
        // A capped height that scrolls, so the panels under it stay on
        // screen; cards scrolled out of view are skipped (`culled`).
        let height = (ui.ctx().content_rect().height() * NEXT_FIGHT_SHARE).max(NEXT_FIGHT_MIN);
        egui::ScrollArea::vertical().id_salt("next-fight").max_height(height).auto_shrink([false, true]).show(
            ui,
            |ui| {
                for t in list {
                    let key = ("threat", v.generation, v.combat.map(|c| c.generation), st.threat_tab, &t.encounter);
                    culled(ui, key, |ui| {
                        ui.add_space(crate::style::space::s());
                        fight_card(ui, v, t, out);
                    });
                }
            },
        );
    });
}

/// Next fight's list takes at most this share of the window's height (and
/// at least `NEXT_FIGHT_MIN` points), then scrolls.
const NEXT_FIGHT_SHARE: f32 = 0.3;
const NEXT_FIGHT_MIN: f32 = 200.0;

/// The fight the game is in, above Next fight (following only, not for
/// bosses: those have the act panel).
pub fn current(ui: &mut Ui, v: &ViewCtx, t: &ThreatPreview, out: &mut Vec<Action>) {
    let room = match t.room {
        RoomType::Elite => "Elite",
        _ => "Enemy",
    };
    panel(ui, "Current fight", Some(room), |ui| fight_card(ui, v, t, out));
}

/// One fight: its name (opens it in Enemies), whether it's covered, and
/// its preview with the deck's outlook.
fn fight_card(ui: &mut Ui, v: &ViewCtx, t: &ThreatPreview, out: &mut Vec<Action>) {
    ui.horizontal(|ui| {
        let name = RichText::new(&t.name).family(crate::style::strong_family()).color(if t.seen {
            text_dim()
        } else {
            text()
        });
        if link(ui, name).on_hover_text("Open in Enemies").clicked() {
            out.push(Action::Open(Link::Encounter(t.encounter.clone())));
        }
        if t.seen {
            ui.label(caption("fought"));
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let (word, color) = status_word(t.worst());
            ui.label(RichText::new(word).small().strong().color(color));
        });
    });
    preview_body(ui, v.generation, t, v.combat.and_then(|c| c.outlook(&t.encounter)));
}
