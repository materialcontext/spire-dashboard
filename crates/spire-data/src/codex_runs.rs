//! Exact encounter lineups and outcomes from the Spire Codex run export
//! (`/api/exports/runs`, JSON lines, one community-submitted run each).
//!
//! The encounter endpoint lists each monster *type* once. The run export
//! records every monster in every fight, so aggregating it gives exact
//! starting lineups (with counts), the monsters that join mid-fight, and
//! average damage taken and turns.
//!
//! Pure: decompressed JSONL text in, enriched [`DataPack`] out.
//!
//! Reading a fight's `monster_ids`:
//! - **Allies** are dropped: player-side units (Necrobinder's Osty, pets,
//!   relic summons) that are not part of the encounter yet show up across
//!   many encounters.
//! - **Anchors** are monster types present at the same count in at least
//!   95% of fights. They are the fixed starting lineup.
//! - If the anchors alone also occur as a whole fight, everything extra was
//!   a mid-fight **spawn** (summons, splits, on-death adds).
//! - Otherwise the encounter draws from a random pool, and each observed
//!   lineup is a **variant** with its share.

use std::collections::{BTreeMap, BTreeSet};

use spire_core::DataPack;
use spire_core::catalog::{EncounterStats, FixedRoom, GameRules, Lineup, NodeKind};
use spire_core::ids::MonsterId;

use crate::run_file::{WRun, entry, node_kind};

/// Fewer fights than this and an encounter is left unenriched.
pub const MIN_FIGHTS: usize = 20;
const ANCHOR_SHARE: f32 = 0.95;
const BASE_MIN_SHARE: f32 = 0.05;
const VARIANT_MIN_SHARE: f32 = 0.03;
/// A non-member monster seen in this many encounters is a player ally.
const ALLY_SPREAD: usize = 5;

/// One fight from one run.
#[derive(Clone, Debug, PartialEq)]
pub struct Fight {
    /// Index of the run within the input (groups fights by run).
    pub run: usize,
    /// 0-based act within the run.
    pub act: usize,
    pub encounter: String,
    /// Sorted monster ids, prefix stripped, duplicates kept.
    pub monsters: Vec<String>,
    pub damage_taken: Option<u32>,
    pub turns: Option<u32>,
    /// HP going into the fight (after the previous room).
    pub hp_in: Option<u32>,
    /// The run ended in this fight.
    pub died: bool,
}

fn is_empty_value(v: &Option<serde_json::Value>) -> bool {
    match v {
        None | Some(serde_json::Value::Null) => true,
        Some(serde_json::Value::Array(a)) => a.is_empty(),
        Some(serde_json::Value::Object(o)) => o.is_empty(),
        Some(_) => false,
    }
}

/// Extracts single-player, unmodified fights from JSONL. Bad lines are skipped.
pub fn fights(jsonl: &str) -> Vec<Fight> {
    let runs = jsonl
        .lines()
        .filter_map(|line| serde_json::from_str::<WRun>(line).ok())
        .filter(|run| run.players.len() == 1 && is_empty_value(&run.modifiers));
    let mut out = Vec::new();
    for (r, run) in runs.enumerate() {
        let killer = run.killed_by_encounter.as_deref().map(entry).filter(|k| *k != "NONE");
        let lost = run.win == Some(false);
        let points = run.map_point_history.len();
        let mut hp_before: Option<u32> = None;
        for (act, acts) in run.map_point_history.into_iter().enumerate() {
            let last_act = act + 1 == points;
            let n = acts.len();
            for (i, point) in acts.into_iter().enumerate() {
                let (damage, hp_after) = match point.player_stats.as_slice() {
                    [only] => (only.damage_taken, only.current_hp.map(|hp| hp.max(0) as u32)),
                    _ => (None, None),
                };
                let last = last_act && i + 1 == n;
                for room in point.rooms.into_iter().filter(|r| !r.monster_ids.is_empty()) {
                    let mut monsters: Vec<String> = room.monster_ids.iter().map(|m| entry(m).to_string()).collect();
                    monsters.sort();
                    let encounter = entry(&room.model_id).to_string();
                    let died = last && lost && killer == Some(encounter.as_str());
                    out.push(Fight {
                        run: r,
                        act,
                        encounter,
                        monsters,
                        damage_taken: damage,
                        turns: room.turns_taken,
                        hp_in: hp_before,
                        died,
                    });
                }
                if hp_after.is_some() {
                    hp_before = hp_after;
                }
            }
        }
    }
    out
}

