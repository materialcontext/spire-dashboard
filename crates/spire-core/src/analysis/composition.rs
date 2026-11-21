//! Composition: the deck's makeup next to decks that beat this act's boss.
//!
//! Each [`Measure`] (a role's job, a role's share of the deck, damage
//! against Block, deck size, dead draws) is placed on the range community
//! decks that beat the act's boss had going into it (one ascension, each
//! player's weight capped; built by `benchmark::measure`). The range
//! follows the run through the act: from what decks had entering it (those
//! that beat the previous boss; the starting deck in act 1) to what they had
//! at its boss, by the share of the act's floors played.
//!
//! The deck warns only where community runs show a side of a measure losing
//! the boss more often than the middle half does: a [`Risk`], measured at
//! data refresh by [`risks`]. Every other measure is shown, never flagged.
//!
//! How a risk is kept (each step guards against a known pitfall):
//! - Decks going into the boss, won and lost: judging by winners alone
//!   hides what losers had too (survivorship).
//! - Each deck is banded against its own character's range (under the 25th
//!   percentile, the middle half, over the 75th), then every character is
//!   pooled: one rule, more decks behind it.
//! - The gap (the side's boss win rate less the middle half's) must be at
//!   least `min_gap`, its bootstrap interval must stay below zero, and it
//!   must point the same way in two halves of the players. Resampling is by
//!   player, not by deck: a player's runs aren't independent.
//!
//! Pure: values in, values out.

use std::collections::BTreeMap;

use crate::analysis::benchmark;
use crate::analysis::deck::DeckProfile;
use crate::analysis::jobs;
use crate::analysis::sim::DeckSim;
use crate::catalog::{Benchmark, Catalog, Measure, Quartiles, Risk, Side};
use crate::ids::CharacterId;
use crate::playbook::{CompositionLines, Role};
use crate::run::RunState;

impl Measure {
    pub const ALL: [Measure; 15] = [
        Measure::Job(Role::Mitigation),
        Measure::Job(Role::Survival),
        Measure::Job(Role::FastDamage),
        Measure::Job(Role::ScalingDamage),
        Measure::Job(Role::Aoe),
        Measure::Job(Role::Chip),
        Measure::Share(Role::Mitigation),
        Measure::Share(Role::Survival),
        Measure::Share(Role::FastDamage),
        Measure::Share(Role::ScalingDamage),
        Measure::Share(Role::Aoe),
        Measure::Share(Role::Chip),
        Measure::Balance,
        Measure::Size,
        Measure::Dead,
    ];

    /// "Mitigation", "Mitigation cards", "Damage to Block"…
    pub fn label(self) -> String {
        match self {
            Measure::Job(r) => r.label().to_string(),
            Measure::Share(r) => format!("{} cards", r.label()),
            Measure::Balance => "Damage to Block".into(),
            Measure::Size => "Deck size".into(),
            Measure::Dead => "Dead draws".into(),
        }
    }

    /// How a value prints: prefix, suffix, decimals and scale.
    fn units(self) -> (&'static str, &'static str, usize, f32) {
        match self {
            Measure::Job(Role::Mitigation | Role::Survival) => ("", " Block", 0, 1.0),
            Measure::Job(Role::FastDamage) => ("", " damage", 0, 1.0),
            Measure::Job(Role::ScalingDamage) => ("×", "", 1, 1.0),
            Measure::Job(Role::Aoe) | Measure::Share(_) | Measure::Dead => ("", "%", 0, 100.0),
            Measure::Job(Role::Chip) => ("", " hits", 1, 1.0),
            Measure::Balance => ("", "", 1, 1.0),
            Measure::Size => ("", " cards", 0, 1.0),
        }
    }

    fn number(self, v: f32) -> String {
        let (_, _, places, scale) = self.units();
        format!("{:.*}", places, v * scale)
    }

