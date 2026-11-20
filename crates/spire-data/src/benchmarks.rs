//! Community decks going into every boss fight: for the deck benchmarks
//! and composition risks (`spire_core::analysis::benchmark`,
//! `composition`), and for the card lines (`spire_core::analysis::lines`).
//!
//! Which runs count for the benchmarks and risks (the playbook's
//! `composition` lines): one ascension, builds at or after `min_build`, each
//! run once (by its hash), and each player weighed so they count for at most
//! `player_cap` runs. Anonymous runs each count as their own player. Card
//! lines keep every ascension (repeats dropped).
//!
//! Each run in the Spire Codex export is read the way monitored runs are
//! ([`crate::game_run::read`]): its floors replay onto the character's
//! starting deck, card by card. The deck is taken just before the run
//! enters the next act, so it is the deck that beat that act's boss; a won
//! run's last act counts too. For the lines, the deck is taken going into
//! each boss fight, with whether the run got past it. Single-player runs
//! without modifiers only, as for the encounter statistics.
//!
//! Pure: JSONL text and a catalog in, samples out.

use std::collections::{BTreeMap, BTreeSet};

use spire_core::analysis::benchmark::{self, Measured, Sample};
use spire_core::analysis::composition::{self, BossSample};
use spire_core::analysis::lines::BossFight;
use spire_core::catalog::Rarity;
use spire_core::playbook::CompositionLines;
use spire_core::run::{NodeKind, Outcome, RunEvent, RunState, apply};
use spire_core::{Catalog, DataPack, Playbook};

use crate::game_run::{self, ActHint};
use crate::run_file::WRun;

/// Who and what a community run is: to keep one ascension, drop repeats,
/// and keep one player from outweighing the rest.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RunMeta {
    /// The export's id for the run.
    pub hash: Option<String>,
    /// An opaque token for the player; none on older runs.
    pub player: Option<String>,
    pub ascension: u32,
    pub build: Option<String>,
}

/// A deck going into a boss fight, and whether the run got past it.
#[derive(Clone, Debug)]
pub struct BossDeck {
    pub sample: Sample,
    pub won: bool,
}

/// One run's decks: at the end of each act it cleared, and going into each
/// boss fight.
#[derive(Default)]
pub struct RunDecks {
    pub meta: RunMeta,
    pub cleared: Vec<Sample>,
    pub bosses: Vec<BossFight>,
    pub boss_decks: Vec<BossDeck>,
}

/// Distinct reward cards in a deck: no starters, curses, statuses or tokens.
fn reward_cards(state: &RunState, catalog: &Catalog) -> Vec<spire_core::ids::CardId> {
    let mut out: Vec<_> = state
        .deck
        .iter()
        .filter(|d| {
            catalog.cards.get(&d.card).is_some_and(|c| {
                !matches!(c.rarity, Rarity::Basic | Rarity::Starter | Rarity::Curse | Rarity::Status | Rarity::Token)
            })
        })
        .map(|d| d.card.clone())
        .collect();
    out.sort();
    out.dedup();
    out
}

