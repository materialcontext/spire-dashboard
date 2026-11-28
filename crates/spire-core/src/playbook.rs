//! Strategy knowledge, kept as data.
//!
//! The playbook is everything the dashboard *believes* about good play: how to
//! classify a card's deck role, what each act demands, which event relics are
//! crucial to which decks, and the thresholds that turn monster stats into
//! threats.
//! It ships as `data/playbook.json` and can be overridden by the user without
//! touching code. Nothing here is a game fact; facts live in the catalog.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::analysis::tags::{self, Tag, Tagged};
use crate::catalog::{Card, CardType, Target};
use crate::ids::{CardId, CharacterId};

/// The problems a deck solves: what the player sees, on the deck and on
/// each card. Each is measured by a job (`analysis::jobs`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    /// Absorb steady pressure, turn after turn.
    #[serde(alias = "frontload_defense", alias = "scaling_defense")]
    Mitigation,
    /// Take a spike turn.
    Survival,
    /// Do most of the work before the fight turns.
    #[serde(alias = "frontload_damage")]
    FastDamage,
    /// Win a long fight.
    ScalingDamage,
    /// Handle a group.
    Aoe,
    /// Wear down an enemy that has to be hit many times.
    Chip,
}

impl Role {
    pub const ALL: [Role; 6] =
        [Role::Mitigation, Role::Survival, Role::FastDamage, Role::ScalingDamage, Role::Aoe, Role::Chip];

    pub fn label(self) -> &'static str {
        match self {
            Role::Mitigation => "Mitigation",
            Role::Survival => "Survival",
            Role::FastDamage => "Fast Damage",
            Role::ScalingDamage => "Scaling Damage",
            Role::Aoe => "AoE",
            Role::Chip => "Chip",
        }
    }

    pub fn short(self) -> &'static str {
        match self {
            Role::Mitigation => "Mitigation",
            Role::Survival => "Survival",
            Role::FastDamage => "Fast dmg",
            Role::ScalingDamage => "Scaling dmg",
            Role::Aoe => "AoE",
            Role::Chip => "Chip",
        }
    }

    /// What the role is for: "Mitigation, for steady pressure".
    pub fn purpose(self) -> &'static str {
        match self {
            Role::Mitigation => "steady pressure",
            Role::Survival => "spike turns",
            Role::FastDamage => "ending fights early",
            Role::ScalingDamage => "long fights",
            Role::Aoe => "groups",
            Role::Chip => "enemies that need many hits",
        }
    }

    /// A role by its saved name, old names included: frontload damage is
    /// Fast Damage, both defenses are Mitigation, and consistency is no
    /// role any more (`None`).
    pub fn from_name(name: &str) -> Option<Role> {
        Some(match name {
            "mitigation" | "frontload_defense" | "scaling_defense" => Role::Mitigation,
            "survival" => Role::Survival,
            "fast_damage" | "frontload_damage" => Role::FastDamage,
            "scaling_damage" => Role::ScalingDamage,
            "aoe" => Role::Aoe,
            "chip" => Role::Chip,
            _ => return None,
        })
    }
}

/// A composable predicate over a card. Serialized as compact JSON, e.g.
/// `"has_damage"`, `{"applies_power": "Strength"}`,
/// `{"all": [{"card_type": "power"}, {"text_contains": "block"}]}`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Condition {
    HasDamage,
    HasBlock,
    Draws,
    GainsEnergy,
    AppliesPower(String),
    /// Gains at least this much Block.
    BlockAtLeast(u32),
    /// Deals at most this much damage per hit.
    DamageAtMost(u32),
    /// Hits at least this many times.
    HitsAtLeast(u32),
    /// Costs at most this much energy (X-cost and unplayable cards don't).
    CostAtMost(u8),
    /// Deals damage, at most this much in all (per hit × hits).
    TotalDamageAtMost(u32),
    /// Deals damage, at most this much per energy it costs (X: per energy
    /// spent; free cards never match).
    DamagePerEnergyAtMost(u32),
    Rarity(crate::catalog::Rarity),
    /// One of the data's own card tags ("Strike", "Defend").
    SourceTag(String),
    /// Costs Stars.
    StarCost,
    /// Case-insensitive substring match on the card text.
    TextContains(String),
    Keyword(String),
    CardType(CardType),
    Target(Target),
    All(Vec<Condition>),
    Any(Vec<Condition>),
    Not(Box<Condition>),
}

