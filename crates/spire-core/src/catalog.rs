//! The game-fact catalog: cards, characters, relics, potions, enchantments,
//! monsters, encounters, events and acts.
//!
//! These are *facts* about the game, as delivered by a data source. Strategy
//! knowledge (what a card is good for, what an act demands) lives in
//! [`crate::playbook`] instead, so the two can be updated independently.
//!
//! A [`DataPack`] is the serialized, source-agnostic form. A [`Catalog`] is
//! built by layering packs (bundled → downloaded → user overrides), with later
//! layers replacing entities of the same id.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::ids::*;

/// Bump when the `DataPack` JSON shape changes incompatibly.
pub const DATA_PACK_FORMAT: u32 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CardType {
    Attack,
    Skill,
    Power,
    Status,
    Curse,
    Quest,
    #[serde(other)]
    Other,
}

impl CardType {
    pub const ALL: [CardType; 7] = [
        CardType::Attack,
        CardType::Skill,
        CardType::Power,
        CardType::Status,
        CardType::Curse,
        CardType::Quest,
        CardType::Other,
    ];

    pub fn label(self) -> &'static str {
        match self {
            CardType::Attack => "Attack",
            CardType::Skill => "Skill",
            CardType::Power => "Power",
            CardType::Status => "Status",
            CardType::Curse => "Curse",
            CardType::Quest => "Quest",
            CardType::Other => "Other",
        }
    }
}

