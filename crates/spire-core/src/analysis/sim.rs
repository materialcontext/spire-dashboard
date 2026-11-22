//! Deck simulation: what the deck does turn by turn, read from the card data.
//!
//! Plays many seeded fights against a passive target: shuffle, draw, spend
//! energy greedily, and carry Strength, Dexterity, Focus, orbs, Poison,
//! Plating, Vulnerable, Aim, Stretch, Vigor and Stars between turns. Two
//! policies share the same shuffles. One plays for damage and one plays for
//! block, so each number means "what the deck does if you lean that way".
//!
//! Powers and relics run through [`crate::mechanics`]: start- and end-of-turn
//! effects, "whenever you play…" triggers, every-N counters and modifiers to
//! other cards (extra damage, Vulnerable, free Skills, echoes). What that
//! reader can't parse is reported, not guessed.
//!
//! Forge: the first Forge each combat adds the rules' Forge card (Sovereign
//! Blade) to the hand; every Forge adds its amount to that card's damage.
//! Named cards a card or power makes (Shivs, Souls, Sadness, Wounds…) join
//! the named pile; copies of a card go where it says; the Exhaust Pile is
//! kept, so exhaust triggers, "for each card in your Exhaust Pile" and
//! "play it from your Exhaust Pile" work ([`mechanics::card_text`]).
//!
//! Not modelled: enemy actions, potions, enchantments, conditions ("if this
//! kills"), random cards, choices ("put a card from your Discard Pile into
//! your Hand") and random effects. The numbers are for comparing
//! decks and spotting gaps, not for predicting a fight.

use crate::catalog::{Card, CardType, Catalog, Orb, OrbEffect, Target};
use crate::mechanics::{self, CardFilter, CardText, Effect, ExhaustWhich, Mechanics, Passive, Pile, When};
use crate::playbook::{SimAssumptions, contains_ci};
use crate::run::{DeckCard, RunState};

/// A fight ends here even if the deck has not cycled (turnover is capped).
const MAX_TURNS: usize = 20;

/// Most cards played in one turn, so endless combos end.
const MAX_PLAYS: usize = 50;
/// Turns reported for draw odds.
const ODDS_TURNS: usize = 3;

/// The group of cards made during a fight (the Forge card, Shivs…): not
/// deck cards, so no draw odds.
const MADE_GROUP: usize = usize::MAX;

/// A mean with the middle half of outcomes (25th to 75th percentile).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Spread {
    pub mean: f32,
    pub low: f32,
    pub high: f32,
}

impl Spread {
    /// From values sorted lowest first.
    fn of(values: &[f32]) -> Spread {
        if values.is_empty() {
            return Spread::default();
        }
        let n = values.len();
        Spread { mean: values.iter().sum::<f32>() / n as f32, low: values[n / 4], high: values[(3 * n) / 4] }
    }
}

/// The whole spread of an outcome over the simulated fights: its value at
/// every 5% of them, lowest first. Unknown (all zero) unless simulated.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Steps {
    pub values: [f32; 21],
    pub known: bool,
}

impl Steps {
    fn of(sorted: &[f32]) -> Steps {
        if sorted.is_empty() {
            return Steps::default();
        }
        let last = (sorted.len() - 1) as f32;
        Steps { values: std::array::from_fn(|i| sorted[(last * i as f32 / 20.0).round() as usize]), known: true }
    }

    /// Every fight gives exactly `x` (for tests and fixed decks).
    pub fn flat(x: f32) -> Steps {
        Steps { values: [x; 21], known: true }
    }

    /// The 20 slices of fights, each by its middle value, scaled.
    fn slices(&self, scale: f32) -> impl Iterator<Item = f32> + '_ {
        self.values.windows(2).map(move |w| (w[0] + w[1]) / 2.0 * scale)
    }

    /// Share of fights where the outcome (times `scale`) reaches `x`.
    pub fn reaching(&self, x: f32, scale: f32) -> f32 {
        self.slices(scale).filter(|v| *v >= x).count() as f32 / 20.0
    }

    /// How far short of `x` the outcome (times `scale`) falls, on average
    /// over the fights (0 where it reaches it).
    pub fn short_of(&self, x: f32, scale: f32) -> f32 {
        self.slices(scale).map(|v| (x - v).max(0.0)).sum::<f32>() / 20.0
    }
}

/// How a deck plays, averaged per turn: what fight rules ask about (many
/// hits, few Skills, cards a turn, debuffs) and how much damage hits all.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PlayStats {
    /// Attack hits landed.
    pub hits: f32,
    /// Damage per attack hit (attack damage only).
    pub per_hit: f32,
    /// Share of damage that hits every enemy (0–1).
    pub area_share: f32,
    pub cards: f32,
    pub skills: f32,
    /// Debuffs put on enemies.
    pub debuffs: f32,
}

/// What persists until used: setup (Aim, Vigor, Stretch) and the Fool's
/// and the Regent's resources (Arrows, Stars).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Store {
    Aim,
    Vigor,
    Stretch,
    Arrows,
    Stars,
}

impl Store {
    pub const ALL: [Store; 5] = [Store::Aim, Store::Vigor, Store::Stretch, Store::Arrows, Store::Stars];

    pub fn label(self) -> &'static str {
        match self {
            Store::Aim => "Aim",
            Store::Vigor => "Vigor",
            Store::Stretch => "Stretch",
            Store::Arrows => "Arrows",
            Store::Stars => "Stars",
        }
    }
}

/// One store per turn: how much was made, how much was used, and how many
/// cards in hand waited on it at the end of the turn (a Shoot card with no
/// Arrow, a Star card with too few Stars).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Flow {
    pub gained: f32,
    pub spent: f32,
    pub waiting: f32,
    /// Shots or Star cards played, for the share that waited.
    pub plays: f32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TurnOutput {
    /// Damage this turn when playing for damage.
    pub damage: Spread,
    /// Block this turn when playing for block.
    pub block: Spread,
    /// That Block's whole spread: what a bad draw blocks, not only the
    /// middle half.
    pub block_steps: Steps,
}

/// Character resources a deck produces, averaged per turn.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Resource {
    Stars,
    Summon,
    Forge,
    Doom,
    Poison,
    Orbs,
}

impl Resource {
    pub const ALL: [Resource; 6] =
        [Resource::Stars, Resource::Summon, Resource::Forge, Resource::Doom, Resource::Poison, Resource::Orbs];

    pub fn label(self) -> &'static str {
        match self {
            Resource::Stars => "Stars gained",
            Resource::Summon => "Summon",
            Resource::Forge => "Forge",
            Resource::Doom => "Doom applied",
            Resource::Poison => "Poison applied",
            Resource::Orbs => "Orbs channeled",
        }
    }
}

