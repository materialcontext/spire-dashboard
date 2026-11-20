//! Spire Codex adapter (<https://spire-codex.com/developers>).
//!
//! Pure mapping from Codex API JSON to a [`DataPack`]. The wire structs below
//! name only the fields we use; everything is optional so new or missing
//! fields on the Codex side never break an update. HTTP lives in `fetch`.

use std::collections::{BTreeMap, BTreeSet};

use serde::Deserialize;
use spire_core::catalog::*;
use spire_core::ids::*;

use crate::DataError;

pub const DEFAULT_BASE_URL: &str = "https://spire-codex.com";

/// The endpoints one update reads, in order. Keyless use allows 15 calls per
/// minute *per endpoint*, so one call to each is well within limits.
pub const ENDPOINTS: [&str; 12] = [
    "acts",
    "characters",
    "cards",
    "relics",
    "potions",
    "enchantments",
    "monsters",
    "encounters",
    "events",
    "ascensions",
    "orbs",
    "powers",
];

/// Which game branch to read. The dashboard follows the beta branch by
/// default; Codex serves stable unless asked (`?channel=beta`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Channel {
    #[default]
    Beta,
    Stable,
}

impl Channel {
    /// Query string for entity endpoints.
    pub fn query(self) -> &'static str {
        match self {
            Channel::Beta => "?channel=beta",
            Channel::Stable => "",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Channel::Beta => "beta",
            Channel::Stable => "stable",
        }
    }
}

/// Raw response bodies keyed by endpoint name.
pub type Responses = BTreeMap<&'static str, String>;

// ---------------------------------------------------------------- wire types

/// Card metrics among high-skill players (A10 runs by players above a 50%
/// win rate): the `responses` key, and the path it's fetched from.
pub const CARD_METRICS: &str = "card_metrics";
pub const CARD_METRICS_PATH: &str = "runs/metrics/cards?bracket=wr50";
/// A card's lift is kept from this many seats.
const MIN_LIFT_SEATS: u32 = 20;

#[derive(Deserialize)]
struct WMetrics {
    #[serde(default)]
    rows: Vec<WMetricRow>,
}

#[derive(Deserialize)]
struct WMetricRow {
    id: String,
    #[serde(default)]
    upgraded: bool,
    #[serde(default)]
    lift: Option<f32>,
    #[serde(default)]
    lift_n: Option<u32>,
}

/// Each card's win-rate lift, from the card metrics (base cards only, and
/// only where enough seats measure it). Unreadable metrics read as none:
/// they only rank suggestions.
pub fn card_lift(body: &str) -> BTreeMap<CardId, CardLift> {
    let Ok(m) = serde_json::from_str::<WMetrics>(body) else { return BTreeMap::new() };
    m.rows
        .into_iter()
        .filter(|r| !r.upgraded)
        .filter_map(|r| {
            let n = r.lift_n.filter(|n| *n >= MIN_LIFT_SEATS)?;
            Some((CardId(r.id), CardLift { lift: r.lift?, n }))
        })
        .collect()
}

/// Treats JSON `null` like a missing field (Codex sends both).
fn null_as_default<'de, D, T>(d: D) -> Result<T, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Default + Deserialize<'de>,
{
    Ok(Option::<T>::deserialize(d)?.unwrap_or_default())
}

#[derive(Deserialize)]
struct WPower {
    #[serde(default)]
    power_key: Option<String>,
    #[serde(default)]
    power: Option<String>,
    #[serde(default)]
    amount: Option<i32>,
}

#[derive(Deserialize)]
struct WCard {
    id: String,
    name: String,
    #[serde(default, deserialize_with = "null_as_default")]
    description: String,
    #[serde(default)]
    cost: Option<i32>,
    #[serde(default)]
    is_x_cost: Option<bool>,
    #[serde(default, rename = "type_key")]
    card_type: Option<String>,
    #[serde(default)]
    rarity_key: Option<String>,
    #[serde(default)]
    target: Option<String>,
    #[serde(default)]
    color: Option<String>,
    #[serde(default)]
    damage: Option<u32>,
    #[serde(default)]
    block: Option<u32>,
    #[serde(default)]
    hit_count: Option<u32>,
    #[serde(default)]
    powers_applied: Option<Vec<WPower>>,
    #[serde(default)]
    cards_draw: Option<u32>,
    #[serde(default)]
    energy_gain: Option<u32>,
    #[serde(default)]
    keywords_key: Option<Vec<String>>,
    #[serde(default)]
    tags: Option<Vec<String>>,
    #[serde(default)]
    upgrade_description: Option<String>,
    #[serde(default, deserialize_with = "null_as_default")]
    vars: BTreeMap<String, serde_json::Value>,
    #[serde(default, deserialize_with = "null_as_default")]
    upgrade: BTreeMap<String, serde_json::Value>,
    #[serde(default)]
    star_cost: Option<i32>,
}

#[derive(Deserialize)]
struct WPowerDef {
    id: String,
    #[serde(default, deserialize_with = "null_as_default")]
    description: String,
}

/// "Receive [blue]50%[/blue] more damage from Attacks…" → 50.
fn vulnerable_pct(powers: &[WPowerDef]) -> Option<u32> {
    let v = powers.iter().find(|p| p.id == "VULNERABLE" || p.id == "VULNERABLE_POWER")?;
    let after = v.description.split("[blue]").nth(1)?;
    after.split('%').next()?.trim().parse().ok()
}

#[derive(Deserialize)]
struct WOrb {
    id: String,
    name: String,
    #[serde(default, deserialize_with = "null_as_default")]
    description: String,
}

