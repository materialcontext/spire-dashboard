//! Setup and resource balance on real Fool and Regent cards.

use spire_core::analysis::balance::warnings;
use spire_core::analysis::sim::{Store, simulate};
use spire_core::ids::{CardId, CharacterId};
use spire_core::run::{DeckCard, RunEvent, RunState, apply};

fn state(character: &str, cards: &[(&str, usize)]) -> RunState {
    let cat = spire_data::bundled_catalog();
    let start = RunEvent::start(&cat, &CharacterId::new(character), 0).expect("character starts");
    let mut s = apply(RunState::default(), &start);
    s.deck = cards
        .iter()
        .flat_map(|(id, n)| {
            std::iter::repeat_n(DeckCard { card: CardId::new(*id), upgraded: false, enchantment: None }, *n)
        })
        .collect();
    s
}

fn stores_and_warnings(s: &RunState) -> ([spire_core::analysis::sim::Flow; 5], Vec<(Store, String)>) {
    let cat = spire_data::bundled_catalog();
    let pb = spire_data::bundled_playbook();
    let sim = simulate(s, &cat, &pb.simulation);
    let w = warnings(&sim, s, &cat, &pb).into_iter().map(|w| (w.store, w.text)).collect();
    (sim.stores, w)
}

#[test]
fn nock_is_worth_what_the_shoot_cards_make_of_it() {
    // With Shoot cards, Nock's Aim is made and spent.
    let shooting =
        state("FOOL", &[("NOCK", 2), ("FOOLISH_SHOT", 2), ("QUICK_SHOT", 2), ("STRIKE_FOOL", 3), ("DEFEND_FOOL", 3)]);
    let (stores, w) = stores_and_warnings(&shooting);
    let aim = stores[Store::Aim as usize];
    assert!(aim.gained > 1.0 && aim.spent > aim.gained * 0.5, "{aim:?}");
    assert!(!w.iter().any(|(s, _)| *s == Store::Aim), "{w:?}");
    // Without one, Nock isn't worth playing, and the deck says why.
    // (Song Bow, the Fool's relic, still gives its Aim each fight.)
    let none = state("FOOL", &[("NOCK", 3), ("STRIKE_FOOL", 5), ("DEFEND_FOOL", 4)]);
    let bare = state("FOOL", &[("STRIKE_FOOL", 5), ("DEFEND_FOOL", 4)]);
    let (stores, w) = stores_and_warnings(&none);
    assert_eq!(
        stores[Store::Aim as usize].gained,
        stores_and_warnings(&bare).0[Store::Aim as usize].gained,
        "Nock is left in hand"
    );
    assert!(w.iter().any(|(s, t)| *s == Store::Aim && t.contains("never used: 3 Aim cards, no Shoot cards")), "{w:?}");
}

#[test]
fn aim_piles_up_with_one_shoot_card() {
    let s = state("FOOL", &[("NOCK", 5), ("FOOLISH_SHOT", 1), ("STRIKE_FOOL", 4), ("DEFEND_FOOL", 3)]);
    let (_, w) = stores_and_warnings(&s);
    assert!(w.iter().any(|(st, t)| *st == Store::Aim && t.contains("piles up: 5 Aim cards for 1 Shoot card")), "{w:?}");
}

#[test]
fn shoot_cards_wait_when_arrows_dont_come_back() {
    // Free Shoot cards and few attack hits: one Arrow, rarely returned.
    let s = state("FOOL", &[("QUICK_SHOT", 6), ("DEFEND_FOOL", 4)]);
    let (stores, w) = stores_and_warnings(&s);
    assert!(stores[Store::Arrows as usize].waiting > 0.0);
    assert!(w.iter().any(|(st, t)| *st == Store::Arrows && t.contains("waits for an Arrow")), "{w:?}");
    // The starter deck's one Shoot card isn't a problem.
    let starter = state("FOOL", &[("STRIKE_FOOL", 5), ("DEFEND_FOOL", 4), ("FOOLISH_SHOT", 1)]);
    assert!(!stores_and_warnings(&starter).1.iter().any(|(st, _)| *st == Store::Arrows));
}

#[test]
fn star_cards_wait_without_stars_to_spend() {
    let s = state("REGENT", &[("CRESCENT_SPEAR", 5), ("STRIKE_REGENT", 3), ("DEFEND_REGENT", 3)]);
    let (_, w) = stores_and_warnings(&s);
    assert!(w.iter().any(|(st, t)| *st == Store::Stars && t.contains("wait for Stars")), "{w:?}");
    let starter = state("REGENT", &[("STRIKE_REGENT", 4), ("DEFEND_REGENT", 4), ("FALLING_STAR", 1), ("VENERATE", 1)]);
    let (_, w) = stores_and_warnings(&starter);
    assert!(!w.iter().any(|(st, _)| *st == Store::Stars), "{w:?}");
}
