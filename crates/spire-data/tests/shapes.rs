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
    // Three segments cycling Bulk, Writhe, Outgas: Bulk's Strength builds.
    let d = shape("DECIMILLIPEDE_ELITE");
    assert!(d.gaps.is_empty(), "{:?}", d.gaps);
    assert_eq!(d.kind, Kind::Ramping);
    // Entomancer grows by 1 Strength per Pheromone Spit (wiki), not 3.
    assert_eq!(e.turns[3].expected, 28.0);
}

#[test]
fn axebots_replace_each_other_so_they_need_no_area_damage() {
    use spire_core::analysis::threats::{demands, threat_profile};
    use spire_core::catalog::RoomType;
    use spire_core::playbook::Role;
    let cat = spire_data::bundled_catalog();
    let pb = spire_data::bundled_playbook();
    let wants_aoe = |id: &str| {
        let enc = cat.encounters.get(&EncounterId::new(id)).unwrap_or_else(|| panic!("{id} in the bundle"));
        let t = threat_profile(enc, &cat, &pb, Scaling::default());
        demands(&t, RoomType::Monster, None, None).iter().any(|d| d.role == Role::Aoe)
    };
    // Stock: "When killed, a new Axebot is summoned in its place". Runs
    // record the replacement in the fight's monsters, but it comes alone.
    assert!(!wants_aoe("AXEBOTS_NORMAL"));
    // Monsters that do stand together still ask for it.
    assert!(wants_aoe("TOADPOLES_WEAK"));
}

/// Each stage's rules and Intangible share, as the survival check fights them.
fn stages(id: &str) -> Vec<(Vec<String>, Option<f32>)> {
    use spire_core::analysis::{survival, threats};
    let cat = spire_data::bundled_catalog();
    let enc = cat.encounters.get(&EncounterId::new(id)).unwrap_or_else(|| panic!("{id} in the bundle"));
    threats::lineup_waves(enc, &cat, Scaling::default(), 12)
        .iter()
        .map(|w| {
            let rules = survival::stage_rules(w, &cat).into_iter().map(|r| r.power.power).collect();
            (rules, survival::intangible_share(w, &cat))
        })
        .collect()
}

#[test]
fn each_phase_fights_under_its_own_powers() {
    // Test Subject gains Painful Stabs on its first revive and Nemesis on
    // its second (patterns.json, from the wiki), not from the start.
    let rules: Vec<Vec<String>> = stages("TEST_SUBJECT_BOSS").into_iter().map(|s| s.0).collect();
    assert_eq!(rules, vec![vec!["ADAPTABLE", "ENRAGE"], vec!["ADAPTABLE", "PAINFUL_STABS"], vec!["NEMESIS"]]);
}

#[test]
fn intangible_from_a_move_counts_the_turns_the_pattern_uses_it() {
    // Soul Fysh fades (Intangible 2) once in its five-move cycle: twice in
    // the twelve turns walked, two turns each.
    let share = stages("SOUL_FYSH_BOSS")[0].1.expect("Fade gives Intangible");
    assert!((share - 4.0 / 12.0).abs() < 0.01, "{share}");
}