/// How likely one distinct card (all copies) is to have been drawn by turns 1–3.
#[derive(Clone, Debug, PartialEq)]
pub struct DrawOdds {
    pub card: DeckCard,
    pub name: String,
    pub copies: u32,
    pub by_turn: [f32; ODDS_TURNS],
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct DeckSim {
    pub trials: u32,
    pub hand_size: u32,
    pub energy: u32,
    /// One entry per simulated turn.
    pub turns: Vec<TurnOutput>,
    /// Mean turns to draw every card once: how fast the deck cycles.
    pub turnover: f32,
    /// Draw odds for each distinct card, likeliest first.
    pub odds: Vec<DrawOdds>,
    /// Share of drawn cards that cannot be played.
    pub dead_draws: f32,
    /// Energy left unspent per turn when playing for damage.
    pub unspent_energy: f32,
    /// Cost of playable cards left in hand per turn: energy the deck wanted.
    pub stranded_energy: f32,
    /// Per-turn averages of the resources this deck produces (non-zero only).
    pub resources: Vec<(Resource, f32)>,
    /// How the deck plays, per turn, when playing for damage.
    pub play: PlayStats,
    /// Each store per turn: Stretch when playing for block, the rest when
    /// playing for damage.
    pub stores: [Flow; 5],
    /// Relics whose combat effects are simulated, and those that aren't.
    pub relics_simulated: Vec<String>,
    pub relics_not_simulated: Vec<String>,
}

// ---------------------------------------------------------------- plays

#[derive(Clone, Debug, Default)]
struct Effects {
    damage: i32,
    hits: i32,
    osty: i32,
    block: i32,
    draw: i32,
    energy: i32,
    /// Energy that arrives next turn ("Next turn, … gain 2 Energy").
    energy_next: i32,
    stars: i32,
    summon: i32,
    forge: i32,
    doom: i32,
    poison: i32,
    vulnerable: i32,
    /// Weak it puts on an enemy.
    weak: i32,
    aim: i32,
    stretch: i32,
    vigor: i32,
    strength: i32,
    dexterity: i32,
    focus: i32,
}

/// One deck card, reduced to what the simulator needs.
#[derive(Clone, Debug)]
struct Play {
    group: usize,
    /// Energy cost; `None` means the card can't be played.
    cost: Option<i32>,
    x: bool,
    star_cost: i32,
    /// A non-power card's own effect, from its numbers.
    now: Effects,
    /// A power's effect, read from its text.
    mech: Mechanics,
    /// (orb index, count, once per energy)
    channels: Vec<(usize, i32, bool)>,
    /// Hits every enemy.
    all: bool,
    /// Needs and spends an Arrow.
    shoots: bool,
    aim_multiplier: i32,
    /// Brings a stuck Arrow back.
    dislodges: bool,
    card_type: CardType,
    /// Lowercase name, for name filters ("strike", "shiv").
    name: String,
    upgraded: bool,
    power: bool,
    exhausts: bool,
    ethereal: bool,
    innate: bool,
    retain: bool,
    /// What the text does beyond the numbers.
    text: CardText,
    /// Cards it makes, resolved: (play index, count, pile); a count of -1
    /// is X, -2 one per card discarded.
    makes: Vec<(usize, i32, Pile)>,
    /// The Forge card: its damage grows with every Forge.
    forged: bool,
}

fn play_of(card: &Card, upgraded: bool, group: usize, orbs: &[&Orb], all_orbs: &[Orb]) -> Play {
    let v = |k: &str| card.value(k, upgraded);
    let either = |a: &str, b: &str| v(a).max(v(b));
    let power = card.card_type == CardType::Power;
    let on_self = matches!(card.target, Target::Own | Target::None | Target::AllAllies);
    let base_stars = card.value("stars", false);
    // Invoke: "Next turn, Summon 2 and gain 2 Energy."
    let next_turn_energy =
        card.text_at(upgraded).split('.').any(|s| contains_ci(s, "next turn") && contains_ci(s, "energy"));
    let mut e = Effects::default();
    let mut mech = Mechanics::default();
    let mut text = mechanics::card_text(card.text_at(upgraded), all_orbs);
    if power {
        // A power makes cards through its triggers ("At the start of your
        // turn, add 1 Shiv"), read below.
        text.makes.clear();
    }
    if power {
        // Powers say what they do in prose; the reader turns it into triggers.
        mech = mechanics::read(card.text_at(upgraded), all_orbs, true);
    } else {
        e = Effects {
            // Some cards give their base damage only as the calculation's
            // base (Ashen Strike: 6 plus 3 per exhausted card).
            damage: if v("damage") > 0 { v("damage") } else { v("calculationbase").max(v("calculateddamage")) },
            hits: v("hitcount").max(1),
            osty: v("ostydamage"),
            block: v("block"),
            // The data's card count also marks "Add 3 Shivs" and "3 or more
            // cards": only a card that says "draw" draws.
            draw: if card.draw.is_some() && text.draws { v("cards") } else { 0 },
            // "When this card is Exhausted, gain 2 Energy" isn't on play.
            energy: if card.energy.is_some() && !next_turn_energy && text.when_exhausted.is_empty() {
                v("energy")
            } else {
                0
            },
            energy_next: if card.energy.is_some() && next_turn_energy { v("energy") } else { 0 },
            stars: if base_stars > 0 && contains_ci(&card.text, &format!("gain {base_stars} star")) {
                v("stars")
            } else {
                0
            },
            summon: v("summon"),
            forge: v("forge"),
            doom: either("doom", "doompower"),
            poison: either("poison", "poisonpower"),
            vulnerable: if on_self { 0 } else { either("vulnerable", "vulnerablepower") },
            weak: if on_self { 0 } else { either("weak", "weakpower") },
            ..Effects::default()
        };
        if card.target != Target::Ally {
            e.aim = either("aim", "aimpower");
            e.stretch = either("stretch", "stretchpower");
            e.vigor = either("vigor", "vigorpower");
        }
        // Strength-like numbers on a card aimed at an enemy are debuffs, and
        // "lose it at end of turn" gains are temporary.
        if on_self && v("strengthloss") == 0 {
            e.strength = either("strength", "strengthpower");
        }
        if on_self {
            e.dexterity = either("dexterity", "dexteritypower");
            e.focus = either("focus", "focuspower");
        }
    }
    let x = card.x_cost;
    // Curses and statuses count as dead even when they can technically be played.
    let cost = if card.is_dead_draw() {
        None
    } else if x {
        Some(0)
    } else {
        card.cost_at(upgraded).map(i32::from)
    };
    Play {
        group,
        cost,
        x,
        star_cost: if card.star_cost.is_some() { v("starcost").max(0) } else { 0 },
        now: e,
        mech,
        channels: card
            .channels
            .iter()
            .filter_map(|c| Some((orbs.iter().position(|o| o.id == c.orb)?, i32::from(c.count), c.x)))
            .collect(),
        all: card.target == Target::AllEnemies,
        shoots: contains_ci(&card.text, "shoot an arrow"),
        aim_multiplier: v("aimmultiplier").max(1),
        dislodges: card.card_type == CardType::Attack
            && contains_ci(&card.text, "dislodge")
            && !contains_ci(&card.text, "shoot"),
        card_type: card.card_type,
        name: card.name.to_lowercase(),
        upgraded,
        power,
        exhausts: card.has_keyword("Exhaust", upgraded),
        ethereal: card.has_keyword("Ethereal", upgraded),
        innate: card.has_keyword("Innate", upgraded),
        retain: card.has_keyword("Retain", upgraded),
        text,
        makes: Vec::new(),
        forged: false,
    }
}

fn matches(f: &CardFilter, p: &Play) -> bool {
    match f {
        CardFilter::Any => true,
        CardFilter::Type(t) => p.card_type == *t,
        CardFilter::CostAtLeast(n) => p.cost.is_some_and(|c| c >= i32::from(*n)),
        CardFilter::Named(n) => p.name.contains(n.as_str()),
        CardFilter::Upgraded(t) => p.upgraded && p.card_type == *t,
    }
}

/// Modifiers from powers and relics in play.
#[derive(Clone, Debug, Default)]
struct Mods {
    extra_damage: Vec<(CardFilter, i32)>,
    extra_osty: i32,
    extra_block: Vec<(CardFilter, i32)>,
    vulnerable_bonus: i32,
    vulnerable_is: Option<i32>,
    poison_extra: i32,
    retain_hand: bool,
    conserve_energy: bool,
    free_skills: bool,
    extra_hit: bool,
    echo_each_turn: bool,
    echo_first_of_combat: bool,
    first_attack_bonus: i32,
}

impl Mods {
    fn add(&mut self, p: &Passive) {
        match p {
            Passive::ExtraDamage(f, n) => self.extra_damage.push((f.clone(), *n)),
            Passive::ExtraOstyDamage(n) => self.extra_osty += n,
            Passive::ExtraBlock(f, n) => self.extra_block.push((f.clone(), *n)),
            Passive::VulnerableBonus(n) => self.vulnerable_bonus += n,
            Passive::VulnerableIs(n) => self.vulnerable_is = Some(*n),
            Passive::PoisonExtraTicks(n) => self.poison_extra += n,
            Passive::RetainHand => self.retain_hand = true,
            Passive::ConserveEnergy => self.conserve_energy = true,
            Passive::FreeSkills => self.free_skills = true,
            Passive::ExtraHit => self.extra_hit = true,
            Passive::EchoEachTurn => self.echo_each_turn = true,
            Passive::EchoFirstOfCombat => self.echo_first_of_combat = true,
            Passive::FirstAttackBonus(n) => self.first_attack_bonus += n,
        }
    }
}

// ---------------------------------------------------------------- rng

/// SplitMix64: tiny, fast and deterministic, so results are stable.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn shuffle<T>(&mut self, items: &mut [T]) {
        for i in (1..items.len()).rev() {
            let j = (self.next() % (i as u64 + 1)) as usize;
            items.swap(i, j);
        }
    }
}

