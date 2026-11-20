//! Lookout: what a `?` room this act can still give that this deck needs,
//! and so is worth steering into `?` rooms for. Two kinds, one shape:
//! - **Relics** crucial to the deck (`relic_fits`), below.
//! - **Event options** that do what the deck needs (`event_fits`): remove
//!   or transform a curse, raise max HP when short of it, heal when low.
//!   What an option does is data (measured from community runs: how often
//!   taking it removed a card, healed, cost HP…); when the deck needs it is
//!   the playbook's, with the same tests as relics plus low HP. Options that
//!   also do what the rule rules out (`unless`: a curse for a curse) don't
//!   count.
//!
//! Each warning names its sources: the event and option, how often it
//! delivers, how often runs meet the event this act, and what it costs.
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
use crate::catalog::{Catalog, EffectKind, Event, EventEffect, EventOption, Rarity, Relic};
use crate::ids::{EventId, RelicId};
use crate::playbook::{EventFit, Playbook, RelicFit, Shortfall, When};
use crate::run::RunState;

/// Something worth fishing for, and where to find it.
#[derive(Clone, Debug, PartialEq)]
pub struct Lookout {
    /// The relic's name, the shortfall ("Short of Block"), or the event
    /// rule's title ("Remove a curse").
    pub title: String,
    /// "6 Powers: each deals 8 damage to ALL enemies when played."
    pub why: String,
    pub wanted: Wanted,
    /// Where it can come from this act, likeliest first.
    pub sources: Vec<Source>,
}

/// What a lookout is after.
#[derive(Clone, Debug, PartialEq)]
pub enum Wanted {
    /// Relics that would answer it and that this act's `?` rooms can give,
    /// none held.
    Relics(Vec<RelicId>),
    /// An event option doing one of these.
    Effects(Vec<EffectKind>),
}

