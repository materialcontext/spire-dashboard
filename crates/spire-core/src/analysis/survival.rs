//! Is the deck on track to survive a fight?
//!
//! Plays the fight out on averages: each turn the deck deals the damage
//! the simulator expects when attacking and blocks what it expects when
//! defending, split by a fixed share; the enemies deal their expected
//! damage for that turn of their pattern. The player acts first, so a
//! fight won on turn N takes N − 1 enemy turns. Several plans are tried
//! (all-out attack; a quarter, half or three quarters to block; block
//! exactly what's coming; block only the turns that hit hardest) and the
//! one that loses the least HP is kept.
//!
//! What it leaves out, so it reads as a guide, not a forecast:
//! - Enemies die together: damage from one that falls early still counts.
//! - Debuffs on the player (Weak, Frail) and on enemies are not modelled,
//!   except Vulnerable the pattern puts on the player.
//! - Potions, and cards the simulator can't read.

use crate::analysis::sim::DeckSim;
use crate::analysis::threats::{Scaling, lineup_incoming, lineup_plans};
use crate::catalog::{Catalog, Encounter};
use crate::run::RunState;

/// Player turns played out before giving up on a kill.
pub const MAX_TURNS: usize = 20;

/// Lost HP below this share of current HP reads as on track.
const SAFE_SHARE: f32 = 0.6;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    /// Expected to win with HP to spare.
    OnTrack,
    /// Expected to win, but with little left.
    Tight,
    /// Expected to lose more HP than the player has, or not to win at all.
    AtRisk,
}

impl Verdict {
    pub fn label(self) -> &'static str {
        match self {
            Verdict::OnTrack => "On track",
            Verdict::Tight => "Tight",
            Verdict::AtRisk => "At risk",
        }
    }
}

/// How a plan splits each turn between attack and block.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Split {
    /// A fixed share of every turn goes to block.
    Share(f32),
    /// Block what each turn brings (up to a full turn), attack with the rest.
    Need,
    /// Like `Need`, but only on turns bringing at least this much damage;
    /// attack all-out otherwise.
    NeedFrom(u32),
}

/// What the deck is up against: the enemies' expected damage per enemy
/// turn, their combined HP, and a blow after death, if any.
#[derive(Clone, Copy, Debug)]
pub struct Foe<'a> {
    pub incoming: &'a [f32],
    pub hp: u32,
    /// Damage the enemies still deal at the end of the player's turn after
    /// the kill (Waterfall Giant's Steam Eruption), at least this much.
    pub blast: Option<f32>,
}

/// One way to play the fight out.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Line {
    pub split: Split,
    /// The turn a blow after death lands (the turn after the kill).
    pub blast_turn: Option<u32>,
    /// Player turn the last enemy falls on.
    pub kill_turn: Option<u32>,
    /// HP lost by then (or by `MAX_TURNS`).
    pub hp_loss: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Outlook {
    pub verdict: Verdict,
    /// The plan that loses the least HP.
    pub best: Line,
    /// All-out attack, for comparison.
    pub race: Line,
    pub hp: i32,
}

/// Plays out one plan.
fn play(sim: &DeckSim, foe: Foe, split: Split) -> Line {
    let output = |t: usize| sim.turns[t.min(sim.turns.len() - 1)];
    let share_for = |coming: f32, block: f32| {
        let need = if block > 0.0 { (coming / block).min(1.0) } else { 0.0 };
        match split {
            Split::Share(s) => s,
            Split::Need => need,
            Split::NeedFrom(at) if coming >= at as f32 => need,
            Split::NeedFrom(_) => 0.0,
        }
    };
    let mut dealt = 0.0f32;
    let mut lost = 0.0f32;
    for t in 0..MAX_TURNS {
        let out = output(t);
        let (attack, block) = (out.damage.mean, out.block.mean);
        let coming = foe.incoming.get(t).copied().unwrap_or(0.0);
        let share = share_for(coming, block);
        dealt += attack * (1.0 - share);
        if dealt >= foe.hp as f32 {
            // A blow after death: one more hit at the end of the next turn,
            // with that turn's block (there's nothing left to attack, but
            // the plan still splits the same way).
            let blast_turn = foe.blast.map(|floor| {
                let blast = floor.max(foe.incoming.get(t + 1).copied().unwrap_or(0.0));
                let block = output(t + 1).block.mean;
                lost += (blast - block * share_for(blast, block)).max(0.0);
                t as u32 + 2
            });
            return Line { split, blast_turn, kill_turn: Some(t as u32 + 1), hp_loss: lost.round() as u32 };
        }
        lost += (coming - block * share).max(0.0);
    }
    Line { split, blast_turn: None, kill_turn: None, hp_loss: lost.round() as u32 }
}