// ---------------------------------------------------------------- fight

#[derive(Clone, Copy, PartialEq, Eq)]
enum Lean {
    Damage,
    Block,
}

#[derive(Clone, Copy)]
struct OrbState {
    orb: usize,
    passive: i32,
    evoke: i32,
}

#[derive(Clone, Copy, Default)]
struct TurnLog {
    damage: f32,
    /// Damage that hit every enemy.
    area: f32,
    /// Attack hits landed, and the damage they dealt.
    hits: f32,
    attack_damage: f32,
    /// Cards played, and how many were Skills.
    cards: f32,
    skills: f32,
    /// Debuffs put on enemies (each power applied counts once).
    debuffs: f32,
    block: f32,
    unspent: f32,
    stranded: f32,
    drawn: u32,
    dead: u32,
    resources: [f32; 6],
    stores: [Flow; 5],
}

type Rule = (When, Vec<Effect>);

struct Setup<'a> {
    plays: &'a [Play],
    orbs: &'a [&'a Orb],
    groups: usize,
    energy: i32,
    orb_slots: i32,
    arrows: i32,
    /// The game's Vulnerable percentage (from the power data), if known.
    vulnerable: i32,
    relic_rules: Vec<&'a Rule>,
    relic_mods: Mods,
    rules: &'a SimAssumptions,
    /// Plays from this index on aren't in the deck: they're made in the
    /// fight (the Forge card, Shivs…).
    deck_len: usize,
    /// The card Forge makes (an index into `plays`), if the rules name one.
    forged_card: Option<usize>,
    /// Made cards by the name the text uses ("shivs", "soul").
    made: Vec<(String, usize)>,
    /// What setup is worth per point: Aim by the deck's Shoot cards (their
    /// mean multiplier), Vigor by its attacks, Stretch by its Block cards;
    /// 0 when the deck has nothing to spend it on. It lasts until used, so
    /// any consumer in the deck cashes it in eventually.
    worth: [f32; 3],
}

/// What a point of Aim, Vigor and Stretch is worth to a deck of these plays.
fn setup_worth(plays: &[Play]) -> [f32; 3] {
    let shooters: Vec<i32> = plays.iter().filter(|p| p.shoots).map(|p| p.aim_multiplier).collect();
    let aim = if shooters.is_empty() { 0.0 } else { shooters.iter().sum::<i32>() as f32 / shooters.len() as f32 };
    let attacks = plays.iter().any(|p| p.card_type == CardType::Attack && p.now.damage > 0);
    let blocks = plays.iter().any(|p| p.now.block > 0);
    [aim, f32::from(u8::from(attacks)), f32::from(u8::from(blocks))]
}

/// Nested triggers (a draw that triggers a draw…) stop at this depth.
const MAX_DEPTH: u8 = 3;

struct Fight<'a> {
    s: &'a Setup<'a>,
    rng: Rng,
    draw: Vec<usize>,
    discard: Vec<usize>,
    hand: Vec<usize>,
    unseen: Vec<bool>,
    unseen_left: usize,
    first_drawn: Vec<Option<usize>>,
    /// Active triggers (relics, then powers as they're played) and their counters.
    rules: Vec<&'a Rule>,
    counters: Vec<u32>,
    mods: Mods,
    orbs: Vec<OrbState>,
    orb_slots: i32,
    strength: i32,
    dexterity: i32,
    focus: i32,
    aim: i32,
    stretch: i32,
    vigor: i32,
    plating: i32,
    poison: i32,
    vulnerable: i32,
    stars: i32,
    /// Arrows ready to shoot, and shot Arrows stuck in the enemy.
    arrows: i32,
    stuck: i32,
    hits_toward_return: i32,
    energy: i32,
    next_energy: i32,
    /// Damage forged onto the Forge card, whether it's been made, and the
    /// replays it has gained.
    forged: i32,
    forge_card_made: bool,
    forge_replay: i32,
    /// The Exhaust Pile, and whether a card was exhausted this turn.
    exhausted: Vec<usize>,
    exhausted_this_turn: bool,
    /// "Your next Skill is played an extra time": the types waiting.
    next_extra: Vec<CardType>,
    /// Extra cards to draw with this turn's hand (start-of-turn effects).
    pending_draw: i32,
    played_this_turn: u32,
    attacked_this_turn: bool,
    played_any: bool,
    depth: u8,
    turn: usize,
    log: TurnLog,
}

/// What one fight produced: per-turn logs, turnover and first-draw turns.
struct FightResult {
    turns: Vec<TurnLog>,
    turnover: usize,
    first_drawn: Vec<Option<usize>>,
}

impl<'a> Fight<'a> {
    fn new(s: &'a Setup<'a>, seed: u64) -> Self {
        let mut rng = Rng(seed);
        // Made cards aren't in the deck until something makes them.
        let mut draw: Vec<usize> = (0..s.deck_len).collect();
        rng.shuffle(&mut draw);
        let unseen: Vec<bool> = (0..s.plays.len()).map(|i| i < s.deck_len).collect();
        // Innate cards go on top (the end of the vector is the top).
        draw.sort_by_key(|&i| s.plays[i].innate);
        Fight {
            s,
            rng,
            draw,
            discard: Vec::new(),
            hand: Vec::new(),
            unseen_left: unseen.iter().filter(|u| **u).count(),
            unseen,
            first_drawn: vec![None; s.groups],
            rules: s.relic_rules.clone(),
            counters: vec![0; s.relic_rules.len()],
            mods: s.relic_mods.clone(),
            orbs: Vec::new(),
            orb_slots: s.orb_slots,
            strength: 0,
            dexterity: 0,
            focus: 0,
            aim: 0,
            stretch: 0,
            vigor: 0,
            plating: 0,
            poison: 0,
            vulnerable: 0,
            stars: 0,
            arrows: s.arrows,
            stuck: 0,
            hits_toward_return: 0,
            energy: 0,
            next_energy: 0,
            forged: 0,
            forge_card_made: false,
            forge_replay: 0,
            exhausted: Vec::new(),
            exhausted_this_turn: false,
            next_extra: Vec::new(),
            pending_draw: 0,
            played_this_turn: 0,
            attacked_this_turn: false,
            played_any: false,
            depth: 0,
            turn: 0,
            log: TurnLog::default(),
        }
    }

