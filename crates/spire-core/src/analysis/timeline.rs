//! A run, floor by floor: vitals, deck shape and fight damage over time.
//!
//! Built from the event log alone, so it works the same for the live run and
//! for archived runs in the history browser.

use crate::analysis::deck;
use crate::catalog::Catalog;
use crate::ids::EncounterId;
use crate::playbook::{Playbook, Role};
use crate::run::{NodeKind, Run, RunEvent, RunState, apply};

/// The run right after one logged floor.
#[derive(Clone, Debug, PartialEq)]
pub struct FloorPoint {
    pub floor: u32,
    pub act_index: u8,
    pub kind: NodeKind,
    pub hp: i32,
    pub max_hp: i32,
    pub gold: i32,
    pub deck_size: u32,
    /// Cards added per role (starter Strikes and Defends excluded), in [`Role::ALL`] order.
    pub roles: [u32; Role::ALL.len()],
    /// Curses, statuses and unplayables in the deck.
    pub dead: u32,
    pub encounter: Option<EncounterId>,
    /// HP lost on this floor; negative when the floor healed.
    pub hp_lost: i32,
    /// Community average HP lost in this encounter, when known.
    pub community_lost: Option<f32>,
    /// Index of this floor's event in the run log (for replaying to here).
    pub event_index: usize,
}

impl FloorPoint {
    pub fn role(&self, role: Role) -> u32 {
        Role::ALL.iter().position(|r| *r == role).map_or(0, |i| self.roles[i])
    }

    pub fn is_fight(&self) -> bool {
        matches!(self.kind, NodeKind::Monster | NodeKind::Elite | NodeKind::Boss) || self.encounter.is_some()
    }

    pub fn dead_share(&self) -> f32 {
        if self.deck_size == 0 { 0.0 } else { self.dead as f32 / self.deck_size as f32 }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Timeline {
    pub points: Vec<FloorPoint>,
    /// Elites fought per act index.
    pub elites_by_act: Vec<u32>,
    /// Mean HP lost per fight, yours and the community's, over fights where
    /// both are known.
    pub fight_damage: Option<(f32, f32)>,
}

impl Timeline {
    pub fn fights(&self) -> impl Iterator<Item = &FloorPoint> {
        self.points.iter().filter(|p| p.is_fight())
    }
}

pub fn timeline(run: &Run, catalog: &Catalog, playbook: &Playbook) -> Timeline {
    let mut state = RunState::default();
    let mut points = Vec::new();
    for (i, event) in run.events.iter().enumerate() {
        let before = state.hp;
        state = apply(state, event);
        let RunEvent::Node(node) = event else { continue };
        let profile = deck::run_profile(&state, catalog, playbook);
        points.push(FloorPoint {
            floor: state.floor,
            act_index: state.act_index,
            kind: node.kind,
            hp: state.hp,
            max_hp: state.max_hp,
            gold: state.gold,
            deck_size: profile.total,
            roles: Role::ALL.map(|r| profile.role(r)),
            dead: profile.dead,
            encounter: node.encounter.clone(),
            hp_lost: before - state.hp,
            community_lost: node
                .encounter
                .as_ref()
                .and_then(|e| catalog.encounters.get(e))
                .and_then(|e| e.stats.as_ref())
                .map(|s| s.avg_damage_taken),
            event_index: i + 1,
        });
    }

    let acts = points.iter().map(|p| p.act_index as usize + 1).max().unwrap_or(0);
    let mut elites_by_act = vec![0; acts];
    for p in points.iter().filter(|p| p.kind == NodeKind::Elite) {
        elites_by_act[p.act_index as usize] += 1;
    }
    let compared: Vec<(f32, f32)> =
        points.iter().filter_map(|p| Some((p.hp_lost.max(0) as f32, p.community_lost?))).collect();
    let fight_damage = (!compared.is_empty()).then(|| {
        let n = compared.len() as f32;
        (compared.iter().map(|c| c.0).sum::<f32>() / n, compared.iter().map(|c| c.1).sum::<f32>() / n)
    });
    Timeline { points, elites_by_act, fight_damage }
}
