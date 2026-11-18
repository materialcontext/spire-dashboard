//! Strategy knowledge, kept as data.
//!
//! The playbook is everything the dashboard *believes* about good play: how to
//! classify a card's deck role, what each act demands, which event options are
//! opportunities, and the thresholds that turn monster stats into threats.
//! It ships as `data/playbook.json` and can be overridden by the user without
//! touching code. Nothing here is a game fact; facts live in the catalog.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::catalog::{Card, CardType, Target};
use crate::ids::CardId;

/// The five macro roles a card can play in a deck.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    FrontloadDamage,
    ScalingDamage,
    FrontloadDefense,
    ScalingDefense,
    Consistency,
}

impl Role {
    pub const ALL: [Role; 5] =
        [Role::FrontloadDamage, Role::ScalingDamage, Role::FrontloadDefense, Role::ScalingDefense, Role::Consistency];

    pub fn label(self) -> &'static str {
        match self {
            Role::FrontloadDamage => "Frontloaded damage",
            Role::ScalingDamage => "Scaling damage",
            Role::FrontloadDefense => "Frontloaded defense",
            Role::ScalingDefense => "Scaling defense",
            Role::Consistency => "Consistency / acceleration",
        }
    }

    pub fn short(self) -> &'static str {
        match self {
            Role::FrontloadDamage => "Front dmg",
            Role::ScalingDamage => "Scale dmg",
            Role::FrontloadDefense => "Front def",
            Role::ScalingDefense => "Scale def",
            Role::Consistency => "Consistency",
        }
    }
}

/// Something a fight can demand of a deck: one of the roles, or area damage.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    Role(Role),
    Aoe,
}

impl Capability {
    pub fn label(self) -> &'static str {
        match self {
            Capability::Role(r) => r.label(),
            Capability::Aoe => "Area damage",
        }
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

/// Where a card's roles come from.
#[derive(Clone, Debug, PartialEq)]
pub enum RoleSource {
    Personal(Vec<Role>),
    Playbook(Vec<Role>),
    /// Each matching rule's role and description.
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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RoleRule {
    pub role: Role,
    pub when: Condition,
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
    #[serde(default)]
    pub needs_aoe: u32,
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
    pub role_rules: Vec<RoleRule>,
    /// Hand-set roles for specific cards; these replace the rules' verdict.
    #[serde(default)]
    pub card_roles: BTreeMap<CardId, Vec<Role>>,
    /// The player's own role overrides, kept apart from the shipped
    /// playbook (stored in their data folder); they win over everything.
    #[serde(skip)]
    pub personal_roles: BTreeMap<CardId, Vec<Role>>,
    pub aoe: Condition,
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
}

impl Default for SimAssumptions {
    fn default() -> Self {
        SimAssumptions { hand_size: 5, max_hand: 10, turns: 5, trials: 400, arrow_return_hits: 3, max_orb_slots: 10 }
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
    /// the rules that matched (with a readable description of each).
    pub fn role_source(&self, card: &Card) -> RoleSource {
        if let Some(roles) = self.personal_roles.get(&card.id) {
            return RoleSource::Personal(roles.clone());
        }
        if let Some(roles) = self.card_roles.get(&card.id) {
            return RoleSource::Playbook(roles.clone());
        }
        let mut matched: Vec<(Role, String)> =
            self.role_rules.iter().filter(|r| r.when.matches(card)).map(|r| (r.role, r.when.describe())).collect();
        matched.sort_by_key(|(r, _)| *r);
        RoleSource::Rules(matched)
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

    pub fn is_aoe(&self, card: &Card) -> bool {
        self.aoe.matches(card)
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
