//! Tags: what a card does, in a small vocabulary.
//!
//! A tag says what a card does (Block, Damage, Multi-Hit, Weak…), with its
//! number where it has one ("Block 8", "Multi-Hit 3×4"). Roles come from
//! tags, by the playbook's `tag_roles`, so a shortfall in a role points at
//! the tags to look for.
//!
//! Tags are read, in order:
//! 1. from the card's numbers (damage, hits, Block, draw, energy, target)
//!    and the powers it applies;
//! 2. from what the mechanics reader understands in its text (powers and
//!    "next turn" effects only say it in prose);
//! 3. from the playbook's `tag_rules`, for effects that only map to a tag
//!    by judgement (channelling Lightning deals damage; Forge grows Sovereign Blade, so it's
//!    scaling damage).
//!
//! Hand-set tags replace the reading: the playbook's `card_tags`, and over
//! those the player's own (`personal_tags`).

use serde::{Deserialize, Serialize};

use crate::catalog::{Card, Rarity, Target};
use crate::mechanics::{self, Effect};
use crate::playbook::Playbook;

/// What a card does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Tag {
    /// Deals damage to one enemy.
    Damage,
    /// Small, cheap damage (the line is in the playbook).
    ChipDamage,
    /// Hits more than once.
    MultiHit,
    /// Hits every enemy.
    Area,
    /// Gains Block.
    Block,
    /// Cuts what enemies deal.
    Weak,
    /// Raises what enemies take.
    Vulnerable,
    /// Draws cards.
    Draw,
    /// Gains energy.
    Energy,
    /// Damage that grows over the fight (Strength, Focus, Poison, Doom…).
    ScalingDamage,
    /// Block that grows over the fight (Dexterity, Plating…).
    ScalingBlock,
    /// Boosts the next Attack (Vigor, Aim).
    SetupAttack,
    /// Boosts the next Block (Stretch).
    SetupBlock,
    /// Gets rid of junk cards (Status and Curse cards).
    CleanUp,
}

impl Tag {
    pub const ALL: [Tag; 14] = [
        Tag::Damage,
        Tag::ChipDamage,
        Tag::MultiHit,
        Tag::Area,
        Tag::Block,
        Tag::Weak,
        Tag::Vulnerable,
        Tag::Draw,
        Tag::Energy,
        Tag::ScalingDamage,
        Tag::ScalingBlock,
        Tag::SetupAttack,
        Tag::SetupBlock,
        Tag::CleanUp,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Tag::Damage => "Damage",
            Tag::ChipDamage => "Chip Damage",
            Tag::MultiHit => "Multi-Hit",
            Tag::Area => "Area",
            Tag::Block => "Block",
            Tag::Weak => "Weak",
            Tag::Vulnerable => "Vulnerable",
            Tag::Draw => "Draw",
            Tag::Energy => "Energy",
            Tag::ScalingDamage => "Scaling (damage)",
            Tag::ScalingBlock => "Scaling (block)",
            Tag::SetupAttack => "Setup (attack)",
            Tag::SetupBlock => "Setup (block)",
            Tag::CleanUp => "Clean-Up",
        }
    }

    pub fn describe(self) -> &'static str {
        match self {
            Tag::Damage => "Deals damage to one enemy",
            Tag::ChipDamage => "Small, cheap damage",
            Tag::MultiHit => "Hits more than once: each hit counts for Chip, Arrow returns and on-hit rules",
            Tag::Area => "Hits every enemy",
            Tag::Block => "Gains Block",
            Tag::Weak => "Cuts what enemies deal",
            Tag::Vulnerable => "Raises what enemies take",
            Tag::Draw => "Draws cards",
            Tag::Energy => "Gains energy",
            Tag::ScalingDamage => "Damage that grows over the fight (Strength, Focus, Poison, Doom…)",
            Tag::ScalingBlock => "Block that grows over the fight (Dexterity, Plating…)",
            Tag::SetupAttack => "Boosts the next Attack until used (Vigor, Aim)",
            Tag::SetupBlock => "Boosts the next Block until used (Stretch)",
            Tag::CleanUp => "Gets rid of Status and Curse cards",
        }
    }

    /// A tag by its saved name (`"multi_hit"`).
    pub fn from_name(name: &str) -> Option<Tag> {
        Tag::ALL.into_iter().find(|t| t.name() == name)
    }

    /// The saved name, as serde writes it.
    pub fn name(self) -> &'static str {
        match self {
            Tag::Damage => "damage",
            Tag::ChipDamage => "chip_damage",
            Tag::MultiHit => "multi_hit",
            Tag::Area => "area",
            Tag::Block => "block",
            Tag::Weak => "weak",
            Tag::Vulnerable => "vulnerable",
            Tag::Draw => "draw",
            Tag::Energy => "energy",
            Tag::ScalingDamage => "scaling_damage",
            Tag::ScalingBlock => "scaling_block",
            Tag::SetupAttack => "setup_attack",
            Tag::SetupBlock => "setup_block",
            Tag::CleanUp => "clean_up",
        }
    }
}