    /// The value in its own units: "12 Block", "24%", "×1.6".
    pub fn format(self, v: f32) -> String {
        let (pre, post, ..) = self.units();
        format!("{pre}{}{post}", self.number(v))
    }

    /// A middle half and its median, units once: "5–9 Block (7 median)".
    pub fn range(self, q: Quartiles) -> String {
        let (pre, post, ..) = self.units();
        format!("{pre}{}–{}{post} ({} median)", self.number(q.p25), self.number(q.p75), self.number(q.p50))
    }

    /// What it measures, for hovers.
    pub fn describe(self) -> String {
        match self {
            Measure::Job(r) => format!("{}: {}", r.label(), jobs::describe_job(r)),
            Measure::Share(r) => format!("Share of the deck's cards that are {}", r.label()),
            Measure::Balance => "Damage over the first 3 turns against 3 turns of ordinary Block".into(),
            Measure::Size => "Cards in the deck".into(),
            Measure::Dead => "Share of draws that can't be played: curses, statuses, unplayables".into(),
        }
    }

    /// Its range in a benchmark.
    pub fn of(self, b: &Benchmark) -> Option<Quartiles> {
        match self {
            Measure::Job(r) => b.jobs.get(&r).copied(),
            Measure::Share(r) => b.shares.get(&r).copied(),
            Measure::Balance => Some(b.balance),
            Measure::Size => Some(b.size),
            Measure::Dead => Some(b.dead),
        }
    }
}

/// "Too little Mitigation", "Too many AoE cards": a measure on the wrong
/// side.
pub fn phrase(measure: Measure, side: Side) -> String {
    let low = side == Side::Low;
    match measure {
        Measure::Job(r) => format!("{} {}", if low { "Too little" } else { "Too much" }, r.label()),
        Measure::Share(r) => format!("{} {} cards", if low { "Too few" } else { "Too many" }, r.label()),
        Measure::Balance if low => "Too much Block for its damage".into(),
        Measure::Balance => "Too much damage for its Block".into(),
        Measure::Size if low => "Deck too small".into(),
        Measure::Size => "Deck too big".into(),
        Measure::Dead if low => "Too few dead draws".into(),
        Measure::Dead => "Too many dead draws".into(),
    }
}

impl Row {
    /// "Too little Mitigation: 9 Block; winners 12–18 Block (15 median)".
    pub fn summary(&self) -> String {
        let side = self.flag.map_or(Side::Low, |f| f.side);
        format!("{}: {}", phrase(self.measure, side), self.numbers())
    }

    /// "9 Block; winners 12–18 Block (15 median)": the deck's value against
    /// the decks that beat the boss.
    pub fn numbers(&self) -> String {
        format!("{}; winners {}", self.measure.format(self.value), self.measure.range(self.range))
    }

    /// "Decks like this beat the Act 3 boss 70% of the time; the middle
    /// half 80%."
    pub fn evidence(&self) -> Option<String> {
        let f = self.flag?;
        Some(format!(
            "Decks like this beat the Act {} boss {:.0}% of the time; the middle half {:.0}%.",
            f.act + 1,
            f.rate * 100.0,
            f.typical * 100.0
        ))
    }
}

/// A deck's value on every measure, from its simulation and profile.
/// Shares count starter cards too: they're in the deck.
pub fn values(sim: &DeckSim, profile: &DeckProfile) -> BTreeMap<Measure, f32> {
    let j = jobs::measure(sim);
    let n = profile.total.max(1) as f32;
    Measure::ALL
        .iter()
        .map(|m| {
            let v = match m {
                Measure::Job(r) => j.value(*r),
                Measure::Share(r) => (profile.role(*r) + profile.starter_role(*r)) as f32 / n,
                Measure::Balance => j.fast_damage / (3.0 * j.mitigation).max(1.0),
                Measure::Size => profile.total as f32,
                Measure::Dead => sim.dead_draws,
            };
            (*m, v)
        })
        .collect()
}

