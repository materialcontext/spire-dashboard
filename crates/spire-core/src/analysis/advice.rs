//! Macro advice: compares the deck against the act plan and the act boss.

use crate::analysis::deck::{Coverage, DeckProfile};
use crate::analysis::threats::{Scaling, ThreatPreview, preview};
use crate::catalog::Catalog;
use crate::playbook::{Playbook, RoleTarget};
use crate::run::RunState;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Urgent,
    Watch,
    Info,
}

/// One line of advice, short, with the reasons behind it for a hover.
#[derive(Clone, Debug, PartialEq)]
pub struct Advice {
    pub severity: Severity,
    pub text: String,
    /// The numbers and reasons, one per line.
    pub detail: String,
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

/// Short lines: the roles missing or thin for the act's targets (one line
/// each), what each boss wants that the deck lacks (one line per boss), and
/// removable curses.
pub fn advise(state: &RunState, catalog: &Catalog, playbook: &Playbook, deck: &DeckProfile) -> Vec<Advice> {
    let mut out = Vec::new();
    let Some(plan) = playbook.act(state.act_index) else { return out };
    let act = state.act_index + 1;

    let short: Vec<&RoleTarget> = plan.targets.iter().filter(|t| deck.role(t.role) < t.min).collect();
    let why = |t: &&RoleTarget| {
        let starter = match deck.starter_role(t.role) {
            0 => String::new(),
            n => format!(" ({n} starter, not counted)"),
        };
        format!("{}: {} of {} added cards{starter}", t.role.label(), deck.role(t.role), t.min)
    };
    let (missing, thin): (Vec<&RoleTarget>, Vec<&RoleTarget>) = short.iter().partition(|t| deck.role(t.role) == 0);
    if !missing.is_empty() {
        out.push(Advice {
            severity: Severity::Urgent,
            text: format!(
                "Missing for Act {act}: {}",
                missing.iter().map(|t| t.role.label()).collect::<Vec<_>>().join(", ")
            ),
            detail: missing.iter().map(why).collect::<Vec<_>>().join("\n"),
        });
    }
    if !thin.is_empty() {
        out.push(Advice {
            severity: Severity::Watch,
            text: format!(
                "Thin for Act {act}: {}",
                thin.iter()
                    .map(|t| format!("{} {}/{}", t.role.label(), deck.role(t.role), t.min))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            detail: thin.iter().map(why).collect::<Vec<_>>().join("\n"),
        });
    }
    for boss in boss_previews(state, catalog, playbook, deck) {
        let gaps: Vec<_> = boss.checks.iter().filter(|c| c.coverage != Coverage::Covered).collect();
        if gaps.is_empty() {
            continue;
        }
        out.push(Advice {
            severity: Severity::Urgent,
            text: format!(
                "{} wants {}",
                boss.name,
                gaps.iter().map(|c| c.demand.role.label()).collect::<Vec<_>>().join(", ")
            ),
            detail: gaps
                .iter()
                .map(|c| format!("{} ({}): you have {}", c.demand.role.label(), c.demand.reason, c.have))
                .collect::<Vec<_>>()
                .join("\n"),
        });
    }

    if deck.removable_curses > 0 {
        let n = deck.removable_curses;
        out.push(Advice {
            severity: Severity::Watch,
            text: if n == 1 { "1 removable curse".into() } else { format!("{n} removable curses") },
            detail: "Shops and events that remove cards are worth more.".into(),
        });
    }
    out.sort_by_key(|a| a.severity);
    out
}
