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

use serde::Deserialize;
use spire_core::DataPack;
use spire_core::catalog::{EncounterStats, Lineup};
use spire_core::ids::MonsterId;

/// Fewer fights than this and an encounter is left unenriched.
pub const MIN_FIGHTS: usize = 20;
const ANCHOR_SHARE: f32 = 0.95;
const BASE_MIN_SHARE: f32 = 0.05;
const VARIANT_MIN_SHARE: f32 = 0.03;
/// A non-member monster seen in this many encounters is a player ally.
const ALLY_SPREAD: usize = 5;

#[derive(Deserialize)]
struct WRun {
    #[serde(default)]
    modifiers: Option<serde_json::Value>,
    #[serde(default)]
    players: Vec<serde_json::Value>,
    #[serde(default)]
    map_point_history: Vec<Vec<WPoint>>,
}

#[derive(Deserialize)]
struct WPoint {
    #[serde(default)]
    player_stats: Vec<WStats>,
    #[serde(default)]
    rooms: Vec<WRoom>,
}

#[derive(Deserialize)]
struct WStats {
    #[serde(default)]
    damage_taken: Option<u32>,
}

#[derive(Deserialize)]
struct WRoom {
    #[serde(default)]
    model_id: String,
    #[serde(default)]
    monster_ids: Vec<String>,
    #[serde(default)]
    turns_taken: Option<u32>,
}

/// One fight from one run.
#[derive(Clone, Debug, PartialEq)]
pub struct Fight {
    pub encounter: String,
    /// Sorted monster ids, prefix stripped, duplicates kept.
    pub monsters: Vec<String>,
    pub damage_taken: Option<u32>,
    pub turns: Option<u32>,
}

fn strip(id: &str) -> &str {
    id.split_once('.').map_or(id, |(_, rest)| rest)
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
    jsonl
        .lines()
        .filter_map(|line| serde_json::from_str::<WRun>(line).ok())
        .filter(|run| run.players.len() == 1 && is_empty_value(&run.modifiers))
        .flat_map(|run| run.map_point_history.into_iter().flatten())
        .flat_map(|point| {
            let damage = match point.player_stats.as_slice() {
                [only] => only.damage_taken,
                _ => None,
            };
            point.rooms.into_iter().filter(|r| !r.monster_ids.is_empty()).map(move |room| {
                let mut monsters: Vec<String> = room.monster_ids.iter().map(|m| strip(m).to_string()).collect();
                monsters.sort();
                Fight {
                    encounter: strip(&room.model_id).to_string(),
                    monsters,
                    damage_taken: damage,
                    turns: room.turns_taken,
                }
            })
        })
        .collect()
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct EnrichSummary {
    pub fights: usize,
    pub encounters: usize,
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

    Some(Reading { lineups, spawns, stats: EncounterStats { fights: n as u32, avg_damage_taken: 0.0, avg_turns: 0.0 } })
}

fn average(values: impl Iterator<Item = u32>) -> f32 {
    let (sum, n) = values.fold((0u64, 0u32), |(s, n), v| (s + u64::from(v), n + 1));
    if n == 0 { 0.0 } else { (sum as f64 / f64::from(n)) as f32 }
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
            total += list.len();
            readings.insert((*enc).to_string(), r);
        }
    }

    let mut summary = EnrichSummary { fights: total, encounters: 0 };
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
}
