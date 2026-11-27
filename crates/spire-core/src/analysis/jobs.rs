//! Jobs: how well a deck does each role, measured by the deck simulator.
//!
//! A role says what problem a card or deck solves; its job says how well
//! the deck actually does it, from the simulator's turn-by-turn spreads
//! (draws, energy and setup included), not from counting cards:
//!
//! | Role | Job |
//! |---|---|
//! | Mitigation | Block on an ordinary turn (the low end: what you can count on) |
//! | Survival | Block on a turn spent defending (the high end) |
//! | Fast Damage | Damage over the first 3 turns |
//! | Scaling Damage | How damage grows: the last simulated turn over the first (turn 8: about a boss fight) |
//! | AoE | Share of damage that hits every enemy |
//! | Chip | Attack hits per turn |
//!
//! Plus a note that isn't a role: how the deck holds up on bad draws (the
//! low end of a turn's damage as a share of its average).
//!
//! What it leaves out: Weak's cut of enemy damage (no enemies in the
//! simulator), and anything the simulator can't read.

use crate::analysis::sim::DeckSim;
use crate::playbook::Role;

/// Turns counted for Fast Damage.
const FAST_TURNS: usize = 3;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Jobs {
    /// Block on an ordinary turn, low end, averaged over the turns.
    pub mitigation: f32,
    /// Block on a defending turn, high end, averaged over the turns.
    pub survival: f32,
    /// Damage over the first turns, playing for damage.
    pub fast_damage: f32,
    /// The best single turn's damage (high end), for windows.
    pub burst: f32,
    /// Last simulated turn's damage over the first's (1.0: flat).
    pub scaling: f32,
    /// The last simulated turn, which `scaling` is read at.
    pub scaling_turn: usize,
    /// Share of damage that hits every enemy (0–1).
    pub aoe: f32,
    /// Attack hits per turn.
    pub chip: f32,
    /// A bad draw's damage as a share of an average one (0–1).
    pub floor: f32,
}

/// Measures the jobs from a simulation. All zero for an empty one.
pub fn measure(sim: &DeckSim) -> Jobs {
    let turns = &sim.turns;
    if turns.is_empty() {
        return Jobs::default();
    }
    let n = turns.len() as f32;
    let mean = |f: fn(&crate::analysis::sim::TurnOutput) -> f32| turns.iter().map(f).sum::<f32>() / n;
    let first = turns[0].damage.mean;
    let last = turns[turns.len() - 1].damage.mean;
    let avg_damage = mean(|t| t.damage.mean);
    Jobs {
        mitigation: mean(|t| t.block.low),
        survival: mean(|t| t.block.high),
        fast_damage: turns.iter().take(FAST_TURNS).map(|t| t.damage.mean).sum(),
        burst: turns.iter().map(|t| t.damage.high).fold(0.0, f32::max),
        scaling: if first > 0.0 { last / first } else { 1.0 },
        scaling_turn: turns.len(),
        aoe: sim.play.area_share,
        chip: sim.play.hits,
        floor: if avg_damage > 0.0 { mean(|t| t.damage.low) / avg_damage } else { 0.0 },
    }
}

impl Jobs {
    /// The role's job as one number (what the plain words describe).
    pub fn value(&self, role: Role) -> f32 {
        match role {
            Role::Mitigation => self.mitigation,
            Role::Survival => self.survival,
            Role::FastDamage => self.fast_damage,
            Role::ScalingDamage => self.scaling,
            Role::Aoe => self.aoe,
            Role::Chip => self.chip,
        }
    }

    /// The role's job in plain words, e.g. "9 Block on an ordinary turn".
    pub fn describe(&self, role: Role) -> String {
        match role {
            Role::Mitigation => format!("{:.0} Block on an ordinary turn", self.mitigation),
            Role::Survival => format!("{:.0} Block on a defending turn", self.survival),
            Role::FastDamage => format!("{:.0} damage over turns 1–{FAST_TURNS}", self.fast_damage),
            Role::ScalingDamage => match self.scaling {
                s if s >= 1.05 => format!("damage grows ×{s:.1} by turn {}", self.scaling_turn),
                s if s <= 0.95 => format!("damage falls to ×{s:.1} by turn {}", self.scaling_turn),
                _ => "damage stays flat".into(),
            },
            Role::Aoe => match self.aoe {
                a if a > 0.0 => format!("{:.0}% of damage hits every enemy", a * 100.0),
                _ => "nothing hits every enemy".into(),
            },
            Role::Chip => format!("{:.1} hits a turn", self.chip),
        }
    }
}

/// What a role's job measures, in words.
pub fn describe_job(role: Role) -> &'static str {
    match role {
        Role::Mitigation => "Block on an ordinary turn",
        Role::Survival => "Block on a turn spent defending",
        Role::FastDamage => "damage over the first 3 turns",
        Role::ScalingDamage => "how much damage grows over a fight",
        Role::Aoe => "share of damage that hits every enemy",
        Role::Chip => "attack hits a turn",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::sim::{PlayStats, Spread, TurnOutput};

    #[test]
    fn measures_each_job_from_the_spreads() {
        let s = |mean: f32, low: f32, high: f32| Spread { mean, low, high };
        let turns = vec![
            TurnOutput { damage: s(10.0, 6.0, 14.0), block: s(8.0, 5.0, 12.0), ..Default::default() },
            TurnOutput { damage: s(12.0, 8.0, 16.0), block: s(8.0, 5.0, 12.0), ..Default::default() },
            TurnOutput { damage: s(14.0, 10.0, 20.0), block: s(8.0, 5.0, 12.0), ..Default::default() },
            TurnOutput { damage: s(16.0, 12.0, 22.0), block: s(8.0, 5.0, 12.0), ..Default::default() },
            TurnOutput { damage: s(20.0, 14.0, 26.0), block: s(8.0, 5.0, 12.0), ..Default::default() },
        ];
        let sim = DeckSim {
            turns,
            play: PlayStats { hits: 3.5, area_share: 0.25, ..Default::default() },
            ..Default::default()
        };
        let j = measure(&sim);
        assert_eq!((j.mitigation, j.survival), (5.0, 12.0));
        assert_eq!(j.fast_damage, 36.0);
        assert_eq!(j.burst, 26.0);
        assert_eq!(j.scaling, 2.0);
        assert_eq!((j.aoe, j.chip), (0.25, 3.5));
        assert!((j.floor - 50.0 / 72.0).abs() < 1e-4);
        assert_eq!(j.describe(Role::ScalingDamage), "damage grows ×2.0 by turn 5");
        assert_eq!(j.describe(Role::Chip), "3.5 hits a turn");
    }

    #[test]
    fn an_empty_simulation_measures_nothing() {
        assert_eq!(measure(&DeckSim::default()), Jobs::default());
    }
}