#[derive(Deserialize)]
struct WCharacter {
    id: String,
    name: String,
    #[serde(default, deserialize_with = "null_as_default")]
    description: String,
    #[serde(default)]
    starting_hp: Option<u32>,
    #[serde(default)]
    starting_gold: Option<u32>,
    #[serde(default)]
    max_energy: Option<u32>,
    #[serde(default)]
    orb_slots: Option<u32>,
    #[serde(default)]
    arrow_slots: Option<u32>,
    #[serde(default, deserialize_with = "null_as_default")]
    starting_deck: Vec<String>,
    #[serde(default, deserialize_with = "null_as_default")]
    starting_relics: Vec<String>,
}

#[derive(Deserialize)]
struct WItem {
    id: String,
    name: String,
    #[serde(default, deserialize_with = "null_as_default")]
    description: String,
    #[serde(default)]
    rarity_key: Option<String>,
    #[serde(default)]
    pool: Option<String>,
}

#[derive(Deserialize)]
struct WEnchantment {
    id: String,
    name: String,
    #[serde(default, deserialize_with = "null_as_default")]
    description: String,
    #[serde(default)]
    card_type: Option<String>,
    #[serde(default)]
    applicable_to: Option<String>,
    #[serde(default)]
    is_stackable: Option<bool>,
}

#[derive(Deserialize)]
struct WDamage {
    #[serde(default)]
    normal: Option<u32>,
    #[serde(default)]
    ascension: Option<u32>,
    #[serde(default)]
    hit_count: Option<u32>,
}

#[derive(Deserialize)]
struct WMove {
    #[serde(default, deserialize_with = "null_as_default")]
    id: String,
    #[serde(default, deserialize_with = "null_as_default")]
    name: String,
    #[serde(default, deserialize_with = "null_as_default")]
    intent: String,
    #[serde(default)]
    damage: Option<WDamage>,
    #[serde(default)]
    block: Option<u32>,
    #[serde(default)]
    powers: Option<Vec<WMovePower>>,
}

#[derive(Deserialize)]
struct WMovePower {
    power_id: String,
    #[serde(default)]
    target: Option<String>,
    #[serde(default)]
    amount: Option<i32>,
}

/// `attack_pattern`: the monster's move state machine.
#[derive(Deserialize)]
struct WPattern {
    #[serde(default)]
    initial_move: Option<String>,
    #[serde(default)]
    states: Option<Vec<WState>>,
    #[serde(default)]
    description: Option<String>,
}

#[derive(Deserialize)]
struct WState {
    id: String,
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    move_id: Option<String>,
    #[serde(default)]
    next: Option<String>,
    #[serde(default)]
    branches: Option<Vec<WBranch>>,
}

#[derive(Deserialize)]
struct WBranch {
    move_id: String,
    #[serde(default)]
    weight: Option<f32>,
    #[serde(default)]
    repeat: Option<String>,
    #[serde(default)]
    max_times: Option<u32>,
    #[serde(default)]
    condition: Option<String>,
}

#[derive(Deserialize)]
struct WInnate {
    power_id: String,
    #[serde(default)]
    amount: Option<i32>,
}

#[derive(Deserialize)]
struct WMonster {
    id: String,
    name: String,
    #[serde(default)]
    min_hp: Option<u32>,
    #[serde(default)]
    max_hp: Option<u32>,
    #[serde(default)]
    min_hp_ascension: Option<u32>,
    #[serde(default)]
    max_hp_ascension: Option<u32>,
    #[serde(default)]
    moves: Option<Vec<WMove>>,
    #[serde(default)]
    innate_powers: Option<Vec<WInnate>>,
    #[serde(default)]
    attack_pattern: Option<WPattern>,
}

#[derive(Deserialize)]
struct WRef {
    id: String,
}

#[derive(Deserialize)]
struct WEncounter {
    id: String,
    name: String,
    #[serde(default, deserialize_with = "null_as_default")]
    room_type: String,
    #[serde(default)]
    is_weak: Option<bool>,
    #[serde(default)]
    tags: Option<Vec<String>>,
    #[serde(default, deserialize_with = "null_as_default")]
    monsters: Vec<WRef>,
}

#[derive(Deserialize)]
struct WOption {
    #[serde(default, deserialize_with = "null_as_default")]
    id: String,
    #[serde(default, deserialize_with = "null_as_default")]
    title: String,
    #[serde(default, deserialize_with = "null_as_default")]
    description: String,
}

#[derive(Deserialize)]
struct WEvent {
    id: String,
    name: String,
    #[serde(default, rename = "type")]
    kind: String,
    #[serde(default)]
    options: Option<Vec<WOption>>,
    /// Later screens, whose options can name relics the first screen
    /// doesn't ("Reach Deeper" → "Pollinous Core").
    #[serde(default)]
    pages: Option<Vec<WPage>>,
    #[serde(default)]
    preconditions: Option<Vec<String>>,
}

#[derive(Deserialize)]
struct WPage {
    #[serde(default)]
    options: Option<Vec<WOption>>,
}

#[derive(Deserialize)]
struct WAct {
    id: String,
    name: String,
    #[serde(default)]
    index: u8,
    #[serde(default)]
    num_rooms: Option<u32>,
    #[serde(default, deserialize_with = "null_as_default")]
    bosses: Vec<String>,
    #[serde(default, deserialize_with = "null_as_default")]
    ancients: Vec<String>,
    #[serde(default, deserialize_with = "null_as_default")]
    events: Vec<String>,
    #[serde(default, deserialize_with = "null_as_default")]
    encounters: Vec<String>,
}

// ---------------------------------------------------------------- helpers

/// Strips Codex/Godot rich-text markup: `[gold]Block[/gold]` → `Block`,
/// `[energy:2]` → `2 Energy`, `[star:1]` → `1 Star`.
pub fn strip_markup(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find('[') {
        out.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        match after.find(']') {
            Some(end) => {
                let tag = &after[..end];
                if let Some((name, n)) = tag.split_once(':')
                    && let Ok(n) = n.trim().parse::<u32>()
                {
                    let word = match name {
                        "energy" => "Energy",
                        "star" => "Star",
                        other => other,
                    };
                    out.push_str(&format!("{n} {word}"));
                }
                rest = &after[end + 1..];
            }
            None => {
                out.push_str(&rest[start..]);
                rest = "";
            }
        }
    }
    out.push_str(rest);
    out
}