    fn draw_cards(&mut self, n: i32) {
        for _ in 0..n.max(0) {
            if self.hand.len() >= self.s.rules.max_hand as usize {
                return;
            }
            if self.draw.is_empty() {
                if self.discard.is_empty() {
                    return;
                }
                self.draw.append(&mut self.discard);
                self.rng.shuffle(&mut self.draw);
                self.fire(|w| *w == When::OnShuffle);
            }
            let Some(i) = self.draw.pop() else { return };
            let p = &self.s.plays[i];
            if self.unseen[i] {
                self.unseen[i] = false;
                self.unseen_left -= 1;
            }
            if let Some(first) = self.first_drawn.get_mut(p.group) {
                first.get_or_insert(self.turn);
            }
            self.log.drawn += 1;
            if p.cost.is_none() {
                self.log.dead += 1;
            }
            self.hand.push(i);
            self.fire(|w| *w == When::OnDraw);
        }
    }

    /// Fires every active rule whose timing matches.
    fn fire(&mut self, when: impl Fn(&When) -> bool) {
        if self.depth >= MAX_DEPTH {
            return;
        }
        self.depth += 1;
        for k in 0..self.rules.len() {
            let rule: &'a Rule = self.rules[k];
            if when(&rule.0) {
                for e in &rule.1 {
                    self.effect(e);
                }
            }
        }
        self.depth -= 1;
    }

    /// After a card resolves: "whenever you play…" and every-N counters.
    fn after_play(&mut self, p: &Play) {
        if self.depth >= MAX_DEPTH {
            return;
        }
        self.depth += 1;
        for k in 0..self.rules.len() {
            let rule: &'a Rule = self.rules[k];
            let fires = match &rule.0 {
                When::OnPlay(f) => matches(f, p),
                When::EveryPlays { filter, n, .. } if matches(filter, p) => {
                    self.counters[k] += 1;
                    let hit = self.counters[k] >= *n;
                    if hit {
                        self.counters[k] = 0;
                    }
                    hit
                }
                _ => false,
            };
            if fires {
                for e in &rule.1 {
                    self.effect(e);
                }
            }
        }
        self.depth -= 1;
    }
}

/// Whether a timing fires at the start of turn `t` (1-based).
fn starts_turn(w: &When, t: u32) -> bool {
    {
        match w {
            When::CombatStart => t == 1,
            When::TurnStart => true,
            When::TurnNumber(n) => t == *n,
            When::FirstTurns(n) => t <= *n,
            When::TurnsFrom(n) => t >= *n,
            When::EveryTurns(n) => t.is_multiple_of((*n).max(1)),
            _ => false,
        }
    }
}

impl<'a> Fight<'a> {
    /// One effect from a power or relic. Draws at the start of a turn join
    /// the hand draw; damage from triggers isn't attack damage, so Strength
    /// and Vulnerable don't apply.
    fn effect(&mut self, e: &Effect) {
        let r = |f: &mut Self, res: Resource, n: i32| f.log.resources[res as usize] += n as f32;
        match e {
            Effect::Block(n) => {
                self.log.block += *n as f32;
                if *n > 0 {
                    self.fire(|w| *w == When::OnGainBlock);
                }
            }
            Effect::Damage(n) => self.log.damage += *n as f32,
            Effect::DamageAll(n) => {
                self.log.damage += *n as f32;
                self.log.area += *n as f32;
            }
            Effect::Energy(n) => self.energy += n,
            Effect::Draw(n) if self.hand.is_empty() && self.played_this_turn == 0 => self.pending_draw += n,
            Effect::Draw(n) => self.draw_cards(*n),
            Effect::Strength(n) => self.strength += n,
            Effect::Dexterity(n) => self.dexterity += n,
            Effect::Focus(n) => self.focus += n,
            Effect::Vigor(n) => self.gain(Store::Vigor, *n),
            Effect::Aim(n) => self.gain(Store::Aim, *n),
            Effect::Stretch(n) => self.gain(Store::Stretch, *n),
            Effect::Plating(n) => self.plating += n,
            Effect::Poison(n) => {
                self.poison += n;
                r(self, Resource::Poison, *n);
            }
            Effect::Doom(n) => r(self, Resource::Doom, *n),
            Effect::Vulnerable(n) => self.vulnerable += n,
            Effect::Forge(n) => self.forge(*n),
            Effect::Stars(n) => {
                self.gain(Store::Stars, *n);
                r(self, Resource::Stars, *n);
            }
            Effect::Summon(n) => r(self, Resource::Summon, *n),
            Effect::Channel(orb, n) => {
                if let Some(o) = self.s.orbs.iter().position(|x| x.id == *orb) {
                    for _ in 0..*n {
                        self.channel(o);
                    }
                }
            }
            Effect::OrbSlots(n) => self.orb_slots = (self.orb_slots + n).clamp(0, self.s.rules.max_orb_slots as i32),
            Effect::Arrows(n) => self.gain(Store::Arrows, *n),
            Effect::Make { name, n, pile } => {
                if let Some(&(_, card)) = self.s.made.iter().find(|(m, _)| m == name) {
                    self.make(card, *n, *pile);
                }
            }
        }
    }

    /// Forges `n`: the first time in the fight it makes the Forge card (into
    /// the hand, or the discard pile when the hand is full).
    fn forge(&mut self, n: i32) {
        if n <= 0 {
            return;
        }
        self.log.resources[Resource::Forge as usize] += n as f32;
        let Some(card) = self.s.forged_card else { return };
        if !self.forge_card_made {
            self.forge_card_made = true;
            if self.hand.len() < self.s.rules.max_hand as usize {
                self.hand.push(card);
            } else {
                self.discard.push(card);
            }
        }
        self.forged += n;
    }

    /// A card's printed damage, plus what's been forged onto the Forge card
    /// and what the Exhaust Pile adds.
    fn damage_of(&self, p: &Play) -> i32 {
        let forged = if p.forged { self.forged } else { 0 };
        let pile = p.text.per_exhausted.as_ref().map_or(0, |(n, what)| {
            let counted = self
                .exhausted
                .iter()
                .filter(|&&i| what.as_ref().is_none_or(|w| self.s.plays[i].name.trim_end_matches('s') == *w))
                .count();
            n * counted as i32
        });
        p.now.damage + forged + pile
    }

    /// Adds `n` copies of a card to a pile. A full hand sends the rest to
    /// the discard pile; the draw pile takes them at random places.
    fn make(&mut self, card: usize, n: i32, pile: Pile) {
        for _ in 0..n.max(0) {
            match pile {
                Pile::Hand if self.hand.len() < self.s.rules.max_hand as usize => self.hand.push(card),
                Pile::Hand | Pile::Discard => self.discard.push(card),
                Pile::Draw => {
                    let at = (self.rng.next() % (self.draw.len() as u64 + 1)) as usize;
                    self.draw.insert(at, card);
                }
            }
        }
    }

    /// Puts a card in the Exhaust Pile and fires what that triggers.
    fn exhaust(&mut self, i: usize) {
        self.exhausted.push(i);
        self.exhausted_this_turn = true;
        self.fire(|w| *w == When::OnExhaust);
        let setup: &'a Setup<'a> = self.s;
        for e in &setup.plays[i].text.when_exhausted {
            self.effect(e);
        }
    }

    /// Whether a card's condition holds now ("If you Exhausted a card this
    /// turn", "If you have 3 or more cards in your Exhaust Pile").
    fn works(&self, p: &Play) -> bool {
        (!p.text.needs_exhaust_this_turn || self.exhausted_this_turn)
            && p.text.exhausted_at_least.is_none_or(|n| self.exhausted.len() as i32 >= n)
    }