/// Fights from several export pages, with run indices kept unique across
/// pages (each page numbers its runs from zero).
pub fn fights_from_pages(pages: &[String]) -> Vec<Fight> {
    let mut out: Vec<Fight> = Vec::new();
    for page in pages {
        let offset = out.last().map_or(0, |f| f.run + 1);
        out.extend(fights(page).into_iter().map(|mut f| {
            f.run += offset;
            f
        }));
    }
    out
}

/// The room sequence of one act that reached its boss.
#[derive(Clone, Debug, PartialEq)]
pub struct ActLayout {
    pub act: usize,
    pub ascension: u32,
    pub rooms: Vec<NodeKind>,
}

/// Room sequences of completed acts (last room a boss), single-player and
/// unmodified runs only.
pub fn layouts(jsonl: &str) -> Vec<ActLayout> {
    jsonl
        .lines()
        .filter_map(|line| serde_json::from_str::<WRun>(line).ok())
        .filter(|run| run.players.len() == 1 && is_empty_value(&run.modifiers))
        .flat_map(|run| {
            let ascension = run.ascension;
            run.map_point_history.into_iter().enumerate().filter_map(move |(act, points)| {
                let rooms: Option<Vec<NodeKind>> =
                    points.iter().map(|p| p.map_point_type.as_deref().and_then(node_kind)).collect();
                let rooms = rooms?;
                (rooms.last() == Some(&NodeKind::Boss)).then_some(ActLayout { act, ascension, rooms })
            })
        })
        .collect()
}

/// Share of acts a floor must agree on to count as fixed.
const FIXED_SHARE: f32 = 0.97;

/// Per act index: the usual length and the floors whose room never varies
/// (counted from the start, and from the end as negative positions).
/// Double-boss acts are left out; `floors::expected_room` adds the extra
/// boss floor itself.
pub fn act_shapes(layouts: &[ActLayout], rules: &GameRules) -> Vec<(u8, u32, Vec<FixedRoom>)> {
    let mut by_act: BTreeMap<usize, Vec<&ActLayout>> = BTreeMap::new();
    for l in layouts {
        let index = u8::try_from(l.act).unwrap_or(u8::MAX);
        if !rules.double_boss(l.ascension, index) {
            by_act.entry(l.act).or_default().push(l);
        }
    }
    by_act
        .into_iter()
        .filter(|(_, ls)| ls.len() >= MIN_FIGHTS)
        .filter_map(|(act, ls)| {
            let mut lengths: BTreeMap<usize, usize> = BTreeMap::new();
            for l in &ls {
                *lengths.entry(l.rooms.len()).or_default() += 1;
            }
            let (length, _) = lengths.into_iter().max_by_key(|(_, n)| *n)?;
            let fixed_at = |pick: &dyn Fn(&ActLayout) -> Option<NodeKind>| {
                let mut counts: BTreeMap<String, (NodeKind, usize)> = BTreeMap::new();
                let mut n = 0;
                for l in &ls {
                    if let Some(k) = pick(l) {
                        n += 1;
                        counts.entry(format!("{k:?}")).or_insert((k, 0)).1 += 1;
                    }
                }
                let (k, c) = counts.into_values().max_by_key(|(_, c)| *c)?;
                (n > 0 && c as f32 / n as f32 >= FIXED_SHARE).then_some(k)
            };
            let mut fixed = Vec::new();
            for i in 0..length {
                if let Some(room) = fixed_at(&|l: &ActLayout| l.rooms.get(i).copied()) {
                    fixed.push(FixedRoom { at: i as i32, room });
                }
            }
            for back in 1..=3usize {
                if let Some(room) = fixed_at(&|l: &ActLayout| l.rooms.len().checked_sub(back).map(|i| l.rooms[i])) {
                    fixed.push(FixedRoom { at: -(back as i32), room });
                }
            }
            // Keep start-counted floors only where they are not the end
            // floors seen from the front (those vary with act length).
            fixed.retain(|f| f.at < 0 || (f.at as usize) < length.saturating_sub(3));
            Some((u8::try_from(act).ok()?, length as u32, fixed))
        })
        .collect()
}