/// `RingOfTheSnake` → `RING_OF_THE_SNAKE`. Codex uses class names in some
/// references (starting decks, relics) and SCREAMING_SNAKE ids elsewhere.
pub fn camel_to_id(name: &str) -> String {
    let mut out = String::with_capacity(name.len() + 8);
    for (i, ch) in name.chars().enumerate() {
        if ch.is_uppercase() && i > 0 {
            out.push('_');
        }
        out.extend(ch.to_uppercase());
    }
    out
}

fn snake(s: &str) -> String {
    camel_to_id(s).to_lowercase()
}

fn parse_enum<T: serde::de::DeserializeOwned>(value: &str) -> Option<T> {
    serde_json::from_value(serde_json::Value::String(snake(value))).ok()
}

fn card_type(s: Option<&str>) -> CardType {
    s.and_then(parse_enum).unwrap_or(CardType::Other)
}

fn rarity(s: Option<&str>) -> Rarity {
    s.and_then(parse_enum).unwrap_or(Rarity::Other)
}

fn target(s: Option<&str>) -> Target {
    match s {
        Some("Self") => Target::Own,
        Some("AnyEnemy") => Target::SingleEnemy,
        Some("AllEnemies") => Target::AllEnemies,
        Some("RandomEnemy") => Target::RandomEnemy,
        Some("AnyAlly") => Target::Ally,
        Some("AllAllies") => Target::AllAllies,
        _ => Target::None,
    }
}

fn room(s: &str) -> Option<RoomType> {
    match s {
        "Monster" => Some(RoomType::Monster),
        "Elite" => Some(RoomType::Elite),
        "Boss" => Some(RoomType::Boss),
        _ => None,
    }
}

/// Resolves a reference that may be a class name or an id against known ids.
fn resolve<'a>(name: &str, known: &'a BTreeSet<String>) -> Option<&'a String> {
    let id = camel_to_id(name);
    known.get(&id).or_else(|| known.get(name))
}

fn parse<T: serde::de::DeserializeOwned>(responses: &Responses, endpoint: &'static str) -> Result<Vec<T>, DataError> {
    let body = responses.get(endpoint).ok_or(DataError::Missing(endpoint))?;
    serde_json::from_str(body).map_err(|e| DataError::Parse(format!("{endpoint}: {e}")))
}

/// An endpoint added after the others: saved responses from before it
/// lack it, which reads as empty.
fn parse_optional<T: serde::de::DeserializeOwned>(
    responses: &Responses,
    endpoint: &'static str,
) -> Result<Vec<T>, DataError> {
    if responses.contains_key(endpoint) { parse(responses, endpoint) } else { Ok(Vec::new()) }
}

// ---------------------------------------------------------------- mapping

fn map_card(c: WCard) -> Card {
    let x_cost = c.is_x_cost.unwrap_or(false);
    Card {
        id: CardId(c.id),
        name: c.name,
        color: c.color.unwrap_or_else(|| "colorless".into()),
        card_type: card_type(c.card_type.as_deref()),
        rarity: rarity(c.rarity_key.as_deref()),
        cost: c.cost.and_then(|n| u8::try_from(n).ok()),
        x_cost,
        text: strip_markup(&c.description),
        upgrade_text: c.upgrade_description.as_deref().map(strip_markup),
        keywords: c.keywords_key.unwrap_or_default(),
        tags: c.tags.unwrap_or_default(),
        target: target(c.target.as_deref()),
        damage: c.damage,
        hits: c.hit_count,
        block: c.block,
        draw: c.cards_draw,
        energy: c.energy_gain,
        powers: c
            .powers_applied
            .unwrap_or_default()
            .into_iter()
            .filter_map(|p| Some(PowerAmount { power: p.power_key.or(p.power)?, amount: p.amount.unwrap_or(0) }))
            .collect(),
        vars: c
            .vars
            .into_iter()
            .filter_map(|(k, v)| Some((k.to_lowercase(), i32::try_from(v.as_i64()?).ok()?)))
            .collect(),
        upgrade: upgrade(c.upgrade),
        star_cost: c.star_cost.and_then(|n| u8::try_from(n).ok()),
        channels: Vec::new(),
    }
}

/// Codex upgrades: `{"damage": "+3", "cost": 0, "add_innate": 1, "remove_exhaust": 1}`.
fn upgrade(raw: BTreeMap<String, serde_json::Value>) -> Upgrade {
    let mut u = Upgrade::default();
    for (key, value) in raw {
        let number = value.as_i64().or_else(|| value.as_str().and_then(|s| s.trim_start_matches('+').parse().ok()));
        let Some(n) = number.and_then(|n| i32::try_from(n).ok()) else { continue };
        let keyword = |k: &str| {
            let mut c = k.chars();
            c.next().map(|f| f.to_ascii_uppercase().to_string() + c.as_str()).unwrap_or_default()
        };
        if key == "cost" {
            u.cost = u8::try_from(n).ok();
        } else if let Some(k) = key.strip_prefix("add_") {
            u.adds.push(keyword(k));
        } else if let Some(k) = key.strip_prefix("remove_") {
            u.removes.push(keyword(k));
        } else if key != "description_changed" {
            u.deltas.insert(key, n);
        }
    }
    u
}