/// ASCII case-insensitive substring test without allocating. Non-ASCII
/// characters must match exactly (game text is effectively ASCII).
pub fn contains_ci(haystack: &str, needle: &str) -> bool {
    let (h, n) = (haystack.as_bytes(), needle.as_bytes());
    n.is_empty() || h.windows(n.len()).any(|w| w.eq_ignore_ascii_case(n))
}

/// Damage per hit × hits, for a card that deals damage by its numbers.
fn total_damage(card: &Card) -> Option<u32> {
    card.damage.filter(|d| *d > 0).map(|d| d * card.hits.unwrap_or(1).max(1))
}

/// Where a card's roles come from.
#[derive(Clone, Debug, PartialEq)]
pub enum RoleSource {
    Personal(Vec<Role>),
    Playbook(Vec<Role>),
    /// Each role with why: the tag that gives it ("Block 8") or the
    /// direct rule that matched.
    Rules(Vec<(Role, String)>),
}

impl Condition {
    /// A short human description, e.g. `deals damage`,
    /// `applies Strength`, `is a Power and mentions "block"`.
    pub fn describe(&self) -> String {
        let join = |cs: &[Condition], word: &str| cs.iter().map(Condition::describe).collect::<Vec<_>>().join(word);
        match self {
            Condition::HasDamage => "deals damage".into(),
            Condition::HasBlock => "gains Block".into(),
            Condition::Draws => "draws cards".into(),
            Condition::GainsEnergy => "gains Energy".into(),
            Condition::AppliesPower(p) => format!("applies {p}"),
            Condition::BlockAtLeast(n) => format!("gains {n}+ Block"),
            Condition::DamageAtMost(n) => format!("deals at most {n} a hit"),
            Condition::HitsAtLeast(n) => format!("hits {n}+ times"),
            Condition::CostAtMost(n) => format!("costs {n} or less"),
            Condition::TotalDamageAtMost(n) => format!("deals {n} damage or less"),
            Condition::DamagePerEnergyAtMost(n) => format!("deals {n} damage or less per energy"),
            Condition::Rarity(r) => format!("is {r:?}"),
            Condition::SourceTag(t) => format!("is a {t}"),
            Condition::StarCost => "costs Stars".into(),
            Condition::TextContains(t) => format!("mentions \"{t}\""),
            Condition::Keyword(k) => format!("has {k}"),
            Condition::CardType(t) => format!("is a {}", t.label()),
            Condition::Target(t) => format!("targets {t:?}"),
            Condition::All(cs) => join(cs, " and "),
            Condition::Any(cs) => format!("({})", join(cs, " or ")),
            Condition::Not(c) => format!("not {}", c.describe()),
        }
    }