/// Writes act lengths and fixed rooms onto the pack's acts.
pub fn enrich_layouts(pack: &mut DataPack, layouts: &[ActLayout]) -> usize {
    let rules = pack.rules.clone().unwrap_or_default();
    let shapes = act_shapes(layouts, &rules);
    for act in &mut pack.acts {
        if let Some((_, length, fixed)) = shapes.iter().find(|(i, _, _)| *i == act.index) {
            act.length = Some(*length);
            act.fixed_rooms = fixed.clone();
        }
    }
    shapes.len()
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct EnrichSummary {
    pub fights: usize,
    pub encounters: usize,
    /// `(act index, weak fights)` measured per act.
    pub weak_fights: Vec<(u8, u32)>,
}

/// The usual number of weak-pool hallway fights in each act index, from
/// acts that reached their boss (so short, dead runs don't skew it).
/// Repeats rarer than this share count as never happening.
const NEVER_SHARE: f32 = 0.005;
/// Fewer fights than this and the repeat rules stay unmeasured.
const RULE_MIN_FIGHTS: usize = 500;

/// How encounters repeat within an act: whether a hallway encounter ever
/// comes up twice, and whether an elite ever comes up twice in a row.
/// Each is `Some(true)` when it (practically) never happens, `None` when
/// there are too few fights to say.
pub fn repeat_rules(pack: &DataPack, fights: &[Fight]) -> (Option<bool>, Option<bool>) {
    use spire_core::catalog::RoomType;
    let rooms: BTreeMap<&str, RoomType> = pack.encounters.iter().map(|e| (e.id.as_str(), e.room)).collect();
    // (run, act) -> (hallway encounters, elite encounters), in order.
    type Seen<'a> = (Vec<&'a str>, Vec<&'a str>);
    let mut acts: BTreeMap<(usize, usize), Seen> = BTreeMap::new();
    for f in fights {
        let entry = acts.entry((f.run, f.act)).or_default();
        match rooms.get(f.encounter.as_str()) {
            Some(RoomType::Monster) => entry.0.push(&f.encounter),
            Some(RoomType::Elite) => entry.1.push(&f.encounter),
            _ => {}
        }
    }
    let (mut hall, mut hall_repeats, mut elites, mut elite_repeats) = (0usize, 0usize, 0usize, 0usize);
    for (halls, els) in acts.values() {
        for (i, e) in halls.iter().enumerate() {
            hall += 1;
            hall_repeats += usize::from(halls[..i].contains(e));
        }
        for (i, e) in els.iter().enumerate() {
            elites += 1;
            elite_repeats += usize::from(i > 0 && els[i - 1] == *e);
        }
    }
    let never = |n: usize, repeats: usize| (n >= RULE_MIN_FIGHTS).then_some((repeats as f32) < n as f32 * NEVER_SHARE);
    (never(hall, hall_repeats), never(elites, elite_repeats))
}

pub fn weak_fights_per_act(pack: &DataPack, fights: &[Fight]) -> Vec<(u8, u32)> {
    use spire_core::catalog::RoomType;
    let encounters: BTreeMap<&str, (RoomType, bool)> =
        pack.encounters.iter().map(|e| (e.id.as_str(), (e.room, e.weak))).collect();
    // (run, act) -> (weak count, reached boss)
    let mut acts: BTreeMap<(usize, usize), (u32, bool)> = BTreeMap::new();
    for f in fights {
        if let Some((room, weak)) = encounters.get(f.encounter.as_str()) {
            let e = acts.entry((f.run, f.act)).or_default();
            e.0 += u32::from(*room == RoomType::Monster && *weak);
            e.1 |= *room == RoomType::Boss;
        }
    }
    let mut by_act: BTreeMap<usize, BTreeMap<u32, usize>> = BTreeMap::new();
    for ((_, act), (weak, boss)) in acts {
        if boss {
            *by_act.entry(act).or_default().entry(weak).or_default() += 1;
        }
    }
    by_act
        .into_iter()
        .filter(|(_, counts)| counts.values().sum::<usize>() >= MIN_FIGHTS)
        .filter_map(|(act, counts)| {
            let (mode, _) = counts.into_iter().max_by_key(|(_, n)| *n)?;
            Some((u8::try_from(act).ok()?, mode))
        })
        .collect()
}

fn count_of(monsters: &[String], id: &str) -> usize {
    monsters.iter().filter(|m| *m == id).count()
}

/// What one encounter's fights say about it.
#[derive(Clone, Debug, PartialEq)]
pub struct Reading {
    pub lineups: Vec<Lineup>,
    pub spawns: Vec<MonsterId>,
    pub stats: EncounterStats,
}

/// Reads one encounter's fights (allies already removed).
pub fn read_encounter(fights: &[&[String]], summoner: bool) -> Option<Reading> {
    let n = fights.len();
    if n < MIN_FIGHTS {
        return None;
    }
    let share = |count: usize| count as f32 / n as f32;
    let types: BTreeSet<&String> = fights.iter().flat_map(|f| f.iter()).collect();

    // Anchors: types at one fixed count in ≥95% of fights.
    let mut anchors: Vec<String> = Vec::new();
    let mut anchor_counts: BTreeMap<&String, usize> = BTreeMap::new();
    for t in &types {
        let mut by_count: BTreeMap<usize, usize> = BTreeMap::new();
        for f in fights {
            *by_count.entry(count_of(f, t)).or_default() += 1;
        }
        if let Some((&count, &seen)) = by_count.iter().max_by_key(|(_, seen)| **seen)
            && count > 0
            && share(seen) >= ANCHOR_SHARE
        {
            anchors.extend(std::iter::repeat_n((*t).clone(), count));
            anchor_counts.insert(t, count);
        }
    }
    anchors.sort();

    let mut lineups: BTreeMap<&[String], usize> = BTreeMap::new();
    for f in fights {
        *lineups.entry(f).or_default() += 1;
    }
    let base_share = share(lineups.get(anchors.as_slice()).copied().unwrap_or(0));
    let new_types = types.iter().any(|t| !anchor_counts.contains_key(t));
    let extras = base_share < 1.0;
    let spawn_mode = !anchors.is_empty() && base_share >= BASE_MIN_SHARE && extras && (summoner || new_types);

    let to_ids = |ms: &[String]| ms.iter().map(|m| MonsterId::new(m.as_str())).collect::<Vec<_>>();
    let (lineups, spawns) = if spawn_mode {
        let spawns = types
            .iter()
            .filter(|t| fights.iter().any(|f| count_of(f, t) > anchor_counts.get(*t).copied().unwrap_or(0)))
            .map(|t| MonsterId::new(t.as_str()))
            .collect();
        (vec![Lineup { monsters: to_ids(&anchors), share: 1.0 }], spawns)
    } else {
        let mut ls: Vec<Lineup> = lineups
            .iter()
            .filter(|(_, c)| share(**c) >= VARIANT_MIN_SHARE)
            .map(|(ms, c)| Lineup { monsters: to_ids(ms), share: share(*c) })
            .collect();
        ls.sort_by(|a, b| b.share.total_cmp(&a.share));
        (ls, Vec::new())
    };

    Some(Reading { lineups, spawns, stats: EncounterStats { fights: n as u32, ..Default::default() } })
}

fn average(values: impl Iterator<Item = u32>) -> f32 {
    let (sum, n) = values.fold((0u64, 0u32), |(s, n), v| (s + u64::from(v), n + 1));
    if n == 0 { 0.0 } else { (sum as f64 / f64::from(n)) as f32 }
}

/// Turns tracked one by one in the death histogram; later deaths count
/// in the last bucket.
pub const DEATH_TURNS: usize = 15;

fn percentile(mut values: Vec<u32>, p: f32) -> Option<u32> {
    if values.is_empty() {
        return None;
    }
    values.sort_unstable();
    let i = ((values.len() as f32 * p) as usize).min(values.len() - 1);
    Some(values[i])
}

/// Deaths, the turns they happened on, what survivors took and the HP
/// both groups came in with.
fn outcomes<'a>(stats: &mut EncounterStats, fights: impl Iterator<Item = &'a Fight>) {
    let fights: Vec<&Fight> = fights.collect();
    let dead: Vec<&&Fight> = fights.iter().filter(|f| f.died).collect();
    stats.deaths = dead.len() as u32;
    stats.death_turns = vec![0; DEATH_TURNS];
    for f in &dead {
        if let Some(t) = f.turns.filter(|t| *t > 0) {
            stats.death_turns[(t as usize).min(DEATH_TURNS) - 1] += 1;
        }
    }
    while stats.death_turns.last() == Some(&0) {
        stats.death_turns.pop();
    }
    let won: Vec<&&Fight> = fights.iter().filter(|f| !f.died).collect();
    let damage: Vec<u32> = won.iter().filter_map(|f| f.damage_taken).collect();
    stats.win_damage = percentile(damage.clone(), 0.5).zip(percentile(damage, 0.9));
    let turns: Vec<u32> = won.iter().filter_map(|f| f.turns.filter(|t| *t > 0)).collect();
    stats.win_turns = percentile(turns.clone(), 0.5).zip(percentile(turns, 0.9));
    let hp = |list: &[&&Fight]| percentile(list.iter().filter_map(|f| f.hp_in).collect(), 0.5);
    stats.hp_in = hp(&won).zip(hp(&dead));
}

