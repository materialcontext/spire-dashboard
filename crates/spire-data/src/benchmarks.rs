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
//! starting deck, card by card, and the deck is taken going into each boss
//! fight, with whether the run got past it. Single-player runs without
//! modifiers only, as for the encounter statistics.
//!
//! Pure: JSONL text and a catalog in, samples out.

use std::collections::{BTreeMap, BTreeSet};

use spire_core::analysis::benchmark::{self, Measured, Sample};
use spire_core::analysis::composition::{self, BossSample};
use spire_core::analysis::fight_demands::{self, FightSample};
use spire_core::analysis::lines::BossFight;
use spire_core::analysis::sim::simulate;
use spire_core::analysis::survival;
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

/// A deck going into a boss (or elite) fight, and whether the run got past
/// it.
#[derive(Clone, Debug)]
pub struct BossDeck {
    pub sample: Sample,
    pub won: bool,
    pub encounter: Option<spire_core::EncounterId>,
    pub kind: NodeKind,
}

/// One run's decks going into each boss fight: as samples for the
/// benchmarks and risks, and as reward cards for the lines.
#[derive(Default)]
pub struct RunDecks {
    pub meta: RunMeta,
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
    fight_decks(line, catalog, &[NodeKind::Boss])
}

/// A run's decks going into each fight of the `kinds` given (the card lines
/// keep bosses only).
pub fn fight_decks(line: &str, catalog: &Catalog, kinds: &[NodeKind]) -> RunDecks {
    let mut out = RunDecks::default();
    let Ok(run) = serde_json::from_str::<WRun>(line) else { return out };
    let modded = run.modifiers.as_ref().is_some_and(|m| !m.is_null() && m.as_array().is_none_or(|a| !a.is_empty()));
    if run.players.len() != 1 || modded {
        return out;
    }
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
        // Going into a boss fight: lost only when the run died there, on
        // its last floor; a run abandoned there says nothing.
        if let RunEvent::Node(n) = e
            && kinds.contains(&n.kind)
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
                out.boss_decks.push(BossDeck { sample, won: won_fight, encounter: n.encounter.clone(), kind: n.kind });
            }
            if n.kind == NodeKind::Boss
                && let Some(won_fight) = outcome
            {
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
    out
}

/// Every run in a page of the run export, with its decks.
pub fn runs(jsonl: &str, catalog: &Catalog) -> Vec<RunDecks> {
    jsonl.lines().map(|line| run_decks(line, catalog)).filter(|r| !r.boss_decks.is_empty()).collect()
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

/// Whether a run counts for benchmarks, risks and fight demands: the
/// ascension and builds the playbook names.
fn counts(meta: &RunMeta, lines: &CompositionLines) -> bool {
    meta.ascension == lines.ascension && meta.build.as_deref().is_some_and(|b| version(b) >= version(&lines.min_build))
}

/// The runs that count for benchmarks and risks, each with its weight.
pub fn weighted_runs<'a>(runs: &'a [RunDecks], lines: &CompositionLines) -> Vec<(&'a RunDecks, f32)> {
    let kept: Vec<&RunDecks> = runs.iter().filter(|r| counts(&r.meta, lines)).collect();
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

/// A community deck going into a fight, weighted and measured.
struct Measuring<'a> {
    sample: Sample,
    deck: &'a BossDeck,
    player: u64,
    measured: Measured,
    /// The survival check's score for the fight (`survival::score`).
    score: Option<f32>,
}