    pub fn matches(&self, card: &Card) -> bool {
        match self {
            Condition::HasDamage => card.damage.is_some_and(|d| d > 0),
            Condition::HasBlock => card.block.is_some_and(|b| b > 0),
            Condition::Draws => card.draw.is_some_and(|d| d > 0),
            Condition::GainsEnergy => card.energy.is_some_and(|e| e > 0),
            Condition::AppliesPower(p) => card.powers.iter().any(|x| x.power.eq_ignore_ascii_case(p)),
            Condition::BlockAtLeast(n) => card.block.is_some_and(|b| b >= *n),
            Condition::DamageAtMost(n) => card.damage.is_some_and(|d| d <= *n),
            Condition::HitsAtLeast(n) => card.hits.unwrap_or(1) >= *n,
            Condition::CostAtMost(n) => !card.x_cost && card.cost.is_some_and(|c| c <= *n),
            Condition::TotalDamageAtMost(n) => total_damage(card).is_some_and(|d| d <= *n),
            Condition::DamagePerEnergyAtMost(n) => total_damage(card).is_some_and(|d| match card.cost {
                _ if card.x_cost => d <= *n,
                Some(c) if c > 0 => d <= *n * u32::from(c),
                _ => false,
            }),
            Condition::Rarity(r) => card.rarity == *r,
            Condition::SourceTag(t) => card.tags.iter().any(|x| x.eq_ignore_ascii_case(t)),
            Condition::StarCost => card.star_cost.is_some_and(|n| n > 0),
            Condition::TextContains(s) => contains_ci(&card.text, s),
            Condition::Keyword(k) => card.keywords.iter().any(|x| x.eq_ignore_ascii_case(k)),
            Condition::CardType(t) => card.card_type == *t,
            Condition::Target(t) => card.target == *t,
            Condition::All(cs) => cs.iter().all(|c| c.matches(card)),
            Condition::Any(cs) => cs.iter().any(|c| c.matches(card)),
            Condition::Not(c) => !c.matches(card),
        }
    }
}

/// A role straight from the card, for what the tags can't say (Intangible,
/// Buffer, summons).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RoleRule {
    pub role: Role,
    pub when: Condition,
}

/// A tag for cards whose effect maps to it by judgement ("channelling
/// Lightning is Damage").
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TagRule {
    pub tag: Tag,
    pub when: Condition,
}

/// Cards that don't fill a role, however they'd get it ("starter Strikes
/// fill none"; "6 damage or less isn't Fast Damage").
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RoleExclusion {
    /// `None`: every role.
    #[serde(default)]
    pub role: Option<Role>,
    pub when: Condition,
    /// The rule in plain words, for tooltips ("is a starter Strike");
    /// otherwise the condition's own words.
    #[serde(default)]
    pub why: Option<String>,
}

/// A role a tag gives, from its number up ("Block 9+ is Survival").
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TagRole {
    pub role: Role,
    pub tag: Tag,
    /// The tag's number must be at least this (0: any).
    #[serde(default)]
    pub min: u32,
}

/// Lines the tag reader draws.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct TagLines {
    /// A card's damage (all its hits) at or under this is Chip Damage, not
    /// Damage.
    pub chip_damage: u32,
    /// How often a trigger on an event (a card played, a Star spent, a card
    /// exhausted) fires on a big turn: a Block engine's Scaling (block) is
    /// its Block per firing times this.
    #[serde(default = "default_big_turn")]
    pub big_turn: u32,
    /// How many cards a big turn plays: a trigger on every card played
    /// (Afterimage) fires this often, more than one on a kind of card.
    #[serde(default = "default_big_turn_cards")]
    pub big_turn_cards: u32,
}

fn default_big_turn() -> u32 {
    5
}

fn default_big_turn_cards() -> u32 {
    9
}

impl Default for TagLines {
    fn default() -> Self {
        TagLines { chip_damage: 4, big_turn: default_big_turn(), big_turn_cards: default_big_turn_cards() }
    }
}

/// When a deck needs something: by its cards (`cards`, counted at
/// `at_least`, dead cards left out), by a shortfall (`short_of`), by low HP
/// (`hp_below`). Every test it names must hold.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct When {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cards: Option<Condition>,
    #[serde(default)]
    pub at_least: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub short_of: Option<Shortfall>,
    /// HP under this share of max HP.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hp_below: Option<f32>,
    /// The cards counted, plural ("Powers"); unused without `cards`.
    #[serde(default)]
    pub what: String,
    /// The same, for one ("removable curse"); `what` when unset.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub what_one: String,
}

