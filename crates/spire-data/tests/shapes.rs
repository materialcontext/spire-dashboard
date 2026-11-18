//! Fight shapes and rules, read from the bundled data, for fights whose
//! shape players know.

use spire_core::analysis::rules::{Ask, Kind as Rule};
use spire_core::analysis::shape::{self, Clock, Gap, Kind};
use spire_core::analysis::threats::Scaling;
use spire_core::ids::EncounterId;

fn shape(id: &str) -> shape::Shape {
    let cat = spire_data::bundled_catalog();
    let pb = spire_data::bundled_playbook();
    let enc = cat.encounters.get(&EncounterId::new(id)).unwrap_or_else(|| panic!("{id} in the bundle"));
    shape::of(enc, &cat, Scaling::default(), &pb.fights)
}

fn has_rule(s: &shape::Shape, kind: Rule) -> bool {
    s.rules.iter().any(|r| r.kind == kind)
}

#[test]
fn bygone_effigy_sleeps_then_presses() {
    let s = shape("BYGONE_EFFIGY_ELITE");
    assert_eq!(s.kind, Kind::SlowStart);
    assert_eq!(s.windows, vec![1, 2]);
    assert!(!s.clocks.contains(&Clock::Ramp), "sleep isn't a ramp: {:?}", s.clocks);
    assert!(has_rule(&s, Rule::MorePerCard));
}

#[test]
fn knight_gang_and_mecha_knight_spike() {
    let k = shape("KNIGHTS_ELITE");
    assert_eq!(k.kind, Kind::Spikes);
    assert!(k.spikes.iter().filter(|s| s.certain).count() >= 3);
    let m = shape("MECHA_KNIGHT_ELITE");
    assert_eq!(m.kind, Kind::Spikes);
    assert!(!m.windows.is_empty(), "a quiet turn follows each spike");
    assert!(has_rule(&m, Rule::NegatesDebuffs));
}

#[test]
fn vantom_wants_many_hits() {
    let s = shape("VANTOM_BOSS");
    assert!(s.rules.iter().any(|r| r.kind == Rule::HitsLoseOne && r.kind.asks().contains(&Ask::ManyHits)));
}

#[test]
fn phrog_parasite_alternates_and_summons() {
    let s = shape("PHROG_PARASITE_ELITE");
    assert_eq!(s.kind, Kind::Alternating);
    assert!(s.spikes.iter().all(|x| !x.certain), "its Lash is its pressure, not a spike");
    assert!(s.clocks.contains(&Clock::Summons) && s.clocks.contains(&Clock::Statuses));
    assert_eq!(s.stages.len(), 2);
}

#[test]
fn gaps_in_the_codex_data_are_marked() {
    // The Codex data alone, without the curated fixes.
    let cat = spire_core::Catalog::from_layers([spire_data::bundled_pack()]);
    let pb = spire_data::bundled_playbook();
    let of = |id: &str| shape::of(&cat.encounters[&EncounterId::new(id)], &cat, Scaling::default(), &pb.fights);
    let e = of("ENTOMANCER_ELITE");
    assert!(e.gaps.iter().any(|g| matches!(g, Gap::OrderLost { .. })));
    assert!(e.spikes.iter().any(|s| !s.certain), "its biggest move stays a possible spike");
    assert!(has_rule(&e, Rule::JunkPerHit));
    let t = of("TEST_SUBJECT_BOSS");
    assert!(t.gaps.iter().any(|g| matches!(g, Gap::NoHp { .. })));
    assert!(t.clocks.contains(&Clock::Revives));
}

#[test]
fn the_curated_fixes_fill_the_gaps() {
    let e = shape("ENTOMANCER_ELITE");
    assert!(e.gaps.is_empty(), "{:?}", e.gaps);
    // Beeeees! 21, Spear! 18, Pheromone Spit 0: spikes, with a quiet turn.
    assert_eq!(e.kind, Kind::Spikes);
    assert!(e.windows.contains(&3));
    let t = shape("TEST_SUBJECT_BOSS");
    assert_eq!(t.stages, vec![100, 200, 300]);
    assert!(t.gaps.is_empty(), "{:?}", t.gaps);
    assert!(!t.clocks.contains(&Clock::Summons), "phases aren't summons");
    // Lagavulin Matriarch sleeps 3 turns, then ramps on Soul Siphon.
    let l = shape("LAGAVULIN_MATRIARCH_BOSS");
    assert_eq!(l.windows.iter().take(3).copied().collect::<Vec<_>>(), vec![1, 2, 3]);
    assert!(l.clocks.contains(&Clock::Ramp));
    let d = shape("DECIMILLIPEDE_ELITE");
    assert!(d.gaps.is_empty(), "{:?}", d.gaps);
}
