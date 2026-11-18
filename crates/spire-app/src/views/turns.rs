//! What enemies do turn by turn, when runs end against them, and whether
//! the deck is on track to survive.

use eframe::egui::{self, RichText, Ui};
use spire_core::analysis::pattern::{MoveOption, Plan, Turn};
use spire_core::analysis::survival::{Outlook, Split, Verdict};
use spire_core::analysis::threats::MonsterPlan;
use spire_core::catalog::EncounterStats;

use crate::style::color::*;
use crate::widgets::chip;

/// A turn with at least this share of an encounter's deaths is marked.
const DEADLY_SHARE: f32 = 0.15;
/// Turns shown in the compact strip before "…".
const STRIP_TURNS: usize = 8;

/// "15", "6×2", "Block 12", or the intent for moves that don't hit.
fn effect(o: &MoveOption) -> String {
    match (o.damage, o.block) {
        (0, Some(b)) => format!("block {b}"),
        (0, None) => o.intent.to_lowercase(),
        (_, _) if o.hits > 1 => format!("{}×{}", o.damage / o.hits.max(1), o.hits),
        (d, _) => d.to_string(),
    }
}

fn short(o: &MoveOption) -> String {
    format!("{} {}", o.name, effect(o))
}

fn turn_text(t: &Turn) -> String {
    match t.options.as_slice() {
        [only] => short(only),
        many => many.iter().map(|o| o.name.as_str()).collect::<Vec<_>>().join(" / "),
    }
}

/// Hover text for a turn: every option with its chance or condition.
fn turn_detail(n: usize, t: &Turn) -> String {
    let mut lines = vec![format!("Turn {n}")];
    for o in &t.options {
        let mut line = short(o);
        if let Some(c) = o.chance.filter(|_| t.options.len() > 1) {
            line.push_str(&format!(" ({:.0}%)", c * 100.0));
        }
        if let Some(c) = &o.condition {
            line.push_str(&format!(" if {c}"));
        }
        if !o.to_player.is_empty() {
            line.push_str(&format!(", applies {}", o.to_player.join(", ")));
        }
        if o.strength > 0 {
            line.push_str(&format!(", +{} Strength", o.strength));
        }
        lines.push(line);
    }
    if t.vulnerable {
        lines.push("You're Vulnerable from an earlier move.".into());
    }
    lines.join("\n")
}

/// The turns most deaths happen on.
fn deadly(stats: Option<&EncounterStats>) -> Vec<(u32, f32)> {
    stats.map(|s| s.deadly_turns(DEADLY_SHARE)).unwrap_or_default()
}

/// The ending of a strip: whether it repeats, varies, or is unknown.
fn tail(p: &Plan) -> Option<String> {
    if p.unknown {
        return Some("order unknown".into());
    }
    match (p.repeats_from, p.open_ended) {
        (Some(1), _) => {
            let s = p.strength_per_cycle();
            Some(if s > 0 { format!("repeats, +{s} Strength each time") } else { "repeats".into() })
        }
        (Some(from), _) => Some(format!("repeats from turn {from}")),
        (None, true) => Some("then depends on the fight".into()),
        (None, false) if p.turns.len() > STRIP_TURNS => Some("…".into()),
        _ => None,
    }
}

/// Where turn `n` (1-based) falls in the shown turns: past the first pass
/// of a cycle it maps back into the cycle.
fn shown_index(p: &Plan, n: usize) -> Option<usize> {
    let shown = p.shown().len();
    if n <= shown {
        return Some(n - 1);
    }
    let start = p.repeats_from? - 1;
    let len = shown.checked_sub(start).filter(|l| *l > 0)?;
    Some(start + (n - 1 - start) % len)
}

fn same_options(a: &Turn, b: &Turn) -> bool {
    a.options.len() == b.options.len() && a.options.iter().zip(&b.options).all(|(x, y)| x.name == y.name)
}

