//! Deck composition: by card type and by macro role.

use std::collections::BTreeMap;

use crate::catalog::{Card, CardType, Catalog, Rarity};
use crate::playbook::{ActPlan, Playbook, Role};
use crate::run::DeckCard;

/// One deck card with its catalog entry and derived roles.
#[derive(Clone, Debug)]
pub struct ClassifiedCard<'a> {
    pub deck: &'a DeckCard,
    pub card: Option<&'a Card>,
    pub roles: Vec<Role>,
    pub tags: Vec<CardTag>,
}

/// What a card does for acceleration, beside its roles: draws cards, gains energy.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CardTag {
    Draw,
    Energy,
}

impl CardTag {
    pub const ALL: [CardTag; 2] = [CardTag::Draw, CardTag::Energy];

    pub fn label(self) -> &'static str {
        match self {
            CardTag::Draw => "Draw",
            CardTag::Energy => "Energy",
        }
    }

    pub fn describe(self) -> &'static str {
        match self {
            CardTag::Draw => "Draws cards",
            CardTag::Energy => "Gains energy",
        }
    }
}

/// Draw and energy tags, from the card's numbers and its text (powers and
/// "next turn" effects only say it in prose). Upgrades are applied.
pub fn tags(card: &Card, upgraded: bool) -> Vec<CardTag> {
    let effects = crate::mechanics::read(card.text_at(upgraded), &[], true);
    let says = |f: fn(&crate::mechanics::Effect) -> bool| effects.triggers.iter().flat_map(|(_, es)| es).any(f);
    let draw = (card.draw.is_some() && card.value("cards", upgraded) > 0)
        || says(|e| matches!(e, crate::mechanics::Effect::Draw(n) if *n > 0));
    let energy = (card.energy.is_some() && card.value("energy", upgraded) > 0)
        || says(|e| matches!(e, crate::mechanics::Effect::Energy(n) if *n > 0));
    CardTag::ALL.into_iter().filter(|t| if *t == CardTag::Draw { draw } else { energy }).collect()
}

impl ClassifiedCard<'_> {
    pub fn card_type(&self) -> CardType {
        self.card.map_or(CardType::Other, |c| c.card_type)
    }

    pub fn name(&self) -> String {
        let base = self.card.map_or_else(|| self.deck.card.to_string(), |c| c.name.clone());
        if self.deck.upgraded { format!("{base}+") } else { base }
    }
}

pub fn classify<'a>(deck: &'a [DeckCard], catalog: &'a Catalog, playbook: &Playbook) -> Vec<ClassifiedCard<'a>> {
    deck.iter()
        .map(|d| {
            let card = catalog.cards.get(&d.card);
            ClassifiedCard {
                deck: d,
                card,
                roles: card.map(|c| playbook.roles_of(c)).unwrap_or_default(),
                tags: card.map(|c| tags(c, d.upgraded)).unwrap_or_default(),
            }
        })
        .collect()
}

/// Identical deck cards grouped for display: one row per (type, card,
/// upgrade, enchantment), in type then name order.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct DeckRow {
    pub card: DeckCard,
    pub card_type: CardType,
    /// Display name, with `+` when upgraded.
    pub name: String,
    pub count: u32,
    pub roles: Vec<Role>,
    pub known: bool,
    /// Cannot be removed or transformed.
    pub eternal: bool,
    pub tags: Vec<CardTag>,
}