/// Fills `lineups`, `spawns` and `stats` on the pack's encounters.
pub fn enrich(pack: &mut DataPack, fights: &[Fight]) -> EnrichSummary {
    let known_monsters: BTreeSet<&str> = pack.monsters.iter().map(|m| m.id.as_str()).collect();
    let members: BTreeMap<&str, BTreeSet<&str>> =
        pack.encounters.iter().map(|e| (e.id.as_str(), e.monsters.iter().map(|m| m.as_str()).collect())).collect();

    // Allies: non-member monsters that show up across many encounters.
    let mut foreign: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for f in fights {
        if let Some(own) = members.get(f.encounter.as_str()) {
            for m in f.monsters.iter().filter(|m| !own.contains(m.as_str())) {
                foreign.entry(m).or_default().insert(&f.encounter);
            }
        }
    }
    let allies: BTreeSet<&str> = foreign.into_iter().filter(|(_, e)| e.len() >= ALLY_SPREAD).map(|(m, _)| m).collect();

    let mut by_encounter: BTreeMap<&str, Vec<(Vec<String>, &Fight)>> = BTreeMap::new();
    for f in fights.iter().filter(|f| members.contains_key(f.encounter.as_str())) {
        let monsters: Vec<String> = f
            .monsters
            .iter()
            .filter(|m| known_monsters.contains(m.as_str()) && !allies.contains(m.as_str()))
            .cloned()
            .collect();
        // A fight with an unknown (e.g. modded) monster is not trustworthy.
        if monsters.len() + f.monsters.iter().filter(|m| allies.contains(m.as_str())).count() == f.monsters.len()
            && !monsters.is_empty()
        {
            by_encounter.entry(&f.encounter).or_default().push((monsters, f));
        }
    }

    let summoners: BTreeSet<&str> = pack
        .monsters
        .iter()
        .filter(|m| m.moves.iter().any(|mv| mv.intent_has("summon")))
        .map(|m| m.id.as_str())
        .collect();

    let mut readings: BTreeMap<String, Reading> = BTreeMap::new();
    let mut total = 0;
    for (enc, list) in &by_encounter {
        let lineups: Vec<&[String]> = list.iter().map(|(m, _)| m.as_slice()).collect();
        let summoner = members[enc].iter().any(|m| summoners.contains(m));
        if let Some(mut r) = read_encounter(&lineups, summoner) {
            r.stats.avg_damage_taken = average(list.iter().filter_map(|(_, f)| f.damage_taken));
            r.stats.avg_turns = average(list.iter().filter_map(|(_, f)| f.turns));
            outcomes(&mut r.stats, list.iter().map(|(_, f)| *f));
            total += list.len();
            readings.insert((*enc).to_string(), r);
        }
    }

    let weak = weak_fights_per_act(pack, fights);
    let (unique_hallway, no_elite_twice) = repeat_rules(pack, fights);
    if let Some(rules) = pack.rules.as_mut() {
        if let Some(r) = unique_hallway {
            rules.unique_hallway_fights = r;
        }
        if let Some(r) = no_elite_twice {
            rules.no_elite_twice_in_a_row = r;
        }
    }
    for act in &mut pack.acts {
        if let Some((_, n)) = weak.iter().find(|(i, _)| *i == act.index) {
            act.weak_fights = Some(*n);
        }
    }
    let mut summary = EnrichSummary { fights: total, encounters: 0, weak_fights: weak };
    for e in &mut pack.encounters {
        if let Some(r) = readings.remove(e.id.as_str()) {
            e.lineups = r.lineups;
            e.spawns = r.spawns;
            e.stats = Some(r.stats);
            summary.encounters += 1;
        }
    }
    summary
}