impl When {
    /// Whether it tests anything at all.
    pub fn is_set(&self) -> bool {
        self.cards.is_some() || self.short_of.is_some() || self.hp_below.is_some()
    }
}

/// When a relic is crucial to a deck.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RelicFit {
    pub relic: crate::ids::RelicId,
    #[serde(flatten)]
    pub when: When,
    /// What the relic does for the deck ("each deals 8 damage to ALL
    /// enemies when played").
    pub why: String,
}

/// When an event option is worth steering into `?` rooms for: the deck
/// needs one of `effects`, by `when`. Options that also do one of `unless`
/// don't answer it (a removal that adds a curse, for a curse).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EventFit {
    /// The warning's title ("Remove a curse").
    pub title: String,
    pub effects: Vec<crate::catalog::EffectKind>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unless: Vec<crate::catalog::EffectKind>,
    #[serde(flatten)]
    pub when: When,
    /// What it does for the deck ("a curse removed is never drawn again").
    pub why: String,
}

/// What a deck can be desperately short of, next to decks that cleared
/// the previous act.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Shortfall {
    /// Share of the deck that gains Block.
    Block,
    /// Max HP gained since the start of the run.
    MaxHp,
}

impl Shortfall {
    pub fn label(self) -> &'static str {
        match self {
            Shortfall::Block => "Block",
            Shortfall::MaxHp => "max HP",
        }
    }
}

/// What the run report counts as a cause (`analysis::report`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ReportLines {
    /// Causes shown, at most.
    pub causes: u32,
    /// Going into the last floor under this share of max HP is a cause.
    pub low_hp: f32,
    /// An act's fights costing this many times the community's average
    /// HP is a cause…
    pub bleed: f32,
    /// …when that's at least this much HP over the average.
    pub bleed_min: f32,
    /// A fight costing this much HP over the community's average is a
    /// turning point.
    pub big_loss: f32,
    /// Upgrading at a rest site under this share of max HP, in the act the
    /// run ended, is a cause.
    pub smith_low_hp: f32,
    /// Ending with at least this much gold is a cause.
    pub unspent_gold: i32,
}

impl Default for ReportLines {
    fn default() -> Self {
        ReportLines {
            causes: 3,
            low_hp: 0.5,
            bleed: 1.5,
            bleed_min: 20.0,
            big_loss: 10.0,
            smith_low_hp: 0.4,
            unspent_gold: 150,
        }
    }
}

/// What card-reward suggestions show (`analysis::lines`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PickLines {
    /// Suggestions shown at most.
    pub max: u32,
    /// A line is within reach when this many of its cards are missing, at
    /// most (and at least one is held).
    pub max_missing: u32,
    /// The missing card is offered in at least this share of runs in the act.
    pub min_offered: f32,
    /// The line beats the boss (or the act's bosses) by this much over the
    /// character's decks, in win rate.
    pub min_gain: f32,
    /// Lines on the same held cards whose gain is within this much of the
    /// best one's join it as options of one suggestion.
    pub similar: f32,
    /// Options one suggestion lists, at most.
    pub max_options: u32,
    /// Roles the deck lacks against the boss that get a filler, at most.
    pub filler_roles: u32,
    /// Cards tried per role in the simulator, best lift first.
    pub filler_candidates: u32,
    /// A filler saves at least this much HP against the boss (or improves
    /// the verdict).
    pub min_hp_saved: u32,
}

impl Default for PickLines {
    fn default() -> Self {
        PickLines {
            max: 3,
            max_missing: 1,
            min_offered: 0.15,
            min_gain: 0.05,
            similar: 0.05,
            max_options: 4,
            filler_roles: 2,
            filler_candidates: 5,
            min_hp_saved: 3,
        }
    }
}

/// When a deck counts as desperately short (`analysis::events`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ShortLines {
    /// Block cards under this share of what 3 in 4 decks had entering the
    /// act (their 25th percentile).
    pub block_share: f32,
    /// Max HP gained this many points under what 3 in 4 decks had gained
    /// entering the act (in act 1: lost since the start).
    pub max_hp_points: f32,
}

