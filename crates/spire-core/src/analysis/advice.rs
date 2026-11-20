//! Macro advice: what the act's bosses ask for that the deck lacks
//! entirely, removable curses, and the deck's composition warnings.

use crate::analysis::composition::Composition;
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

/// Short lines: what each boss wants that the deck has no card for (one
/// line per boss), and removable curses. Composition warnings come from
/// [`composition`], once the deck has been simulated.
pub fn advise(state: &RunState, catalog: &Catalog, playbook: &Playbook, deck: &DeckProfile) -> Vec<Advice> {
    let mut out = Vec::new();
    for boss in boss_previews(state, catalog, playbook, deck) {
        let gaps: Vec<_> = boss.checks.iter().filter(|c| c.coverage == Coverage::Missing).collect();
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

/// One line per composition warning, the biggest win-rate gap first.
pub fn composition(c: &Composition) -> Vec<Advice> {
    c.flags()
        .into_iter()
        .map(|row| Advice {
            severity: Severity::Watch,
            text: row.summary(),
            detail: row.evidence().unwrap_or_default(),
        })
        .collect()
}
