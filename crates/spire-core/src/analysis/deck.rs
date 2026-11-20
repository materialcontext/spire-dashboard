//! Deck composition: by card type and by macro role.

use std::collections::BTreeMap;

use crate::analysis::tags::{self, Tag, Tagged};
use crate::catalog::{Card, CardType, Catalog, Rarity};
use crate::playbook::{Playbook, Role};
use crate::run::{DeckCard, RunState};

/// One deck card with its catalog entry and derived roles.
#[derive(Clone, Debug)]
pub struct ClassifiedCard<'a> {
    pub deck: &'a DeckCard,
    pub card: Option<&'a Card>,
    pub roles: Vec<Role>,
    /// Tags with the upgrade applied.
    pub tags: Vec<Tagged>,
    /// What it needs that nothing in the run supplies: it does nothing, so
    /// it's a dead card with no roles.
    pub unmet: Option<String>,
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
                tags: card.map(|c| tags::tags(c, d.upgraded, playbook)).unwrap_or_default(),
                unmet: None,
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
    pub tags: Vec<Tagged>,
    /// What it needs that nothing in the run supplies (a dead card).
    pub unmet: Option<String>,
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
                unmet: c.unmet.clone(),
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
    /// Cards carrying each tag, starter Strikes and Defends included.
    pub by_tag: BTreeMap<Tag, u32>,
    /// Cards that gain Stars (the Regent's resource).
    pub star_makers: u32,
}

impl DeckProfile {
    pub fn role(&self, role: Role) -> u32 {
        self.by_role.get(&role).copied().unwrap_or(0)
    }

    pub fn starter_role(&self, role: Role) -> u32 {
        self.starter_roles.get(&role).copied().unwrap_or(0)
    }

    pub fn tag(&self, tag: Tag) -> u32 {
        self.by_tag.get(&tag).copied().unwrap_or(0)
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
        for t in &c.tags {
            *p.by_tag.entry(t.tag).or_default() += 1;
        }
        p.upgraded += u32::from(c.deck.upgraded);
        p.unknown += u32::from(c.card.is_none());
        p.starter_basics += u32::from(starter);
        p.no_role += u32::from(c.roles.is_empty());
        let eternal = c.card.is_some_and(|k| k.is_eternal());
        p.eternal += u32::from(eternal);
        p.removable_curses += u32::from(c.card_type() == CardType::Curse && !eternal);
        p.dead += u32::from(c.card.is_some_and(|k| k.is_dead_draw()) || c.unmet.is_some());
        p.star_makers += u32::from(c.unmet.is_none() && c.card.is_some_and(|k| makes_stars(k, c.deck.upgraded)));
        p
    })
}

/// Whether a card gains Stars: it has a Star amount and its text says so.
pub fn makes_stars(card: &Card, upgraded: bool) -> bool {
    card.value("stars", upgraded) > 0 && crate::playbook::contains_ci(card.text_at(upgraded), "star")
}

pub fn deck_profile(deck: &[DeckCard], catalog: &Catalog, playbook: &Playbook) -> DeckProfile {
    profile(&classify(deck, catalog, playbook))
}

/// The run's deck classified, with the cards whose need nothing in the run
/// supplies (its relics and character count) marked dead.
pub fn classify_run<'a>(state: &'a RunState, catalog: &'a Catalog, playbook: &Playbook) -> Vec<ClassifiedCard<'a>> {
    let unmet = crate::analysis::requirements::unmet(state, catalog, &playbook.simulation.requirements);
    let mut cards = classify(&state.deck, catalog, playbook);
    for c in &mut cards {
        if let Some(what) = unmet.get(&c.deck.card) {
            c.unmet = Some(what.clone());
            c.roles.clear();
            c.tags.clear();
        }
    }
    cards
}

/// The run's deck profile (see [`classify_run`]).
pub fn run_profile(state: &RunState, catalog: &Catalog, playbook: &Playbook) -> DeckProfile {
    profile(&classify_run(state, catalog, playbook))
}

/// Whether a deck has anything for what a fight demands. How much it has
/// is the survival outlook's question, against the fight itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Coverage {
    Missing,
    Covered,
}

impl Coverage {
    pub fn label(self) -> &'static str {
        match self {
            Coverage::Missing => "missing",
            Coverage::Covered => "covered",
        }
    }
}

/// Whether the deck has any card added for a role.
pub fn coverage(profile: &DeckProfile, role: Role) -> Coverage {
    if profile.role(role) == 0 { Coverage::Missing } else { Coverage::Covered }
}