    /// The cards an "Exhaust …" card takes, removed from where they are.
    fn take_for_exhaust(&mut self, which: &ExhaustWhich) -> Vec<usize> {
        let setup: &'a Setup<'a> = self.s;
        let pick = |i: &usize| {
            let p = &setup.plays[*i];
            match which {
                ExhaustWhich::Hand => true,
                ExhaustWhich::HandNonAttacks => p.card_type != CardType::Attack,
                ExhaustWhich::Statuses => p.card_type == CardType::Status,
                ExhaustWhich::HandNamed(n) => p.name.trim_end_matches('s') == n.trim_end_matches('s'),
            }
        };
        let mut taken: Vec<usize> = self.hand.iter().copied().filter(|i| pick(i)).collect();
        self.hand.retain(|i| !pick(i));
        if *which == ExhaustWhich::Statuses {
            taken.extend(self.draw.iter().copied().filter(|i| pick(i)));
            taken.extend(self.discard.iter().copied().filter(|i| pick(i)));
            self.draw.retain(|i| !pick(i));
            self.discard.retain(|i| !pick(i));
        }
        taken
    }

    /// Adds to a store, counting what was made.
    fn gain(&mut self, store: Store, n: i32) {
        let slot = match store {
            Store::Aim => &mut self.aim,
            Store::Vigor => &mut self.vigor,
            Store::Stretch => &mut self.stretch,
            Store::Arrows => &mut self.arrows,
            Store::Stars => &mut self.stars,
        };
        *slot += n;
        if n > 0 {
            self.log.stores[store as usize].gained += n as f32;
        }
    }

    fn spend(&mut self, store: Store, n: i32) {
        if n > 0 {
            self.log.stores[store as usize].spent += n as f32;
        }
    }

    fn orb_value(&self, o: OrbState, base: i32) -> i32 {
        if self.s.orbs[o.orb].focus { (base + self.focus).max(0) } else { base }
    }

    fn orb_effect(&mut self, effect: OrbEffect, amount: i32, now: bool) {
        match effect {
            OrbEffect::Damage => self.log.damage += amount as f32,
            OrbEffect::DamageAll => {
                self.log.damage += amount as f32;
                self.log.area += amount as f32;
            }
            OrbEffect::Block => self.log.block += amount as f32,
            OrbEffect::Energy if now => self.energy += amount,
            OrbEffect::Energy => self.next_energy += amount,
            OrbEffect::Grow | OrbEffect::Other => {}
        }
    }

    fn channel(&mut self, orb: usize) {
        if self.orb_slots <= 0 {
            return;
        }
        while self.orbs.len() >= self.orb_slots as usize {
            let o = self.orbs.remove(0);
            let def = self.s.orbs[o.orb];
            self.orb_effect(def.evoke_effect, self.orb_value(o, o.evoke), true);
        }
        let def = self.s.orbs[orb];
        self.orbs.push(OrbState { orb, passive: def.passive, evoke: def.evoke });
        // (Glass evokes for its current passive; see `end_turn`.)
        self.log.resources[Resource::Orbs as usize] += 1.0;
    }

    fn vulnerable_pct(&self) -> i32 {
        self.mods.vulnerable_is.unwrap_or(self.s.vulnerable) + self.mods.vulnerable_bonus
    }

    /// A card's own effect, `times` times for X costs.
    fn apply(&mut self, p: &Play, times: i32) {
        let e = &p.now;
        let damage = self.damage_of(p);
        if damage > 0 {
            // Per hit, in the game's order (Spire Codex, combat mechanics):
            // printed damage plus flat bonuses (Strength, "deals N additional
            // damage", Vigor, and Aim on shots), then multipliers, rounded
            // down once per hit. Vigor and Aim are spent by the attack but
            // count on every one of its hits.
            let extra: i32 = self.mods.extra_damage.iter().filter(|(f, _)| matches(f, p)).map(|(_, n)| n).sum();
            let hits = (e.hits + i32::from(self.mods.extra_hit && e.hits > 1)) * times;
            let mut bonus = std::mem::take(&mut self.vigor);
            self.spend(Store::Vigor, bonus);
            if p.shoots {
                let aim = std::mem::take(&mut self.aim);
                self.spend(Store::Aim, aim);
                bonus += aim * p.aim_multiplier;
            }
            let mut mult = 1.0f32;
            if p.card_type == CardType::Attack {
                if !self.attacked_this_turn && self.mods.first_attack_bonus > 0 {
                    mult *= 1.0 + self.mods.first_attack_bonus as f32 / 100.0;
                }
                if self.vulnerable > 0 {
                    mult *= 1.0 + self.vulnerable_pct() as f32 / 100.0;
                }
                self.attacked_this_turn = true;
            }
            let hit = |bonus: i32| ((damage + self.strength + extra + bonus).max(0) as f32 * mult).floor();
            let dealt = hit(bonus) * hits as f32;
            self.log.damage += dealt;
            self.log.hits += hits as f32;
            self.log.attack_damage += dealt;
            if p.all {
                self.log.area += dealt;
            }
        }
        if e.osty > 0 {
            self.log.damage += ((e.osty + self.mods.extra_osty) * times) as f32;
        }
        if e.block > 0 {
            let extra: i32 = self.mods.extra_block.iter().filter(|(f, _)| matches(f, p)).map(|(_, n)| n).sum();
            // Stretch is spent by the card but adds to each of its Block gains.
            let stretch = std::mem::take(&mut self.stretch);
            self.spend(Store::Stretch, stretch);
            self.log.block += ((e.block + self.dexterity + extra + stretch).max(0) * times) as f32;
            self.fire(|w| *w == When::OnGainBlock);
        }
        self.vulnerable += e.vulnerable * times;
        let debuffs = [e.vulnerable, e.weak, e.poison, e.doom].iter().filter(|n| **n > 0).count();
        self.log.debuffs += (debuffs as i32 * times) as f32;
        self.energy += e.energy;
        self.next_energy += e.energy_next;
        self.gain(Store::Stars, e.stars * times);
        self.strength += e.strength;
        self.dexterity += e.dexterity;
        self.focus += e.focus;
        self.gain(Store::Aim, e.aim * times);
        self.gain(Store::Stretch, e.stretch * times);
        self.gain(Store::Vigor, e.vigor * times);
        self.poison += e.poison * times;
        let r = &mut self.log.resources;
        r[Resource::Stars as usize] += (e.stars * times) as f32;
        r[Resource::Summon as usize] += (e.summon * times) as f32;
        self.forge(e.forge * times);
        let r = &mut self.log.resources;
        r[Resource::Doom as usize] += (e.doom * times) as f32;
        r[Resource::Poison as usize] += (e.poison * times) as f32;
        if e.draw > 0 {
            self.draw_cards(e.draw * times);
        }
    }

    fn cost_of(&self, p: &Play) -> Option<i32> {
        if self.mods.free_skills && p.card_type == CardType::Skill {
            return p.cost.map(|_| 0);
        }
        // "Costs 1 less Energy for each card Exhausted this combat."
        let less = if p.text.cheaper_per_exhausted { self.exhausted.len() as i32 } else { 0 };
        p.cost.map(|c| (c - less).max(0))
    }

