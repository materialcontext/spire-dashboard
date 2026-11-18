//! A fight's shape: how its pressure is spread over the turns, and what
//! punishes a slow fight.
//!
//! Read from the lineup's move patterns, turn by turn, keeping the spread
//! (least, expected, most) rather than one average:
//! - **Pressure:** the typical turn: the median of the first turns that
//!   deal damage (quiet turns are windows, not part of the pressure).
//! - **Spikes:** turns well over the pressure. Certain when the expected
//!   damage is over the line; possible when only the most it can deal is
//!   (a random pick, or a move set after the order is lost).
//! - **Windows:** quiet turns, well under the pressure (sleep, stun, buffs).
//! - **Ramp:** later turns against early ones.
//! - **Clocks:** summons, a blast after death, Status cards, ramps, revives.
//! - **Gaps:** where the pattern data runs out (no pattern, order lost, no
//!   HP), so the shape is a guess past that point.
//! - **Rules:** powers that change what kind of damage or play works.
//!
//! The lines (what wins the fight) are played out against this.

use crate::analysis::pattern::HORIZON;
use crate::analysis::rules::{self, Rule};
use crate::analysis::threats::{Scaling, lineup_incoming, lineup_statuses, lineup_waves};
use crate::catalog::{Catalog, Encounter};
use crate::playbook::FightThresholds;

/// Turns the typical turn is read from.
const EARLY: usize = 8;

/// One enemy turn's damage: least, expected and most.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Load {
    pub low: u32,
    pub expected: f32,
    pub high: u32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spike {
    /// Enemy turn, 1-based.
    pub turn: usize,
    pub load: Load,
    /// Expected over the line, not only possible.
    pub certain: bool,
}

/// What punishes a slow fight.
#[derive(Clone, Debug, PartialEq)]
pub enum Clock {
    /// Monsters join when others die, or mid-fight.
    Summons,
    /// It deals damage after it dies.
    Blast,
    /// Its moves put Status cards in your deck.
    Statuses,
    /// Its damage grows over the fight.
    Ramp,
    /// It revives into another phase.
    Revives,
}

/// Where the data runs out.
#[derive(Clone, Debug, PartialEq)]
pub enum Gap {
    /// No pattern: any of its moves, any turn.
    NoPattern { monster: String },
    /// The order is lost from this enemy turn: its reachable moves.
    OrderLost { monster: String, from: usize },
    /// No HP in the data (a multi-phase boss).
    NoHp { monster: String },
}

/// The shape in one word.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// About the same every turn.
    Steady,
    /// Up and down, without spikes.
    Alternating,
    /// Big turns well over the rest.
    Spikes,
    /// Quiet first turns, then pressure.
    SlowStart,
    /// Pressure that keeps growing.
    Ramping,
}