pub fn run_decks(line: &str, catalog: &Catalog) -> RunDecks {
    let mut out = RunDecks::default();
    let Ok(run) = serde_json::from_str::<WRun>(line) else { return out };
    let modded = run.modifiers.as_ref().is_some_and(|m| !m.is_null() && m.as_array().is_none_or(|a| !a.is_empty()));
    if run.players.len() != 1 || modded {
        return out;
    }
    let won = run.win == Some(true);
    out.meta = RunMeta {
        hash: run.run_hash.clone(),
        player: run.player_token.clone(),
        ascension: run.ascension,
        build: run.build_id.clone(),
    };
    let Ok(read) = game_run::read(line, catalog, &ActHint::default()) else { return out };
    let ending =
        read.events.iter().find_map(|e| if let RunEvent::End { outcome, .. } = e { Some(outcome) } else { None });
    let last_node = read.events.iter().rposition(|e| matches!(e, RunEvent::Node(_)));
    let mut state = RunState::default();
    for (i, e) in read.events.iter().enumerate() {
        // Entering a later act: the act before it was cleared.
        if let RunEvent::EnterAct { index, .. } = e
            && *index > 0
            && state.act.is_some()
            && let Some(character) = &state.character
        {
            out.cleared.push(Sample {
                character: character.clone(),
                act: state.act_index,
                state: state.clone(),
                weight: 1.0,
            });
        }
        // Going into a boss fight: lost only when the run died there, on
        // its last floor; a run abandoned there says nothing.
        if let RunEvent::Node(n) = e
            && n.kind == NodeKind::Boss
            && let Some(character) = &state.character
        {
            let last = Some(i) == last_node;
            let outcome = match ending {
                Some(Outcome::Died { encounter }) if last => {
                    Some(encounter.is_none() || encounter == &n.encounter).map(|died_here| !died_here)
                }
                Some(Outcome::Abandoned) if last => None,
                _ => Some(true),
            };
            if let Some(won_fight) = outcome {
                let sample =
                    Sample { character: character.clone(), act: state.act_index, state: state.clone(), weight: 1.0 };
                out.boss_decks.push(BossDeck { sample, won: won_fight });
                out.bosses.push(BossFight {
                    character: character.clone(),
                    act: state.act_index,
                    boss: n.encounter.clone(),
                    cards: reward_cards(&state, catalog),
                    won: won_fight,
                });
            }
        }
        state = apply(state, e);
    }
    if won
        && state.act.is_some()
        && let Some(character) = &state.character
    {
        out.cleared.push(Sample {
            character: character.clone(),
            act: state.act_index,
            state: state.clone(),
            weight: 1.0,
        });
    }
    out
}

/// The decks one run cleared acts with.
fn run_samples(line: &str, catalog: &Catalog) -> Vec<Sample> {
    run_decks(line, catalog).cleared
}

/// Every run in a page of the run export, with its decks.
pub fn runs(jsonl: &str, catalog: &Catalog) -> Vec<RunDecks> {
    jsonl
        .lines()
        .map(|line| run_decks(line, catalog))
        .filter(|r| !r.cleared.is_empty() || !r.boss_decks.is_empty())
        .collect()
}

/// Every boss fight's deck in a page of the run export.
pub fn boss_fights(jsonl: &str, catalog: &Catalog) -> Vec<BossFight> {
    jsonl.lines().flat_map(|line| run_decks(line, catalog).bosses).collect()
}

/// Every cleared act's deck in a page of the run export.
pub fn samples(jsonl: &str, catalog: &Catalog) -> Vec<Sample> {
    jsonl.lines().flat_map(|line| run_samples(line, catalog)).collect()
}

/// A game build's version numbers ("v0.107.1" is 0, 107, 1).
fn version(build: &str) -> Vec<u32> {
    build.trim_start_matches('v').split('.').map(|p| p.parse().unwrap_or(0)).collect()
}

/// A stable id for a run's player: its token, else the run itself.
fn player_id(meta: &RunMeta) -> u64 {
    // FNV-1a: stable across builds and platforms, unlike the std hasher.
    let key = meta.player.as_deref().or(meta.hash.as_deref()).unwrap_or("");
    key.bytes().fold(0xcbf29ce484222325u64, |h, b| (h ^ u64::from(b)).wrapping_mul(0x100000001b3))
}

/// The runs that count for benchmarks and risks, each with its weight.
pub fn weighted_runs<'a>(runs: &'a [RunDecks], lines: &CompositionLines) -> Vec<(&'a RunDecks, f32)> {
    let min = version(&lines.min_build);
    let kept: Vec<&RunDecks> = runs
        .iter()
        .filter(|r| r.meta.ascension == lines.ascension)
        .filter(|r| r.meta.build.as_deref().is_some_and(|b| version(b) >= min))
        .collect();
    let mut per_player: BTreeMap<&str, u32> = BTreeMap::new();
    for r in &kept {
        if let Some(p) = &r.meta.player {
            *per_player.entry(p).or_default() += 1;
        }
    }
    kept.into_iter()
        .map(|r| {
            let w = r.meta.player.as_deref().map_or(1.0, |p| (lines.player_cap / per_player[p] as f32).min(1.0));
            (r, w)
        })
        .collect()
}

