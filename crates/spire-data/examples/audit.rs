//! Prints how the playbook reads the bundled catalog, for tuning
//! `data/playbook.json`: role coverage per character, unclassified cards,
//! and encounter threat numbers per act and room.
//!
//! `cargo run -p spire-data --example audit [-- --cards] [-- --sim]`
//!
//! `--sim` runs the deck simulator on every starter deck instead.

use std::collections::BTreeMap;

use spire_core::analysis::threats::threat_profile;
use spire_core::catalog::{CardType, Catalog};
use spire_core::playbook::Role;

fn main() {
    let show_cards = std::env::args().any(|a| a == "--cards");
    let cat = Catalog::from_layers([spire_data::bundled_pack()]);
    let pb = spire_data::bundled_playbook();
    if std::env::args().any(|a| a == "--sim") {
        starter_sims(&cat, &pb);
        return;
    }

    println!("== roles by pool (excluding curses/statuses)");
    let mut by_color: BTreeMap<&str, BTreeMap<Role, u32>> = BTreeMap::new();
    let mut none = Vec::new();
    for c in cat.cards.values() {
        if matches!(c.card_type, CardType::Curse | CardType::Status | CardType::Quest) {
            continue;
        }
        let roles = pb.roles_of(c);
        if roles.is_empty() {
            none.push(format!("{} [{}] {}", c.name, c.color, c.text.replace('\n', " ")));
        }
        for r in roles {
            *by_color.entry(&c.color).or_default().entry(r).or_default() += 1;
        }
        if show_cards {
            println!("{:<24} {:?} aoe={} | {}", c.name, pb.roles_of(c), pb.is_aoe(c), c.text.replace('\n', " "));
        }
    }
    for (color, roles) in &by_color {
        let row: Vec<String> =
            Role::ALL.iter().map(|r| format!("{}={}", r.short(), roles.get(r).unwrap_or(&0))).collect();
        println!("{color:<12} {}", row.join("  "));
    }
    println!("\n== unclassified: {}", none.len());
    for n in none.iter().take(40) {
        println!("  {n}");
    }

    println!("\n== encounters (hp_min / worst turn)");
    for act in cat.acts.values() {
        for room in [
            spire_core::catalog::RoomType::Monster,
            spire_core::catalog::RoomType::Elite,
            spire_core::catalog::RoomType::Boss,
        ] {
            let mut rows: Vec<String> = cat
                .encounters_in(&act.id, room)
                .map(|e| {
                    let t = threat_profile(e, &cat, &pb, Default::default());
                    format!("{}{}:{}hp/{}dmg", e.name, if e.weak { "(w)" } else { "" }, t.hp_min, t.worst_turn)
                })
                .collect();
            rows.sort();
            println!("{} idx{} {:?}: {}", act.name, act.index, room, rows.join(", "));
        }
    }
}

fn starter_sims(cat: &Catalog, pb: &spire_core::playbook::Playbook) {
    use spire_core::analysis::sim::simulate;
    use spire_core::run::{RunEvent, RunState, apply};
    for id in cat.characters.keys() {
        let Some(start) = RunEvent::start(cat, id, 0) else { continue };
        let state = apply(RunState::default(), &start);
        let t0 = std::time::Instant::now();
        let sim = simulate(&state, cat, &pb.simulation);
        let took = t0.elapsed();
        let turns: Vec<String> =
            sim.turns.iter().map(|t| format!("{:.1}/{:.1}", t.damage.mean, t.block.mean)).collect();
        println!(
            "{id:<12} dmg/blk by turn {}  turnover {:.1}  dead {:.0}%  unspent {:.2}  stranded {:.2}  {:?}  ({took:.1?})",
            turns.join(" "),
            sim.turnover,
            sim.dead_draws * 100.0,
            sim.unspent_energy,
            sim.stranded_energy,
            sim.resources,
        );
    }
}
