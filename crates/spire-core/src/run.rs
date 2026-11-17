//! Run tracking, event-sourced.
//!
//! The player never has their game observed; they report what happened after
//! each node. Each report is a [`RunEvent`] appended to a log, and the current
//! [`RunState`] is a pure fold over that log. That gives undo for free (drop
//! the last event), makes saved runs replayable, and keeps every state
//! transition in one function: [`apply`].

use serde::{Deserialize, Serialize};

use crate::catalog::{Catalog, RoomType};
use crate::ids::*;

/// A card as it sits in the deck.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DeckCard {
    pub card: CardId,
    #[serde(default)]
    pub upgraded: bool,
    #[serde(default)]
    pub enchantment: Option<EnchantmentId>,
}

impl DeckCard {
    pub fn new(card: CardId) -> Self {
        Self { card, upgraded: false, enchantment: None }
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

/// One atomic change to the run. Card changes match by value (the first
/// equal card in the deck), so they stay valid however the deck is ordered.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Change {
    AddCard(DeckCard),
    RemoveCard(DeckCard),
    UpgradeCard(DeckCard),
    Enchant { card: DeckCard, enchantment: EnchantmentId },
    Transform { from: DeckCard, to: DeckCard },
    AddRelic(RelicId),
    RemoveRelic(RelicId),
    AddPotion(PotionId),
    RemovePotion(PotionId),
    SetHp(i32),
    SetMaxHp(i32),
    SetGold(i32),
    SetFloor(u32),
    SetPotionSlots(u32),
}

/// What the player reports after finishing a node.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NodeReport {
    pub kind: NodeKind,
    #[serde(default)]
    pub encounter: Option<EncounterId>,
    #[serde(default)]
    pub event: Option<EventId>,
    #[serde(default)]
    pub changes: Vec<Change>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunEvent {
    /// Starts a run. Carries the resolved starting kit so replay never needs
    /// the catalog (and survives catalog updates).
    Start {
        character: CharacterId,
        ascension: u32,
        max_hp: i32,
        gold: i32,
        deck: Vec<CardId>,
        relics: Vec<RelicId>,
        /// Absent in runs saved before ascension rules were modelled.
        #[serde(default)]
        potion_slots: Option<u32>,
    },
    EnterAct {
        act: ActId,
        index: u8,
        #[serde(default)]
        boss: Option<EncounterId>,
    },
    SetBoss(EncounterId),
    /// The second boss of a double-boss act (A10, act 3).
    SetSecondBoss(EncounterId),
    Node(NodeReport),
    /// Corrections outside a node.
    Adjust(Vec<Change>),
}

impl RunEvent {
    /// A `Start` event for a character, resolved from the catalog and its
    /// ascension rules (fewer potion slots, a starting curse).
    pub fn start(catalog: &Catalog, character: &CharacterId, ascension: u32) -> Option<RunEvent> {
        let c = catalog.characters.get(character)?;
        let rules = &catalog.rules;
        let ascension = ascension.min(rules.max_ascension);
        let mut deck = c.starting_deck.clone();
        deck.extend(rules.start_curse(ascension).filter(|id| catalog.cards.contains_key(*id)).cloned());
        Some(RunEvent::Start {
            character: c.id.clone(),
            ascension,
            max_hp: c.max_hp as i32,
            gold: c.gold as i32,
            deck,
            relics: c.starting_relics.clone(),
            potion_slots: Some(rules.potion_slots(ascension)),
        })
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RunState {
    pub character: Option<CharacterId>,
    pub ascension: u32,
    pub act: Option<ActId>,
    pub act_index: u8,
    pub boss: Option<EncounterId>,
    /// Second boss of a double-boss act.
    pub second_boss: Option<EncounterId>,
    pub floor: u32,
    pub hp: i32,
    pub max_hp: i32,
    pub gold: i32,
    pub deck: Vec<DeckCard>,
    pub relics: Vec<RelicId>,
    pub potions: Vec<PotionId>,
    pub potion_slots: u32,
    /// Hallway (non-elite) fights finished in the current act.
    pub hallway_fights_in_act: u32,
    pub encounters_seen: Vec<EncounterId>,
    pub events_seen: Vec<EventId>,
    pub nodes: Vec<NodeKind>,
}

impl Default for RunState {
    fn default() -> Self {
        Self {
            character: None,
            ascension: 0,
            act: None,
            act_index: 0,
            boss: None,
            second_boss: None,
            floor: 0,
            hp: 0,
            max_hp: 0,
            gold: 0,
            deck: Vec::new(),
            relics: Vec::new(),
            potions: Vec::new(),
            potion_slots: 3,
            hallway_fights_in_act: 0,
            encounters_seen: Vec::new(),
            events_seen: Vec::new(),
            nodes: Vec::new(),
        }
    }
}

impl RunState {
    /// The known bosses of the current act (one, or two at A10 act 3).
    pub fn bosses(&self) -> impl Iterator<Item = &EncounterId> {
        self.boss.iter().chain(self.second_boss.iter())
    }

    pub fn hp_fraction(&self) -> f32 {
        if self.max_hp <= 0 { 0.0 } else { (self.hp as f32 / self.max_hp as f32).clamp(0.0, 1.0) }
    }

    pub fn started(&self) -> bool {
        self.character.is_some()
    }
}

fn remove_first<T: PartialEq>(items: &mut Vec<T>, item: &T) -> Option<T> {
    items.iter().position(|x| x == item).map(|i| items.remove(i))
}

fn find_mut<'a>(deck: &'a mut [DeckCard], card: &DeckCard) -> Option<&'a mut DeckCard> {
    deck.iter_mut().find(|c| *c == card)
}

/// Applies one change. Unmatched removals are ignored: the log is the
/// player's word, and a typo shouldn't poison the rest of the run.
pub fn apply_change(mut s: RunState, change: &Change) -> RunState {
    match change {
        Change::AddCard(c) => s.deck.push(c.clone()),
        Change::RemoveCard(c) => {
            remove_first(&mut s.deck, c);
        }
        Change::UpgradeCard(c) => {
            if let Some(card) = find_mut(&mut s.deck, c) {
                card.upgraded = true;
            }
        }
        Change::Enchant { card, enchantment } => {
            if let Some(card) = find_mut(&mut s.deck, card) {
                card.enchantment = Some(enchantment.clone());
            }
        }
        Change::Transform { from, to } => {
            if let Some(card) = find_mut(&mut s.deck, from) {
                *card = to.clone();
            }
        }
        Change::AddRelic(r) => s.relics.push(r.clone()),
        Change::RemoveRelic(r) => {
            remove_first(&mut s.relics, r);
        }
        Change::AddPotion(p) => s.potions.push(p.clone()),
        Change::RemovePotion(p) => {
            remove_first(&mut s.potions, p);
        }
        Change::SetHp(hp) => s.hp = (*hp).clamp(0, s.max_hp.max(*hp)),
        Change::SetMaxHp(m) => {
            s.max_hp = (*m).max(1);
            s.hp = s.hp.min(s.max_hp);
        }
        Change::SetGold(g) => s.gold = (*g).max(0),
        Change::SetFloor(f) => s.floor = *f,
        Change::SetPotionSlots(n) => s.potion_slots = *n,
    }
    s
}

/// The single state-transition function.
pub fn apply(s: RunState, event: &RunEvent) -> RunState {
    match event {
        RunEvent::Start { character, ascension, max_hp, gold, deck, relics, potion_slots } => RunState {
            character: Some(character.clone()),
            ascension: *ascension,
            hp: *max_hp,
            max_hp: *max_hp,
            gold: *gold,
            deck: deck.iter().cloned().map(DeckCard::new).collect(),
            relics: relics.clone(),
            potion_slots: potion_slots.unwrap_or(RunState::default().potion_slots),
            ..RunState::default()
        },
        RunEvent::EnterAct { act, index, boss } => RunState {
            act: Some(act.clone()),
            act_index: *index,
            boss: boss.clone(),
            second_boss: None,
            hallway_fights_in_act: 0,
            ..s
        },
        RunEvent::SetBoss(boss) => RunState { boss: Some(boss.clone()), ..s },
        RunEvent::SetSecondBoss(boss) => RunState { second_boss: Some(boss.clone()), ..s },
        RunEvent::Node(report) => {
            let mut s = report.changes.iter().fold(s, apply_change);
            s.floor += 1;
            s.nodes.push(report.kind);
            if let Some(enc) = &report.encounter {
                s.encounters_seen.push(enc.clone());
                if report.kind != NodeKind::Elite && report.kind != NodeKind::Boss {
                    s.hallway_fights_in_act += 1;
                }
            } else if report.kind == NodeKind::Monster {
                s.hallway_fights_in_act += 1;
            }
            if let Some(ev) = &report.event {
                s.events_seen.push(ev.clone());
            }
            s
        }
        RunEvent::Adjust(changes) => changes.iter().fold(s, apply_change),
    }
}

/// A run: its event log. The state is always derived.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Run {
    pub events: Vec<RunEvent>,
}

impl Run {
    pub fn state(&self) -> RunState {
        replay(&self.events)
    }

    /// The state with extra, not-yet-committed events applied (for previews).
    pub fn state_with(&self, pending: &RunEvent) -> RunState {
        apply(self.state(), pending)
    }

    pub fn push(&mut self, event: RunEvent) {
        self.events.push(event);
    }

    pub fn undo(&mut self) -> Option<RunEvent> {
        // Never undo past the start event; start a new run instead.
        if self.events.len() > 1 { self.events.pop() } else { None }
    }
}

pub fn replay(events: &[RunEvent]) -> RunState {
    events.iter().fold(RunState::default(), apply)
}
