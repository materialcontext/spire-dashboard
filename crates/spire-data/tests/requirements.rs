//! Cards that need something the run doesn't supply are dead.

use spire_core::analysis::deck::run_profile;
use spire_core::analysis::requirements::unmet;
use spire_core::analysis::sim::simulate;
use spire_core::ids::{CardId, CharacterId};
use spire_core::run::{DeckCard, RunEvent, RunState, apply};

fn run(character: &str, extra: &[&str]) -> RunState {
    let cat = spire_data::bundled_catalog();
    let start = RunEvent::start(&cat, &CharacterId::new(character), 0).expect("character starts");
    let mut s = apply(RunState::default(), &start);
    s.deck.extend(extra.iter().map(|id| DeckCard { card: CardId::new(*id), upgraded: false, enchantment: None }));
    s
}

fn missing(s: &RunState) -> Vec<(String, String)> {
    let cat = spire_data::bundled_catalog();
    let pb = spire_data::bundled_playbook();
    unmet(s, &cat, &pb.simulation.requirements).into_iter().map(|(c, w)| (c.to_string(), w)).collect()
}

#[test]
fn needs_nothing_supplies_make_dead_cards() {
    let pair = |c: &str, w: &str| (c.to_string(), w.to_string());
    // Growth exhausts Sadness: dead without a card that adds it.
    assert_eq!(missing(&run("FOOL", &["GROWTH"])), vec![pair("GROWTH", "Sadness")]);
    assert!(missing(&run("FOOL", &["GROWTH", "BRIGHTEN_UP"])).is_empty());
    // Evoking needs orbs: the Defect's Zap channels one, the Ironclad has none.
    assert_eq!(missing(&run("IRONCLAD", &["DUALCAST"])), vec![pair("DUALCAST", "orbs")]);
    assert!(missing(&run("DEFECT", &["DUALCAST"])).is_empty());
    // Shoot cards need the Fool's Arrows; Star cards need a Star maker.
    assert_eq!(missing(&run("IRONCLAD", &["FOOLISH_SHOT"])), vec![pair("FOOLISH_SHOT", "Arrows")]);
    assert_eq!(missing(&run("IRONCLAD", &["CRESCENT_SPEAR"])), vec![pair("CRESCENT_SPEAR", "Stars")]);
    assert!(missing(&run("REGENT", &["CRESCENT_SPEAR"])).is_empty(), "Venerate makes Stars");
    // Exhausting Status cards answers what a fight adds: never dead.
    assert!(missing(&run("DEFECT", &["FLAK_CANNON"])).is_empty());
}

#[test]
fn dead_cards_fill_no_role_and_are_never_played() {
    let cat = spire_data::bundled_catalog();
    let pb = spire_data::bundled_playbook();
    let with = run("IRONCLAD", &["DUALCAST", "DUALCAST"]);
    let profile = run_profile(&with, &cat, &pb);
    assert_eq!(profile.dead, 2);
    let sim = simulate(&with, &cat, &pb.simulation);
    assert!(sim.dead_draws > 0.1, "{}", sim.dead_draws);
}
