//! `?` rooms from the Spire Codex run export: which event relics each event
//! gave, how often runs meet each event in each act, and how often a `?`
//! room turns into a treasure room, with the rarity of chest relics per act.
//!
//! The event endpoint names only some of the relics an event gives (Trash
//! Heap's "random forgotten Relic" names none). The runs record every relic
//! picked up on an event's floor, so the event relics seen there fill the
//! gap. Relics from the general pools ("Obtain a random Relic") are left out:
//! no one can fish an event for one of those. How often an event comes up is
//! measured per act (Overgrowth and Underdocks apart, though both are act 1).
//!
//! Pure: decompressed JSONL text in, enriched [`DataPack`] out.

use std::collections::{BTreeMap, BTreeSet};

use spire_core::DataPack;
use spire_core::catalog::{EventRelic, Rarity};
use spire_core::ids::{ActId, RelicId};

use crate::run_file::{WRun, entry};

/// An event relic seen fewer times than this from an event is left out
/// (stray or modded data).
const MIN_GIVEN: u32 = 2;
/// Fewer runs than this reaching an act and its shares stay unmeasured.
const MIN_RUNS: u32 = 100;

/// Events met in community runs.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Sightings {
    /// Runs that played each act, by act id.
    pub runs: BTreeMap<String, u32>,
    /// Runs that met each event, per act: `(event, act)`.
    pub met: BTreeMap<(String, String), u32>,
    /// Times each event gave each relic.
    pub given: BTreeMap<(String, String), u32>,
    /// Runs in which a `?` room turned into a treasure room, per act.
    pub unknown_treasure: BTreeMap<String, u32>,
    /// Relics taken from treasure rooms (any), per act: `(act, relic)`.
    pub chest: BTreeMap<(String, String), u32>,
}

/// Reads single-player, unmodified runs. Bad lines are skipped.
pub fn sightings(pages: &[String]) -> Sightings {
    let mut s = Sightings::default();
    let runs = pages
        .iter()
        .flat_map(|p| p.lines())
        .filter_map(|line| serde_json::from_str::<WRun>(line).ok())
        .filter(|run| run.players.len() == 1 && run.modifiers.as_ref().is_none_or(is_empty));
    for run in runs {
        for (i, points) in run.map_point_history.iter().enumerate() {
            let Some(act) = run.acts.get(i).map(|a| entry(&a.id).to_string()) else { continue };
            if points.is_empty() {
                continue;
            }
            *s.runs.entry(act.clone()).or_default() += 1;
            let mut met: BTreeSet<&str> = BTreeSet::new();
            let mut treasure = false;
            for p in points {
                let picked = || p.player_stats.iter().flat_map(|st| &st.relic_choices).filter(|c| c.was_picked);
                if p.rooms.iter().any(|r| r.room_type.as_deref() == Some("treasure")) {
                    treasure |= p.map_point_type.as_deref() == Some("unknown");
                    for c in picked() {
                        *s.chest.entry((act.clone(), entry(&c.choice).to_string())).or_default() += 1;
                    }
                }
            }
            if treasure {
                *s.unknown_treasure.entry(act.clone()).or_default() += 1;
            }
            for p in points.iter().filter(|p| p.map_point_type.as_deref() != Some("ancient")) {
                for room in p.rooms.iter().filter(|r| r.model_id.starts_with("EVENT.")) {
                    let event = entry(&room.model_id);
                    met.insert(event);
                    let picked = p.player_stats.iter().flat_map(|st| &st.relic_choices).filter(|c| c.was_picked);
                    for c in picked {
                        *s.given.entry((event.to_string(), entry(&c.choice).to_string())).or_default() += 1;
                    }
                }
            }
            for e in met {
                *s.met.entry((e.to_string(), act.clone())).or_default() += 1;
            }
        }
    }
    s
}

fn is_empty(v: &serde_json::Value) -> bool {
    match v {
        serde_json::Value::Null => true,
        serde_json::Value::Array(a) => a.is_empty(),
        serde_json::Value::Object(o) => o.is_empty(),
        _ => false,
    }
}

/// "Choose 1 of 3": the most an option lets you choose from.
fn choose_of(text: &str) -> Option<u32> {
    let lower = text.to_lowercase();
    lower.split("choose 1 of ").skip(1).filter_map(|rest| rest.split_whitespace().next()?.parse().ok()).max()
}

