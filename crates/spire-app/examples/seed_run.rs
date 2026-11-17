//! Writes a sample run into a data folder, for screenshots and manual
//! testing: `seed_run <data-dir> <mid|late|fool|history>`. `history` files
//! three finished runs in the run history and leaves the current run alone.

use spire_core::catalog::{Catalog, RoomType};
use spire_core::ids::*;
use spire_core::run::{Change, DeckCard, NodeKind, NodeReport};
use spire_core::{Run, RunEvent};
use spire_data::store::Store;

/// The Fool at A10, deep in act 3 with both bosses known.
fn fool_run(cat: &Catalog) -> Run {
    let mut run = Run::default();
    run.push(RunEvent::start(cat, &CharacterId::new("FOOL"), 10).expect("Fool in bundled data"));
    let cards: Vec<CardId> = cat
        .cards
        .values()
        .filter(|c| c.color == "fool" && !c.tags.iter().any(|t| t == "Strike" || t == "Defend"))
        .map(|c| c.id.clone())
        .take(16)
        .collect();
    let mut changes: Vec<Change> = cards
        .iter()
        .enumerate()
        .map(|(i, c)| Change::AddCard(DeckCard { card: c.clone(), upgraded: i % 3 == 0, enchantment: None }))
        .collect();
    changes.extend(
        cat.relics
            .values()
            .filter(|r| r.pool == "fool" || r.pool == "shared")
            .take(9)
            .map(|r| Change::AddRelic(r.id.clone())),
    );
    changes.extend(cat.potions.keys().take(1).cloned().map(Change::AddPotion));
    changes.extend([Change::SetMaxHp(84), Change::SetHp(47), Change::SetGold(312)]);
    run.push(RunEvent::Adjust(changes));
    for (index, act) in [(0u8, "OVERGROWTH"), (1, "HIVE"), (2, "GLORY")] {
        let act = ActId::new(act);
        let bosses = cat.acts[&act].bosses.clone();
        run.push(RunEvent::EnterAct { act: act.clone(), index, boss: bosses.first().cloned() });
        for e in cat
            .encounters_in(&act, RoomType::Monster)
            .take(if index == 2 { 2 } else { 4 })
            .map(|e| e.id.clone())
            .collect::<Vec<_>>()
        {
            run.push(RunEvent::Node(NodeReport {
                kind: NodeKind::Monster,
                encounter: Some(e),
                event: None,
                changes: vec![],
            }));
        }
        if index < 2 {
            run.push(RunEvent::Node(NodeReport {
                kind: NodeKind::Boss,
                encounter: bosses.first().cloned(),
                event: None,
                changes: vec![],
            }));
        } else {
            run.push(RunEvent::SetSecondBoss(bosses[1].clone()));
        }
    }
    run
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let (dir, kind) = (args.get(1).expect("data dir"), args.get(2).map_or("late", String::as_str));
    let store = Store::at(dir);
    let cat = store.load().catalog;
    if kind == "history" {
        for run in spire_app::bench::archived_runs(&cat) {
            store.archive_run(&run).expect("archive run");
        }
        println!("filed sample runs in {dir}");
        return;
    }
    let run = match kind {
        "mid" => spire_app::bench::mid_run(&cat),
        "fool" => fool_run(&cat),
        _ => spire_app::bench::late_run(&cat),
    };
    store.save_run(&run).expect("save run");
    println!("wrote {kind} run to {dir}");
}
