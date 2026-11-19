//! Relic lookout: relics crucial to this deck that a `?` room this act can
//! still give, and so worth steering into `?` rooms for.
//!
//! A relic is crucial when the deck meets one of the playbook's
//! `relic_fits`:
//! - by its cards: Lost Wisp with 6+ Powers. One warning per relic.
//! - by a shortfall the relic fixes: desperately short of Block or max HP
//!   next to community decks that cleared the previous act (what they had
//!   entering this one). One warning per shortfall, naming every relic
//!   that would fix it.
//!
//! A `?` room is an event or, sometimes, a treasure room. Where a relic can
//! come from is data:
//! - events that name it in an option, or that community runs saw give
//!   it, with how often runs meet each event in the act;
//! - a `?` room that turns into a treasure room (how often, per act, from
//!   community runs), whose chest gives a relic by the act's rarity shares,
//!   at random among the character's pool of that rarity.
//!
//! Left out: relics already held, events already seen this run (they don't
//! repeat), and events the act doesn't allow.

use crate::analysis::benchmark;
use crate::analysis::deck::{ClassifiedCard, DeckProfile};
use crate::analysis::tags::Tag;
use crate::catalog::{Catalog, Event, Rarity, Relic};
use crate::ids::{EventId, RelicId};
use crate::playbook::{Playbook, RelicFit, Shortfall};
use crate::run::RunState;

/// Something worth fishing for, and where to find it.
#[derive(Clone, Debug, PartialEq)]
pub struct Lookout {
    /// The relic's name, or the shortfall ("Short of Block").
    pub title: String,
    /// "6 Powers: each deals 8 damage to ALL enemies when played."
    pub why: String,
    /// The relics that would answer it and that this act's `?` rooms can
    /// give, none held.
    pub relics: Vec<RelicId>,
    /// Where they can come from this act, likeliest first.
    pub sources: Vec<Source>,
}

impl Lookout {
    /// The likeliest source's chance this act, when known.
    pub fn best_chance(&self) -> f32 {
        self.sources.iter().filter_map(Source::chance).fold(0.0, f32::max)
    }
}

/// A `?` room outcome that can give one of the relics.
#[derive(Clone, Debug, PartialEq)]
pub struct Source {
    pub room: Room,
    /// Which of the lookout's relics it can give.
    pub gives: Vec<RelicId>,
    /// Chance it gives one of them, once there (1 when an option gives it
    /// outright or lets you choose).
    pub odds: f32,
    /// Share of community runs that came to it in this act.
    pub met: Option<f32>,
}

/// Where a relic comes from.
#[derive(Clone, Debug, PartialEq)]
pub enum Room {
    Event {
        event: EventId,
        name: String,
        /// The option that gives it, and that option's text (its cost).
        option: Option<(String, String)>,
        /// Given at random among this many (1: outright or chosen).
        one_of: u32,
        /// What the game checks before offering it, act limits aside.
        requires: Vec<String>,
    },
    /// A `?` room that turns into a treasure room.
    Treasure,
}

impl Source {
    /// Chance of getting one of its relics this act, when community runs
    /// say how often it comes up.
    pub fn chance(&self) -> Option<f32> {
        self.met.map(|m| m * self.odds)
    }
}

/// Why a fit holds for this run (its cards, its shortfall, or both), or
/// `None` when it doesn't.
pub fn fit_holds(
    fit: &RelicFit,
    state: &RunState,
    deck: &[ClassifiedCard],
    profile: &DeckProfile,
    catalog: &Catalog,
    playbook: &Playbook,
) -> Option<String> {
    let mut why = Vec::new();
    if let Some(cards) = &fit.cards {
        let n = deck.iter().filter(|c| c.unmet.is_none() && c.card.is_some_and(|card| cards.matches(card))).count();
        if (n as u32) < fit.at_least.max(1) {
            return None;
        }
        why.push(format!("{n} {}", fit.what));
    }
    if let Some(short) = fit.short_of {
        why.push(shortfall(short, state, profile, catalog, playbook)?);
    }
    (!why.is_empty()).then(|| why.join("; "))
}

/// Why the deck is desperately short, or `None` when it isn't. Measured
/// against decks that cleared the previous act; in act 1, only max HP lost
/// since the start counts.
fn shortfall(
    short: Shortfall,
    state: &RunState,
    profile: &DeckProfile,
    catalog: &Catalog,
    pb: &Playbook,
) -> Option<String> {
    let lines = &pb.short;
    let entering = state
        .act_index
        .checked_sub(1)
        .and_then(|prev| state.character.as_ref().and_then(|c| benchmark::for_run(catalog, c, prev)));
    let cleared = format!("decks that cleared Act {}", state.act_index);
    match short {
        Shortfall::Block => {
            let b = entering?;
            let p25 = b.tags.get(&Tag::Block)?.p25;
            let yours = profile.tag(Tag::Block) as f32 / profile.total.max(1) as f32;
            (yours < p25 * lines.block_share).then(|| {
                format!(
                    "Block cards are {:.0}% of the deck; 3 in 4 {cleared} had {:.0}% or more",
                    yours * 100.0,
                    p25 * 100.0
                )
            })
        }
        Shortfall::MaxHp => {
            let gained = (state.max_hp - state.start_max_hp) as f32;
            let p25 = entering.map_or(0.0, |b| b.max_hp_gained.p25);
            (gained < p25 - lines.max_hp_points).then(|| match entering {
                Some(_) => format!("Max HP {:+.0} since the start; 3 in 4 {cleared} had {:+.0} or more", gained, p25),
                None => format!("Max HP {gained:+.0} since the start"),
            })
        }
    }
}