/// `Channel 2 Frost`, `Channel X Lightning` in plain card text. Phrases
/// naming no known orb ("Channel 1 random Orb") are left out.
fn channels(text: &str, orbs: &[Orb]) -> Vec<OrbChannel> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(i) = rest.find("Channel ") {
        rest = &rest[i + "Channel ".len()..];
        let mut words = rest.split_whitespace();
        let (Some(count), Some(name)) = (words.next(), words.next()) else { break };
        let name = name.trim_end_matches(|c: char| !c.is_alphanumeric());
        let Some(orb) = orbs.iter().find(|o| o.name.eq_ignore_ascii_case(name)) else { continue };
        let (count, x) = match count {
            "X" => (1, true),
            n => match n.parse() {
                Ok(n) => (n, false),
                Err(_) => continue,
            },
        };
        out.push(OrbChannel { orb: orb.id.clone(), count, x });
    }
    out
}

/// Reads an orb's passive and evoke from its text (`Passive: ... Evoke: ...`).
fn map_orb(o: WOrb) -> Orb {
    let raw = o.description.replace("[gold]", "").replace("[/gold]", "");
    let (passive, evoke) = raw.split_once("Evoke:").unwrap_or((raw.as_str(), ""));
    let number = |s: &str| -> i32 {
        let blue = s.split("[blue]").nth(1).and_then(|r| r.split("[/blue]").next());
        let energy = s.split("[energy:").nth(1).and_then(|r| r.split(']').next());
        blue.or(energy).and_then(|n| n.trim().parse().ok()).unwrap_or(0)
    };
    let effect = |s: &str| {
        let has = |n: &str| spire_core::playbook::contains_ci(s, n);
        if has("increase this orb") {
            OrbEffect::Grow
        } else if has("damage") && has("all enemies") {
            OrbEffect::DamageAll
        } else if has("damage") {
            OrbEffect::Damage
        } else if has("block") {
            OrbEffect::Block
        } else if has("energy") {
            OrbEffect::Energy
        } else {
            OrbEffect::Other
        }
    };
    Orb {
        id: OrbId(o.id),
        name: o.name,
        text: strip_markup(&o.description),
        passive: number(passive),
        passive_effect: effect(passive),
        evoke: number(evoke),
        evoke_effect: effect(evoke),
        focus: !spire_core::playbook::contains_ci(&raw, "unaffected by focus"),
        // "…and reduce this orb's value by [blue]1[/blue]" (Glass).
        decay: raw.split("reduce this orb's value by").nth(1).map(number).unwrap_or(0),
    }
}

/// Powers whose text says the holder deals damage once it's killed.
fn death_blow_powers(powers: &[WPowerDef]) -> Vec<(String, String)> {
    powers_where(powers, |t| t.contains("when killed") && t.contains("deals damage"))
}

/// Powers whose text says the holder summons monsters when it dies.
fn death_summon_powers(powers: &[WPowerDef]) -> Vec<(String, String)> {
    powers_where(powers, |t| {
        (t.contains("upon dying") || t.contains("when killed") || t.contains("when this dies")) && t.contains("summon")
    })
}

fn powers_where(powers: &[WPowerDef], pick: impl Fn(&str) -> bool) -> Vec<(String, String)> {
    powers
        .iter()
        .map(|p| (p.id.clone(), strip_markup(&p.description)))
        .filter(|(_, text)| pick(&text.to_lowercase()))
        .collect()
}

/// The powers a monster starts with or gives itself, with their text, each
/// once (the first amount the data gives). Strength is left out: patterns
/// already carry it.
fn own_powers(m: &Monster, texts: &[(String, String)]) -> Vec<MonsterPower> {
    let mut out: Vec<MonsterPower> = Vec::new();
    for (power, amount) in gained(m) {
        if power == "STRENGTH" || out.iter().any(|p| p.power == power) {
            continue;
        }
        let text = texts.iter().find(|(id, _)| id == power).map_or_else(String::new, |(_, t)| t.clone());
        out.push(MonsterPower { power: power.to_string(), text, amount: Some(amount).filter(|a| *a > 0) });
    }
    out
}

/// Powers a monster starts with, then those its moves give itself.
fn gained(m: &Monster) -> impl Iterator<Item = (&str, i32)> {
    m.innate_powers.iter().map(|p| (p.power.as_str(), p.amount)).chain(m.moves.iter().flat_map(|mv| {
        mv.powers.iter().filter(|p| p.target == PowerTarget::Monster).map(|p| (p.power.as_str(), p.amount))
    }))
}

/// The first of `powers` the monster starts with or gains from its moves.
fn death_power(m: &Monster, powers: &[(String, String)]) -> Option<MonsterPower> {
    for (power, amount) in gained(m) {
        if let Some((id, text)) = powers.iter().find(|(id, _)| id == power) {
            return Some(MonsterPower {
                power: id.clone(),
                text: text.clone(),
                amount: Some(amount).filter(|a| *a > 0),
            });
        }
    }
    None
}

fn map_monster(m: WMonster) -> Monster {
    Monster {
        id: MonsterId(m.id),
        name: m.name,
        hp_min: m.min_hp.unwrap_or(0),
        hp_max: m.max_hp,
        moves: m
            .moves
            .unwrap_or_default()
            .into_iter()
            .map(|mv| MonsterMove {
                id: mv.id,
                name: mv.name,
                intent: mv.intent,
                damage: mv.damage.as_ref().and_then(|d| d.normal),
                hits: mv.damage.as_ref().and_then(|d| d.hit_count),
                block: mv.block,
                damage_asc: mv.damage.as_ref().and_then(|d| d.ascension),
                powers: mv
                    .powers
                    .unwrap_or_default()
                    .into_iter()
                    .map(|p| MovePower {
                        power: p.power_id,
                        target: match p.target.as_deref() {
                            Some("self") => PowerTarget::Monster,
                            Some("player") => PowerTarget::Player,
                            _ => PowerTarget::Other,
                        },
                        amount: p.amount.unwrap_or(0),
                    })
                    .collect(),
            })
            .collect(),
        innate_powers: m
            .innate_powers
            .unwrap_or_default()
            .into_iter()
            .map(|p| PowerAmount { power: p.power_id, amount: p.amount.unwrap_or(0) })
            .collect(),
        hp_min_asc: m.min_hp_ascension,
        hp_max_asc: m.max_hp_ascension,
        pattern: m.attack_pattern.map(map_pattern),
        death_blow: None,
        death_summons: None,
        powers: Vec::new(),
        phases: Vec::new(),
    }
}

