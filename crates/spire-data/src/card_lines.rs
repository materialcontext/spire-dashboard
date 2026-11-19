//! Card rewards offered in the Spire Codex run export, for the card lines
//! (`spire_core::analysis::lines`): per character and act, how many runs
//! played it and how many of them were offered each card at least once.
//!
//! Pure: decompressed JSONL text in, counts out.

use std::collections::BTreeSet;

use spire_core::DataPack;
use spire_core::analysis::lines::{BossFight, Offers, measure};
use spire_core::ids::{CardId, CharacterId};

use crate::run_file::{WRun, entry};

/// Card offers in single-player, unmodified runs. Bad lines are skipped.
pub fn offers(pages: &[String]) -> Offers {
    let mut out = Offers::new();
    let runs =
        pages.iter().flat_map(|p| p.lines()).filter_map(|line| serde_json::from_str::<WRun>(line).ok()).filter(|run| {
            run.players.len() == 1
                && run.modifiers.as_ref().is_none_or(|m| m.is_null() || m.as_array().is_some_and(|a| a.is_empty()))
        });
    for run in runs {
        let Some(character) = run.players[0].character.as_deref().map(|c| CharacterId::new(entry(c))) else { continue };
        for (act, points) in run.map_point_history.iter().enumerate() {
            let Ok(act) = u8::try_from(act) else { continue };
            if points.is_empty() {
                continue;
            }
            let seen: BTreeSet<&str> = points
                .iter()
                .flat_map(|p| &p.player_stats)
                .flat_map(|s| &s.card_choices)
                .map(|c| entry(&c.card.id))
                .collect();
            let (runs, cards) = out.entry((character.clone(), act)).or_default();
            *runs += 1;
            for c in seen {
                *cards.entry(CardId::new(c)).or_default() += 1;
            }
        }
    }
    out
}

/// Stores the card lines measured from `fights` and the offers in `pages`.
/// Returns how many lines were kept.
pub fn add(pack: &mut DataPack, fights: &[BossFight], pages: &[String]) -> usize {
    pack.card_lines = measure(fights, &offers(pages));
    pack.card_lines.iter().map(|a| a.lines.len()).sum()
}
