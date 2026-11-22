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
//! `--tags` counts each tag per character and lists every class card the
//! reader leaves untagged.
//! `--lines` prints the card lines in the catalog: per character and act,
//! the boss fights behind them and the best lines.
//! `--fights <page.gz>...` rebuilds A10 decks going into every elite and
//! boss fight and shows, per encounter, how the death rate moves with each
//! role's job and share, next to the playbook's demands for it.
//! `--coverage` prints, per card, what the simulator reads from it and the
//! sentences it leaves unread, and its tags (tab-separated; the card
//! dossier's coverage).
//! `--lookout <page.gz>...` rebuilds community decks at the end of each act
//! they cleared (run export pages) and counts, per `relic_fits` rule, the
//! share that qualify: a lookout warning should be rare.

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
    if std::env::args().any(|a| a == "--tags") {
        tags(&cat, &pb);
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
    if std::env::args().any(|a| a == "--coverage") {
        coverage(&cat, &pb);
        return;
    }
    if std::env::args().any(|a| a == "--mechanics") {
        mechanics(&cat);
        return;
    }
    if std::env::args().any(|a| a == "--lines") {
        lines(&cat);
        return;
    }
    if std::env::args().any(|a| a == "--roles") {
        let pages: Vec<String> = std::env::args().skip_while(|a| a != "--roles").skip(1).collect();
        roles(&cat, &pb, &pages);
        return;
    }
    if std::env::args().any(|a| a == "--fights") {
        let pages: Vec<String> = std::env::args().skip_while(|a| a != "--fights").skip(1).collect();
        fights(&cat, &pb, &pages);
        return;
    }
    if std::env::args().any(|a| a == "--lookout") {
        let pages: Vec<String> = std::env::args().skip_while(|a| a != "--lookout").skip(1).collect();
        lookout(&cat, &pb, &pages);
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
            let Some(o) = survival::for_encounter(&sim, e, cat, &state, pb) else { continue };
            let community = e.stats.as_ref().map_or(String::new(), |s| {
                format!(
                    "community {:.1} turns, {:.0} dmg, {:.0}% end the run",
                    s.avg_turns,
                    s.avg_damage_taken,
                    s.death_rate() * 100.0
                )
            });
            let joins: Vec<String> = o.joins.iter().map(|j| format!("+{}x {}", j.count, j.name)).collect();
            let limits: Vec<&str> = o.limits.iter().map(|l| l.label()).collect();
            println!(
                "  {:<22} {:<8} {:<7} kill {:>2?} lose {:>3}  junk {:>4.1} {:<16} limits [{}] | {community}",
                e.name,
                o.verdict.label(),
                o.lead.way.label(),
                o.lead.kill_turn,
                o.lead.hp_loss,
                o.lead.junk,
                joins.join(" "),
                limits.join(", "),
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

/// Splits a `Debug` struct body at its top-level commas and keeps the
/// fields that differ from their default.
fn set_fields(debug: &str) -> Vec<String> {
    let body = debug.split_once(" { ").map_or("", |(_, b)| b.trim_end_matches(" }"));
    let (mut out, mut depth, mut start) = (Vec::new(), 0i32, 0);
    let mut push = |part: &str| {
        let part = part.trim();
        let empty = ["[]", "None", "false", "0", "[0, 0, 0]"];
        if let Some((k, v)) = part.split_once(": ")
            && !empty.contains(&v)
        {
            out.push(format!("{k}={v}"));
        }
    };
    for (i, ch) in body.char_indices() {
        match ch {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth -= 1,
            ',' if depth == 0 => {
                push(&body[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    push(&body[start..]);
    out
}

fn coverage(cat: &Catalog, pb: &spire_core::playbook::Playbook) {
    use spire_core::analysis::sim::reading;
    println!("id\tstatus\treading\tunread\ttags");
    for c in cat.cards.values() {
        let r = reading(c, false, cat);
        let mut read: Vec<String> =
            set_fields(&format!("{:?}", r.now)).into_iter().filter(|f| f != "hits=1" && f != "block_times=1").collect();
        read.extend(
            r.channels.iter().map(|(o, n, x)| format!("channel {} {o}", if *x { "X".into() } else { n.to_string() })),
        );
        if r.x {
            read.push("X-cost".into());
        }
        if r.x_stars {
            read.push("X in Stars".into());
        }
        if r.star_cost > 0 {
            read.push(format!("{} Stars to play", r.star_cost));
        }
        if r.all_enemies {
            read.push("all enemies".into());
        }
        if r.shoots {
            read.push("shoots".into());
        }
        // A power acts through its triggers; of its card text only these
        // two fields are used.
        let power = c.card_type == CardType::Power;
        read.extend(
            set_fields(&format!("{:?}", r.text))
                .into_iter()
                .filter(|f| !power || f.starts_with("next_extra") || f.starts_with("gives_replay")),
        );
        read.extend(r.mech.describe());
        let unread = &r.mech.unread;
        let status = match (read.is_empty(), unread.is_empty()) {
            (_, true) => "read",
            (false, false) => "partly",
            (true, false) => "none",
        };
        let tags: Vec<String> =
            spire_core::analysis::tags::tags(c, false, pb).iter().map(|t| t.tag.name().to_string()).collect();
        println!("{}\t{status}\t{}\t{}\t{}", c.id, read.join("; "), unread.join(" / "), tags.join(", "));
    }
    let orbs: Vec<_> = cat.orbs.values().cloned().collect();
    for r in cat.relics.values() {
        let m = spire_core::mechanics::read(&r.text, &orbs, false);
        let status = match (m.is_empty(), m.unread.is_empty()) {
            (_, true) => "read",
            (false, false) => "partly",
            (true, false) => "none",
        };
        println!("{}\t{status}\t{}\t{}", r.id, m.describe().join("; "), m.unread.join(" / "));
    }
}

fn tags(cat: &Catalog, pb: &spire_core::playbook::Playbook) {
    use spire_core::analysis::tags::{self, Tag};
    let classes: Vec<String> = cat.characters.keys().map(|c| c.to_string().to_lowercase()).collect();
    let class_cards: Vec<_> = cat.cards.values().filter(|c| classes.contains(&c.color)).collect();
    println!("== tags by character ({} class cards)", class_cards.len());
    let mut untagged = Vec::new();
    for class in &classes {
        let mut counts: BTreeMap<Tag, u32> = BTreeMap::new();
        for c in class_cards.iter().filter(|c| &c.color == class) {
            let t = tags::tags(c, false, pb);
            if t.is_empty() {
                untagged.push(format!("{:<12} {:<24} | {}", c.color, c.name, c.text.replace('\n', " ")));
            }
            for x in t {
                *counts.entry(x.tag).or_default() += 1;
            }
        }
        let row: Vec<String> =
            Tag::ALL.iter().map(|t| format!("{}={}", t.name(), counts.get(t).unwrap_or(&0))).collect();
        println!("{class:<12} {}", row.join(" "));
    }
    println!("\n== untagged class cards: {}", untagged.len());
    for u in &untagged {
        println!("  {u}");
    }
}

fn lookout(cat: &Catalog, pb: &spire_core::Playbook, paths: &[String]) {
    use spire_core::analysis::deck::{classify_run, profile};
    use spire_core::analysis::events::holds;
    use spire_core::playbook::When;
    let mut samples = Vec::new();
    for p in paths {
        let bytes = std::fs::read(p).unwrap_or_else(|e| panic!("{p}: {e}"));
        let text = spire_data::fetch::decode_maybe_gzip(&bytes).unwrap_or_else(|e| panic!("{p}: {e}"));
        samples
            .extend(spire_data::benchmarks::runs(&text, cat).into_iter().flat_map(|r| r.boss_decks).map(|d| d.sample));
    }
    // Every rule, relic and event, by name and its `when`.
    let rules: Vec<(String, &When)> = pb
        .relic_fits
        .iter()
        .map(|f| (cat.relics.get(&f.relic).map_or(f.relic.to_string(), |r| r.name.clone()), &f.when))
        .chain(pb.event_fits.iter().map(|f| (format!("event: {}", f.title), &f.when)))
        .collect();
    println!("== relic_fits and event_fits against {} community decks going into a boss", samples.len());
    let mut acts: BTreeMap<u8, usize> = BTreeMap::new();
    for s in &samples {
        *acts.entry(s.act).or_default() += 1;
    }
    let mut hits: BTreeMap<(usize, u8), usize> = BTreeMap::new();
    for s in &samples {
        let deck = classify_run(&s.state, cat, pb);
        let prof = profile(&deck);
        for (i, (_, when)) in rules.iter().enumerate() {
            if holds(when, &s.state, &deck, &prof, cat, pb).is_some() {
                *hits.entry((i, s.act)).or_default() += 1;
            }
        }
    }
    for (i, (name, when)) in rules.iter().enumerate() {
        let mut rule = Vec::new();
        if when.cards.is_some() {
            rule.push(format!("{} {}", when.at_least, when.what));
        }
        if let Some(short) = when.short_of {
            rule.push(format!("short of {}", short.label()));
        }
        if let Some(below) = when.hp_below {
            rule.push(format!("HP under {:.0}%", below * 100.0));
        }
        let mut line = format!("{name:<26} {:<36}", rule.join(" & "));
        for (act, n) in &acts {
            let h = hits.get(&(i, *act)).copied().unwrap_or(0);
            line.push_str(&format!("  act {}: {:>5.1}%", act + 1, h as f32 * 100.0 / (*n).max(1) as f32));
        }
        println!("{line}");
    }
}

fn lines(cat: &Catalog) {
    use spire_core::analysis::lines::shrunk;
    let rate = |r: spire_core::catalog::Record| r.wins as f32 / r.fights.max(1) as f32;
    for a in &cat.card_lines {
        let base = rate(a.all);
        println!(
            "== {} act {}: {} boss fights, {:.0}% won, {} lines",
            a.character,
            a.act + 1,
            a.all.fights,
            base * 100.0,
            a.lines.len()
        );
        for l in a.lines.iter().take(8) {
            let names: Vec<String> = l.cards.iter().map(|c| cat.card_name(c)).collect();
            let offered: Vec<String> =
                l.cards.iter().map(|c| format!("{:.0}%", a.offered.get(c).copied().unwrap_or(0.0) * 100.0)).collect();
            println!(
                "   {:<48} {:>4} decks  {:>3.0}% (shrunk {:.0}%) vs {:.0}% apart  offered {}",
                names.join(" + "),
                l.record.fights,
                rate(l.record) * 100.0,
                shrunk(l.record, base) * 100.0,
                l.apart * 100.0,
                offered.join("/")
            );
        }
    }
    println!("== card lift: {} cards", cat.card_lift.len());
}

/// Boss fights by character and act: each deck's measures, whether it won,
/// and its weight.
type Fights = BTreeMap<(String, u8), Vec<(Vec<f64>, bool, f64)>>;

/// A player counts for at most this many runs.
const PLAYER_CAP: f64 = 20.0;

/// Weighted share of `xs` at or under each value: quantile `q` of weighted
/// values.
fn wquantile(xs: &[(f64, f64)], q: f64) -> f64 {
    let mut v: Vec<(f64, f64)> = xs.to_vec();
    v.sort_by(|a, b| a.0.total_cmp(&b.0));
    let total: f64 = v.iter().map(|x| x.1).sum();
    let mut acc = 0.0;
    for (x, w) in &v {
        acc += w;
        if acc >= q * total {
            return *x;
        }
    }
    v.last().map_or(0.0, |x| x.0)
}

/// Research for the role overhaul: A10 decks going into each act's boss,
/// their jobs, and how the boss win rate moves with each job. Runs are
/// deduplicated by hash and each player counts for at most 20 runs.
fn roles(cat: &Catalog, pb: &spire_core::Playbook, paths: &[String]) {
    use spire_core::analysis::jobs;
    use spire_core::analysis::sim::simulate;
    use std::collections::{BTreeSet, HashMap};
    let asc: u32 = std::env::var("ASC").ok().and_then(|a| a.parse().ok()).unwrap_or(10);
    let mut seen: BTreeSet<String> = BTreeSet::new();
    let mut runs = Vec::new();
    let (mut dupes, mut other_asc) = (0, 0);
    for p in paths {
        let bytes = std::fs::read(p).unwrap_or_else(|e| panic!("{p}: {e}"));
        let Ok(text) = spire_data::fetch::decode_maybe_gzip(&bytes) else {
            eprintln!("skipped {p}: unreadable");
            continue;
        };
        for r in spire_data::benchmarks::runs(&text, cat) {
            if let Some(h) = &r.meta.hash
                && !seen.insert(h.clone())
            {
                dupes += 1;
                continue;
            }
            if r.meta.ascension != asc {
                other_asc += 1;
                continue;
            }
            // BUILD=v0.111.0 keeps one build; BUILD=-v0.111.0 every other.
            if let Ok(b) = std::env::var("BUILD") {
                let this = r.meta.build.as_deref() == Some(b.trim_start_matches('-'));
                if this == b.starts_with('-') {
                    continue;
                }
            }
            runs.push(r);
        }
    }
    let mut per_player: HashMap<Option<String>, usize> = HashMap::new();
    for r in &runs {
        *per_player.entry(r.meta.player.clone()).or_default() += 1;
    }
    let anonymous = per_player.get(&None).copied().unwrap_or(0);
    // Anonymous runs: each its own player (no way to group them).
    let weight = |r: &spire_data::benchmarks::RunDecks| match &r.meta.player {
        None => 1.0,
        Some(_) => (PLAYER_CAP / per_player[&r.meta.player] as f64).min(1.0),
    };
    println!(
        "== A{asc}: {} runs ({} players, {anonymous} anonymous), {dupes} repeats dropped, {other_asc} other ascensions",
        runs.len(),
        per_player.len() - usize::from(anonymous > 0)
    );
    let mut builds: BTreeMap<String, usize> = BTreeMap::new();
    for r in &runs {
        *builds.entry(r.meta.build.clone().unwrap_or("?".into())).or_default() += 1;
    }
    println!("   builds: {builds:?}");
    // What's measured on each deck going into the boss: each role's job,
    // each role's share of the deck, its size, balance and reliability.
    let mut names: Vec<String> = Role::ALL.iter().map(|r| format!("job {}", r.label())).collect();
    names.extend(Role::ALL.iter().map(|r| format!("share {}", r.label())));
    names.extend(["deck size", "damage:block", "bad-draw floor", "dead draws", "damage spread"].map(String::from));
    let measure = |state: &spire_core::run::RunState| -> Vec<f64> {
        let sim = simulate(state, cat, &pb.simulation);
        let j = jobs::measure(&sim);
        let p = spire_core::analysis::deck::run_profile(state, cat, pb);
        let n = p.total.max(1) as f64;
        let mut v: Vec<f64> = Role::ALL.iter().map(|r| j.value(*r) as f64).collect();
        v.extend(Role::ALL.iter().map(|r| (p.role(*r) + p.starter_role(*r)) as f64 / n));
        let t3 = sim.turns.get(2).map(|t| t.damage);
        v.extend([
            n,
            j.fast_damage as f64 / (3.0 * j.mitigation as f64).max(1.0),
            j.floor as f64,
            sim.dead_draws as f64,
            t3.map_or(0.0, |d| ((d.high - d.low) / d.mean.max(1.0)) as f64),
        ]);
        v
    };
    // (character, act) -> [(measures, won, weight)]
    let mut fights: Fights = BTreeMap::new();
    for r in &runs {
        let w = weight(r);
        for b in &r.boss_decks {
            fights.entry((b.sample.character.to_string(), b.sample.act)).or_default().push((
                measure(&b.sample.state),
                b.won,
                w,
            ));
        }
    }
    pooled(&names, &fights);
    if std::env::var("POOLED_ONLY").is_ok() {
        return;
    }
    for ((ch, act), fs) in &fights {
        let tw: f64 = fs.iter().map(|f| f.2).sum();
        let ess = tw * tw / fs.iter().map(|f| f.2 * f.2).sum::<f64>();
        let wr = fs.iter().filter(|f| f.1).map(|f| f.2).sum::<f64>() / tw;
        println!("\n{ch} act {}: {} boss fights, effective {ess:.0}, won {:.0}%", act + 1, fs.len(), wr * 100.0);
        println!(
            "   {:<24} {:>6} {:>7} {:>7} | win% by fifth, low to high | under p25 n  win% | AUC",
            "measure", "p25", "p50", "p75"
        );
        for (m, name) in names.iter().enumerate() {
            let val = |f: &(Vec<f64>, bool, f64)| f.0[m];
            let vals: Vec<(f64, f64)> = fs.iter().map(|f| (val(f), f.2)).collect();
            // Cleared decks' quartiles: winners going in.
            let winners: Vec<(f64, f64)> = fs.iter().filter(|f| f.1).map(|f| (val(f), f.2)).collect();
            let (p25, p50, p75) = (wquantile(&winners, 0.25), wquantile(&winners, 0.5), wquantile(&winners, 0.75));
            let cuts: Vec<f64> = (1..5).map(|k| wquantile(&vals, k as f64 / 5.0)).collect();
            let mut bins = [(0.0f64, 0.0f64); 5];
            for f in fs {
                let b = cuts.iter().filter(|c| val(f) > **c).count();
                bins[b].0 += f.2;
                bins[b].1 += if f.1 { f.2 } else { 0.0 };
            }
            let fifths: Vec<String> = bins
                .iter()
                .map(|(n, w)| if *n > 0.0 { format!("{:>3.0}", w / n * 100.0) } else { "  -".into() })
                .collect();
            let under: Vec<_> = fs.iter().filter(|f| val(f) < p25).collect();
            let un: f64 = under.iter().map(|f| f.2).sum();
            let uw = under.iter().filter(|f| f.1).map(|f| f.2).sum::<f64>() / un.max(1e-9);
            // Weighted AUC: chance a winner's value beats a loser's.
            let (mut num, mut den) = (0.0, 0.0);
            for a in fs.iter().filter(|f| f.1) {
                for b in fs.iter().filter(|f| !f.1) {
                    let w = a.2 * b.2;
                    den += w;
                    num += w * match val(a).total_cmp(&val(b)) {
                        std::cmp::Ordering::Greater => 1.0,
                        std::cmp::Ordering::Equal => 0.5,
                        std::cmp::Ordering::Less => 0.0,
                    };
                }
            }
            println!(
                "   {name:<24} {p25:>6.2} {p50:>7.2} {p75:>7.2} | {}  | {:>6.0} {:>4.0}% | {:.2}",
                fifths.join(" "),
                un,
                uw * 100.0,
                num / den.max(1e-9)
            );
        }
    }
}

/// Every character's decks pooled per act, each placed by its percentile
/// among its own character's decks going into that act's boss: win rate of
/// the bottom quarter, middle half and top quarter, and how far each end
/// sits from the middle, with a 90% bootstrap interval.
fn pooled(names: &[String], fights: &Fights) {
    println!("\n== Pooled across characters (each deck placed within its own character and act)");
    println!("   {:<24} act | bottom 1/4  middle  top 1/4 | low - mid [90%]      | high - mid [90%]", "measure");
    let acts: std::collections::BTreeSet<u8> = fights.keys().map(|k| k.1).filter(|a| *a < 3).collect();
    for (m, name) in names.iter().enumerate() {
        for act in &acts {
            // (band, won, weight): band 0 bottom quarter, 1 middle, 2 top.
            let mut rows: Vec<(usize, bool, f64)> = Vec::new();
            for (_, fs) in fights.iter().filter(|(k, _)| k.1 == *act) {
                let vals: Vec<(f64, f64)> = fs.iter().map(|f| (f.0[m], f.2)).collect();
                let (lo, hi) = (wquantile(&vals, 0.25), wquantile(&vals, 0.75));
                for f in fs {
                    let v = f.0[m];
                    // Ties at a cut stay in the middle.
                    let band = if v < lo {
                        0
                    } else if v > hi {
                        2
                    } else {
                        1
                    };
                    rows.push((band, f.1, f.2));
                }
            }
            let rate = |rows: &[(usize, bool, f64)], band: usize| {
                let (n, w) = rows
                    .iter()
                    .filter(|r| r.0 == band)
                    .fold((0.0, 0.0), |(n, w), r| (n + r.2, w + if r.1 { r.2 } else { 0.0 }));
                if n > 0.0 { w / n } else { f64::NAN }
            };
            let point = [rate(&rows, 0), rate(&rows, 1), rate(&rows, 2)];
            // Bootstrap: resample decks with replacement (simple LCG).
            let mut seed: u64 = 0x9E37_79B9_7F4A_7C15 ^ (m as u64 * 31 + *act as u64);
            let mut next = || {
                seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
                (seed >> 33) as usize
            };
            let (mut lows, mut highs) = (Vec::new(), Vec::new());
            for _ in 0..400 {
                let sample: Vec<(usize, bool, f64)> = (0..rows.len()).map(|_| rows[next() % rows.len()]).collect();
                let (a, b, c) = (rate(&sample, 0), rate(&sample, 1), rate(&sample, 2));
                lows.push(a - b);
                highs.push(c - b);
            }
            let ci = |v: &mut Vec<f64>| {
                v.retain(|x| x.is_finite());
                v.sort_by(|a, b| a.total_cmp(b));
                if v.is_empty() { (f64::NAN, f64::NAN) } else { (v[v.len() * 5 / 100], v[v.len() * 95 / 100]) }
            };
            let (l, h) = (ci(&mut lows), ci(&mut highs));
            let mark = |lo: f64, hi: f64| {
                if hi < 0.0 {
                    " worse"
                } else if lo > 0.0 {
                    " better"
                } else {
                    ""
                }
            };
            println!(
                "   {name:<24} {:>3} |   {:>5.1}%   {:>5.1}%  {:>5.1}% | {:>+5.1} [{:>+5.1},{:>+5.1}]{:<7} | {:>+5.1} [{:>+5.1},{:>+5.1}]{}",
                act + 1,
                point[0] * 100.0,
                point[1] * 100.0,
                point[2] * 100.0,
                (point[0] - point[1]) * 100.0,
                l.0 * 100.0,
                l.1 * 100.0,
                mark(l.0, l.1),
                (point[2] - point[1]) * 100.0,
                h.0 * 100.0,
                h.1 * 100.0,
                mark(h.0, h.1)
            );
        }
    }
}

/// Research for the fight demands: A10 decks going into each elite and boss
/// (deduplicated, each player capped at 20 runs), and per encounter how
/// each measure separates the decks that died there from those that won.
fn fights(cat: &Catalog, pb: &spire_core::Playbook, paths: &[String]) {
    use spire_core::analysis::jobs;
    use spire_core::analysis::sim::simulate;
    use spire_core::analysis::threats::{Scaling, demands};
    use spire_core::run::NodeKind;
    use std::collections::{BTreeSet, HashMap};
    let asc: u32 = std::env::var("ASC").ok().and_then(|a| a.parse().ok()).unwrap_or(10);
    let version =
        |b: &str| -> Vec<u32> { b.trim_start_matches('v').split('.').map(|p| p.parse().unwrap_or(0)).collect() };
    let min = version(&pb.composition.min_build);
    let mut seen: BTreeSet<String> = BTreeSet::new();
    let mut runs = Vec::new();
    for p in paths {
        let bytes = std::fs::read(p).unwrap_or_else(|e| panic!("{p}: {e}"));
        let Ok(text) = spire_data::fetch::decode_maybe_gzip(&bytes) else { continue };
        for line in text.lines() {
            let r = spire_data::benchmarks::fight_decks(line, cat, &[NodeKind::Elite, NodeKind::Boss]);
            if r.boss_decks.is_empty() || r.meta.ascension != asc {
                continue;
            }
            if r.meta.build.as_deref().is_none_or(|b| version(b) < min) {
                continue;
            }
            if r.meta.hash.as_ref().is_some_and(|h| !seen.insert(h.clone())) {
                continue;
            }
            runs.push(r);
        }
        eprint!(".");
    }
    eprintln!();
    let mut per_player: HashMap<Option<String>, usize> = HashMap::new();
    for r in &runs {
        *per_player.entry(r.meta.player.clone()).or_default() += 1;
    }
    let weight = |r: &spire_data::benchmarks::RunDecks| match &r.meta.player {
        None => 1.0,
        Some(_) => (PLAYER_CAP / per_player[&r.meta.player] as f64).min(1.0),
    };
    let mut names: Vec<String> = Role::ALL.iter().map(|r| format!("job {}", r.label())).collect();
    names.extend(Role::ALL.iter().map(|r| format!("share {}", r.label())));
    names.extend(["deck size", "damage:block", "dead draws", "entering HP%"].map(String::from));
    let measure = |state: &spire_core::run::RunState| -> Vec<f64> {
        let sim = simulate(state, cat, &pb.simulation);
        let j = jobs::measure(&sim);
        let p = spire_core::analysis::deck::run_profile(state, cat, pb);
        let n = p.total.max(1) as f64;
        let mut v: Vec<f64> = Role::ALL.iter().map(|r| j.value(*r) as f64).collect();
        v.extend(Role::ALL.iter().map(|r| (p.role(*r) + p.starter_role(*r)) as f64 / n));
        v.extend([
            n,
            j.fast_damage as f64 / (3.0 * j.mitigation as f64).max(1.0),
            sim.dead_draws as f64,
            state.hp as f64 / state.max_hp.max(1) as f64,
        ]);
        v
    };
    // encounter -> [(measures, won, weight)]
    let mut by_enc: BTreeMap<String, Vec<(Vec<f64>, bool, f64)>> = BTreeMap::new();
    // The survival check against what happened: (encounter, verdict, lead
    // way, predicted loss over HP, won, weight).
    type Called = (String, &'static str, &'static str, f64, bool, f64);
    let mut called: Vec<Called> = Vec::new();
    let check = std::env::var("OUTLOOK").is_ok();
    let mut kind_of: BTreeMap<String, (NodeKind, u8)> = BTreeMap::new();
    for r in &runs {
        let w = weight(r);
        for d in &r.boss_decks {
            let Some(e) = &d.encounter else { continue };
            kind_of.insert(e.to_string(), (d.kind, d.sample.act));
            by_enc.entry(e.to_string()).or_default().push((measure(&d.sample.state), d.won, w));
            if check && let Some(enc) = cat.encounters.get(e) {
                let st = &d.sample.state;
                let sim = simulate(st, cat, &pb.simulation);
                if let Some(o) = spire_core::analysis::survival::for_encounter(&sim, enc, cat, st, pb) {
                    let loss = o.lead.hp_loss as f64 / st.hp.max(1) as f64;
                    let loss = if o.lead.kill_turn.is_none() { 9.0 } else { loss };
                    called.push((e.to_string(), o.verdict.label(), o.lead.way.label(), loss, d.won, w));
                }
            }
        }
    }
    if check {
        calibration(&called, cat);
    }
    println!(
        "== A{asc}, builds >= {}: {} runs, {} fights",
        pb.composition.min_build,
        runs.len(),
        by_enc.values().map(Vec::len).sum::<usize>()
    );
    let mut keys: Vec<&String> = by_enc.keys().collect();
    keys.sort_by_key(|k| (kind_of[*k].1, kind_of[*k].0 == NodeKind::Boss, (*k).clone()));
    for k in keys {
        let fs = &by_enc[k];
        let (kind, act) = kind_of[k];
        let tw: f64 = fs.iter().map(|f| f.2).sum();
        let lost: f64 = fs.iter().filter(|f| !f.1).map(|f| f.2).sum();
        let enc = cat.encounters.get(&spire_core::EncounterId::new(k));
        let asks: Vec<String> = enc
            .map(|e| {
                let t = threat_profile(e, cat, pb, Scaling { tough: asc >= 8, deadly: asc >= 9 });
                demands(&t, e.room, pb.act(act), None)
                    .into_iter()
                    .map(|d| format!("{} ({})", d.role.label(), d.reason))
                    .collect()
            })
            .unwrap_or_default();
        println!(
            "\n{} [{:?}, act {}]: {:.0} fights, {:.0} deaths ({:.1}%)",
            enc.map_or(k.as_str(), |e| e.name.as_str()),
            kind,
            act + 1,
            tw,
            lost,
            lost / tw * 100.0
        );
        println!("   playbook asks: {}", if asks.is_empty() { "nothing".into() } else { asks.join("; ") });
        if lost < 15.0 {
            println!("   (too few deaths to measure)");
            continue;
        }
        println!("   {:<22} {:>7} {:>7} | death% low q / mid / high q | AUC", "measure", "p25", "p75");
        for (m, name) in names.iter().enumerate() {
            let vals: Vec<(f64, f64)> = fs.iter().map(|f| (f.0[m], f.2)).collect();
            let (p25, p75) = (wquantile(&vals, 0.25), wquantile(&vals, 0.75));
            let rate = |pick: &dyn Fn(f64) -> bool| {
                let (n, d) = fs
                    .iter()
                    .filter(|f| pick(f.0[m]))
                    .fold((0.0, 0.0), |(n, d), f| (n + f.2, d + if f.1 { 0.0 } else { f.2 }));
                if n > 0.0 { format!("{:>5.1}", d / n * 100.0) } else { "    -".into() }
            };
            let (mut num, mut den) = (0.0, 0.0);
            for a in fs.iter().filter(|f| f.1) {
                for b in fs.iter().filter(|f| !f.1) {
                    let w = a.2 * b.2;
                    den += w;
                    num += w * match a.0[m].total_cmp(&b.0[m]) {
                        std::cmp::Ordering::Greater => 1.0,
                        std::cmp::Ordering::Equal => 0.5,
                        std::cmp::Ordering::Less => 0.0,
                    };
                }
            }
            println!(
                "   {name:<22} {p25:>7.2} {p75:>7.2} | {} {} {} | {:.2}",
                rate(&|v| v < p25),
                rate(&|v| v >= p25 && v <= p75),
                rate(&|v| v > p75),
                num / den.max(1e-9)
            );
        }
    }
}

/// The survival check's calls against what happened: death rate by verdict,
/// by lead way and by predicted loss, overall and per encounter.
fn calibration(called: &[(String, &'static str, &'static str, f64, bool, f64)], cat: &Catalog) {
    let table = |title: &str, rows: &[&(String, &'static str, &'static str, f64, bool, f64)]| {
        let mut by: BTreeMap<String, (f64, f64)> = BTreeMap::new();
        for r in rows {
            for key in [format!("verdict {}", r.1), format!("way {}", r.2), {
                let b = if r.3 >= 9.0 {
                    "no win".to_string()
                } else {
                    format!("loss {:.1}", (r.3 * 5.0).floor().min(6.0) / 5.0)
                };
                format!("pred {b}")
            }] {
                let e = by.entry(key).or_default();
                e.0 += r.5;
                e.1 += if r.4 { 0.0 } else { r.5 };
            }
        }
        let cells: Vec<String> = by.iter().map(|(k, (n, d))| format!("{k}: {n:.0} {:.1}%", d / n * 100.0)).collect();
        println!("{title}\n   {}", cells.join("\n   "));
    };
    let all: Vec<_> = called.iter().collect();
    table("== survival check vs deaths, all elite and boss fights", &all);
    let mut encs: Vec<&String> = called.iter().map(|c| &c.0).collect();
    encs.sort();
    encs.dedup();
    for e in encs {
        let rows: Vec<_> = called.iter().filter(|c| &c.0 == e).collect();
        let name = cat.encounters.get(&spire_core::EncounterId::new(e)).map_or(e.as_str(), |x| x.name.as_str());
        table(&format!("-- {name}"), &rows);
    }
    println!();
}
