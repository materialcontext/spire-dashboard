//! Benchmarks: the run's deck next to community decks that cleared the act.
//!
//! At data refresh, each community run's deck is rebuilt at the end of every
//! act it cleared, tagged and simulated ([`measure`]). The catalog keeps the
//! quartiles per character and act, and for all characters together (the
//! fallback when a character has fewer than [`MIN_DECKS`]). In the app,
//! [`compare`] lines the run's deck up against them, shortfalls first.
//!
//! Benchmark decks are counted with the bundled playbook's tags, at every
//! ascension, and their jobs include their relics. Starter Strikes and
//! Defends are counted in the tag shares (they're in the deck), and shown
//! apart as their own share.

use std::collections::BTreeMap;

use crate::analysis::deck::{DeckProfile, run_profile};
use crate::analysis::jobs::{self, Jobs};
use crate::analysis::sim::simulate;
use crate::analysis::tags::Tag;
use crate::catalog::{Benchmark, Catalog, Quartiles};
use crate::ids::CharacterId;
use crate::playbook::{Playbook, Role};
use crate::run::RunState;

/// Fewer decks than this and a character falls back to every character's.
pub const MIN_DECKS: usize = 30;

/// One community deck at the end of an act it cleared.
#[derive(Clone, Debug)]
pub struct Sample {
    pub character: CharacterId,
    /// 0-based act it cleared.
    pub act: u8,
    pub state: RunState,
}

/// One deck's measures.
struct Measured {
    size: f32,
    max_hp_gained: f32,
    starter: f32,
    tags: BTreeMap<Tag, f32>,
    jobs: Jobs,
}

fn measured(state: &RunState, catalog: &Catalog, playbook: &Playbook) -> Measured {
    let p = run_profile(state, catalog, playbook);
    let n = p.total.max(1) as f32;
    Measured {
        size: p.total as f32,
        max_hp_gained: (state.max_hp - state.start_max_hp) as f32,
        starter: p.starter_basics as f32 / n,
        tags: Tag::ALL.iter().map(|t| (*t, p.tag(*t) as f32 / n)).collect(),
        jobs: jobs::measure(&simulate(state, catalog, &playbook.simulation)),
    }
}

fn benchmark(character: Option<CharacterId>, act: u8, decks: &[&Measured]) -> Benchmark {
    let q = |f: &dyn Fn(&Measured) -> f32| Quartiles::of(decks.iter().map(|m| f(m)).collect());
    Benchmark {
        character,
        act,
        decks: decks.len() as u32,
        size: q(&|m| m.size),
        starter: q(&|m| m.starter),
        tags: Tag::ALL.iter().map(|t| (*t, q(&|m| m.tags[t]))).collect(),
        jobs: Role::ALL.iter().map(|r| (*r, q(&|m| m.jobs.value(*r)))).collect(),
        max_hp_gained: q(&|m| m.max_hp_gained),
    }
}

/// Benchmarks from community decks: one per character and act with at
/// least [`MIN_DECKS`] decks, and one per act for all characters.
pub fn measure(samples: &[Sample], catalog: &Catalog, playbook: &Playbook) -> Vec<Benchmark> {
    let all: Vec<(&Sample, Measured)> = samples.iter().map(|s| (s, measured(&s.state, catalog, playbook))).collect();
    let mut acts: Vec<u8> = samples.iter().map(|s| s.act).collect();
    acts.sort_unstable();
    acts.dedup();
    let mut characters: Vec<&CharacterId> = samples.iter().map(|s| &s.character).collect();
    characters.sort();
    characters.dedup();
    let mut out = Vec::new();
    for act in acts {
        let in_act: Vec<&(&Sample, Measured)> = all.iter().filter(|(s, _)| s.act == act).collect();
        for c in &characters {
            let decks: Vec<&Measured> = in_act.iter().filter(|(s, _)| s.character == **c).map(|(_, m)| m).collect();
            if decks.len() >= MIN_DECKS {
                out.push(benchmark(Some((*c).clone()), act, &decks));
            }
        }
        let decks: Vec<&Measured> = in_act.iter().map(|(_, m)| m).collect();
        if decks.len() >= MIN_DECKS {
            out.push(benchmark(None, act, &decks));
        }
    }
    out
}

