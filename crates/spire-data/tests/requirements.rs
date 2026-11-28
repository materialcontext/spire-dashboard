//! Cards that need something the run doesn't supply are dead.

use spire_core::analysis::deck::run_profile;
use spire_core::analysis::requirements::{Dead, dead};
use spire_core::analysis::sim::simulate;
use spire_core::ids::{CardId, CharacterId};
use spire_core::run::{DeckCard, RunEvent, RunState, apply};

fn run(character: &str, extra: &[&str]) -> RunState {
    let cat = spire_data::bundled_catalog();
    let start = RunEvent::start(&cat, &CharacterId::new(character), 0).expect("character starts");
    let mut s = apply(RunState::default(), &start);
    s.deck.extend(extra.iter().map(|id| DeckCard {
        card: CardId::new(*id),
        upgraded: false,
        enchantment: None,
        enchant_amount: None,
    }));
    s
}

fn missing(s: &RunState) -> Vec<(String, String)> {
    let cat = spire_data::bundled_catalog();
    let pb = spire_data::bundled_playbook();
    dead(s, &cat, &pb.simulation.requirements)
        .into_iter()
        .map(|(c, d)| {
            let w = match d {
                Dead::Needs(w) => w,
                Dead::Locked { by, .. } => by,
            };
            (c.to_string(), w)
        })
        .collect()
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

fn with_relic(mut s: RunState, relic: &str) -> RunState {
    s.relics.push(spire_core::ids::RelicId::new(relic));
    s
}

#[test]
fn relic_locks_are_read_from_their_text() {
    use spire_core::mechanics::{Passive, read};
    let cat = spire_data::bundled_catalog();
    let passives = |id: &str| read(&cat.relics[&spire_core::ids::RelicId::new(id)].text, &[], false).passives;
    assert!(passives("FIDDLE").contains(&Passive::NoDrawInTurn));
    assert!(passives("VELVET_CHOKER").contains(&Passive::PlayCap(6)));
    assert!(passives("SOZU").contains(&Passive::NoPotions));
    assert!(passives("ECTOPLASM").contains(&Passive::NoGold));
}

#[test]
fn a_relic_lock_kills_the_cards_it_forbids_all_of() {
    let pair = |c: &str, w: &str| (c.to_string(), w.to_string());
    // Fiddle: no draws during your turn. Cards that only draw do nothing;
    // Pommel Strike keeps its damage; Machine Learning draws at the start
    // of a turn, which Fiddle allows; Dark Embrace draws on an exhaust.
    let fiddle = with_relic(
        run("IRONCLAD", &["BATTLE_TRANCE", "POMMEL_STRIKE", "DARK_EMBRACE", "ACROBATICS", "MACHINE_LEARNING"]),
        "FIDDLE",
    );
    assert_eq!(
        missing(&fiddle),
        vec![pair("ACROBATICS", "Fiddle"), pair("BATTLE_TRANCE", "Fiddle"), pair("DARK_EMBRACE", "Fiddle")]
    );
    // Without Fiddle they're fine.
    let free = run("IRONCLAD", &["BATTLE_TRANCE", "ACROBATICS"]);
    assert!(missing(&free).is_empty());
    // Sozu: no Potions, so a card that only makes one does nothing.
    assert_eq!(missing(&with_relic(run("SILENT", &["ALCHEMIZE"]), "SOZU")), vec![pair("ALCHEMIZE", "Sozu")]);
}

#[test]
fn the_simulator_keeps_to_the_locks() {
    let cat = spire_data::bundled_catalog();
    let pb = spire_data::bundled_playbook();
    // Fiddle: Flash of Steel (0 energy, draw 1) chains through the deck
    // without it; with it, only the start-of-turn hand is played.
    let cards = |s: &RunState| simulate(s, &cat, &pb.simulation).play.cards;
    let deck = run("IRONCLAD", &["FLASH_OF_STEEL"; 10]);
    let (fiddled, free) = (cards(&with_relic(deck.clone(), "FIDDLE")), cards(&deck));
    assert!(fiddled < free - 0.8, "{fiddled} vs {free}");
    // Velvet Choker: no more than 6 cards a turn, however cheap.
    let cheap = run("IRONCLAD", &["ANGER"; 12]);
    let choked = with_relic(cheap.clone(), "VELVET_CHOKER");
    assert!(cards(&choked) <= 6.0, "{}", cards(&choked));
}
