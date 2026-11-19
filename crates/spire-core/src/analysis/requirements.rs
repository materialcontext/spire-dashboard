//! Cards that do nothing without something the deck must supply.
//!
//! Growth exhausts Sadness; with nothing that adds Sadness it does nothing.
//! Dualcast evokes an orb; with nothing that channels one, likewise. The
//! playbook lists these needs (`simulation.requirements`): which cards need
//! what, and which cards, relics or characters supply it. A card whose need
//! nothing in the run supplies is a dead card: unplayable in the simulator,
//! and filling no role. Cards that answer junk the enemies add (exhausting
//! Status cards) aren't listed: a fight can supply what they need.

use std::collections::BTreeMap;

use crate::catalog::Catalog;
use crate::ids::CardId;
use crate::playbook::{Requirement, contains_ci};
use crate::run::RunState;

/// The run's deck cards whose need nothing supplies, with what they need.
pub fn unmet(state: &RunState, catalog: &Catalog, requirements: &[Requirement]) -> BTreeMap<CardId, String> {
    let cards: Vec<_> = state.deck.iter().filter_map(|d| catalog.cards.get(&d.card)).collect();
    let relic_text: Vec<&str> =
        state.relics.iter().filter_map(|r| catalog.relics.get(r)).map(|r| r.text.as_str()).collect();
    let mut out = BTreeMap::new();
    for req in requirements {
        let supplies = |c: &crate::catalog::Card| req.met_by.as_ref().is_some_and(|m| m.matches(c));
        let met = state.character.as_ref().is_some_and(|c| req.characters.contains(c))
            || relic_text.iter().any(|t| req.relics.iter().any(|p| contains_ci(t, p)))
            || cards.iter().any(|c| supplies(c));
        if met {
            continue;
        }
        for c in cards.iter().filter(|c| req.needs.matches(c) && !supplies(c)) {
            out.entry(c.id.clone()).or_insert_with(|| req.what.clone());
        }
    }
    out
}
