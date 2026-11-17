//! Deck simulation: what the deck does turn by turn, read from the card data.
//!
//! Plays many seeded fights against a passive target: shuffle, draw, spend
//! energy greedily, and carry Strength, Dexterity, Focus, orbs, Poison, Aim,
//! Stretch, Vigor and Stars between turns. Two policies share the same
//! shuffles. One plays for damage and one plays for block, so each number
//! means "what the deck does if you lean that way this fight".
//!
//! Not modelled: enemy actions, relics, potions, enchantments, conditions
//! ("if this kills"), generated cards and random effects. The numbers are for
//! comparing decks and spotting gaps, not for predicting a fight.

use crate::catalog::{Card, CardType, Catalog, Orb, OrbEffect, Target};
use crate::playbook::{SimAssumptions, contains_ci};
use crate::run::{DeckCard, RunState};

/// A fight ends here even if the deck has not cycled (turnover is capped).
const MAX_TURNS: usize = 20;
/// Turns reported for draw odds.
const ODDS_TURNS: usize = 3;

/// A mean with the middle half of outcomes (25th to 75th percentile).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Spread {
    pub mean: f32,
    pub low: f32,
    pub high: f32,
}

impl Spread {
    fn of(mut values: Vec<f32>) -> Spread {
        if values.is_empty() {
            return Spread::default();
        }
        values.sort_by(f32::total_cmp);
        let n = values.len();
        Spread { mean: values.iter().sum::<f32>() / n as f32, low: values[n / 4], high: values[(3 * n) / 4] }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TurnOutput {
    /// Damage this turn when playing for damage.
    pub damage: Spread,
    /// Block this turn when playing for block.
    pub block: Spread,
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
    stars: i32,
    summon: i32,
    forge: i32,
    doom: i32,
    poison: i32,
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
    now: Effects,
    /// Repeats at the start of each turn once played ("At the start of your turn").
    each_turn: Option<Effects>,
    /// (orb index, count, once per energy)
    channels: Vec<(usize, i32, bool)>,
    /// Needs and spends an Arrow.
    shoots: bool,
    aim_multiplier: i32,
    /// Brings a stuck Arrow back.
    dislodges: bool,
    /// Arrows the card adds for the fight (Quiver).
    extra_arrows: i32,
    attack: bool,
    removed_on_play: bool,
    ethereal: bool,
    innate: bool,
    retain: bool,
}

fn play_of(card: &Card, upgraded: bool, group: usize, orbs: &[&Orb]) -> Play {
    let v = |k: &str| card.value(k, upgraded);
    let either = |a: &str, b: &str| v(a).max(v(b));
    let power = card.card_type == CardType::Power;
    let on_self = power || matches!(card.target, Target::Own | Target::None | Target::AllAllies);
    let base_stars = card.value("stars", false);
    let mut e = Effects {
        damage: v("damage"),
        hits: v("hitcount").max(1),
        osty: v("ostydamage"),
        block: v("block"),
        draw: if card.draw.is_some() { v("cards") } else { 0 },
        energy: if card.energy.is_some() { v("energy") } else { 0 },
        stars: if base_stars > 0 && contains_ci(&card.text, &format!("gain {base_stars} star")) {
            v("stars")
        } else {
            0
        },
        summon: v("summon"),
        forge: v("forge"),
        doom: either("doom", "doompower"),
        poison: either("poison", "poisonpower").max(v("poisonperturn")),
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
    let recurring = power && contains_ci(&card.text, "at the start of your turn");
    let (now, each_turn) = if recurring {
        (Effects::default(), Some(e))
    } else if power {
        // A power's damage or block numbers are triggers, not an immediate effect.
        (Effects { damage: 0, block: 0, osty: 0, ..e }, None)
    } else {
        (e, None)
    };
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
        now,
        each_turn,
        channels: card
            .channels
            .iter()
            .filter_map(|c| Some((orbs.iter().position(|o| o.id == c.orb)?, i32::from(c.count), c.x)))
            .collect(),
        shoots: contains_ci(&card.text, "shoot an arrow"),
        aim_multiplier: v("aimmultiplier").max(1),
        dislodges: card.card_type == CardType::Attack
            && contains_ci(&card.text, "dislodge")
            && !contains_ci(&card.text, "shoot"),
        extra_arrows: if contains_ci(&card.text, "additional arrow") { v("repeat").max(1) } else { 0 },
        attack: card.card_type == CardType::Attack,
        removed_on_play: power || card.has_keyword("Exhaust", upgraded),
        ethereal: card.has_keyword("Ethereal", upgraded),
        innate: card.has_keyword("Innate", upgraded),
        retain: card.has_keyword("Retain", upgraded),
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
    block: f32,
    unspent: f32,
    stranded: f32,
    drawn: u32,
    dead: u32,
    resources: [f32; 6],
}

struct Setup<'a> {
    plays: &'a [Play],
    orbs: &'a [&'a Orb],
    groups: usize,
    energy: i32,
    orb_slots: usize,
    arrows: i32,
    rules: &'a SimAssumptions,
}

struct Fight<'a> {
    s: &'a Setup<'a>,
    rng: Rng,
    draw: Vec<usize>,
    discard: Vec<usize>,
    hand: Vec<usize>,
    unseen: Vec<bool>,
    unseen_left: usize,
    first_drawn: Vec<Option<usize>>,
    recurring: Vec<usize>,
    orbs: Vec<OrbState>,
    strength: i32,
    dexterity: i32,
    focus: i32,
    aim: i32,
    stretch: i32,
    vigor: i32,
    poison: i32,
    stars: i32,
    /// Arrows ready to shoot, and shot Arrows stuck in the enemy.
    arrows: i32,
    stuck: i32,
    hits_toward_return: i32,
    energy: i32,
    next_energy: i32,
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
        let mut draw: Vec<usize> = (0..s.plays.len()).collect();
        rng.shuffle(&mut draw);
        // Innate cards go on top (the end of the vector is the top).
        draw.sort_by_key(|&i| s.plays[i].innate);
        Fight {
            s,
            rng,
            draw,
            discard: Vec::new(),
            hand: Vec::new(),
            unseen: vec![true; s.plays.len()],
            unseen_left: s.plays.len(),
            first_drawn: vec![None; s.groups],
            recurring: Vec::new(),
            orbs: Vec::new(),
            strength: 0,
            dexterity: 0,
            focus: 0,
            aim: 0,
            stretch: 0,
            vigor: 0,
            poison: 0,
            stars: 0,
            arrows: s.arrows,
            stuck: 0,
            hits_toward_return: 0,
            energy: 0,
            next_energy: 0,
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
                self.draw.append(&mut self.discard);
                self.rng.shuffle(&mut self.draw);
            }
            let Some(i) = self.draw.pop() else { return };
            let p = &self.s.plays[i];
            if self.unseen[i] {
                self.unseen[i] = false;
                self.unseen_left -= 1;
            }
            self.first_drawn[p.group].get_or_insert(self.turn);
            self.log.drawn += 1;
            if p.cost.is_none() {
                self.log.dead += 1;
            }
            self.hand.push(i);
        }
    }

    fn orb_value(&self, o: OrbState, base: i32) -> i32 {
        if self.s.orbs[o.orb].focus { (base + self.focus).max(0) } else { base }
    }

    fn orb_effect(&mut self, effect: OrbEffect, amount: i32, now: bool) {
        match effect {
            OrbEffect::Damage | OrbEffect::DamageAll => self.log.damage += amount as f32,
            OrbEffect::Block => self.log.block += amount as f32,
            OrbEffect::Energy if now => self.energy += amount,
            OrbEffect::Energy => self.next_energy += amount,
            OrbEffect::Grow | OrbEffect::Other => {}
        }
    }

    fn channel(&mut self, orb: usize) {
        if self.s.orb_slots == 0 {
            return;
        }
        if self.orbs.len() >= self.s.orb_slots {
            let o = self.orbs.remove(0);
            let def = self.s.orbs[o.orb];
            self.orb_effect(def.evoke_effect, self.orb_value(o, o.evoke), true);
        }
        let def = self.s.orbs[orb];
        self.orbs.push(OrbState { orb, passive: def.passive, evoke: def.evoke });
        self.log.resources[Resource::Orbs as usize] += 1.0;
    }

    /// Applies effects once (or `times` times for X costs).
    fn apply(&mut self, e: &Effects, times: i32, shoots: bool, aim_multiplier: i32) {
        if e.damage > 0 {
            let mut dmg = (e.damage + self.strength).max(0) * e.hits * times + self.vigor;
            self.vigor = 0;
            if shoots {
                dmg += self.aim * aim_multiplier;
                self.aim = 0;
            }
            self.log.damage += dmg as f32;
        }
        self.log.damage += (e.osty * times) as f32;
        if e.block > 0 {
            self.log.block += ((e.block + self.dexterity).max(0) * times + self.stretch) as f32;
            self.stretch = 0;
        }
        self.energy += e.energy;
        self.stars += e.stars * times;
        self.strength += e.strength;
        self.dexterity += e.dexterity;
        self.focus += e.focus;
        self.aim += e.aim * times;
        self.stretch += e.stretch * times;
        self.vigor += e.vigor * times;
        self.poison += e.poison * times;
        let r = &mut self.log.resources;
        r[Resource::Stars as usize] += (e.stars * times) as f32;
        r[Resource::Summon as usize] += (e.summon * times) as f32;
        r[Resource::Forge as usize] += (e.forge * times) as f32;
        r[Resource::Doom as usize] += (e.doom * times) as f32;
        r[Resource::Poison as usize] += (e.poison * times) as f32;
        if e.draw > 0 {
            self.draw_cards(e.draw * times);
        }
    }

    /// How much a play is worth to the current lean, per energy. Setup and
    /// acceleration are given rough exchange rates so they get played.
    fn score(&self, p: &Play, lean: Lean) -> f32 {
        let times = if p.x { self.energy } else { 1 };
        if times == 0 {
            return 0.0;
        }
        let e = &p.now;
        let t = times as f32;
        let attack = if e.damage > 0 { ((e.damage + self.strength).max(0) * e.hits) as f32 * t } else { 0.0 }
            + (e.osty as f32
                + e.poison as f32 * 1.5
                + e.doom as f32 * 0.5
                + e.forge as f32
                + e.aim as f32
                + e.vigor as f32)
                * t
            + if p.shoots { (self.aim * p.aim_multiplier) as f32 } else { 0.0 };
        let defend = if e.block > 0 { (e.block + self.dexterity).max(0) as f32 * t } else { 0.0 }
            + (e.stretch + e.summon) as f32 * t;
        let (main, side) = match lean {
            Lean::Damage => (attack, defend * 0.1),
            Lean::Block => (defend, attack * 0.1),
        };
        let setup = (e.strength + e.dexterity) as f32 * 4.0
            + e.focus as f32 * 3.0 * self.s.orb_slots as f32
            + e.draw as f32 * 3.0 * t
            + e.energy as f32 * 4.0
            + e.stars as f32 * 2.0 * t
            + if p.each_turn.is_some() { 8.0 } else { 0.0 }
            + p.channels
                .iter()
                .map(|&(o, n, x)| {
                    let n = if x { n * times } else { n };
                    let orb = self.s.orbs[o];
                    (orb.passive.max(1) * 2 + orb.evoke) as f32 * n as f32 * 0.5
                })
                .sum::<f32>();
        let value = main + side + setup;
        let cost = p.cost.unwrap_or(0).max(if p.x { self.energy } else { 0 });
        value / (cost as f32).max(0.5)
    }

    fn affordable(&self, p: &Play) -> bool {
        match p.cost {
            Some(c) => c <= self.energy && p.star_cost <= self.stars && (!p.shoots || self.arrows > 0),
            None => false,
        }
    }

    /// Shooting sticks an Arrow in the enemy; further hits or a Dislodge bring it back.
    fn arrows_after(&mut self, p: &Play, times: i32) {
        self.arrows += p.extra_arrows;
        if p.shoots {
            self.arrows -= 1;
            self.stuck += 1;
            return;
        }
        if self.stuck == 0 {
            return;
        }
        if p.dislodges {
            self.stuck -= 1;
            self.arrows += 1;
        } else if p.attack && p.now.damage > 0 {
            let per = self.s.rules.arrow_return_hits.max(1) as i32;
            self.hits_toward_return += p.now.hits * times;
            while self.hits_toward_return >= per && self.stuck > 0 {
                self.hits_toward_return -= per;
                self.stuck -= 1;
                self.arrows += 1;
            }
        }
    }

    fn play_turn(&mut self, lean: Lean) {
        loop {
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
            // The plays outlive this fight, so this borrow doesn't hold `self`.
            let setup: &'a Setup<'a> = self.s;
            let p = &setup.plays[i];
            let times = if p.x { self.energy } else { 1 };
            self.energy -= if p.x { self.energy } else { p.cost.unwrap_or(0) };
            self.stars -= p.star_cost;
            if p.each_turn.is_some() {
                self.recurring.push(i);
            }
            for &(orb, n, x) in &p.channels {
                for _ in 0..if x { n * times } else { n } {
                    self.channel(orb);
                }
            }
            self.arrows_after(p, times);
            self.apply(&p.now, times, p.shoots, p.aim_multiplier);
            // A card resolves before it is discarded, so its own draw can't
            // shuffle it back into the draw pile.
            if !p.removed_on_play {
                self.discard.push(i);
            }
        }
    }

    fn end_turn(&mut self) {
        for k in 0..self.orbs.len() {
            let o = self.orbs[k];
            let def = self.s.orbs[o.orb];
            let amount = self.orb_value(o, o.passive);
            if def.passive_effect == OrbEffect::Grow {
                self.orbs[k].evoke += amount;
            } else {
                self.orb_effect(def.passive_effect, amount, false);
            }
        }
        // Poison ticks at the start of the enemy's turn, then drops by one.
        self.log.damage += self.poison as f32;
        self.poison = (self.poison - 1).max(0);
        self.log.unspent = self.energy as f32;
        self.log.stranded = self
            .hand
            .iter()
            .filter_map(|&i| self.s.plays[i].cost.filter(|c| *c > self.energy && !self.s.plays[i].x))
            .sum::<i32>() as f32;
        let hand = std::mem::take(&mut self.hand);
        for i in hand {
            let p = &self.s.plays[i];
            if p.retain {
                self.hand.push(i);
            } else if !p.ethereal {
                self.discard.push(i);
            }
        }
    }

    fn run(mut self, lean: Lean) -> FightResult {
        let reported = self.s.rules.turns as usize;
        let mut turns = Vec::with_capacity(reported);
        let mut turnover = None;
        while self.turn < MAX_TURNS && (self.turn < reported || turnover.is_none()) {
            self.log = TurnLog::default();
            self.energy = self.s.energy + std::mem::take(&mut self.next_energy);
            self.draw_cards(self.s.rules.hand_size as i32);
            let setup: &'a Setup<'a> = self.s;
            for k in 0..self.recurring.len() {
                if let Some(e) = &setup.plays[self.recurring[k]].each_turn {
                    self.apply(e, 1, false, 1);
                }
            }
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

/// Simulates the current deck. Deterministic: the same deck gives the same numbers.
pub fn simulate(state: &RunState, catalog: &Catalog, rules: &SimAssumptions) -> DeckSim {
    let character = state.character.as_ref().and_then(|c| catalog.characters.get(c));
    let energy = character.map_or(3, |c| c.energy) as i32;
    let orb_slots = character.and_then(|c| c.orb_slots).unwrap_or(0) as usize;
    let arrows = character.and_then(|c| c.arrow_slots).unwrap_or(0) as i32;
    let orbs: Vec<&Orb> = catalog.orbs.values().collect();

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
        plays.push(play_of(card, d.upgraded, group, &orbs));
    }
    if plays.is_empty() || rules.trials == 0 || rules.turns == 0 {
        return DeckSim::default();
    }
    let setup = Setup { plays: &plays, orbs: &orbs, groups: groups.len(), energy, orb_slots, arrows, rules };

    let turns = rules.turns as usize;
    let mut damage = vec![Vec::with_capacity(rules.trials as usize); turns];
    let mut block = vec![Vec::with_capacity(rules.trials as usize); turns];
    let mut turnover = 0usize;
    let mut drawn_by = vec![[0u32; ODDS_TURNS]; groups.len()];
    let (mut drawn, mut dead, mut unspent, mut stranded) = (0u32, 0u32, 0.0f32, 0.0f32);
    let mut resources = [0.0f32; 6];
    for trial in 0..rules.trials {
        let seed = 0x5EED_0000 ^ u64::from(trial);
        let attack = Fight::new(&setup, seed).run(Lean::Damage);
        let defend = Fight::new(&setup, seed).run(Lean::Block);
        for (t, (a, d)) in attack.turns.iter().zip(&defend.turns).enumerate() {
            damage[t].push(a.damage);
            block[t].push(d.block);
            drawn += a.drawn;
            dead += a.dead;
            unspent += a.unspent;
            stranded += a.stranded;
            for (k, r) in resources.iter_mut().enumerate() {
                *r += (a.resources[k] + d.resources[k]) / 2.0;
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
    DeckSim {
        trials: rules.trials,
        hand_size: rules.hand_size,
        energy: energy as u32,
        turns: damage
            .into_iter()
            .zip(block)
            .map(|(d, b)| TurnOutput { damage: Spread::of(d), block: Spread::of(b) })
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
    }
}
