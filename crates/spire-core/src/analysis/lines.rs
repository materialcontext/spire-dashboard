//! Card lines and card-reward suggestions: what to look for in card rewards
//! to beat this act's boss, from the cards the deck already has.
//!
//! **Lines** come from community runs ([`measure`], at data refresh). Each
//! run's deck is taken at every boss fight it reached, with whether it won.
//! Per character and act, a line is a set of two or three cards whose decks
//! beat the boss more often together than with any smaller part of the set,
//! and more often than the character's decks overall. Win rates are shrunk
//! toward the act's rate ([`PRIOR`] fights' worth), so a line seen in few
//! decks can't top the list on luck. On held-out runs, mined lines beat the
//! baseline by a few points on average: a nudge, not a guarantee.
//!
//! **Suggestions** ([`suggest`]) are at most a few, best first:
//! 1. lines within reach: the deck holds part of the line and the missing
//!    card is offered in this act often enough to hope for; ranked by the
//!    line's rate against the act's boss when known;
//! 2. shortfall fillers: for the roles the deck lacks against the boss (its
//!    outlook's limits, then the boss check's gaps), the class cards that
//!    fill them, ranked by how much they lift high-skill players' win rate,
//!    and kept only when the simulator shows the deck doing better against
//!    the boss with the card. A card that needs a partner the deck lacks
//!    (Shivs, orbs) comes with its partner.

use std::collections::{BTreeMap, HashMap};

use crate::analysis::deck::classify_run;
use crate::analysis::sim::simulate;
use crate::analysis::survival::{self, Outlook, Verdict};
use crate::analysis::{Coverage, ThreatPreview};
use crate::catalog::{ActLines, CardLift, CardLine, Catalog, Rarity, Record};
use crate::ids::{CardId, CharacterId, EncounterId};
use crate::playbook::{Playbook, Role};
use crate::run::{Change, DeckCard, RunEvent, RunState, apply};

/// Fights' worth of the act's own rate a line's rate is shrunk toward.
pub const PRIOR: f32 = 20.0;
/// Fewer decks than this with a line and it isn't measured.
pub const MIN_DECKS: u32 = 30;
/// A line beats each of its smaller sets by this much (shrunk rates).
const MIN_APART: f32 = 0.03;
/// And the act's decks overall by this much.
const MIN_LIFT: f32 = 0.05;
/// Lines kept per character and act.
const KEEP: usize = 40;
/// A boss's record for a line is kept from this many fights.
const MIN_BOSS_FIGHTS: u32 = 10;

/// One community deck at a boss fight.
#[derive(Clone, Debug, PartialEq)]
pub struct BossFight {
    pub character: CharacterId,
    /// 0-based act.
    pub act: u8,
    pub boss: Option<EncounterId>,
    /// Distinct reward cards in the deck (no starters, curses or statuses).
    pub cards: Vec<CardId>,
    pub won: bool,
}

/// Card rewards offered in community runs: per character and act, the runs
/// that played it, and how many of them were offered each card.
pub type Offers = BTreeMap<(CharacterId, u8), (u32, BTreeMap<CardId, u32>)>;

/// A record's win rate shrunk toward `base`.
pub fn shrunk(r: Record, base: f32) -> f32 {
    (r.wins as f32 + PRIOR * base) / (r.fights as f32 + PRIOR)
}

fn rate(r: Record) -> f32 {
    r.wins as f32 / r.fights.max(1) as f32
}

fn add(r: &mut Record, won: bool) {
    r.fights += 1;
    r.wins += u32::from(won);
}

/// Card lines per character and act, from community boss fights.
pub fn measure(fights: &[BossFight], offers: &Offers) -> Vec<ActLines> {
    let mut groups: BTreeMap<(CharacterId, u8), Vec<&BossFight>> = BTreeMap::new();
    for f in fights {
        groups.entry((f.character.clone(), f.act)).or_default().push(f);
    }
    groups.into_iter().filter_map(|((character, act), fs)| measure_group(character, act, &fs, offers)).collect()
}