#[cfg(test)]
mod tests {
    use super::*;

    fn obs(spec: &[(&[&str], usize)]) -> Vec<Vec<String>> {
        spec.iter()
            .flat_map(|(ms, n)| {
                let mut v: Vec<String> = ms.iter().map(|s| s.to_string()).collect();
                v.sort();
                std::iter::repeat_n(v, *n)
            })
            .collect()
    }

    fn read(spec: &[(&[&str], usize)], summoner: bool) -> Reading {
        let o = obs(spec);
        let refs: Vec<&[String]> = o.iter().map(|v| v.as_slice()).collect();
        read_encounter(&refs, summoner).expect("enough fights")
    }

    fn names(l: &Lineup) -> Vec<&str> {
        l.monsters.iter().map(|m| m.as_str()).collect()
    }

    #[test]
    fn fixed_group_keeps_counts() {
        let r = read(&[(&["SLUG", "SLUG", "SLUG"], 50)], false);
        assert_eq!(r.lineups.len(), 1);
        assert_eq!(names(&r.lineups[0]), ["SLUG", "SLUG", "SLUG"]);
        assert!(r.spawns.is_empty());
    }

    #[test]
    fn mid_fight_adds_are_spawns() {
        // Fogmog-style: the base lineup alone is common, an add joins later.
        let r = read(&[(&["FOGMOG", "EYE"], 60), (&["FOGMOG"], 40)], true);
        assert_eq!(r.lineups.len(), 1);
        assert_eq!(names(&r.lineups[0]), ["FOGMOG"]);
        assert_eq!(r.spawns, vec![MonsterId::new("EYE")]);
    }

