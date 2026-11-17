//! The node report being filled in, before it is committed to the run.

use spire_core::ids::*;
use spire_core::run::{Change, NodeKind, NodeReport, RunEvent, RunState};

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
        }
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
        })
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