impl Lookout {
    /// The relics it's after (none for an event option).
    pub fn relics(&self) -> &[RelicId] {
        match &self.wanted {
            Wanted::Relics(r) => r,
            Wanted::Effects(_) => &[],
        }
    }

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
    /// What the option costs, from community runs (HP, max HP, a curse,
    /// gold).
    pub costs: Vec<EventEffect>,
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

/// Cards matching a rule are named when there are this many or fewer.
const NAMED: usize = 3;

/// Why a rule's `when` holds for this run (its cards, its shortfall, its
/// HP), or `None` when it doesn't.
pub fn holds(
    when: &When,
    state: &RunState,
    deck: &[ClassifiedCard],
    profile: &DeckProfile,
    catalog: &Catalog,
    playbook: &Playbook,
) -> Option<String> {
    let mut why = Vec::new();
    if let Some(cards) = &when.cards {
        let matched: Vec<&str> = deck
            .iter()
            .filter(|c| c.unmet.is_none())
            .filter_map(|c| c.card.filter(|card| cards.matches(card)))
            .map(|card| card.name.as_str())
            .collect();
        let n = matched.len();
        if (n as u32) < when.at_least.max(1) {
            return None;
        }
        let what = if n == 1 && !when.what_one.is_empty() { &when.what_one } else { &when.what };
        // A few are named ("2× Inflame").
        let mut names: Vec<(&str, usize)> = Vec::new();
        for m in &matched {
            match names.iter_mut().find(|(name, _)| name == m) {
                Some((_, k)) => *k += 1,
                None => names.push((m, 1)),
            }
        }
        let named: Vec<String> =
            names.iter().map(|(name, k)| if *k > 1 { format!("{k}× {name}") } else { name.to_string() }).collect();
        why.push(if n <= NAMED { format!("{n} {what} ({})", named.join(", ")) } else { format!("{n} {what}") });
    }
    if let Some(short) = when.short_of {
        why.push(shortfall(short, state, profile, catalog, playbook)?);
    }
    if let Some(below) = when.hp_below {
        if state.max_hp <= 0 || state.hp_fraction() >= below {
            return None;
        }
        why.push(format!("HP {}/{}", state.hp, state.max_hp));
    }
    (!why.is_empty()).then(|| why.join("; "))
}

/// What an option costs: its cost effects, by community runs.
fn costs(option: Option<&EventOption>) -> Vec<EventEffect> {
    option.map_or_else(Vec::new, |o| o.effects.iter().filter(|e| e.kind.is_cost()).copied().collect())
}

/// The room a source is in: the event, the option and what it requires.
fn event_room(e: &Event, option: Option<&EventOption>, one_of: u32) -> Room {
    Room::Event {
        event: e.id.clone(),
        name: e.name.clone(),
        option: option.map(|o| (o.title.clone(), o.text.clone())),
        one_of,
        requires: e.requires.iter().filter(|r| !r.starts_with("Act ")).cloned().collect(),
    }
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
        // Named on a later screen, the option has no text of its own here.
        let option = first.option.as_ref().map(|t| {
            e.options
                .iter()
                .find(|o| &o.title == t)
                .cloned()
                .unwrap_or_else(|| EventOption { title: t.clone(), ..Default::default() })
        });
        out.push(Source {
            room: event_room(e, option.as_ref(), first.one_of),
            gives: given.iter().map(|r| r.relic.clone()).collect(),
            odds: given.iter().map(|r| 1.0 / r.one_of.max(1) as f32).sum::<f32>().min(1.0),
            met: e.met.get(act_id).copied(),
            costs: costs(option.as_ref()),
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
            out.push(Source { room: Room::Treasure, gives, odds, met: act.unknown_treasure, costs: Vec::new() });
        }
    }
    out.sort_by(|a, b| b.chance().unwrap_or(0.0).total_cmp(&a.chance().unwrap_or(0.0)));
    out
}

/// The event options this act that answer a rule: one source per option
/// doing one of its effects and none of its `unless`.
fn option_sources(fit: &EventFit, state: &RunState, events: &[&Event]) -> Vec<Source> {
    let Some(act_id) = &state.act else { return Vec::new() };
    let mut out: Vec<Source> = Vec::new();
    for e in events {
        for o in &e.options {
            if fit.unless.iter().any(|k| o.effect(*k).is_some()) {
                continue;
            }
            let odds = fit.effects.iter().filter_map(|k| o.effect(*k)).map(|x| x.rate).fold(0.0, f32::max);
            if odds <= 0.0 {
                continue;
            }
            out.push(Source {
                room: event_room(e, Some(o), 1),
                gives: Vec::new(),
                odds: odds.min(1.0),
                met: e.met.get(act_id).copied(),
                costs: costs(Some(o)).into_iter().filter(|c| !fit.effects.contains(&c.kind)).collect(),
            });
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
    for fit in playbook.relic_fits.iter().filter(wanted).filter(|f| f.when.short_of.is_none()) {
        let Some(why) = holds(&fit.when, state, deck, profile, catalog, playbook) else { continue };
        if out.iter().any(|l| l.relics().contains(&fit.relic)) {
            continue;
        }
        let relics = vec![fit.relic.clone()];
        let sources = sources(&relics, state, catalog, &events);
        if !sources.is_empty() {
            let wanted = Wanted::Relics(relics);
            out.push(Lookout { title: name(&fit.relic), why: format!("{why}: {}", fit.why), wanted, sources });
        }
    }

    // By a shortfall: one warning naming every relic that fixes it.
    for short in [Shortfall::Block, Shortfall::MaxHp] {
        let fits: Vec<&RelicFit> = playbook
            .relic_fits
            .iter()
            .filter(wanted)
            .filter(|f| f.when.short_of == Some(short))
            .filter(|f| holds(&f.when, state, deck, profile, catalog, playbook).is_some())
            .collect();
        let Some(why) = fits.first().and_then(|_| shortfall(short, state, profile, catalog, playbook)) else {
            continue;
        };
        let wanted: Vec<RelicId> = fits.iter().map(|f| f.relic.clone()).collect();
        let sources = sources(&wanted, state, catalog, &events);
        // Only the relics this act's `?` rooms can give, in the playbook's order.
        let relics: Vec<RelicId> = wanted.into_iter().filter(|r| sources.iter().any(|s| s.gives.contains(r))).collect();
        if !sources.is_empty() {
            let title = format!("Short of {}", short.label());
            out.push(Lookout { title, why, wanted: Wanted::Relics(relics), sources });
        }
    }

    // Event options: one warning per rule that holds.
    for fit in &playbook.event_fits {
        let Some(why) = holds(&fit.when, state, deck, profile, catalog, playbook) else { continue };
        let sources = option_sources(fit, state, &events);
        if !sources.is_empty() {
            let wanted = Wanted::Effects(fit.effects.clone());
            out.push(Lookout { title: fit.title.clone(), why: format!("{why}: {}", fit.why), wanted, sources });
        }
    }
    out.sort_by(|a, b| b.best_chance().total_cmp(&a.best_chance()));
    out
}