fn measure_group(character: CharacterId, act: u8, fights: &[&BossFight], offers: &Offers) -> Option<ActLines> {
    let mut all = Record::default();
    let mut bosses: BTreeMap<EncounterId, Record> = BTreeMap::new();
    let mut singles: HashMap<&CardId, Record> = HashMap::new();
    for f in fights {
        add(&mut all, f.won);
        if let Some(b) = &f.boss {
            add(bosses.entry(b.clone()).or_default(), f.won);
        }
        for c in &f.cards {
            add(singles.entry(c).or_default(), f.won);
        }
    }
    if all.fights < MIN_DECKS {
        return None;
    }
    let base = rate(all);
    // Sets are counted among cards common enough to make one.
    let mut index: Vec<&CardId> = singles.iter().filter(|(_, r)| r.fights >= MIN_DECKS).map(|(c, _)| *c).collect();
    index.sort();
    let pos: HashMap<&CardId, u16> = index.iter().enumerate().map(|(i, c)| (*c, i as u16)).collect();
    let mut sets: HashMap<Vec<u16>, Record> = HashMap::new();
    for f in fights {
        let mut have: Vec<u16> = f.cards.iter().filter_map(|c| pos.get(c).copied()).collect();
        have.sort_unstable();
        for (i, a) in have.iter().enumerate() {
            for (j, b) in have.iter().enumerate().skip(i + 1) {
                add(sets.entry(vec![*a, *b]).or_default(), f.won);
                for c in &have[j + 1..] {
                    add(sets.entry(vec![*a, *b, *c]).or_default(), f.won);
                }
            }
        }
    }
    let single = |i: u16| singles[index[i as usize]];
    let record_of = |s: &[u16]| if s.len() == 1 { single(s[0]) } else { sets.get(s).copied().unwrap_or_default() };
    let mut lines: Vec<(f32, Vec<u16>, Record, f32)> = sets
        .iter()
        .filter(|(_, r)| r.fights >= MIN_DECKS)
        .filter_map(|(s, r)| {
            let apart = (0..s.len())
                .map(|skip| {
                    let part: Vec<u16> = s.iter().enumerate().filter(|(k, _)| *k != skip).map(|(_, c)| *c).collect();
                    shrunk(record_of(&part), base)
                })
                .fold(0.0, f32::max);
            let together = shrunk(*r, base);
            (together - apart >= MIN_APART && together - base >= MIN_LIFT).then(|| (together, s.clone(), *r, apart))
        })
        .collect();
    lines.sort_by(|a, b| b.0.total_cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
    lines.truncate(KEEP);

    let lines: Vec<CardLine> = lines
        .into_iter()
        .map(|(_, s, record, apart)| {
            let cards: Vec<CardId> = s.iter().map(|i| index[*i as usize].clone()).collect();
            let mut by_boss: BTreeMap<EncounterId, Record> = BTreeMap::new();
            for f in fights.iter().filter(|f| cards.iter().all(|c| f.cards.contains(c))) {
                if let Some(b) = &f.boss {
                    add(by_boss.entry(b.clone()).or_default(), f.won);
                }
            }
            by_boss.retain(|_, r| r.fights >= MIN_BOSS_FIGHTS);
            CardLine { cards, record, apart, bosses: by_boss }
        })
        .collect();
    let offered = offers
        .get(&(character.clone(), act))
        .filter(|(runs, _)| *runs > 0)
        .map(|(runs, cards)| cards.iter().map(|(c, n)| (c.clone(), *n as f32 / *runs as f32)).collect())
        .unwrap_or_default();
    Some(ActLines { character, act, all, bosses, offered, lines })
}

/// A suggestion for card rewards.
#[derive(Clone, Debug, PartialEq)]
pub struct Suggestion {
    /// Cards the deck has that it builds on (none for a filler).
    pub held: Vec<CardId>,
    /// What to look for: any one of these options completes it. An option
    /// is one card, or a card and the partner it needs. Lines on the same
    /// held cards that do about as well share one suggestion.
    pub options: Vec<Vec<CardId>>,
    /// Why, for the best option.
    pub why: Why,
    /// Share of community runs offered each card to look for, in this act.
    pub offered: Vec<(CardId, f32)>,
}

impl Suggestion {
    /// Every card to look for.
    pub fn missing(&self) -> impl Iterator<Item = &CardId> {
        self.options.iter().flatten()
    }
}

/// Why a suggestion is made.
#[derive(Clone, Debug, PartialEq)]
pub enum Why {
    /// Community decks beat this act's boss more often with these together.
    Line {
        /// Boss fights with the whole line, shrunk rate, and the rates of the
        /// character's decks overall and of the line's smaller parts.
        together: Record,
        rate: f32,
        base: f32,
        apart: f32,
        /// The same against the boss being faced, when fought often enough.
        boss: Option<(EncounterId, Record, f32)>,
    },
    /// Fills a role the deck lacks against the boss: the simulator's HP
    /// lost to it without and with the cards.
    Filler {
        role: Role,
        boss: EncounterId,
        hp_before: u32,
        hp_after: u32,
        verdict_before: Verdict,
        verdict_after: Verdict,
        /// High-skill players' win-rate lift for the suggested card.
        lift: Option<CardLift>,
    },
}

fn held(state: &RunState, card: &CardId) -> bool {
    state.deck.iter().any(|d| &d.card == card)
}

/// This act's lines for the run's character.
fn act_lines<'a>(state: &RunState, catalog: &'a Catalog) -> Option<&'a ActLines> {
    let character = state.character.as_ref()?;
    catalog.card_lines.iter().find(|l| &l.character == character && l.act == state.act_index)
}