    #[test]
    fn random_pools_are_variants() {
        // Bowlbug-style: one fixed member, one random partner, never alone.
        let r = read(&[(&["ROCK", "EGG"], 50), (&["ROCK", "NECTAR"], 48), (&["ROCK", "NECTAR", "X"], 1)], false);
        assert_eq!(r.lineups.len(), 2, "rare lineups are dropped");
        assert!(r.spawns.is_empty());
        assert!((r.lineups[0].share - 50.0 / 99.0).abs() < 1e-4);
    }

    #[test]
    fn same_type_variants_without_summoner() {
        let r = read(&[(&["AXEBOT"], 78), (&["AXEBOT", "AXEBOT"], 20)], false);
        assert_eq!(r.lineups.len(), 2);
        assert_eq!(names(&r.lineups[1]), ["AXEBOT", "AXEBOT"]);
    }

    #[test]
    fn too_few_fights_is_unknown() {
        let o = obs(&[(&["A"], 5)]);
        let refs: Vec<&[String]> = o.iter().map(|v| v.as_slice()).collect();
        assert!(read_encounter(&refs, false).is_none());
    }

    #[test]
    fn act_shapes_find_fixed_floors_and_skip_double_boss_acts() {
        use NodeKind::*;
        let rules = GameRules::default();
        let mut ls = Vec::new();
        for i in 0..40 {
            let middle = if i % 2 == 0 { Monster } else { Unknown };
            ls.push(ActLayout {
                act: 0,
                ascension: 0,
                rooms: vec![Ancient, Monster, middle, Treasure, middle, Rest, Boss],
            });
        }
        // A10 act 3 has an extra boss floor; it must not skew the act length.
        for _ in 0..40 {
            ls.push(ActLayout { act: 2, ascension: 10, rooms: vec![Ancient, Monster, Rest, Boss, Boss] });
        }
        let shapes = act_shapes(&ls, &rules);
        assert_eq!(shapes.len(), 1, "double-boss acts are excluded: {shapes:?}");
        let (act, length, fixed) = &shapes[0];
        assert_eq!((*act, *length), (0, 7));
        let at = |i: i32| fixed.iter().find(|f| f.at == i).map(|f| f.room);
        assert_eq!(at(0), Some(Ancient));
        assert_eq!(at(1), Some(Monster));
        assert_eq!(at(2), None, "varies");
        assert_eq!(at(3), Some(Treasure));
        assert_eq!(at(-2), Some(Rest));
        assert_eq!(at(-1), Some(Boss));
    }

