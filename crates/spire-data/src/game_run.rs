//! Slay the Spire 2's own run record, as the game saves it (the run in
//! progress in `current_run.save`, finished runs in `history/*.run`), read
//! into our run log for monitored runs.
//!
//! The game records each floor in `map_point_history`: the room, the fight
//! or event, cards gained, removed, upgraded, enchanted and transformed,
//! relics and potions picked, used or lost, and HP, max HP and gold after it.
//! That maps onto our [`RunEvent`]s nearly one for one. The converter replays
//! as it goes, so changes name the exact card in the deck (an upgraded or
//! enchanted copy), and ends with one correction if our state still differs
//! from the game's (`players[].deck`, `relics`, `potions`).
//!
//! Notes on the format (checked against community `.run` exports):
//! - Purchases also appear as picked choices (`relic_choices`,
//!   `potion_choices`) and bought colourless cards in `cards_gained`, so
//!   only those lists are read.
//! - Ids are `CATEGORY.ENTRY` (`CARD.SNAP`); the entry is our id.
//! - Card offers (`card_choices`) aren't recorded (decision M5).
//!
//! Pure: JSON text in, events out. Unknown fields are ignored and missing
//! ones default, so a game update that adds fields doesn't break it.

use serde::Deserialize;
use spire_core::Catalog;
use spire_core::ids::*;
use spire_core::run::{Change, DeckCard, NodeKind, NodeReport, Outcome, RunEvent, RunState, apply};