/// Lines the deck holds part of, with the rest likely to be offered this
/// act; best first.
pub fn lines_within_reach(state: &RunState, catalog: &Catalog, playbook: &Playbook) -> Vec<Suggestion> {
    let lines = &playbook.picks;
    let Some(act) = act_lines(state, catalog) else { return Vec::new() };
    let base = rate(act.all);
    let boss = state.boss.as_ref();
    let mut found: Vec<(f32, f32, Suggestion)> = Vec::new();
    for line in &act.lines {
        let missing: Vec<&CardId> = line.cards.iter().filter(|c| !held(state, c)).collect();
        if missing.is_empty() || missing.len() > lines.max_missing as usize || missing.len() == line.cards.len() {
            continue;
        }
        let offered: Vec<(CardId, f32)> =
            missing.iter().map(|c| ((*c).clone(), act.offered.get(*c).copied().unwrap_or(0.0))).collect();
        let reach = offered.iter().map(|(_, s)| *s).fold(1.0, f32::min);
        if reach < lines.min_offered {
            continue;
        }
        let together = shrunk(line.record, base);
        // Against the boss being faced: the line's record there, shrunk
        // toward the line's own rate, next to that boss's base rate.
        let at_boss = boss.and_then(|b| {
            let r = *line.bosses.get(b)?;
            let boss_base = act.bosses.get(b).map_or(base, |all| shrunk(*all, base));
            Some((b.clone(), r, shrunk(r, together), boss_base))
        });
        let gain = match &at_boss {
            Some((_, _, rate, boss_base)) => rate - boss_base,
            None => together - base,
        };
        if gain < lines.min_gain {
            continue;
        }
        let have: Vec<CardId> = line.cards.iter().filter(|c| held(state, c)).cloned().collect();
        let option: Vec<CardId> = missing.into_iter().cloned().collect();
        let why = Why::Line {
            together: line.record,
            rate: together,
            base,
            apart: line.apart,
            boss: at_boss.map(|(b, r, rate, _)| (b, r, rate)),
        };
        found.push((gain * reach, gain, Suggestion { held: have, options: vec![option], why, offered }));
    }
    found.sort_by(|a, b| b.0.total_cmp(&a.0));
    // Lines on the same held cards that do about as well as the best of
    // them become options of one suggestion.
    let mut out: Vec<(f32, Suggestion)> = Vec::new();
    for (_, gain, s) in found {
        let joins = out.iter_mut().find(|(best, o)| {
            o.held == s.held && best - gain <= lines.similar && o.options.len() < lines.max_options as usize
        });
        match joins {
            Some((_, o)) => {
                for option in s.options {
                    if !o.options.contains(&option) {
                        o.options.push(option);
                    }
                }
                for (card, share) in s.offered {
                    if !o.offered.iter().any(|(x, _)| *x == card) {
                        o.offered.push((card, share));
                    }
                }
            }
            None => out.push((gain, s)),
        }
    }
    out.into_iter().map(|(_, s)| s).collect()
}

