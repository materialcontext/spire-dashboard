//! Fight shapes and rules, read from the bundled data, for fights whose
//! shape players know.

use spire_core::analysis::rules::{Ask, Kind as Rule};
use spire_core::analysis::shape::{self, Clock, Gap, Kind};
use spire_core::analysis::threats::Scaling;
use spire_core::catalog::Catalog;
use spire_core::ids::EncounterId;

fn shape(id: &str) -> shape::Shape {
    let cat = Catalog::from_layers([spire_data::bundled_pack()]);
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
fn gaps_in_the_data_are_marked() {
    let e = shape("ENTOMANCER_ELITE");
    assert!(e.gaps.iter().any(|g| matches!(g, Gap::OrderLost { .. })));
    assert!(e.spikes.iter().any(|s| !s.certain), "its biggest move stays a possible spike");
    assert!(has_rule(&e, Rule::JunkPerHit));
    let t = shape("TEST_SUBJECT_BOSS");
    assert!(t.gaps.iter().any(|g| matches!(g, Gap::NoHp { .. })));
    assert!(t.clocks.contains(&Clock::Revives));
}
