//! Macro advice: what the act's bosses ask for that the deck lacks
//! entirely, removable curses, and the deck's composition warnings.

use crate::analysis::composition::Composition;
use crate::analysis::deck::{Coverage, DeckProfile};
use crate::analysis::threats::{Scaling, ThreatPreview, preview};
use crate::catalog::Catalog;
use crate::catalog::Measure;
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

/// What each boss wants that the deck lacks, one line per boss, from its
/// demands checked against the simulated deck.
pub fn boss_gaps(bosses: &[ThreatPreview]) -> Vec<Advice> {
    bosses
        .iter()
        .filter_map(|boss| {
            let gaps: Vec<_> = boss.checks.iter().filter(|c| c.coverage == Coverage::Missing).collect();
            (!gaps.is_empty()).then(|| Advice {
                severity: Severity::Urgent,
                text: format!(
                    "{} wants {}",
                    boss.name,
                    gaps.iter().map(|c| c.demand.role.label()).collect::<Vec<_>>().join(", ")
                ),
                detail: gaps
                    .iter()
                    .map(|c| format!("{}: {}", c.demand.role.label(), c.detail()))
                    .collect::<Vec<_>>()
                    .join("\n"),
            })
        })
        .collect()
}

/// Short lines the deck alone tells: removable curses. Boss gaps
/// ([`boss_gaps`]) and composition warnings ([`composition`]) come once the
/// deck has been simulated.
pub fn advise(deck: &DeckProfile) -> Vec<Advice> {
    let mut out = Vec::new();
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

/// One line per composition warning, the biggest win-rate gap first. A
/// role's job and its share of the deck are two views of one problem: only
/// the bigger gap of the two is a line.
pub fn composition(c: &Composition) -> Vec<Advice> {
    let role = |m: Measure| match m {
        Measure::Job(r) | Measure::Share(r) => Some(r),
        _ => None,
    };
    let mut seen = Vec::new();
    c.flags()
        .into_iter()
        .filter(|row| match role(row.measure) {
            Some(r) if seen.contains(&r) => false,
            Some(r) => {
                seen.push(r);
                true
            }
            None => true,
        })
        .map(|row| Advice {
            severity: Severity::Watch,
            text: row.flag.map(|f| crate::analysis::composition::phrase(row.measure, f.side)).unwrap_or_default(),
            detail: format!("{}\n{}", row.numbers(), row.evidence().unwrap_or_default()),
        })
        .collect()
}
