//! The game's run record, described once: finished runs (`history/*.run`,
//! and the community runs Spire Codex exports) and the run in progress
//! (`current_run.save`). [`crate::game_run`] turns a player's own record
//! into our run log; [`crate::codex_runs`] reads community fights from it.
//!
//! The two files differ in places (checked against a schema 21 live save):
//! the live save writes acts as objects with their rooms, names the
//! character `character_id` and keeps the seed under `rng`; see the field
//! notes. Every list reads item by item and every id reads leniently, so a
//! game update that reshapes one field loses that field, not the record.

use serde::Deserialize;
use spire_core::catalog::NodeKind;

/// The live run save (`current_run.save`) writes some ids as objects
/// where a finished `.run` writes `"ACT.HIVE"`: the act, say, with its id
/// inside, or an id as `{category, entry}`. These read either, so the same
/// record types serve both files.
pub(crate) mod lenient {
    use serde::{Deserialize, Deserializer};
    use serde_json::Value;

    /// `CATEGORY.ENTRY`, as the game writes ids.
    fn looks_like_id(s: &str) -> bool {
        s.split_once('.').is_some_and(|(c, e)| {
            !c.is_empty() && !e.is_empty() && c.chars().all(|ch| ch.is_ascii_uppercase() || ch == '_')
        })
    }

    /// The id a value stands for: a string, a `{category, entry}` pair, an
    /// `id`-like key, or else the first `CATEGORY.ENTRY` string inside.
    pub fn id_of(v: &Value) -> Option<String> {
        match v {
            Value::String(s) => Some(s.clone()),
            Value::Number(n) => Some(n.to_string()),
            Value::Object(m) => pair(m)
                .or_else(|| ["id", "model_id", "modelid"].iter().find_map(|k| get(m, k).and_then(id_of)))
                .or_else(|| deep(v)),
            _ => None,
        }
    }

    fn get<'a>(m: &'a serde_json::Map<String, Value>, k: &str) -> Option<&'a Value> {
        m.iter().find(|(key, _)| key.eq_ignore_ascii_case(k)).map(|(_, v)| v)
    }

    /// `{"category": "ACT", "entry": "HIVE"}` is `ACT.HIVE`.
    fn pair(m: &serde_json::Map<String, Value>) -> Option<String> {
        match (get(m, "category"), get(m, "entry")) {
            (Some(Value::String(c)), Some(Value::String(e))) => Some(format!("{c}.{e}")),
            _ => None,
        }
    }

    /// The first string inside `v` that starts with `prefix` (`CHARACTER.`).
    pub fn with_prefix(v: &Value, prefix: &str) -> Option<String> {
        match v {
            Value::String(s) if s.starts_with(prefix) => Some(s.clone()),
            Value::Object(m) => match pair(m) {
                Some(id) if id.starts_with(prefix) => Some(id),
                _ => m.values().find_map(|x| with_prefix(x, prefix)),
            },
            Value::Array(a) => a.iter().find_map(|x| with_prefix(x, prefix)),
            _ => None,
        }
    }

    fn deep(v: &Value) -> Option<String> {
        match v {
            Value::String(s) if looks_like_id(s) => Some(s.clone()),
            Value::Object(m) => pair(m).or_else(|| m.values().find_map(deep)),
            Value::Array(a) => a.iter().find_map(deep),
            _ => None,
        }
    }

    pub fn id<'de, D: Deserializer<'de>>(d: D) -> Result<String, D::Error> {
        Ok(id_of(&Value::deserialize(d)?).unwrap_or_default())
    }

    pub fn opt_id<'de, D: Deserializer<'de>>(d: D) -> Result<Option<String>, D::Error> {
        Ok(id_of(&Value::deserialize(d)?))
    }

    pub fn ids<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<String>, D::Error> {
        Ok(match Value::deserialize(d)? {
            Value::Array(a) => a.iter().filter_map(id_of).collect(),
            other => id_of(&other).into_iter().collect(),
        })
    }

    pub fn acts<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<super::WAct>, D::Error> {
        let Value::Array(items) = Value::deserialize(d)? else { return Ok(Vec::new()) };
        Ok(items
            .iter()
            .filter_map(|a| {
                let rooms = &a["rooms"];
                Some(super::WAct {
                    id: id_of(a)?,
                    boss: rooms.get("boss_id").and_then(id_of),
                    second_boss: rooms.get("second_boss_id").and_then(id_of),
                })
            })
            .collect())
    }

    /// A list whose items read one by one: an item that doesn't fit its
    /// type reads as its default rather than failing the whole record.
    pub fn each<'de, D: Deserializer<'de>, T: serde::de::DeserializeOwned + Default>(d: D) -> Result<Vec<T>, D::Error> {
        Ok(match Value::deserialize(d)? {
            Value::Array(a) => a.into_iter().map(|v| T::deserialize(v).unwrap_or_default()).collect(),
            _ => Vec::new(),
        })
    }

    /// `map_point_history`: acts of floors, each floor read as in [`each`].
    pub fn history<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<Vec<super::WPoint>>, D::Error> {
        Ok(match Value::deserialize(d)? {
            Value::Array(acts) => acts
                .into_iter()
                .map(|act| match act {
                    Value::Array(points) => {
                        points.into_iter().map(|p| super::WPoint::deserialize(p).unwrap_or_default()).collect()
                    }
                    _ => Vec::new(),
                })
                .collect(),
            _ => Vec::new(),
        })
    }

    pub fn opt_i64<'de, D: Deserializer<'de>>(d: D) -> Result<Option<i64>, D::Error> {
        Ok(match Value::deserialize(d)? {
            Value::Number(n) => n.as_i64(),
            Value::String(s) => s.parse().ok(),
            _ => None,
        })
    }
}

