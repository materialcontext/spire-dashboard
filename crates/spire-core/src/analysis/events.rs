//! `?` event watchlist: which events this act can show, and which of them
//! the current run should be hoping for.

use crate::analysis::deck::DeckProfile;
use crate::catalog::{Catalog, Event};
use crate::ids::EventId;
use crate::playbook::{Opportunity, Playbook};
use crate::run::RunState;

/// Something the run currently wants, and why.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Need {
    pub opportunity: Opportunity,
    pub reason: String,
    pub weight: u32,
}

/// Reads the run into a list of wants, strongest first.
pub fn needs(state: &RunState, deck: &DeckProfile, playbook: &Playbook) -> Vec<Need> {
    let t = &playbook.needs;
    let mut out = Vec::new();
    let mut want = |opportunity, weight, reason: String| out.push(Need { opportunity, reason, weight });

    // Eternal curses can be neither removed nor transformed, so they never
    // make removal or transform worth more.
    let curses = deck.removable_curses;
    if curses > 0 {
        want(Opportunity::CardRemoval, 3, format!("{curses} removable curse(s) in deck"));
        want(Opportunity::Transform, 2, format!("{curses} removable curse(s) in deck"));
    }
    if deck.starter_basics >= t.starter_cards {
        want(Opportunity::CardRemoval, 2, format!("{} starter Strikes/Defends", deck.starter_basics));
        want(Opportunity::Transform, 1, format!("{} starter Strikes/Defends", deck.starter_basics));
    }
    if state.max_hp > 0 && state.hp_fraction() < t.low_hp {
        want(Opportunity::Heal, 3, format!("HP {}/{}", state.hp, state.max_hp));
        want(Opportunity::MaxHp, 1, "low HP".into());
    }
    if deck.total > 0 && deck.upgraded_fraction() < t.low_upgrades {
        want(Opportunity::Upgrade, 2, format!("{}/{} cards upgraded", deck.upgraded, deck.total));
    }
    if (state.potions.len() as u32) < state.potion_slots {
        want(Opportunity::Potion, 1, "open potion slot".into());
    }
    want(Opportunity::Relic, 2, "relics compound every fight".into());
    want(Opportunity::Enchant, 1, "enchantments upgrade key cards".into());

    out.sort_by_key(|n| std::cmp::Reverse(n.weight));
    out
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct EventWatch {
    pub event: EventId,
    pub name: String,
    pub seen: bool,
    pub opportunities: Vec<Opportunity>,
    pub risks: Vec<Opportunity>,
    /// Needs this event can answer.
    pub matches: Vec<Need>,
    pub score: u32,
}

pub fn event_opportunities(event: &Event, playbook: &Playbook) -> Vec<Opportunity> {
    let mut all: Vec<Opportunity> = event.options.iter().flat_map(|o| playbook.opportunities_in(&o.text)).collect();
    all.sort();
    all.dedup();
    all
}

/// Events possible in the current act, ranked by how well they answer the
/// run's needs. Seen events sink to the bottom (events rarely repeat).
pub fn watchlist(state: &RunState, catalog: &Catalog, playbook: &Playbook, needs: &[Need]) -> Vec<EventWatch> {
    let Some(act) = &state.act else { return Vec::new() };
    let mut out: Vec<EventWatch> = catalog
        .events_in(act)
        .map(|e| {
            let found = event_opportunities(e, playbook);
            let (risks, opportunities): (Vec<_>, Vec<_>) = found.into_iter().partition(|o| o.is_risk());
            let matches: Vec<Need> = needs.iter().filter(|n| opportunities.contains(&n.opportunity)).cloned().collect();
            let score = matches.iter().map(|n| n.weight).sum();
            EventWatch {
                event: e.id.clone(),
                name: e.name.clone(),
                seen: state.events_seen.contains(&e.id),
                opportunities,
                risks,
                matches,
                score,
            }
        })
        .collect();
    out.sort_by(|a, b| a.seen.cmp(&b.seen).then(b.score.cmp(&a.score)).then(a.name.cmp(&b.name)));
    out
}
