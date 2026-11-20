//! History's run report: how the run ended and why, the deciding fight, a
//! row per act and the decision log. Compact lines; the numbers behind each
//! are on hover. The report is computed once per selected run
//! (`analysis::report`), never per frame.

use eframe::egui::{self, RichText, Ui};
use spire_core::analysis::benchmark::Comparison;
use spire_core::analysis::report::{ActCard, Bleed, Decision, FloorDecisions, RunReport};
use spire_core::analysis::survival::Verdict;
use spire_core::catalog::NodeKind;
use spire_core::ids::CardId;
use spire_core::run::Outcome;

use crate::app::ViewCtx;
use crate::style::color::*;
use crate::widgets::{caption, panel, secondary};

/// Width the summary needs to put Acts and How it ended side by side: each
/// half fits the Acts table (about 500 points with its frame).
const SIDE_BY_SIDE: f32 = 1040.0;

/// Acts beside How it ended, the two panels ending level; stacked when a
/// half is too narrow for the Acts table.
pub fn summary(ui: &mut Ui, v: &ViewCtx, r: &RunReport) {
    let has_acts = !r.acts.is_empty();
    if ui.available_width() >= SIDE_BY_SIDE && has_acts {
        let id = egui::Id::unique("report-summary");
        ui.columns(2, |cols| {
            let [a, b] = cols else { return };
            panel(a, "Acts", None, |ui| level(ui, id, 0, |ui| acts(ui, v, r)));
            panel(b, "How it ended", None, |ui| level(ui, id, 1, |ui| ending(ui, v, r)));
        });
        return;
    }
    panel(ui, "How it ended", None, |ui| ending(ui, v, r));
    if has_acts {
        ui.add_space(crate::style::space::m());
        panel(ui, "Acts", None, |ui| acts(ui, v, r));
    }
}

/// Lays out one of several panels side by side at least as tall as the
/// tallest of them, so they end level. Each records its own height for the
/// next frame (a frame late when the heights change, then repainted).
fn level(ui: &mut Ui, id: egui::Id, slot: usize, body: impl FnOnce(&mut Ui)) {
    let key = id.with(slot);
    let (mine, tallest) = ui.ctx().data(|d| {
        let tallest = (0..2).filter_map(|i| d.get_temp::<f32>(id.with(i))).fold(0.0, f32::max);
        (d.get_temp::<f32>(key), tallest)
    });
    body(ui);
    let natural = ui.min_rect().height();
    if mine != Some(natural) {
        ui.ctx().data_mut(|d| d.insert_temp(key, natural));
        ui.ctx().request_repaint();
    }
    // Pad by the difference (`set_min_height` counts from the cursor, not
    // the top).
    if tallest > natural {
        ui.add_space(tallest - natural);
    }
}

/// The run's ending, its likely causes, and the deciding fight.
fn ending(ui: &mut Ui, v: &ViewCtx, r: &RunReport) {
    ui.label(RichText::new(headline(v, r)).color(text()));
    for c in &r.causes {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            ui.label(RichText::new("•").color(bad()));
            ui.label(RichText::new(&c.text).color(text())).on_hover_text(&c.detail);
        });
    }
    let Some(d) = &r.deciding else { return };
    ui.add_space(crate::style::space::s());
    let label = if d.won { "Costliest fight" } else { "Deciding fight" };
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        ui.label(secondary(label));
        ui.label(RichText::new(v.catalog.encounter_name(&d.encounter)).strong().color(text()));
        ui.label(caption(format!("floor {} · lost {} of {} HP", d.floor, d.lost, d.hp.0)));
    });
    // The deck's outlook explains a loss; a fight won speaks for itself.
    if !d.won {
        let stats = v.catalog.encounters.get(&d.encounter).and_then(|e| e.stats.as_ref());
        crate::views::turns::outlook(ui, &d.outlook, stats);
    }
}

