//! What decides each elite and boss fight, measured from community decks
//! going into it (the same runs as the composition risks: one ascension,
//! recent builds, each player capped).
//!
//! A deck is low on a role when it's in the bottom quarter of its
//! character's decks going into that act's fights of the same kind (elites
//! or the boss): bands are per character so a character's usual build
//! doesn't read as a shortfall. A role is a demand of the fight when low
//! decks die there clearly more often than the middle half, by the same
//! test as the composition risks (`composition::holds`): gap, player
//! bootstrap, both halves of the players. Per role, the job or the share
//! with the wider gap is kept.
//!
//! Pure: samples in, demands out.

use std::collections::BTreeMap;

use crate::analysis::composition::{band, holds};
use crate::catalog::{FightDemand, FightDemands, Measure, OutlookBands, Quartiles};
use crate::ids::{CharacterId, EncounterId};
use crate::playbook::{CompositionLines, Role};

/// A community deck going into an elite or boss fight.
#[derive(Clone, Debug)]
pub struct FightSample {
    pub encounter: EncounterId,
    pub character: CharacterId,
    pub act: u8,
    pub boss: bool,
    pub values: BTreeMap<Measure, f32>,
    pub won: bool,
    pub weight: f32,
    pub player: u64,
    /// The survival check's score for the deck here (`survival::score`).
    pub score: Option<f32>,
}

/// The measures a fight demand can be: each role's job and share.
fn measures() -> impl Iterator<Item = Measure> {
    Role::ALL.into_iter().flat_map(|r| [Measure::Job(r), Measure::Share(r)])
}

fn salt_of(enc: &EncounterId) -> u64 {
    enc.as_str().bytes().fold(0u64, |h, b| h.wrapping_mul(31) ^ u64::from(b))
}

/// The survival check's score banded on one fight's decks, with each band's
/// death rate; `None` with too few scored decks.
fn bands(fs: &[&FightSample], lines: &CompositionLines, salt: u64) -> Option<OutlookBands> {
    let scored: Vec<(&FightSample, f32)> = fs.iter().filter_map(|s| Some((*s, s.score?))).collect();
    if scored.iter().map(|(s, _)| s.weight).sum::<f32>() < lines.min_decks {
        return None;
    }
    let q = Quartiles::weighted(scored.iter().map(|(s, v)| (*v, s.weight)).collect());
    let rows: Vec<(usize, bool, f32)> = scored.iter().map(|(s, v)| (band(*v, q), s.won, s.weight)).collect();
    let flipped: Vec<(usize, bool, f32)> = rows.iter().map(|(b, w, n)| (*b, !*w, *n)).collect();
    let players: Vec<u64> = scored.iter().map(|(s, _)| s.player).collect();
    let death = |b: usize| {
        let (n, d) =
            rows.iter().filter(|r| r.0 == b).fold((0.0, 0.0), |(n, d), r| (n + r.2, d + if r.1 { 0.0 } else { r.2 }));
        if n > 0.0 { d / n } else { 0.0 }
    };
    Some(OutlookBands {
        p25: q.p25,
        p75: q.p75,
        rates: [death(0), death(1), death(2)],
        // The best quarter dies clearly less: its "wins" (deaths) are under
        // the middle half's.
        better: holds(0, &flipped, &players, lines, salt ^ 1).is_some(),
        worse: holds(2, &rows, &players, lines, salt ^ 2).is_some(),
    })
}

/// Decks of one act and kind (elite or boss) on one measure.
type Key = (u8, bool, Measure);
/// Those decks of one character, or of all (`None`).
type Group<'a> = (Key, Option<&'a CharacterId>);

/// Quartiles whose `p25` is the low line: a deck under it is low. When the
/// bottom quarter ties at one value (most decks have no AoE at all), the
/// line moves up to the next value, so the tie counts as low.
fn low_line(values: Vec<(f32, f32)>) -> Quartiles {
    let q = Quartiles::weighted(values.clone());
    let total: f32 = values.iter().map(|v| v.1).sum();
    let at: f32 = values.iter().filter(|v| v.0 == q.p25).map(|v| v.1).sum();
    let above = values.iter().map(|v| v.0).filter(|v| *v > q.p25).fold(f32::INFINITY, f32::min);
    // A quarter or more of the decks on the line itself.
    if at >= total / 4.0 && q.p25 < q.p75 && above.is_finite() { Quartiles { p25: above, ..q } } else { q }
}