/// How a fight against `enemy_hp` total HP dealing `incoming` expected
/// damage per enemy turn is likely to go for a deck at `hp`. `None` when
/// the simulator has nothing to go on (no deck).
pub fn outlook(sim: &DeckSim, foe: Foe, hp: i32) -> Option<Outlook> {
    if sim.turns.is_empty() || foe.hp == 0 {
        return None;
    }
    let race = play(sim, foe, Split::Share(0.0));
    // Thresholds for blocking only the big turns: the distinct incoming
    // amounts of the first turns, so each plan blocks one more kind of turn.
    let mut big: Vec<u32> = foe.incoming.iter().take(8).map(|d| d.round() as u32).filter(|d| *d > 0).collect();
    big.sort_unstable();
    big.dedup();
    let splits = [Split::Share(0.25), Split::Share(0.5), Split::Share(0.75), Split::Need]
        .into_iter()
        .chain(big.into_iter().map(Split::NeedFrom));
    let best = splits
        .map(|s| play(sim, foe, s))
        .chain(std::iter::once(race))
        .min_by_key(|l| (l.kill_turn.is_none(), l.hp_loss, l.kill_turn))?;
    let verdict = match best.kill_turn {
        None => Verdict::AtRisk,
        Some(_) if best.hp_loss as f32 >= hp as f32 => Verdict::AtRisk,
        Some(_) if best.hp_loss as f32 >= hp as f32 * SAFE_SHARE => Verdict::Tight,
        Some(_) => Verdict::OnTrack,
    };
    Some(Outlook { verdict, best, race, hp })
}

/// The outlook for the run's deck and HP against an encounter's most
/// common lineup, following each monster's move pattern.
pub fn for_encounter(sim: &DeckSim, enc: &Encounter, catalog: &Catalog, state: &RunState) -> Option<Outlook> {
    let scaling = Scaling::for_run(state, catalog);
    let (plans, enemy_hp) = lineup_plans(enc, catalog, scaling, MAX_TURNS);
    let incoming: Vec<f32> = lineup_incoming(&plans, MAX_TURNS).into_iter().map(|(_, expected, _)| expected).collect();
    // The blast's size grows over the fight and the data gives only its
    // starting value, so it counts as at least that, and at least the turn
    // it would otherwise have had.
    let blast = plans
        .iter()
        .filter_map(|p| catalog.monsters.get(&p.monster)?.death_blow.as_ref())
        .map(|b| b.amount.unwrap_or(0) as f32)
        .reduce(f32::max);
    outlook(sim, Foe { incoming: &incoming, hp: enemy_hp, blast }, state.hp)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::sim::{Spread, TurnOutput};

    fn sim(attack: f32, block: f32) -> DeckSim {
        let s = |m| Spread { mean: m, low: m, high: m };
        DeckSim { turns: vec![TurnOutput { damage: s(attack), block: s(block) }; 5], ..Default::default() }
    }

    fn foe(incoming: &[f32], hp: u32, blast: Option<f32>) -> Foe<'_> {
        Foe { incoming, hp, blast }
    }

    #[test]
    fn a_blow_after_death_costs_one_more_turn() {
        // Killed on turn 3, it still blasts at the end of turn 4: the fight
        // costs what killing it a turn later would, unless it dies sooner.
        let plain = outlook(&sim(20.0, 0.0), foe(&[10.0; 20], 50, None), 60).unwrap();
        let blast = outlook(&sim(20.0, 0.0), foe(&[10.0; 20], 50, Some(25.0)), 60).unwrap();
        assert_eq!(plain.race.hp_loss, 20);
        assert_eq!(blast.race.kill_turn, Some(3));
        assert_eq!(blast.race.blast_turn, Some(4));
        assert_eq!(blast.race.hp_loss, 45);
    }

    #[test]
    fn a_fast_kill_takes_few_enemy_turns() {
        // 20 a turn into 50 HP: dead on turn 3, after two enemy turns of 10.
        let o = outlook(&sim(20.0, 10.0), foe(&[10.0; 20], 50, None), 60).unwrap();
        assert_eq!(o.race.kill_turn, Some(3));
        assert_eq!(o.race.hp_loss, 20);
        assert_eq!(o.verdict, Verdict::OnTrack);
    }

    #[test]
    fn blocking_helps_against_big_hits() {
        let o = outlook(&sim(10.0, 40.0), foe(&[20.0; 20], 100, None), 80).unwrap();
        assert!(o.best.hp_loss < o.race.hp_loss);
    }

    #[test]
    fn hopeless_fights_are_at_risk() {
        let o = outlook(&sim(1.0, 1.0), foe(&[30.0; 20], 500, None), 40).unwrap();
        assert_eq!(o.verdict, Verdict::AtRisk);
    }
}