/// One measure: the deck's value, the range at this point of the act, and
/// the risk it's on the wrong side of, if any.
#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    pub measure: Measure,
    pub value: f32,
    pub range: Quartiles,
    pub flag: Option<Risk>,
}

/// The deck against decks that beat this act's boss.
#[derive(Clone, Debug, PartialEq)]
pub struct Composition {
    pub act: u8,
    /// Share of the act's floors played (0–1): where the range sits between
    /// entering the act and its boss.
    pub progress: f32,
    pub decks: u32,
    pub effective: f32,
    /// The character had too few decks: every character's are used.
    pub all_characters: bool,
    /// Every measure, in [`Measure::ALL`] order.
    pub rows: Vec<Row>,
}

impl Composition {
    pub fn row(&self, m: Measure) -> Option<&Row> {
        self.rows.iter().find(|r| r.measure == m)
    }

    /// Rows on the wrong side of a risk, the biggest win-rate gap first.
    pub fn flags(&self) -> Vec<&Row> {
        let mut out: Vec<&Row> = self.rows.iter().filter(|r| r.flag.is_some()).collect();
        let gap = |r: &Row| r.flag.map_or(0.0, |f| f.typical - f.rate);
        out.sort_by(|a, b| gap(b).total_cmp(&gap(a)));
        out
    }
}

/// Share of the act's floors played, by the act's usual length (with the
/// extra boss floor in a double-boss act).
pub fn progress(state: &RunState, catalog: &Catalog) -> f32 {
    let rooms = state.act.as_ref().and_then(|a| catalog.acts.get(a)).map(|a| a.rooms);
    let length = super::floors::act_length(state, catalog).or(rooms).filter(|n| *n > 0).unwrap_or(17);
    (state.act_floor as f32 / length as f32).clamp(0.0, 1.0)
}

/// Where the run's deck stands: `values` are its measures, `start` the
/// character's starting deck's (where act 1's range begins). `None` when
/// the data has no benchmark for the act.
pub fn standing(
    state: &RunState,
    catalog: &Catalog,
    values: &BTreeMap<Measure, f32>,
    start: &BTreeMap<Measure, f32>,
) -> Option<Composition> {
    let character = state.character.as_ref()?;
    let act = state.act_index;
    let end = benchmark::for_run(catalog, character, act)?;
    let entering = act.checked_sub(1).and_then(|prev| benchmark::for_run(catalog, character, prev));
    let progress = progress(state, catalog);
    let rows = Measure::ALL
        .iter()
        .filter_map(|m| {
            let to = m.of(end)?;
            let from = match entering {
                Some(b) => m.of(b).unwrap_or(to),
                // Every starting deck is the same: centre the boss range's
                // spread on it, or a single point would flag any pick.
                None => start.get(m).map_or(to, |v| Quartiles {
                    p25: v - (to.p50 - to.p25),
                    p50: *v,
                    p75: v + (to.p75 - to.p50),
                }),
            };
            let range = from.lerp(to, progress);
            let value = values.get(m).copied().unwrap_or(0.0);
            let side = if value < range.p25 {
                Some(Side::Low)
            } else if value > range.p75 {
                Some(Side::High)
            } else {
                None
            };
            let flag = side
                .and_then(|s| catalog.risks.iter().find(|r| r.measure == *m && r.act == act && r.side == s).copied());
            Some(Row { measure: *m, value, range, flag })
        })
        .collect();
    Some(Composition {
        act,
        progress,
        decks: end.decks,
        effective: end.effective,
        all_characters: end.character.is_none(),
        rows,
    })
}

/// A deck going into a boss in a community run.
#[derive(Clone, Debug, PartialEq)]
pub struct BossSample {
    pub character: CharacterId,
    pub act: u8,
    pub values: BTreeMap<Measure, f32>,
    pub won: bool,
    /// The run's weight (each player's runs capped).
    pub weight: f32,
    /// Who played it: runs are resampled by player. An anonymous run is its
    /// own player.
    pub player: u64,
}

