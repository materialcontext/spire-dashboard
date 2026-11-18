//! Slay the Spire 2's own run record, as the game saves it (the run in
//! progress in `current_run.save`, finished runs in `history/*.run`), read
//! into our run log for monitored runs.
//!
//! The game records each floor in `map_point_history`: the room, the fight
//! or event, cards gained, removed, upgraded, enchanted and transformed,
//! relics and potions picked, used or lost, and HP, max HP and gold after it.
//! That maps onto our [`RunEvent`]s nearly one for one. The converter replays
//! as it goes, so changes name the exact card in the deck (an upgraded or
//! enchanted copy), and ends with one correction if our state still differs
//! from the game's (`players[].deck`, `relics`, `potions`).
//!
//! Notes on the format (checked against community `.run` exports):
//! - Purchases also appear as picked choices (`relic_choices`,
//!   `potion_choices`) and bought colourless cards in `cards_gained`, so
//!   only those lists are read.
//! - Ids are `CATEGORY.ENTRY` (`CARD.SNAP`); the entry is our id.
//! - Card offers (`card_choices`) aren't recorded (decision M5).
//!
//! Pure: JSON text in, events out. Unknown fields are ignored and missing
//! ones default, so a game update that adds fields doesn't break it.

use serde::Deserialize;
use spire_core::Catalog;
use spire_core::ids::*;
use spire_core::run::{Change, DeckCard, NodeKind, NodeReport, Outcome, RunEvent, RunState, apply};

