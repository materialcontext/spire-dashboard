//! The run report on a real run (the finished-run fixture), won and cut
//! short.

use spire_core::analysis::report::{Decision, TurningKind, report};
use spire_core::catalog::NodeKind;
use spire_core::run::{Outcome, Run};

fn run(json: &str) -> Run {
    let cat = spire_data::bundled_catalog();
    let g = spire_data::game_run::read(json, &cat, &Default::default()).expect("reads");
    Run { events: g.events, ..Default::default() }
}

#[test]
fn a_won_run_reports_its_acts_turning_points_and_decisions() {
    let cat = spire_data::bundled_catalog();
    let pb = spire_data::bundled_playbook();
    let r = report(&run(include_str!("fixtures/finished.run")), &cat, &pb);
    assert_eq!(r.ending.outcome, Some(Outcome::Victory));
    assert!(r.causes.is_empty(), "a win has nothing to explain");
    assert_eq!(r.acts.len(), 3);
    assert!(r.acts.iter().all(|a| a.boss.as_ref().is_some_and(|b| b.3 == Some(true))), "every boss beaten");
    // The first shop: two cards bought, the rest of the shelf passed over,
    // 146 gold spent.
    let shop = r.floors.iter().find(|f| f.kind == NodeKind::Shop).expect("a shop");
    assert!(
        shop.decisions
            .iter()
            .any(|d| matches!(d, Decision::Cards { taken, passed } if taken.len() == 2 && passed.len() == 5))
    );
    assert!(shop.decisions.iter().any(|d| matches!(d, Decision::Gold { spent: 146, .. })));
    // Rest sites, and a hallway fight that cost far more than usual.
    assert!(r.turning.iter().any(|t| matches!(&t.what, TurningKind::Rest { choice, .. } if choice == "SMITH")));
    assert!(r.turning.iter().any(|t| matches!(t.what, TurningKind::BigLoss { .. })));
}

#[test]
fn a_death_names_its_fight_and_ranked_causes() {
    let cat = spire_data::bundled_catalog();
    let pb = spire_data::bundled_playbook();
    // The same run, ended at its first boss.
    let mut v: serde_json::Value = serde_json::from_str(include_str!("fixtures/finished.run")).unwrap();
    let first = v["map_point_history"][0].clone();
    v["map_point_history"] = serde_json::json!([first]);
    v["win"] = serde_json::json!(false);
    v["killed_by_encounter"] = serde_json::json!("ENCOUNTER.SOUL_FYSH_BOSS");
    let r = report(&run(&v.to_string()), &cat, &pb);
    assert!(matches!(r.ending.outcome, Some(Outcome::Died { .. })));
    let d = r.deciding.as_ref().expect("the fight that ended it");
    assert_eq!(d.encounter.as_str(), "SOUL_FYSH_BOSS");
    assert!(!d.won);
    assert!(!r.causes.is_empty() && r.causes.len() <= pb.report.causes as usize);
    assert!(r.causes.windows(2).all(|w| w[0].weight >= w[1].weight), "ranked");
    assert_eq!(r.acts.len(), 1);
    assert_eq!(r.acts[0].boss.as_ref().map(|b| b.3), Some(Some(false)), "the boss wasn't beaten");
}