/// A deck's band on a measure: 0 under the 25th percentile, 1 the middle
/// half, 2 over the 75th.
pub(crate) fn band(v: f32, q: Quartiles) -> usize {
    if v < q.p25 {
        0
    } else if v > q.p75 {
        2
    } else {
        1
    }
}

/// Boss win rate of `band` among weighted `(band, won, weight)` rows.
fn rate(rows: &[(usize, bool, f32)], band: usize) -> Option<f32> {
    let (n, w) =
        rows.iter().filter(|r| r.0 == band).fold((0.0, 0.0), |(n, w), r| (n + r.2, w + if r.1 { r.2 } else { 0.0 }));
    (n > 0.0).then(|| w / n)
}

/// A small deterministic generator for the bootstrap.
struct Lcg(u64);

impl Lcg {
    fn next(&mut self, below: usize) -> usize {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((self.0 >> 33) as usize) % below.max(1)
    }
}

/// The measure-and-side risks that hold up in `samples`, banded against
/// `benchmarks` (decks that beat each boss). See the module notes.
pub fn risks(samples: &[BossSample], benchmarks: &[Benchmark], lines: &CompositionLines) -> Vec<Risk> {
    let mut acts: Vec<u8> = samples.iter().map(|s| s.act).collect();
    acts.sort_unstable();
    acts.dedup();
    let mut out = Vec::new();
    for (mi, m) in Measure::ALL.iter().enumerate() {
        for act in &acts {
            // (band, won, weight) per sample, and its player.
            let mut rows: Vec<(usize, bool, f32)> = Vec::new();
            let mut players: Vec<u64> = Vec::new();
            for s in samples.iter().filter(|s| s.act == *act) {
                let Some(q) = benchmark::find(benchmarks, &s.character, *act).and_then(|b| m.of(b)) else {
                    continue;
                };
                let Some(v) = s.values.get(m) else { continue };
                rows.push((band(*v, q), s.won, s.weight));
                players.push(s.player);
            }
            for side in [Side::Low, Side::High] {
                let b = if side == Side::Low { 0 } else { 2 };
                if let Some(risk) = risk(*m, *act, side, b, &rows, &players, lines, mi as u64) {
                    out.push(risk);
                }
            }
        }
    }
    out
}

/// One measure, act and side: kept when the gap is big enough, its
/// interval (resampling players) stays below zero, and both halves of the
/// players agree.
#[allow(clippy::too_many_arguments)]
fn risk(
    measure: Measure,
    act: u8,
    side: Side,
    band: usize,
    rows: &[(usize, bool, f32)],
    players: &[u64],
    lines: &CompositionLines,
    salt: u64,
) -> Option<Risk> {
    let salt = salt << 8 ^ u64::from(act) << 4;
    let (rate, typical, decks) = holds(band, rows, players, lines, salt)?;
    Some(Risk { measure, act, side, rate, typical, decks })
}