/// The live run save (`current_run.save`) writes some ids as objects
/// where a finished `.run` writes `"ACT.HIVE"`: the act, say, with its id
/// inside, or an id as `{category, entry}`. These read either, so the same
/// record types serve both files.
mod lenient {
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
struct WRun {
    #[serde(deserialize_with = "lenient::acts")]
    acts: Vec<WAct>,
    /// The act in progress (live saves).
    current_act_index: Option<usize>,
    ascension: u32,
    #[serde(deserialize_with = "lenient::opt_id")]
    seed: Option<String>,
    #[serde(deserialize_with = "lenient::opt_i64")]
    start_time: Option<i64>,
    players: Vec<WPlayer>,
    /// Live saves keep the seed here rather than at the top.
    rng: WRng,
    map_point_history: Vec<Vec<WPoint>>,
    win: Option<bool>,
    was_abandoned: Option<bool>,
    #[serde(deserialize_with = "lenient::opt_id")]
    killed_by_encounter: Option<String>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct WPlayer {
    #[serde(alias = "net_id")]
    id: Option<serde_json::Value>,
    #[serde(alias = "character_id", deserialize_with = "lenient::opt_id")]
    character: Option<String>,
    /// Live saves only: the player's HP, max HP and gold right now.
    current_hp: Option<i32>,
    max_hp: Option<i32>,
    gold: Option<i32>,
    deck: Vec<WCard>,
    relics: Vec<WId>,
    potions: Vec<WId>,
    max_potion_slot_count: Option<u32>,
}

#[derive(Deserialize, Default, Clone)]
#[serde(default)]
struct WCard {
    #[serde(deserialize_with = "lenient::id")]
    id: String,
    current_upgrade_level: Option<u32>,
    enchantment: Option<WId>,
}

#[derive(Deserialize, Default, Clone)]
#[serde(default)]
struct WId {
    #[serde(deserialize_with = "lenient::id")]
    id: String,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct WRng {
    #[serde(deserialize_with = "lenient::opt_id")]
    seed: Option<String>,
}

/// An act: an id in finished runs; in live saves, an object with its
/// rooms, which name the boss before it's fought.
#[derive(Default)]
struct WAct {
    id: String,
    boss: Option<String>,
    second_boss: Option<String>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct WPoint {
    #[serde(deserialize_with = "lenient::opt_id")]
    map_point_type: Option<String>,
    player_stats: Vec<WStats>,
    rooms: Vec<WRoom>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct WRoom {
    #[serde(deserialize_with = "lenient::id")]
    model_id: String,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct WChoice {
    #[serde(deserialize_with = "lenient::id")]
    choice: String,
    was_picked: bool,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct WEnchanted {
    card: WCard,
    #[serde(deserialize_with = "lenient::id")]
    enchantment: String,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct WTransformed {
    original_card: WCard,
    final_card: WCard,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct WStats {
    player_id: Option<serde_json::Value>,
    current_hp: Option<i32>,
    max_hp: Option<i32>,
    current_gold: Option<i32>,
    cards_gained: Vec<WCard>,
    cards_removed: Vec<WCard>,
    #[serde(deserialize_with = "lenient::ids")]
    upgraded_cards: Vec<String>,
    cards_enchanted: Vec<WEnchanted>,
    cards_transformed: Vec<WTransformed>,
    relic_choices: Vec<WChoice>,
    #[serde(deserialize_with = "lenient::ids")]
    relics_removed: Vec<String>,
    potion_choices: Vec<WChoice>,
    #[serde(deserialize_with = "lenient::ids")]
    potion_used: Vec<String>,
    #[serde(deserialize_with = "lenient::ids")]
    potion_discarded: Vec<String>,
}

/// `CARD.SNAP` → `SNAP`.
fn entry(id: &str) -> &str {
    id.split_once('.').map_or(id, |(_, rest)| rest)
}

fn none_id(id: &Option<String>) -> Option<&str> {
    id.as_deref().map(entry).filter(|e| !e.is_empty() && *e != "NONE")
}

fn deck_card(c: &WCard) -> DeckCard {
    DeckCard {
        card: CardId::new(entry(&c.id)),
        upgraded: c.current_upgrade_level.unwrap_or(0) > 0,
        enchantment: c.enchantment.as_ref().map(|e| EnchantmentId::new(entry(&e.id))),
    }
}

fn kind(t: Option<&str>) -> NodeKind {
    match t {
        Some("monster") => NodeKind::Monster,
        Some("elite") => NodeKind::Elite,
        Some("boss") => NodeKind::Boss,
        Some("rest_site") => NodeKind::Rest,
        Some("shop") => NodeKind::Shop,
        Some("treasure") => NodeKind::Treasure,
        Some("ancient") => NodeKind::Ancient,
        _ => NodeKind::Unknown,
    }
}

/// The card in `deck` that a game card refers to: the exact one if present,
/// else the same card with any upgrade or enchantment, preferring the
/// least-changed copy.
fn find(deck: &[DeckCard], want: &DeckCard) -> DeckCard {
    if deck.contains(want) {
        return want.clone();
    }
    let mut same: Vec<&DeckCard> = deck.iter().filter(|c| c.card == want.card).collect();
    same.sort_by_key(|c| (c.upgraded != want.upgraded, c.enchantment != want.enchantment));
    same.first().map_or_else(|| want.clone(), |c| (*c).clone())
}

/// What the dashboard knows about the act in progress that the save
/// doesn't say (from the companion mod).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ActHint {
    /// 0-based act index.
    pub index: Option<u8>,
    pub act: Option<String>,
    pub boss: Option<String>,
    pub second_boss: Option<String>,
}

/// A game run read into our log.
#[derive(Clone, Debug, PartialEq)]
pub struct GameRun {
    /// Identifies the run across saves: start time and seed.
    pub key: String,
    /// When the run started (Unix seconds), if the game says.
    pub start_time: Option<i64>,
    pub events: Vec<RunEvent>,
    /// Floors recorded.
    pub floors: usize,
    /// The run is over (a finished `.run`).
    pub ended: bool,
    /// Ids the catalog doesn't know (kept in the log as they are).
    pub unknown: Vec<String>,
}

/// Why a run save couldn't be read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Unreadable {
    /// Not JSON at all (the game may compress or encode it).
    NotJson,
    /// JSON, but not shaped like a run (the parser's message).
    Shape(String),
    NoPlayer,
    /// No character id anywhere in the player; its fields, to see where it went.
    NoCharacter(Vec<String>),
    /// A character the catalog doesn't know.
    UnknownCharacter(String),
}

impl std::fmt::Display for Unreadable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Unreadable::NotJson => write!(f, "it isn't plain JSON (the game may compress or encode it)"),
            Unreadable::Shape(e) => write!(f, "it isn't shaped like a run record ({e})"),
            Unreadable::NoPlayer => write!(f, "it has no players"),
            Unreadable::NoCharacter(keys) if keys.is_empty() => write!(f, "its player has no character"),
            Unreadable::NoCharacter(keys) => {
                write!(f, "its player has no character (the player's fields: {})", keys.join(", "))
            }
            Unreadable::UnknownCharacter(c) => write!(f, "the game data doesn't know its character, {c}"),
        }
    }
}

/// Which top-level field didn't read, with the parser's message.
fn shape_error(value: &serde_json::Value, e: serde_json::Error) -> String {
    let field = value.as_object().and_then(|m| {
        m.iter().find(|(k, v)| {
            let one = serde_json::Value::Object([((*k).clone(), (*v).clone())].into_iter().collect());
            WRun::deserialize(&one).is_err()
        })
    });
    match field {
        Some((k, _)) => format!("field `{k}`: {e}"),
        None => e.to_string(),
    }
}

/// Reads a run save or `.run` file, or says why it can't.
pub fn read(json: &str, catalog: &Catalog, hint: &ActHint) -> Result<GameRun, Unreadable> {
    let value: serde_json::Value = serde_json::from_str(json).map_err(|_| Unreadable::NotJson)?;
    let run = WRun::deserialize(&value).map_err(|e| Unreadable::Shape(shape_error(&value, e)))?;
    let player = run.players.first().ok_or(Unreadable::NoPlayer)?;
    // Finished runs keep it in `character`; otherwise any character id in
    // the player (the live save may name the field differently).
    let first = &value["players"][0];
    let character =
        player.character.clone().or_else(|| lenient::with_prefix(first, "CHARACTER.")).ok_or_else(|| {
            Unreadable::NoCharacter(first.as_object().map(|m| m.keys().cloned().collect()).unwrap_or_default())
        })?;
    let character = CharacterId::new(entry(&character));
    let start = RunEvent::start(catalog, &character, run.ascension)
        .ok_or_else(|| Unreadable::UnknownCharacter(character.to_string()))?;
    let seed = run.seed.clone().or_else(|| run.rng.seed.clone()).unwrap_or_default();
    let key = format!("{}:{seed}", run.start_time.unwrap_or(0));
    let mut events = vec![start];
    let mut state = apply(RunState::default(), &events[0]);
    let mut unknown: Vec<String> = Vec::new();
    let mut note = |id: String, known: bool| {
        if !known && !unknown.contains(&id) {
            unknown.push(id);
        }
    };
    let push = |events: &mut Vec<RunEvent>, state: &mut RunState, e: RunEvent| {
        *state = apply(std::mem::take(state), &e);
        events.push(e);
    };

    let mut floors = 0;
    // The act in progress may have no floors yet: the hint says it began.
    let current_act = hint.index.map(usize::from).or(run.current_act_index);
    let acts = run.map_point_history.len().max(current_act.map_or(0, |i| i + 1));
    for i in 0..acts {
        let points: &[WPoint] = run.map_point_history.get(i).map_or(&[], Vec::as_slice);
        let current = current_act == Some(i);
        let saved = run.acts.get(i);
        let act = saved
            .map(|a| entry(&a.id).to_string())
            .or_else(|| hint.act.as_deref().filter(|_| current).map(|a| entry(a).to_string()));
        let Some(act) = act else { continue };
        let fought: Vec<EncounterId> = points
            .iter()
            .filter(|p| p.map_point_type.as_deref() == Some("boss"))
            .filter_map(|p| p.rooms.first())
            .map(|r| EncounterId::new(entry(&r.model_id)))
            .collect();
        let hinted = |b: &Option<String>| b.as_deref().filter(|_| current).map(|b| EncounterId::new(entry(b)));
        // Fought, else as the live save's act names it, else as the mod saw it.
        let named = |b: Option<&String>| b.map(|b| EncounterId::new(entry(b)));
        let boss = fought
            .first()
            .cloned()
            .or_else(|| named(saved.and_then(|a| a.boss.as_ref())))
            .or_else(|| hinted(&hint.boss));
        let second = fought
            .get(1)
            .cloned()
            .or_else(|| named(saved.and_then(|a| a.second_boss.as_ref())))
            .or_else(|| hinted(&hint.second_boss));
        note(act.clone(), catalog.acts.contains_key(&ActId::new(&act)));
        push(&mut events, &mut state, RunEvent::EnterAct { act: ActId::new(&act), index: i as u8, boss });
        if let Some(s) = second {
            push(&mut events, &mut state, RunEvent::SetSecondBoss(s));
        }

        for p in points {
            let stats = p
                .player_stats
                .iter()
                .find(|s| s.player_id.is_some() && s.player_id == player.id)
                .or(p.player_stats.first());
            let room = p.rooms.first().map(|r| r.model_id.as_str()).unwrap_or("");
            let (encounter, event) = match room.split_once('.') {
                Some(("ENCOUNTER", e)) => (Some(EncounterId::new(e)), None),
                Some(("EVENT", e)) => (None, Some(EventId::new(e))),
                _ => (None, None),
            };
            let changes = stats.map(|s| changes(s, &state, catalog, &mut note)).unwrap_or_default();
            let report = NodeReport { kind: kind(p.map_point_type.as_deref()), encounter, event, changes };
            push(&mut events, &mut state, RunEvent::Node(report));
            floors += 1;
        }
    }

    // Whatever the per-floor record doesn't itemise, set straight from the
    // game's own totals.
    let fix = reconcile(&state, player);
    if !fix.is_empty() {
        push(&mut events, &mut state, RunEvent::Adjust(fix));
    }

    let outcome = match (run.win, run.was_abandoned) {
        (Some(true), _) => Some(Outcome::Victory),
        (_, Some(true)) => Some(Outcome::Abandoned),
        (Some(false), _) => Some(Outcome::Died { encounter: none_id(&run.killed_by_encounter).map(EncounterId::new) }),
        _ => None,
    };
    let ended = outcome.is_some();
    if let Some(outcome) = outcome {
        push(&mut events, &mut state, RunEvent::End { outcome, at: None });
    }
    Ok(GameRun { key, start_time: run.start_time, events, floors, ended, unknown })
}

/// One floor's changes, naming cards as they sit in the deck now.
fn changes(s: &WStats, state: &RunState, catalog: &Catalog, note: &mut impl FnMut(String, bool)) -> Vec<Change> {
    let mut deck = state.deck.clone();
    let mut out = Vec::new();
    let take = |deck: &mut Vec<DeckCard>, c: &DeckCard| -> DeckCard {
        let found = find(deck, c);
        if let Some(i) = deck.iter().position(|d| *d == found) {
            deck.remove(i);
        }
        found
    };
    for c in &s.cards_removed {
        let card = take(&mut deck, &deck_card(c));
        out.push(Change::RemoveCard(card));
    }
    for t in &s.cards_transformed {
        let from = take(&mut deck, &deck_card(&t.original_card));
        let to = deck_card(&t.final_card);
        deck.push(to.clone());
        out.push(Change::Transform { from, to });
    }
    for id in &s.upgraded_cards {
        let want = DeckCard::new(CardId::new(entry(id)));
        let card = take(&mut deck, &want);
        deck.push(DeckCard { upgraded: true, ..card.clone() });
        out.push(Change::UpgradeCard(card));
    }
    for e in &s.cards_enchanted {
        let enchantment = EnchantmentId::new(entry(&e.enchantment));
        let want = DeckCard { enchantment: None, ..deck_card(&e.card) };
        let card = take(&mut deck, &want);
        deck.push(DeckCard { enchantment: Some(enchantment.clone()), ..card.clone() });
        out.push(Change::Enchant { card, enchantment });
    }
    for c in &s.cards_gained {
        let card = deck_card(c);
        note(c.id.clone(), catalog.cards.contains_key(&card.card));
        deck.push(card.clone());
        out.push(Change::AddCard(card));
    }
    for r in &s.relics_removed {
        out.push(Change::RemoveRelic(RelicId::new(entry(r))));
    }
    for r in s.relic_choices.iter().filter(|c| c.was_picked) {
        let id = RelicId::new(entry(&r.choice));
        note(r.choice.clone(), catalog.relics.contains_key(&id));
        out.push(Change::AddRelic(id));
    }
    for p in s.potion_used.iter().chain(&s.potion_discarded) {
        out.push(Change::RemovePotion(PotionId::new(entry(p))));
    }
    for p in s.potion_choices.iter().filter(|c| c.was_picked) {
        out.push(Change::AddPotion(PotionId::new(entry(&p.choice))));
    }
    // A live save's floor in progress has zeros for HP and gold until it's
    // done: no max HP means not recorded yet.
    if s.max_hp == Some(0) {
        return out;
    }
    out.extend(totals(state, s.max_hp, s.current_hp, s.current_gold));
    out
}

/// Max HP, HP and gold where they differ from ours.
fn totals(state: &RunState, max_hp: Option<i32>, hp: Option<i32>, gold: Option<i32>) -> Vec<Change> {
    let mut out = Vec::new();
    if let Some(m) = max_hp.filter(|m| *m != state.max_hp) {
        out.push(Change::SetMaxHp(m));
    }
    if let Some(hp) = hp.filter(|hp| *hp != state.hp) {
        out.push(Change::SetHp(hp));
    }
    if let Some(g) = gold.filter(|g| *g != state.gold) {
        out.push(Change::SetGold(g));
    }
    out
}

/// Changes that bring our deck, relics, potions and potion slots to the
/// game's own totals. Empty when they already agree.
fn reconcile(state: &RunState, player: &WPlayer) -> Vec<Change> {
    let mut out = Vec::new();
    // Multiset differences, by value.
    fn diff<T: Clone + PartialEq>(ours: &[T], theirs: &[T]) -> (Vec<T>, Vec<T>) {
        let mut extra: Vec<T> = ours.to_vec();
        let mut missing = Vec::new();
        for t in theirs {
            match extra.iter().position(|x| x == t) {
                Some(i) => {
                    extra.remove(i);
                }
                None => missing.push(t.clone()),
            }
        }
        (extra, missing)
    }
    // An empty list in the save means "not recorded", not "none".
    if !player.deck.is_empty() {
        let theirs: Vec<DeckCard> = player.deck.iter().map(deck_card).collect();
        let (extra, missing) = diff(&state.deck, &theirs);
        out.extend(extra.into_iter().map(Change::RemoveCard));
        out.extend(missing.into_iter().map(Change::AddCard));
    }
    if !player.relics.is_empty() {
        let theirs: Vec<RelicId> = player.relics.iter().map(|r| RelicId::new(entry(&r.id))).collect();
        let (extra, missing) = diff(&state.relics, &theirs);
        out.extend(extra.into_iter().map(Change::RemoveRelic));
        out.extend(missing.into_iter().map(Change::AddRelic));
    }
    let theirs: Vec<PotionId> = player.potions.iter().map(|p| PotionId::new(entry(&p.id))).collect();
    let (extra, missing) = diff(&state.potions, &theirs);
    out.extend(extra.into_iter().map(Change::RemovePotion));
    out.extend(missing.into_iter().map(Change::AddPotion));
    if let Some(n) = player.max_potion_slot_count.filter(|n| *n != state.potion_slots) {
        out.push(Change::SetPotionSlots(n));
    }
    // Live saves give the player's totals now; finished runs don't.
    out.extend(totals(state, player.max_hp.filter(|m| *m > 0), player.current_hp, player.gold));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn catalog() -> Catalog {
        Catalog::from_layers([crate::bundled_pack()])
    }

    #[test]
    fn ids_and_rooms_read_as_ours() {
        assert_eq!(entry("CARD.SNAP"), "SNAP");
        assert_eq!(none_id(&Some("NONE.NONE".into())), None);
        assert_eq!(kind(Some("rest_site")), NodeKind::Rest);
        let c = WCard {
            id: "CARD.BASH".into(),
            current_upgrade_level: Some(1),
            enchantment: Some(WId { id: "ENCHANTMENT.SHARP".into() }),
        };
        let d = deck_card(&c);
        assert_eq!((d.card.as_str(), d.upgraded), ("BASH", true));
        assert_eq!(d.enchantment.as_ref().map(|e| e.as_str()), Some("SHARP"));
    }

    #[test]
    fn reads_ids_written_as_objects() {
        // The live save writes acts (and maybe other ids) as objects.
        let cat = catalog();
        let json = include_str!("../tests/fixtures/finished.run");
        let plain = read(json, &cat, &ActHint::default()).expect("reads");
        let mut v: serde_json::Value = serde_json::from_str(json).unwrap();
        v["acts"] = serde_json::json!([
            {"id": "ACT.UNDERDOCKS", "rooms": {"boss": "ENCOUNTER.X"}},
            {"id": {"category": "ACT", "entry": "HIVE"}},
            {"model": {"Category": "ACT", "Entry": "GLORY"}},
        ]);
        v["players"][0]["character"] = serde_json::json!({"id": v["players"][0]["character"].clone()});
        v["seed"] = serde_json::json!(12345);
        let objects = read(&v.to_string(), &cat, &ActHint::default()).expect("reads");
        assert_eq!(objects.events[1..], plain.events[1..]);
        assert_eq!(objects.floors, plain.floors);
    }

    #[test]
    fn reads_a_live_run_save() {
        // A real current_run.save on the first floor, trimmed of RNG state,
        // relic pools and unlocks. The floor in progress reports zero HP and
        // gold; the player's own totals are what count.
        let cat = catalog();
        let json = include_str!("../tests/fixtures/in_progress.save");
        let run = read(json, &cat, &ActHint::default()).expect("reads");
        assert!(!run.ended);
        assert_eq!(run.floors, 1);
        assert!(run.key.ends_with(":QLP5BYSX5LF5"), "{}", run.key);
        let state = run.events.iter().fold(RunState::default(), apply);
        assert_eq!(state.character.as_ref().map(|c| c.as_str()), Some("FOOL"));
        assert_eq!(state.ascension, 9);
        assert_eq!((state.hp, state.max_hp, state.gold), (61, 77, 99));
        assert_eq!(state.act.as_ref().map(|a| a.as_str()), Some("UNDERDOCKS"));
        assert_eq!(state.boss.as_ref().map(|b| b.as_str()), Some("WATERFALL_GIANT_BOSS"));
        assert_eq!(state.deck.len(), 12);
        assert!(state.deck.iter().any(|c| c.card.as_str() == "YAH"));
        assert!(state.relics.iter().any(|r| r.as_str() == "ARCANE_SCROLL"));
        assert!(run.events.iter().all(|e| !matches!(e, RunEvent::Node(n) if n.changes.contains(&Change::SetHp(0)))));
    }

    #[test]
    fn finds_the_character_under_another_name() {
        let cat = catalog();
        let json = include_str!("../tests/fixtures/finished.run");
        let plain = read(json, &cat, &ActHint::default()).expect("reads");
        let mut v: serde_json::Value = serde_json::from_str(json).unwrap();
        let character = v["players"][0].as_object_mut().unwrap().remove("character").unwrap();
        v["players"][0]["character_id"] = serde_json::json!({"model": character});
        let renamed = read(&v.to_string(), &cat, &ActHint::default()).expect("reads");
        assert_eq!(renamed.events, plain.events);
        // None at all: the player's fields say where to look.
        v["players"][0].as_object_mut().unwrap().remove("character_id");
        let Err(why) = read(&v.to_string(), &cat, &ActHint::default()) else { panic!("should not read") };
        assert!(why.to_string().contains("deck"), "{why}");
    }

    #[test]
    fn names_the_field_that_did_not_read() {
        let v = serde_json::json!({"acts": [], "players": "nobody", "ascension": 1});
        let Err(Unreadable::Shape(why)) = read(&v.to_string(), &catalog(), &ActHint::default()) else {
            panic!("should not read")
        };
        assert!(why.starts_with("field `players`"), "{why}");
    }

    #[test]
    fn a_finished_run_replays_to_the_games_own_totals() {
        let cat = catalog();
        let json = include_str!("../tests/fixtures/finished.run");
        let run = read(json, &cat, &ActHint::default()).expect("reads");
        assert!(run.ended);
        assert!(run.floors > 40, "every floor recorded: {}", run.floors);
        let state = spire_core::run::replay(&run.events);
        let wire: WRun = serde_json::from_str(json).unwrap();
        let p = &wire.players[0];
        let mut ours: Vec<String> =
            state.deck.iter().map(|c| format!("{}{}", c.card.as_str(), if c.upgraded { "+" } else { "" })).collect();
        let mut theirs: Vec<String> = p
            .deck
            .iter()
            .map(|c| format!("{}{}", entry(&c.id), if c.current_upgrade_level.unwrap_or(0) > 0 { "+" } else { "" }))
            .collect();
        ours.sort();
        theirs.sort();
        assert_eq!(ours, theirs);
        assert_eq!(state.relics.len(), p.relics.len());
        assert_eq!(state.outcome, Some(Outcome::Victory));
        assert_eq!(state.act_index, 2);
        // The per-floor record itself accounts for nearly everything: the
        // final correction is small.
        let correction = run.events.iter().find_map(|e| match e {
            RunEvent::Adjust(c) => Some(c.len()),
            _ => None,
        });
        assert!(correction.unwrap_or(0) <= 4, "correction: {correction:?}");
    }

    #[test]
    fn the_act_in_progress_takes_its_boss_from_the_hint() {
        let cat = catalog();
        let act = cat.acts.values().min_by_key(|a| a.index).unwrap();
        let character = cat.characters.keys().next().unwrap();
        let json = format!(
            r#"{{"acts":[],"ascension":2,"start_time":5,"seed":"S","players":[{{"id":1,"character":"CHARACTER.{}"}}],"map_point_history":[]}}"#,
            character.as_str()
        );
        let hint = ActHint {
            index: Some(0),
            act: Some(format!("ACT.{}", act.id.as_str())),
            boss: act.bosses.first().map(|b| format!("ENCOUNTER.{}", b.as_str())),
            second_boss: None,
        };
        let run = read(&json, &cat, &hint).expect("reads");
        assert_eq!(run.key, "5:S");
        assert!(!run.ended);
        let state = spire_core::run::replay(&run.events);
        assert_eq!(state.act.as_ref(), Some(&act.id));
        assert_eq!(state.boss, act.bosses.first().cloned());
        assert_eq!(state.ascension, 2);
    }
}
