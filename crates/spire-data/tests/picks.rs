//! Card-reward suggestions on the bundled data.

use spire_core::analysis::lines::{Why, suggest};
use spire_core::analysis::sim::simulate;
use spire_core::analysis::threats::next_threats;
use spire_core::analysis::{deck::deck_profile, survival};
use spire_core::catalog::NodeKind;
use spire_core::ids::{ActId, CharacterId, EncounterId};
use spire_core::run::{RunEvent, RunState, apply};

/// A starter deck entering an act with its boss known.
fn entering(character: &str, act: &str, index: u8, boss: &str) -> RunState {
    let cat = spire_data::bundled_catalog();
    let start = RunEvent::start(&cat, &CharacterId::new(character), 0).expect("character starts");
    let s = apply(RunState::default(), &start);
    apply(s, &RunEvent::EnterAct { act: ActId::new(act), index, boss: Some(EncounterId::new(boss)) })
}

#[test]
fn a_starter_deck_gets_at_most_three_and_fillers_beat_the_boss_better() {
    let cat = spire_data::bundled_catalog();
    let pb = spire_data::bundled_playbook();
    assert!(!cat.card_lines.is_empty(), "the bundle carries card lines");
    assert!(!cat.card_lift.is_empty(), "the bundle carries card lift");
    for (character, act, boss) in
        [("IRONCLAD", "OVERGROWTH", "THE_KIN_BOSS"), ("SILENT", "UNDERDOCKS", "SOUL_FYSH_BOSS")]
    {
        let s = entering(character, act, 0, boss);
        let deck = deck_profile(&s.deck, &cat, &pb);
        let preview = next_threats(&s, NodeKind::Boss, &cat, &pb, &deck).into_iter().next().expect("the boss");
        let sim = simulate(&s, &cat, &pb.simulation);
        let enc = &cat.encounters[&preview.encounter];
        let outlook = survival::for_encounter(&sim, enc, &cat, &s, &pb).expect("an outlook");
        let picks = suggest(&s, &cat, &pb, Some((&preview, &outlook)));
        assert!(picks.len() <= 3, "{character}: {} suggestions", picks.len());
        for p in &picks {
            match &p.why {
                Why::Filler { hp_before, hp_after, verdict_before, verdict_after, .. } => assert!(
                    hp_after + pb.picks.min_hp_saved <= *hp_before || verdict_after < verdict_before,
                    "{character}: a filler must do better against the boss"
                ),
                Why::Line { .. } => {
                    assert!(!p.held.is_empty(), "a line is within reach");
                    assert!(p.options.iter().all(|o| o.len() <= pb.picks.max_missing as usize));
                    assert!(p.options.len() <= pb.picks.max_options as usize);
                }
            }
        }
    }
}