impl Default for ShortLines {
    fn default() -> Self {
        ShortLines { block_share: 0.75, max_hp_points: 5.0 }
    }
}

/// How a deck's composition is measured against community decks, and
/// when it warns (`analysis::composition`). Method settings, not targets:
/// the ranges and the risks come from community runs at data refresh.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct CompositionLines {
    /// Community runs at this ascension only.
    pub ascension: u32,
    /// Runs on game builds older than this are left out ("v0.107.0").
    pub min_build: String,
    /// One player counts for at most this many runs.
    pub player_cap: f32,
    /// A character with fewer effective decks for an act uses every
    /// character's.
    pub min_decks: f32,
    /// Confidence of the interval a risk's gap must stay below zero in.
    pub confidence: f32,
    /// Smallest boss win-rate gap (0–1) worth a warning.
    pub min_gap: f32,
    /// Bootstrap resamples for that interval.
    pub resamples: u32,
}

impl Default for CompositionLines {
    fn default() -> Self {
        CompositionLines {
            ascension: 10,
            min_build: "v0.107.0".into(),
            player_cap: 20.0,
            min_decks: 120.0,
            confidence: 0.9,
            min_gap: 0.03,
            resamples: 400,
        }
    }
}

/// Per-act strategy: priorities, thresholds and notes.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ActPlan {
    /// 0-based act index this plan applies to.
    pub index: u8,
    /// Hallway fights drawn from the weak pool at the start of the act.
    #[serde(default)]
    pub weak_fights: u32,
    /// A single enemy turn at or above this much damage is a "big hit".
    pub big_hit: u32,
    /// Total enemy HP at or above this means a long fight (by room).
    pub long_fight_hp: RoomThresholds,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RoomThresholds {
    pub monster: u32,
    pub elite: u32,
    pub boss: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Playbook {
    pub version: u32,
    /// Roles straight from a card, beside those its tags give.
    #[serde(default)]
    pub role_rules: Vec<RoleRule>,
    /// Tags for effects the reader can't map by itself.
    #[serde(default)]
    pub tag_rules: Vec<TagRule>,
    /// The roles each tag gives.
    #[serde(default)]
    pub tag_roles: Vec<TagRole>,
    #[serde(default)]
    pub tag_lines: TagLines,
    /// The most a card may cost and still fill a role read from its tags or
    /// the role rules ("anything costing more than 1 is not Mitigation").
    /// X-cost cards count as over any cap. Hand-set roles aren't capped.
    #[serde(default)]
    pub role_max_cost: BTreeMap<Role, u8>,
    /// Roles cards don't fill, however they'd get them. Hand-set roles
    /// aren't excluded.
    #[serde(default)]
    pub role_exclusions: Vec<RoleExclusion>,
    /// Hand-set tags for specific cards; these replace the reading.
    #[serde(default)]
    pub card_tags: BTreeMap<CardId, Vec<Tag>>,
    /// The player's own tag overrides (stored in their data folder).
    #[serde(skip)]
    pub personal_tags: BTreeMap<CardId, Vec<Tag>>,
    /// Hand-set roles for specific cards; these replace the rules' verdict.
    #[serde(default)]
    pub card_roles: BTreeMap<CardId, Vec<Role>>,
    /// The player's own role overrides, kept apart from the shipped
    /// playbook (stored in their data folder); they win over everything.
    #[serde(skip)]
    pub personal_roles: BTreeMap<CardId, Vec<Role>>,
    /// Words in an encounter name that signal several enemies. Data sources
    /// list each distinct monster once, so `Many Corpse Slugs` looks like one.
    #[serde(default)]
    pub multi_enemy_names: Vec<String>,
    pub acts: Vec<ActPlan>,
    /// When a relic is worth fishing `?` rooms for.
    #[serde(default)]
    pub relic_fits: Vec<RelicFit>,
    /// When an event option is worth steering into `?` rooms for.
    #[serde(default)]
    pub event_fits: Vec<EventFit>,
    /// When a deck counts as desperately short of Block or max HP.
    #[serde(default)]
    pub short: ShortLines,
    /// What card-reward suggestions show (`analysis::lines`).
    #[serde(default)]
    pub picks: PickLines,
    /// What the run report counts as a cause (`analysis::report`).
    #[serde(default)]
    pub report: ReportLines,
    /// How composition is measured and when it warns.
    #[serde(default)]
    pub composition: CompositionLines,
    #[serde(default)]
    pub simulation: SimAssumptions,
    /// Which roles a relic plays, from what the deck simulator reads in its
    /// text: an effect (or passive) name, when it happens, and the role.
    #[serde(default)]
    pub relic_roles: Vec<RelicRoleRule>,
    /// Roles a relic plays when its text says so ("heal" is Mitigation).
    #[serde(default)]
    pub relic_text_roles: Vec<RelicTextRole>,
    /// Where a fight's turns count as spikes or quiet, and as ramping.
    #[serde(default)]
    pub fights: FightThresholds,
    /// When setup or a resource counts as out of balance.
    #[serde(default)]
    pub balance: BalanceLines,
}

/// Lines for the setup and resource warnings (`analysis::balance`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct BalanceLines {
    /// Setup used at under this share of what's made piles up.
    pub pile_up: f32,
    /// Shots or Star cards waiting at or over this share run short.
    pub waiting: f32,
    /// Less made a turn than this isn't worth a warning.
    pub min_gained: f32,
    /// More setup cards than this per card that uses them piles up.
    pub makers_per_user: f32,
}