/// Whether `band`'s win rate is clearly under the middle half's (band 1)
/// in `(band, won, weight)` rows: the gap is at least `min_gap`, its
/// interval (resampling players) stays below zero, and both halves of the
/// players agree. Returns both win rates and the band's effective decks.
pub(crate) fn holds(
    band: usize,
    rows: &[(usize, bool, f32)],
    players: &[u64],
    lines: &CompositionLines,
    salt: u64,
) -> Option<(f32, f32, f32)> {
    let gap = |rows: &[(usize, bool, f32)]| Some(rate(rows, band)? - rate(rows, 1)?);
    let full = gap(rows)?;
    if full > -lines.min_gap {
        return None;
    }
    // Both halves of the players.
    for half in 0..2 {
        let part: Vec<(usize, bool, f32)> =
            rows.iter().zip(players).filter(|(_, p)| *p % 2 == half).map(|(r, _)| *r).collect();
        if gap(&part).is_none_or(|g| g >= 0.0) {
            return None;
        }
    }
    // Resample players with replacement.
    let mut by_player: BTreeMap<u64, Vec<(usize, bool, f32)>> = BTreeMap::new();
    for (r, p) in rows.iter().zip(players) {
        by_player.entry(*p).or_default().push(*r);
    }
    let groups: Vec<&Vec<(usize, bool, f32)>> = by_player.values().collect();
    let mut rng = Lcg(0x9E37_79B9_7F4A_7C15 ^ salt ^ band as u64);
    let mut gaps: Vec<f32> = (0..lines.resamples.max(1))
        .filter_map(|_| {
            let sample: Vec<(usize, bool, f32)> =
                (0..groups.len()).flat_map(|_| groups[rng.next(groups.len())].iter().copied()).collect();
            gap(&sample)
        })
        .collect();
    gaps.sort_by(f32::total_cmp);
    let upper =
        gaps.get(((gaps.len() as f32 * (1.0 + lines.confidence) / 2.0) as usize).min(gaps.len().saturating_sub(1)))?;
    if *upper >= 0.0 {
        return None;
    }
    let decks: f32 = rows.iter().filter(|r| r.0 == band).map(|r| r.2).sum();
    Some((rate(rows, band)?, rate(rows, 1)?, decks))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn q(p25: f32, p50: f32, p75: f32) -> Quartiles {
        Quartiles { p25, p50, p75 }
    }

    #[test]
    fn ranges_print_their_units_once() {
        assert_eq!(Measure::Job(Role::Mitigation).range(q(5.0, 7.0, 9.0)), "5–9 Block (7 median)");
        assert_eq!(Measure::Share(Role::Aoe).range(q(0.1, 0.15, 0.2)), "10–20% (15 median)");
        assert_eq!(Measure::Job(Role::ScalingDamage).range(q(1.2, 1.5, 1.8)), "×1.2–1.8 (1.5 median)");
        assert_eq!(Measure::Job(Role::Mitigation).format(7.0), "7 Block");
    }

    fn bench(character: Option<&str>, act: u8, block: Quartiles) -> Benchmark {
        Benchmark {
            character: character.map(CharacterId::new),
            act,
            decks: 200,
            effective: 200.0,
            jobs: [(Role::Mitigation, block)].into_iter().collect(),
            ..Default::default()
        }
    }

    /// `n` decks per player for `players` players, Block `block`, won with
    /// chance `win` (deterministic: the first `win` share of each player's
    /// decks).
    fn decks(players: std::ops::Range<u64>, block: f32, win: f32, n: usize) -> Vec<BossSample> {
        players
            .flat_map(|p| {
                (0..n).map(move |i| BossSample {
                    character: CharacterId::new("IRONCLAD"),
                    act: 1,
                    values: [(Measure::Job(Role::Mitigation), block)].into_iter().collect(),
                    won: (i as f32) < win * n as f32,
                    weight: 1.0,
                    player: p,
                })
            })
            .collect()
    }

    #[test]
    fn a_side_that_loses_more_for_every_player_is_a_risk() {
        let b = vec![bench(Some("IRONCLAD"), 1, q(10.0, 14.0, 18.0))];
        // Low Block (5) wins 6 in 10; the middle half (14) 9 in 10.
        let mut s = decks(0..40, 5.0, 0.6, 10);
        s.extend(decks(0..40, 14.0, 0.9, 10));
        let r = risks(&s, &b, &CompositionLines::default());
        let low = r.iter().find(|r| r.measure == Measure::Job(Role::Mitigation) && r.side == Side::Low);
        let low = low.expect("low Block is a risk");
        assert!((low.rate - 0.6).abs() < 1e-4 && (low.typical - 0.9).abs() < 1e-4);
        assert_eq!(low.act, 1);
        assert!(!r.iter().any(|r| r.side == Side::High), "no deck is over the range");
    }

    #[test]
    fn a_gap_from_a_few_players_or_too_small_is_not() {
        let b = vec![bench(Some("IRONCLAD"), 1, q(10.0, 14.0, 18.0))];
        // One player loses with low Block, everyone else wins with it.
        let mut s = decks(0..1, 5.0, 0.0, 40);
        s.extend(decks(1..40, 5.0, 0.95, 2));
        s.extend(decks(0..40, 14.0, 0.9, 10));
        assert!(risks(&s, &b, &CompositionLines::default()).is_empty(), "one player's decks don't make a rule");
        // A 1-point gap.
        let mut s = decks(0..40, 5.0, 0.89, 100);
        s.extend(decks(0..40, 14.0, 0.9, 100));
        assert!(risks(&s, &b, &CompositionLines::default()).is_empty(), "under min_gap");
    }

    #[test]
    fn the_deck_stands_on_a_moving_range_and_warns_only_on_a_risk() {
        use crate::catalog::Act;
        use crate::ids::ActId;
        let mut cat = Catalog::default();
        let act = ActId::new("ACT1");
        let ten_floors: Act =
            serde_json::from_str(r#"{"id": "ACT1", "name": "Act 1", "index": 0, "length": 10}"#).expect("an act");
        cat.acts.insert(act.clone(), ten_floors);
        cat.benchmarks = vec![bench(Some("IRONCLAD"), 0, q(10.0, 14.0, 18.0))];
        cat.risks = vec![Risk {
            measure: Measure::Job(Role::Mitigation),
            act: 0,
            side: Side::Low,
            rate: 0.7,
            typical: 0.85,
            decks: 100.0,
        }];
        let mut state =
            RunState { character: Some(CharacterId::new("IRONCLAD")), act: Some(act), ..Default::default() };
        let block = |v: f32| -> BTreeMap<Measure, f32> { [(Measure::Job(Role::Mitigation), v)].into_iter().collect() };
        let start = block(6.0);
        // Floor 0: the starting deck's value, with the boss range's spread.
        let c = standing(&state, &cat, &block(6.0), &start).expect("a benchmark");
        let row = c.row(Measure::Job(Role::Mitigation)).unwrap();
        assert_eq!(row.range, q(2.0, 6.0, 10.0));
        assert!(row.flag.is_none());
        // Halfway: halfway to the boss range; 5 Block is under it, a risk.
        state.act_floor = 5;
        let c = standing(&state, &cat, &block(5.0), &start).unwrap();
        let row = c.row(Measure::Job(Role::Mitigation)).unwrap();
        assert_eq!(row.range, q(6.0, 10.0, 14.0));
        assert_eq!(row.flag.map(|f| f.side), Some(Side::Low));
        assert_eq!(c.flags().len(), 1);
        // Over the range: no risk on that side, no flag.
        let c = standing(&state, &cat, &block(30.0), &start).unwrap();
        assert!(c.flags().is_empty());
    }

    #[test]
    fn the_range_moves_from_entering_the_act_to_its_boss() {
        let a = q(0.0, 0.0, 0.0);
        let b = q(10.0, 20.0, 30.0);
        assert_eq!(a.lerp(b, 0.5), q(5.0, 10.0, 15.0));
        assert_eq!(a.lerp(b, 2.0), b, "clamped");
    }

    #[test]
    fn weighted_quartiles_follow_the_weights() {
        // A heavy 10 outweighs three light values.
        let w = Quartiles::weighted(vec![(1.0, 1.0), (2.0, 1.0), (3.0, 1.0), (10.0, 7.0)]);
        assert_eq!((w.p25, w.p50, w.p75), (3.0, 10.0, 10.0));
        assert_eq!(Quartiles::weighted(vec![]), Quartiles::default());
    }
}