/// Adds the event relics seen given, the share of runs meeting each event
/// per act, and per act how often a `?` room is a treasure room and the
/// rarity of chest relics. Returns how many events gained a relic.
pub fn enrich(pack: &mut DataPack, s: &Sightings) -> usize {
    let rarity: BTreeMap<&str, Rarity> = pack.relics.iter().map(|r| (r.id.0.as_str(), r.rarity)).collect();
    for act in &mut pack.acts {
        let id = act.id.0.as_str();
        let Some(runs) = s.runs.get(id).filter(|n| **n >= MIN_RUNS) else { continue };
        act.unknown_treasure = Some(s.unknown_treasure.get(id).copied().unwrap_or(0) as f32 / *runs as f32);
        let mut by_rarity: BTreeMap<Rarity, u32> = BTreeMap::new();
        for ((a, relic), n) in &s.chest {
            if let Some(r) = rarity.get(relic.as_str()).filter(|_| a == id) {
                *by_rarity.entry(*r).or_default() += n;
            }
        }
        let total: u32 = by_rarity.values().sum();
        if total >= MIN_RUNS {
            act.chest_rarity = by_rarity.into_iter().map(|(r, n)| (r, n as f32 / total as f32)).collect();
        }
    }
    let event_relics: BTreeSet<&str> =
        pack.relics.iter().filter(|r| r.rarity == Rarity::Event).map(|r| r.id.0.as_str()).collect();
    let mut gained = 0;
    for event in &mut pack.events {
        let id = event.id.0.as_str();
        event.met = s
            .runs
            .iter()
            .filter(|(_, runs)| **runs >= MIN_RUNS)
            .map(|(act, runs)| {
                let met = s.met.get(&(id.to_string(), act.clone())).copied().unwrap_or(0);
                (ActId::new(act), met as f32 / *runs as f32)
            })
            .filter(|(_, share)| *share > 0.0)
            .collect();

        // Seen given, not named by any option: given at random among them,
        // unless an option lets you choose from that many.
        let seen: Vec<&str> = s
            .given
            .iter()
            .filter(|((e, r), n)| e == id && **n >= MIN_GIVEN && event_relics.contains(r.as_str()))
            .map(|((_, r), _)| r.as_str())
            .filter(|r| !event.relics.iter().any(|x| x.relic.0 == *r))
            .collect();
        if seen.is_empty() {
            continue;
        }
        gained += 1;
        let n = seen.len() as u32;
        let chooser = event.options.iter().find(|o| choose_of(&o.text).is_some_and(|k| k >= n));
        for r in seen {
            event.relics.push(EventRelic {
                relic: RelicId(r.to_string()),
                option: chooser.map(|o| o.title.clone()),
                one_of: if chooser.is_some() { 1 } else { n },
            });
        }
    }
    gained
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(act_events: &[&[(&str, &[&str])]]) -> String {
        let acts: Vec<serde_json::Value> = act_events
            .iter()
            .map(|points| {
                let points: Vec<serde_json::Value> = points
                    .iter()
                    .map(|(event, relics)| {
                        let choices: Vec<_> = relics
                            .iter()
                            .map(|r| serde_json::json!({"choice": format!("RELIC.{r}"), "was_picked": true}))
                            .collect();
                        serde_json::json!({
                            "map_point_type": "unknown",
                            "player_stats": [{"relic_choices": choices}],
                            "rooms": [{"model_id": format!("EVENT.{event}"), "room_type": "event"}]
                        })
                    })
                    .collect();
                serde_json::Value::Array(points)
            })
            .collect();
        let ids: Vec<String> =
            ["ACT.ONE", "ACT.TWO", "ACT.THREE"][..acts.len()].iter().map(|s| s.to_string()).collect();
        serde_json::json!({"players": [{}], "acts": ids, "map_point_history": acts}).to_string()
    }

    #[test]
    fn counts_runs_meeting_events_and_the_relics_they_gave() {
        let page = [
            run(&[&[("TRASH_HEAP", &["HAND_DRILL"])], &[("DOLL_ROOM", &["MR_STRUGGLES"])]]),
            run(&[&[("TRASH_HEAP", &["THE_BOOT"]), ("TRASH_HEAP", &[])]]),
            run(&[&[("OTHER", &[])]]),
        ]
        .join("\n");
        let s = sightings(&[page]);
        assert_eq!(s.runs, BTreeMap::from([("ONE".to_string(), 3), ("TWO".to_string(), 1)]));
        // Met twice in one act counts the run once.
        assert_eq!(s.met[&("TRASH_HEAP".to_string(), "ONE".to_string())], 2);
        assert_eq!(s.given[&("TRASH_HEAP".to_string(), "HAND_DRILL".to_string())], 1);
        assert_eq!(s.given[&("DOLL_ROOM".to_string(), "MR_STRUGGLES".to_string())], 1);
    }

    #[test]
    fn reads_how_many_an_option_lets_you_choose_from() {
        assert_eq!(choose_of("Lose 15 HP. Choose 1 of 3 Doll Relics."), Some(3));
        assert_eq!(choose_of("Obtain a random Doll Relic."), None);
    }
}