/// Rarity across cards, relics and potions. Unknown values from a newer data
/// source deserialize as `Other` rather than failing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Rarity {
    Basic,
    Starter,
    Common,
    Uncommon,
    Rare,
    Shop,
    Ancient,
    Event,
    Status,
    Curse,
    Token,
    Quest,
    #[serde(other)]
    Other,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Target {
    #[serde(rename = "self")]
    Own,
    SingleEnemy,
    AllEnemies,
    RandomEnemy,
    Ally,
    AllAllies,
    #[default]
    #[serde(other)]
    None,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PowerAmount {
    pub power: String,
    pub amount: i32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Card {
    pub id: CardId,
    pub name: String,
    /// Card pool: a lowercase character id (`"ironclad"`), or `"colorless"`,
    /// `"curse"`, `"status"`, `"event"`, `"token"`, `"quest"`.
    pub color: String,
    pub card_type: CardType,
    pub rarity: Rarity,
    /// Energy cost. `None` means unplayable.
    #[serde(default)]
    pub cost: Option<u8>,
    #[serde(default)]
    pub x_cost: bool,
    pub text: String,
    #[serde(default)]
    pub upgrade_text: Option<String>,
    #[serde(default)]
    pub keywords: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub target: Target,
    #[serde(default)]
    pub damage: Option<u32>,
    #[serde(default)]
    pub hits: Option<u32>,
    #[serde(default)]
    pub block: Option<u32>,
    #[serde(default)]
    pub draw: Option<u32>,
    #[serde(default)]
    pub energy: Option<u32>,
    #[serde(default)]
    pub powers: Vec<PowerAmount>,
}

impl Card {
    pub fn cost_label(&self) -> String {
        match (self.x_cost, self.cost) {
            (true, _) => "X".into(),
            (false, Some(c)) => c.to_string(),
            (false, None) => "-".into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Character {
    pub id: CharacterId,
    pub name: String,
    pub max_hp: u32,
    #[serde(default)]
    pub gold: u32,
    #[serde(default)]
    pub energy: u32,
    #[serde(default)]
    pub starting_deck: Vec<CardId>,
    #[serde(default)]
    pub starting_relics: Vec<RelicId>,
    #[serde(default)]
    pub description: String,
}

impl Character {
    /// The card `color` this character's pool uses.
    pub fn color(&self) -> String {
        self.id.as_str().to_lowercase()
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Relic {
    pub id: RelicId,
    pub name: String,
    pub rarity: Rarity,
    /// `"shared"` or a lowercase character id.
    pub pool: String,
    pub text: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Potion {
    pub id: PotionId,
    pub name: String,
    pub rarity: Rarity,
    pub pool: String,
    pub text: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Enchantment {
    pub id: EnchantmentId,
    pub name: String,
    pub text: String,
    /// Card type restriction, if any.
    #[serde(default)]
    pub card_type: Option<CardType>,
    /// Free-text restriction from the source (e.g. "Defend cards").
    #[serde(default)]
    pub applicable_to: Option<String>,
    #[serde(default)]
    pub stackable: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoomType {
    Monster,
    Elite,
    Boss,
}

impl RoomType {
    pub fn label(self) -> &'static str {
        match self {
            RoomType::Monster => "Monster",
            RoomType::Elite => "Elite",
            RoomType::Boss => "Boss",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MonsterMove {
    pub name: String,
    /// Intent label, e.g. `"Attack + Buff"`.
    pub intent: String,
    #[serde(default)]
    pub damage: Option<u32>,
    #[serde(default)]
    pub hits: Option<u32>,
    #[serde(default)]
    pub block: Option<u32>,
}

impl MonsterMove {
    pub fn total_damage(&self) -> u32 {
        self.damage.unwrap_or(0) * self.hits.unwrap_or(1).max(1)
    }

    pub fn intent_has(&self, word: &str) -> bool {
        self.intent.split(['+', ' ']).any(|w| w.eq_ignore_ascii_case(word))
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Monster {
    pub id: MonsterId,
    pub name: String,
    pub hp_min: u32,
    #[serde(default)]
    pub hp_max: Option<u32>,
    #[serde(default)]
    pub moves: Vec<MonsterMove>,
    #[serde(default)]
    pub innate_powers: Vec<PowerAmount>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Encounter {
    pub id: EncounterId,
    pub name: String,
    pub room: RoomType,
    /// Drawn from the act's easy pool (the first hallway fights of an act).
    #[serde(default)]
    pub weak: bool,
    #[serde(default)]
    pub act: Option<ActId>,
    /// Monster types that can take part, each listed once.
    pub monsters: Vec<MonsterId>,
    #[serde(default)]
    pub tags: Vec<String>,
    /// Exact starting lineups (duplicates = counts), most common first.
    /// Empty when unknown; then only `monsters` is available.
    #[serde(default)]
    pub lineups: Vec<Lineup>,
    /// Monsters that can join mid-fight (summons, splits, on-death spawns).
    #[serde(default)]
    pub spawns: Vec<MonsterId>,
    /// Community outcomes for this fight, if known.
    #[serde(default)]
    pub stats: Option<EncounterStats>,
}

/// One starting lineup and how often it occurs.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Lineup {
    /// Every monster at the start of the fight; repeats mean several.
    pub monsters: Vec<MonsterId>,
    /// Fraction of observed fights with this lineup (0–1).
    pub share: f32,
}

impl Lineup {
    /// `[(monster, count)]` in first-seen order.
    pub fn counts(&self) -> Vec<(&MonsterId, u32)> {
        let mut out: Vec<(&MonsterId, u32)> = Vec::new();
        for m in &self.monsters {
            match out.iter_mut().find(|(id, _)| *id == m) {
                Some((_, n)) => *n += 1,
                None => out.push((m, 1)),
            }
        }
        out
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EncounterStats {
    /// Fights the averages are based on.
    pub fights: u32,
    pub avg_damage_taken: f32,
    pub avg_turns: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    /// Appears in specific acts.
    Act,
    /// Can appear in any act.
    Shared,
    /// Ancient (act-start relic choice).
    Ancient,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EventOption {
    pub title: String,
    pub text: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Event {
    pub id: EventId,
    pub name: String,
    pub kind: EventKind,
    /// Acts this event can appear in. Empty means any act.
    #[serde(default)]
    pub acts: Vec<ActId>,
    #[serde(default)]
    pub options: Vec<EventOption>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Act {
    pub id: ActId,
    pub name: String,
    /// 0-based act number. Several acts can share an index (alternate act 1s).
    pub index: u8,
    #[serde(default)]
    pub rooms: u32,
    #[serde(default)]
    pub bosses: Vec<EncounterId>,
    #[serde(default)]
    pub ancients: Vec<EventId>,
    #[serde(default)]
    pub events: Vec<EventId>,
    #[serde(default)]
    pub encounters: Vec<EncounterId>,
}

/// Serialized, source-agnostic game data. Every list is optional so partial
/// packs (e.g. a user override with three cards) are valid.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DataPack {
    pub format: u32,
    /// Where this pack came from, e.g. `"spire-codex"` or `"user"`.
    pub source: String,
    pub game_version: Option<String>,
    /// RFC 3339 timestamp of when the pack was produced.
    pub fetched_at: Option<String>,
    pub characters: Vec<Character>,
    pub cards: Vec<Card>,
    pub relics: Vec<Relic>,
    pub potions: Vec<Potion>,
    pub enchantments: Vec<Enchantment>,
    pub monsters: Vec<Monster>,
    pub encounters: Vec<Encounter>,
    pub events: Vec<Event>,
    pub acts: Vec<Act>,
}

impl DataPack {
    pub fn summary(&self) -> String {
        format!(
            "{} cards, {} relics, {} potions, {} enchantments, {} monsters, {} encounters, {} events, {} acts, {} characters",
            self.cards.len(),
            self.relics.len(),
            self.potions.len(),
            self.enchantments.len(),
            self.monsters.len(),
            self.encounters.len(),
            self.events.len(),
            self.acts.len(),
            self.characters.len(),
        )
    }
}

/// Metadata about one layer that went into a catalog.
#[derive(Clone, Debug, PartialEq)]
pub struct LayerInfo {
    pub source: String,
    pub game_version: Option<String>,
    pub fetched_at: Option<String>,
    pub summary: String,
}

/// The merged, queryable view of all game data.
#[derive(Clone, Debug, Default)]
pub struct Catalog {
    pub layers: Vec<LayerInfo>,
    pub characters: BTreeMap<CharacterId, Character>,
    pub cards: BTreeMap<CardId, Card>,
    pub relics: BTreeMap<RelicId, Relic>,
    pub potions: BTreeMap<PotionId, Potion>,
    pub enchantments: BTreeMap<EnchantmentId, Enchantment>,
    pub monsters: BTreeMap<MonsterId, Monster>,
    pub encounters: BTreeMap<EncounterId, Encounter>,
    pub events: BTreeMap<EventId, Event>,
    pub acts: BTreeMap<ActId, Act>,
}

fn merge<K: Ord, V>(map: &mut BTreeMap<K, V>, items: Vec<V>, key: impl Fn(&V) -> K) {
    map.extend(items.into_iter().map(|v| (key(&v), v)));
}

impl Catalog {
    /// Builds a catalog from packs in priority order: later packs override
    /// earlier ones entity-by-entity.
    pub fn from_layers(packs: impl IntoIterator<Item = DataPack>) -> Catalog {
        packs.into_iter().fold(Catalog::default(), |mut cat, pack| {
            cat.layers.push(LayerInfo {
                source: pack.source.clone(),
                game_version: pack.game_version.clone(),
                fetched_at: pack.fetched_at.clone(),
                summary: pack.summary(),
            });
            merge(&mut cat.characters, pack.characters, |v| v.id.clone());
            merge(&mut cat.cards, pack.cards, |v| v.id.clone());
            merge(&mut cat.relics, pack.relics, |v| v.id.clone());
            merge(&mut cat.potions, pack.potions, |v| v.id.clone());
            merge(&mut cat.enchantments, pack.enchantments, |v| v.id.clone());
            merge(&mut cat.monsters, pack.monsters, |v| v.id.clone());
            merge(&mut cat.encounters, pack.encounters, |v| v.id.clone());
            merge(&mut cat.events, pack.events, |v| v.id.clone());
            merge(&mut cat.acts, pack.acts, |v| v.id.clone());
            cat
        })
    }

    pub fn card_name(&self, id: &CardId) -> String {
        self.cards.get(id).map_or_else(|| id.to_string(), |c| c.name.clone())
    }

    pub fn relic_name(&self, id: &RelicId) -> String {
        self.relics.get(id).map_or_else(|| id.to_string(), |r| r.name.clone())
    }

    pub fn potion_name(&self, id: &PotionId) -> String {
        self.potions.get(id).map_or_else(|| id.to_string(), |p| p.name.clone())
    }

    pub fn encounter_name(&self, id: &EncounterId) -> String {
        self.encounters.get(id).map_or_else(|| id.to_string(), |e| e.name.clone())
    }

    pub fn event_name(&self, id: &EventId) -> String {
        self.events.get(id).map_or_else(|| id.to_string(), |e| e.name.clone())
    }

    pub fn monster_name(&self, id: &MonsterId) -> String {
        self.monsters.get(id).map_or_else(|| id.to_string(), |m| m.name.clone())
    }

    /// "2× Nibbit + Leaf Slime (M)" for a lineup.
    pub fn lineup_label(&self, lineup: &Lineup) -> String {
        lineup
            .counts()
            .into_iter()
            .map(|(m, n)| if n > 1 { format!("{n}× {}", self.monster_name(m)) } else { self.monster_name(m) })
            .collect::<Vec<_>>()
            .join(" + ")
    }

    pub fn enchantment_name(&self, id: &EnchantmentId) -> String {
        self.enchantments.get(id).map_or_else(|| id.to_string(), |e| e.name.clone())
    }

    /// Acts that can be the `index`-th act (e.g. both act 1 variants).
    pub fn acts_at(&self, index: u8) -> impl Iterator<Item = &Act> {
        self.acts.values().filter(move |a| a.index == index)
    }

    pub fn max_act_index(&self) -> u8 {
        self.acts.values().map(|a| a.index).max().unwrap_or(0)
    }

    /// Encounters of a room type in an act.
    pub fn encounters_in<'a>(&'a self, act: &'a ActId, room: RoomType) -> impl Iterator<Item = &'a Encounter> + 'a {
        self.encounters.values().filter(move |e| e.room == room && e.act.as_ref() == Some(act))
    }

    /// Non-ancient events that can appear in an act: the act's own events
    /// plus shared ones.
    pub fn events_in<'a>(&'a self, act: &'a ActId) -> impl Iterator<Item = &'a Event> + 'a {
        self.events.values().filter(move |e| match e.kind {
            EventKind::Shared => true,
            EventKind::Act => e.acts.is_empty() || e.acts.contains(act),
            EventKind::Ancient => false,
        })
    }

    /// Cards a character can find: their pool plus colorless.
    pub fn character_pool<'a>(&'a self, character: &CharacterId) -> impl Iterator<Item = &'a Card> + 'a {
        let color = character.as_str().to_lowercase();
        self.cards.values().filter(move |c| c.color == color || c.color == "colorless")
    }
}
