//! The node report being filled in, before it is committed to the run.

use spire_core::Catalog;
use spire_core::analysis::floors::expected_room;
use spire_core::ids::*;
use spire_core::run::{Change, FloorRecord, NodeKind, NodeReport, RunEvent, RunState};

#[derive(Clone, Debug)]
pub struct NodeDraft {
    pub kind: NodeKind,
    pub encounter: Option<EncounterId>,
    pub event: Option<EventId>,
    pub hp: i32,
    pub max_hp: i32,
    pub gold: i32,
    pub changes: Vec<Change>,
    pub card_query: String,
    pub relic_query: String,
    pub potion_query: String,
    pub add_upgraded: bool,
    /// The room this floor always has, when the act layout fixes it.
    pub expected: Option<NodeKind>,
    /// At a rest site: what the player did (HEAL, SMITH, OTHER), when chosen.
    pub rest: Option<&'static str>,
}

impl NodeDraft {
    /// A fresh draft whose numbers start at the current state.
    pub fn from_state(s: &RunState) -> Self {
        Self {
            kind: NodeKind::Monster,
            encounter: None,
            event: None,
            hp: s.hp,
            max_hp: s.max_hp,
            gold: s.gold,
            changes: Vec::new(),
            card_query: String::new(),
            relic_query: String::new(),
            potion_query: String::new(),
            add_upgraded: false,
            rest: None,
            expected: None,
        }
    }

    /// A draft for the next floor with the room pre-selected when the act's
    /// layout fixes it, the boss pre-filled on boss floors and the act's
    /// Ancient pre-filled when there is only one.
    pub fn for_next(s: &RunState, catalog: &Catalog) -> Self {
        let mut d = Self::from_state(s);
        d.expected = expected_room(s, catalog);
        if let Some(kind) = d.expected {
            d.kind = kind;
        }
        match d.kind {
            NodeKind::Boss => {
                d.encounter = s.bosses().find(|b| !s.encounters_seen.contains(b)).cloned();
            }
            NodeKind::Ancient => {
                let ancients = s.act.as_ref().and_then(|a| catalog.acts.get(a)).map(|a| a.ancients.as_slice());
                if let Some([only]) = ancients {
                    d.event = Some(only.clone());
                }
            }
            _ => {}
        }
        d
    }

    /// Queued changes plus any edited numbers, in a safe order
    /// (max HP before HP so the clamp uses the new maximum).
    fn all_changes(&self, s: &RunState) -> Vec<Change> {
        let mut out = Vec::new();
        if self.max_hp != s.max_hp {
            out.push(Change::SetMaxHp(self.max_hp));
        }
        if self.hp != s.hp {
            out.push(Change::SetHp(self.hp));
        }
        if self.gold != s.gold {
            out.push(Change::SetGold(self.gold));
        }
        out.extend(self.changes.iter().cloned());
        out
    }

    pub fn is_empty_against(&self, s: &RunState) -> bool {
        self.all_changes(s).is_empty() && self.encounter.is_none() && self.event.is_none()
    }

    pub fn to_node(&self, s: &RunState) -> RunEvent {
        let fight = self.kind.room().is_some() || self.kind == NodeKind::Unknown;
        RunEvent::Node(NodeReport {
            kind: self.kind,
            encounter: self.encounter.clone().filter(|_| fight),
            event: self.event.clone().filter(|_| matches!(self.kind, NodeKind::Unknown | NodeKind::Ancient)),
            changes: self.all_changes(s),
            record: Some(self.record(s)).filter(|r| !r.is_empty()).map(Box::new),
        })
    }

    /// What the floor log knows of the floor's decisions: the rest site's
    /// choice (as picked, else read from the floor: an upgrade is Smith,
    /// HP gained is Heal) and the gold spent at a shop.
    fn record(&self, s: &RunState) -> FloorRecord {
        let changes = self.all_changes(s);
        let rest = match (self.kind, self.rest) {
            (NodeKind::Rest, Some(choice)) => vec![choice.to_string()],
            (NodeKind::Rest, None) if changes.iter().any(|c| matches!(c, Change::UpgradeCard(_))) => {
                vec!["SMITH".into()]
            }
            (NodeKind::Rest, None) if self.hp > s.hp => vec!["HEAL".into()],
            _ => Vec::new(),
        };
        let gold_spent = if self.kind == NodeKind::Shop { (s.gold - self.gold).max(0) } else { 0 };
        FloorRecord { rest, gold_spent, ..FloorRecord::default() }
    }

    pub fn to_adjustment(&self, s: &RunState) -> RunEvent {
        RunEvent::Adjust(self.all_changes(s))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use spire_core::run::{DeckCard, RunState};

    fn state() -> RunState {
        RunState { hp: 50, max_hp: 80, gold: 100, ..RunState::default() }
    }

    #[test]
    fn rest_sites_and_shops_record_their_decisions() {
        let s = state();
        let record = |d: &NodeDraft| match d.to_node(&s) {
            RunEvent::Node(n) => n.record.map(|r| *r).unwrap_or_default(),
            _ => unreachable!(),
        };
        // A rest site with an upgrade and no choice picked: Smith.
        let mut d = NodeDraft::from_state(&s);
        d.kind = NodeKind::Rest;
        d.changes.push(Change::UpgradeCard(DeckCard::new("BASH".into())));
        assert_eq!(record(&d).rest, vec!["SMITH"]);
        // Picked: as picked.
        d.rest = Some("OTHER");
        assert_eq!(record(&d).rest, vec!["OTHER"]);
        // A shop: the gold spent.
        let mut d = NodeDraft::from_state(&s);
        d.kind = NodeKind::Shop;
        d.gold = 40;
        assert_eq!(record(&d).gold_spent, 60);
        // A fight records nothing extra.
        let d = NodeDraft::from_state(&s);
        assert!(matches!(d.to_node(&s), RunEvent::Node(n) if n.record.is_none()));
    }

    #[test]
    fn unchanged_numbers_produce_no_changes() {
        let s = state();
        let d = NodeDraft::from_state(&s);
        assert!(d.is_empty_against(&s));
    }

    #[test]
    fn max_hp_is_set_before_hp() {
        let s = state();
        let mut d = NodeDraft::from_state(&s);
        d.max_hp = 90;
        d.hp = 85;
        let RunEvent::Node(n) = d.to_node(&s) else { panic!("expected a node") };
        assert_eq!(n.changes, vec![Change::SetMaxHp(90), Change::SetHp(85)]);
    }

    #[test]
    fn encounter_only_kept_for_fights() {
        let s = state();
        let mut d = NodeDraft::from_state(&s);
        d.kind = NodeKind::Shop;
        d.encounter = Some("X".into());
        d.changes.push(Change::AddCard(DeckCard::new("C".into())));
        let RunEvent::Node(n) = d.to_node(&s) else { panic!("expected a node") };
        assert_eq!(n.encounter, None);
        assert_eq!(n.changes.len(), 1);
    }
}
