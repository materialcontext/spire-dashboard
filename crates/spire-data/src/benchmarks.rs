//! Community decks at the end of each act they cleared, for the deck
//! benchmarks (`spire_core::analysis::benchmark`), and at every boss fight,
//! for the card lines (`spire_core::analysis::lines`).
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

use spire_core::analysis::benchmark::{self, Sample};
use spire_core::analysis::lines::BossFight;
use spire_core::catalog::Rarity;
use spire_core::run::{NodeKind, Outcome, RunEvent, RunState, apply};
use spire_core::{Catalog, DataPack, Playbook};

use crate::game_run::{self, ActHint};
use crate::run_file::WRun;

/// One run's decks: at the end of each act it cleared, and going into each
/// boss fight.
#[derive(Default)]
struct RunDecks {
    cleared: Vec<Sample>,
    bosses: Vec<BossFight>,
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

fn run_decks(line: &str, catalog: &Catalog) -> RunDecks {
    let mut out = RunDecks::default();
    let Ok(run) = serde_json::from_str::<WRun>(line) else { return out };
    let modded = run.modifiers.as_ref().is_some_and(|m| !m.is_null() && m.as_array().is_none_or(|a| !a.is_empty()));
    if run.players.len() != 1 || modded {
        return out;
    }
    let won = run.win == Some(true);
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
            out.cleared.push(Sample { character: character.clone(), act: state.act_index, state: state.clone() });
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
        out.cleared.push(Sample { character: character.clone(), act: state.act_index, state: state.clone() });
    }
    out
}

/// The decks one run cleared acts with.
fn run_samples(line: &str, catalog: &Catalog) -> Vec<Sample> {
    run_decks(line, catalog).cleared
}

/// Every boss fight's deck in a page of the run export.
pub fn boss_fights(jsonl: &str, catalog: &Catalog) -> Vec<BossFight> {
    jsonl.lines().flat_map(|line| run_decks(line, catalog).bosses).collect()
}

/// Every cleared act's deck in a page of the run export.
pub fn samples(jsonl: &str, catalog: &Catalog) -> Vec<Sample> {
    jsonl.lines().flat_map(|line| run_samples(line, catalog)).collect()
}

/// Rebuilds the community decks in `pages` and stores their benchmarks in
/// the pack (tagged with `playbook`, simulated against the pack's own
/// data). Returns how many decks were measured, and every boss fight's deck
/// (one replay serves both).
pub fn add(pack: &mut DataPack, pages: &[String], playbook: &Playbook) -> (usize, Vec<BossFight>) {
    let catalog = Catalog::from_layers([pack.clone()]);
    let mut samples: Vec<Sample> = Vec::new();
    let mut bosses: Vec<BossFight> = Vec::new();
    for line in pages.iter().flat_map(|p| p.lines()) {
        let decks = run_decks(line, &catalog);
        samples.extend(decks.cleared);
        bosses.extend(decks.bosses);
    }
    pack.benchmarks = benchmark::measure(&samples, &catalog, playbook);
    (samples.len(), bosses)
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
