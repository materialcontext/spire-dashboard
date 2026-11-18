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
    /// The source's numeric card variables with lowercased keys
    /// (`damage`, `ostydamage`, `stars`, `aimpower`, ...).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub vars: BTreeMap<String, i32>,
    /// What upgrading changes, as data.
    #[serde(default, skip_serializing_if = "Upgrade::is_empty")]
    pub upgrade: Upgrade,
    /// Stars the card costs on top of energy (Regent).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub star_cost: Option<u8>,
    /// Orbs the card channels (Defect).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub channels: Vec<OrbChannel>,
}

/// A readable name for a card variable key (`"ostydamage"` to `"Osty damage"`).
pub fn var_label(key: &str) -> String {
    let named = match key {
        "cards" => "Cards drawn",
        "hitcount" => "Hits",
        "ostydamage" => "Osty damage",
        "starcost" => "Star cost",
        "calculationbase" => "Base amount",
        "calculationextra" => "Extra per stack",
        "extradamage" => "Extra damage",
        "hploss" => "HP loss",
        "maxhp" => "Max HP",
        "poisonperturn" => "Poison per turn",
        "strengthloss" => "Strength loss",
        _ => "",
    };
    if !named.is_empty() {
        return named.into();
    }
    let k = key.strip_suffix("power").unwrap_or(key);
    let mut c = k.chars();
    c.next().map(|f| f.to_ascii_uppercase().to_string() + c.as_str()).unwrap_or_default()
}

/// What an upgrade changes. Deltas are keyed like [`Card::vars`].
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Upgrade {
    /// The new energy cost, if the upgrade changes it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost: Option<u8>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub deltas: BTreeMap<String, i32>,
    /// Keywords the upgrade adds (`Innate`, `Retain`).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub adds: Vec<String>,
    /// Keywords the upgrade removes (`Exhaust`, `Ethereal`).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub removes: Vec<String>,
}

impl Upgrade {
    pub fn is_empty(&self) -> bool {
        self == &Upgrade::default()
    }
}

/// `count` orbs of one kind channeled per play; `x` means once per energy.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OrbChannel {
    pub orb: OrbId,
    pub count: u8,
    #[serde(default)]
    pub x: bool,
}

impl Card {
    /// A numeric variable (lowercase key), with the upgrade's delta applied.
    /// The card's dedicated fields stand in for packs without `vars`.
    pub fn value(&self, key: &str, upgraded: bool) -> i32 {
        let field = |v: Option<u32>| v.map(|n| n as i32);
        let base = self.vars.get(key).copied().or_else(|| match key {
            "damage" => field(self.damage),
            "block" => field(self.block),
            "cards" => field(self.draw),
            "energy" => field(self.energy),
            "hitcount" => field(self.hits),
            _ => None,
        });
        let delta = if upgraded { self.upgrade.deltas.get(key).copied().unwrap_or(0) } else { 0 };
        base.map_or(0, |b| b + delta)
    }

    /// Energy cost, with the upgrade applied. `None` means unplayable.
    pub fn cost_at(&self, upgraded: bool) -> Option<u8> {
        if upgraded && let Some(c) = self.upgrade.cost {
            return Some(c);
        }
        self.cost
    }

    /// Whether the card has a keyword, with the upgrade applied.
    pub fn has_keyword(&self, keyword: &str, upgraded: bool) -> bool {
        let eq = |k: &String| k.eq_ignore_ascii_case(keyword);
        if upgraded && self.upgrade.removes.iter().any(eq) {
            return false;
        }
        self.keywords.iter().any(eq) || upgraded && self.upgrade.adds.iter().any(eq)
    }

    /// Eternal cards cannot be removed or transformed (e.g. Ascender's Bane).
    pub fn is_eternal(&self) -> bool {
        self.keywords.iter().any(|k| k.eq_ignore_ascii_case("Eternal"))
    }

    pub fn is_unplayable(&self) -> bool {
        self.cost.is_none() && !self.x_cost || self.keywords.iter().any(|k| k.eq_ignore_ascii_case("Unplayable"))
    }

    /// A draw that does nothing useful: curses, statuses, unplayables.
    pub fn is_dead_draw(&self) -> bool {
        matches!(self.card_type, CardType::Curse | CardType::Status) || self.is_unplayable()
    }

    pub fn cost_label(&self) -> String {
        self.cost_label_at(false)
    }

