//! What enemies do turn by turn, when runs end against them, and whether
//! the deck is on track to survive.

use eframe::egui::{self, RichText, Ui};
use spire_core::analysis::pattern::{MoveOption, Plan, Turn};
use spire_core::analysis::survival::{Outlook, Verdict};
use spire_core::analysis::threats::MonsterPlan;
use spire_core::catalog::EncounterStats;

use crate::style::color::*;
use crate::style::role_color;
use crate::widgets::{caption, chip, secondary};

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

/// One chip of a strip: label, whether runs often end on that turn, and
/// which turn of the plan it shows (for the tooltip).
#[derive(Clone)]
struct Chip {
    label: String,
    marked: bool,
    turn: usize,
}

/// One monster's line of chips, laid out once per dashboard generation.
#[derive(Clone)]
struct Line {
    plan: usize,
    name: Option<String>,
    /// "Any of: …" for a monster with no known order.
    any: Option<String>,
    chips: Vec<Chip>,
    tail: Option<String>,
}

fn lines(plans: &[MonsterPlan], stats: Option<&EncounterStats>) -> Vec<Line> {
    let deadly = deadly(stats);
    let named = plans.len() > 1;
    let mut out = Vec::new();
    for (pi, mp) in plans.iter().enumerate() {
        if mp.plan.turns.is_empty() {
            continue;
        }
        let name = named.then(|| if mp.count > 1 { format!("{}×{}", mp.name, mp.count) } else { mp.name.clone() });
        if mp.plan.unknown {
            let names: Vec<&str> = mp.plan.turns[0].options.iter().map(|o| o.name.as_str()).collect();
            let any = Some(format!("Any of: {}", names.join(", ")));
            out.push(Line { plan: pi, name, any, chips: Vec::new(), tail: None });
            continue;
        }
        let shown = mp.plan.shown();
        // Only a single-monster plan maps deaths to its own turns.
        let marked: Vec<usize> = if named {
            Vec::new()
        } else {
            deadly.iter().filter_map(|(d, _)| shown_index(&mp.plan, *d as usize)).collect()
        };
        let mut chips = Vec::new();
        let mut collapsed = false;
        for (i, t) in shown.iter().take(STRIP_TURNS).enumerate() {
            let n = i + 1;
            // The same pick every turn from here on: show it once.
            let rest_same = i + 1 < shown.len() && shown[i..].iter().all(|x| same_options(x, t));
            let plus = if rest_same && !t.known() { "+" } else { "" };
            chips.push(Chip { label: format!("{n}{plus} {}", turn_text(t)), marked: marked.contains(&i), turn: i });
            if !plus.is_empty() {
                collapsed = true;
                break;
            }
        }
        let mut tail = tail(&mp.plan).filter(|_| !collapsed);
        if mp.death_blow {
            let blow = "explodes the turn after it dies".to_string();
            tail = Some(match tail {
                Some(t) => format!("{t}; {blow}"),
                None => blow,
            });
        }
        out.push(Line { plan: pi, name, any: None, chips, tail });
    }
    out
}