/// Rebuilds the community decks in `pages`, once each, and stores in the
/// pack the benchmarks (decks that beat each boss, going in), the
/// composition risks and what decides each elite and boss fight (tagged
/// with `playbook`, simulated against the pack's own data). Returns how many
/// boss decks were measured, and every boss fight's deck for the card lines
/// (one replay serves all).
pub fn add(pack: &mut DataPack, pages: &[String], playbook: &Playbook) -> (usize, Vec<BossFight>) {
    // With the curated fixes the app layers over it, so a fight plays out
    // here as it will there.
    let mut layers = vec![pack.clone()];
    layers.extend(crate::curated::pack(&layers));
    let catalog = Catalog::from_layers(layers);
    let lines = &playbook.composition;
    let mut seen: BTreeSet<String> = BTreeSet::new();
    let mut runs: Vec<RunDecks> = Vec::new();
    for line in pages.iter().flat_map(|p| p.lines()) {
        let mut r = fight_decks(line, &catalog, &[NodeKind::Elite, NodeKind::Boss]);
        if r.meta.hash.as_ref().is_some_and(|h| !seen.insert(h.clone())) {
            continue;
        }
        // Card lines keep every ascension; the decks only the ones counted.
        if !counts(&r.meta, lines) {
            r.boss_decks.clear();
        }
        runs.push(r);
    }
    let bosses: Vec<BossFight> = runs.iter().flat_map(|r| r.bosses.iter().cloned()).collect();
    // Every deck going into an elite or boss, weighted, simulated and
    // measured once, with the survival check's score for the fight.
    let mut decks: Vec<Measuring> = Vec::new();
    for (r, w) in weighted_runs(&runs, lines) {
        let player = player_id(&r.meta);
        for deck in &r.boss_decks {
            let sample = Sample { weight: w, ..deck.sample.clone() };
            let sim = simulate(&sample.state, &catalog, &playbook.simulation);
            let measured = benchmark::measured_with(&sample.state, &sim, &catalog, playbook);
            let score = deck
                .encounter
                .as_ref()
                .and_then(|e| catalog.encounters.get(e))
                .and_then(|e| survival::for_encounter(&sim, e, &catalog, &sample.state, playbook))
                .map(|o| o.score);
            decks.push(Measuring { sample, deck, player, measured, score });
        }
    }
    // What decides each elite and boss fight.
    let fights: Vec<FightSample> = decks
        .iter()
        .filter_map(|d| {
            Some(FightSample {
                encounter: d.deck.encounter.clone()?,
                character: d.sample.character.clone(),
                act: d.sample.act,
                boss: d.deck.kind == NodeKind::Boss,
                values: d.measured.values.clone(),
                won: d.deck.won,
                weight: d.sample.weight,
                player: d.player,
                score: d.score,
            })
        })
        .collect();
    pack.fights = fight_demands::measure(&fights, lines);
    decks.retain(|d| d.deck.kind == NodeKind::Boss);
    // Benchmarks: the decks that beat each boss.
    let winners: Vec<(&Sample, &Measured)> =
        decks.iter().filter(|d| d.deck.won).map(|d| (&d.sample, &d.measured)).collect();
    pack.benchmarks = benchmark::measure(&winners, lines.min_decks);
    // Risks: every deck, won or lost.
    let samples: Vec<BossSample> = decks
        .iter()
        .map(|d| BossSample {
            character: d.sample.character.clone(),
            act: d.sample.act,
            values: d.measured.values.clone(),
            won: d.deck.won,
            weight: d.sample.weight,
            player: d.player,
        })
        .collect();
    pack.risks = composition::risks(&samples, &pack.benchmarks, lines);
    (decks.len(), bosses)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_deck_per_boss_fight_won_or_lost() {
        let catalog = crate::bundled_catalog();
        let run: serde_json::Value =
            serde_json::from_str(include_str!("../tests/fixtures/finished.run")).expect("fixture");
        let line = |v: &serde_json::Value| serde_json::to_string(v).unwrap();
        let decks = |v: &serde_json::Value| run_decks(&line(v), &catalog).boss_decks;
        let won = decks(&run);
        assert_eq!(won.iter().map(|d| d.sample.act).collect::<Vec<_>>(), vec![0, 1, 2]);
        assert!(won.iter().all(|d| d.won && d.sample.character.as_str() == "IRONCLAD"));
        // The deck grows from boss to boss.
        assert!(won[0].sample.state.deck.len() < won[2].sample.state.deck.len());
        let mut lost = run.clone();
        lost["win"] = serde_json::Value::Bool(false);
        let lost = decks(&lost);
        assert_eq!(lost.len(), 3);
        assert!(lost[0].won && lost[1].won && !lost[2].won, "the run died at its last boss");
        let mut duo = run.clone();
        let p = duo["players"][0].clone();
        duo["players"].as_array_mut().unwrap().push(p);
        assert!(decks(&duo).is_empty(), "multiplayer runs are skipped");
        let two_runs = format!("{}\n{}", line(&run), line(&run));
        assert_eq!(runs(&two_runs, &catalog).len(), 2, "one run per line");
    }
}