#[derive(Deserialize, Default)]
#[serde(default)]
struct WRun {
    acts: Vec<String>,
    ascension: u32,
    seed: Option<String>,
    start_time: Option<i64>,
    players: Vec<WPlayer>,
    map_point_history: Vec<Vec<WPoint>>,
    win: Option<bool>,
    was_abandoned: Option<bool>,
    killed_by_encounter: Option<String>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct WPlayer {
    id: Option<serde_json::Value>,
    character: Option<String>,
    deck: Vec<WCard>,
    relics: Vec<WId>,
    potions: Vec<WId>,
    max_potion_slot_count: Option<u32>,
}

#[derive(Deserialize, Default, Clone)]
#[serde(default)]
struct WCard {
    id: String,
    current_upgrade_level: Option<u32>,
    enchantment: Option<WId>,
}

#[derive(Deserialize, Default, Clone)]
#[serde(default)]
struct WId {
    id: String,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct WPoint {
    map_point_type: Option<String>,
    player_stats: Vec<WStats>,
    rooms: Vec<WRoom>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct WRoom {
    model_id: String,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct WChoice {
    choice: String,
    was_picked: bool,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct WEnchanted {
    card: WCard,
    enchantment: String,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct WTransformed {
    original_card: WCard,
    final_card: WCard,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct WStats {
    player_id: Option<serde_json::Value>,
    current_hp: Option<i32>,
    max_hp: Option<i32>,
    current_gold: Option<i32>,
    cards_gained: Vec<WCard>,
    cards_removed: Vec<WCard>,
    upgraded_cards: Vec<String>,
    cards_enchanted: Vec<WEnchanted>,
    cards_transformed: Vec<WTransformed>,
    relic_choices: Vec<WChoice>,
    relics_removed: Vec<String>,
    potion_choices: Vec<WChoice>,
    potion_used: Vec<String>,
    potion_discarded: Vec<String>,
}

/// `CARD.SNAP` → `SNAP`.
fn entry(id: &str) -> &str {
    id.split_once('.').map_or(id, |(_, rest)| rest)
}

fn none_id(id: &Option<String>) -> Option<&str> {
    id.as_deref().map(entry).filter(|e| !e.is_empty() && *e != "NONE")
}

fn deck_card(c: &WCard) -> DeckCard {
    DeckCard {
        card: CardId::new(entry(&c.id)),
        upgraded: c.current_upgrade_level.unwrap_or(0) > 0,
        enchantment: c.enchantment.as_ref().map(|e| EnchantmentId::new(entry(&e.id))),
    }
}

fn kind(t: Option<&str>) -> NodeKind {
    match t {
        Some("monster") => NodeKind::Monster,
        Some("elite") => NodeKind::Elite,
        Some("boss") => NodeKind::Boss,
        Some("rest_site") => NodeKind::Rest,
        Some("shop") => NodeKind::Shop,
        Some("treasure") => NodeKind::Treasure,
        Some("ancient") => NodeKind::Ancient,
        _ => NodeKind::Unknown,
    }
}

/// The card in `deck` that a game card refers to: the exact one if present,
/// else the same card with any upgrade or enchantment, preferring the
/// least-changed copy.
fn find(deck: &[DeckCard], want: &DeckCard) -> DeckCard {
    if deck.contains(want) {
        return want.clone();
    }
    let mut same: Vec<&DeckCard> = deck.iter().filter(|c| c.card == want.card).collect();
    same.sort_by_key(|c| (c.upgraded != want.upgraded, c.enchantment != want.enchantment));
    same.first().map_or_else(|| want.clone(), |c| (*c).clone())
}

/// What the dashboard knows about the act in progress that the save
/// doesn't say (from the companion mod).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ActHint {
    /// 0-based act index.
    pub index: Option<u8>,
    pub act: Option<String>,
    pub boss: Option<String>,
    pub second_boss: Option<String>,
}

/// A game run read into our log.
#[derive(Clone, Debug, PartialEq)]
pub struct GameRun {
    /// Identifies the run across saves: start time and seed.
    pub key: String,
    pub events: Vec<RunEvent>,
    /// Floors recorded.
    pub floors: usize,
    /// The run is over (a finished `.run`).
    pub ended: bool,
    /// Ids the catalog doesn't know (kept in the log as they are).
    pub unknown: Vec<String>,
}

/// Reads a run save or `.run` file. `None` if it isn't one, or names a
/// character the catalog doesn't know.
pub fn read(json: &str, catalog: &Catalog, hint: &ActHint) -> Option<GameRun> {
    let run: WRun = serde_json::from_str(json).ok()?;
    let player = run.players.first()?;
    let character = CharacterId::new(entry(player.character.as_deref()?));
    let start = RunEvent::start(catalog, &character, run.ascension)?;
    let key = format!("{}:{}", run.start_time.unwrap_or(0), run.seed.clone().unwrap_or_default());
    let mut events = vec![start];
    let mut state = apply(RunState::default(), &events[0]);
    let mut unknown: Vec<String> = Vec::new();
    let mut note = |id: String, known: bool| {
        if !known && !unknown.contains(&id) {
            unknown.push(id);
        }
    };
    let push = |events: &mut Vec<RunEvent>, state: &mut RunState, e: RunEvent| {
        *state = apply(std::mem::take(state), &e);
        events.push(e);
    };

    let mut floors = 0;
    // The act in progress may have no floors yet: the hint says it began.
    let acts = run.map_point_history.len().max(hint.index.map_or(0, |i| i as usize + 1));
    for i in 0..acts {
        let points: &[WPoint] = run.map_point_history.get(i).map_or(&[], Vec::as_slice);
        let current = hint.index.is_some_and(|h| h as usize == i);
        let act = run
            .acts
            .get(i)
            .map(|a| entry(a).to_string())
            .or_else(|| hint.act.as_deref().filter(|_| current).map(|a| entry(a).to_string()));
        let Some(act) = act else { continue };
        let fought: Vec<EncounterId> = points
            .iter()
            .filter(|p| p.map_point_type.as_deref() == Some("boss"))
            .filter_map(|p| p.rooms.first())
            .map(|r| EncounterId::new(entry(&r.model_id)))
            .collect();
        let hinted = |b: &Option<String>| b.as_deref().filter(|_| current).map(|b| EncounterId::new(entry(b)));
        let boss = fought.first().cloned().or_else(|| hinted(&hint.boss));
        let second = fought.get(1).cloned().or_else(|| hinted(&hint.second_boss));
        note(act.clone(), catalog.acts.contains_key(&ActId::new(&act)));
        push(&mut events, &mut state, RunEvent::EnterAct { act: ActId::new(&act), index: i as u8, boss });
        if let Some(s) = second {
            push(&mut events, &mut state, RunEvent::SetSecondBoss(s));
        }

        for p in points {
            let stats = p
                .player_stats
                .iter()
                .find(|s| s.player_id.is_some() && s.player_id == player.id)
                .or(p.player_stats.first());
            let room = p.rooms.first().map(|r| r.model_id.as_str()).unwrap_or("");
            let (encounter, event) = match room.split_once('.') {
                Some(("ENCOUNTER", e)) => (Some(EncounterId::new(e)), None),
                Some(("EVENT", e)) => (None, Some(EventId::new(e))),
                _ => (None, None),
            };
            let changes = stats.map(|s| changes(s, &state, catalog, &mut note)).unwrap_or_default();
            let report = NodeReport { kind: kind(p.map_point_type.as_deref()), encounter, event, changes };
            push(&mut events, &mut state, RunEvent::Node(report));
            floors += 1;
        }
    }

    // Whatever the per-floor record doesn't itemise, set straight from the
    // game's own totals.
    let fix = reconcile(&state, player);
    if !fix.is_empty() {
        push(&mut events, &mut state, RunEvent::Adjust(fix));
    }

    let outcome = match (run.win, run.was_abandoned) {
        (Some(true), _) => Some(Outcome::Victory),
        (_, Some(true)) => Some(Outcome::Abandoned),
        (Some(false), _) => Some(Outcome::Died { encounter: none_id(&run.killed_by_encounter).map(EncounterId::new) }),
        _ => None,
    };
    let ended = outcome.is_some();
    if let Some(outcome) = outcome {
        push(&mut events, &mut state, RunEvent::End { outcome, at: None });
    }
    Some(GameRun { key, events, floors, ended, unknown })
}

/// One floor's changes, naming cards as they sit in the deck now.
fn changes(s: &WStats, state: &RunState, catalog: &Catalog, note: &mut impl FnMut(String, bool)) -> Vec<Change> {
    let mut deck = state.deck.clone();
    let mut out = Vec::new();
    let take = |deck: &mut Vec<DeckCard>, c: &DeckCard| -> DeckCard {
        let found = find(deck, c);
        if let Some(i) = deck.iter().position(|d| *d == found) {
            deck.remove(i);
        }
        found
    };
    for c in &s.cards_removed {
        let card = take(&mut deck, &deck_card(c));
        out.push(Change::RemoveCard(card));
    }
    for t in &s.cards_transformed {
        let from = take(&mut deck, &deck_card(&t.original_card));
        let to = deck_card(&t.final_card);
        deck.push(to.clone());
        out.push(Change::Transform { from, to });
    }
    for id in &s.upgraded_cards {
        let want = DeckCard::new(CardId::new(entry(id)));
        let card = take(&mut deck, &want);
        deck.push(DeckCard { upgraded: true, ..card.clone() });
        out.push(Change::UpgradeCard(card));
    }
    for e in &s.cards_enchanted {
        let enchantment = EnchantmentId::new(entry(&e.enchantment));
        let want = DeckCard { enchantment: None, ..deck_card(&e.card) };
        let card = take(&mut deck, &want);
        deck.push(DeckCard { enchantment: Some(enchantment.clone()), ..card.clone() });
        out.push(Change::Enchant { card, enchantment });
    }
    for c in &s.cards_gained {
        let card = deck_card(c);
        note(c.id.clone(), catalog.cards.contains_key(&card.card));
        deck.push(card.clone());
        out.push(Change::AddCard(card));
    }
    for r in &s.relics_removed {
        out.push(Change::RemoveRelic(RelicId::new(entry(r))));
    }
    for r in s.relic_choices.iter().filter(|c| c.was_picked) {
        let id = RelicId::new(entry(&r.choice));
        note(r.choice.clone(), catalog.relics.contains_key(&id));
        out.push(Change::AddRelic(id));
    }
    for p in s.potion_used.iter().chain(&s.potion_discarded) {
        out.push(Change::RemovePotion(PotionId::new(entry(p))));
    }
    for p in s.potion_choices.iter().filter(|c| c.was_picked) {
        out.push(Change::AddPotion(PotionId::new(entry(&p.choice))));
    }
    if let Some(m) = s.max_hp.filter(|m| *m != state.max_hp) {
        out.push(Change::SetMaxHp(m));
    }
    if let Some(hp) = s.current_hp {
        out.push(Change::SetHp(hp));
    }
    if let Some(g) = s.current_gold.filter(|g| *g != state.gold) {
        out.push(Change::SetGold(g));
    }
    out
}

/// Changes that bring our deck, relics, potions and potion slots to the
/// game's own totals. Empty when they already agree.
fn reconcile(state: &RunState, player: &WPlayer) -> Vec<Change> {
    let mut out = Vec::new();
    // Multiset differences, by value.
    fn diff<T: Clone + PartialEq>(ours: &[T], theirs: &[T]) -> (Vec<T>, Vec<T>) {
        let mut extra: Vec<T> = ours.to_vec();
        let mut missing = Vec::new();
        for t in theirs {
            match extra.iter().position(|x| x == t) {
                Some(i) => {
                    extra.remove(i);
                }
                None => missing.push(t.clone()),
            }
        }
        (extra, missing)
    }
    // An empty list in the save means "not recorded", not "none".
    if !player.deck.is_empty() {
        let theirs: Vec<DeckCard> = player.deck.iter().map(deck_card).collect();
        let (extra, missing) = diff(&state.deck, &theirs);
        out.extend(extra.into_iter().map(Change::RemoveCard));
        out.extend(missing.into_iter().map(Change::AddCard));
    }
    if !player.relics.is_empty() {
        let theirs: Vec<RelicId> = player.relics.iter().map(|r| RelicId::new(entry(&r.id))).collect();
        let (extra, missing) = diff(&state.relics, &theirs);
        out.extend(extra.into_iter().map(Change::RemoveRelic));
        out.extend(missing.into_iter().map(Change::AddRelic));
    }
    let theirs: Vec<PotionId> = player.potions.iter().map(|p| PotionId::new(entry(&p.id))).collect();
    let (extra, missing) = diff(&state.potions, &theirs);
    out.extend(extra.into_iter().map(Change::RemovePotion));
    out.extend(missing.into_iter().map(Change::AddPotion));
    if let Some(n) = player.max_potion_slot_count.filter(|n| *n != state.potion_slots) {
        out.push(Change::SetPotionSlots(n));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn catalog() -> Catalog {
        Catalog::from_layers([crate::bundled_pack()])
    }

    #[test]
    fn ids_and_rooms_read_as_ours() {
        assert_eq!(entry("CARD.SNAP"), "SNAP");
        assert_eq!(none_id(&Some("NONE.NONE".into())), None);
        assert_eq!(kind(Some("rest_site")), NodeKind::Rest);
        let c = WCard {
            id: "CARD.BASH".into(),
            current_upgrade_level: Some(1),
            enchantment: Some(WId { id: "ENCHANTMENT.SHARP".into() }),
        };
        let d = deck_card(&c);
        assert_eq!((d.card.as_str(), d.upgraded), ("BASH", true));
        assert_eq!(d.enchantment.as_ref().map(|e| e.as_str()), Some("SHARP"));
    }

    #[test]
    fn a_finished_run_replays_to_the_games_own_totals() {
        let cat = catalog();
        let json = include_str!("../tests/fixtures/finished.run");
        let run = read(json, &cat, &ActHint::default()).expect("reads");
        assert!(run.ended);
        assert!(run.floors > 40, "every floor recorded: {}", run.floors);
        let state = spire_core::run::replay(&run.events);
        let wire: WRun = serde_json::from_str(json).unwrap();
        let p = &wire.players[0];
        let mut ours: Vec<String> =
            state.deck.iter().map(|c| format!("{}{}", c.card.as_str(), if c.upgraded { "+" } else { "" })).collect();
        let mut theirs: Vec<String> = p
            .deck
            .iter()
            .map(|c| format!("{}{}", entry(&c.id), if c.current_upgrade_level.unwrap_or(0) > 0 { "+" } else { "" }))
            .collect();
        ours.sort();
        theirs.sort();
        assert_eq!(ours, theirs);
        assert_eq!(state.relics.len(), p.relics.len());
        assert_eq!(state.outcome, Some(Outcome::Victory));
        assert_eq!(state.act_index, 2);
        // The per-floor record itself accounts for nearly everything: the
        // final correction is small.
        let correction = run.events.iter().find_map(|e| match e {
            RunEvent::Adjust(c) => Some(c.len()),
            _ => None,
        });
        assert!(correction.unwrap_or(0) <= 4, "correction: {correction:?}");
    }

    #[test]
    fn the_act_in_progress_takes_its_boss_from_the_hint() {
        let cat = catalog();
        let act = cat.acts.values().min_by_key(|a| a.index).unwrap();
        let character = cat.characters.keys().next().unwrap();
        let json = format!(
            r#"{{"acts":[],"ascension":2,"start_time":5,"seed":"S","players":[{{"id":1,"character":"CHARACTER.{}"}}],"map_point_history":[]}}"#,
            character.as_str()
        );
        let hint = ActHint {
            index: Some(0),
            act: Some(format!("ACT.{}", act.id.as_str())),
            boss: act.bosses.first().map(|b| format!("ENCOUNTER.{}", b.as_str())),
            second_boss: None,
        };
        let run = read(&json, &cat, &hint).expect("reads");
        assert_eq!(run.key, "5:S");
        assert!(!run.ended);
        let state = spire_core::run::replay(&run.events);
        assert_eq!(state.act.as_ref(), Some(&act.id));
        assert_eq!(state.boss, act.bosses.first().cloned());
        assert_eq!(state.ascension, 2);
    }
}