/// Every elite and boss with enough decks (`min_decks`), with the roles that
/// decide it.
pub fn measure(samples: &[FightSample], lines: &CompositionLines) -> Vec<FightDemands> {
    // Quartiles per (act, kind, measure): per character, and across all.
    let mut values: BTreeMap<Group, Vec<(f32, f32)>> = BTreeMap::new();
    for s in samples {
        for m in measures() {
            let Some(v) = s.values.get(&m) else { continue };
            for who in [Some(&s.character), None] {
                values.entry(((s.act, s.boss, m), who)).or_default().push((*v, s.weight));
            }
        }
    }
    let quartiles: BTreeMap<Group, Quartiles> = values
        .into_iter()
        .filter(|(_, v)| v.iter().map(|x| x.1).sum::<f32>() >= lines.min_decks)
        .map(|(k, v)| (k, low_line(v)))
        .collect();
    let q = |key: Key, c: &CharacterId| quartiles.get(&(key, Some(c))).or_else(|| quartiles.get(&(key, None))).copied();

    let mut by_enc: BTreeMap<&EncounterId, Vec<&FightSample>> = BTreeMap::new();
    for s in samples {
        by_enc.entry(&s.encounter).or_default().push(s);
    }
    let mut out = Vec::new();
    for (enc, fs) in by_enc {
        let decks: f32 = fs.iter().map(|s| s.weight).sum();
        if decks < lines.min_decks {
            continue;
        }
        let died: f32 = fs.iter().filter(|s| !s.won).map(|s| s.weight).sum();
        let (act, boss) = (fs[0].act, fs[0].boss);
        let mut best: BTreeMap<Role, FightDemand> = BTreeMap::new();
        for (mi, m) in measures().enumerate() {
            let key = (act, boss, m);
            let mut rows = Vec::new();
            let mut players = Vec::new();
            for s in &fs {
                let (Some(v), Some(q)) = (s.values.get(&m), q(key, &s.character)) else { continue };
                rows.push((band(*v, q), s.won, s.weight));
                players.push(s.player);
            }
            let salt = (mi as u64) << 8 ^ salt_of(enc);
            let Some((won_low, won_mid, n)) = holds(0, &rows, &players, lines, salt) else { continue };
            let role = match m {
                Measure::Job(r) | Measure::Share(r) => r,
                _ => continue,
            };
            let demand = FightDemand {
                measure: m,
                rate: 1.0 - won_low,
                typical: 1.0 - won_mid,
                decks: n,
                lines: quartiles
                    .iter()
                    .filter(|((k, _), _)| *k == key)
                    .filter_map(|((_, c), q)| Some(((*c)?.clone(), q.p25)))
                    .collect(),
                line: quartiles.get(&(key, None)).map_or(0.0, |q| q.p25),
            };
            let gap = demand.rate - demand.typical;
            if best.get(&role).is_none_or(|b| gap > b.rate - b.typical) {
                best.insert(role, demand);
            }
        }
        let mut demands: Vec<FightDemand> = best.into_values().collect();
        demands.sort_by(|a, b| (b.rate - b.typical).total_cmp(&(a.rate - a.typical)));
        let outlook = bands(&fs, lines, salt_of(enc));
        out.push(FightDemands { encounter: enc.clone(), decks, death_rate: died / decks, demands, outlook });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `players` players with `n` decks each at Block `block`, dying with
    /// chance `die` (deterministic: the first share of each player's decks).
    fn decks(players: std::ops::Range<u64>, block: f32, die: f32, n: usize) -> Vec<FightSample> {
        players
            .flat_map(|p| {
                (0..n).map(move |i| FightSample {
                    encounter: EncounterId::new("CRAB"),
                    character: CharacterId::new("IRONCLAD"),
                    act: 1,
                    boss: true,
                    values: [
                        (Measure::Job(Role::Survival), block),
                        (Measure::Job(Role::Aoe), ((p + i as u64) % 4) as f32),
                    ]
                    .into_iter()
                    .collect(),
                    won: (i as f32) >= die * n as f32,
                    weight: 1.0,
                    player: p,
                    score: Some(30.0 - block),
                })
            })
            .collect()
    }

    #[test]
    fn a_role_whose_low_side_dies_more_is_a_demand() {
        // A fifth of decks block little and die 40%; the rest die 10%.
        let mut s = decks(0..50, 8.0, 0.4, 10);
        s.extend(decks(50..240, 20.0, 0.1, 10));
        let lines = CompositionLines::default();
        let fights = measure(&s, &lines);
        assert_eq!(fights.len(), 1);
        let f = &fights[0];
        assert!((f.death_rate - (0.4 * 50.0 + 0.1 * 190.0) / 240.0).abs() < 0.01, "{}", f.death_rate);
        let d = &f.demands[0];
        assert_eq!(d.measure, Measure::Job(Role::Survival));
        assert!(d.rate > 0.35 && d.typical < 0.15, "{} {}", d.rate, d.typical);
        assert!(d.line > 8.0 && d.line <= 20.0, "{}", d.line);
        // AoE varies but says nothing about dying: no demand.
        assert!(f.demands.iter().all(|d| d.measure != Measure::Job(Role::Aoe)));
        // The survival score (worse with less Block) bands the deaths too.
        let o = f.outlook.as_ref().unwrap();
        assert!(o.worse && o.rates[2] > o.rates[1], "{o:?}");
    }

    #[test]
    fn a_bottom_quarter_tied_at_zero_counts_as_low() {
        let q = low_line((0..10).map(|i| (if i < 6 { 0.0 } else { i as f32 / 10.0 }, 1.0)).collect());
        assert!((q.p25 - 0.6).abs() < 1e-6, "the next value up: {q:?}");
        // Spread values keep their quartile.
        let q = low_line((0..8).map(|i| (i as f32, 1.0)).collect());
        assert_eq!(q.p25, 1.0);
    }

    #[test]
    fn too_few_decks_measure_nothing() {
        let s = decks(0..10, 8.0, 0.4, 5);
        assert!(measure(&s, &CompositionLines::default()).is_empty());
    }
}
