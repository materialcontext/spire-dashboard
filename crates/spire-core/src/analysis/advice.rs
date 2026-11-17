//! Macro advice: compares the deck against the act plan and the act boss.

use crate::analysis::deck::{Coverage, DeckProfile};
use crate::analysis::threats::{ThreatPreview, preview};
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

/// The act boss as a threat preview, if known.
pub fn boss_preview(
    state: &RunState,
    catalog: &Catalog,
    playbook: &Playbook,
    deck: &DeckProfile,
) -> Option<ThreatPreview> {
    let enc = catalog.encounters.get(state.boss.as_ref()?)?;
    Some(preview(enc, catalog, playbook, state.act_index, deck, false))
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

    if let Some(boss) = boss_preview(state, catalog, playbook, deck) {
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

    if deck.of_type(crate::catalog::CardType::Curse) > 0 {
        out.push(Advice {
            severity: Severity::Watch,
            text: "Curses in deck: value shops and events that remove cards.".into(),
        });
    }
    out.extend(plan.notes.iter().map(|n| Advice { severity: Severity::Info, text: n.clone() }));
    out.sort_by_key(|a| a.severity);
    out
}