/// Rebuilds the community decks in `pages`, once each, and stores the
/// benchmarks (decks that beat each boss, going in) and the composition
/// risks in the pack (tagged with `playbook`, simulated against the pack's
/// own data). Returns how many decks were measured, and every boss fight's
/// deck for the card lines (one replay serves both).
pub fn add(pack: &mut DataPack, pages: &[String], playbook: &Playbook) -> (usize, Vec<BossFight>) {
    let catalog = Catalog::from_layers([pack.clone()]);
    let mut seen: BTreeSet<String> = BTreeSet::new();
    let mut runs: Vec<RunDecks> = Vec::new();
    for line in pages.iter().flat_map(|p| p.lines()) {
        let r = run_decks(line, &catalog);
        if r.meta.hash.as_ref().is_some_and(|h| !seen.insert(h.clone())) {
            continue;
        }
        runs.push(r);
    }
    let bosses: Vec<BossFight> = runs.iter().flat_map(|r| r.bosses.iter().cloned()).collect();
    let lines = &playbook.composition;
    // Every deck going into a boss, weighted and measured once.
    let mut decks: Vec<(Sample, bool, u64, Measured)> = Vec::new();
    for (r, w) in weighted_runs(&runs, lines) {
        let player = player_id(&r.meta);
        for b in &r.boss_decks {
            let sample = Sample { weight: w, ..b.sample.clone() };
            let m = benchmark::measured(&sample.state, &catalog, playbook);
            decks.push((sample, b.won, player, m));
        }
    }
    // Benchmarks: the decks that beat each boss.
    let winners: Vec<(&Sample, &Measured)> = decks.iter().filter(|d| d.1).map(|d| (&d.0, &d.3)).collect();
    pack.benchmarks = benchmark::measure(&winners, lines.min_decks);
    // Risks: every deck, won or lost.
    let samples: Vec<BossSample> = decks
        .iter()
        .map(|(s, won, player, m)| BossSample {
            character: s.character.clone(),
            act: s.act,
            values: m.values.clone(),
            won: *won,
            weight: s.weight,
            player: *player,
        })
        .collect();
    pack.risks = composition::risks(&samples, &pack.benchmarks, lines);
    (decks.len(), bosses)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_won_run_gives_a_deck_per_act_and_a_lost_one_stops_short() {
        let catalog = crate::bundled_catalog();
        let run: serde_json::Value =
            serde_json::from_str(include_str!("../tests/fixtures/finished.run")).expect("fixture");
        let line = |v: &serde_json::Value| serde_json::to_string(v).unwrap();
        let won = samples(&line(&run), &catalog);
        assert_eq!(won.iter().map(|s| s.act).collect::<Vec<_>>(), vec![0, 1, 2]);
        // Each act's deck is the deck going into the next act: it grows.
        assert!(won[0].state.deck.len() < won[2].state.deck.len());
        assert!(won.iter().all(|s| s.character.as_str() == "IRONCLAD"));
        let mut lost = run.clone();
        lost["win"] = serde_json::Value::Bool(false);
        assert_eq!(samples(&line(&lost), &catalog).len(), 2, "the last act wasn't cleared");
        let mut duo = run.clone();
        let p = duo["players"][0].clone();
        duo["players"].as_array_mut().unwrap().push(p);
        assert!(samples(&line(&duo), &catalog).is_empty(), "multiplayer runs are skipped");
        let two_runs = format!("{}\n{}", line(&run), line(&lost));
        assert_eq!(samples(&two_runs, &catalog).len(), 5, "one run per line");
    }
}