    /// Cost as shown on the card, with the upgrade applied.
    pub fn cost_label_at(&self, upgraded: bool) -> String {
        match (self.x_cost, self.cost_at(upgraded)) {
            (true, _) => "X".into(),
            (false, Some(c)) => c.to_string(),
            (false, None) => "-".into(),
        }
    }

    /// Card text, upgraded or not. Upgrades that only change the cost or
    /// keywords keep the base text.
    pub fn text_at(&self, upgraded: bool) -> &str {
        match &self.upgrade_text {
            Some(t) if upgraded && !t.trim().is_empty() => t,
            _ => &self.text,
        }
    }

    /// Keywords, with the upgrade's additions and removals applied.
    pub fn keywords_at(&self, upgraded: bool) -> Vec<&str> {
        let mut out: Vec<&str> = self
            .keywords
            .iter()
            .map(String::as_str)
            .filter(|k| !upgraded || !self.upgrade.removes.iter().any(|r| r.eq_ignore_ascii_case(k)))
            .collect();
        if upgraded {
            for k in &self.upgrade.adds {
                if !out.iter().any(|x| x.eq_ignore_ascii_case(k)) {
                    out.push(k);
                }
            }
        }
        out
    }

    /// What upgrading changes, one plain line each ("Cost 1 to 0", "Damage 8 to 10").
    pub fn upgrade_changes(&self) -> Vec<String> {
        let mut out = Vec::new();
        if let Some(c) = self.upgrade.cost
            && Some(c) != self.cost
        {
            out.push(format!("Cost {} to {c}", self.cost_label()));
        }
        for (key, delta) in &self.upgrade.deltas {
            let base = self.value(key, false);
            if base != 0 || self.vars.contains_key(key) {
                out.push(format!("{} {base} to {}", var_label(key), base + delta));
            } else {
                out.push(format!("{} {delta:+}", var_label(key)));
            }
        }
        out.extend(self.upgrade.adds.iter().map(|k| format!("Gains {k}")));
        out.extend(self.upgrade.removes.iter().map(|k| format!("Loses {k}")));
        out
    }
}

/// What an orb's passive or evoke does.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrbEffect {
    Damage,
    DamageAll,
    Block,
    Energy,
    /// Grows the orb's evoke value (Dark).
    Grow,
    #[default]
    #[serde(other)]
    Other,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Orb {
    pub id: OrbId,
    pub name: String,
    pub text: String,
    pub passive: i32,
    pub passive_effect: OrbEffect,
    pub evoke: i32,
    pub evoke_effect: OrbEffect,
    /// Whether Focus changes this orb's values.
    #[serde(default = "yes")]
    pub focus: bool,
    /// How much the passive drops after each trigger (Glass).
    #[serde(default, skip_serializing_if = "is_zero")]
    pub decay: i32,
}

fn is_zero(n: &i32) -> bool {
    *n == 0
}

fn yes() -> bool {
    true
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
    /// Orb slots at the start of a fight (Defect).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub orb_slots: Option<u32>,
    /// Arrows at the start of a fight (the Fool).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arrow_slots: Option<u32>,
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
    /// Card types it can go on; empty means any.
    #[serde(default)]
    pub card_types: Vec<CardType>,
    /// Free-text restriction from the source (e.g. "Defend cards").
    #[serde(default)]
    pub applicable_to: Option<String>,
    #[serde(default)]
    pub stackable: bool,
}

impl Enchantment {
    /// Whether this enchantment can go on `card`. Reads the type list and the
    /// source's text restriction ("Defend cards", "cards that gain Block",
    /// "Basic Strike or Defend cards"). Curses and statuses never qualify.
    pub fn fits(&self, card: &Card) -> bool {
        use crate::playbook::contains_ci;
        if matches!(card.card_type, CardType::Curse | CardType::Status | CardType::Quest) {
            return false;
        }
        if !self.card_types.is_empty() && !self.card_types.contains(&card.card_type) {
            return false;
        }
        let Some(rule) = &self.applicable_to else { return true };
        if contains_ci(rule, "basic") && card.rarity != Rarity::Basic {
            return false;
        }
        let tags: Vec<&str> = ["Strike", "Defend"].into_iter().filter(|t| contains_ci(rule, t)).collect();
        if !tags.is_empty() && !card.tags.iter().any(|t| tags.contains(&t.as_str())) {
            return false;
        }
        if contains_ci(rule, "gain block") && !(card.block.is_some_and(|b| b > 0) || contains_ci(&card.text, "block")) {
            return false;
        }
        true
    }
}

