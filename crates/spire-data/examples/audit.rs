//! Prints how the playbook reads the bundled catalog, for tuning
//! `data/playbook.json`: role coverage per character, unclassified cards,
//! and encounter threat numbers per act and room.
//!
//! `cargo run -p spire-data --example audit [-- --cards] [-- --sim]`
//!
//! `--sim` runs the deck simulator on every starter deck instead.
//! `--mechanics` shows what the simulator reads from every power and relic.
//! `--outlook` plays every starter deck against every elite (the survival
//! check), next to the community's outcomes.
//! `--patterns` prints every elite's and boss's expected damage per enemy
//! turn (12 turns), its HP, and the community's turns and damage taken.

use std::collections::BTreeMap;

use spire_core::analysis::threats::threat_profile;
use spire_core::catalog::{CardType, Catalog};
use spire_core::playbook::Role;

fn main() {
    let show_cards = std::env::args().any(|a| a == "--cards");
    let cat = spire_data::bundled_catalog();
    let pb = spire_data::bundled_playbook();
    if std::env::args().any(|a| a == "--sim") {
        starter_sims(&cat, &pb);
        return;
    }
    if std::env::args().any(|a| a == "--patterns") {
        patterns(&cat, &pb);
        return;
    }
    if std::env::args().any(|a| a == "--outlook") {
        starter_outlooks(&cat, &pb);
        return;
    }
    if std::env::args().any(|a| a == "--mechanics") {
        mechanics(&cat);
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
            println!("{:<24} {:?} | {}", c.name, pb.roles_of(c), c.text.replace('\n', " "));
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
                    format!("{}{}:{}hp/{}dmg", e.name, if e.weak { "(w)" } else { "" }, t.hp_min, t.peak_turn)
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
        let jobs = spire_core::analysis::jobs::measure(&sim);
        let described: Vec<String> =
            Role::ALL.iter().map(|r| format!("{}: {}", r.short(), jobs.describe(*r))).collect();
        println!("{:<12} {} · bad draws {:.0}%", "", described.join(" · "), jobs.floor * 100.0);
        let p = sim.play;
        println!(
            "{:<12} hits {:.1}/turn at {:.1} each · area {:.0}% · cards {:.1} (skills {:.1}) · debuffs {:.1}",
            "",
            p.hits,
            p.per_hit,
            p.area_share * 100.0,
            p.cards,
            p.skills,
            p.debuffs
        );
    }
}

fn starter_outlooks(cat: &Catalog, pb: &spire_core::playbook::Playbook) {
    use spire_core::analysis::{sim::simulate, survival};
    use spire_core::run::{RunEvent, RunState, apply};
    let mut elites: Vec<_> =
        cat.encounters.values().filter(|e| e.room == spire_core::catalog::RoomType::Elite).collect();
    elites.sort_by_key(|e| (e.act.as_ref().and_then(|a| cat.acts.get(a)).map(|a| a.index), e.name.clone()));
    for id in cat.characters.keys() {
        let Some(start) = RunEvent::start(cat, id, 0) else { continue };
        let state = apply(RunState::default(), &start);
        let sim = simulate(&state, cat, &pb.simulation);
        println!("== {id} ({} HP, {} cards)", state.hp, state.deck.len());
        for e in &elites {
            let Some(o) = survival::for_encounter(&sim, e, cat, &state, &pb.simulation) else { continue };
            let community = e.stats.as_ref().map_or(String::new(), |s| {
                format!(
                    "community {:.1} turns, {:.0} dmg, {:.0}% end the run",
                    s.avg_turns,
                    s.avg_damage_taken,
                    s.death_rate() * 100.0
                )
            });
            let joins: Vec<String> = o.joins.iter().map(|j| format!("+{}x {}", j.count, j.name)).collect();
            println!(
                "  {:<22} {:<8} kill {:>2?} lose {:>3}  junk {:>4.1} {:<16} | {community}",
                e.name,
                o.verdict.label(),
                o.best.kill_turn,
                o.best.hp_loss,
                o.best.junk,
                joins.join(" "),
            );
        }
    }
}