    /// How much a play is worth to the current lean, per energy. Setup and
    /// acceleration are given rough exchange rates so they get played.
    fn score(&self, p: &Play, lean: Lean) -> f32 {
        let times = if p.x { self.energy } else { 1 };
        if times == 0 || !self.works(p) {
            return 0.0;
        }
        // "… for each card Exhausted": the hand it would take, less itself.
        let times = match &p.text.exhausts {
            Some(which) if p.text.own_per_card => {
                let n = self
                    .hand
                    .iter()
                    .filter(|&&i| {
                        let q = &self.s.plays[i];
                        match which {
                            ExhaustWhich::Hand => true,
                            ExhaustWhich::HandNonAttacks => q.card_type != CardType::Attack,
                            ExhaustWhich::Statuses => q.card_type == CardType::Status,
                            ExhaustWhich::HandNamed(n) => q.name.trim_end_matches('s') == n.trim_end_matches('s'),
                        }
                    })
                    .count() as i32;
                (n - i32::from(*which != ExhaustWhich::Statuses)).max(0) * times
            }
            _ => times,
        };
        let e = &p.now;
        let t = times as f32;
        let damage = self.damage_of(p);
        let attack = if damage > 0 { ((damage + self.strength).max(0) * e.hits) as f32 * t } else { 0.0 }
            + (e.osty as f32
                + e.poison as f32 * 1.5
                + e.doom as f32 * 0.5
                + e.forge as f32
                + e.aim as f32 * self.s.worth[0]
                + e.vigor as f32 * self.s.worth[1]
                + e.vulnerable as f32 * 3.0)
                * t
            // Setup waiting to be used counts for the card that uses it.
            + if p.shoots { (self.aim * p.aim_multiplier) as f32 } else { 0.0 }
            + if e.damage > 0 && p.card_type == CardType::Attack { (self.vigor * e.hits) as f32 } else { 0.0 };
        let defend = if e.block > 0 { (e.block + self.dexterity + self.stretch).max(0) as f32 * t } else { 0.0 }
            + (e.stretch as f32 * self.s.worth[2] + e.summon as f32) * t;
        let (main, side) = match lean {
            Lean::Damage => (attack, defend * 0.1),
            Lean::Block => (defend, attack * 0.1),
        };
        // Powers are worth playing early: a flat value plus what they do now.
        let power = if p.power && !p.mech.is_empty() {
            8.0 + p
                .mech
                .triggers
                .iter()
                .filter(|(w, _)| *w == When::Played)
                .flat_map(|(_, es)| es)
                .map(|e| match e {
                    Effect::Strength(n) | Effect::Dexterity(n) => *n as f32 * 4.0,
                    Effect::Focus(n) => *n as f32 * 3.0 * self.orb_slots as f32,
                    Effect::Energy(n) | Effect::Draw(n) => *n as f32 * 3.0,
                    Effect::Block(n) | Effect::Plating(n) => *n as f32,
                    _ => 2.0,
                })
                .sum::<f32>()
        } else {
            0.0
        };
        // Replay for the Forge card: its damage again, each time it's played.
        let replay = match (&p.text.gives_replay, self.s.forged_card) {
            (Some((name, n)), Some(f)) if self.s.plays[f].name == *name => {
                8.0 + self.damage_of(&self.s.plays[f]) as f32 * *n as f32
            }
            _ => 0.0,
        };
        let setup = replay
            + (e.strength + e.dexterity) as f32 * 4.0
            + e.focus as f32 * 3.0 * self.orb_slots as f32
            + e.draw as f32 * 3.0 * t
            + (e.energy + e.energy_next) as f32 * 4.0
            + e.stars as f32 * 2.0 * t
            + power
            + p.channels
                .iter()
                .map(|&(o, n, x)| {
                    let n = if x { n * times } else { n };
                    let orb = self.s.orbs[o];
                    (orb.passive.max(1) * 2 + orb.evoke) as f32 * n as f32 * 0.5
                })
                .sum::<f32>();
        // Cards it makes: what they'd do, half when they land in a pile.
        let made: f32 = p
            .makes
            .iter()
            .map(|&(card, n, pile)| {
                let q = &self.s.plays[card].now;
                let n = match n {
                    -1 => times,
                    -2 => self.hand.len() as i32 - 1,
                    n => n,
                };
                let worth = match lean {
                    Lean::Damage => (q.damage * q.hits) as f32 + q.block as f32 * 0.1,
                    Lean::Block => q.block as f32 + (q.damage * q.hits) as f32 * 0.1,
                } + (q.draw * 3 + q.energy * 4) as f32;
                worth * n.max(0) as f32 * if pile == Pile::Hand { 1.0 } else { 0.5 }
            })
            .sum::<f32>()
            + if p.text.next_extra.is_some() { 6.0 } else { 0.0 };
        let value = main + side + setup + made;
        let cost = self.cost_of(p).unwrap_or(0).max(if p.x { self.energy } else { 0 });
        value / (cost as f32).max(0.5)
    }

    fn affordable(&self, p: &Play) -> bool {
        match self.cost_of(p) {
            Some(c) => c <= self.energy && p.star_cost <= self.stars && (!p.shoots || self.arrows > 0),
            None => false,
        }
    }

    /// Shooting sticks an Arrow in the enemy; further hits or a Dislodge bring it back.
    fn arrows_after(&mut self, p: &Play, times: i32) {
        if p.shoots {
            self.arrows -= 1;
            self.stuck += 1;
            self.spend(Store::Arrows, 1);
            self.log.stores[Store::Arrows as usize].plays += 1.0;
            return;
        }
        if self.stuck == 0 {
            return;
        }
        if p.dislodges {
            self.stuck -= 1;
            self.gain(Store::Arrows, 1);
        } else if p.card_type == CardType::Attack && p.now.damage > 0 {
            let per = self.s.rules.arrow_return_hits.max(1) as i32;
            self.hits_toward_return += p.now.hits * times;
            while self.hits_toward_return >= per && self.stuck > 0 {
                self.hits_toward_return -= per;
                self.stuck -= 1;
                self.gain(Store::Arrows, 1);
            }
        }
    }

    /// Plays one card: pays for it, resolves it (twice when echoed), fires
    /// play triggers, then exhausts or discards it.
    fn play(&mut self, i: usize) {
        // The plays outlive this fight, so this borrow doesn't hold `self`.
        let setup: &'a Setup<'a> = self.s;
        let p: &'a Play = &setup.plays[i];
        let times = if p.x { self.energy } else { 1 };
        self.energy -= if p.x { self.energy } else { self.cost_of(p).unwrap_or(0) };
        self.stars -= p.star_cost;
        if p.star_cost > 0 {
            self.spend(Store::Stars, p.star_cost);
            self.log.stores[Store::Stars as usize].plays += 1.0;
        }
        self.log.cards += 1.0;
        if p.card_type == CardType::Skill {
            self.log.skills += 1.0;
        }
        let echo = (self.mods.echo_each_turn && self.played_this_turn == 0)
            || (self.mods.echo_first_of_combat && !self.played_any);
        // "Your next Skill is played an extra time", and the Forge card's
        // Replay.
        let extra = match self.next_extra.iter().position(|t| *t == p.card_type) {
            Some(k) => {
                self.next_extra.remove(k);
                1
            }
            None => 0,
        };
        let replay = if p.forged { self.forge_replay } else { 0 };
        let works = self.works(p);
        for _ in 0..1 + i32::from(echo) + extra + replay {
            for &(orb, n, x) in &p.channels {
                for _ in 0..if x { n * times } else { n } {
                    self.channel(orb);
                }
            }
            self.arrows_after(p, times);
            if p.power {
                for passive in &p.mech.passives {
                    self.mods.add(passive);
                }
                for rule in p.mech.triggers.iter().filter(|r| r.0 == When::Played) {
                    for e in &rule.1 {
                        self.effect(e);
                    }
                }
            } else if works {
                self.resolve(p, times);
            }
            if let Some(t) = p.text.next_extra {
                self.next_extra.push(t);
            }
            if let Some((name, n)) = &p.text.gives_replay
                && setup.forged_card.is_some_and(|f| setup.plays[f].name == *name)
            {
                self.forge_replay += n;
            }
        }
        self.played_this_turn += 1;
        self.played_any = true;
        self.after_play(p);
        // A new power's triggers start with the next card: "whenever you
        // play a card" doesn't count the power itself.
        if p.power {
            for _ in 0..if echo { 2 } else { 1 } {
                for rule in p.mech.triggers.iter().filter(|r| r.0 != When::Played) {
                    self.rules.push(rule);
                    self.counters.push(0);
                }
            }
        }
        if let Some(pile) = p.text.copy_to {
            self.make(i, 1, pile);
        }
        // A card resolves before it leaves the hand, so its own draw can't
        // shuffle it back into the draw pile.
        let exhausted = p.exhausts || (self.mods.free_skills && p.card_type == CardType::Skill);
        if exhausted {
            self.exhaust(i);
        } else if !p.power {
            self.discard.push(i);
        }
    }