#[derive(Deserialize, Default)]
#[serde(default)]
pub(crate) struct WRun {
    #[serde(deserialize_with = "lenient::acts")]
    pub(crate) acts: Vec<WAct>,
    /// The act in progress (live saves).
    pub(crate) current_act_index: Option<usize>,
    pub(crate) ascension: u32,
    #[serde(deserialize_with = "lenient::opt_id")]
    pub(crate) seed: Option<String>,
    #[serde(deserialize_with = "lenient::opt_i64")]
    pub(crate) start_time: Option<i64>,
    #[serde(deserialize_with = "lenient::each")]
    pub(crate) players: Vec<WPlayer>,
    /// Live saves keep the seed here rather than at the top.
    pub(crate) rng: WRng,
    #[serde(deserialize_with = "lenient::history")]
    pub(crate) map_point_history: Vec<Vec<WPoint>>,
    pub(crate) win: Option<bool>,
    pub(crate) was_abandoned: Option<bool>,
    #[serde(deserialize_with = "lenient::opt_id")]
    pub(crate) killed_by_encounter: Option<String>,
    /// Custom-run modifiers (community runs with any are left out).
    pub(crate) modifiers: Option<serde_json::Value>,
    /// The room being played (live saves): it joins `map_point_history`
    /// only once it's finished.
    pub(crate) pre_finished_room: Option<WCurrentRoom>,
}

