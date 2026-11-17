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
pub const ENDPOINTS: [&str; 9] =
    ["acts", "characters", "cards", "relics", "potions", "enchantments", "monsters", "encounters", "events"];

/// Raw response bodies keyed by endpoint name.
pub type Responses = BTreeMap<&'static str, String>;

// ---------------------------------------------------------------- wire types

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
    #[serde(default)]
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
}

#[derive(Deserialize)]
struct WCharacter {
    id: String,
    name: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    starting_hp: Option<u32>,
    #[serde(default)]
    starting_gold: Option<u32>,
    #[serde(default)]
    max_energy: Option<u32>,
    #[serde(default)]
    starting_deck: Vec<String>,
    #[serde(default)]
    starting_relics: Vec<String>,
}

#[derive(Deserialize)]
struct WItem {
    id: String,
    name: String,
    #[serde(default)]
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
    #[serde(default)]
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
    hit_count: Option<u32>,
}

#[derive(Deserialize)]
struct WMove {
    #[serde(default)]
    name: String,
    #[serde(default)]
    intent: String,
    #[serde(default)]
    damage: Option<WDamage>,
    #[serde(default)]
    block: Option<u32>,
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
    moves: Option<Vec<WMove>>,
    #[serde(default)]
    innate_powers: Option<Vec<WInnate>>,
}

#[derive(Deserialize)]
struct WRef {
    id: String,
}

#[derive(Deserialize)]
struct WEncounter {
    id: String,
    name: String,
    #[serde(default)]
    room_type: String,
    #[serde(default)]
    is_weak: Option<bool>,
    #[serde(default)]
    tags: Option<Vec<String>>,
    #[serde(default)]
    monsters: Vec<WRef>,
}

#[derive(Deserialize)]
struct WOption {
    #[serde(default)]
    title: String,
    #[serde(default)]
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
}

#[derive(Deserialize)]
struct WAct {
    id: String,
    name: String,
    #[serde(default)]
    index: u8,
    #[serde(default)]
    num_rooms: Option<u32>,
    #[serde(default)]
    bosses: Vec<String>,
    #[serde(default)]
    ancients: Vec<String>,
    #[serde(default)]
    events: Vec<String>,
    #[serde(default)]
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
    }
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
                name: mv.name,
                intent: mv.intent,
                damage: mv.damage.as_ref().and_then(|d| d.normal),
                hits: mv.damage.as_ref().and_then(|d| d.hit_count),
                block: mv.block,
            })
            .collect(),
        innate_powers: m
            .innate_powers
            .unwrap_or_default()
            .into_iter()
            .map(|p| PowerAmount { power: p.power_id, amount: p.amount.unwrap_or(0) })
            .collect(),
    }
}

/// Builds a [`DataPack`] from one response per endpoint in [`ENDPOINTS`].
pub fn to_pack(
    responses: &Responses,
    game_version: Option<String>,
    fetched_at: Option<String>,
) -> Result<DataPack, DataError> {
    let acts: Vec<WAct> = parse(responses, "acts")?;
    let cards: Vec<Card> = parse::<WCard>(responses, "cards")?.into_iter().map(map_card).collect();
    let card_ids: BTreeSet<String> = cards.iter().map(|c| c.id.0.clone()).collect();

    let relics: Vec<Relic> = parse::<WItem>(responses, "relics")?
        .into_iter()
        .map(|r| Relic {
            id: RelicId(r.id),
            name: r.name,
            rarity: rarity(r.rarity_key.as_deref()),
            pool: r.pool.unwrap_or_else(|| "shared".into()),
            text: strip_markup(&r.description),
        })
        .collect();
    let relic_ids: BTreeSet<String> = relics.iter().map(|r| r.id.0.clone()).collect();

    let characters = parse::<WCharacter>(responses, "characters")?
        .into_iter()
        .map(|c| Character {
            id: CharacterId(c.id),
            name: c.name,
            max_hp: c.starting_hp.unwrap_or(0),
            gold: c.starting_gold.unwrap_or(0),
            energy: c.max_energy.unwrap_or(3),
            starting_deck: c
                .starting_deck
                .iter()
                .map(|n| CardId(resolve(n, &card_ids).cloned().unwrap_or_else(|| camel_to_id(n))))
                .collect(),
            starting_relics: c
                .starting_relics
                .iter()
                .map(|n| RelicId(resolve(n, &relic_ids).cloned().unwrap_or_else(|| camel_to_id(n))))
                .collect(),
            description: strip_markup(&c.description),
        })
        .collect();

    let potions = parse::<WItem>(responses, "potions")?
        .into_iter()
        .map(|p| Potion {
            id: PotionId(p.id),
            name: p.name,
            rarity: rarity(p.rarity_key.as_deref()),
            pool: p.pool.unwrap_or_else(|| "shared".into()),
            text: strip_markup(&p.description),
        })
        .collect();

    let enchantments = parse::<WEnchantment>(responses, "enchantments")?
        .into_iter()
        .map(|e| Enchantment {
            id: EnchantmentId(e.id),
            name: e.name,
            text: strip_markup(&e.description),
            card_type: e.card_type.as_deref().map(|t| card_type(Some(t))),
            applicable_to: e.applicable_to,
            stackable: e.is_stackable.unwrap_or(false),
        })
        .collect();

    let monsters = parse::<WMonster>(responses, "monsters")?.into_iter().map(map_monster).collect();

    // Act membership comes from the acts endpoint: its lists are ids, while
    // the encounter/event `act` fields are display strings.
    let encounter_act: BTreeMap<&str, &str> = acts
        .iter()
        .flat_map(|a| a.encounters.iter().chain(&a.bosses).map(move |e| (e.as_str(), a.id.as_str())))
        .collect();
    let encounters = parse::<WEncounter>(responses, "encounters")?
        .into_iter()
        .filter_map(|e| {
            Some(Encounter {
                act: encounter_act.get(e.id.as_str()).map(|a| ActId::new(*a)),
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
        })
        .collect();

    let mut event_acts: BTreeMap<&str, Vec<ActId>> = BTreeMap::new();
    for a in &acts {
        for e in a.events.iter().chain(&a.ancients) {
            event_acts.entry(e.as_str()).or_default().push(ActId::new(&a.id));
        }
    }
    let events = parse::<WEvent>(responses, "events")?
        .into_iter()
        .map(|e| {
            let kind = match e.kind.as_str() {
                "Ancient" => EventKind::Ancient,
                "Shared" => EventKind::Shared,
                _ => EventKind::Act,
            };
            Event {
                acts: event_acts.get(e.id.as_str()).cloned().unwrap_or_default(),
                options: e
                    .options
                    .unwrap_or_default()
                    .into_iter()
                    .map(|o| EventOption { title: strip_markup(&o.title), text: strip_markup(&o.description) })
                    .collect(),
                id: EventId(e.id),
                name: e.name,
                kind,
            }
        })
        .collect();

    let acts = acts
        .into_iter()
        .map(|a| Act {
            id: ActId(a.id),
            name: a.name,
            index: a.index,
            rooms: a.num_rooms.unwrap_or(0),
            bosses: a.bosses.into_iter().map(EncounterId).collect(),
            ancients: a.ancients.into_iter().map(EventId).collect(),
            events: a.events.into_iter().map(EventId).collect(),
            encounters: a.encounters.into_iter().map(EncounterId).collect(),
        })
        .collect();

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
    })
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
}