pub fn deck_rows(cards: &[ClassifiedCard<'_>]) -> Vec<DeckRow> {
    let mut rows: BTreeMap<(CardType, String, Option<String>), DeckRow> = BTreeMap::new();
    for c in cards {
        let name = c.name();
        let enchant = c.deck.enchantment.as_ref().map(|e| e.to_string());
        rows.entry((c.card_type(), name.clone(), enchant))
            .or_insert_with(|| DeckRow {
                card: c.deck.clone(),
                card_type: c.card_type(),
                name,
                count: 0,
                roles: c.roles.clone(),
                known: c.card.is_some(),
                eternal: c.card.is_some_and(|k| k.is_eternal()),
                tags: c.tags.clone(),
            })
            .count += 1;
    }
    rows.into_values().collect()
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct DeckProfile {
    pub total: u32,
    pub by_type: BTreeMap<CardType, u32>,
    /// Role counts, excluding starter Strikes/Defends: what the deck has
    /// *added*. Targets and coverage use this.
    pub by_role: BTreeMap<Role, u32>,
    /// Role counts contributed by starter Strikes/Defends.
    pub starter_roles: BTreeMap<Role, u32>,
    pub upgraded: u32,
    /// Cards not found in the catalog.
    pub unknown: u32,
    /// Starter Strikes and Defends still in the deck.
    pub starter_basics: u32,
    /// Cards that fill no role (curses, statuses, unclassified).
    pub no_role: u32,
    /// Curses that can be removed or transformed.
    pub removable_curses: u32,
    /// Eternal cards: permanent, cannot be removed or transformed.
    pub eternal: u32,
    /// Draws that do nothing: curses, statuses, unplayables.
    pub dead: u32,
    /// Cards that draw cards, and cards that gain energy.
    pub draw: u32,
    pub energy: u32,
}

impl DeckProfile {
    pub fn role(&self, role: Role) -> u32 {
        self.by_role.get(&role).copied().unwrap_or(0)
    }

    pub fn starter_role(&self, role: Role) -> u32 {
        self.starter_roles.get(&role).copied().unwrap_or(0)
    }

    pub fn of_type(&self, t: CardType) -> u32 {
        self.by_type.get(&t).copied().unwrap_or(0)
    }

    pub fn upgraded_fraction(&self) -> f32 {
        if self.total == 0 { 0.0 } else { self.upgraded as f32 / self.total as f32 }
    }
}

fn is_starter_basic(card: &Card) -> bool {
    card.rarity == Rarity::Basic && card.tags.iter().any(|t| t == "Strike" || t == "Defend")
}

pub fn profile(cards: &[ClassifiedCard<'_>]) -> DeckProfile {
    cards.iter().fold(DeckProfile::default(), |mut p, c| {
        p.total += 1;
        *p.by_type.entry(c.card_type()).or_default() += 1;
        let starter = c.card.is_some_and(is_starter_basic);
        let roles = if starter { &mut p.starter_roles } else { &mut p.by_role };
        for r in &c.roles {
            *roles.entry(*r).or_default() += 1;
        }
        p.draw += u32::from(c.tags.contains(&CardTag::Draw));
        p.energy += u32::from(c.tags.contains(&CardTag::Energy));
        p.upgraded += u32::from(c.deck.upgraded);
        p.unknown += u32::from(c.card.is_none());
        p.starter_basics += u32::from(starter);
        p.no_role += u32::from(c.roles.is_empty());
        let eternal = c.card.is_some_and(|k| k.is_eternal());
        p.eternal += u32::from(eternal);
        p.removable_curses += u32::from(c.card_type() == CardType::Curse && !eternal);
        p.dead += u32::from(c.card.is_some_and(|k| k.is_dead_draw()));
        p
    })
}

pub fn deck_profile(deck: &[DeckCard], catalog: &Catalog, playbook: &Playbook) -> DeckProfile {
    profile(&classify(deck, catalog, playbook))
}

/// How well a deck covers something a fight or act demands.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Coverage {
    Missing,
    Thin,
    Covered,
}

impl Coverage {
    pub fn label(self) -> &'static str {
        match self {
            Coverage::Missing => "missing",
            Coverage::Thin => "thin",
            Coverage::Covered => "covered",
        }
    }
}

/// Coverage of a role, judged against the act plan's target for it.
/// Without a target, two cards count as covered.
pub fn coverage(profile: &DeckProfile, role: Role, plan: Option<&ActPlan>) -> Coverage {
    let have = profile.role(role);
    let want = plan
        .and_then(|p| p.targets.iter().find(|t| t.role == role).map(|t| t.min))
        .filter(|w| *w > 0)
        .unwrap_or(2);
    match have {
        0 => Coverage::Missing,
        h if h < want => Coverage::Thin,
        _ => Coverage::Covered,
    }
}
