//! Run tracking, event-sourced.
//!
//! The player reports what happened after each node, or (a monitored run)
//! the app reads it from the game's own run save. Each report is a [`RunEvent`] appended to a log, and the current
//! [`RunState`] is a pure fold over that log. That gives undo for free (drop
//! the last event), makes saved runs replayable, and keeps every state
//! transition in one function: [`apply`].

use serde::{Deserialize, Serialize};

use crate::catalog::Catalog;
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

pub use crate::catalog::NodeKind;

/// One atomic change to the run. Card changes match by value (the first
/// equal card in the deck), so they stay valid however the deck is ordered.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Change {
    AddCard(DeckCard),
    RemoveCard(DeckCard),
    UpgradeCard(DeckCard),
    Enchant {
        card: DeckCard,
        enchantment: EnchantmentId,
    },
    Transform {
        from: DeckCard,
        to: DeckCard,
    },
    AddRelic(RelicId),
    RemoveRelic(RelicId),
    AddPotion(PotionId),
    RemovePotion(PotionId),
    SetHp(i32),
    SetMaxHp(i32),
    SetGold(i32),
    SetFloor(u32),
    SetPotionSlots(u32),
    /// Extra potion slots, e.g. from a relic's pickup effect.
    AddPotionSlots(u32),
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

/// How a run ended.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    Victory,
    Died {
        #[serde(default)]
        encounter: Option<EncounterId>,
    },
    Abandoned,
}

impl Outcome {
    pub fn label(&self) -> &'static str {
        match self {
            Outcome::Victory => "Victory",
            Outcome::Died { .. } => "Died",
            Outcome::Abandoned => "Abandoned",
        }
    }
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
        /// Starting HP, when below max (Weary Traveler). Older saves lack it.
        #[serde(default)]
        hp: Option<i32>,
    },
    EnterAct {
        act: ActId,
        index: u8,
        #[serde(default)]
        boss: Option<EncounterId>,
    },
    SetBoss(EncounterId),
    /// Ends the run. `at` is an RFC 3339 timestamp supplied by the app.
    End {
        outcome: Outcome,
        #[serde(default)]
        at: Option<String>,
    },
    /// The second boss of a double-boss act (A10, act 3).
    SetSecondBoss(EncounterId),
    Node(NodeReport),
    /// Corrections outside a node.
    Adjust(Vec<Change>),
    /// The act's Ancient heals this percent of missing HP (rounded down).
    AncientHeal {
        percent: u32,
    },
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
            hp: Some(rules.start_hp(c.max_hp as i32, ascension)).filter(|hp| *hp != c.max_hp as i32),
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
    /// Floors finished in the current act (0 on entering it).
    pub act_floor: u32,
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
    pub outcome: Option<Outcome>,
    /// The current act's Ancient heal has been applied.
    pub ancient_healed: bool,
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
            act_floor: 0,
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
            outcome: None,
            ancient_healed: false,
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

    /// The Ancient heal that's due: in act 2 and later, once the boss is
    /// known and before anything has been logged in the act. Act 1's heal is
    /// the starting HP. Returns the percent to heal.
    pub fn ancient_heal_due(&self, rules: &crate::catalog::GameRules) -> Option<u32> {
        let due = self.act_index > 0 && self.boss.is_some() && !self.ancient_healed && self.act_floor == 0;
        due.then(|| rules.ancient_heal_pct(self.ascension))
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
        Change::AddPotionSlots(n) => s.potion_slots += *n,
    }
    s
}