fn headline(v: &ViewCtx, r: &RunReport) -> String {
    let e = &r.ending;
    let hp = e.hp_in.map_or(String::new(), |(hp, max)| format!(", going in at {hp}/{max} HP"));
    let at = format!("floor {} (Act {})", e.floor, e.act_index + 1);
    match &e.outcome {
        Some(Outcome::Victory) => format!("Won on {at}{hp}"),
        Some(Outcome::Died { encounter: Some(k) }) => format!("Died to {} on {at}{hp}", v.catalog.encounter_name(k)),
        Some(Outcome::Died { encounter: None }) => format!("Died on {at}{hp}"),
        Some(Outcome::Abandoned) => format!("Abandoned on {at}"),
        None => format!("Ongoing, on {at}"),
    }
}

/// One row per act: HP in and out, damage against the community's, elites,
/// deck changes, unmet targets and the boss.
fn acts(ui: &mut Ui, v: &ViewCtx, r: &RunReport) {
    egui::Grid::new("report-acts").num_columns(5).spacing([12.0, 4.0]).show(ui, |ui| {
        for h in ["", "HP out", "Damage", "Deck", "Boss"] {
            ui.label(caption(h));
        }
        ui.end_row();
        for a in &r.acts {
            act_row(ui, v, a);
            ui.end_row();
        }
    });
}

fn act_row(ui: &mut Ui, v: &ViewCtx, a: &ActCard) {
    let name = a.act.as_ref().and_then(|id| v.catalog.acts.get(id)).map(|x| x.name.as_str());
    let title = ui.label(RichText::new(format!("Act {}", a.index + 1)).strong().color(text()));
    if let Some(n) = name {
        title.on_hover_text(n);
    }

    let frac = |(hp, max): (i32, i32)| if max > 0 { hp as f32 / max as f32 } else { 0.0 };
    // HP at the act's end; going in on hover.
    ui.label(RichText::new(format!("{}/{}", a.hp_out.0, a.hp_out.1)).color(crate::style::hp_color(frac(a.hp_out))))
        .on_hover_text(format!("{}/{} going in", a.hp_in.0, a.hp_in.1));

    // Damage against the community's: the worst room type, the rest on hover.
    let rooms = ["Hallway", "Elite", "Boss"];
    let worst =
        a.bleed.iter().zip(rooms).filter_map(|(b, n)| b.ratio().map(|x| (x, n))).max_by(|x, y| x.0.total_cmp(&y.0));
    let damage = match worst {
        Some((x, n)) => RichText::new(format!("×{x:.1} {}", n.to_lowercase())).color(ratio_color(x)),
        None => secondary("—"),
    };
    ui.label(damage).on_hover_ui(|ui| {
        ui.label("HP lost against players' average for the same fights");
        for (b, n) in a.bleed.iter().zip(rooms) {
            bleed_line(ui, n, b);
        }
        ui.label(secondary(format!("{} elites fought", a.elites)));
    });

    let deck = format!("+{} −{}", a.added, a.removed);
    let unmet: Vec<&(spire_core::playbook::Role, u32, u32)> = a.roles.iter().filter(|(_, n, t)| n < t).collect();
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        ui.label(RichText::new(deck).color(text()))
            .on_hover_text(format!("{} cards added, {} removed, {} upgraded", a.added, a.removed, a.upgraded));
        let gaps = unmet.len() + a.short.len();
        if gaps > 0 {
            ui.label(RichText::new(format!("{gaps} short")).small().color(warn())).on_hover_ui(|ui| {
                ui.set_max_width(320.0);
                for (role, n, t) in &unmet {
                    ui.label(format!("{}: {n} of {t} (act target)", role.label()));
                }
                for c in &a.short {
                    ui.label(comparison(c));
                }
            });
        }
    });

    match &a.boss {
        Some((id, verdict, hp_loss, beaten)) => {
            let (word, color) = match beaten {
                Some(true) => ("Beaten", good()),
                Some(false) => ("Lost", bad()),
                None => ("Not fought", text_dim()),
            };
            ui.label(RichText::new(v.catalog.encounter_name(id)).color(color)).on_hover_text(format!(
                "{word}. The deck going in was {}: about {hp_loss} HP lost in the simulation.",
                verdict_word(*verdict)
            ));
        }
        None => {
            ui.label(secondary("—"));
        }
    }
}