/// The kind of map node just completed (or about to be entered).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeKind {
    Monster,
    Elite,
    Boss,
    /// A `?` node. What it turned into is recorded by the encounter/event.
    Unknown,
    Shop,
    Rest,
    Treasure,
    Ancient,
}

impl NodeKind {
    pub const ALL: [NodeKind; 8] = [
        NodeKind::Monster,
        NodeKind::Elite,
        NodeKind::Unknown,
        NodeKind::Rest,
        NodeKind::Shop,
        NodeKind::Treasure,
        NodeKind::Boss,
        NodeKind::Ancient,
    ];

    pub fn label(self) -> &'static str {
        match self {
            // StS2's own room names.
            NodeKind::Monster => "Enemy",
            NodeKind::Elite => "Elite",
            NodeKind::Boss => "Boss",
            NodeKind::Unknown => "Event",
            NodeKind::Shop => "Shop",
            NodeKind::Rest => "Rest Site",
            NodeKind::Treasure => "Treasure",
            NodeKind::Ancient => "Ancient",
        }
    }

    pub fn room(self) -> Option<RoomType> {
        match self {
            NodeKind::Monster => Some(RoomType::Monster),
            NodeKind::Elite => Some(RoomType::Elite),
            NodeKind::Boss => Some(RoomType::Boss),
            _ => None,
        }
    }
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

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct MonsterMove {
    /// Stable id, as the attack pattern names it (e.g. `"EYE_LASERS"`).
    #[serde(default)]
    pub id: String,
    pub name: String,
    /// Intent label, e.g. `"Attack + Buff"`.
    pub intent: String,
    #[serde(default)]
    pub damage: Option<u32>,
    #[serde(default)]
    pub hits: Option<u32>,
    #[serde(default)]
    pub block: Option<u32>,
    /// Damage per hit at ascensions with deadlier enemies.
    #[serde(default)]
    pub damage_asc: Option<u32>,
    /// Powers the move applies: Strength to itself, Weak to the player…
    #[serde(default)]
    pub powers: Vec<MovePower>,
}

/// A power a monster move applies, and to whom.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MovePower {
    pub power: String,
    pub target: PowerTarget,
    pub amount: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PowerTarget {
    /// The monster itself.
    Monster,
    Player,
    /// Allies or anything else.
    Other,
}

