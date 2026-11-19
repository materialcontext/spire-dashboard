//! Setup and resource balance: what a deck makes against what it uses.
//!
//! Aim, Vigor and Stretch last until used, across turns, so their worth is
//! the cards that use them: Shoot cards for Aim, attacks for Vigor, Block
//! cards for Stretch. The simulator values them that way; this reads its
//! per-turn flows ([`DeckSim::stores`]) against the deck's cards and warns
//! when:
//! - **setup piles up:** much more is made than used, many makers share
//!   few users (it waits turns to be cashed in), or nothing uses it;
//! - **Arrows run short** (the Fool): Shoot cards wait in hand with no Arrow.
//!   A shot Arrow returns after 3 attack hits (each hit of a multi-hit
//!   attack counts) or a Dislodge;
//! - **Stars run short** (the Regent): Star cards wait for Stars; or Stars
//!   are made with nothing in the deck to spend them on.
//!
//! The lines are the playbook's (`balance`).

use crate::analysis::sim::{DeckSim, Store};
use crate::catalog::{Card, CardType, Catalog};
use crate::playbook::{Playbook, contains_ci};
use crate::run::{DeckCard, RunState};

/// A store out of balance, in words, with the numbers behind it.
#[derive(Clone, Debug, PartialEq)]
pub struct Warning {
    pub store: Store,
    pub text: String,
}

/// Cards that make a store, and cards that use it.
fn makes(store: Store, c: &Card, up: bool) -> bool {
    let any = |keys: &[&str]| keys.iter().any(|k| c.value(k, up) > 0);
    match store {
        Store::Aim => any(&["aim", "aimpower"]),
        Store::Vigor => any(&["vigor", "vigorpower"]),
        Store::Stretch => any(&["stretch", "stretchpower"]),
        Store::Arrows => contains_ci(&c.text, "dislodge") || contains_ci(&c.text, "additional arrow"),
        Store::Stars => c.value("stars", up) > 0 && contains_ci(c.text_at(up), "star"),
    }
}

fn uses(store: Store, c: &Card, up: bool) -> bool {
    let shoots = contains_ci(c.text_at(up), "shoot an arrow");
    match store {
        Store::Aim | Store::Arrows => shoots,
        Store::Vigor => c.card_type == CardType::Attack && c.value("damage", up) > 0,
        Store::Stretch => c.value("block", up) > 0,
        Store::Stars => c.star_cost.is_some_and(|n| n > 0) || contains_ci(c.text_at(up), "spend"),
    }
}

fn count(deck: &[DeckCard], catalog: &Catalog, f: impl Fn(&Card, bool) -> bool) -> usize {
    deck.iter().filter(|d| catalog.cards.get(&d.card).is_some_and(|c| f(c, d.upgraded))).count()
}

fn plural(n: usize, one: &str) -> String {
    if n == 1 { format!("1 {one}") } else { format!("{n} {one}s") }
}

/// The run's balance warnings, from its deck's simulation.
pub fn warnings(sim: &DeckSim, state: &RunState, catalog: &Catalog, playbook: &Playbook) -> Vec<Warning> {
    let deck = &state.deck;
    let lines = &playbook.balance;
    let mut out = Vec::new();
    for store in [Store::Aim, Store::Vigor, Store::Stretch] {
        let f = sim.stores[store as usize];
        let makers = count(deck, catalog, |c, up| makes(store, c, up));
        if makers == 0 || f.gained < lines.min_gained {
            continue;
        }
        let users = count(deck, catalog, |c, up| uses(store, c, up));
        let (maker, user) = match store {
            Store::Aim => ("Aim card", "Shoot card"),
            Store::Vigor => ("Vigor card", "attack"),
            _ => ("Stretch card", "Block card"),
        };
        let text = if users == 0 {
            format!("{} and no {user}s: the {} is never used", plural(makers, maker), store.label())
        } else if f.spent < f.gained * lines.pile_up || makers as f32 > users as f32 * lines.makers_per_user {
            format!(
                "{} piles up: {} for {}; about {:.1} made a turn, {:.1} used",
                store.label(),
                plural(makers, maker),
                plural(users, user),
                f.gained,
                f.spent
            )
        } else {
            continue;
        };
        out.push(Warning { store, text });
    }

    let arrows = sim.stores[Store::Arrows as usize];
    let wanted = arrows.plays + arrows.waiting;
    if wanted > 0.0 && arrows.waiting / wanted >= lines.waiting {
        let shooters = count(deck, catalog, |c, up| uses(Store::Arrows, c, up));
        let slots =
            state.character.as_ref().and_then(|c| catalog.characters.get(c)).and_then(|c| c.arrow_slots).unwrap_or(0);
        out.push(Warning {
            store: Store::Arrows,
            text: format!(
                "about 1 shot in {:.0} waits for an Arrow ({}, {}, {:.1} attack hits a turn; a shot \
                 Arrow returns after {} hits)",
                (wanted / arrows.waiting).round().max(1.0),
                plural(shooters, "Shoot card"),
                plural(slots as usize, "Arrow slot"),
                sim.play.hits,
                playbook.simulation.arrow_return_hits
            ),
        });
    }

    let stars = sim.stores[Store::Stars as usize];
    let makers = count(deck, catalog, |c, up| makes(Store::Stars, c, up));
    let users = count(deck, catalog, |c, up| uses(Store::Stars, c, up));
    let wanted = stars.plays + stars.waiting;
    if wanted > 0.0 && stars.waiting / wanted >= lines.waiting {
        out.push(Warning {
            store: Store::Stars,
            text: format!(
                "Star cards wait for Stars: about {:.1} made a turn, {:.1} spent ({}, {})",
                stars.gained,
                stars.spent,
                plural(makers, "card that makes them"),
                plural(users, "card that spends them")
            ),
        });
    } else if users == 0 && stars.gained >= lines.min_gained {
        out.push(Warning {
            store: Store::Stars,
            text: format!(
                "about {:.1} Stars made a turn and nothing in the deck spends them ({})",
                stars.gained,
                plural(makers, "card that makes them")
            ),
        });
    }
    out
}