/// One line of turn chips per monster type. Turns where many runs end
/// are drawn in the danger colour.
pub fn strip(ui: &mut Ui, plans: &[MonsterPlan], stats: Option<&EncounterStats>) {
    let deadly = deadly(stats);
    let named = plans.len() > 1;
    for mp in plans {
        if mp.plan.turns.is_empty() {
            continue;
        }
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(4.0, 4.0);
            if named {
                let name = if mp.count > 1 { format!("{}×{}", mp.name, mp.count) } else { mp.name.clone() };
                ui.label(RichText::new(name).small().color(text_dim()));
            }
            if mp.plan.unknown {
                let names: Vec<&str> = mp.plan.turns[0].options.iter().map(|o| o.name.as_str()).collect();
                ui.label(RichText::new(format!("Any of: {}", names.join(", "))).small().color(text_dim()));
                return;
            }
            let shown = mp.plan.shown();
            // Only a single-monster plan maps deaths to its own turns.
            let marked: Vec<usize> = if named {
                Vec::new()
            } else {
                deadly.iter().filter_map(|(d, _)| shown_index(&mp.plan, *d as usize)).collect()
            };
            let mut collapsed = false;
            for (i, t) in shown.iter().take(STRIP_TURNS).enumerate() {
                let n = i + 1;
                let color = if marked.contains(&i) { bad() } else { text_dim() };
                // The same pick every turn from here on: show it once.
                let rest_same = i + 1 < shown.len() && shown[i..].iter().all(|x| same_options(x, t));
                if rest_same && !t.known() {
                    chip(ui, format!("{n}+ {}", turn_text(t)), color).on_hover_text(turn_detail(n, t));
                    collapsed = true;
                    break;
                }
                chip(ui, format!("{n} {}", turn_text(t)), color).on_hover_text(turn_detail(n, t));
            }
            if let Some(tail) = tail(&mp.plan).filter(|_| !collapsed) {
                ui.label(RichText::new(tail).small().color(text_faint()));
            }
        });
    }
}

/// The move a single-monster encounter makes on turn `n`, if known
/// (cycles are followed past their first pass).
fn move_on(plans: &[MonsterPlan], n: u32) -> Option<String> {
    let [mp] = plans else { return None };
    let t = mp.plan.turns.get(n as usize - 1)?;
    t.known().then(|| t.options[0].name.clone())
}

/// "1 in 5 runs that reach this fight end here, most on turn 7 (Dismember)."
pub fn deaths(ui: &mut Ui, plans: &[MonsterPlan], stats: Option<&EncounterStats>) {
    let Some(s) = stats.filter(|s| s.deaths > 0 && s.fights > 0) else { return };
    let rate = s.death_rate();
    let turns: Vec<String> = deadly(Some(s))
        .iter()
        .take(3)
        .map(|(t, share)| match move_on(plans, *t) {
            Some(m) => format!("turn {t} ({m}, {:.0}%)", share * 100.0),
            None => format!("turn {t} ({:.0}%)", share * 100.0),
        })
        .collect();
    let mut text = format!("{:.0}% of runs that reach this fight end here", rate * 100.0);
    if !turns.is_empty() {
        text.push_str(&format!(", most on {}", turns.join(", ")));
    }
    text.push('.');
    ui.label(RichText::new(text).small().color(if rate >= 0.1 { bad() } else { text_dim() })).on_hover_ui(|ui| {
        ui.label(format!(
            "{} of {} community fights (Spire Codex) ended the run. A turn is listed when it has at least {:.0}% of those deaths.",
            s.deaths,
            s.fights,
            DEADLY_SHARE * 100.0
        ));
        if let Some((won, died)) = s.hp_in {
            ui.label(format!("Players who won came in with {won} HP (median); those who died, {died}."));
        }
    });
}

fn verdict_color(v: Verdict) -> egui::Color32 {
    match v {
        Verdict::OnTrack => good(),
        Verdict::Tight => warn(),
        Verdict::AtRisk => bad(),
    }
}