/// The benchmark for a character's act: its own, else every character's.
pub fn for_run<'a>(catalog: &'a Catalog, character: &CharacterId, act: u8) -> Option<&'a Benchmark> {
    let of = |c: Option<&CharacterId>| catalog.benchmarks.iter().find(|b| b.act == act && b.character.as_ref() == c);
    of(Some(character)).or_else(|| of(None))
}

/// What's measured.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Measure {
    Job(Role),
    Tag(Tag),
    /// Starter Strikes and Defends: more is worse.
    Starter,
}

impl Measure {
    pub fn label(self) -> &'static str {
        match self {
            Measure::Job(r) => r.label(),
            Measure::Tag(t) => t.label(),
            Measure::Starter => "Starter cards",
        }
    }

    /// Shares are shown as percentages; jobs in their own units.
    pub fn is_share(self) -> bool {
        !matches!(self, Measure::Job(_))
    }
}

/// The run's deck on one measure, against the cleared decks' quartiles.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Comparison {
    pub measure: Measure,
    pub yours: f32,
    pub range: Quartiles,
    /// Worse than three in four cleared decks (under the 25th percentile,
    /// or over the 75th for starter cards).
    pub short: bool,
    /// How far out of the middle half, as a share of the median (0 when
    /// inside it); orders the shortfalls.
    pub by: f32,
}

/// The run's deck against a benchmark: every job, every tag the cleared
/// decks have, and the starter share. Shortfalls first, worst first.
pub fn compare(b: &Benchmark, profile: &DeckProfile, jobs: &Jobs) -> Vec<Comparison> {
    let n = profile.total.max(1) as f32;
    let mut out: Vec<Comparison> = Vec::new();
    let mut push = |measure: Measure, yours: f32, range: Quartiles| {
        let more_is_worse = measure == Measure::Starter;
        let gap = if more_is_worse { yours - range.p75 } else { range.p25 - yours };
        let scale = range.p50.abs().max(f32::EPSILON);
        out.push(Comparison { measure, yours, range, short: gap > 0.0, by: (gap / scale).max(0.0) });
    };
    for r in Role::ALL {
        if let Some(range) = b.jobs.get(&r) {
            push(Measure::Job(r), jobs.value(r), *range);
        }
    }
    for t in Tag::ALL {
        // Tags the cleared decks mostly don't carry aren't worth a row.
        if let Some(range) = b.tags.get(&t).filter(|q| q.p75 > 0.0) {
            push(Measure::Tag(t), profile.tag(t) as f32 / n, *range);
        }
    }
    push(Measure::Starter, profile.starter_basics as f32 / n, b.starter);
    out.sort_by(|a, b| b.short.cmp(&a.short).then(b.by.total_cmp(&a.by)));
    out
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
    fn shortfalls_come_first_and_starter_cards_count_against() {
        let q = |p25, p50, p75| Quartiles { p25, p50, p75 };
        let b = Benchmark {
            character: None,
            act: 0,
            decks: 40,
            size: q(18.0, 20.0, 23.0),
            starter: q(0.2, 0.3, 0.4),
            tags: [(Tag::Block, q(0.2, 0.25, 0.3)), (Tag::Draw, q(0.05, 0.1, 0.15)), (Tag::CleanUp, q(0.0, 0.0, 0.0))]
                .into_iter()
                .collect(),
            jobs: [(Role::Mitigation, q(6.0, 8.0, 10.0)), (Role::FastDamage, q(30.0, 40.0, 50.0))]
                .into_iter()
                .collect(),
            max_hp_gained: q(0.0, 5.0, 10.0),
        };
        let profile = DeckProfile {
            total: 10,
            starter_basics: 6,
            by_tag: [(Tag::Block, 4), (Tag::Draw, 0)].into_iter().collect(),
            ..Default::default()
        };
        let jobs = Jobs { mitigation: 5.0, fast_damage: 45.0, ..Default::default() };
        let c = compare(&b, &profile, &jobs);
        let short: Vec<Measure> = c.iter().filter(|c| c.short).map(|c| c.measure).collect();
        // Starter 60% vs 40% (by 0.67), Draw 0% vs 5% (0.5), Mitigation 5 vs 6 (0.125).
        assert_eq!(short, vec![Measure::Starter, Measure::Tag(Tag::Draw), Measure::Job(Role::Mitigation)]);
        assert!(!c.iter().any(|c| c.measure == Measure::Tag(Tag::CleanUp)), "tags cleared decks lack are skipped");
        assert!(c.iter().any(|c| c.measure == Measure::Job(Role::FastDamage) && !c.short));
    }
}