/// The roles the deck lacks against the boss: the outlook's limits first
/// (what the simulator found holding it back), then the boss check's gaps.
fn wanted_roles(boss: &ThreatPreview, outlook: &Outlook) -> Vec<Role> {
    let mut out: Vec<Role> = outlook.limits.iter().filter_map(|l| l.role).collect();
    out.extend(boss.checks.iter().filter(|c| c.coverage == Coverage::Missing).map(|c| c.demand.role));
    let mut seen = Vec::new();
    out.retain(|r| {
        let new = !seen.contains(r);
        seen.push(*r);
        new
    });
    out
}

/// Class cards a reward can offer this character.
fn reward_cards<'a>(state: &'a RunState, catalog: &'a Catalog) -> impl Iterator<Item = &'a crate::catalog::Card> + 'a {
    let color = state.character.as_ref().map(|c| c.0.to_lowercase()).unwrap_or_default();
    catalog
        .cards
        .values()
        .filter(move |c| c.color == color && matches!(c.rarity, Rarity::Common | Rarity::Uncommon | Rarity::Rare))
}

fn lift_of(catalog: &Catalog, card: &CardId) -> f32 {
    catalog.card_lift.get(card).map_or(f32::MIN, |l| l.lift)
}

/// The run with these cards added.
fn with_cards(state: &RunState, cards: &[CardId]) -> RunState {
    let changes = cards.iter().map(|c| Change::AddCard(DeckCard::new(c.clone()))).collect();
    apply(state.clone(), &RunEvent::Adjust(changes))
}