/// The room a live save was written in.
#[derive(Deserialize, Default)]
#[serde(default)]
pub(crate) struct WCurrentRoom {
    #[serde(deserialize_with = "lenient::opt_id")]
    pub(crate) encounter_id: Option<String>,
    pub(crate) room_type: Option<String>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
pub(crate) struct WPlayer {
    #[serde(alias = "net_id")]
    pub(crate) id: Option<serde_json::Value>,
    #[serde(alias = "character_id", deserialize_with = "lenient::opt_id")]
    pub(crate) character: Option<String>,
    /// Live saves only: the player's HP, max HP and gold right now.
    pub(crate) current_hp: Option<i32>,
    pub(crate) max_hp: Option<i32>,
    pub(crate) gold: Option<i32>,
    pub(crate) deck: Vec<WCard>,
    pub(crate) relics: Vec<WId>,
    pub(crate) potions: Vec<WId>,
    pub(crate) max_potion_slot_count: Option<u32>,
}

#[derive(Deserialize, Default, Clone)]
#[serde(default)]
pub(crate) struct WCard {
    #[serde(deserialize_with = "lenient::id")]
    pub(crate) id: String,
    pub(crate) current_upgrade_level: Option<u32>,
    pub(crate) enchantment: Option<WId>,
}

#[derive(Deserialize, Default, Clone)]
#[serde(default)]
pub(crate) struct WId {
    #[serde(deserialize_with = "lenient::id")]
    pub(crate) id: String,
}

#[derive(Deserialize, Default)]
#[serde(default)]
pub(crate) struct WRng {
    #[serde(deserialize_with = "lenient::opt_id")]
    pub(crate) seed: Option<String>,
}

/// An act: an id in finished runs; in live saves, an object with its
/// rooms, which name the boss before it's fought.
#[derive(Default)]
pub(crate) struct WAct {
    pub(crate) id: String,
    pub(crate) boss: Option<String>,
    pub(crate) second_boss: Option<String>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
pub(crate) struct WPoint {
    #[serde(deserialize_with = "lenient::opt_id")]
    pub(crate) map_point_type: Option<String>,
    #[serde(deserialize_with = "lenient::each")]
    pub(crate) player_stats: Vec<WStats>,
    pub(crate) rooms: Vec<WRoom>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
pub(crate) struct WRoom {
    #[serde(deserialize_with = "lenient::id")]
    pub(crate) model_id: String,
    /// Every monster in the fight, repeats kept.
    #[serde(deserialize_with = "lenient::ids")]
    pub(crate) monster_ids: Vec<String>,
    pub(crate) turns_taken: Option<u32>,
    /// What the room turned out to be (a `?` room's event, fight, shop or
    /// treasure).
    #[serde(deserialize_with = "lenient::opt_id")]
    pub(crate) room_type: Option<String>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
pub(crate) struct WChoice {
    #[serde(deserialize_with = "lenient::id")]
    pub(crate) choice: String,
    pub(crate) was_picked: bool,
}

#[derive(Deserialize, Default)]
#[serde(default)]
pub(crate) struct WEnchanted {
    pub(crate) card: WCard,
    #[serde(deserialize_with = "lenient::id")]
    pub(crate) enchantment: String,
}

#[derive(Deserialize, Default)]
#[serde(default)]
pub(crate) struct WTransformed {
    pub(crate) original_card: WCard,
    pub(crate) final_card: WCard,
}

#[derive(Deserialize, Default)]
#[serde(default)]
pub(crate) struct WStats {
    pub(crate) player_id: Option<serde_json::Value>,
    pub(crate) current_hp: Option<i32>,
    pub(crate) max_hp: Option<i32>,
    pub(crate) current_gold: Option<i32>,
    pub(crate) damage_taken: Option<u32>,
    pub(crate) cards_gained: Vec<WCard>,
    pub(crate) cards_removed: Vec<WCard>,
    #[serde(deserialize_with = "lenient::ids")]
    pub(crate) upgraded_cards: Vec<String>,
    pub(crate) cards_enchanted: Vec<WEnchanted>,
    pub(crate) cards_transformed: Vec<WTransformed>,
    pub(crate) relic_choices: Vec<WChoice>,
    #[serde(deserialize_with = "lenient::ids")]
    pub(crate) relics_removed: Vec<String>,
    pub(crate) potion_choices: Vec<WChoice>,
    #[serde(deserialize_with = "lenient::ids")]
    pub(crate) potion_used: Vec<String>,
    #[serde(deserialize_with = "lenient::ids")]
    pub(crate) potion_discarded: Vec<String>,
}

/// `CARD.SNAP` → `SNAP`.
/// A floor's room, from its `map_point_type`.
pub(crate) fn node_kind(map_point_type: &str) -> Option<NodeKind> {
    Some(match map_point_type {
        "ancient" => NodeKind::Ancient,
        "monster" => NodeKind::Monster,
        "elite" => NodeKind::Elite,
        "unknown" => NodeKind::Unknown,
        "shop" => NodeKind::Shop,
        "rest_site" => NodeKind::Rest,
        "treasure" => NodeKind::Treasure,
        "boss" => NodeKind::Boss,
        _ => return None,
    })
}

pub(crate) fn entry(id: &str) -> &str {
    id.split_once('.').map_or(id, |(_, rest)| rest)
}