fn verdict_word(v: Verdict) -> &'static str {
    match v {
        Verdict::OnTrack => "on track",
        Verdict::Tight => "tight",
        Verdict::AtRisk => "at risk",
    }
}

fn ratio_color(x: f32) -> egui::Color32 {
    if x >= 1.5 {
        bad()
    } else if x >= 1.15 {
        warn()
    } else {
        text()
    }
}

fn bleed_line(ui: &mut Ui, room: &str, b: &Bleed) {
    if b.fights == 0 {
        return;
    }
    let typical = if b.typical > 0.0 { format!(", players {:.0}", b.typical) } else { String::new() };
    ui.label(secondary(format!("{room}: {} fights, you {:.0}{typical}", b.fights, b.yours)));
}

fn comparison(c: &Comparison) -> String {
    let fmt = |x: f32| if c.measure.is_share() { format!("{:.0}%", x * 100.0) } else { format!("{x:.1}") };
    format!("{}: {} (cleared decks {}–{})", c.measure.label(), fmt(c.yours), fmt(c.range.p25), fmt(c.range.p75))
}

/// Floor-by-floor decisions; clicking a floor replays it. `at` is the floor
/// being replayed.
pub fn decisions(ui: &mut Ui, v: &ViewCtx, r: &RunReport, at: u32) -> Option<u32> {
    if r.floors.is_empty() {
        return None;
    }
    let mut pick = None;
    panel(ui, "Decisions", None, |ui| {
        egui::ScrollArea::vertical().id_salt("report-decisions").max_height(260.0).show(ui, |ui| {
            let mut act = None;
            for f in &r.floors {
                if act != Some(f.act_index) {
                    act = Some(f.act_index);
                    ui.label(caption(format!("Act {}", f.act_index + 1)));
                }
                if floor_row(ui, v, f, f.floor == at) {
                    pick = Some(f.floor);
                }
            }
        });
    });
    pick
}

/// One floor: its number and room (a link to replay it), then its decisions.
fn floor_row(ui: &mut Ui, v: &ViewCtx, f: &FloorDecisions, current: bool) -> bool {
    let mut clicked = false;
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        // A label that breaks inside itself throws the row's later labels
        // onto the wrong line (they overlap); whole labels move down instead.
        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
        let place = f
            .encounter
            .as_ref()
            .map(|e| v.catalog.encounter_name(e))
            .or_else(|| f.event.as_ref().map(|e| v.catalog.event_name(e)))
            .unwrap_or_else(|| f.kind.label().to_string());
        let label = RichText::new(format!("{:>2}", f.floor)).monospace();
        let label = if current { label.color(accent()).strong() } else { label.color(text_dim()) };
        clicked |= crate::widgets::link(ui, label).on_hover_text(format!("{place} · replay this floor")).clicked();
        ui.label(RichText::new(short_kind(f.kind)).small().color(text_dim())).on_hover_text(place);
        for (i, d) in f.decisions.iter().enumerate() {
            if i > 0 {
                ui.label(secondary("·"));
            }
            decision(ui, v, f.kind, d);
        }
    });
    clicked
}

fn short_kind(k: NodeKind) -> &'static str {
    match k {
        NodeKind::Monster => "Fight",
        NodeKind::Elite => "Elite",
        NodeKind::Boss => "Boss",
        NodeKind::Shop => "Shop",
        NodeKind::Rest => "Rest",
        NodeKind::Treasure => "Chest",
        NodeKind::Unknown => "?",
        NodeKind::Ancient => "Ancient",
    }
}