/// The move state machine. Duplicate state ids keep their last entry with
/// branches (the source repeats a random state, once empty).
fn map_pattern(p: WPattern) -> AttackPattern {
    let mut states: Vec<PatternState> = Vec::new();
    for s in p.states.unwrap_or_default() {
        let kind = match s.kind.as_str() {
            "move" => StateKind::Move,
            "random" => StateKind::Random,
            _ => StateKind::Conditional,
        };
        let branches: Vec<PatternBranch> = s
            .branches
            .unwrap_or_default()
            .into_iter()
            .map(|b| PatternBranch {
                move_id: b.move_id,
                weight: b.weight,
                no_repeat: b.repeat.as_deref() == Some("CannotRepeat"),
                max_times: b.max_times,
                condition: b.condition.as_deref().map(condition_words),
            })
            .collect();
        let state = PatternState { id: s.id, kind, move_id: s.move_id, next: s.next, branches };
        match states.iter_mut().find(|x| x.id == state.id) {
            Some(old) if old.branches.is_empty() => *old = state,
            Some(_) => {}
            None => states.push(state),
        }
    }
    AttackPattern { initial: p.initial_move, states, description: p.description.unwrap_or_default() }
}

/// A game condition in plain words: `_curseOfKnowledgeCounter < 3` reads
/// "curse of knowledge counter < 3", `!HasAmalgamDied` "not has amalgam died".
pub fn condition_words(c: &str) -> String {
    let c = c.trim().trim_start_matches("base.").trim_start_matches("Creature.");
    let (neg, c) = match c.strip_prefix('!') {
        Some(rest) => (true, rest),
        None => (false, c),
    };
    let mut out = String::new();
    let mut prev_lower = false;
    for ch in c.chars() {
        match ch {
            '_' | '.' => {
                if !out.ends_with(' ') && !out.is_empty() {
                    out.push(' ');
                }
                prev_lower = false;
            }
            c if c.is_ascii_uppercase() => {
                if prev_lower {
                    out.push(' ');
                }
                out.push(c.to_ascii_lowercase());
                prev_lower = false;
            }
            c => {
                out.push(c);
                prev_lower = c.is_ascii_lowercase() || c.is_ascii_digit();
            }
        }
    }
    let out = out.split_whitespace().collect::<Vec<_>>().join(" ");
    if neg { format!("not {out}") } else { out }
}

#[derive(Deserialize)]
struct WAscension {
    level: u32,
    #[serde(default)]
    name: String,
    #[serde(default)]
    description: String,
}

/// Reads the mechanical ascension rules from the level descriptions.
/// A rule whose description is gone is switched off, not guessed.
pub fn rules_from(levels: &[AscensionLevel]) -> Option<GameRules> {
    let max = levels.iter().map(|l| l.level).max()?;
    let find = |needles: &[&str]| {
        levels.iter().find(|l| needles.iter().any(|n| spire_core::playbook::contains_ci(&l.text, n))).map(|l| l.level)
    };
    let defaults = GameRules::default();
    Some(GameRules {
        max_ascension: max,
        fewer_potion_slot_from: find(&["less potion slot", "fewer potion slot"]),
        cursed_start_from: find(&["cursed"]),
        tougher_enemies_from: find(&["harder to kill"]),
        deadlier_enemies_from: find(&["deadlier"]),
        double_boss_from: find(&["2 bosses", "two bosses"]),
        // "Ancients only heal 80% of your missing HP."
        weary_from: find(&["ancients only heal"]),
        weary_heal_pct: levels
            .iter()
            .find(|l| spire_core::playbook::contains_ci(&l.text, "ancients only heal"))
            .and_then(|l| l.text.split('%').next()?.rsplit(' ').next()?.parse().ok())
            .unwrap_or(100),
        ..defaults
    })
}

