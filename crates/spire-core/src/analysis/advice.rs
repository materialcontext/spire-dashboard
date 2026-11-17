//! Macro advice: compares the deck against the act plan and the act boss.

use crate::analysis::deck::{Coverage, DeckProfile};
use crate::analysis::threats::{Scaling, ThreatPreview, preview};
use crate::catalog::Catalog;
use crate::playbook::Playbook;
use crate::run::RunState;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Urgent,
    Watch,
    Info,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Advice {
    pub severity: Severity,
    pub text: String,
}

/// The act's known bosses as threat previews (two at A10 in act 3).
pub fn boss_previews(
    state: &RunState,
    catalog: &Catalog,
    playbook: &Playbook,
    deck: &DeckProfile,
) -> Vec<ThreatPreview> {
    let scaling = Scaling::for_run(state, catalog);
    state
        .bosses()
        .filter_map(|b| catalog.encounters.get(b))
        .map(|enc| preview(enc, catalog, playbook, state.act_index, scaling, deck, false))
        .collect()
}

pub fn advise(state: &RunState, catalog: &Catalog, playbook: &Playbook, deck: &DeckProfile) -> Vec<Advice> {
    let mut out = Vec::new();
    let Some(plan) = playbook.act(state.act_index) else { return out };
    let act = state.act_index + 1;

    for target in &plan.targets {
        let have = deck.role(target.role);
        if have < target.min {
            out.push(Advice {
                severity: if have == 0 { Severity::Urgent } else { Severity::Watch },
                text: format!(
                    "{}: {have}/{} added cards by the end of Act {act} (starter cards not counted{}).",
                    target.role.label(),
                    target.min,
                    match deck.starter_role(target.role) {
                        0 => String::new(),
                        n => format!("; {n} starter"),
                    }
                ),
            });
        }
    }
    if plan.needs_aoe > 0 && deck.aoe < plan.needs_aoe {
        out.push(Advice {
            severity: Severity::Watch,
            text: format!("Area damage: {}/{} for multi-enemy fights.", deck.aoe, plan.needs_aoe),
        });
    }

    for boss in boss_previews(state, catalog, playbook, deck) {
        for check in boss.checks.iter().filter(|c| c.coverage != Coverage::Covered) {
            out.push(Advice {
                severity: Severity::Urgent,
                text: format!(
                    "{} wants {} ({}): you have {}.",
                    boss.name,
                    check.demand.capability.label(),
                    check.demand.reason,
                    check.have
                ),
            });
        }
    }

    if deck.removable_curses > 0 {
        out.push(Advice {
            severity: Severity::Watch,
            text: format!(
                "{} removable curse(s) in deck: value shops and events that remove cards.",
                deck.removable_curses
            ),
        });
    }
    let mut permanent: Vec<String> = state
        .deck
        .iter()
        .filter_map(|d| catalog.cards.get(&d.card))
        .filter(|c| c.is_eternal() && c.card_type == crate::catalog::CardType::Curse)
        .map(|c| c.name.clone())
        .collect();
    permanent.dedup();
    if !permanent.is_empty() {
        out.push(Advice {
            severity: Severity::Info,
            text: format!(
                "{} can't be removed or transformed: plan around the dead draw (more draw, Retain, smaller deck).",
                permanent.join(", ")
            ),
        });
    }
    out.extend(plan.notes.iter().map(|n| Advice { severity: Severity::Info, text: n.clone() }));
    out.sort_by_key(|a| a.severity);
    out
}