fn decision(ui: &mut Ui, v: &ViewCtx, kind: NodeKind, d: &Decision) {
    let took = if kind == NodeKind::Shop { "Bought" } else { "Took" };
    match d {
        Decision::Cards { taken, passed } => {
            if taken.is_empty() {
                ui.label(secondary("Skipped"));
            } else {
                ui.label(secondary(took));
                cards(ui, v, taken, text());
            }
            if !passed.is_empty() {
                ui.label(caption(if taken.is_empty() { "" } else { "over" }));
                cards(ui, v, passed, text_faint());
            }
        }
        Decision::Relics { taken, passed } => {
            named(ui, took, taken.iter().map(|r| v.catalog.relic_name(r)), text());
            if !passed.is_empty() {
                named(ui, "over", passed.iter().map(|r| v.catalog.relic_name(r)), text_faint());
            }
        }
        Decision::Potions { taken, passed } => {
            if !taken.is_empty() {
                named(ui, took, taken.iter().map(|p| v.catalog.potion_name(p)), text());
            }
            if !passed.is_empty() {
                named(ui, "left", passed.iter().map(|p| v.catalog.potion_name(p)), text_faint());
            }
        }
        Decision::Removed(c) => {
            ui.label(secondary("Removed"));
            cards(ui, v, c, text());
        }
        Decision::Upgraded(c) => {
            ui.label(secondary("Upgraded"));
            cards(ui, v, c, text());
        }
        Decision::Transformed(pairs) => {
            ui.label(secondary("Transformed"));
            for (from, to) in pairs {
                cards(ui, v, std::slice::from_ref(from), text_dim());
                ui.label(secondary("into"));
                cards(ui, v, std::slice::from_ref(to), text());
            }
        }
        Decision::Rest { choice, hp, max_hp } => {
            let low = *max_hp > 0 && (*hp as f32 / *max_hp as f32) < 0.4;
            let word = rest_word(choice);
            let color = if low && choice == "SMITH" { warn() } else { text() };
            ui.label(RichText::new(word).color(color));
            ui.label(caption(format!("at {hp}/{max_hp}")));
        }
        Decision::Gold { spent, left } => {
            ui.label(RichText::new(format!("−{spent} gold")).color(text()));
            ui.label(caption(format!("{left} left")));
        }
        Decision::PotionsUsed(p) => {
            named(ui, "Used", p.iter().map(|p| v.catalog.potion_name(p)), text());
        }
    }
}

/// A rest-site choice in words (the game's ids are upper case).
fn rest_word(choice: &str) -> String {
    match choice {
        "HEAL" => "Rested".into(),
        "SMITH" => "Smithed".into(),
        "OTHER" => "Other".into(),
        c => {
            let mut s = c.to_lowercase();
            if let Some(f) = s.get_mut(..1) {
                f.make_ascii_uppercase();
            }
            s
        }
    }
}

fn named(ui: &mut Ui, verb: &str, names: impl Iterator<Item = String>, color: egui::Color32) {
    ui.label(secondary(verb));
    // One label per name, so a long list wraps between names.
    let names: Vec<String> = names.collect();
    for (i, name) in names.iter().enumerate() {
        let sep = if i + 1 < names.len() { "," } else { "" };
        ui.label(RichText::new(format!("{name}{sep}")).color(color));
    }
}

/// Card names, each with its details on hover.
fn cards(ui: &mut Ui, v: &ViewCtx, ids: &[CardId], color: egui::Color32) {
    for (i, id) in ids.iter().enumerate() {
        let sep = if i + 1 < ids.len() { "," } else { "" };
        ui.label(RichText::new(format!("{}{sep}", v.catalog.card_name(id))).color(color)).on_hover_ui(|ui| {
            ui.set_max_width(320.0);
            let (roles, tags) = v.catalog.cards.get(id).map_or_else(Default::default, |c| {
                (v.playbook.roles_of(c), spire_core::analysis::tags::tags(c, false, v.playbook))
            });
            crate::views::deck::card_tip(ui, v, id, false, &roles, &tags);
        });
    }
}
