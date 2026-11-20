//! `?` rooms from the Spire Codex run export: which event relics each event
//! gave, how often runs meet each event in each act, what taking each event
//! option did, and how often a `?` room turns into a treasure room, with the
//! rarity of chest relics per act.
//!
//! What an option does is read from the floor it was taken on: cards
//! removed, transformed, upgraded, enchanted or added, curses, relics,
//! potions, HP healed and lost, max HP, gold. It's credited to the first
//! option taken, so later screens count toward it (Slippery Bridge's Hold On
//! ends in a removal too).
//!
//! The event endpoint names only some of the relics an event gives (Trash
//! Heap's "random forgotten Relic" names none). The runs record every relic
//! picked up on an event's floor, so the event relics seen there fill the
//! gap. Relics from the general pools ("Obtain a random Relic") are left out:
//! no one can fish an event for one of those. How often an event comes up is
//! measured per act (Overgrowth and Underdocks apart, though both are act 1).
//!
//! Pure: decompressed JSONL text in, enriched [`DataPack`] out.

use std::collections::{BTreeMap, BTreeSet};

use spire_core::DataPack;
use spire_core::catalog::{CardType, EffectKind, EventEffect, EventRelic, Rarity};
use spire_core::ids::{ActId, RelicId};

use crate::run_file::{WRun, WStats, entry};

/// An event relic seen fewer times than this from an event is left out
/// (stray or modded data).
const MIN_GIVEN: u32 = 2;
/// Fewer runs than this reaching an act and its shares stay unmeasured.
const MIN_RUNS: u32 = 100;
/// Options taken fewer times than this keep no effects.
const MIN_PICKS: u32 = 20;
/// Effects seen in fewer of an option's picks than this are left out (a
/// relic's heal on the floor, a rare follow-up).
const MIN_RATE: f32 = 0.1;

/// What one event option did across the runs that took it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Tally {
    pub picks: u32,
    /// Picks in which each effect happened, and its total amount.
    pub effects: BTreeMap<EffectKind, (u32, f64)>,
    /// Picks in which a curse was removed or transformed.
    pub uncursed: u32,
}

/// Events met in community runs.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Sightings {
    /// Runs that played each act, by act id.
    pub runs: BTreeMap<String, u32>,
    /// Runs that met each event, per act: `(event, act)`.
    pub met: BTreeMap<(String, String), u32>,
    /// Times each event gave each relic.
    pub given: BTreeMap<(String, String), u32>,
    /// Runs in which a `?` room turned into a treasure room, per act.
    pub unknown_treasure: BTreeMap<String, u32>,
    /// Relics taken from treasure rooms (any), per act: `(act, relic)`.
    pub chest: BTreeMap<(String, String), u32>,
    /// What taking each event option did: `(event, option)`.
    pub options: BTreeMap<(String, String), Tally>,
}

/// What a floor's stats say happened, by effect. `curses` are the curse
/// card ids, to tell a curse from a card gained.
fn effects(st: &WStats, curses: &BTreeSet<String>) -> Vec<(EffectKind, f64)> {
    let n = |v: usize| v as f64;
    let curse = st.cards_gained.iter().filter(|c| curses.contains(entry(&c.id))).count();
    let picked = |c: &[crate::run_file::WChoice]| c.iter().filter(|c| c.was_picked).count();
    let num = |v: Option<f64>| v.unwrap_or(0.0).max(0.0);
    let all = [
        (EffectKind::Remove, n(st.cards_removed.len())),
        (EffectKind::Transform, n(st.cards_transformed.len())),
        (EffectKind::Upgrade, n(st.upgraded_cards.len())),
        (EffectKind::Enchant, n(st.cards_enchanted.len())),
        (EffectKind::Card, n(st.cards_gained.len() - curse)),
        (EffectKind::Curse, n(curse)),
        (EffectKind::Relic, n(picked(&st.relic_choices))),
        (EffectKind::Potion, n(picked(&st.potion_choices))),
        (EffectKind::Heal, num(st.hp_healed)),
        (EffectKind::MaxHp, num(st.max_hp_gained)),
        (EffectKind::MaxHpLoss, num(st.max_hp_lost)),
        (EffectKind::Damage, f64::from(st.damage_taken.unwrap_or(0))),
        (EffectKind::Gold, num(st.gold_gained)),
        (EffectKind::GoldCost, f64::from(st.gold_spent.unwrap_or(0).max(0)) + num(st.gold_lost)),
    ];
    all.into_iter().filter(|(_, v)| *v > 0.0).collect()
}