impl Default for BalanceLines {
    fn default() -> Self {
        BalanceLines { pile_up: 0.5, waiting: 0.2, min_gained: 1.0, makers_per_user: 2.0 }
    }
}

/// How a fight's turns are read against its typical turn (its pressure).
/// Beliefs about what's worth calling out, not game facts.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct FightThresholds {
    /// A turn at or over this multiple of the pressure is a spike.
    pub spike: f32,
    /// A turn at or under this share of the pressure is quiet (a window).
    pub quiet: f32,
    /// Later turns at or over this multiple of the early ones: it ramps.
    pub ramp: f32,
    /// Turns over the players' usual pace that count as clearly slower: in
    /// a fight with a clock, speed then leads, and Fast Damage is a limit.
    pub slower_by: f32,
    /// Hits a turn under which a hit rule (Slippery, Skittish…) holds the
    /// deck back.
    pub chip_hits: f32,
    /// Junk at or over this share of the deck is worth cleaning up.
    pub junk_share: f32,
    /// A spike blocked in full on fewer hands than this holds the deck back
    /// (Survival).
    pub spike_odds: f32,
}

impl Default for FightThresholds {
    fn default() -> Self {
        FightThresholds {
            spike: 1.5,
            quiet: 0.35,
            ramp: 1.3,
            slower_by: 1.5,
            chip_hits: 3.0,
            junk_share: 0.3,
            spike_odds: 0.5,
        }
    }
}

/// "A relic that heals is Mitigation": a role from phrases in a relic's
/// text, for what the simulator doesn't play out.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelicTextRole {
    pub role: Role,
    /// Any of these, case-insensitive.
    pub any_text: Vec<String>,
}

/// "A relic that gains Block once a combat is frontloaded defense."
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelicRoleRule {
    /// An effect or passive name (`Effect::key`, `Passive::key`).
    pub effect: String,
    /// `Some(false)`: only once a combat; `Some(true)`: only repeating;
    /// `None`: either.
    #[serde(default)]
    pub recurring: Option<bool>,
    pub role: Role,
}

