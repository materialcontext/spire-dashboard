//! Strategy knowledge, kept as data.
//!
//! The playbook is everything the dashboard *believes* about good play: how to
//! classify a card's deck role, what each act demands, which event options are
//! opportunities, and the thresholds that turn monster stats into threats.
//! It ships as `data/playbook.json` and can be overridden by the user without
//! touching code. Nothing here is a game fact; facts live in the catalog.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::analysis::tags::{self, Tag, Tagged};
use crate::catalog::{Card, CardType, Target};
use crate::ids::CardId;

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
    /// Damage per hit at or under this is Chip Damage, on a cheap card.
    pub chip_damage: u32,
    /// Costs at or under this count as cheap.
    pub chip_cost: u8,
}

impl Default for TagLines {
    fn default() -> Self {
        TagLines { chip_damage: 4, chip_cost: 1 }
    }
}

/// What an event option can give (or cost) the player.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Opportunity {
    CardRemoval,
    Upgrade,
    Transform,
    Enchant,
    NewCard,
    Relic,
    Potion,
    Gold,
    MaxHp,
    Heal,
    Fight,
    CurseRisk,
    HpCost,
    #[serde(other)]
    Other,
}

impl Opportunity {
    pub fn label(self) -> &'static str {
        match self {
            Opportunity::CardRemoval => "Card removal",
            Opportunity::Upgrade => "Upgrade",
            Opportunity::Transform => "Transform",
            Opportunity::Enchant => "Enchant",
            Opportunity::NewCard => "New card",
            Opportunity::Relic => "Relic",
            Opportunity::Potion => "Potion",
            Opportunity::Gold => "Gold",
            Opportunity::MaxHp => "Max HP",
            Opportunity::Heal => "Heal",
            Opportunity::Fight => "Fight",
            Opportunity::CurseRisk => "Curse risk",
            Opportunity::HpCost => "HP cost",
            Opportunity::Other => "Other",
        }
    }

    /// Costs rather than rewards.
    pub fn is_risk(self) -> bool {
        matches!(self, Opportunity::CurseRisk | Opportunity::HpCost)
    }
}

/// Tags an event option with an opportunity when its text contains any of
/// the given phrases (case-insensitive).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OpportunityRule {
    pub opportunity: Opportunity,
    pub any_text: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RoleTarget {
    pub role: Role,
    /// Minimum number of cards filling this role by the end of the act.
    pub min: u32,
}

/// Per-act strategy: priorities, thresholds and notes.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ActPlan {
    /// 0-based act index this plan applies to.
    pub index: u8,
    /// Hallway fights drawn from the weak pool at the start of the act.
    #[serde(default)]
    pub weak_fights: u32,
    #[serde(default)]
    pub targets: Vec<RoleTarget>,
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

/// Thresholds for reading the run state into needs.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NeedThresholds {
    /// HP fraction below which healing is a priority.
    pub low_hp: f32,
    /// Starter Strikes/Defends at or above which removal is a priority.
    pub starter_cards: u32,
    /// Fraction of upgraded cards below which upgrades are a priority.
    pub low_upgrades: f32,
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
    pub opportunity_rules: Vec<OpportunityRule>,
    pub acts: Vec<ActPlan>,
    pub needs: NeedThresholds,
    #[serde(default)]
    pub simulation: SimAssumptions,
    /// Which roles a relic plays, from what the deck simulator reads in its
    /// text: an effect (or passive) name, when it happens, and the role.
    #[serde(default)]
    pub relic_roles: Vec<RelicRoleRule>,
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
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
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
}

impl Default for SimAssumptions {
    fn default() -> Self {
        SimAssumptions {
            hand_size: 5,
            max_hand: 10,
            turns: 5,
            trials: 400,
            arrow_return_hits: 3,
            max_orb_slots: 10,
            status_cards: 2,
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

    /// The tags that give a role, for "what to look for".
    pub fn tags_for(&self, role: Role) -> Vec<Tag> {
        let mut out: Vec<Tag> = self.tag_roles.iter().filter(|t| t.role == role).map(|t| t.tag).collect();
        out.dedup();
        out
    }

    /// The roles a relic plays, from what the simulator understands in its
    /// text (`crate::mechanics::read`). Relics with nothing understood
    /// play none, so only obvious fits are flagged.
    pub fn relic_roles(&self, m: &crate::mechanics::Mechanics) -> Vec<Role> {
        let facts = m.facts();
        let mut roles: Vec<Role> = self
            .relic_roles
            .iter()
            .filter(|r| facts.iter().any(|(k, rec)| *k == r.effect && r.recurring.is_none_or(|want| want == *rec)))
            .map(|r| r.role)
            .collect();
        roles.sort();
        roles.dedup();
        roles
    }

    pub fn sounds_like_group(&self, encounter_name: &str) -> bool {
        self.multi_enemy_names.iter().any(|w| contains_ci(encounter_name, w))
    }

    /// Opportunities found in a piece of event text.
    pub fn opportunities_in(&self, text: &str) -> Vec<Opportunity> {
        let mut found: Vec<Opportunity> = self
            .opportunity_rules
            .iter()
            .filter(|r| r.any_text.iter().any(|p| contains_ci(text, p)))
            .map(|r| r.opportunity)
            .collect();
        found.sort();
        found.dedup();
        found
    }
}