/// Cards (or a card and its partner) that fill a role the deck lacks
/// against the boss and do better against it in the simulator; best first.
pub fn fillers(
    state: &RunState,
    catalog: &Catalog,
    playbook: &Playbook,
    boss: &ThreatPreview,
    outlook: &Outlook,
) -> Vec<Suggestion> {
    let lines = &playbook.picks;
    let Some(enc) = catalog.encounters.get(&boss.encounter) else { return Vec::new() };
    let offered_in = act_lines(state, catalog).map(|a| &a.offered);
    let mut out: Vec<(u32, Suggestion)> = Vec::new();
    for role in wanted_roles(boss, outlook).into_iter().take(lines.filler_roles as usize) {
        let mut candidates: Vec<&crate::catalog::Card> = reward_cards(state, catalog)
            .filter(|c| !held(state, &c.id) && playbook.roles_of(c).contains(&role))
            .filter(|c| catalog.card_lift.get(&c.id).is_some_and(|l| l.lift > 0.0))
            .collect();
        candidates.sort_by(|a, b| lift_of(catalog, &b.id).total_cmp(&lift_of(catalog, &a.id)));
        let mut best: Option<(u32, Suggestion)> = None;
        for card in candidates.into_iter().take(lines.filler_candidates as usize) {
            // A card that does nothing without a partner comes with one.
            let mut cards = vec![card.id.clone()];
            let tried = with_cards(state, &cards);
            if let Some(partner) = partner_for(&tried, catalog, playbook, &card.id) {
                cards.push(partner);
            }
            let after_state = with_cards(state, &cards);
            if classify_run(&after_state, catalog, playbook).iter().any(|c| c.deck.card == card.id && c.unmet.is_some())
            {
                continue;
            }
            let sim = simulate(&after_state, catalog, &playbook.simulation);
            let Some(after) = survival::for_encounter(&sim, enc, catalog, &after_state, playbook) else { continue };
            let (before_hp, after_hp) = (outlook.lead.hp_loss, after.lead.hp_loss);
            let better_verdict = after.verdict < outlook.verdict;
            if before_hp < after_hp + lines.min_hp_saved && !better_verdict {
                continue;
            }
            let saved = before_hp.saturating_sub(after_hp) + if better_verdict { 100 } else { 0 };
            if best.as_ref().is_none_or(|(s, _)| saved > *s) {
                let offered = cards
                    .iter()
                    .map(|c| (c.clone(), offered_in.and_then(|o| o.get(c)).copied().unwrap_or(0.0)))
                    .collect();
                let why = Why::Filler {
                    role,
                    boss: boss.encounter.clone(),
                    hp_before: before_hp,
                    hp_after: after_hp,
                    verdict_before: outlook.verdict,
                    verdict_after: after.verdict,
                    lift: catalog.card_lift.get(&card.id).copied(),
                };
                best = Some((saved, Suggestion { held: Vec::new(), options: vec![cards], why, offered }));
            }
        }
        out.extend(best);
    }
    out.sort_by_key(|(saved, _)| std::cmp::Reverse(*saved));
    out.into_iter().map(|(_, s)| s).collect()
}

/// The best class card supplying what `card` needs, when the run lacks it.
fn partner_for(state: &RunState, catalog: &Catalog, playbook: &Playbook, card: &CardId) -> Option<CardId> {
    let cards = classify_run(state, catalog, playbook);
    let what = cards.iter().find(|c| &c.deck.card == card)?.unmet.clone()?;
    let req = playbook.simulation.requirements.iter().find(|r| r.what == what)?;
    let met_by = req.met_by.as_ref()?;
    reward_cards(state, catalog)
        .filter(|c| met_by.matches(c))
        .max_by(|a, b| lift_of(catalog, &a.id).total_cmp(&lift_of(catalog, &b.id)))
        .map(|c| c.id.clone())
}