/// A tag on a card, with its number where it has one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Tagged {
    pub tag: Tag,
    /// Damage per hit, Block, cards drawn, energy, stacks; 0 when none.
    pub value: u32,
    /// Hits, for Multi-Hit; 1 otherwise.
    pub hits: u32,
}

impl Tagged {
    pub fn bare(tag: Tag) -> Tagged {
        Tagged { tag, value: 0, hits: 1 }
    }

    /// "Damage 9", "Multi-Hit 3×4", "Weak".
    pub fn label(&self) -> String {
        match (self.tag, self.value) {
            (_, 0) => self.tag.label().into(),
            (Tag::MultiHit, v) => format!("{} {}×{v}", self.tag.label(), self.hits),
            (t, v) => format!("{} {v}", t.label()),
        }
    }
}

/// Where a card's tags come from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TagSource {
    /// The player's own override.
    Personal,
    /// Hand-set in the playbook.
    Playbook,
    /// Read from the card.
    Read,
}

/// The tags read from a card, before any override. Upgrades are applied.
pub fn read(card: &Card, upgraded: bool, playbook: &Playbook) -> Vec<Tagged> {
    let mut out: Vec<Tagged> = Vec::new();
    let mut add = |tag: Tag, value: i32, hits: u32| {
        let value = value.max(0) as u32;
        match out.iter_mut().find(|t| t.tag == tag) {
            Some(t) => t.value = t.value.max(value),
            None => out.push(Tagged { tag, value, hits }),
        }
    };
    let effects = mechanics::read(card.text_at(upgraded), &[], true);
    let effects: Vec<&Effect> = effects.triggers.iter().flat_map(|(_, es)| es).collect();
    let power = |name: &str| card.powers.iter().find(|p| p.power.eq_ignore_ascii_case(name)).map(|p| p.amount);

    // Damage, by target, and its hits. A curse's or status's damage is to
    // the player.
    let harms_player = matches!(card.rarity, Rarity::Curse | Rarity::Status);
    // Some cards give their base damage only as the calculation's base
    // (Ashen Strike: 6 plus 3 per exhausted card).
    let printed = match card.value("damage", upgraded) {
        0 => card.value("calculationbase", upgraded).max(card.value("calculateddamage", upgraded)),
        n => n,
    };
    let damage = if harms_player { 0 } else { printed };
    let hits = card.value("hitcount", upgraded).max(1) as u32;
    let area = card.target == Target::AllEnemies;
    let said = |all: bool| {
        let n = effects.iter().filter_map(|e| match e {
            Effect::DamageAll(n) if all => Some(*n),
            Effect::Damage(n) if !all => Some(*n),
            _ => None,
        });
        n.max().filter(|_| !harms_player)
    };
    let (said_area, said_one) = (said(true), said(false));
    let per_hit = damage.max(said_one.unwrap_or(0)).max(said_area.unwrap_or(0));
    // Small damage (all hits together, X cards per X) is Chip Damage, not
    // Damage: it wears an enemy down, it doesn't end a fight.
    let chip = per_hit > 0 && per_hit as u32 * hits <= playbook.tag_lines.chip_damage;
    if chip {
        add(Tag::ChipDamage, per_hit * hits as i32, 1);
    }
    if damage > 0 && area || said_area.is_some_and(|n| n > 0) {
        add(Tag::Area, damage.max(said_area.unwrap_or(0)), 1);
    } else if !chip && (damage > 0 || said_one.is_some_and(|n| n > 0)) {
        add(Tag::Damage, damage.max(said_one.unwrap_or(0)), 1);
    }
    if hits > 1 && per_hit > 0 {
        add(Tag::MultiHit, per_hit, hits);
    }

    let block = card.value("block", upgraded);
    let said_block = effects.iter().filter_map(|e| if let Effect::Block(n) = e { Some(*n) } else { None }).max();
    if block > 0 || said_block.is_some_and(|n| n > 0) {
        add(Tag::Block, block.max(said_block.unwrap_or(0)), 1);
    }
    if let Some(n) = power("Weak") {
        add(Tag::Weak, n, 1);
    }
    let vulnerable = effects.iter().find_map(|e| if let Effect::Vulnerable(n) = e { Some(*n) } else { None });
    if let Some(n) = power("Vulnerable").or(vulnerable) {
        add(Tag::Vulnerable, n, 1);
    }
    let draw = card.value("cards", upgraded);
    let said_draw = effects.iter().filter_map(|e| if let Effect::Draw(n) = e { Some(*n) } else { None }).max();
    // The data's card count also marks "Add 3 Shivs" and "3 or more cards":
    // only a card that says "draw" draws.
    let draws = mechanics::card_text(card.text_at(upgraded), &[]).draws;
    if (card.draw.is_some() && draw > 0 && draws) || said_draw.is_some_and(|n| n > 0) {
        add(Tag::Draw, draw.max(said_draw.unwrap_or(0)), 1);
    }
    let energy = card.value("energy", upgraded);
    let said_energy = effects.iter().filter_map(|e| if let Effect::Energy(n) = e { Some(*n) } else { None }).max();
    if (card.energy.is_some() && energy > 0) || said_energy.is_some_and(|n| n > 0) {
        add(Tag::Energy, energy.max(said_energy.unwrap_or(0)), 1);
    }

    // Growth and setup, from the powers it applies and what its text does.
    for e in &effects {
        let (tag, n) = match **e {
            Effect::Strength(n) | Effect::Focus(n) | Effect::Poison(n) | Effect::Doom(n) => (Tag::ScalingDamage, n),
            Effect::Dexterity(n) | Effect::Plating(n) => (Tag::ScalingBlock, n),
            Effect::Vigor(n) | Effect::Aim(n) => (Tag::SetupAttack, n),
            Effect::Stretch(n) => (Tag::SetupBlock, n),
            _ => continue,
        };
        if n > 0 {
            add(tag, n, 1);
        }
    }
    for (names, tag) in [
        (&["Strength", "Focus", "Poison", "Doom"][..], Tag::ScalingDamage),
        (&["Dexterity", "Plating"][..], Tag::ScalingBlock),
        (&["Vigor", "Aim"][..], Tag::SetupAttack),
        (&["Stretch"][..], Tag::SetupBlock),
    ] {
        if let Some(n) = names.iter().filter_map(|p| power(p)).find(|n| *n > 0) {
            add(tag, n, 1);
        }
    }

    for rule in &playbook.tag_rules {
        // A text rule's Damage doesn't undo the small-damage reading.
        if rule.when.matches(card) && !(chip && rule.tag == Tag::Damage) {
            add(rule.tag, 0, 1);
        }
    }
    out.sort();
    out
}

/// A card's tags and where they come from: the player's override, the
/// playbook's, or the reading. Overrides keep the read numbers where the
/// reading has that tag.
pub fn of(card: &Card, upgraded: bool, playbook: &Playbook) -> (Vec<Tagged>, TagSource) {
    let read = read(card, upgraded, playbook);
    let set = |tags: &[Tag]| -> Vec<Tagged> {
        let mut v: Vec<Tagged> =
            tags.iter().map(|t| read.iter().find(|r| r.tag == *t).copied().unwrap_or(Tagged::bare(*t))).collect();
        v.sort();
        v
    };
    if let Some(tags) = playbook.personal_tags.get(&card.id) {
        return (set(tags), TagSource::Personal);
    }
    if let Some(tags) = playbook.card_tags.get(&card.id) {
        return (set(tags), TagSource::Playbook);
    }
    (read, TagSource::Read)
}

/// Just the tags.
pub fn tags(card: &Card, upgraded: bool, playbook: &Playbook) -> Vec<Tagged> {
    of(card, upgraded, playbook).0
}