/// One line of turn chips per monster type. Turns where many runs end
/// are drawn in the danger colour. The labels are built once per `key`
/// (pass the dashboard generation and the encounter).
pub fn strip(
    ui: &mut Ui,
    key: impl std::hash::Hash + std::fmt::Debug,
    plans: &[MonsterPlan],
    stats: Option<&EncounterStats>,
) {
    let id = egui::Id::unique(("turn-strip", key));
    let built: std::sync::Arc<Vec<Line>> =
        ui.ctx().data_mut(|d| d.get_temp_mut_or_insert_with(id, || std::sync::Arc::new(lines(plans, stats))).clone());
    for line in built.iter() {
        let Some(mp) = plans.get(line.plan) else { continue };
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(4.0, 4.0);
            if let Some(name) = &line.name {
                ui.label(secondary(name));
            }
            if let Some(any) = &line.any {
                ui.label(secondary(any));
            }
            for c in &line.chips {
                let color = if c.marked { bad() } else { text_dim() };
                let r = chip(ui, c.label.as_str(), color);
                if let Some(t) = mp.plan.turns.get(c.turn) {
                    r.on_hover_ui(|ui| {
                        ui.label(turn_detail(c.turn + 1, t));
                    });
                }
            }
            if let Some(tail) = &line.tail {
                ui.label(caption(tail));
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

/// "STEAM_ERUPTION" reads "Steam eruption".
pub fn power_name(id: &str) -> String {
    spire_core::analysis::pattern::power_words(id)
}

/// "1 in 5 runs that reach this fight end here, most on turn 7 (Dismember)."
pub fn deaths(
    ui: &mut Ui,
    key: impl std::hash::Hash + std::fmt::Debug,
    plans: &[MonsterPlan],
    stats: Option<&EncounterStats>,
) {
    let Some(s) = stats.filter(|s| s.deaths > 0 && s.fights > 0) else { return };
    let rate = s.death_rate();
    let id = egui::Id::unique(("deaths-line", key));
    let text: std::sync::Arc<str> = ui.ctx().data_mut(|d| {
        d.get_temp_mut_or_insert_with::<std::sync::Arc<str>>(id, || deaths_text(plans, s).into()).clone()
    });
    ui.label(RichText::new(&*text).small().color(if rate >= 0.1 { bad() } else { text_dim() })).on_hover_ui(|ui| {
        ui.label(format!("{} of {} community fights (Spire Codex) ended the run.", s.deaths, s.fights));
        let shares: Vec<String> =
            deadly(Some(s)).iter().take(4).map(|(t, share)| format!("turn {t}: {:.0}%", share * 100.0)).collect();
        if !shares.is_empty() {
            ui.label(format!("Deaths by turn: {}.", shares.join(", ")));
        }
        if let Some((won, died)) = s.hp_in {
            ui.label(format!("Players who won came in with {won} HP (median); those who died, {died}."));
        }
    });
}

/// "Ends 20% of runs · most on turn 7 (Lunging Bite)". The shares and
/// counts are in the tooltip.
fn deaths_text(plans: &[MonsterPlan], s: &EncounterStats) -> String {
    let turns: Vec<String> = deadly(Some(s))
        .iter()
        .take(2)
        .map(|(t, _)| {
            // A monster that explodes after dying can also kill on the turn
            // after its death.
            let blast = plans.iter().any(|p| p.death_blow) && *t > 1;
            match (move_on(plans, *t), blast) {
                (Some(m), true) => format!("turn {t} ({m} or its blast)"),
                (None, true) => format!("turn {t} (or its blast)"),
                (Some(m), false) => format!("turn {t} ({m})"),
                (None, false) => format!("turn {t}"),
            }
        })
        .collect();
    let mut text = format!("Ends {:.0}% of runs", s.death_rate() * 100.0);
    if !turns.is_empty() {
        text.push_str(&format!(" · most on {}", turns.join(" and ")));
    }
    text
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
        ui.label(RichText::new(o.lead.way.label()).small().strong());
        let l = &o.lead;
        let text = match (l.kill_turn, l.blast_turn) {
            (Some(t), Some(b)) if l.hp_loss as i32 >= o.hp => format!(
                "kill on about turn {t}, but its blast on turn {b} brings the loss to about {} HP; you have {}",
                l.hp_loss, o.hp
            ),
            (Some(t), Some(b)) => format!(
                "kill on about turn {t} and survive its blast on turn {b}, losing about {} of your {} HP",
                l.hp_loss, o.hp
            ),
            (Some(t), None) if l.hp_loss as i32 >= o.hp => {
                format!("would lose about {} HP over {t} turns; you have {}", l.hp_loss, o.hp)
            }
            (Some(t), None) => format!("win in about {t} turns, losing about {} of your {} HP", l.hp_loss, o.hp),
            (None, _) => format!("no win within 20 turns; about {} HP lost by then", l.hp_loss),
        };
        ui.label(secondary(text));
    })
    .response
    .on_hover_ui(|ui| outlook_detail(ui, o, stats));
    if !o.limits.is_empty() {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            ui.label(caption("Would help:"));
            for limit in &o.limits {
                let color = limit.role.map_or_else(crate::style::tag_color, role_color);
                chip(ui, limit.label(), color).on_hover_text(look_for(&limit.text, &limit.tags));
            }
        });
    }
}

/// "spikes of 40…" then "Look for: Block, Weak" on its own line.
fn look_for(text: &str, tags: &[spire_core::analysis::tags::Tag]) -> String {
    if tags.is_empty() {
        return text.to_string();
    }
    let names: Vec<&str> = tags.iter().map(|t| t.label()).collect();
    format!("{text}\nLook for: {}", names.join(", "))
}

/// The outlook's hover: each way played, the rules, joins, junk and blast.
fn outlook_detail(ui: &mut Ui, o: &Outlook, stats: Option<&EncounterStats>) {
    ui.set_max_width(380.0);
    ui.label(format!("Lead with {}: {}.", o.lead.way.label(), o.lead.way.describe()));
    for l in &o.ways {
        let result = match l.kill_turn {
            Some(t) => format!("win on turn {t}, losing about {} HP", l.hp_loss),
            None => format!("no win within 20 turns, about {} HP lost", l.hp_loss),
        };
        ui.label(secondary(format!("{}: {result}", l.way.label())));
    }
    for r in &o.rules {
        ui.label(look_for(&format!("{}.", r.text), &r.tags));
    }
    for j in &o.joins {
        let who = if j.count == 1 { j.name.clone() } else { format!("{} {}s", j.count, j.name) };
        ui.label(format!(
            "Then {who} join ({}), fought once the ones before fall. The data says the power \
             summons but not how many, so its amount is read as the count.",
            spire_core::analysis::pattern::power_words(&j.power)
        ));
    }
    if o.status_cards > 0 && o.lead.junk >= 0.5 {
        ui.label(format!(
            "Its moves add about {:.0} unplayable cards to your deck by then, thinning your draws. \
             Status moves count as {} each: the game data marks the move but not how many cards.",
            o.lead.junk, o.status_cards
        ));
    }
    if let Some(b) = o.lead.blast_turn {
        ui.label(format!(
            "It explodes at the end of your turn after it dies, so the fight costs one more enemy turn \
             than the kill: what matters is killing it a turn sooner. The blast is counted as at least \
             its starting size and that turn's usual hit (turn {b}); it grows over the fight."
        ));
    }
    if let Some((median, p90)) = stats.and_then(|s| s.win_damage) {
        ui.label(format!("Players who won lost {median} HP (median); 1 in 10 lost {p90} or more."));
    }
    ui.label(caption(
        "From the deck simulator's average damage and block per turn against the enemies' \
             expected moves and rules. Leaves out debuffs on you, enemies dying early, overkill, and potions.",
    ));
}

/// Every monster's turns in full, for the Enemies tab.
pub fn table(ui: &mut Ui, mp: &MonsterPlan, stats: Option<&EncounterStats>, single: bool) {
    let p = &mp.plan;
    if p.unknown || p.turns.is_empty() {
        ui.label(caption("Move order unknown: any of its moves, any turn."));
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
                    ui.add(egui::Label::new(RichText::new(name).small().color(text())).extend());
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
        ui.label(caption(format!("Then {t}.")));
    }
}
