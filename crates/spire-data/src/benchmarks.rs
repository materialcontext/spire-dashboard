//! Community decks at the end of each act they cleared, for the deck
//! benchmarks (`spire_core::analysis::benchmark`).
//!
//! Each run in the Spire Codex export is read the way monitored runs are
//! ([`crate::game_run::read`]): its floors replay onto the character's
//! starting deck, card by card. The deck is taken just before the run
//! enters the next act, so it is the deck that beat that act's boss; a won
//! run's last act counts too. Single-player runs without modifiers only,
//! as for the encounter statistics.
//!
//! Pure: JSONL text and a catalog in, samples out.

use spire_core::analysis::benchmark::{self, Sample};
use spire_core::run::{RunEvent, RunState, apply};
use spire_core::{Catalog, DataPack, Playbook};

use crate::game_run::{self, ActHint};
use crate::run_file::WRun;

/// The decks one run cleared acts with.
fn run_samples(line: &str, catalog: &Catalog) -> Vec<Sample> {
    let Ok(run) = serde_json::from_str::<WRun>(line) else { return Vec::new() };
    let modded = run.modifiers.as_ref().is_some_and(|m| !m.is_null() && m.as_array().is_none_or(|a| !a.is_empty()));
    if run.players.len() != 1 || modded {
        return Vec::new();
    }
    let won = run.win == Some(true);
    let Ok(read) = game_run::read(line, catalog, &ActHint::default()) else { return Vec::new() };
    let mut out = Vec::new();
    let mut state = RunState::default();
    let mut take = |state: &RunState| {
        if let Some(character) = &state.character {
            out.push(Sample { character: character.clone(), act: state.act_index, state: state.clone() });
        }
    };
    for e in &read.events {
        // Entering a later act: the act before it was cleared.
        if let RunEvent::EnterAct { index, .. } = e
            && *index > 0
            && state.act.is_some()
        {
            take(&state);
        }
        state = apply(state, e);
    }
    if won && state.act.is_some() {
        take(&state);
    }
    out
}

/// Every cleared act's deck in a page of the run export.
pub fn samples(jsonl: &str, catalog: &Catalog) -> Vec<Sample> {
    jsonl.lines().flat_map(|line| run_samples(line, catalog)).collect()
}

/// Rebuilds the community decks in `pages` and stores their benchmarks in
/// the pack (tagged with `playbook`, simulated against the pack's own
/// data). Returns how many decks were measured.
pub fn add(pack: &mut DataPack, pages: &[String], playbook: &Playbook) -> usize {
    let catalog = Catalog::from_layers([pack.clone()]);
    let samples: Vec<Sample> = pages.iter().flat_map(|p| samples(p, &catalog)).collect();
    pack.benchmarks = benchmark::measure(&samples, &catalog, playbook);
    samples.len()
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