/// Builds a [`DataPack`] from one response per endpoint in [`ENDPOINTS`].
pub fn to_pack(
    responses: &Responses,
    game_version: Option<String>,
    fetched_at: Option<String>,
) -> Result<DataPack, DataError> {
    let acts: Vec<WAct> = parse(responses, "acts")?;
    let orbs: Vec<Orb> = parse_optional::<WOrb>(responses, "orbs")?.into_iter().map(map_orb).collect();
    let cards: Vec<Card> = parse::<WCard>(responses, "cards")?
        .into_iter()
        .map(map_card)
        .map(|c| Card { channels: channels(&c.text, &orbs), ..c })
        .collect();
    let relics: Vec<Relic> = parse::<WItem>(responses, "relics")?.into_iter().map(map_relic).collect();
    let card_ids: BTreeSet<String> = cards.iter().map(|c| c.id.0.clone()).collect();
    let relic_ids: BTreeSet<String> = relics.iter().map(|r| r.id.0.clone()).collect();
    let characters = parse::<WCharacter>(responses, "characters")?
        .into_iter()
        .map(|c| map_character(c, &card_ids, &relic_ids))
        .collect();
    let potions = parse::<WItem>(responses, "potions")?.into_iter().map(map_potion).collect();
    let enchantments = parse::<WEnchantment>(responses, "enchantments")?.into_iter().map(map_enchantment).collect();

    let powers = parse_optional::<WPowerDef>(responses, "powers")?;
    let (blows, summons) = (death_blow_powers(&powers), death_summon_powers(&powers));
    let texts: Vec<(String, String)> = powers.iter().map(|p| (p.id.clone(), strip_markup(&p.description))).collect();
    let monsters = parse::<WMonster>(responses, "monsters")?
        .into_iter()
        .map(map_monster)
        .map(|m| Monster {
            death_blow: death_power(&m, &blows),
            death_summons: death_power(&m, &summons),
            powers: own_powers(&m, &texts),
            ..m
        })
        .collect();

    // Act membership comes from the acts endpoint: its lists are ids, while
    // the encounter/event `act` fields are display strings.
    let encounter_act: BTreeMap<&str, &str> = acts
        .iter()
        .flat_map(|a| a.encounters.iter().chain(&a.bosses).map(move |e| (e.as_str(), a.id.as_str())))
        .collect();
    let encounters = parse::<WEncounter>(responses, "encounters")?
        .into_iter()
        .filter_map(|e| map_encounter(e, &encounter_act))
        .collect();
    let mut event_acts: BTreeMap<&str, Vec<ActId>> = BTreeMap::new();
    for a in &acts {
        for e in a.events.iter().chain(&a.ancients) {
            event_acts.entry(e.as_str()).or_default().push(ActId::new(&a.id));
        }
    }
    let event_relics: Vec<&Relic> = relics.iter().filter(|r| r.rarity == Rarity::Event).collect();
    let events =
        parse::<WEvent>(responses, "events")?.into_iter().map(|e| map_event(e, &event_acts, &event_relics)).collect();

    let ascensions: Vec<AscensionLevel> = parse_optional::<WAscension>(responses, "ascensions")?
        .into_iter()
        .map(|a| AscensionLevel { level: a.level, name: a.name, text: strip_markup(&a.description) })
        .collect();
    let mut rules = rules_from(&ascensions);
    if let Some(r) = rules.as_mut().filter(|_| !powers.is_empty()) {
        r.vulnerable_pct = vulnerable_pct(&powers);
    }
    let acts = acts.into_iter().map(map_act).collect();

    Ok(DataPack {
        format: DATA_PACK_FORMAT,
        source: "spire-codex".into(),
        game_version,
        fetched_at,
        characters,
        cards,
        relics,
        potions,
        enchantments,
        monsters,
        encounters,
        events,
        acts,
        ascensions,
        orbs,
        rules,
        // From community runs, added after (`fetch::add_community_data`).
        benchmarks: Vec::new(),
        card_lines: Vec::new(),
        card_lift: responses.get(CARD_METRICS).map(|b| card_lift(b)).unwrap_or_default(),
    })
}

fn map_relic(r: WItem) -> Relic {
    Relic {
        id: RelicId(r.id),
        name: r.name,
        rarity: rarity(r.rarity_key.as_deref()),
        pool: r.pool.unwrap_or_else(|| "shared".into()),
        text: strip_markup(&r.description),
    }
}

fn map_potion(p: WItem) -> Potion {
    Potion {
        id: PotionId(p.id),
        name: p.name,
        rarity: rarity(p.rarity_key.as_deref()),
        pool: p.pool.unwrap_or_else(|| "shared".into()),
        text: strip_markup(&p.description),
    }
}

/// Starting decks and relics are named in the source's own casing; they
/// resolve against the card and relic ids.
fn map_character(c: WCharacter, card_ids: &BTreeSet<String>, relic_ids: &BTreeSet<String>) -> Character {
    Character {
        id: CharacterId(c.id),
        name: c.name,
        max_hp: c.starting_hp.unwrap_or(0),
        gold: c.starting_gold.unwrap_or(0),
        energy: c.max_energy.unwrap_or(3),
        starting_deck: c
            .starting_deck
            .iter()
            .map(|n| CardId(resolve(n, card_ids).cloned().unwrap_or_else(|| camel_to_id(n))))
            .collect(),
        starting_relics: c
            .starting_relics
            .iter()
            .map(|n| RelicId(resolve(n, relic_ids).cloned().unwrap_or_else(|| camel_to_id(n))))
            .collect(),
        description: strip_markup(&c.description),
        orb_slots: c.orb_slots,
        arrow_slots: c.arrow_slots,
    }
}

fn map_enchantment(e: WEnchantment) -> Enchantment {
    Enchantment {
        id: EnchantmentId(e.id),
        name: e.name,
        text: strip_markup(&e.description),
        // "Attack, Skill" lists several types.
        card_types: e
            .card_type
            .as_deref()
            .map(|t| t.split(',').map(|x| card_type(Some(x.trim()))).collect())
            .unwrap_or_default(),
        applicable_to: e.applicable_to,
        stackable: e.is_stackable.unwrap_or(false),
    }
}

/// `None` for room types we don't track.
fn map_encounter(e: WEncounter, act_of: &BTreeMap<&str, &str>) -> Option<Encounter> {
    Some(Encounter {
        act: act_of.get(e.id.as_str()).map(|a| ActId::new(*a)),
        room: room(&e.room_type)?,
        weak: e.is_weak.unwrap_or(false),
        monsters: e.monsters.into_iter().map(|m| MonsterId(m.id)).collect(),
        tags: e.tags.unwrap_or_default(),
        lineups: Vec::new(),
        spawns: Vec::new(),
        stats: None,
        id: EncounterId(e.id),
        name: e.name,
    })
}

fn map_event(e: WEvent, acts_of: &BTreeMap<&str, Vec<ActId>>, event_relics: &[&Relic]) -> Event {
    let kind = match e.kind.as_str() {
        "Ancient" => EventKind::Ancient,
        "Shared" => EventKind::Shared,
        _ => EventKind::Act,
    };
    let option = |o: &WOption| EventOption {
        id: o.id.clone(),
        title: strip_markup(&o.title),
        text: strip_markup(&o.description),
        effects: Vec::new(),
    };
    let options: Vec<EventOption> = e.options.iter().flatten().map(option).collect();
    // Every option on every screen, for the relics they name.
    let every: Vec<EventOption> = options
        .iter()
        .cloned()
        .chain(e.pages.iter().flatten().flat_map(|p| p.options.iter().flatten()).map(option))
        .collect();
    let relics = event_relics
        .iter()
        .filter_map(|r| {
            let gives = every.iter().find(|o| names(&o.text, &r.name) || names(&o.title, &r.name))?;
            Some(EventRelic { relic: r.id.clone(), option: Some(gives.title.clone()), one_of: 1 })
        })
        .collect();
    let requires: Vec<String> = e.preconditions.unwrap_or_default().iter().map(|p| strip_markup(p)).collect();
    Event {
        acts: acts_of.get(e.id.as_str()).cloned().unwrap_or_default(),
        act_span: requires.iter().find_map(|r| act_span(r)),
        requires,
        relics,
        met: BTreeMap::new(),
        options,
        id: EventId(e.id),
        name: e.name,
        kind,
    }
}