/// "On track · win in about 6 turns, losing about 24 of your 54 HP".
pub fn outlook(ui: &mut Ui, o: &Outlook, stats: Option<&EncounterStats>) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        ui.label(RichText::new(o.verdict.label()).small().strong().color(verdict_color(o.verdict)));
        let text = match o.best.kill_turn {
            Some(t) if o.best.hp_loss as i32 >= o.hp => {
                format!("would lose about {} HP over {t} turns; you have {}", o.best.hp_loss, o.hp)
            }
            Some(t) => format!("win in about {t} turns, losing about {} of your {} HP", o.best.hp_loss, o.hp),
            None => format!("no win within 20 turns; about {} HP lost by then", o.best.hp_loss),
        };
        ui.label(RichText::new(text).small().color(text_dim()));
    })
    .response
    .on_hover_ui(|ui| {
        ui.set_max_width(360.0);
        let plan = match o.best.split {
            Split::Share(0.0) => "attacking every turn".to_string(),
            Split::Share(s) => format!("spending about {:.0}% of each turn on block", s * 100.0),
            Split::Need => "blocking what each turn brings, attacking with the rest".to_string(),
            Split::NeedFrom(at) => format!("blocking turns that bring {at}+ damage, attacking all-out otherwise"),
        };
        ui.label(format!("Best simple plan: {plan}."));
        if o.best != o.race {
            match o.race.kill_turn {
                Some(t) => ui.label(format!("All-out attack: win on turn {t}, losing about {} HP.", o.race.hp_loss)),
                None => ui.label("All-out attack doesn't win within 20 turns."),
            };
        }
        if let Some((median, p90)) = stats.and_then(|s| s.win_damage) {
            ui.label(format!("Players who won lost {median} HP (median); 1 in 10 lost {p90} or more."));
        }
        ui.label(
            RichText::new(
                "From the deck simulator's average damage and block per turn against the enemies' \
                 expected moves. Leaves out debuffs on you, enemies dying early, and potions.",
            )
            .small()
            .color(text_faint()),
        );
    });
}

/// Every monster's turns in full, for the Enemies tab.
pub fn table(ui: &mut Ui, mp: &MonsterPlan, stats: Option<&EncounterStats>, single: bool) {
    let p = &mp.plan;
    if p.unknown || p.turns.is_empty() {
        ui.label(RichText::new("Move order unknown: any of its moves, any turn.").small().color(text_faint()));
        return;
    }
    let deadly = if single { deadly(stats) } else { Vec::new() };
    egui::Grid::new(("turns", &mp.monster)).num_columns(3).spacing([12.0, 2.0]).show(ui, |ui| {
        for (i, t) in p.shown().iter().enumerate() {
            let n = i + 1;
            let marked = deadly.iter().find(|(d, _)| *d as usize == n);
            let color = if marked.is_some() { bad() } else { text_faint() };
            ui.label(RichText::new(format!("Turn {n}")).small().color(color));
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                for o in &t.options {
                    let mut name = o.name.clone();
                    if let Some(c) = o.chance.filter(|_| t.options.len() > 1) {
                        name.push_str(&format!("  {:.0}%", c * 100.0));
                    }
                    if let Some(c) = &o.condition {
                        name.push_str(&format!("  if {c}"));
                    }
                    ui.label(RichText::new(name).small().color(text()));
                }
            });
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                for o in &t.options {
                    let mut parts = vec![effect(o)];
                    if !o.to_player.is_empty() {
                        parts.push(o.to_player.join(", "));
                    }
                    if o.strength > 0 {
                        parts.push(format!("+{} Strength", o.strength));
                    }
                    let c = if o.damage > 0 { bad() } else { text_dim() };
                    ui.label(RichText::new(parts.join(" · ")).small().color(c));
                }
            });
            ui.end_row();
        }
    });
    if let Some(t) = tail(p).filter(|t| t != "…") {
        let t = t.strip_prefix("then ").unwrap_or(&t);
        ui.label(RichText::new(format!("Then {t}.")).small().color(text_faint()));
    }
}
