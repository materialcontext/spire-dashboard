//! Pure analysis over a run, the catalog and the playbook.

pub mod advice;
pub mod balance;
pub mod benchmark;
pub mod deck;
pub mod events;
pub mod floors;
pub mod healing;
pub mod jobs;
pub mod lines;
pub mod pattern;
pub mod report;
pub mod requirements;
pub mod rules;
pub mod shape;
pub mod sim;
pub mod survival;
pub mod tags;
pub mod threats;
pub mod timeline;

use crate::catalog::Catalog;
use crate::playbook::Playbook;
use crate::run::{NodeKind, RunState};

pub use advice::{Advice, Severity};
pub use deck::{ClassifiedCard, Coverage, DeckProfile, DeckRow};
pub use events::{Lookout, Room as RelicRoom, Source as RelicSource};
pub use threats::{ThreatPreview, ThreatProfile};

/// Everything the dashboard shows, computed in one pass from a state.
/// Recompute it when the run, catalog or playbook changes; never per frame.
#[derive(Clone, Debug)]
pub struct Dashboard {
    pub deck: DeckProfile,
    /// The deck grouped for display.
    pub rows: Vec<DeckRow>,
    /// Event relics crucial to this deck that this act can still give.
    pub lookout: Vec<Lookout>,
    /// The act's known bosses (two in a double-boss act).
    pub bosses: Vec<ThreatPreview>,
    pub advice: Vec<Advice>,
    /// Threat previews for each fight node kind.
    pub threats: Vec<(NodeKind, Vec<ThreatPreview>)>,
    /// Roles of the run's relics, in the order held (empty for relics the
    /// simulator can't read).
    pub relic_roles: Vec<Vec<crate::playbook::Role>>,
}

impl Dashboard {
    pub fn compute(state: &RunState, catalog: &Catalog, playbook: &Playbook) -> Dashboard {
        let classified = deck::classify_run(state, catalog, playbook);
        let deck = deck::profile(&classified);
        let rows = deck::deck_rows(&classified);
        let lookout = events::lookout(state, &classified, &deck, catalog, playbook);
        let bosses = advice::boss_previews(state, catalog, playbook, &deck);
        let advice = advice::advise(state, catalog, playbook, &deck);
        let threats = [NodeKind::Monster, NodeKind::Elite, NodeKind::Boss]
            .into_iter()
            .map(|k| (k, threats::next_threats(state, k, catalog, playbook, &deck)))
            .collect();
        let orbs: Vec<crate::catalog::Orb> = catalog.orbs.values().cloned().collect();
        let relic_roles = state
            .relics
            .iter()
            .map(|r| {
                catalog.relics.get(r).map_or_else(Vec::new, |relic| {
                    playbook.relic_roles(&relic.text, &crate::mechanics::read(&relic.text, &orbs, false))
                })
            })
            .collect();
        Dashboard { deck, rows, lookout, bosses, advice, threats, relic_roles }
    }
}
