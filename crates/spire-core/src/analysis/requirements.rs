//! Cards that do nothing in this run: dead cards.
//!
//! **Needs.** Growth exhausts Sadness; with nothing that adds Sadness it
//! does nothing. Dualcast evokes an orb; with nothing that channels one,
//! likewise. The playbook lists these needs (`simulation.requirements`):
//! which cards need what, and which cards, relics or characters supply it.
//! Cards that answer junk the enemies add (exhausting Status cards) aren't
//! listed: a fight can supply what they need.
//!
//! **Locks.** Some relics forbid something for the rest of the run: Fiddle
//! (no draws during your turn), Sozu (no Potions), Ectoplasm (no Gold). A
//! card whose every effect a lock forbids (Acrobatics under Fiddle) does
//! nothing. One that keeps another effect (Pommel Strike's damage) isn't
//! dead; the simulator plays it without the locked part.
//!
//! A dead card is unplayable in the simulator, fills no role, and is never
//! suggested.

use std::collections::BTreeMap;

use crate::analysis::sim::{Effects, Reading};
use crate::catalog::{Card, Catalog};
use crate::ids::CardId;
use crate::mechanics::{Effect, Passive, When};
use crate::playbook::{Requirement, contains_ci};
use crate::run::RunState;

/// Why a card does nothing in this run.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Dead {
    /// It needs what nothing in the run supplies (`Requirement::what`).
    Needs(String),
    /// A relic forbids all it does: the relic's name and what it forbids.
    Locked { by: String, what: String },
}

impl Dead {
    /// A short label, e.g. "needs Sadness", "Fiddle: no draws".
    pub fn label(&self) -> String {
        match self {
            Dead::Needs(what) => format!("needs {what}"),
            Dead::Locked { by, .. } => format!("blocked by {by}"),
        }
    }

    /// Why, in a sentence.
    pub fn explain(&self) -> String {
        match self {
            Dead::Needs(what) => format!("Nothing in this run supplies {what}, so this card does nothing."),
            Dead::Locked { by, what } => format!("{by}: {what}. That's all this card does, so it does nothing."),
        }
    }
}

/// The run's deck cards that do nothing, and why: a need nothing supplies,
/// or a relic's lock on all the card does.
pub fn dead(state: &RunState, catalog: &Catalog, requirements: &[Requirement]) -> BTreeMap<CardId, Dead> {
    let cards: Vec<_> = state.deck.iter().filter_map(|d| catalog.cards.get(&d.card)).collect();
    let relic_text: Vec<&str> =
        state.relics.iter().filter_map(|r| catalog.relics.get(r)).map(|r| r.text.as_str()).collect();
    let mut out = BTreeMap::new();
    for req in requirements {
        let supplies = |c: &Card| req.met_by.as_ref().is_some_and(|m| m.matches(c));
        let met = state.character.as_ref().is_some_and(|c| req.characters.contains(c))
            || relic_text.iter().any(|t| req.relics.iter().any(|p| contains_ci(t, p)))
            || cards.iter().any(|c| supplies(c));
        if met {
            continue;
        }
        for c in cards.iter().filter(|c| req.needs.matches(c) && !supplies(c)) {
            out.entry(c.id.clone()).or_insert_with(|| Dead::Needs(req.what.clone()));
        }
    }
    for (lock, by) in locks(state, catalog) {
        for c in cards.iter().filter(|c| blocked(c, &lock, catalog)) {
            out.entry(c.id.clone()).or_insert_with(|| Dead::Locked { by: by.clone(), what: lock.describe() });
        }
    }
    out
}

/// The locks the run's relics put on it, with each relic's name.
pub fn locks(state: &RunState, catalog: &Catalog) -> Vec<(Passive, String)> {
    let orbs: Vec<_> = catalog.orbs.values().cloned().collect();
    state
        .relics
        .iter()
        .filter_map(|r| catalog.relics.get(r))
        .flat_map(|r| {
            crate::mechanics::read(&r.text, &orbs, false)
                .passives
                .into_iter()
                .filter(|p| matches!(p, Passive::NoDrawInTurn | Passive::NoGold | Passive::NoPotions))
                .map(|p| (p, r.name.clone()))
        })
        .collect()
}

/// Whether `lock` forbids everything `card` does.
pub fn blocked(card: &Card, lock: &Passive, catalog: &Catalog) -> bool {
    let r = crate::analysis::sim::reading(card, false, catalog);
    match lock {
        // It draws during the turn, and that's all it does.
        Passive::NoDrawInTurn => draws_in_turn(&r) && !does_besides_drawing(&r),
        // Its text gains what's locked, and it does nothing in a fight.
        Passive::NoGold => contains_ci(&card.text, "gold") && !does_besides_drawing(&r) && !draws_in_turn(&r),
        Passive::NoPotions => contains_ci(&card.text, "potion") && !does_besides_drawing(&r) && !draws_in_turn(&r),
        _ => false,
    }
}

/// It draws when played, or a trigger of its draws on an event (not at
/// the start of a turn, which a no-draws lock allows).
fn draws_in_turn(r: &Reading) -> bool {
    let starts_turn = |w: &When| {
        matches!(
            w,
            When::TurnStart
                | When::CombatStart
                | When::TurnNumber(_)
                | When::FirstTurns(_)
                | When::TurnsFrom(_)
                | When::EveryTurns(_)
        )
    };
    r.now.draw > 0
        || r.mech.triggers.iter().any(|(w, es)| !starts_turn(w) && es.iter().any(|e| matches!(e, Effect::Draw(_))))
}

/// Anything the card does in a fight other than draw (and discard, the
/// price of a draw like Acrobatics').
fn does_besides_drawing(r: &Reading) -> bool {
    let draw_only = |e: &Effect| matches!(e, Effect::Draw(_) | Effect::Discard(_));
    let rest = Effects { draw: 0, hits: 0, block_times: 0, ..r.now.clone() };
    let text = &r.text;
    rest != Effects::default()
        || r.x_stars
        || !r.channels.is_empty()
        || r.shoots
        || !text.makes.is_empty()
        || text.actions.iter().any(|e| !draw_only(e))
        || text.later.iter().any(|(_, es)| es.iter().any(|e| !draw_only(e)))
        || text.this_turn.iter().any(|(_, es)| es.iter().any(|e| !draw_only(e)))
        || text.scale.iter().any(|s| s.into != crate::mechanics::Scaled::Draw)
        // Battle Trance's "no more draws this turn" is about drawing too.
        || text.mods.iter().any(|m| !matches!(m, crate::mechanics::CardMod::DrawLock))
        || !text.stats.is_empty()
        || text.copy_to.is_some()
        || text.exhausts.is_some()
        || !r.mech.passives.is_empty()
        || r.mech.triggers.iter().any(|(_, es)| es.iter().any(|e| !draw_only(e)))
}
