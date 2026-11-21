//! Benchmarks: what community decks that beat each act's boss had going
//! into it, per character and act.
//!
//! At data refresh, each community run's deck is rebuilt going into every
//! boss it beat, tagged and simulated ([`measure`]). The catalog keeps the
//! weighted quartiles per character and act, and for all characters
//! together (the fallback when a character has too few effective decks).
//! `analysis::composition` places the run's deck on them.
//!
//! The data layer picks the runs (one ascension, recent builds, no repeats)
//! and weighs them (each player capped). Decks are counted with the bundled
//! playbook's tags, and their jobs include their relics. Starter Strikes
//! and Defends are counted in the shares (they're in the deck), and shown
//! apart as their own share.

use std::collections::BTreeMap;

use crate::analysis::composition;
use crate::analysis::deck::run_profile;
use crate::analysis::sim::{DeckSim, simulate};
use crate::analysis::tags::Tag;
use crate::catalog::{Benchmark, Catalog, Measure, Quartiles};
use crate::ids::CharacterId;
use crate::playbook::{Playbook, Role};
use crate::run::RunState;

/// The all-characters benchmark needs at least this many effective decks.
pub const MIN_POOLED: f32 = 30.0;

/// One community deck at a point of its run, and its weight.
#[derive(Clone, Debug)]
pub struct Sample {
    pub character: CharacterId,
    /// 0-based act it's in.
    pub act: u8,
    pub state: RunState,
    /// Its run's weight (each player capped); 1 when unweighted.
    pub weight: f32,
}

/// One deck's measures.
pub struct Measured {
    pub values: BTreeMap<Measure, f32>,
    max_hp_gained: f32,
    starter: f32,
    tags: BTreeMap<Tag, f32>,
}

/// A deck's measures: its composition values, tag shares, starter share and
/// max HP gained.
pub fn measured(state: &RunState, catalog: &Catalog, playbook: &Playbook) -> Measured {
    measured_with(state, &simulate(state, catalog, &playbook.simulation), catalog, playbook)
}

/// [`measured`] with the deck already simulated.
pub fn measured_with(state: &RunState, sim: &DeckSim, catalog: &Catalog, playbook: &Playbook) -> Measured {
    let p = run_profile(state, catalog, playbook);
    let n = p.total.max(1) as f32;
    Measured {
        values: composition::values(sim, &p),
        max_hp_gained: (state.max_hp - state.start_max_hp) as f32,
        starter: p.starter_basics as f32 / n,
        tags: Tag::ALL.iter().map(|t| (*t, p.tag(*t) as f32 / n)).collect(),
    }
}

/// How many runs' worth the decks are once each player is capped: the sum
/// of their weights. (Kish's effective size would miss it: one player's
/// 40 runs at a tenth each are still 40 equal weights.)
fn effective(weights: impl Iterator<Item = f32>) -> f32 {
    weights.sum()
}

fn benchmark(character: Option<CharacterId>, act: u8, decks: &[(&Measured, f32)]) -> Benchmark {
    let q = |f: &dyn Fn(&Measured) -> f32| Quartiles::weighted(decks.iter().map(|(m, w)| (f(m), *w)).collect());
    let value = |m: Measure| move |x: &Measured| x.values.get(&m).copied().unwrap_or(0.0);
    Benchmark {
        character,
        act,
        decks: decks.len() as u32,
        effective: effective(decks.iter().map(|(_, w)| *w)),
        size: q(&value(Measure::Size)),
        starter: q(&|m| m.starter),
        tags: Tag::ALL.iter().map(|t| (*t, q(&|m| m.tags[t]))).collect(),
        jobs: Role::ALL.iter().map(|r| (*r, q(&value(Measure::Job(*r))))).collect(),
        shares: Role::ALL.iter().map(|r| (*r, q(&value(Measure::Share(*r))))).collect(),
        balance: q(&value(Measure::Balance)),
        dead: q(&value(Measure::Dead)),
        max_hp_gained: q(&|m| m.max_hp_gained),
    }
}

/// Benchmarks from measured decks: one per character and act with at least
/// `min_decks` effective decks, and one per act for all characters.
pub fn measure(decks: &[(&Sample, &Measured)], min_decks: f32) -> Vec<Benchmark> {
    let mut acts: Vec<u8> = decks.iter().map(|(s, _)| s.act).collect();
    acts.sort_unstable();
    acts.dedup();
    let mut characters: Vec<&CharacterId> = decks.iter().map(|(s, _)| &s.character).collect();
    characters.sort();
    characters.dedup();
    let mut out = Vec::new();
    for act in acts {
        let in_act: Vec<&(&Sample, &Measured)> = decks.iter().filter(|(s, _)| s.act == act).collect();
        for c in &characters {
            let mine: Vec<(&Measured, f32)> =
                in_act.iter().filter(|(s, _)| s.character == **c).map(|(s, m)| (*m, s.weight)).collect();
            if effective(mine.iter().map(|(_, w)| *w)) >= min_decks {
                out.push(benchmark(Some((*c).clone()), act, &mine));
            }
        }
        let all: Vec<(&Measured, f32)> = in_act.iter().map(|(s, m)| (*m, s.weight)).collect();
        if effective(all.iter().map(|(_, w)| *w)) >= MIN_POOLED {
            out.push(benchmark(None, act, &all));
        }
    }
    out
}

/// The benchmark for a character's act among `benchmarks`: its own, else
/// every character's.
pub fn find<'a>(benchmarks: &'a [Benchmark], character: &CharacterId, act: u8) -> Option<&'a Benchmark> {
    let of = |c: Option<&CharacterId>| benchmarks.iter().find(|b| b.act == act && b.character.as_ref() == c);
    of(Some(character)).or_else(|| of(None))
}

/// The benchmark for a character's act: its own, else every character's.
pub fn for_run<'a>(catalog: &'a Catalog, character: &CharacterId, act: u8) -> Option<&'a Benchmark> {
    find(&catalog.benchmarks, character, act)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quartiles_interpolate_between_ranks() {
        let q = Quartiles::of(vec![4.0, 1.0, 3.0, 2.0, 5.0]);
        assert_eq!((q.p25, q.p50, q.p75), (2.0, 3.0, 4.0));
        let q = Quartiles::of(vec![1.0, 2.0]);
        assert_eq!((q.p25, q.p50, q.p75), (1.25, 1.5, 1.75));
        assert_eq!(Quartiles::of(vec![]), Quartiles::default());
    }

    #[test]
    fn capped_weights_shrink_the_effective_count() {
        assert_eq!(effective([1.0f32; 10].into_iter()), 10.0);
        // One player's 10 runs at a tenth each, plus 10 single runs.
        let w: Vec<f32> = std::iter::repeat_n(0.1, 10).chain(std::iter::repeat_n(1.0, 10)).collect();
        assert!((effective(w.into_iter()) - 11.0).abs() < 1e-3);
    }
}