/// Whether `text` names `name` as whole words, ignoring case.
pub(crate) fn names(text: &str, name: &str) -> bool {
    let (text, name) = (text.to_lowercase(), name.to_lowercase());
    let word = |c: Option<char>| c.is_some_and(char::is_alphanumeric);
    text.match_indices(&name)
        .any(|(at, _)| !word(text[..at].chars().next_back()) && !word(text[at + name.len()..].chars().next()))
}

/// The 0-based acts a precondition limits an event to: "Act 2 only",
/// "Act 1–2 only", "Act 2+".
fn act_span(precondition: &str) -> Option<(u8, u8)> {
    let rest = precondition.trim().strip_prefix("Act ")?;
    let num = |s: &str| s.trim().parse::<u8>().ok().filter(|n| *n > 0).map(|n| n - 1);
    if let Some(n) = rest.strip_suffix('+') {
        return Some((num(n)?, u8::MAX));
    }
    let span = rest.strip_suffix(" only")?;
    match span.split_once(['–', '-']) {
        Some((a, b)) => Some((num(a)?, num(b)?)),
        None => num(span).map(|n| (n, n)),
    }
}

/// Room counts and layouts come later, from community runs.
fn map_act(a: WAct) -> Act {
    Act {
        id: ActId(a.id),
        name: a.name,
        index: a.index,
        rooms: a.num_rooms.unwrap_or(0),
        bosses: a.bosses.into_iter().map(EncounterId).collect(),
        ancients: a.ancients.into_iter().map(EventId).collect(),
        events: a.events.into_iter().map(EventId).collect(),
        encounters: a.encounters.into_iter().map(EncounterId).collect(),
        weak_fights: None,
        length: None,
        fixed_rooms: Vec::new(),
        unknown_treasure: None,
        chest_rarity: Default::default(),
    }
}

/// Reads `beta_version` from the `/api/beta/version` body.
pub fn beta_version(body: &str) -> Option<String> {
    #[derive(Deserialize)]
    struct V {
        beta_version: String,
    }
    serde_json::from_str::<V>(body).ok().map(|v| v.beta_version)
}

/// Picks the latest stable version from the `/api/versions` body.
pub fn latest_stable_version(body: &str) -> Option<String> {
    #[derive(Deserialize)]
    struct V {
        version: String,
        #[serde(default)]
        is_beta: bool,
    }
    let versions: Vec<V> = serde_json::from_str(body).ok()?;
    versions.into_iter().find(|v| !v.is_beta).map(|v| v.version)
}

#[cfg(test)]
mod tests {
    #[test]
    fn reads_act_limits_from_preconditions() {
        assert_eq!(act_span("Act 2 only"), Some((1, 1)));
        assert_eq!(act_span("Act 1–2 only"), Some((0, 1)));
        assert_eq!(act_span("Act 2+"), Some((1, u8::MAX)));
        assert_eq!(act_span("Requires 100+ gold"), None);
        assert_eq!(act_span("Floor 7+"), None);
    }

    #[test]
    fn relic_names_match_whole_words() {
        assert!(names("Obtain the Chosen Cheese.", "The Chosen Cheese"));
        assert!(names("Lose 8 HP. Obtain Wuppy with 30 HP.", "Wuppy"));
        assert!(!names("Obtain Lanterns.", "Lantern"));
    }

    use super::*;

    #[test]
    fn strips_markup() {
        assert_eq!(strip_markup("Gain 5 [gold]Block[/gold]."), "Gain 5 Block.");
        assert_eq!(strip_markup("Gain [energy:2]."), "Gain 2 Energy.");
        assert_eq!(strip_markup("[blue]X[/blue] [gold]Block[/gold]"), "X Block");
        assert_eq!(strip_markup("unclosed [tag"), "unclosed [tag");
    }

    #[test]
    fn converts_class_names() {
        assert_eq!(camel_to_id("RingOfTheSnake"), "RING_OF_THE_SNAKE");
        assert_eq!(camel_to_id("StrikeDefect"), "STRIKE_DEFECT");
        assert_eq!(camel_to_id("Zap"), "ZAP");
    }

    #[test]
    fn parses_enums_from_codex_casing() {
        assert_eq!(card_type(Some("Attack")), CardType::Attack);
        assert_eq!(card_type(Some("Mystery")), CardType::Other);
        assert_eq!(rarity(Some("Uncommon")), Rarity::Uncommon);
    }