/// The single state-transition function.
pub fn apply(s: RunState, event: &RunEvent) -> RunState {
    match event {
        RunEvent::Start { character, ascension, max_hp, gold, deck, relics, potion_slots, hp } => RunState {
            character: Some(character.clone()),
            ascension: *ascension,
            hp: hp.unwrap_or(*max_hp),
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
            act_floor: 0,
            hallway_fights_in_act: 0,
            ancient_healed: false,
            ..s
        },
        RunEvent::AncientHeal { percent } => {
            RunState { hp: s.hp + (s.max_hp - s.hp).max(0) * *percent as i32 / 100, ancient_healed: true, ..s }
        }
        RunEvent::SetBoss(boss) => RunState { boss: Some(boss.clone()), ..s },
        RunEvent::SetSecondBoss(boss) => RunState { second_boss: Some(boss.clone()), ..s },
        RunEvent::End { outcome, .. } => RunState { outcome: Some(outcome.clone()), ..s },
        RunEvent::Node(report) => {
            let mut s = report.changes.iter().fold(s, apply_change);
            s.floor += 1;
            s.act_floor += 1;
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
    /// RFC 3339 time the run was started (set by the app).
    #[serde(default)]
    pub started_at: Option<String>,
    /// The player's own notes, added when editing history.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    /// The game run this one follows (monitored runs): its start time and
    /// seed. `None` for runs logged by hand.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

/// One line of run history.
#[derive(Clone, Debug, PartialEq)]
pub struct RunSummary {
    pub character: Option<CharacterId>,
    pub ascension: u32,
    pub floor: u32,
    pub act_index: u8,
    pub outcome: Option<Outcome>,
    pub started_at: Option<String>,
    pub ended_at: Option<String>,
    pub deck_size: usize,
}

impl Run {
    pub fn state(&self) -> RunState {
        replay(&self.events)
    }

    /// The state after the first `n` events (for scrubbing through history).
    pub fn state_at(&self, n: usize) -> RunState {
        replay(&self.events[..n.min(self.events.len())])
    }

    pub fn ended(&self) -> bool {
        self.events.iter().any(|e| matches!(e, RunEvent::End { .. }))
    }

    pub fn summary(&self) -> RunSummary {
        let s = self.state();
        let ended_at = self.events.iter().rev().find_map(|e| match e {
            RunEvent::End { at, .. } => at.clone(),
            _ => None,
        });
        RunSummary {
            character: s.character,
            ascension: s.ascension,
            floor: s.floor,
            act_index: s.act_index,
            outcome: s.outcome,
            started_at: self.started_at.clone(),
            ended_at,
            deck_size: s.deck.len(),
        }
    }

    /// The state with extra, not-yet-committed events applied (for previews).
    pub fn state_with(&self, pending: &RunEvent) -> RunState {
        apply(self.state(), pending)
    }

    pub fn push(&mut self, event: RunEvent) {
        self.events.push(event);
    }

    /// The run as if started at another ascension: the start is resolved
    /// again, so starting HP, potion slots and the ascension curse follow
    /// the new level. Everything logged after it is kept as recorded.
    pub fn with_ascension(&self, catalog: &Catalog, ascension: u32) -> Option<Run> {
        let character = self.events.iter().find_map(|e| match e {
            RunEvent::Start { character, .. } => Some(character.clone()),
            _ => None,
        })?;
        let start = RunEvent::start(catalog, &character, ascension)?;
        let mut run = self.clone();
        let i = run.events.iter().position(|e| matches!(e, RunEvent::Start { .. }))?;
        run.events[i] = start;
        Some(run)
    }

    /// The run with its result set (or cleared), keeping when it ended.
    pub fn with_outcome(&self, outcome: Option<Outcome>) -> Run {
        let mut run = self.clone();
        let at = run.events.iter().find_map(|e| match e {
            RunEvent::End { at, .. } => at.clone(),
            _ => None,
        });
        run.events.retain(|e| !matches!(e, RunEvent::End { .. }));
        if let Some(outcome) = outcome {
            run.events.push(RunEvent::End { outcome, at });
        }
        run
    }

    /// The run without the floor logged at `event` (an index into
    /// `events`). Anything that isn't a floor is left alone.
    pub fn without_floor(&self, event: usize) -> Run {
        let mut run = self.clone();
        if matches!(run.events.get(event), Some(RunEvent::Node(_))) {
            run.events.remove(event);
        }
        run
    }

    /// The run with the fight on the floor at `event` changed.
    pub fn with_floor_encounter(&self, event: usize, encounter: Option<EncounterId>) -> Run {
        let mut run = self.clone();
        if let Some(RunEvent::Node(n)) = run.events.get_mut(event) {
            n.encounter = encounter;
        }
        run
    }

    /// Floors as `(event index, floor number, report)`, in order.
    pub fn floors(&self) -> Vec<(usize, u32, &NodeReport)> {
        self.events
            .iter()
            .enumerate()
            .filter_map(|(i, e)| match e {
                RunEvent::Node(n) => Some((i, n)),
                _ => None,
            })
            .enumerate()
            .map(|(floor, (i, n))| (i, floor as u32 + 1, n))
            .collect()
    }

    pub fn undo(&mut self) -> Option<RunEvent> {
        // Never undo past the start event; start a new run instead.
        if self.events.len() > 1 { self.events.pop() } else { None }
    }
}

pub fn replay(events: &[RunEvent]) -> RunState {
    events.iter().fold(RunState::default(), apply)
}