    /// A non-power card's effect: what it exhausts or discards first, its
    /// numbers (per card exhausted when it says so), what it makes.
    fn resolve(&mut self, p: &Play, times: i32) {
        let mut discarded = 0;
        if p.text.discards_hand {
            discarded = self.hand.len() as i32;
            self.discard.append(&mut self.hand);
        }
        let mut per = times;
        if let Some(which) = &p.text.exhausts {
            let taken = self.take_for_exhaust(which);
            let k = taken.len() as i32;
            for c in taken {
                self.exhaust(c);
            }
            if p.text.own_per_card {
                per = times * k;
            } else {
                for _ in 0..k {
                    for e in &p.text.per_card {
                        self.effect(e);
                    }
                }
            }
        }
        self.apply(p, per);
        if p.text.block_if_exhausted > 0 && self.exhausted_this_turn {
            self.log.block += (p.text.block_if_exhausted + self.dexterity).max(0) as f32;
        }
        for &(card, n, pile) in &p.makes {
            let n = match n {
                -1 => times,
                -2 => discarded,
                n => n,
            };
            self.make(card, n, pile);
        }
    }

    /// Cards in the Exhaust Pile that play themselves at this timing
    /// (Bombardment at the start of the turn, Howl from Beyond at the end).
    fn play_from_exhaust(&mut self, when: When) {
        let setup: &'a Setup<'a> = self.s;
        let due: Vec<usize> = self
            .exhausted
            .iter()
            .copied()
            .filter(|&i| setup.plays[i].text.replays_from_exhaust == Some(when.clone()))
            .collect();
        for i in due {
            self.apply(&setup.plays[i], 1);
        }
    }

    fn play_turn(&mut self, lean: Lean) {
        // A deck can loop for ever (free cards that draw each other); the
        // game would let it, the simulator stops.
        for _ in 0..MAX_PLAYS {
            let best = self
                .hand
                .iter()
                .enumerate()
                .filter(|(_, i)| self.affordable(&self.s.plays[**i]))
                .map(|(h, i)| (h, self.score(&self.s.plays[*i], lean)))
                .filter(|(_, s)| *s > 0.0)
                .max_by(|a, b| a.1.total_cmp(&b.1));
            let Some((h, _)) = best else { break };
            let i = self.hand.remove(h);
            self.play(i);
        }
        // Cards left waiting on an Arrow or on Stars.
        for &i in &self.hand {
            let p = &self.s.plays[i];
            if p.shoots && self.arrows <= 0 {
                self.log.stores[Store::Arrows as usize].waiting += 1.0;
            }
            if p.star_cost > self.stars {
                self.log.stores[Store::Stars as usize].waiting += 1.0;
            }
        }
    }

    fn begin_turn(&mut self) {
        self.log = TurnLog::default();
        self.exhausted_this_turn = false;
        self.next_extra.clear();
        if self.turn > 0 {
            self.plating = (self.plating - 1).max(0);
        }
        let carried = if self.mods.conserve_energy { self.energy } else { 0 };
        self.energy = self.s.energy + std::mem::take(&mut self.next_energy) + carried;
        self.played_this_turn = 0;
        self.attacked_this_turn = false;
        for (k, rule) in self.rules.iter().enumerate() {
            if let When::EveryPlays { per_turn: true, .. } = rule.0 {
                self.counters[k] = 0;
            }
        }
        // Start-of-turn rules before the draw: their draws join the hand.
        self.pending_draw = 0;
        let hand_before = std::mem::take(&mut self.hand);
        let t = self.turn as u32 + 1;
        self.fire(move |w| starts_turn(w, t));
        self.play_from_exhaust(When::TurnStart);
        self.hand.extend(hand_before);
        let extra = std::mem::take(&mut self.pending_draw);
        self.draw_cards(self.s.rules.hand_size as i32 + extra);
    }

    fn end_turn(&mut self) {
        self.fire(|w| *w == When::TurnEnd);
        self.play_from_exhaust(When::TurnEnd);
        self.log.block += self.plating as f32;
        for k in 0..self.orbs.len() {
            let o = self.orbs[k];
            let def = self.s.orbs[o.orb];
            let amount = self.orb_value(o, o.passive);
            if def.passive_effect == OrbEffect::Grow {
                self.orbs[k].evoke += amount;
            } else {
                self.orb_effect(def.passive_effect, amount, false);
            }
            // Decaying orbs (Glass) lose value each turn; the evoke shrinks in step.
            if def.decay > 0 && def.passive > 0 {
                let passive = (o.passive - def.decay).max(0);
                self.orbs[k].passive = passive;
                self.orbs[k].evoke = def.evoke * passive / def.passive;
            }
        }
        // Poison ticks at the start of the enemy's turn, then drops by one.
        for _ in 0..1 + self.mods.poison_extra.max(0) {
            self.log.damage += self.poison as f32;
            self.poison = (self.poison - 1).max(0);
        }
        self.vulnerable = (self.vulnerable - 1).max(0);
        self.log.unspent = self.energy as f32;
        self.log.stranded = self
            .hand
            .iter()
            .filter_map(|&i| {
                let p = &self.s.plays[i];
                self.cost_of(p).filter(|c| *c > self.energy && !p.x)
            })
            .sum::<i32>() as f32;
        let hand = std::mem::take(&mut self.hand);
        let setup: &'a Setup<'a> = self.s;
        for i in hand {
            let p = &setup.plays[i];
            if p.retain || (self.mods.retain_hand && !p.ethereal) {
                self.hand.push(i);
            } else if p.ethereal {
                self.exhaust(i);
            } else {
                self.discard.push(i);
            }
        }
    }

    fn run(mut self, lean: Lean) -> FightResult {
        let reported = self.s.rules.turns as usize;
        let mut turns = Vec::with_capacity(reported);
        let mut turnover = None;
        while self.turn < MAX_TURNS && (self.turn < reported || turnover.is_none()) {
            self.begin_turn();
            self.play_turn(lean);
            self.end_turn();
            if self.unseen_left == 0 && turnover.is_none() {
                turnover = Some(self.turn + 1);
            }
            if self.turn < reported {
                turns.push(self.log);
            }
            self.turn += 1;
        }
        FightResult { turns, turnover: turnover.unwrap_or(MAX_TURNS), first_drawn: self.first_drawn }
    }
}

// ---------------------------------------------------------------- entry

/// The card a text names ("shivs", "soul", "sweeping gaze"), singular or
/// plural.
fn named_card<'c>(catalog: &'c Catalog, name: &str) -> Option<&'c Card> {
    let single = name.strip_suffix('s').unwrap_or(name);
    catalog
        .cards
        .values()
        .find(|c| c.name.eq_ignore_ascii_case(name))
        .or_else(|| catalog.cards.values().find(|c| c.name.eq_ignore_ascii_case(single)))
}