fn patterns(cat: &Catalog, pb: &spire_core::playbook::Playbook) {
    use spire_core::analysis::pattern::power_words;
    use spire_core::analysis::shape::{self, Gap};
    use spire_core::analysis::threats::Scaling;
    use spire_core::catalog::RoomType;
    let mut fights: Vec<_> = cat.encounters.values().filter(|e| e.room != RoomType::Monster).collect();
    fights.sort_by_key(|e| (e.act.as_ref().and_then(|a| cat.acts.get(a)).map(|a| a.index), e.room, e.name.clone()));
    for e in fights {
        let s = shape::of(e, cat, Scaling::default(), &pb.fights);
        let turns: Vec<String> = s.turns.iter().map(|l| format!("{:.0}", l.expected)).collect();
        let hp: Vec<String> = s.stages.iter().map(u32::to_string).collect();
        let community = e.stats.as_ref().map_or(String::new(), |st| {
            format!("{:.1} turns, {:.0} dmg, {:.0}% die", st.avg_turns, st.avg_damage_taken, st.death_rate() * 100.0)
        });
        println!("{} ({:?}) hp {} | {community}", e.name, e.room, hp.join("+"));
        println!("  {:<11} pressure {:.0} | {}", s.kind.label(), s.pressure, turns.join(" "));
        let certain: Vec<String> =
            s.spikes.iter().filter(|x| x.certain).map(|x| format!("t{} {:.0}", x.turn, x.load.expected)).collect();
        let possible = s.spikes.iter().filter(|x| !x.certain).map(|x| x.load.high).max();
        if !certain.is_empty() || possible.is_some() {
            let up_to = possible.map_or(String::new(), |h| format!(" · possible up to {h}"));
            println!("  spikes {}{up_to}", certain.join(", "));
        }
        if !s.windows.is_empty() {
            println!("  quiet turns {:?}", s.windows);
        }
        if (s.ramp - 1.0).abs() > 0.05 {
            println!("  ramp x{:.2}", s.ramp);
        }
        if !s.clocks.is_empty() {
            println!("  clocks {:?}", s.clocks);
        }
        for g in &s.gaps {
            let text = match g {
                Gap::NoPattern { monster } => format!("{monster}: no pattern"),
                Gap::OrderLost { monster, from } => format!("{monster}: order unknown from turn {from}"),
                Gap::NoHp { monster } => format!("{monster}: no HP"),
            };
            println!("  gap: {text}");
        }
        for r in &s.rules {
            let asks: Vec<&str> = r.kind.asks().iter().map(|a| a.label()).collect();
            let amount = r.power.amount.map_or(String::new(), |a| format!(" {a}"));
            println!(
                "  rule: {}{amount} ({:?}){}",
                power_words(&r.power.power),
                r.kind,
                if asks.is_empty() { String::new() } else { format!(" → {}", asks.join(", ")) }
            );
        }
    }
}

fn mechanics(cat: &Catalog) {
    use spire_core::mechanics::read;
    let orbs: Vec<_> = cat.orbs.values().cloned().collect();
    let verbose = std::env::args().any(|a| a == "-v");
    let report = |kind: &str, name: &str, text: &str, played: bool| -> bool {
        let m = read(text, &orbs, played);
        let ok = !m.is_empty();
        if verbose || !ok {
            println!("{kind} {} {name:<26} {:?} | unread: {:?}", if ok { "+" } else { "-" }, m.describe(), m.unread);
        }
        ok
    };
    let (mut p_ok, mut p_all) = (0, 0);
    for c in cat.cards.values().filter(|c| c.card_type == CardType::Power) {
        p_all += 1;
        p_ok += usize::from(report("P", &c.name, &c.text, true));
    }
    let (mut r_ok, mut r_all) = (0, 0);
    for r in cat.relics.values() {
        r_all += 1;
        r_ok += usize::from(report("R", &r.name, &r.text, false));
    }
    println!("powers read: {p_ok}/{p_all}  relics read: {r_ok}/{r_all}");
}