    #[test]
    fn run_indices_stay_unique_across_pages() {
        let run = r#"{"players":[{}],"modifiers":[],"map_point_history":[[{"rooms":[{"model_id":"ENCOUNTER.X","monster_ids":["MONSTER.A"]}]}]]}"#;
        let page = format!("{run}\n{run}");
        let f = fights_from_pages(&[page.clone(), page]);
        let runs: Vec<usize> = f.iter().map(|f| f.run).collect();
        assert_eq!(runs, [0, 1, 2, 3]);
    }

    #[test]
    fn parses_export_lines_and_skips_multiplayer_and_modded() {
        let solo = r#"{"players":[{}],"modifiers":[],"map_point_history":[[{"player_stats":[{"damage_taken":7}],"rooms":[{"model_id":"ENCOUNTER.X","monster_ids":["MONSTER.B","MONSTER.A"],"turns_taken":3}]}]]}"#;
        let duo = r#"{"players":[{},{}],"modifiers":[],"map_point_history":[[{"rooms":[{"model_id":"ENCOUNTER.X","monster_ids":["MONSTER.A"]}]}]]}"#;
        let modded = r#"{"players":[{}],"modifiers":["X"],"map_point_history":[[{"rooms":[{"model_id":"ENCOUNTER.X","monster_ids":["MONSTER.A"]}]}]]}"#;
        let f = fights(&[solo, duo, modded, "not json"].join("\n"));
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].encounter, "X");
        assert_eq!(f[0].monsters, ["A", "B"]);
        assert_eq!(f[0].damage_taken, Some(7));
        assert_eq!(f[0].turns, Some(3));
    }

    #[test]
    fn records_entering_hp_and_the_fight_that_ended_the_run() {
        let run = r#"{"players":[{}],"modifiers":[],"win":false,"killed_by_encounter":"ENCOUNTER.BOSS","map_point_history":[[{"player_stats":[{"current_hp":50}],"rooms":[{"model_id":"EVENT.NEOW"}]},{"player_stats":[{"damage_taken":10,"current_hp":40}],"rooms":[{"model_id":"ENCOUNTER.BOSS","monster_ids":["MONSTER.A"],"turns_taken":4}]}],[{"player_stats":[{"damage_taken":40,"current_hp":0}],"rooms":[{"model_id":"ENCOUNTER.BOSS","monster_ids":["MONSTER.A"],"turns_taken":7}]}]]}"#;
        let f = fights(run);
        assert_eq!(f.len(), 2);
        assert_eq!((f[0].hp_in, f[0].died), (Some(50), false));
        assert_eq!((f[1].hp_in, f[1].died), (Some(40), true));
        let mut stats = EncounterStats::default();
        outcomes(&mut stats, f.iter());
        assert_eq!(stats.deaths, 1);
        assert_eq!(stats.death_turns, vec![0, 0, 0, 0, 0, 0, 1]);
        assert_eq!(stats.win_damage, Some((10, 10)));
        assert_eq!(stats.hp_in, Some((50, 40)));
    }

    #[test]
    fn measures_how_encounters_repeat_within_an_act() {
        use spire_core::catalog::{Encounter, RoomType};
        let enc = |id: &str, room| Encounter {
            id: id.into(),
            name: id.into(),
            room,
            weak: false,
            act: None,
            monsters: vec![],
            tags: vec![],
            lineups: vec![],
            spawns: vec![],
            stats: None,
        };
        let pack = DataPack {
            encounters: vec![
                enc("H1", RoomType::Monster),
                enc("H2", RoomType::Monster),
                enc("E1", RoomType::Elite),
                enc("E2", RoomType::Elite),
            ],
            ..Default::default()
        };
        let fight = |run, e: &str| Fight {
            run,
            act: 0,
            encounter: e.into(),
            monsters: vec![],
            damage_taken: None,
            turns: None,
            hp_in: None,
            died: false,
        };
        // Hallway fights never repeat; elites repeat, but never back to back.
        let fights: Vec<Fight> = (0..300).flat_map(|r| ["H1", "E1", "H2", "E2", "E1"].map(|e| fight(r, e))).collect();
        assert_eq!(repeat_rules(&pack, &fights), (Some(true), Some(true)));
        let twice: Vec<Fight> = (0..300).flat_map(|r| ["H1", "H1", "E1", "E1"].map(|e| fight(r, e))).collect();
        assert_eq!(repeat_rules(&pack, &twice), (Some(false), Some(false)));
        assert_eq!(repeat_rules(&pack, &fights[..10]), (None, None));
    }
}