/// What the deck simulator assumes where the game data is silent.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SimAssumptions {
    /// Cards drawn at the start of each turn.
    pub hand_size: u32,
    /// Most cards a hand can hold; further draws are lost.
    pub max_hand: u32,
    /// Turns per simulated fight.
    pub turns: u32,
    /// Simulated fights per estimate. More is steadier and slower.
    pub trials: u32,
    /// Further attack hits that bring a shot Arrow back (the Fool), per
    /// community guides; the card data only says "Dislodge".
    pub arrow_return_hits: u32,
    /// Most orb slots a character can have (Spire Codex, orb mechanics).
    pub max_orb_slots: u32,
    /// Unplayable cards a monster's Status move adds to your deck. The game
    /// data marks the move ("Status" intent) but not the card or how many,
    /// so this is an assumption; the survival check uses it.
    pub status_cards: u32,
    /// Cards that do nothing without something the deck must supply
    /// (Growth without Sadness, Dualcast without orbs). Unmet, they're dead
    /// cards: unplayable in the simulator, no roles in the deck.
    pub requirements: Vec<Requirement>,
}

/// What a kind of card needs to do anything at all, and what supplies it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Requirement {
    /// What's needed, in words ("Sadness", "orbs").
    pub what: String,
    /// The cards that need it (cards that also supply it don't).
    pub needs: Condition,
    /// Deck cards that supply it.
    #[serde(default)]
    pub met_by: Option<Condition>,
    /// Relics whose text has any of these supply it.
    #[serde(default)]
    pub relics: Vec<String>,
    /// Characters that have it built in (the Fool's Arrows).
    #[serde(default)]
    pub characters: Vec<CharacterId>,
}

impl Default for SimAssumptions {
    fn default() -> Self {
        SimAssumptions {
            hand_size: 5,
            max_hand: 10,
            turns: 8,
            trials: 400,
            arrow_return_hits: 3,
            max_orb_slots: 10,
            status_cards: 2,
            requirements: Vec::new(),
        }
    }
}

impl Playbook {
    /// The plan for an act index, falling back to the last plan.
    pub fn act(&self, index: u8) -> Option<&ActPlan> {
        self.acts.iter().find(|a| a.index == index).or_else(|| self.acts.iter().max_by_key(|a| a.index))
    }

    /// Roles a card fills: a hand-set override, else the rule table.
    pub fn roles_of(&self, card: &Card) -> Vec<Role> {
        match self.role_source(card) {
            RoleSource::Personal(roles) | RoleSource::Playbook(roles) => roles,
            RoleSource::Rules(matched) => {
                let mut roles: Vec<Role> = matched.into_iter().map(|(r, _)| r).collect();
                roles.dedup();
                roles
            }
        }
    }

    /// Why a card has its roles: your override, the playbook's override, or
    /// its tags and the direct rules (with a readable reason for each).
    pub fn role_source(&self, card: &Card) -> RoleSource {
        if let Some(roles) = self.personal_roles.get(&card.id) {
            return RoleSource::Personal(roles.clone());
        }
        if let Some(roles) = self.card_roles.get(&card.id) {
            return RoleSource::Playbook(roles.clone());
        }
        RoleSource::Rules(self.roles_from(card, &tags::tags(card, false, self)))
    }

    /// The roles `tags` give a card, with the tag (or direct rule) behind
    /// each, in role order.
    pub fn roles_from(&self, card: &Card, tags: &[Tagged]) -> Vec<(Role, String)> {
        let mut matched: Vec<(Role, String)> = Vec::new();
        for tr in &self.tag_roles {
            if let Some(t) = tags.iter().find(|t| t.tag == tr.tag && t.value >= tr.min) {
                matched.push((tr.role, t.label()));
            }
        }
        matched.extend(self.role_rules.iter().filter(|r| r.when.matches(card)).map(|r| (r.role, r.when.describe())));
        matched.retain(|(role, _)| {
            self.role_max_cost.get(role).is_none_or(|max| Condition::CostAtMost(*max).matches(card))
                && !self.role_exclusions.iter().any(|x| x.role.is_none_or(|r| r == *role) && x.when.matches(card))
        });
        matched.sort_by_key(|(r, _)| *r);
        matched.dedup_by(|a, b| a.0 == b.0 && a.1 == b.1);
        matched
    }