    #[test]
    fn reads_upgrades_vars_and_orbs() {
        let up = upgrade(
            serde_json::from_str(
                r#"{"damage": "+3", "cost": 0, "add_innate": 1, "remove_exhaust": 1, "description_changed": 1}"#,
            )
            .unwrap(),
        );
        assert_eq!(up.cost, Some(0));
        assert_eq!(up.deltas.get("damage"), Some(&3));
        assert_eq!(
            (up.adds.as_slice(), up.removes.as_slice()),
            (&["Innate".to_string()][..], &["Exhaust".to_string()][..])
        );
        assert!(!up.deltas.contains_key("description_changed"));

        let frost = map_orb(WOrb {
            id: "FROST_ORB".into(),
            name: "Frost".into(),
            description: "[gold]Passive:[/gold] At the end of turn, gain [blue]2[/blue] [gold]Block[/gold].\n\
                          [gold]Evoke:[/gold] Gain [blue]5[/blue] [gold]Block[/gold]."
                .into(),
        });
        assert_eq!(
            (frost.passive, frost.passive_effect, frost.evoke, frost.evoke_effect),
            (2, OrbEffect::Block, 5, OrbEffect::Block)
        );
        let plasma = map_orb(WOrb {
            id: "PLASMA_ORB".into(),
            name: "Plasma".into(),
            description: "[gold]Passive:[/gold] At the start of turn, gain [energy:1].\n\
                          [gold]Evoke:[/gold] Gain [energy:2]. Plasma is unaffected by [gold]Focus[/gold]."
                .into(),
        });
        assert_eq!((plasma.passive, plasma.evoke, plasma.passive_effect), (1, 2, OrbEffect::Energy));
        assert!(!plasma.focus);
        let dark = map_orb(WOrb {
            id: "DARK_ORB".into(),
            name: "Dark".into(),
            description: "[gold]Passive:[/gold] At the end of turn, increase this Orb's damage by [blue]6[/blue].\n\
                          [gold]Evoke:[/gold] Deal [blue]6[/blue] damage to the enemy with the lowest HP."
                .into(),
        });
        assert_eq!((dark.passive_effect, dark.evoke_effect), (OrbEffect::Grow, OrbEffect::Damage));

        let orbs = [frost, plasma];
        let found = channels("Gain 6 Block. Channel 2 Frost. Channel X Plasma. Channel 1 random Orb.", &orbs);
        assert_eq!(
            found,
            vec![
                OrbChannel { orb: OrbId::new("FROST_ORB"), count: 2, x: false },
                OrbChannel { orb: OrbId::new("PLASMA_ORB"), count: 1, x: true },
            ]
        );
    }

    #[test]
    fn reads_vulnerable_from_powers() {
        let powers = [
            WPowerDef { id: "WEAK".into(), description: "deal [blue]25%[/blue] less".into() },
            WPowerDef {
                id: "VULNERABLE".into(),
                description: "Receive [blue]50%[/blue] more damage from Attacks for [blue][Amount][/blue] turns."
                    .into(),
            },
        ];
        assert_eq!(vulnerable_pct(&powers), Some(50));
        assert_eq!(vulnerable_pct(&powers[..1]), None);
    }

    #[test]
    fn keeps_card_vars_lowercased() {
        let card: WCard = serde_json::from_str(
            r#"{"id": "FALLING_STAR", "name": "Falling Star", "description": "Deal 8 damage.", "cost": 0,
                "vars": {"Damage": 8, "StarCost": 2, "Odd": "text"}, "upgrade": {"damage": "+4"}, "star_cost": 2}"#,
        )
        .unwrap();
        let card = map_card(card);
        assert_eq!(card.vars.get("damage"), Some(&8));
        assert_eq!(card.vars.get("starcost"), Some(&2));
        assert!(!card.vars.contains_key("odd"), "non-numeric vars are dropped");
        assert_eq!(card.star_cost, Some(2));
        assert_eq!((card.value("damage", false), card.value("damage", true)), (8, 12));
    }

    #[test]
    fn finds_powers_that_strike_after_death() {
        let powers = [
            WPowerDef {
                id: "STEAM_ERUPTION".into(),
                description: "When killed, deals damage at the end of your next turn.".into(),
            },
            WPowerDef { id: "STRENGTH".into(), description: "Increases attack damage.".into() },
        ];
        let blows = death_blow_powers(&powers);
        assert_eq!(blows.len(), 1);
        let infested = [WPowerDef { id: "INFESTED".into(), description: "Upon dying, summons... something.".into() }];
        assert_eq!(death_summon_powers(&infested).len(), 1);
        assert!(death_summon_powers(&powers).is_empty());
        let giant = Monster {
            id: MonsterId::new("GIANT"),
            name: "Giant".into(),
            hp_min: 240,
            hp_max: None,
            moves: vec![MonsterMove {
                id: "PRESSURIZE".into(),
                powers: vec![MovePower { power: "STEAM_ERUPTION".into(), target: PowerTarget::Monster, amount: 15 }],
                ..Default::default()
            }],
            innate_powers: vec![],
            hp_min_asc: None,
            hp_max_asc: None,
            pattern: None,
            death_blow: None,
            death_summons: None,
            powers: Vec::new(),
            phases: Vec::new(),
        };
        let blow = death_power(&giant, &blows).unwrap();
        assert_eq!((blow.power.as_str(), blow.amount), ("STEAM_ERUPTION", Some(15)));
    }

    #[test]
    fn reads_conditions_as_words() {
        assert_eq!(condition_words("_curseOfKnowledgeCounter < 3"), "curse of knowledge counter < 3");
        assert_eq!(condition_words("!HasAmalgamDied"), "not has amalgam died");
        assert_eq!(condition_words("base.Creature.SlotName == \"wriggler1\""), "slot name == \"wriggler1\"");
    }

    #[test]
    fn maps_patterns_keeping_the_random_state_with_branches() {
        let json = r#"{"initial_move":"A","description":"A then random","states":[
            {"id":"A_MOVE","type":"move","move_id":"A","next":"RAND"},
            {"id":"RAND","type":"random","branches":[]},
            {"id":"RAND","type":"random","branches":[{"move_id":"A","weight":1.0,"repeat":"CannotRepeat"}]}]}"#;
        let p = map_pattern(serde_json::from_str(json).unwrap());
        assert_eq!(p.initial.as_deref(), Some("A"));
        assert_eq!(p.states.len(), 2);
        assert_eq!(p.states[1].kind, StateKind::Random);
        assert!(p.states[1].branches[0].no_repeat);
    }
}