/// How a monster picks its moves, from the game's move state machine.
///
/// States are moves (with a fixed `next`), random picks (weighted
/// branches, some that can't repeat or have a cap) or conditional picks
/// (branches guarded by a game condition we can't evaluate).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct AttackPattern {
    /// The move the fight opens with, if fixed.
    #[serde(default)]
    pub initial: Option<String>,
    #[serde(default)]
    pub states: Vec<PatternState>,
    /// The data source's own summary, e.g. "Ebb → Eye Lasers → repeat".
    #[serde(default)]
    pub description: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PatternState {
    pub id: String,
    pub kind: StateKind,
    /// For a move state: the move it performs.
    #[serde(default)]
    pub move_id: Option<String>,
    /// For a move state: the state after it.
    #[serde(default)]
    pub next: Option<String>,
    /// For a random or conditional state: the moves it picks from.
    #[serde(default)]
    pub branches: Vec<PatternBranch>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StateKind {
    Move,
    Random,
    Conditional,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PatternBranch {
    pub move_id: String,
    /// Relative weight of a random branch.
    #[serde(default)]
    pub weight: Option<f32>,
    /// A random branch that can't be picked twice in a row.
    #[serde(default)]
    pub no_repeat: bool,
    /// A random branch that can be picked at most this many times in a row.
    #[serde(default)]
    pub max_times: Option<u32>,
    /// The game condition guarding a conditional branch, in plain words.
    #[serde(default)]
    pub condition: Option<String>,
}

impl MonsterMove {
    /// Damage per hit, using the ascension value when `deadly` and known.
    pub fn per_hit_at(&self, deadly: bool) -> Option<u32> {
        if deadly { self.damage_asc.or(self.damage) } else { self.damage }
    }

    /// Strength the move gives the monster itself.
    pub fn self_strength(&self) -> i32 {
        self.powers.iter().filter(|p| p.target == PowerTarget::Monster && p.power == "STRENGTH").map(|p| p.amount).sum()
    }

    pub fn total_damage(&self) -> u32 {
        self.total_damage_at(false)
    }

    /// Damage × hits, using the ascension value when `deadly` and known.
    pub fn total_damage_at(&self, deadly: bool) -> u32 {
        self.per_hit_at(deadly).unwrap_or(0) * self.hits.unwrap_or(1).max(1)
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
    /// HP at ascensions with tougher enemies.
    #[serde(default)]
    pub hp_min_asc: Option<u32>,
    #[serde(default)]
    pub hp_max_asc: Option<u32>,
    /// How it picks moves, if known.
    #[serde(default)]
    pub pattern: Option<AttackPattern>,
    /// It strikes once more after it dies (Waterfall Giant's Steam
    /// Eruption: "When killed, deals damage at the end of your next turn").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub death_blow: Option<DeathBlow>,
}

/// A power that makes a monster deal damage after it's killed.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DeathBlow {
    pub power: String,
    /// The power's text, as the data source words it.
    pub text: String,
    /// The amount the data gives when the monster gains it (its starting
    /// value; moves may add to it without the data saying how much).
    #[serde(default)]
    pub amount: Option<i32>,
}

impl Monster {
    pub fn move_by_id(&self, id: &str) -> Option<&MonsterMove> {
        self.moves.iter().find(|m| m.id == id)
    }

    /// `(min, max)` HP, using ascension values when `tough` and known.
    pub fn hp_at(&self, tough: bool) -> (u32, u32) {
        let (min, max) = if tough {
            (self.hp_min_asc.unwrap_or(self.hp_min), self.hp_max_asc.or(self.hp_min_asc).or(self.hp_max))
        } else {
            (self.hp_min, self.hp_max)
        };
        (min, max.unwrap_or(min).max(min))
    }
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

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct EncounterStats {
    /// Fights the averages are based on.
    pub fights: u32,
    pub avg_damage_taken: f32,
    pub avg_turns: f32,
    /// Runs that ended in this fight.
    #[serde(default)]
    pub deaths: u32,
    /// Deaths by the turn they happened on: index 0 is turn 1. The last
    /// entry also counts every later turn.
    #[serde(default)]
    pub death_turns: Vec<u32>,
    /// Damage the survivors took: median and 90th percentile.
    #[serde(default)]
    pub win_damage: Option<(u32, u32)>,
    /// Median HP players entered with: survivors, then those who died.
    #[serde(default)]
    pub hp_in: Option<(u32, u32)>,
}

impl EncounterStats {
    /// Share of fights that ended the run (0–1).
    pub fn death_rate(&self) -> f32 {
        if self.fights == 0 { 0.0 } else { self.deaths as f32 / self.fights as f32 }
    }

    /// The turns most deaths happen on: every turn with at least
    /// `share` of the deaths, most first.
    pub fn deadly_turns(&self, share: f32) -> Vec<(u32, f32)> {
        let total: u32 = self.death_turns.iter().sum();
        if total == 0 {
            return Vec::new();
        }
        let mut out: Vec<(u32, f32)> = self
            .death_turns
            .iter()
            .enumerate()
            .map(|(i, n)| (i as u32 + 1, *n as f32 / total as f32))
            .filter(|(_, s)| *s >= share)
            .collect();
        out.sort_by(|a, b| b.1.total_cmp(&a.1));
        out
    }
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
    /// Hallway fights drawn from the weak pool at the start of this act,
    /// when measured from community runs.
    #[serde(default)]
    pub weak_fights: Option<u32>,
    /// Usual number of floors (single-boss), measured from community runs.
    #[serde(default)]
    pub length: Option<u32>,
    /// Floors whose room never varies, measured from community runs.
    #[serde(default)]
    pub fixed_rooms: Vec<FixedRoom>,
}

/// A floor whose room type is always the same.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FixedRoom {
    /// 0-based floor within the act; negative counts from the end
    /// (-1 = the last floor).
    pub at: i32,
    pub room: NodeKind,
}

/// One ascension level as the game describes it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AscensionLevel {
    pub level: u32,
    pub name: String,
    pub text: String,
}

/// Mechanical rules the dashboard models. Defaults are Slay the Spire 2
/// (Codex ascension list, v0.113 beta); a data source may override them.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct GameRules {
    pub max_ascension: u32,
    pub base_potion_slots: u32,
    /// Ascension from which runs start with one fewer potion slot.
    pub fewer_potion_slot_from: Option<u32>,
    /// Ascension from which runs start with `start_curse` in the deck.
    pub cursed_start_from: Option<u32>,
    pub start_curse: Option<CardId>,
    /// Ascension from which enemies use their ascension HP.
    pub tougher_enemies_from: Option<u32>,
    /// Ascension from which enemies use their ascension damage.
    pub deadlier_enemies_from: Option<u32>,
    /// Ascension from which the act at `double_boss_act` has two bosses.
    pub double_boss_from: Option<u32>,
    pub double_boss_act: u8,
    /// How much more damage a Vulnerable creature takes from attacks, in
    /// percent, read from the power's description. `None` if unknown.
    #[serde(default)]
    pub vulnerable_pct: Option<u32>,
    /// Ascension from which Ancients heal only `weary_heal_pct` of missing
    /// HP ("Weary Traveler"). Every run starts at 0 HP and is healed by the
    /// first Ancient, so this also lowers starting HP.
    #[serde(default)]
    pub weary_from: Option<u32>,
    #[serde(default = "full_heal")]
    pub weary_heal_pct: u32,
    /// A rest heals this percent of max HP (Spire Codex, campfire options;
    /// matches community runs).
    #[serde(default = "rest_heal")]
    pub rest_heal_pct: u32,
}

