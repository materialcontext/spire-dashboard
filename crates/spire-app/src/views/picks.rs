//! What to look for in card rewards: lines within reach, then cards for
//! what the deck lacks against the boss. Shown only when there's any.

use eframe::egui::{RichText, Ui};
use spire_core::analysis::lines::{Suggestion, Why};
use spire_core::catalog::Record;

use crate::actions::{Action, Link};
use crate::app::ViewCtx;
use crate::style::color::*;
use crate::widgets::{caption, link, panel, secondary};

pub fn show(ui: &mut Ui, v: &ViewCtx, out: &mut Vec<Action>) -> bool {
    let picks = v.combat.map(|c| c.picks.as_slice()).unwrap_or_default();
    if picks.is_empty() {
        return false;
    }
    let note = v.state.boss.as_ref().map_or_else(|| "this act".to_string(), |b| format!("for {}", boss_name(v, b)));
    panel(ui, "Look for in card rewards", Some(&note), |ui| {
        for (i, s) in picks.iter().enumerate() {
            if i > 0 {
                ui.add_space(crate::style::space::s());
            }
            suggestion(ui, v, s, out);
        }
    });
    true
}

fn boss_name(v: &ViewCtx, id: &spire_core::EncounterId) -> String {
    v.catalog.encounters.get(id).map_or_else(|| id.to_string(), |e| e.name.clone())
}

fn pct(x: f32) -> String {
    format!("{:.0}%", x * 100.0)
}

fn won(r: Record) -> f32 {
    r.wins as f32 / r.fights.max(1) as f32
}

fn suggestion(ui: &mut Ui, v: &ViewCtx, s: &Suggestion, out: &mut Vec<Action>) {
    // The cards: held ones dim, the ones to look for as links.
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        for (i, (card, held)) in s.cards.iter().enumerate() {
            if i > 0 {
                ui.label(secondary("+"));
            }
            let name = v.catalog.card_name(card);
            if *held {
                ui.label(RichText::new(format!("{name} (have)")).color(text_dim()));
            } else {
                let text = RichText::new(name).family(crate::style::strong_family()).color(accent());
                if link(ui, text).on_hover_text("Open in Cards").clicked() {
                    out.push(Action::Open(Link::Card(card.clone())));
                }
            }
        }
    });
    let (all, fewer) = if s.cards.len() > 2 { ("all three", "only two") } else { ("both", "only one") };
    let character = v
        .state
        .character
        .as_ref()
        .and_then(|c| v.catalog.characters.get(c))
        .map_or_else(|| "the character".to_string(), |c| c.name.clone());
    match &s.why {
        Why::Line { together, base, apart, boss, .. } => {
            match boss {
                Some((b, r, _)) => ui.label(RichText::new(format!(
                    "Decks with {all} beat {} {} times in {}",
                    boss_name(v, b),
                    r.wins,
                    r.fights
                ))),
                None => ui.label(RichText::new(format!(
                    "Decks with {all} beat Act {}'s boss {} of the time",
                    v.state.act_index + 1,
                    pct(won(*together))
                ))),
            };
            ui.label(caption(format!(
                "{} with {all} at this act's bosses, {} with {fewer}, {} across {character}'s decks · {} decks",
                pct(won(*together)),
                pct(*apart),
                pct(*base),
                together.fights
            )))
            .on_hover_text(
                "Community runs, deck as it went into the boss fight. Rates with few decks are pulled \
                 toward the act's: a nudge, not a guarantee.",
            );
        }
        Why::Filler { role, boss, hp_before, hp_after, verdict_before, verdict_after, lift } => {
            let verdict = if verdict_after != verdict_before {
                format!("; {} instead of {}", verdict_after.label(), verdict_before.label())
            } else {
                String::new()
            };
            ui.label(RichText::new(format!(
                "{} against {}: about {hp_after} HP lost with it, {hp_before} without{verdict}",
                role.label(),
                boss_name(v, boss)
            )))
            .on_hover_text("The deck lacks this role against the boss; the simulator fights it with the card added.");
            if let Some(l) = lift {
                ui.label(caption(format!("High-skill players win {:+.0} points more with it ({} runs)", l.lift, l.n)))
                    .on_hover_text(
                        "Spire Codex: A10 players above a 50% win rate, against what they'd expect from the floor \
                         they took it on.",
                    );
            }
        }
    }
    // How often the missing cards turn up.
    let offers: Vec<String> = s
        .offered
        .iter()
        .filter(|(_, share)| *share > 0.0)
        .map(|(c, share)| format!("{} in 1 run in {:.0}", v.catalog.card_name(c), (1.0 / share).round().max(1.0)))
        .collect();
    if !offers.is_empty() {
        ui.label(caption(format!("Offered this act: {}", offers.join(", "))));
    }
}