/// What to look for in card rewards, at most `picks.max`: lines within
/// reach first, then fillers for what the deck lacks against the boss.
pub fn suggest(
    state: &RunState,
    catalog: &Catalog,
    playbook: &Playbook,
    boss: Option<(&ThreatPreview, &Outlook)>,
) -> Vec<Suggestion> {
    let max = playbook.picks.max as usize;
    let mut out: Vec<Suggestion> = lines_within_reach(state, catalog, playbook).into_iter().take(max).collect();
    if out.len() < max
        && let Some((preview, outlook)) = boss
    {
        for f in fillers(state, catalog, playbook, preview, outlook) {
            if out.len() >= max {
                break;
            }
            let repeats = f.missing().any(|c| out.iter().any(|s| s.missing().any(|m| m == c)));
            if !repeats {
                out.push(f);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fight(cards: &[&str], won: bool) -> BossFight {
        BossFight {
            character: CharacterId::new("HERO"),
            act: 0,
            boss: Some(EncounterId::new("BOSS")),
            cards: cards.iter().map(|c| CardId::new(*c)).collect(),
            won,
        }
    }

    /// `n` fights with these cards, `wins` of them won.
    fn fights(out: &mut Vec<BossFight>, cards: &[&str], n: usize, wins: usize) {
        out.extend((0..n).map(|i| fight(cards, i < wins)));
    }

    #[test]
    fn a_pair_that_wins_more_together_than_apart_is_a_line() {
        let mut f = Vec::new();
        fights(&mut f, &["A", "B"], 60, 57); // together: 95%
        fights(&mut f, &["A"], 60, 36); // A alone: 60%
        fights(&mut f, &["B"], 60, 36);
        fights(&mut f, &["C", "D"], 60, 36); // a pair no better than base
        fights(&mut f, &["C"], 60, 36);
        let act = &measure(&f, &Offers::new())[0];
        let lines: Vec<Vec<&str>> = act.lines.iter().map(|l| l.cards.iter().map(|c| c.0.as_str()).collect()).collect();
        assert_eq!(lines, vec![vec!["A", "B"]]);
        let line = &act.lines[0];
        assert_eq!(line.record, Record { fights: 60, wins: 57 });
        assert_eq!(line.bosses[&EncounterId::new("BOSS")].fights, 60);
        assert!(line.apart < shrunk(line.record, rate(act.all)));
    }

    #[test]
    fn a_line_is_within_reach_when_part_is_held_and_the_rest_is_offered() {
        let mut f = Vec::new();
        fights(&mut f, &["A", "B"], 60, 57);
        fights(&mut f, &["A"], 60, 36);
        fights(&mut f, &["B"], 60, 36);
        let offers: Offers = [(
            (CharacterId::new("HERO"), 0),
            (100, [(CardId::new("A"), 50), (CardId::new("B"), 40)].into_iter().collect()),
        )]
        .into_iter()
        .collect();
        let catalog = Catalog { card_lines: measure(&f, &offers), ..Default::default() };
        let playbook: Playbook = serde_json::from_str(r#"{"version": 1, "acts": []}"#).unwrap();
        let mut state = RunState { character: Some(CharacterId::new("HERO")), ..Default::default() };
        assert!(state.deck.is_empty());

        // Neither held: out of reach.
        assert!(lines_within_reach(&state, &catalog, &playbook).is_empty());
        // A held: look for B, offered in 40% of runs.
        state = with_cards(&state, &[CardId::new("A")]);
        let s = lines_within_reach(&state, &catalog, &playbook);
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].held, vec![CardId::new("A")]);
        assert_eq!(s[0].options, vec![vec![CardId::new("B")]]);
        assert_eq!(s[0].offered, vec![(CardId::new("B"), 0.4)]);
        // Both held: nothing left to look for.
        state = with_cards(&state, &[CardId::new("B")]);
        assert!(lines_within_reach(&state, &catalog, &playbook).is_empty());
    }

    #[test]
    fn similar_lines_on_the_same_held_card_share_one_suggestion() {
        let mut f = Vec::new();
        fights(&mut f, &["A", "B"], 60, 57);
        fights(&mut f, &["A", "C"], 60, 56);
        fights(&mut f, &["A"], 60, 36);
        fights(&mut f, &["B"], 60, 36);
        fights(&mut f, &["C"], 60, 36);
        let offers: Offers = [(
            (CharacterId::new("HERO"), 0),
            (100, [(CardId::new("B"), 40), (CardId::new("C"), 30)].into_iter().collect()),
        )]
        .into_iter()
        .collect();
        let catalog = Catalog { card_lines: measure(&f, &offers), ..Default::default() };
        let playbook: Playbook = serde_json::from_str(r#"{"version": 1, "acts": []}"#).unwrap();
        let state = with_cards(
            &RunState { character: Some(CharacterId::new("HERO")), ..Default::default() },
            &[CardId::new("A")],
        );
        let s = lines_within_reach(&state, &catalog, &playbook);
        assert_eq!(s.len(), 1, "one entry: A with B or C");
        assert_eq!(s[0].held, vec![CardId::new("A")]);
        assert_eq!(s[0].options, vec![vec![CardId::new("B")], vec![CardId::new("C")]]);
    }
}