fn full_heal() -> u32 {
    100
}

fn rest_heal() -> u32 {
    30
}

impl Default for GameRules {
    fn default() -> Self {
        GameRules {
            max_ascension: 10,
            base_potion_slots: 3,
            fewer_potion_slot_from: Some(4),
            cursed_start_from: Some(5),
            start_curse: Some(CardId::new("ASCENDERS_BANE")),
            tougher_enemies_from: Some(8),
            deadlier_enemies_from: Some(9),
            double_boss_from: Some(10),
            double_boss_act: 2,
            vulnerable_pct: None,
            weary_from: Some(2),
            weary_heal_pct: 80,
            rest_heal_pct: 30,
        }
    }
}

impl GameRules {
    fn at(from: Option<u32>, ascension: u32) -> bool {
        from.is_some_and(|f| ascension >= f)
    }

    /// Percent of missing HP an Ancient heals at this ascension.
    pub fn ancient_heal_pct(&self, ascension: u32) -> u32 {
        if Self::at(self.weary_from, ascension) { self.weary_heal_pct } else { 100 }
    }

    /// HP after an Ancient's heal, rounded down (as in community runs).
    pub fn ancient_heal(&self, hp: i32, max_hp: i32, ascension: u32) -> i32 {
        let missing = (max_hp - hp).max(0);
        hp + missing * self.ancient_heal_pct(ascension) as i32 / 100
    }

    /// Starting HP: runs begin at 0 and Neow heals them.
    pub fn start_hp(&self, max_hp: i32, ascension: u32) -> i32 {
        self.ancient_heal(0, max_hp, ascension)
    }

    pub fn potion_slots(&self, ascension: u32) -> u32 {
        self.base_potion_slots - u32::from(Self::at(self.fewer_potion_slot_from, ascension))
    }

    pub fn start_curse(&self, ascension: u32) -> Option<&CardId> {
        self.start_curse.as_ref().filter(|_| Self::at(self.cursed_start_from, ascension))
    }

    pub fn tough_enemies(&self, ascension: u32) -> bool {
        Self::at(self.tougher_enemies_from, ascension)
    }

    pub fn deadly_enemies(&self, ascension: u32) -> bool {
        Self::at(self.deadlier_enemies_from, ascension)
    }

    pub fn double_boss(&self, ascension: u32, act_index: u8) -> bool {
        act_index == self.double_boss_act && Self::at(self.double_boss_from, ascension)
    }
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
    pub ascensions: Vec<AscensionLevel>,
    pub orbs: Vec<Orb>,
    /// Game rules, when the source knows them; otherwise StS2 defaults.
    pub rules: Option<GameRules>,
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
    pub ascensions: BTreeMap<u32, AscensionLevel>,
    pub orbs: BTreeMap<OrbId, Orb>,
    pub rules: GameRules,
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
            merge(&mut cat.ascensions, pack.ascensions, |v| v.level);
            merge(&mut cat.orbs, pack.orbs, |v| v.id.clone());
            if let Some(rules) = pack.rules {
                cat.rules = rules;
            }
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