impl Kind {
    pub fn label(self) -> &'static str {
        match self {
            Kind::Steady => "Steady",
            Kind::Alternating => "Alternating",
            Kind::Spikes => "Spikes",
            Kind::SlowStart => "Slow start",
            Kind::Ramping => "Ramping",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Shape {
    pub kind: Kind,
    /// The first stage's turns (the starting lineup).
    pub turns: Vec<Load>,
    pub pressure: f32,
    pub spikes: Vec<Spike>,
    /// Quiet turns, 1-based.
    pub windows: Vec<usize>,
    /// Later turns' damage over early turns' (1.0: flat).
    pub ramp: f32,
    pub clocks: Vec<Clock>,
    /// HP of each stage: the lineup, then whoever its deaths summon.
    pub stages: Vec<u32>,
    pub gaps: Vec<Gap>,
    pub rules: Vec<Rule>,
}

/// What the turns alone say: pressure, spikes, windows, ramp and kind.
pub fn read_turns(turns: &[Load], t: &FightThresholds) -> (f32, Vec<Spike>, Vec<usize>, f32, Kind) {
    let early: Vec<f32> = turns.iter().take(EARLY).map(|l| l.expected).collect();
    let hitting: Vec<f32> = early.iter().copied().filter(|d| *d > 0.0).collect();
    let pressure = median(&hitting);
    let spike_line = pressure * t.spike;
    let spikes: Vec<Spike> = turns
        .iter()
        .enumerate()
        .filter(|(_, l)| pressure > 0.0 && l.high as f32 >= spike_line)
        .map(|(i, l)| Spike { turn: i + 1, load: *l, certain: l.expected >= spike_line })
        .collect();
    let windows: Vec<usize> =
        turns.iter().enumerate().filter(|(_, l)| l.expected <= pressure * t.quiet).map(|(i, _)| i + 1).collect();
    // Ramp from the first turn after any quiet opening (sleep isn't a ramp).
    let start = windows.iter().enumerate().take_while(|(i, w)| **w == i + 1).count();
    let rest = &turns[start.min(turns.len())..];
    let half = rest.len() / 2;
    let mean = |ls: &[Load]| ls.iter().map(|l| l.expected).sum::<f32>() / ls.len().max(1) as f32;
    let (first, later) = (mean(&rest[..half]), mean(&rest[half..]));
    let ramp = if first > 0.0 { later / first } else { 1.0 };
    let spread = early.iter().copied().fold(0.0f32, f32::max) - hitting.iter().copied().fold(f32::MAX, f32::min);
    // Quiet turns only at the start are a slow start; quiet turns that come
    // back make the fight alternate.
    let recurring = windows.len() > start;
    let kind = if spikes.iter().any(|s| s.certain) {
        Kind::Spikes
    } else if start > 0 && !recurring {
        Kind::SlowStart
    } else if ramp >= t.ramp {
        Kind::Ramping
    } else if recurring || (pressure > 0.0 && spread / pressure > 0.3) {
        Kind::Alternating
    } else {
        Kind::Steady
    };
    (pressure, spikes, windows, ramp, kind)
}

fn median(xs: &[f32]) -> f32 {
    let mut v = xs.to_vec();
    v.sort_by(f32::total_cmp);
    match v.len() {
        0 => 0.0,
        n if n % 2 == 1 => v[n / 2],
        n => (v[n / 2 - 1] + v[n / 2]) / 2.0,
    }
}

/// The shape of an encounter's most common lineup, at `scaling`.
pub fn of(enc: &Encounter, catalog: &Catalog, scaling: Scaling, t: &FightThresholds) -> Shape {
    let waves = lineup_waves(enc, catalog, scaling, HORIZON);
    let first = waves.first().map(|w| w.plans.as_slice()).unwrap_or(&[]);
    let turns: Vec<Load> =
        lineup_incoming(first, HORIZON).into_iter().map(|(low, expected, high)| Load { low, expected, high }).collect();
    let (pressure, spikes, windows, ramp, kind) = read_turns(&turns, t);

    let monsters: Vec<_> =
        waves.iter().flat_map(|w| &w.plans).filter_map(|p| catalog.monsters.get(&p.monster)).collect();
    let mut rules: Vec<Rule> = Vec::new();
    for m in &monsters {
        for r in rules::of(m) {
            if !rules.iter().any(|x| x.monster == r.monster && x.power.power == r.power.power) {
                rules.push(r);
            }
        }
    }
    let mut clocks = Vec::new();
    if waves.len() > 1 || !enc.spawns.is_empty() {
        clocks.push(Clock::Summons);
    }
    if monsters.iter().any(|m| m.death_blow.is_some()) {
        clocks.push(Clock::Blast);
    }
    if waves.iter().any(|w| lineup_statuses(&w.plans, HORIZON).iter().any(|n| *n > 0.0)) {
        clocks.push(Clock::Statuses);
    }
    if ramp >= t.ramp || rules.iter().any(|r| r.kind == rules::Kind::Ramps) {
        clocks.push(Clock::Ramp);
    }
    if rules.iter().any(|r| r.kind == rules::Kind::Revives) {
        clocks.push(Clock::Revives);
    }

    let mut gaps = Vec::new();
    for w in &waves {
        for p in &w.plans {
            let monster = p.name.clone();
            if p.plan.unknown {
                gaps.push(Gap::NoPattern { monster: monster.clone() });
            } else if let Some(from) = p.plan.order_unknown_from {
                gaps.push(Gap::OrderLost { monster: monster.clone(), from });
            }
            if catalog.monsters.get(&p.monster).is_some_and(|m| m.hp_min == 0 && m.hp_max.unwrap_or(0) == 0) {
                gaps.push(Gap::NoHp { monster });
            }
        }
    }
    gaps.dedup();

    Shape {
        kind,
        turns,
        pressure,
        spikes,
        windows,
        ramp,
        clocks,
        stages: waves.iter().map(|w| w.hp).collect(),
        gaps,
        rules,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn loads(xs: &[(f32, u32)]) -> Vec<Load> {
        xs.iter().map(|&(e, h)| Load { low: e as u32, expected: e, high: h }).collect()
    }

    fn read(xs: &[f32]) -> (f32, Vec<Spike>, Vec<usize>, f32, Kind) {
        let l: Vec<(f32, u32)> = xs.iter().map(|&x| (x, x as u32)).collect();
        read_turns(&loads(&l), &FightThresholds::default())
    }

    #[test]
    fn reads_the_shapes_fights_have() {
        // Phantasmal Gardeners: 15 a turn.
        assert_eq!(read(&[15.0; 12]).4, Kind::Steady);
        // Terror Eel: 16 and 9 in turn.
        assert_eq!(read(&[16.0, 9.0, 16.0, 9.0, 16.0, 9.0, 16.0, 9.0, 16.0, 9.0, 16.0, 9.0]).4, Kind::Alternating);
        // Bygone Effigy: asleep for two turns, then 23.
        let (pressure, _, windows, ramp, kind) =
            read(&[0.0, 0.0, 23.0, 23.0, 23.0, 23.0, 23.0, 23.0, 23.0, 23.0, 23.0, 23.0]);
        assert_eq!((pressure, kind, ramp), (23.0, Kind::SlowStart, 1.0));
        assert_eq!(windows, vec![1, 2]);
        // Phrog Parasite: Infect, then Lash for 16. Its 16s are its pressure,
        // not spikes; the Infect turns are quiet.
        let (pressure, spikes, windows, _, kind) =
            read(&[0.0, 16.0, 0.0, 16.0, 0.0, 16.0, 0.0, 16.0, 0.0, 16.0, 0.0, 16.0]);
        assert_eq!((pressure, spikes.len(), windows.len(), kind), (16.0, 0, 6, Kind::Alternating));
        // Knight Gang: 58 every third turn.
        let (_, spikes, _, _, kind) = read(&[21.0, 26.0, 33.0, 23.0, 58.0, 33.0, 23.0, 58.0, 33.0, 23.0, 58.0, 33.0]);
        assert_eq!(kind, Kind::Spikes);
        assert_eq!(spikes.iter().map(|s| s.turn).collect::<Vec<_>>(), vec![5, 8, 11]);
        // Vantom: 26 then a quiet turn, every fourth.
        let (_, spikes, windows, _, _) = read(&[7.0, 12.0, 26.0, 0.0, 7.0, 12.0, 26.0, 0.0, 7.0, 12.0, 26.0, 0.0]);
        assert_eq!((spikes.len(), windows), (3, vec![4, 8, 12]));
    }

    #[test]
    fn a_possible_spike_is_not_a_certain_one() {
        // Entomancer past its first turn: 13 expected, up to 21.
        let mut l = vec![(21.0, 21)];
        l.extend([(13.0, 21); 11]);
        let (_, spikes, _, _, kind) = read_turns(&loads(&l), &FightThresholds::default());
        assert!(spikes.iter().skip(1).all(|s| !s.certain));
        assert!(spikes.iter().any(|s| s.certain));
        assert_eq!(kind, Kind::Spikes);
    }
}
