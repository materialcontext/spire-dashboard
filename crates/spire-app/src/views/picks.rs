//! What to look for in card rewards: lines within reach, then cards for
//! what the deck lacks against the boss. Shown only when there's any.
//!
//! Each entry is one line of cards and a short label; the numbers behind it
//! are on the label's hover, and each card's details on its own.

use eframe::egui::{RichText, Ui};
use spire_core::analysis::lines::{Suggestion, Why};
use spire_core::ids::CardId;

use crate::actions::{Action, Link};
use crate::app::ViewCtx;
use crate::style::color::*;
use crate::widgets::{caption, link, panel, secondary};

pub fn show(ui: &mut Ui, v: &ViewCtx, out: &mut Vec<Action>) -> bool {
    let picks = v.combat.map(|c| c.picks.as_slice()).unwrap_or_default();
    if picks.is_empty() {
        return false;
    }
    panel(ui, "Look for in card rewards", None, |ui| {
        for (i, s) in picks.iter().enumerate() {
            if i > 0 {
                ui.add_space(crate::style::space::xs());
            }
            suggestion(ui, v, s, out);
        }
    });
    true
}

fn boss_name(v: &ViewCtx, id: &spire_core::EncounterId) -> String {
    v.catalog.encounters.get(id).map_or_else(|| id.to_string(), |e| e.name.clone())
}

/// A card's details on hover, as in the deck panel.
fn tip(ui: &mut Ui, v: &ViewCtx, id: &CardId) {
    ui.set_max_width(320.0);
    let (roles, tags) = v.catalog.cards.get(id).map_or_else(Default::default, |c| {
        (v.playbook.roles_of(c), spire_core::analysis::tags::tags(c, false, v.playbook))
    });
    crate::views::deck::card_tip(ui, v, id, false, &roles, &tags);
}

fn suggestion(ui: &mut Ui, v: &ViewCtx, s: &Suggestion, out: &mut Vec<Action>) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        for id in &s.held {
            ui.label(RichText::new(v.catalog.card_name(id)).color(text_dim())).on_hover_ui(|ui| tip(ui, v, id));
            ui.label(secondary("+"));
        }
        // Options: any one completes it; a filler's option may be a card and
        // its partner.
        let n = s.options.len();
        for (i, option) in s.options.iter().enumerate() {
            if i > 0 {
                ui.label(secondary(if i + 1 == n { "or" } else { "," }));
            }
            for (j, id) in option.iter().enumerate() {
                if j > 0 {
                    ui.label(secondary("+"));
                }
                let text = RichText::new(v.catalog.card_name(id)).family(crate::style::strong_family()).color(accent());
                if link(ui, text).on_hover_ui(|ui| tip(ui, v, id)).clicked() {
                    out.push(Action::Open(Link::Card(id.clone())));
                }
            }
        }
    });
    let label = match &s.why {
        Why::Line { boss: Some((b, _, _)), .. } => format!("Synergy vs {}", boss_name(v, b)),
        Why::Line { .. } => format!("Synergy for Act {}'s boss", v.state.act_index + 1),
        Why::Filler { role, boss, .. } => format!("{} vs {}", role.label(), boss_name(v, boss)),
    };
    ui.label(caption(label)).on_hover_ui(|ui| detail(ui, v, s));
}

/// The numbers behind a suggestion, on hover.
fn detail(ui: &mut Ui, v: &ViewCtx, s: &Suggestion) {
    ui.set_max_width(320.0);
    let pct = |x: f32| format!("{:.0}%", x * 100.0);
    match &s.why {
        Why::Line { together, base, apart, boss, .. } => {
            if let Some((b, r, _)) = boss {
                ui.label(format!("Won {} of {} fights against {}", r.wins, r.fights, boss_name(v, b)));
            }
            let rate = together.wins as f32 / together.fights.max(1) as f32;
            ui.label(format!("{} at this act's bosses ({} decks)", pct(rate), together.fights));
            ui.label(secondary(format!("{} with part of it, {} for all decks", pct(*apart), pct(*base))));
        }
        Why::Filler { hp_before, hp_after, lift, .. } => {
            ui.label(format!("About {hp_after} HP lost to the boss with it, {hp_before} without"));
            if let Some(l) = lift {
                ui.label(secondary(format!("{:+.0} win-rate points for high-skill players", l.lift)));
            }
        }
    }
    let offers: Vec<String> = s
        .offered
        .iter()
        .filter(|(_, share)| *share > 0.0)
        .map(|(c, share)| format!("{} {}", v.catalog.card_name(c), pct(*share)))
        .collect();
    if !offers.is_empty() {
        ui.label(secondary(format!("Offered this act: {}", offers.join(", "))));
    }
}
