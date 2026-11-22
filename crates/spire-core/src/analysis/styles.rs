//! Deck styles: what a deck leans on, read against other decks of its
//! character at the same act.
//!
//! Clustering A10 decks going into bosses and elites by their simulated
//! jobs (each standardised within character and act) gives four groups,
//! the same for elites and bosses: strong Mitigation and Survival
//! (Guarded), strong Fast Damage and Chip (Burst), strong AoE (Wide),
//! strong Scaling Damage (Build-up). A deck's style is the group its
//! standardised jobs lean to most. Each fight measures how each style
//! fares there (`fight_demands`).
//!
//! Pure: values in, values out.

use std::collections::BTreeMap;

use crate::catalog::{Measure, Quartiles, Style, StyleNorm};
use crate::ids::CharacterId;
use crate::playbook::Role;

/// The jobs a style is read from, in `StyleNorm` order.
const JOBS: [Role; 6] = Role::ALL;

fn jobs(values: &BTreeMap<Measure, f32>) -> [f32; 6] {
    JOBS.map(|r| values.get(&Measure::Job(r)).copied().unwrap_or(0.0))
}

/// A deck's style against its character's decks at the act.
pub fn style_of(values: &BTreeMap<Measure, f32>, norm: &StyleNorm) -> Style {
    let v = jobs(values);
    let z: Vec<f32> = (0..6).map(|i| (v[i] - norm.mean[i]) / norm.sd[i].max(1e-6)).collect();
    let at = |r: Role| z[JOBS.iter().position(|x| *x == r).unwrap_or(0)];
    let lean = |s: Style| s.roles().iter().map(|r| at(*r)).sum::<f32>() / s.roles().len() as f32;
    Style::ALL.into_iter().max_by(|a, b| lean(*a).total_cmp(&lean(*b))).unwrap_or(Style::Guarded)
}

/// The norm for a character at an act, if measured.
pub fn norm_for<'n>(norms: &'n [StyleNorm], character: &CharacterId, act: u8) -> Option<&'n StyleNorm> {
    norms.iter().find(|n| &n.character == character && n.act == act)
}

/// A deck's jobs and its weight.
type Weighted = ([f32; 6], f32);

/// Each character's mean and spread of every job at each act, from decks
/// `(character, act, values, weight)`; groups under `min_decks` are left
/// out.
pub fn norms<'a>(
    decks: impl Iterator<Item = (&'a CharacterId, u8, &'a BTreeMap<Measure, f32>, f32)>,
    min_decks: f32,
) -> Vec<StyleNorm> {
    let mut groups: BTreeMap<(&CharacterId, u8), Vec<Weighted>> = BTreeMap::new();
    for (c, act, values, w) in decks {
        groups.entry((c, act)).or_default().push((jobs(values), w));
    }
    groups
        .into_iter()
        .filter(|(_, d)| d.iter().map(|x| x.1).sum::<f32>() >= min_decks)
        .map(|((c, act), d)| {
            let total: f32 = d.iter().map(|x| x.1).sum();
            let mean: [f32; 6] = std::array::from_fn(|i| d.iter().map(|(v, w)| v[i] * w).sum::<f32>() / total);
            let sd: [f32; 6] = std::array::from_fn(|i| {
                (d.iter().map(|(v, w)| (v[i] - mean[i]).powi(2) * w).sum::<f32>() / total).sqrt()
            });
            StyleNorm { character: c.clone(), act, mean, sd }
        })
        .collect()
}

/// The weighted median of `(value, weight)`.
pub(crate) fn median(values: Vec<(f32, f32)>) -> f32 {
    Quartiles::weighted(values).p50
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_deck_leans_to_its_strongest_group_against_its_character() {
        let norm = StyleNorm {
            character: CharacterId::new("X"),
            act: 0,
            mean: [10.0, 15.0, 50.0, 1.0, 0.1, 2.0],
            sd: [2.0, 3.0, 10.0, 0.2, 0.1, 0.5],
        };
        let deck = |v: [f32; 6]| JOBS.iter().zip(v).map(|(r, x)| (Measure::Job(*r), x)).collect::<BTreeMap<_, _>>();
        assert_eq!(style_of(&deck([14.0, 20.0, 50.0, 1.0, 0.1, 2.0]), &norm), Style::Guarded);
        assert_eq!(style_of(&deck([10.0, 15.0, 70.0, 1.0, 0.1, 3.0]), &norm), Style::Burst);
        assert_eq!(style_of(&deck([10.0, 15.0, 50.0, 1.0, 0.4, 2.0]), &norm), Style::Wide);
        assert_eq!(style_of(&deck([10.0, 15.0, 50.0, 1.6, 0.1, 2.0]), &norm), Style::BuildUp);
    }

    #[test]
    fn norms_are_per_character_and_act() {
        let (a, b) = (CharacterId::new("A"), CharacterId::new("B"));
        let v = |m: f32| [(Measure::Job(Role::Mitigation), m)].into_iter().collect::<BTreeMap<_, _>>();
        let (lo, hi) = (v(5.0), v(15.0));
        let decks = vec![(&a, 0, &lo, 60.0), (&a, 0, &hi, 60.0), (&b, 0, &hi, 10.0)];
        let n = norms(decks.into_iter(), 100.0);
        assert_eq!(n.len(), 1, "B has too few decks");
        assert_eq!((n[0].mean[0], n[0].sd[0]), (10.0, 5.0));
    }
}