/// Reads single-player, unmodified runs. Bad lines are skipped. `curses`
/// are the curse card ids.
pub fn sightings(pages: &[String], curses: &BTreeSet<String>) -> Sightings {
    let mut s = Sightings::default();
    let runs = pages
        .iter()
        .flat_map(|p| p.lines())
        .filter_map(|line| serde_json::from_str::<WRun>(line).ok())
        .filter(|run| run.players.len() == 1 && run.modifiers.as_ref().is_none_or(is_empty));
    for run in runs {
        for (i, points) in run.map_point_history.iter().enumerate() {
            let Some(act) = run.acts.get(i).map(|a| entry(&a.id).to_string()) else { continue };
            if points.is_empty() {
                continue;
            }
            *s.runs.entry(act.clone()).or_default() += 1;
            let mut met: BTreeSet<&str> = BTreeSet::new();
            let mut treasure = false;
            for p in points {
                let picked = || p.player_stats.iter().flat_map(|st| &st.relic_choices).filter(|c| c.was_picked);
                if p.rooms.iter().any(|r| r.room_type.as_deref() == Some("treasure")) {
                    treasure |= p.map_point_type.as_deref() == Some("unknown");
                    for c in picked() {
                        *s.chest.entry((act.clone(), entry(&c.choice).to_string())).or_default() += 1;
                    }
                }
            }
            if treasure {
                *s.unknown_treasure.entry(act.clone()).or_default() += 1;
            }
            for p in points.iter().filter(|p| p.map_point_type.as_deref() != Some("ancient")) {
                for room in p.rooms.iter().filter(|r| r.model_id.starts_with("EVENT.")) {
                    let event = entry(&room.model_id);
                    met.insert(event);
                    let picked = p.player_stats.iter().flat_map(|st| &st.relic_choices).filter(|c| c.was_picked);
                    for c in picked {
                        *s.given.entry((event.to_string(), entry(&c.choice).to_string())).or_default() += 1;
                    }
                    for st in &p.player_stats {
                        let Some(option) = st.event_choices.iter().find_map(|c| c.option()) else { continue };
                        let t = s.options.entry((event.to_string(), option.to_string())).or_default();
                        t.picks += 1;
                        let taken_out = st
                            .cards_removed
                            .iter()
                            .chain(st.cards_transformed.iter().map(|c| &c.original_card))
                            .any(|c| curses.contains(entry(&c.id)));
                        t.uncursed += u32::from(taken_out);
                        for (kind, amount) in effects(st, curses) {
                            let e = t.effects.entry(kind).or_default();
                            e.0 += 1;
                            e.1 += amount;
                        }
                    }
                }
            }
            for e in met {
                *s.met.entry((e.to_string(), act.clone())).or_default() += 1;
            }
        }
    }
    s
}

/// An option's effects, most frequent first; none when too few runs took it.
fn option_effects(t: Option<&Tally>) -> Vec<EventEffect> {
    let Some(t) = t.filter(|t| t.picks >= MIN_PICKS) else { return Vec::new() };
    let mut out: Vec<EventEffect> = t
        .effects
        .iter()
        .map(|(kind, (hits, total))| EventEffect {
            kind: *kind,
            rate: *hits as f32 / t.picks as f32,
            amount: (*total / f64::from((*hits).max(1))) as f32,
        })
        .filter(|e| e.rate >= MIN_RATE)
        .collect();
    // Seen taking out a curse: it can, as often as it removes or transforms.
    let takes = out.iter().filter(|e| matches!(e.kind, EffectKind::Remove | EffectKind::Transform));
    if let Some(rate) = takes.map(|e| e.rate).reduce(f32::max).filter(|_| t.uncursed > 0) {
        out.push(EventEffect { kind: EffectKind::Uncurse, rate, amount: 1.0 });
    }
    out.sort_by(|a, b| b.rate.total_cmp(&a.rate));
    out
}

/// The curse card ids in a pack.
pub fn curses(pack: &DataPack) -> BTreeSet<String> {
    pack.cards.iter().filter(|c| c.card_type == CardType::Curse).map(|c| c.id.0.clone()).collect()
}

fn is_empty(v: &serde_json::Value) -> bool {
    match v {
        serde_json::Value::Null => true,
        serde_json::Value::Array(a) => a.is_empty(),
        serde_json::Value::Object(o) => o.is_empty(),
        _ => false,
    }
}

/// "Choose 1 of 3": the most an option lets you choose from.
fn choose_of(text: &str) -> Option<u32> {
    let lower = text.to_lowercase();
    lower.split("choose 1 of ").skip(1).filter_map(|rest| rest.split_whitespace().next()?.parse().ok()).max()
}

