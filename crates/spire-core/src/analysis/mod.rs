//! Pure analysis over a run, the catalog and the playbook.

pub mod advice;
pub mod deck;
pub mod events;
pub mod floors;
pub mod sim;
pub mod threats;
pub mod timeline;

use crate::catalog::Catalog;
use crate::playbook::Playbook;
use crate::run::{NodeKind, RunState};

pub use advice::{Advice, Severity};
pub use deck::{ClassifiedCard, Coverage, DeckProfile, DeckRow};
pub use events::{EventWatch, Need};
pub use threats::{ThreatPreview, ThreatProfile};

/// Everything the dashboard shows, computed in one pass from a state.
/// Recompute it when the run, catalog or playbook changes; never per frame.
#[derive(Clone, Debug)]
pub struct Dashboard {
    pub deck: DeckProfile,
    /// The deck grouped for display.
    pub rows: Vec<DeckRow>,
    pub needs: Vec<Need>,
    pub events: Vec<EventWatch>,
    /// The act's known bosses (two in a double-boss act).
    pub bosses: Vec<ThreatPreview>,
    pub advice: Vec<Advice>,
    /// Threat previews for each fight node kind.
    pub threats: Vec<(NodeKind, Vec<ThreatPreview>)>,
}

impl Dashboard {
    pub fn compute(state: &RunState, catalog: &Catalog, playbook: &Playbook) -> Dashboard {
        let classified = deck::classify(&state.deck, catalog, playbook);
        let deck = deck::profile(&classified);
        let rows = deck::deck_rows(&classified);
        let needs = events::needs(state, &deck, playbook);
        let events = events::watchlist(state, catalog, playbook, &needs);
        let bosses = advice::boss_previews(state, catalog, playbook, &deck);
        let advice = advice::advise(state, catalog, playbook, &deck);
        let threats = [NodeKind::Monster, NodeKind::Elite, NodeKind::Boss]
            .into_iter()
            .map(|k| (k, threats::next_threats(state, k, catalog, playbook, &deck)))
            .collect();
        Dashboard { deck, rows, needs, events, bosses, advice, threats }
    }
}
