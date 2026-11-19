//! Relic lookout: event relics crucial to this deck that an event still
//! possible this act can give, and so worth steering into `?` rooms for.
//!
//! A relic is crucial when the deck meets one of the playbook's
//! `relic_fits` (Lost Wisp with 4+ Powers). Where it can come from is data:
//! the events that name it in an option, or that community runs saw give
//! it, with how often runs meet each event in the act and whether it's
//! given outright or one of several at random.
//!
//! Left out: relics already held, events already seen this run (they don't
//! repeat), and events the act doesn't allow.

use crate::analysis::deck::ClassifiedCard;
use crate::catalog::{Catalog, Event, EventRelic};
use crate::ids::{ActId, EventId, RelicId};
use crate::playbook::{Playbook, RelicFit};
use crate::run::RunState;

/// A relic worth fishing for, and where to find it.
#[derive(Clone, Debug, PartialEq)]
pub struct Lookout {
    pub relic: RelicId,
    pub name: String,
    /// What the relic does.
    pub text: String,
    /// "5 Powers: each deals 8 damage to ALL enemies when played."
    pub why: String,
    /// The events that give it, likeliest first.
    pub sources: Vec<Source>,
}

/// An event that gives a relic.
#[derive(Clone, Debug, PartialEq)]
pub struct Source {
    pub event: EventId,
    pub name: String,
    /// The option that gives it.
    pub option: Option<String>,
    /// That option's text (its cost).
    pub option_text: Option<String>,
    /// Given outright (1), or one of this many at random.
    pub one_of: u32,
    /// Share of community runs that met the event in this act.
    pub met: Option<f32>,
    /// What the game checks before offering it.
    pub requires: Vec<String>,
}

impl Source {
    /// Chance of getting the relic in this act through this event, when
    /// community runs say how often it comes up.
    pub fn chance(&self) -> Option<f32> {
        self.met.map(|m| m / self.one_of.max(1) as f32)
    }
}

/// Deck cards (dead ones left out) that a fit counts.
fn count(fit: &RelicFit, deck: &[ClassifiedCard]) -> u32 {
    deck.iter().filter(|c| c.unmet.is_none() && c.card.is_some_and(|card| fit.cards.matches(card))).count() as u32
}

fn source(event: &Event, given: &EventRelic, act: &ActId) -> Source {
    let option = given.option.as_ref().and_then(|t| event.options.iter().find(|o| &o.title == t));
    Source {
        event: event.id.clone(),
        name: event.name.clone(),
        option: given.option.clone(),
        option_text: option.map(|o| o.text.clone()),
        one_of: given.one_of,
        met: event.met.get(act).copied(),
        requires: event.requires.clone(),
    }
}

/// Relics to fish for in this act, likeliest first. Empty when nothing
/// crucial is on offer, which is most of the time.
pub fn lookout(state: &RunState, deck: &[ClassifiedCard], catalog: &Catalog, playbook: &Playbook) -> Vec<Lookout> {
    let Some(act) = &state.act else { return Vec::new() };
    let events: Vec<&Event> = catalog.events_in(act).filter(|e| !state.events_seen.contains(&e.id)).collect();
    let mut out: Vec<Lookout> = Vec::new();
    for fit in &playbook.relic_fits {
        if state.relics.contains(&fit.relic) || out.iter().any(|l| l.relic == fit.relic) {
            continue;
        }
        let n = count(fit, deck);
        if n < fit.at_least {
            continue;
        }
        let mut sources: Vec<Source> = events
            .iter()
            .flat_map(|e| e.relics.iter().filter(|r| r.relic == fit.relic).map(move |r| source(e, r, act)))
            .collect();
        if sources.is_empty() {
            continue;
        }
        sources.sort_by(|a, b| b.chance().unwrap_or(0.0).total_cmp(&a.chance().unwrap_or(0.0)));
        let relic = catalog.relics.get(&fit.relic);
        out.push(Lookout {
            relic: fit.relic.clone(),
            name: relic.map_or_else(|| fit.relic.to_string(), |r| r.name.clone()),
            text: relic.map(|r| r.text.clone()).unwrap_or_default(),
            why: format!("{n} {}: {}", fit.what, fit.why),
            sources,
        });
    }
    let best = |l: &Lookout| l.sources.iter().filter_map(Source::chance).fold(0.0, f32::max);
    out.sort_by(|a, b| best(b).total_cmp(&best(a)));
    out
}