/// Adds the event relics seen given, the share of runs meeting each event
/// per act, and per act how often a `?` room is a treasure room and the
/// rarity of chest relics. Returns how many events gained a relic.
pub fn enrich(pack: &mut DataPack, s: &Sightings) -> usize {
    let rarity: BTreeMap<&str, Rarity> = pack.relics.iter().map(|r| (r.id.0.as_str(), r.rarity)).collect();
    for act in &mut pack.acts {
        let id = act.id.0.as_str();
        let Some(runs) = s.runs.get(id).filter(|n| **n >= MIN_RUNS) else { continue };
        act.unknown_treasure = Some(s.unknown_treasure.get(id).copied().unwrap_or(0) as f32 / *runs as f32);
        let mut by_rarity: BTreeMap<Rarity, u32> = BTreeMap::new();
        for ((a, relic), n) in &s.chest {
            if let Some(r) = rarity.get(relic.as_str()).filter(|_| a == id) {
                *by_rarity.entry(*r).or_default() += n;
            }
        }
        let total: u32 = by_rarity.values().sum();
        if total >= MIN_RUNS {
            act.chest_rarity = by_rarity.into_iter().map(|(r, n)| (r, n as f32 / total as f32)).collect();
        }
    }
    let event_relics: BTreeSet<&str> =
        pack.relics.iter().filter(|r| r.rarity == Rarity::Event).map(|r| r.id.0.as_str()).collect();
    let mut gained = 0;
    for event in &mut pack.events {
        let id = event.id.0.as_str();
        for option in &mut event.options {
            option.effects = option_effects(s.options.get(&(id.to_string(), option.id.clone())));
        }
        event.met = s
            .runs
            .iter()
            .filter(|(_, runs)| **runs >= MIN_RUNS)
            .map(|(act, runs)| {
                let met = s.met.get(&(id.to_string(), act.clone())).copied().unwrap_or(0);
                (ActId::new(act), met as f32 / *runs as f32)
            })
            .filter(|(_, share)| *share > 0.0)
            .collect();

        // Seen given, not named by any option: given at random among them,
        // unless an option lets you choose from that many.
        let seen: Vec<&str> = s
            .given
            .iter()
            .filter(|((e, r), n)| e == id && **n >= MIN_GIVEN && event_relics.contains(r.as_str()))
            .map(|((_, r), _)| r.as_str())
            .filter(|r| !event.relics.iter().any(|x| x.relic.0 == *r))
            .collect();
        if seen.is_empty() {
            continue;
        }
        gained += 1;
        let n = seen.len() as u32;
        let chooser = event.options.iter().find(|o| choose_of(&o.text).is_some_and(|k| k >= n));
        for r in seen {
            event.relics.push(EventRelic {
                relic: RelicId(r.to_string()),
                option: chooser.map(|o| o.title.clone()),
                one_of: if chooser.is_some() { 1 } else { n },
            });
        }
    }
    gained
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(act_events: &[&[(&str, &[&str])]]) -> String {
        let acts: Vec<serde_json::Value> = act_events
            .iter()
            .map(|points| {
                let points: Vec<serde_json::Value> = points
                    .iter()
                    .map(|(event, relics)| {
                        let choices: Vec<_> = relics
                            .iter()
                            .map(|r| serde_json::json!({"choice": format!("RELIC.{r}"), "was_picked": true}))
                            .collect();
                        serde_json::json!({
                            "map_point_type": "unknown",
                            "player_stats": [{"relic_choices": choices}],
                            "rooms": [{"model_id": format!("EVENT.{event}"), "room_type": "event"}]
                        })
                    })
                    .collect();
                serde_json::Value::Array(points)
            })
            .collect();
        let ids: Vec<String> =
            ["ACT.ONE", "ACT.TWO", "ACT.THREE"][..acts.len()].iter().map(|s| s.to_string()).collect();
        serde_json::json!({"players": [{}], "acts": ids, "map_point_history": acts}).to_string()
    }

    #[test]
    fn counts_runs_meeting_events_and_the_relics_they_gave() {
        let page = [
            run(&[&[("TRASH_HEAP", &["HAND_DRILL"])], &[("DOLL_ROOM", &["MR_STRUGGLES"])]]),
            run(&[&[("TRASH_HEAP", &["THE_BOOT"]), ("TRASH_HEAP", &[])]]),
            run(&[&[("OTHER", &[])]]),
        ]
        .join("\n");
        let s = sightings(&[page], &BTreeSet::new());
        assert_eq!(s.runs, BTreeMap::from([("ONE".to_string(), 3), ("TWO".to_string(), 1)]));
        // Met twice in one act counts the run once.
        assert_eq!(s.met[&("TRASH_HEAP".to_string(), "ONE".to_string())], 2);
        assert_eq!(s.given[&("TRASH_HEAP".to_string(), "HAND_DRILL".to_string())], 1);
        assert_eq!(s.given[&("DOLL_ROOM".to_string(), "MR_STRUGGLES".to_string())], 1);
    }

    /// One run meeting Slippery Bridge, taking `options` in turn, with the
    /// floor's stats.
    fn bridge(options: &[&str], stats: serde_json::Value) -> String {
        let choices: Vec<_> = options
            .iter()
            .map(|o| serde_json::json!({"title": {"key": format!("SLIPPERY_BRIDGE.pages.INITIAL.options.{o}.title"), "table": "events"}}))
            .collect();
        let mut st = stats;
        st["event_choices"] = serde_json::Value::Array(choices);
        serde_json::json!({
            "players": [{}],
            "acts": ["ACT.ONE"],
            "map_point_history": [[{
                "map_point_type": "unknown",
                "player_stats": [st],
                "rooms": [{"model_id": "EVENT.SLIPPERY_BRIDGE", "room_type": "event"}]
            }]]
        })
        .to_string()
    }

    #[test]
    fn credits_what_a_floor_did_to_the_first_option_taken() {
        let removed = serde_json::json!({"cards_removed": [{"id": "CARD.STRIKE_IRONCLAD"}]});
        let page = [
            bridge(&["OVERCOME"], removed.clone()),
            bridge(&["HOLD_ON_0", "OVERCOME"], {
                let mut v = removed.clone();
                v["damage_taken"] = serde_json::json!(3);
                v
            }),
            bridge(&["HOLD_ON_0", "OVERCOME"], {
                let mut v = removed;
                v["damage_taken"] = serde_json::json!(7);
                v["cards_gained"] = serde_json::json!([{"id": "CARD.DEBT"}, {"id": "CARD.BASH"}]);
                v
            }),
        ]
        .join("\n");
        let s = sightings(&[page], &BTreeSet::from(["DEBT".to_string()]));
        let overcome = &s.options[&("SLIPPERY_BRIDGE".to_string(), "OVERCOME".to_string())];
        assert_eq!(overcome.picks, 1);
        assert_eq!(overcome.effects[&EffectKind::Remove], (1, 1.0));
        let hold = &s.options[&("SLIPPERY_BRIDGE".to_string(), "HOLD_ON_0".to_string())];
        assert_eq!(hold.picks, 2);
        assert_eq!(hold.effects[&EffectKind::Remove], (2, 2.0));
        assert_eq!(hold.effects[&EffectKind::Damage], (2, 10.0));
        assert_eq!(hold.effects[&EffectKind::Curse], (1, 1.0), "Debt is a curse");
        assert_eq!(hold.effects[&EffectKind::Card], (1, 1.0), "Bash is a card");
        assert_eq!(hold.uncursed, 0, "a Strike was removed, not a curse");
    }

    #[test]
    fn keeps_an_options_frequent_effects_once_enough_runs_took_it() {
        let mut t = Tally { picks: MIN_PICKS, ..Default::default() };
        t.effects.insert(EffectKind::Remove, (MIN_PICKS, f64::from(MIN_PICKS)));
        t.effects.insert(EffectKind::Damage, (MIN_PICKS / 2, f64::from(MIN_PICKS / 2) * 8.0));
        t.effects.insert(EffectKind::Heal, (1, 7.0));
        let e = option_effects(Some(&t));
        assert_eq!(e.len(), 2, "a heal in 1 pick of 20 is noise");
        assert_eq!((e[0].kind, e[0].rate, e[0].amount), (EffectKind::Remove, 1.0, 1.0));
        assert_eq!((e[1].kind, e[1].rate, e[1].amount), (EffectKind::Damage, 0.5, 8.0));
        assert!(!e.iter().any(|e| e.kind == EffectKind::Uncurse), "never seen taking out a curse");
        let curse = option_effects(Some(&Tally { uncursed: 1, ..t.clone() }));
        let u = curse.iter().find(|e| e.kind == EffectKind::Uncurse).expect("can take out a curse");
        assert_eq!(u.rate, 1.0, "as often as it removes");
        assert!(option_effects(Some(&Tally { picks: MIN_PICKS - 1, ..t })).is_empty(), "too few picks");
    }

    #[test]
    fn reads_how_many_an_option_lets_you_choose_from() {
        assert_eq!(choose_of("Lose 15 HP. Choose 1 of 3 Doll Relics."), Some(3));
        assert_eq!(choose_of("Obtain a random Doll Relic."), None);
    }
}