/// Relics of a rarity a chest can give this character.
fn chest_pool<'a>(catalog: &'a Catalog, state: &'a RunState, rarity: Rarity) -> impl Iterator<Item = &'a Relic> + 'a {
    let own = state.character.as_ref().map(|c| c.0.to_lowercase());
    catalog.relics.values().filter(move |r| {
        r.rarity == rarity
            && (r.pool == "shared" || own.as_deref() == Some(r.pool.as_str()))
            && !state.relics.contains(&r.id)
    })
}

/// The `?` rooms this act that can give any of `relics`.
fn sources(relics: &[RelicId], state: &RunState, catalog: &Catalog, events: &[&Event]) -> Vec<Source> {
    let Some(act_id) = &state.act else { return Vec::new() };
    let mut out: Vec<Source> = Vec::new();
    for e in events {
        let given: Vec<_> = e.relics.iter().filter(|r| relics.contains(&r.relic)).collect();
        let Some(first) = given.first() else { continue };
        let option = first.option.as_ref().map(|t| {
            let text = e.options.iter().find(|o| &o.title == t).map(|o| o.text.clone()).unwrap_or_default();
            (t.clone(), text)
        });
        out.push(Source {
            room: Room::Event {
                event: e.id.clone(),
                name: e.name.clone(),
                option,
                one_of: first.one_of,
                requires: e.requires.iter().filter(|r| !r.starts_with("Act ")).cloned().collect(),
            },
            gives: given.iter().map(|r| r.relic.clone()).collect(),
            odds: given.iter().map(|r| 1.0 / r.one_of.max(1) as f32).sum::<f32>().min(1.0),
            met: e.met.get(act_id).copied(),
        });
    }
    if let Some(act) = catalog.acts.get(act_id) {
        let mut gives = Vec::new();
        let mut odds = 0.0;
        for (rarity, share) in &act.chest_rarity {
            let pool = chest_pool(catalog, state, *rarity).count().max(1) as f32;
            for r in chest_pool(catalog, state, *rarity).filter(|r| relics.contains(&r.id)) {
                gives.push(r.id.clone());
                odds += share / pool;
            }
        }
        if !gives.is_empty() {
            out.push(Source { room: Room::Treasure, gives, odds, met: act.unknown_treasure });
        }
    }
    out.sort_by(|a, b| b.chance().unwrap_or(0.0).total_cmp(&a.chance().unwrap_or(0.0)));
    out
}

/// What to fish for in this act, likeliest first. Empty when nothing
/// crucial is on offer, which is most of the time.
pub fn lookout(
    state: &RunState,
    deck: &[ClassifiedCard],
    profile: &DeckProfile,
    catalog: &Catalog,
    playbook: &Playbook,
) -> Vec<Lookout> {
    let Some(act) = &state.act else { return Vec::new() };
    let events: Vec<&Event> = catalog.events_in(act).filter(|e| !state.events_seen.contains(&e.id)).collect();
    let wanted = |f: &&RelicFit| !state.relics.contains(&f.relic);
    let name = |id: &RelicId| catalog.relics.get(id).map_or_else(|| id.to_string(), |r| r.name.clone());
    let mut out: Vec<Lookout> = Vec::new();

    // By the deck's cards: one warning per relic, with what it does.
    for fit in playbook.relic_fits.iter().filter(wanted).filter(|f| f.short_of.is_none()) {
        let Some(why) = fit_holds(fit, state, deck, profile, catalog, playbook) else { continue };
        if out.iter().any(|l| l.relics.contains(&fit.relic)) {
            continue;
        }
        let relics = vec![fit.relic.clone()];
        let sources = sources(&relics, state, catalog, &events);
        if !sources.is_empty() {
            out.push(Lookout { title: name(&fit.relic), why: format!("{why}: {}", fit.why), relics, sources });
        }
    }

    // By a shortfall: one warning naming every relic that fixes it.
    for short in [Shortfall::Block, Shortfall::MaxHp] {
        let fits: Vec<&RelicFit> = playbook
            .relic_fits
            .iter()
            .filter(wanted)
            .filter(|f| f.short_of == Some(short))
            .filter(|f| fit_holds(f, state, deck, profile, catalog, playbook).is_some())
            .collect();
        let Some(why) = fits.first().and_then(|_| shortfall(short, state, profile, catalog, playbook)) else {
            continue;
        };
        let wanted: Vec<RelicId> = fits.iter().map(|f| f.relic.clone()).collect();
        let sources = sources(&wanted, state, catalog, &events);
        // Only the relics this act's `?` rooms can give, in the playbook's order.
        let relics: Vec<RelicId> = wanted.into_iter().filter(|r| sources.iter().any(|s| s.gives.contains(r))).collect();
        if !sources.is_empty() {
            out.push(Lookout { title: format!("Short of {}", short.label()), why, relics, sources });
        }
    }
    out.sort_by(|a, b| b.best_chance().total_cmp(&a.best_chance()));
    out
}