    /// How a card gets a role, in words, from the rules as they stand: the
    /// tags and other rules that give it, and what rules it out. For
    /// tooltips on role names.
    pub fn role_criteria(&self, role: Role) -> String {
        // "is a starter Strike" reads as "starter Strike" in a list.
        let terse = |s: String| s.strip_prefix("is a ").or(s.strip_prefix("is ")).map_or(s.clone(), str::to_string);
        let mut lines = vec![format!("{}, for {}.", role.label(), role.purpose())];
        let mut counts: Vec<String> = self
            .tag_roles
            .iter()
            .filter(|t| t.role == role)
            .map(|t| match t.min {
                0 => t.tag.label().to_string(),
                1 => format!("{} that deals damage", t.tag.label()),
                n => format!("{} {n}+", t.tag.label()),
            })
            .collect();
        counts.extend(self.role_rules.iter().filter(|r| r.role == role).map(|r| terse(r.when.describe())));
        if !counts.is_empty() {
            lines.push(format!("Counts: {}.", counts.join(", ")));
        }
        let mut never: Vec<String> = Vec::new();
        if let Some(max) = self.role_max_cost.get(&role) {
            never.push(format!("over {max} energy or X"));
        }
        never.extend(
            self.role_exclusions
                .iter()
                .filter(|x| x.role.is_none_or(|r| r == role))
                .map(|x| terse(x.why.clone().unwrap_or_else(|| x.when.describe()))),
        );
        if !never.is_empty() {
            lines.push(format!("Not: {}.", never.join("; ")));
        }
        lines.join("\n")
    }

    /// How a relic gets a role: what it does, or phrases in its text.
    pub fn relic_criteria(&self, role: Role) -> String {
        let mut lines = vec![format!("{}, for {}.", role.label(), role.purpose())];
        let mut relics: Vec<String> =
            self.relic_roles.iter().filter(|r| r.role == role).map(|r| r.effect.replace('_', " ")).collect();
        relics.dedup();
        relics.extend(
            self.relic_text_roles
                .iter()
                .filter(|r| r.role == role)
                .flat_map(|r| r.any_text.iter().map(|t| format!("\"{t}\""))),
        );
        if !relics.is_empty() {
            lines.push(format!("Relics: {}.", relics.join(", ")));
        }
        lines.join("\n")
    }

    /// The tags that give a role, for "what to look for".
    pub fn tags_for(&self, role: Role) -> Vec<Tag> {
        let mut out: Vec<Tag> = self.tag_roles.iter().filter(|t| t.role == role).map(|t| t.tag).collect();
        out.dedup();
        out
    }

    /// The roles a relic plays: from what the simulator understands in its
    /// text (`crate::mechanics::read`), and from phrases the simulator
    /// doesn't play out (healing, max HP, Thorns). Relics with neither play
    /// none, so only obvious fits are flagged.
    pub fn relic_roles(&self, text: &str, m: &crate::mechanics::Mechanics) -> Vec<Role> {
        let facts = m.facts();
        let mut roles: Vec<Role> = self
            .relic_roles
            .iter()
            .filter(|r| facts.iter().any(|(k, rec)| *k == r.effect && r.recurring.is_none_or(|want| want == *rec)))
            .map(|r| r.role)
            .chain(
                self.relic_text_roles
                    .iter()
                    .filter(|r| r.any_text.iter().any(|p| contains_ci(text, p)))
                    .map(|r| r.role),
            )
            .collect();
        roles.sort();
        roles.dedup();
        roles
    }

    pub fn sounds_like_group(&self, encounter_name: &str) -> bool {
        self.multi_enemy_names.iter().any(|w| contains_ci(encounter_name, w))
    }
}