/// Simulates the current deck. Deterministic: the same deck gives the same numbers.
pub fn simulate(state: &RunState, catalog: &Catalog, rules: &SimAssumptions) -> DeckSim {
    let character = state.character.as_ref().and_then(|c| catalog.characters.get(c));
    let energy = character.map_or(3, |c| c.energy) as i32;
    let orb_slots = character.and_then(|c| c.orb_slots).unwrap_or(0) as i32;
    let arrows = character.and_then(|c| c.arrow_slots).unwrap_or(0) as i32;
    let orbs: Vec<&Orb> = catalog.orbs.values().collect();
    let all_orbs: Vec<Orb> = catalog.orbs.values().cloned().collect();

    // Cards whose need nothing in the run supplies do nothing: dead.
    let unmet = crate::analysis::requirements::unmet(state, catalog, &rules.requirements);
    let mut groups: Vec<(DeckCard, String, u32)> = Vec::new();
    let mut plays = Vec::new();
    for d in &state.deck {
        let Some(card) = catalog.cards.get(&d.card) else { continue };
        let key = DeckCard { enchantment: None, ..d.clone() };
        let group = match groups.iter().position(|g| g.0 == key) {
            Some(g) => {
                groups[g].2 += 1;
                g
            }
            None => {
                let name = if d.upgraded { format!("{}+", card.name) } else { card.name.clone() };
                groups.push((key, name, 1));
                groups.len() - 1
            }
        };
        let mut play = play_of(card, d.upgraded, group, &orbs, &all_orbs);
        if unmet.contains_key(&d.card) {
            play.cost = None;
        }
        plays.push(play);
    }
    if plays.is_empty() || rules.trials == 0 || rules.turns == 0 {
        return DeckSim::default();
    }
    let deck_len = plays.len();
    // The card Forge makes, kept outside the deck until a Forge.
    let forged_card = catalog.rules.forge_card.as_ref().and_then(|id| catalog.cards.get(id)).map(|card| {
        plays.push(Play { forged: true, ..play_of(card, false, MADE_GROUP, &orbs, &all_orbs) });
        plays.len() - 1
    });

    // Relics: what the reader understands joins every fight from the start.
    let relics: Vec<(String, Mechanics)> = state
        .relics
        .iter()
        .filter_map(|r| catalog.relics.get(r))
        .map(|r| (r.name.clone(), mechanics::read(&r.text, &all_orbs, false)))
        .collect();

    // Cards the deck, its powers and relics make, by the name the text
    // uses, each once outside the deck.
    let names: Vec<String> = plays
        .iter()
        .flat_map(|p| p.text.makes.iter().chain(p.mech.triggers.iter().flat_map(|(_, es)| es)))
        .chain(relics.iter().flat_map(|(_, m)| m.triggers.iter().flat_map(|(_, es)| es)))
        .filter_map(|e| if let Effect::Make { name, .. } = e { Some(name.clone()) } else { None })
        .collect();
    let mut made: Vec<(String, usize)> = Vec::new();
    for name in names {
        if made.iter().any(|(m, _)| *m == name) {
            continue;
        }
        if let Some(card) = named_card(catalog, &name) {
            plays.push(play_of(card, false, MADE_GROUP, &orbs, &all_orbs));
            made.push((name, plays.len() - 1));
        }
    }
    for p in &mut plays {
        p.makes = p
            .text
            .makes
            .iter()
            .filter_map(|e| match e {
                Effect::Make { name, n, pile } => {
                    made.iter().find(|(m, _)| m == name).map(|&(_, card)| (card, *n, *pile))
                }
                _ => None,
            })
            .collect();
    }
    let mut relic_mods = Mods::default();
    for (_, m) in &relics {
        for p in &m.passives {
            relic_mods.add(p);
        }
    }
    let setup = Setup {
        plays: &plays,
        orbs: &orbs,
        groups: groups.len(),
        energy,
        orb_slots,
        arrows,
        vulnerable: catalog.rules.vulnerable_pct.map_or(0, |n| n as i32),
        relic_rules: relics.iter().flat_map(|(_, m)| m.triggers.iter()).collect(),
        relic_mods,
        rules,
        deck_len,
        forged_card,
        made,
        worth: setup_worth(&plays),
    };

    let turns = rules.turns as usize;
    let mut damage = vec![Vec::with_capacity(rules.trials as usize); turns];
    let mut block = vec![Vec::with_capacity(rules.trials as usize); turns];
    let mut turnover = 0usize;
    let mut drawn_by = vec![[0u32; ODDS_TURNS]; groups.len()];
    let (mut drawn, mut dead, mut unspent, mut stranded) = (0u32, 0u32, 0.0f32, 0.0f32);
    // Totals for PlayStats, playing for damage.
    let (mut hits, mut hit_damage, mut area, mut all_damage) = (0.0f32, 0.0f32, 0.0f32, 0.0f32);
    let (mut cards, mut skills, mut debuffs) = (0.0f32, 0.0f32, 0.0f32);
    let mut resources = [0.0f32; 6];
    let mut stores = [Flow::default(); 5];
    for trial in 0..rules.trials {
        let seed = 0x5EED_0000 ^ u64::from(trial);
        let attack = Fight::new(&setup, seed).run(Lean::Damage);
        let defend = Fight::new(&setup, seed).run(Lean::Block);
        for (t, (a, d)) in attack.turns.iter().zip(&defend.turns).enumerate() {
            damage[t].push(a.damage);
            block[t].push(d.block);
            hits += a.hits;
            hit_damage += a.attack_damage;
            area += a.area;
            all_damage += a.damage;
            cards += a.cards;
            skills += a.skills;
            debuffs += a.debuffs;
            drawn += a.drawn;
            dead += a.dead;
            unspent += a.unspent;
            stranded += a.stranded;
            for (k, r) in resources.iter_mut().enumerate() {
                *r += (a.resources[k] + d.resources[k]) / 2.0;
            }
            for store in Store::ALL {
                // Stretch is spent by Block cards: read it playing for block.
                let from = if store == Store::Stretch { d } else { a };
                let (f, to) = (&from.stores[store as usize], &mut stores[store as usize]);
                to.gained += f.gained;
                to.spent += f.spent;
                to.waiting += f.waiting;
                to.plays += f.plays;
            }
        }
        turnover += attack.turnover;
        for (g, first) in attack.first_drawn.iter().enumerate() {
            if let Some(t) = first {
                for slot in drawn_by[g].iter_mut().skip(*t) {
                    *slot += 1;
                }
            }
        }
    }

    let n = rules.trials as f32;
    let turn_count = n * turns as f32;
    let mut odds: Vec<DrawOdds> = groups
        .into_iter()
        .zip(drawn_by)
        .map(|((card, name, copies), by)| DrawOdds { card, name, copies, by_turn: by.map(|c| c as f32 / n) })
        .collect();
    odds.sort_by(|a, b| b.by_turn[0].total_cmp(&a.by_turn[0]).then_with(|| a.name.cmp(&b.name)));
    let (simulated, not): (Vec<_>, Vec<_>) = relics.into_iter().partition(|(_, m)| !m.is_empty());
    DeckSim {
        trials: rules.trials,
        hand_size: rules.hand_size,
        energy: energy as u32,
        turns: damage
            .into_iter()
            .zip(block)
            .map(|(mut d, mut b)| {
                d.sort_by(f32::total_cmp);
                b.sort_by(f32::total_cmp);
                TurnOutput { damage: Spread::of(&d), block: Spread::of(&b), block_steps: Steps::of(&b) }
            })
            .collect(),
        turnover: turnover as f32 / n,
        odds,
        dead_draws: if drawn == 0 { 0.0 } else { dead as f32 / drawn as f32 },
        unspent_energy: unspent / turn_count,
        stranded_energy: stranded / turn_count,
        resources: Resource::ALL
            .into_iter()
            .map(|r| (r, resources[r as usize] / turn_count))
            .filter(|(_, v)| *v >= 0.05)
            .collect(),
        play: PlayStats {
            hits: hits / turn_count,
            per_hit: if hits > 0.0 { hit_damage / hits } else { 0.0 },
            area_share: if all_damage > 0.0 { area / all_damage } else { 0.0 },
            cards: cards / turn_count,
            skills: skills / turn_count,
            debuffs: debuffs / turn_count,
        },
        stores: stores.map(|f| Flow {
            gained: f.gained / turn_count,
            spent: f.spent / turn_count,
            waiting: f.waiting / turn_count,
            plays: f.plays / turn_count,
        }),
        relics_simulated: simulated.into_iter().map(|(n, _)| n).collect(),
        relics_not_simulated: not.into_iter().map(|(n, _)| n).collect(),
    }
}
