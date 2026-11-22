//! Reading combat effects from card and relic text.
//!
//! The data gives numbers for a card's own effect (`vars`), but powers and
//! relics describe what they do in prose: "At the start of your turn, gain 3
//! Strength", "Whenever you play an Attack, gain 1 Block". This module reads
//! that prose into timed triggers ([`When`] with its [`Effect`]s) and
//! [`Passive`]s the deck simulator can run.
//!
//! The reader is deliberately small: a timing phrase, then simple effects
//! ("gain N X", "deal N damage", "draw N cards", "apply N X", "Channel N X").
//! Every sentence it does not understand is kept in [`Mechanics::unread`], so
//! the app can say exactly what is and isn't simulated. Conditions ("if…",
//! "the first time…") are left unread rather than guessed.

use crate::catalog::{CardType, Orb};
use crate::ids::OrbId;

/// Which played cards a trigger or passive applies to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CardFilter {
    Any,
    Type(CardType),
    /// Cards costing at least this much energy.
    CostAtLeast(u8),
    /// Cards whose name contains this (lowercase): "strike", "shiv".
    Named(String),
    /// Upgraded cards of a type.
    Upgraded(CardType),
    /// Cards with a keyword (lowercase): "ethereal".
    Keyword(String),
    /// Cards that cost 0.
    ZeroCost,
    /// Cards that Shoot an Arrow.
    Shooter,
    /// Cards whose name contains any of these (lowercase).
    AnyNamed(Vec<String>),
}

impl CardFilter {
    pub fn describe(&self) -> String {
        match self {
            CardFilter::Any => "a card".into(),
            CardFilter::Type(t) => {
                format!("an {}", t.label()).replace("an Skill", "a Skill").replace("an Power", "a Power")
            }
            CardFilter::CostAtLeast(n) => format!("a card costing {n}+"),
            CardFilter::Named(n) => format!("a card named \"{n}\""),
            CardFilter::Upgraded(t) => format!("an upgraded {}", t.label()),
            CardFilter::Keyword(k) => format!("an {k} card"),
            CardFilter::ZeroCost => "a 0-cost card".into(),
            CardFilter::Shooter => "a card that can Shoot".into(),
            CardFilter::AnyNamed(names) => format!("a card named {}", names.join(" or ")),
        }
    }
}

/// When a trigger fires.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum When {
    /// Once, when the card is played (a power's own effect).
    Played,
    CombatStart,
    TurnStart,
    /// The start of one turn (1-based).
    TurnNumber(u32),
    /// The start of each of the first N turns.
    FirstTurns(u32),
    /// The start of turn N and every turn after.
    TurnsFrom(u32),
    TurnEnd,
    /// The start of every Nth turn.
    EveryTurns(u32),
    /// After each matching card is played.
    OnPlay(CardFilter),
    /// After every Nth matching card; `per_turn` counts restart each turn.
    EveryPlays {
        filter: CardFilter,
        n: u32,
        per_turn: bool,
    },
    OnExhaust,
    OnShuffle,
    OnGainBlock,
    OnDraw,
    /// Drawing a matching card ("Whenever you draw an Ethereal card").
    OnDrawOf(CardFilter),
    /// Applying a debuff to an enemy.
    OnApply(Debuff),
    /// Creating a matching card ("Whenever you create a Status").
    OnMake(CardFilter),
    /// Gaining or spending Stars; `each` repeats the effects per Star.
    OnStars {
        gain: bool,
        spend: bool,
        each: bool,
    },
    /// An orb (of this kind, or any) being evoked.
    OnEvoke(Option<OrbId>),
    /// Losing HP yourself.
    OnHpLoss,
    OnDislodge,
    /// Shooting an Arrow (which Impales the enemy).
    OnShoot,
    OnOstyAttack,
    OnDiscard,
    /// Each hit of an attack ("Whenever an Attack deals unblocked damage").
    OnAttackHit,
    /// Every N of something counted over the combat.
    Every {
        what: Counted,
        n: u32,
    },
    /// Gaining a Melody (of this kind, or any).
    OnMelody(Option<Melody>),
    /// Holding every Melody, the moment it happens.
    OnAllMelodies,
    /// Only the first time the trigger fires each turn (or each combat).
    First {
        when: Box<When>,
        per_turn: bool,
    },
}

/// One of The Fool's Melodies, gained by playing a Wah, Cho or Ki card
/// (per community guides; the card data names the Melodies, not the rule).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Melody {
    Wah,
    Cho,
    Ki,
}

impl Melody {
    pub const ALL: [Melody; 3] = [Melody::Wah, Melody::Cho, Melody::Ki];

    fn named(w: &str) -> Option<Melody> {
        match w {
            "wah" => Some(Melody::Wah),
            "cho" => Some(Melody::Cho),
            "ki" => Some(Melody::Ki),
            _ => None,
        }
    }

    /// The Melodies a card's name gives ("Wah Cho" gives two).
    pub fn of_card(name: &str) -> Vec<Melody> {
        name.split_whitespace().filter_map(|w| Melody::named(&w.to_lowercase())).collect()
    }
}

/// A debuff a trigger watches for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Debuff {
    Vulnerable,
    Weak,
    Poison,
    Doom,
    /// Any debuff.
    Any,
}

/// What an [`When::Every`] trigger counts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Counted {
    Exhausts,
    EnergySpent,
    StarsSpent,
    CardsDrawn,
}

impl When {
    pub fn describe(&self) -> String {
        match self {
            When::Played => "When played".into(),
            When::CombatStart => "Combat start".into(),
            When::TurnStart => "Each turn".into(),
            When::TurnNumber(n) => format!("Turn {n}"),
            When::FirstTurns(n) => format!("First {n} turns"),
            When::TurnsFrom(n) => format!("Turn {n} onward"),
            When::TurnEnd => "End of each turn".into(),
            When::EveryTurns(n) => format!("Every {n} turns"),
            When::OnPlay(f) => format!("Whenever you play {}", f.describe()),
            When::EveryPlays { filter, n, per_turn } => {
                let scope = if *per_turn { " in a turn" } else { "" };
                format!("Every {n} plays of {}{scope}", filter.describe())
            }
            When::OnExhaust => "Whenever a card is exhausted".into(),
            When::OnShuffle => "Whenever you shuffle".into(),
            When::OnGainBlock => "Whenever you gain Block".into(),
            When::OnDraw => "Whenever you draw a card".into(),
            When::OnDrawOf(f) => format!("Whenever you draw {}", f.describe()),
            When::OnApply(d) => format!("Whenever you apply {d:?}"),
            When::OnMake(f) => format!("Whenever you create {}", f.describe()),
            When::OnStars { gain, spend, each } => {
                let what = match (gain, spend) {
                    (true, true) => "gain or spend",
                    (true, false) => "gain",
                    _ => "spend",
                };
                format!("Whenever you {what} Stars{}", if *each { " (per Star)" } else { "" })
            }
            When::OnEvoke(orb) => match orb {
                Some(o) => format!("Whenever you evoke {}", o.as_str().trim_end_matches("_ORB").to_lowercase()),
                None => "Whenever you evoke an orb".into(),
            },
            When::OnHpLoss => "Whenever you lose HP".into(),
            When::OnDislodge => "Whenever you dislodge an Arrow".into(),
            When::OnShoot => "Whenever you shoot".into(),
            When::OnOstyAttack => "Whenever Osty attacks".into(),
            When::OnDiscard => "Whenever you discard a card".into(),
            When::OnAttackHit => "Whenever an attack hits".into(),
            When::Every { what, n } => format!("Every {n} {what:?}"),
            When::OnMelody(m) => match m {
                Some(m) => format!("Whenever you gain a {m:?} Melody"),
                None => "Whenever you gain a Melody".into(),
            },
            When::OnAllMelodies => "When you gain every Melody".into(),
            When::First { when, per_turn } => {
                format!("First time each {}: {}", if *per_turn { "turn" } else { "combat" }, when.describe())
            }
        }
    }
}

/// One simple effect.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Effect {
    Block(i32),
    Damage(i32),
    DamageAll(i32),
    Energy(i32),
    Draw(i32),
    Strength(i32),
    Dexterity(i32),
    Focus(i32),
    Vigor(i32),
    Aim(i32),
    Stretch(i32),
    Plating(i32),
    Poison(i32),
    Doom(i32),
    /// Turns of Vulnerable on the enemy.
    Vulnerable(i32),
    /// Turns of Weak on the enemy.
    Weak(i32),
    Forge(i32),
    Stars(i32),
    Summon(i32),
    Channel(OrbId, i32),
    OrbSlots(i32),
    /// Arrows for the Fool.
    Arrows(i32),
    /// HP you lose yourself ("Lose 2 HP", "take 2 damage").
    LoseHp(i32),
    /// Evoke orbs `times` times (-1: X times).
    Evoke(OrbPick, i32),
    /// Trigger orbs' passives `times` times.
    OrbPassive(OrbPick, i32),
    /// Discard N cards of your choice from your hand.
    Discard(i32),
    /// Exhaust N cards (`up_to`: as many as you like, up to N), chosen or
    /// at random, from your hand or Draw Pile.
    Exhaust {
        n: i32,
        random: bool,
        from: Pile,
    },
    /// Osty dies.
    OstyDies,
    /// Moves N matching cards (-1: all) from a pile to the hand, or to the
    /// top of the Draw Pile (`to: Draw`).
    Fetch {
        from: Pile,
        to: Pile,
        n: i32,
        filter: CardFilter,
    },
    /// Plays N matching cards (-1: X, or all) for free; `exhaust` exhausts them after.
    AutoPlay {
        from: Source,
        n: i32,
        filter: CardFilter,
        exhaust: bool,
    },
    /// Adds N random cards (-1: X) from a pool; `free` makes them cost 0 until played.
    MakeRandom {
        pool: Pool,
        n: i32,
        pile: Pile,
        free: bool,
    },
    /// Cards made and put in a pile: "add 1 Shiv into your Hand". `name`
    /// is as written (plural or not); the simulator finds the card.
    Make {
        name: String,
        n: i32,
        pile: Pile,
    },
}

/// Where a random card comes from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Pool {
    /// The character's own cards (of one type, or any).
    Class(Option<CardType>),
    Colorless,
    /// Cards that match a filter, from the character's own cards.
    ClassWith(CardFilter),
    /// The Tarot cards Mythic Form makes.
    Tarot,
}

/// Where cards are played from by "Play the top card…" and the like.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    DrawTop,
    /// At random from a pile.
    Draw,
    Discard,
    Hand,
    /// The cheapest card in the hand.
    HandCheapest,
    /// Every matching card in the Exhaust Pile (they stay there).
    Exhaust,
}

/// Which orbs an evoke or passive trigger takes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OrbPick {
    Rightmost,
    Leftmost,
    All,
    /// Every orb of one kind.
    Kind(OrbId),
}

/// Where a made or moved card goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pile {
    Hand,
    Draw,
    Discard,
}

impl Effect {
    pub fn describe(&self) -> String {
        let signed =
            |n: i32, what: &str| if n < 0 { format!("lose {} {what}", -n) } else { format!("gain {n} {what}") };
        match self {
            Effect::Block(n) => signed(*n, "Block"),
            Effect::Damage(n) => format!("deal {n} damage"),
            Effect::DamageAll(n) => format!("deal {n} damage to all"),
            Effect::Energy(n) => signed(*n, "Energy"),
            Effect::Draw(n) => format!("draw {n}"),
            Effect::Strength(n) => signed(*n, "Strength"),
            Effect::Dexterity(n) => signed(*n, "Dexterity"),
            Effect::Focus(n) => signed(*n, "Focus"),
            Effect::Vigor(n) => signed(*n, "Vigor"),
            Effect::Aim(n) => signed(*n, "Aim"),
            Effect::Stretch(n) => signed(*n, "Stretch"),
            Effect::Plating(n) => signed(*n, "Plating"),
            Effect::Poison(n) => format!("apply {n} Poison"),
            Effect::Doom(n) => format!("apply {n} Doom"),
            Effect::Vulnerable(n) => format!("apply {n} Vulnerable"),
            Effect::Weak(n) => format!("apply {n} Weak"),
            Effect::Forge(n) => format!("Forge {n}"),
            Effect::Stars(n) => signed(*n, "Stars"),
            Effect::Summon(n) => format!("Summon {n}"),
            Effect::Channel(orb, n) => format!("Channel {n} {}", orb.as_str().trim_end_matches("_ORB").to_lowercase()),
            Effect::OrbSlots(n) => signed(*n, "orb slots"),
            Effect::Arrows(n) => signed(*n, "Arrows"),
            Effect::LoseHp(n) => format!("lose {n} HP"),
            Effect::Evoke(pick, n) => format!("evoke {pick:?} ×{n}"),
            Effect::OrbPassive(pick, n) => format!("trigger {pick:?} passive ×{n}"),
            Effect::Discard(n) => format!("discard {n}"),
            Effect::Exhaust { n, random, .. } => format!("exhaust {n}{}", if *random { " at random" } else { "" }),
            Effect::OstyDies => "Osty dies".into(),
            Effect::Fetch { from, to, n, filter } => {
                format!("put {n} {} from {from:?} into {to:?}", filter.describe())
            }
            Effect::AutoPlay { from, n, filter, .. } => format!("play {n} {} from {from:?}", filter.describe()),
            Effect::MakeRandom { pool, n, pile, free } => {
                format!("add {n} random {pool:?} to {pile:?}{}", if *free { ", free" } else { "" })
            }
            Effect::Make { name, n, pile } => format!("add {n} {name} to your {pile:?}"),
        }
    }
}

/// A rule that changes how other cards work, rather than doing something itself.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Passive {
    /// Matching attacks deal this much more damage.
    ExtraDamage(CardFilter, i32),
    /// Osty's attacks deal this much more damage.
    ExtraOstyDamage(i32),
    /// Matching cards give this much more Block.
    ExtraBlock(CardFilter, i32),
    /// Vulnerable adds this many more percent.
    VulnerableBonus(i32),
    /// Vulnerable is this many percent instead of the game's default.
    VulnerableIs(i32),
    /// Poison ticks this many extra times.
    PoisonExtraTicks(i32),
    /// Each Poison applied is this much more.
    PoisonBonus(i32),
    /// Weak is this many percent instead of the game's default.
    WeakIs(i32),
    /// The first N cards each turn are free.
    FreeFirst(i32),
    /// The hand is not discarded at end of turn.
    RetainHand,
    /// Unspent energy carries over.
    ConserveEnergy,
    /// Skills cost 0 and exhaust.
    FreeSkills,
    /// Multi-hit attacks hit once more.
    ExtraHit,
    /// The first card each turn is played twice.
    EchoEachTurn,
    /// The first card of the combat is played twice.
    EchoFirstOfCombat,
    /// The first attack each turn deals this many percent more.
    FirstAttackBonus(i32),
}

impl Passive {
    pub fn describe(&self) -> String {
        match self {
            Passive::ExtraDamage(f, n) => format!("{} deals {n} more damage", f.describe()),
            Passive::ExtraOstyDamage(n) => format!("Osty deals {n} more damage"),
            Passive::ExtraBlock(f, n) => format!("{} gives {n} more Block", f.describe()),
            Passive::VulnerableBonus(n) => format!("Vulnerable +{n}%"),
            Passive::VulnerableIs(n) => format!("Vulnerable is {n}%"),
            Passive::PoisonExtraTicks(n) => format!("Poison ticks {n} more time(s)"),
            Passive::PoisonBonus(n) => format!("Poison applied +{n}"),
            Passive::WeakIs(n) => format!("Weak is {n}%"),
            Passive::FreeFirst(n) => format!("First {n} cards each turn are free"),
            Passive::RetainHand => "Keep your hand".into(),
            Passive::ConserveEnergy => "Unspent Energy carries over".into(),
            Passive::FreeSkills => "Skills cost 0 and exhaust".into(),
            Passive::ExtraHit => "Multi-hit attacks hit once more".into(),
            Passive::EchoEachTurn => "First card each turn plays twice".into(),
            Passive::EchoFirstOfCombat => "First card of combat plays twice".into(),
            Passive::FirstAttackBonus(n) => format!("First attack each turn +{n}%"),
        }
    }
}

/// What the simulator understood of a text, and what it didn't.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Mechanics {
    pub triggers: Vec<(When, Vec<Effect>)>,
    pub passives: Vec<Passive>,
    /// Sentences left unread, as written.
    pub unread: Vec<String>,
}

impl Effect {
    /// The effect's name in the playbook's relic role rules.
    pub fn key(&self) -> &'static str {
        match self {
            Effect::Block(_) => "block",
            Effect::Damage(_) => "damage",
            Effect::DamageAll(_) => "damage_all",
            Effect::Energy(_) => "energy",
            Effect::Draw(_) => "draw",
            Effect::Strength(_) => "strength",
            Effect::Dexterity(_) => "dexterity",
            Effect::Focus(_) => "focus",
            Effect::Vigor(_) => "vigor",
            Effect::Aim(_) => "aim",
            Effect::Stretch(_) => "stretch",
            Effect::Plating(_) => "plating",
            Effect::Poison(_) => "poison",
            Effect::Doom(_) => "doom",
            Effect::Vulnerable(_) => "vulnerable",
            Effect::Weak(_) => "weak",
            Effect::Forge(_) => "forge",
            Effect::Stars(_) => "stars",
            Effect::Summon(_) => "summon",
            Effect::Channel(..) => "channel",
            Effect::OrbSlots(_) => "orb_slots",
            Effect::Arrows(_) => "arrows",
            Effect::LoseHp(_) => "hp_loss",
            Effect::Evoke(..) => "evoke",
            Effect::OrbPassive(..) => "orb_passive",
            Effect::Discard(_) => "discard",
            Effect::Exhaust { .. } => "exhaust",
            Effect::OstyDies => "osty_dies",
            Effect::Fetch { .. } => "fetch",
            Effect::AutoPlay { .. } => "autoplay",
            Effect::MakeRandom { .. } => "random_card",
            Effect::Make { .. } => "make",
        }
    }

    /// Gains and applications, as opposed to losses ("lose 1 Strength").
    fn positive(&self) -> bool {
        match self {
            Effect::Block(n)
            | Effect::Energy(n)
            | Effect::Draw(n)
            | Effect::Strength(n)
            | Effect::Dexterity(n)
            | Effect::Focus(n)
            | Effect::Vigor(n)
            | Effect::Aim(n)
            | Effect::Stretch(n)
            | Effect::Plating(n)
            | Effect::Stars(n)
            | Effect::OrbSlots(n)
            | Effect::Arrows(n) => *n > 0,
            _ => true,
        }
    }
}

impl Passive {
    /// The passive's name in the playbook's relic role rules.
    pub fn key(&self) -> &'static str {
        match self {
            Passive::ExtraDamage(..) | Passive::ExtraOstyDamage(_) => "extra_damage",
            Passive::ExtraBlock(..) => "extra_block",
            Passive::VulnerableBonus(_) | Passive::VulnerableIs(_) => "vulnerable_bonus",
            Passive::PoisonExtraTicks(_) => "poison_ticks",
            Passive::PoisonBonus(_) => "poison_bonus",
            Passive::WeakIs(_) => "weak_is",
            Passive::FreeFirst(_) => "free_first",
            Passive::RetainHand => "retain_hand",
            Passive::ConserveEnergy => "conserve_energy",
            Passive::FreeSkills => "free_skills",
            Passive::ExtraHit => "extra_hit",
            Passive::EchoEachTurn | Passive::EchoFirstOfCombat => "echo",
            Passive::FirstAttackBonus(_) => "first_attack_bonus",
        }
    }
}

impl When {
    /// Happens once a combat (or once when played), rather than again
    /// and again.
    pub fn once(&self) -> bool {
        matches!(self, When::Played | When::CombatStart | When::TurnNumber(_))
    }
}

impl Mechanics {
    /// What was understood, as `(name, recurring)`: each gained or applied
    /// effect with whether it repeats, then each passive (always on).
    pub fn facts(&self) -> Vec<(&'static str, bool)> {
        let mut out: Vec<(&'static str, bool)> = self
            .triggers
            .iter()
            .flat_map(|(w, es)| es.iter().filter(|e| e.positive()).map(move |e| (e.key(), !w.once())))
            .collect();
        out.extend(self.passives.iter().map(|p| (p.key(), true)));
        out
    }

    pub fn is_empty(&self) -> bool {
        self.triggers.is_empty() && self.passives.is_empty()
    }

    /// One line per understood rule ("Each turn: gain 3 Strength").
    pub fn describe(&self) -> Vec<String> {
        let mut out: Vec<String> = self
            .triggers
            .iter()
            .map(|(w, es)| {
                let what: Vec<String> = es.iter().map(Effect::describe).collect();
                format!("{}: {}", w.describe(), what.join(", "))
            })
            .collect();
        out.extend(self.passives.iter().map(Passive::describe));
        out
    }
}

// ---------------------------------------------------------------- reading

fn words(s: &str) -> Vec<String> {
    s.split_whitespace()
        .map(|w| {
            w.trim_matches(|c: char| !c.is_alphanumeric() && c != '%' && c != '\'' && c != '’')
                .to_lowercase()
                .replace('’', "'")
        })
        .filter(|w| !w.is_empty())
        .collect()
}

fn number(w: &str) -> Option<i32> {
    w.trim_end_matches('%').parse().ok()
}

/// "2nd" → 2, "first" → 1.
fn ordinal(w: &str) -> Option<u32> {
    match w {
        "first" => Some(1),
        "second" => Some(2),
        "third" => Some(3),
        _ => w.trim_end_matches(|c: char| c.is_alphabetic()).parse().ok(),
    }
}

fn card_type(w: &str) -> Option<CardType> {
    match w.trim_end_matches('s') {
        "attack" => Some(CardType::Attack),
        "skill" => Some(CardType::Skill),
        "power" => Some(CardType::Power),
        "statu" | "statuse" => Some(CardType::Status),
        _ => None,
    }
}

/// "a card", "an attack", "a shiv", "a card that costs 2 energy or more".
fn filter(ws: &[String]) -> Option<CardFilter> {
    let w = ws.iter().map(String::as_str).find(|w| !matches!(*w, "a" | "an"))?;
    if w == "card" || w == "cards" {
        if let Some(i) = ws.iter().position(|w| w == "costs")
            && let Some(n) = ws.get(i + 1).and_then(|n| number(n))
        {
            return Some(CardFilter::CostAtLeast(n.max(0) as u8));
        }
        return Some(CardFilter::Any);
    }
    if let Some(t) = card_type(w) {
        return Some(CardFilter::Type(t));
    }
    if w == "ethereal" {
        return Some(CardFilter::Keyword("ethereal".into()));
    }
    if w == "sovereign" {
        return Some(CardFilter::Named("sovereign blade".into()));
    }
    let name = match w {
        "sadness" => "sadness",
        w => w.trim_end_matches('s'),
    };
    matches!(name, "shiv" | "strike" | "defend" | "soul" | "sadness" | "joy" | "minion")
        .then(|| CardFilter::Named(name.into()))
}

/// Splits a sentence into its timing and the rest. `None` timing means the
/// sentence has no recognised timing phrase.
fn timing(ws: &[String]) -> (Option<When>, usize) {
    let s: Vec<&str> = ws.iter().map(String::as_str).collect();
    let starts = |p: &[&str]| s.len() >= p.len() && s[..p.len()] == *p;
    if starts(&["at", "the", "start", "of", "each", "combat"]) || starts(&["at", "the", "start", "of", "combat"]) {
        let n = if s[4] == "each" { 6 } else { 5 };
        return (Some(When::CombatStart), n);
    }
    // "Start each combat with 10 Block": keep "with", which reads as "gain".
    if starts(&["start", "each", "combat", "with"]) || starts(&["start", "combat", "with"]) {
        let n = if s[1] == "each" { 3 } else { 2 };
        return (Some(When::CombatStart), n);
    }
    if starts(&["at", "the", "start", "of", "your", "first"]) && s.get(6).and_then(|w| number(w)).is_some() {
        let n = number(s[6]).unwrap_or(1).max(1) as u32;
        return (Some(When::FirstTurns(n)), 8);
    }
    if starts(&["at", "the", "start", "of", "your"])
        && s.get(5).is_some_and(|w| *w != "turn")
        && let Some(n) = s.get(5).and_then(|w| ordinal(w))
    {
        return (Some(When::TurnNumber(n)), 7);
    }
    if starts(&["at", "the", "start", "of", "your", "turn"]) || starts(&["at", "the", "start", "of", "each", "turn"]) {
        return (Some(When::TurnStart), 6);
    }
    // Bread: "At the start of your first turn, lose 2 Energy. At the start of all other turns, gain 1 Energy."
    if starts(&["at", "the", "start", "of", "all", "other", "turns"]) {
        return (Some(When::TurnsFrom(2)), 7);
    }
    if starts(&["at", "the", "end", "of", "your", "turn"]) || starts(&["at", "the", "end", "of", "each", "turn"]) {
        return (Some(When::TurnEnd), 6);
    }
    if starts(&["every"])
        && s.get(2).is_some_and(|w| *w == "turns")
        && let Some(n) = number(s[1])
    {
        return (Some(When::EveryTurns(n.max(1) as u32)), 3);
    }
    // "Every time you play 3 Attacks in a single turn," / "Every time you play 10 Attacks,"
    if starts(&["every", "time", "you", "play"])
        && let (Some(n), Some(f)) = (s.get(4).and_then(|w| number(w)), filter(&ws[5..ws.len().min(6)]))
    {
        let per_turn = s.windows(3).any(|w| w == ["in", "a", "single"]);
        let end = s.iter().position(|w| *w == "turn").map_or(6, |i| i + 1).max(6);
        let end = if per_turn { end } else { 6 };
        return (Some(When::EveryPlays { filter: f, n: n.max(1) as u32, per_turn }), end);
    }
    // "Every 4 cards you play,"
    if starts(&["every"])
        && s.get(3..5).is_some_and(|w| w == ["you", "play"])
        && let (Some(n), Some(f)) = (number(s[1]), filter(&ws[2..3]))
    {
        return (Some(When::EveryPlays { filter: f, n: n.max(1) as u32, per_turn: false }), 5);
    }
    if starts(&["whenever", "you", "play"]) {
        let end = s.iter().position(|w| w.ends_with(',')).unwrap_or(s.len());
        // Words were stripped of commas; find the effect verb instead.
        let verb = s
            .iter()
            .enumerate()
            .skip(4)
            .find(|(_, w)| {
                matches!(
                    **w,
                    "gain" | "deal" | "draw" | "channel" | "apply" | "summon" | "forge" | "add" | "lose" | "the" | "a"
                )
            })
            .map_or(end, |(i, _)| i);
        if let Some(f) = filter(&ws[3..verb]) {
            return (Some(When::OnPlay(f)), verb);
        }
    }
    if starts(&["whenever", "a", "card", "is", "exhausted"]) {
        return (Some(When::OnExhaust), 5);
    }
    if starts(&["whenever", "you", "exhaust", "a", "card"]) {
        return (Some(When::OnExhaust), 5);
    }
    if starts(&["whenever", "you", "shuffle", "your", "draw", "pile"]) {
        return (Some(When::OnShuffle), 6);
    }
    if starts(&["whenever", "you", "gain", "block"]) && s.get(4).is_some_and(|w| *w != "on") {
        return (Some(When::OnGainBlock), 4);
    }
    if starts(&["whenever", "you", "draw", "a", "card"]) {
        let n = if s.get(5..7).is_some_and(|w| w == ["during", "your"]) { 8 } else { 5 };
        return (Some(When::OnDraw), n);
    }
    let upto = |end: &str| s.iter().position(|w| *w == end);
    if starts(&["whenever", "you", "draw"])
        && let Some(f) = filter(&ws[3..ws.len().min(6)])
    {
        return (Some(When::OnDrawOf(f)), upto("card").map_or(5, |i| i + 1));
    }
    if starts(&["whenever", "you", "apply"]) {
        let d = match s.get(3).copied().unwrap_or("") {
            "vulnerable" => Some(Debuff::Vulnerable),
            "weak" => Some(Debuff::Weak),
            "poison" => Some(Debuff::Poison),
            "doom" => Some(Debuff::Doom),
            "a" if s.get(4) == Some(&"debuff") => Some(Debuff::Any),
            _ => None,
        };
        if let Some(d) = d {
            let end = if d == Debuff::Any { upto("enemy").map_or(5, |i| i + 1) } else { 4 };
            return (Some(When::OnApply(d)), end);
        }
    }
    if starts(&["whenever", "you", "create"])
        && let Some(f) = filter(&ws[3..ws.len().min(5)])
    {
        return (Some(When::OnMake(f)), 5);
    }
    if starts(&["whenever", "you", "spend", "or", "gain"]) {
        return (Some(When::OnStars { gain: true, spend: true, each: false }), 7);
    }
    if starts(&["whenever", "you", "spend"]) && s.contains(&"star") {
        let each = s.windows(2).any(|w| w == ["for", "each"]);
        return (Some(When::OnStars { gain: false, spend: true, each }), upto("star").map_or(4, |i| i + 1));
    }
    if starts(&["whenever", "you", "evoke"]) {
        let kind = s.get(3).map(|w| w.to_string());
        return (Some(When::OnEvoke(kind.map(|k| OrbId::new(format!("{}_ORB", k.to_uppercase()))))), 4);
    }
    if starts(&["whenever", "you", "lose", "hp"]) {
        let end = if s.get(4..6).is_some_and(|w| w == ["on", "your"] || w == ["in", "combat"]) {
            if s[4] == "on" { 7 } else { 6 }
        } else {
            4
        };
        return (Some(When::OnHpLoss), end);
    }
    if starts(&["whenever", "you", "dislodge"]) {
        return (Some(When::OnDislodge), 5);
    }
    if starts(&["whenever", "you", "impale"]) {
        return (Some(When::OnShoot), 5);
    }
    if starts(&["whenever", "osty", "attacks"]) {
        return (Some(When::OnOstyAttack), 3);
    }
    if starts(&["whenever", "you", "discard", "a", "card"]) {
        let end = if s.get(5..8).is_some_and(|w| w == ["during", "your", "turn"]) { 8 } else { 5 };
        return (Some(When::OnDiscard), end);
    }
    if starts(&["whenever", "an", "attack", "deals"]) || starts(&["whenever", "attacks", "deal"]) {
        return (Some(When::OnAttackHit), upto("damage").map_or(5, |i| i + 1));
    }
    // "Every 5 times you Exhaust a card", "Every 4 Energy you spend",
    // "Every 10 cards you draw", "For every 10 Stars spent".
    let lead = if starts(&["for", "every"]) { 1 } else { 0 };
    if s.get(lead) == Some(&"every")
        && let Some(n) = s.get(lead + 1).and_then(|w| number(w))
    {
        let rest = &s[lead + 2..];
        let what = if rest.starts_with(&["times", "you", "exhaust"]) {
            Some((Counted::Exhausts, lead + 7))
        } else if rest.starts_with(&["energy", "you", "spend"]) {
            Some((Counted::EnergySpent, lead + 5))
        } else if rest.starts_with(&["star", "spent"]) || rest.starts_with(&["stars", "spent"]) {
            Some((Counted::StarsSpent, lead + 4))
        } else if rest.starts_with(&["cards", "you", "draw"]) {
            Some((Counted::CardsDrawn, lead + 5))
        } else {
            None
        };
        if let Some((what, end)) = what {
            return (Some(When::Every { what, n: n.max(1) as u32 }), end);
        }
    }
    if starts(&["whenever", "you", "gain", "a"]) && s.contains(&"melody") {
        let kind = s.get(4).and_then(|w| Melody::named(w));
        return (Some(When::OnMelody(kind)), if kind.is_some() { 6 } else { 5 });
    }
    if starts(&["when", "you", "gain", "every", "melody"]) {
        return (Some(When::OnAllMelodies), 5);
    }
    // Parry: "Sovereign Blade now gains 10 Block".
    if starts(&["sovereign", "blade", "now", "gains"]) {
        return (Some(When::OnPlay(CardFilter::Named("sovereign blade".into()))), 3);
    }
    (None, 0)
}

/// Timing at the end of a sentence: "Gain 1 Energy at the start of each turn",
/// "Gain an additional 1 Energy at the start of your 3rd turn, and every turn after that".
fn trailing_timing(lower: &str) -> Option<When> {
    if lower.contains("every turn after that") {
        let ws = words(lower);
        let n = ws.iter().position(|w| w == "your").and_then(|i| ws.get(i + 1)).and_then(|w| ordinal(w))?;
        return Some(When::TurnsFrom(n));
    }
    (lower.contains("at the start of each turn") || lower.contains("at the start of your turn"))
        .then_some(When::TurnStart)
}

/// Effects in the words after the timing phrase.
fn effects(ws: &[String], orbs: &[Orb]) -> Vec<Effect> {
    let s: Vec<&str> = ws.iter().map(String::as_str).collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < s.len() {
        // Effects on someone else ("ALL enemies gain 1 Strength", "allies gain") aren't ours.
        let theirs = i > 0 && matches!(s[i - 1], "enemies" | "enemy" | "allies" | "players" | "player" | "they");
        match s[i] {
            "gain" | "gains" | "with" | "lose"
                if !theirs && !(s[i] == "lose" && matches!(s.get(i + 2), Some(&"hp") | Some(&"damage"))) =>
            {
                let sign = if s[i] == "lose" { -1 } else { 1 };
                let mut j = i + 1;
                if matches!(s.get(j), Some(&"an") | Some(&"additional")) {
                    j += 1;
                }
                if s.get(j) == Some(&"additional") {
                    j += 1;
                }
                if let Some(n) = s.get(j).and_then(|w| number(w)) {
                    let n = n * sign;
                    // "gain 1 additional Arrow"
                    if s.get(j + 1) == Some(&"additional") && s.get(j + 2).is_some_and(|w| w.starts_with("arrow")) {
                        out.push(Effect::Arrows(n));
                        i = j + 3;
                        continue;
                    }
                    let kind = s.get(j + 1).copied().unwrap_or("");
                    let e = match kind {
                        "block" => Some(Effect::Block(n)),
                        "energy" => Some(Effect::Energy(n)),
                        "strength" => Some(Effect::Strength(n)),
                        "dexterity" => Some(Effect::Dexterity(n)),
                        "focus" => Some(Effect::Focus(n)),
                        "vigor" => Some(Effect::Vigor(n)),
                        "aim" => Some(Effect::Aim(n)),
                        "stretch" => Some(Effect::Stretch(n)),
                        "plating" => Some(Effect::Plating(n)),
                        "star" | "stars" => Some(Effect::Stars(n)),
                        "orb" if s.get(j + 2).is_some_and(|w| w.starts_with("slot")) => Some(Effect::OrbSlots(n)),
                        "additional" if s.get(j + 2) == Some(&"orb") => Some(Effect::OrbSlots(n)),
                        _ => None,
                    };
                    if let Some(e) = e {
                        out.push(e);
                    }
                    i = j + 2;
                    // "gain 1 Strength and 1 Dexterity": the verb carries over.
                    if s.get(i) == Some(&"and") && s.get(i + 1).and_then(|w| number(w)).is_some() {
                        i += 1;
                        let verb = if sign < 0 { "lose" } else { "gain" };
                        let rest: Vec<String> = std::iter::once(verb.to_string())
                            .chain(s[i..s.len().min(i + 3)].iter().map(|w| w.to_string()))
                            .collect();
                        out.extend(effects(&rest, orbs));
                        i += 2;
                    }
                    continue;
                }
            }
            "lose" | "take" if !theirs && s.get(i + 2).is_some_and(|w| matches!(*w, "hp" | "damage")) => {
                if let Some(n) = s.get(i + 1).and_then(|w| number(w)) {
                    out.push(Effect::LoseHp(n));
                    i += 3;
                    continue;
                }
            }
            // "they take 9 damage": the enemy's.
            "take" if theirs && s.get(i + 2) == Some(&"damage") => {
                if let Some(n) = s.get(i + 1).and_then(|w| number(w)) {
                    out.push(Effect::Damage(n));
                    i += 3;
                    continue;
                }
            }
            // "a random enemy loses 7 HP": HP loss, through Block.
            "loses" if theirs && s.get(i + 2) == Some(&"hp") => {
                if let Some(n) = s.get(i + 1).and_then(|w| number(w)) {
                    out.push(Effect::Damage(n));
                    i += 3;
                    continue;
                }
            }
            "put" if !theirs => {
                // "Put a card from your Discard Pile into your Hand",
                // "Put an Attack from your Draw Pile into your Hand",
                // "Put a card from your Discard Pile on top of your Draw Pile".
                if let (Some((n, used)), Some(from)) =
                    (amount(&s[i + 1..]), s.iter().skip(i).position(|w| *w == "from").map(|k| k + i))
                {
                    let filter = kind(&s[i + 1 + used..from]);
                    let source = if s.get(from + 2) == Some(&"draw") { Pile::Draw } else { Pile::Discard };
                    let to = if s[from..].windows(2).any(|w| w == ["on", "top"]) { Pile::Draw } else { Pile::Hand };
                    out.push(Effect::Fetch { from: source, to, n, filter });
                    i = from + 4;
                    continue;
                }
            }
            "choose" if !theirs => {
                // "Choose 1 of 3 cards in your Draw Pile to add into your Hand",
                // "Choose 1 of 3 random Colorless cards to add into your Hand".
                let end = s.iter().skip(i).position(|w| *w == "add").map_or(s.len(), |k| k + i);
                let phrase = &s[i..end];
                if phrase.contains(&"random") {
                    let at = phrase.iter().position(|w| *w == "random").unwrap_or(0) + 1;
                    out.push(Effect::MakeRandom { pool: pool(&phrase[at..]), n: 1, pile: Pile::Hand, free: false });
                } else {
                    out.push(Effect::Fetch { from: Pile::Draw, to: Pile::Hand, n: 1, filter: CardFilter::Any });
                }
                i = end + 1;
                continue;
            }
            "play" if !theirs => {
                let rest = &s[i + 1..];
                let has = |p: &[&str]| rest.windows(p.len()).any(|w| w == p);
                let exhaust = has(&["exhaust", "it"]);
                let found = if rest.starts_with(&["the", "top"]) {
                    let n = match rest.get(2) {
                        Some(&"card") => 1,
                        Some(&"x") => -1,
                        Some(w) => number(w).unwrap_or(1),
                        None => 1,
                    };
                    Some(Effect::AutoPlay { from: Source::DrawTop, n, filter: CardFilter::Any, exhaust })
                } else if rest.starts_with(&["the", "least", "expensive"]) {
                    Some(Effect::AutoPlay { from: Source::HandCheapest, n: 1, filter: CardFilter::Any, exhaust })
                } else if let Some((n, used)) = amount(rest) {
                    let pile = rest.iter().rposition(|w| *w == "pile" || *w == "hand").unwrap_or(rest.len());
                    let words = &rest[used..pile.min(rest.len())];
                    let from = if has(&["exhaust", "pile"]) {
                        Source::Exhaust
                    } else if has(&["discard", "pile"]) {
                        Source::Discard
                    } else if has(&["draw", "pile"]) {
                        Source::Draw
                    } else {
                        Source::Hand
                    };
                    Some(Effect::AutoPlay { from, n, filter: kind(words), exhaust })
                } else {
                    None
                };
                if let Some(e) = found {
                    out.push(e);
                    i = s.len();
                    continue;
                }
            }
            "discard" if !theirs => {
                let n = match s.get(i + 1) {
                    Some(&"a") => Some(1),
                    Some(w) => number(w),
                    None => None,
                };
                if let Some(n) = n.filter(|_| s.get(i + 2).is_some_and(|w| w.starts_with("card"))) {
                    out.push(Effect::Discard(n));
                    i += 3;
                    continue;
                }
            }
            "exhaust" if !theirs => {
                // "Exhaust 1 card", "Exhaust a card", "Exhaust 1 card at random",
                // "Exhaust up to 3 cards in your Hand", "Exhaust 1 card from your Draw Pile".
                let j = if s.get(i + 1..i + 3) == Some(&["up", "to"]) { i + 3 } else { i + 1 };
                let n = match s.get(j) {
                    Some(&"a") => Some(1),
                    Some(w) => number(w),
                    None => None,
                };
                if let Some(n) = n.filter(|_| s.get(j + 1).is_some_and(|w| w.starts_with("card"))) {
                    let rest = &s[j + 2..];
                    let random = rest.starts_with(&["at", "random"]);
                    let from = if rest.windows(2).any(|w| w == ["draw", "pile"]) { Pile::Draw } else { Pile::Hand };
                    out.push(Effect::Exhaust { n, random, from });
                    i = j + 2;
                    continue;
                }
            }
            // Stampede: "1 random Attack in your Hand is played against a random enemy".
            w if number(w).is_some()
                && s.get(i + 1) == Some(&"random")
                && s[i..].windows(2).any(|w| w == ["is", "played"]) =>
            {
                let end = s.iter().skip(i).position(|w| *w == "in").map_or(i + 3, |k| k + i);
                out.push(Effect::AutoPlay {
                    from: Source::Hand,
                    n: number(w).unwrap_or(1),
                    filter: kind(&s[i + 2..end]),
                    exhaust: false,
                });
                i = s.len();
                continue;
            }
            "osty" if s.get(i + 1) == Some(&"dies") => {
                out.push(Effect::OstyDies);
                i += 2;
                continue;
            }
            "evoke" => {
                let pick = match (s.get(i + 1), s.get(i + 2)) {
                    (Some(&"your"), Some(&"rightmost")) => Some((OrbPick::Rightmost, i + 4)),
                    (Some(&"your"), Some(&"leftmost")) => Some((OrbPick::Leftmost, i + 4)),
                    (Some(&"all"), _) => Some((OrbPick::All, i + 5)),
                    _ => None,
                };
                if let Some((pick, j)) = pick {
                    let (times, used) = repeat_count(&s[j.min(s.len())..]);
                    out.push(Effect::Evoke(pick, times));
                    i = j + used;
                    continue;
                }
            }
            "trigger" => {
                // "trigger the passive ability of your rightmost Orb (2 times)",
                // "… of all Dark Orbs (twice)", "trigger all Lightning against the enemy".
                let of = s.iter().skip(i).position(|w| *w == "of").map(|k| k + i);
                let pick = match of.and_then(|k| s.get(k + 1..k + 3)) {
                    Some([_, "rightmost"]) => Some((OrbPick::Rightmost, of.unwrap_or(i) + 4)),
                    Some(["all", "orbs"]) => Some((OrbPick::All, of.unwrap_or(i) + 3)),
                    Some(["all", kind]) => orb_named(orbs, kind).map(|o| (OrbPick::Kind(o), of.unwrap_or(i) + 4)),
                    _ => match s.get(i + 1..i + 3) {
                        Some(["all", kind]) => orb_named(orbs, kind)
                            .map(|o| (OrbPick::Kind(o), i + 3 + if s.get(i + 3) == Some(&"against") { 3 } else { 0 })),
                        _ => None,
                    },
                };
                if let Some((pick, j)) = pick {
                    let (times, used) = repeat_count(&s[j.min(s.len())..]);
                    out.push(Effect::OrbPassive(pick, times));
                    i = j + used;
                    continue;
                }
            }
            "deal" if !theirs => {
                if let Some(n) = s.get(i + 1).and_then(|w| number(w))
                    && s.get(i + 2) == Some(&"damage")
                {
                    let all = s[i + 3..s.len().min(i + 6)].windows(2).any(|w| w == ["all", "enemies"]);
                    out.push(if all { Effect::DamageAll(n) } else { Effect::Damage(n) });
                    i += 3;
                    continue;
                }
            }
            "draw" if !theirs => {
                let mut j = i + 1;
                if let Some(n) = s.get(j).and_then(|w| number(w)) {
                    j += 1;
                    if s.get(j) == Some(&"additional") {
                        j += 1;
                    }
                    if s.get(j).is_some_and(|w| w.starts_with("card")) {
                        out.push(Effect::Draw(n));
                        i = j + 1;
                        continue;
                    }
                }
            }
            "apply" if !theirs => {
                if let Some(n) = s.get(i + 1).and_then(|w| number(w)) {
                    let e = match s.get(i + 2).copied().unwrap_or("") {
                        "poison" => Some(Effect::Poison(n)),
                        "doom" => Some(Effect::Doom(n)),
                        "vulnerable" => Some(Effect::Vulnerable(n)),
                        "weak" => Some(Effect::Weak(n)),
                        _ => None,
                    };
                    // "apply 3 Doom to yourself" is not an attack on the enemy.
                    let to_self = s.get(i + 3..i + 5).is_some_and(|w| w == ["to", "yourself"]);
                    if let Some(e) = e.filter(|_| !to_self) {
                        out.push(e);
                    }
                    i += 3;
                    continue;
                }
            }
            "add" | "adds"
                if !theirs && s[i..].iter().take_while(|w| !matches!(**w, "into" | "to")).any(|w| *w == "random") =>
            {
                // "Add a random Attack into your Hand", "Add 3 random Colorless cards into your Hand".
                let to = s.iter().skip(i).position(|w| matches!(*w, "into" | "to")).map_or(s.len(), |k| k + i);
                let (n, _) = amount(&s[i + 1..]).unwrap_or((1, 1));
                let at = s[i..to].iter().position(|w| *w == "random").map_or(i + 1, |k| k + i + 1);
                let pile = if s[to..].windows(2).any(|w| w == ["draw", "pile"]) { Pile::Draw } else { Pile::Hand };
                out.push(Effect::MakeRandom { pool: pool(&s[at..to]), n, pile, free: false });
                i = to + 1;
                continue;
            }
            "add" | "adds" | "shuffle" if !theirs || s[i - 1] == "players" => {
                if let Some((made, used)) = make(&s[i + 1..]) {
                    out.extend(made);
                    i += 1 + used;
                    continue;
                }
            }
            "forge" => {
                if let Some(n) = s.get(i + 1).and_then(|w| number(w)) {
                    out.push(Effect::Forge(n));
                    i += 2;
                    continue;
                }
            }
            "summon" => {
                if let Some(n) = s.get(i + 1).and_then(|w| number(w)) {
                    out.push(Effect::Summon(n));
                    i += 2;
                    continue;
                }
            }
            "channel" => {
                if let (Some(n), Some(name)) = (s.get(i + 1).and_then(|w| number(w)), s.get(i + 2))
                    && let Some(orb) = orbs.iter().find(|o| o.name.eq_ignore_ascii_case(name))
                {
                    out.push(Effect::Channel(orb.id.clone(), n));
                    i += 3;
                    continue;
                }
            }
            _ => {}
        }
        i += 1;
    }
    out
}

/// How many: "a" → 1, "all"/"every" → -1 (all), "x" → -1, "up to N" → N.
fn amount(s: &[&str]) -> Option<(i32, usize)> {
    match s {
        ["a" | "an", ..] => Some((1, 1)),
        ["all" | "every" | "x", ..] => Some((-1, 1)),
        ["up", "to", n, ..] => number(n).map(|n| (n, 3)),
        [n, ..] => number(n).map(|n| (n, 1)),
        _ => None,
    }
}

/// The kind of card a phrase names: "card", "Attack", "card that can
/// Shoot", "0 Energy cards", "Ethereal cards", "Shiv", "“Wah” and “Ki” cards".
fn kind(s: &[&str]) -> CardFilter {
    let has = |p: &[&str]| s.windows(p.len()).any(|w| w == p);
    let melody: Vec<String> = s.iter().filter(|w| matches!(**w, "wah" | "cho" | "ki")).map(|w| w.to_string()).collect();
    if !melody.is_empty() {
        return CardFilter::AnyNamed(melody);
    }
    if has(&["can", "shoot"]) {
        return CardFilter::Shooter;
    }
    if has(&["0", "energy"]) {
        return CardFilter::ZeroCost;
    }
    if s.contains(&"ethereal") {
        return CardFilter::Keyword("ethereal".into());
    }
    for w in s {
        if let Some(t) = card_type(w) {
            return CardFilter::Type(t);
        }
        if matches!(w.trim_end_matches('s'), "shiv" | "soul") {
            return CardFilter::Named(w.trim_end_matches('s').into());
        }
    }
    CardFilter::Any
}

/// The random pool a phrase names: "Attack", "Colorless card", "Tarot card"…
fn pool(s: &[&str]) -> Pool {
    if s.contains(&"colorless") {
        return Pool::Colorless;
    }
    if s.contains(&"tarot") {
        return Pool::Tarot;
    }
    match kind(s) {
        CardFilter::Type(t) => Pool::Class(Some(t)),
        CardFilter::Any => Pool::Class(None),
        f => Pool::ClassWith(f),
    }
}

/// "twice" → 2, "4 times" → 4, "X times" → -1, else 1; and the words used.
fn repeat_count(s: &[&str]) -> (i32, usize) {
    match s {
        ["twice", ..] => (2, 1),
        ["x", "times", ..] | ["x+1", "times", ..] => (-1, 2),
        [n, "times", ..] if number(n).is_some() => (number(n).unwrap_or(1), 2),
        _ => (1, 0),
    }
}

fn orb_named(orbs: &[Orb], name: &str) -> Option<OrbId> {
    orbs.iter().find(|o| o.name.eq_ignore_ascii_case(name.trim_end_matches('s'))).map(|o| o.id.clone())
}

/// "3 Shivs into your Hand", "a Soul into your Draw Pile, Hand, and Discard
/// Pile", "2 Shivs to ALL players' Hands": the cards made and where they go,
/// and how many words that took. Random cards and copies aren't read here.
fn make(s: &[&str]) -> Option<(Vec<Effect>, usize)> {
    let n = match *s.first()? {
        "a" | "an" => 1,
        "x" => -1,
        w => number(w)?,
    };
    let to = s.iter().position(|w| matches!(*w, "into" | "to"))?;
    let name: Vec<&str> = s[1..to].to_vec();
    if name.is_empty() || name.iter().any(|w| matches!(*w, "random" | "copy" | "copies" | "of" | "card" | "cards")) {
        return None;
    }
    let mut piles = Vec::new();
    let mut j = to + 1;
    while j < s.len() {
        match s[j] {
            "hand" | "hands" => piles.push(Pile::Hand),
            "draw" if s.get(j + 1) == Some(&"pile") => piles.push(Pile::Draw),
            "discard" if s.get(j + 1) == Some(&"pile") => piles.push(Pile::Discard),
            "your" | "all" | "players'" | "their" | "pile" | "and" | "everyone's" => {}
            _ => break,
        }
        j += 1;
    }
    if piles.is_empty() {
        return None;
    }
    // "a Joy and a Sadness": two cards, one each.
    let names: Vec<(String, i32)> = match name.iter().position(|w| *w == "and") {
        Some(k) if matches!(name.get(k + 1), Some(&"a") | Some(&"an")) && k + 2 < name.len() => {
            vec![(name[..k].join(" "), n), (name[k + 2..].join(" "), 1)]
        }
        _ => vec![(name.join(" "), n)],
    };
    let made = piles
        .into_iter()
        .flat_map(|pile| names.iter().map(move |(name, n)| Effect::Make { name: name.clone(), n: *n, pile }))
        .collect();
    Some((made, j))
}

/// Something a card's number counts ("for each Skill in your Hand").
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Count {
    /// Deck cards whose name contains any of these (lowercase).
    DeckNamed(Vec<String>),
    /// Deck cards that cost Stars.
    DeckStarCost,
    /// Other Osty Attacks in the deck.
    DeckOtherOsty,
    EnemyVulnerable,
    EnemyDoom,
    EnemyPoison,
    /// Distinct debuffs on the enemy.
    EnemyDebuffs,
    AttacksThisTurn,
    SkillsThisTurn,
    SkillsInHand,
    OtherCardsInHand,
    Orbs,
    UniqueOrbs,
    DrawnThisCombat,
    DrawnThisTurn,
    EtherealPlayed,
    StarsGainedThisTurn,
    /// Cards created this combat (of this name, or any).
    Made(Option<String>),
    EnergySpentThisTurn,
    PlayedThisCombat,
    DrawPile,
    DiscardPile,
    Block,
    /// Times you lost HP this combat.
    HpLosses,
    OstyAttacksThisTurn,
    /// Melodies you hold.
    Melodies,
    /// Osty's current and Max HP (the same: nothing hits him here).
    OstyHp,
}

/// What a count changes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scaled {
    /// Damage is `per` × count ("Deal 5 damage for each Skill in your Hand").
    DamagePer,
    /// Damage gains `per` × count ("Deals 2 additional damage for each…").
    DamageAdd,
    /// Hits gain the count.
    Hits,
    /// Block is the count.
    Block,
    /// Draws `per` × count.
    Draw,
}

/// A number that scales with something in the fight or the deck.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Scale {
    pub count: Count,
    pub per: i32,
    pub into: Scaled,
}

/// A condition a card checks ("If the enemy is Impaled").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cond {
    EnemyImpaled,
    EnemyVulnerable,
    EnemyPoisoned,
    LostHpThisTurn,
    HaveArrow,
    DrawPileEmpty,
    HandAllAttacks,
    NoAttacksInHand,
    HandEmpty,
    AppliedDoomThisTurn,
    /// The card's own hits bring an Arrow back.
    Dislodges,
    /// A kill: the simulator's target doesn't die, so never.
    Kill,
    OstyAttackedThisTurn,
    OstyAlive,
    /// Holding any of these Melodies.
    HaveMelody(bool, bool, bool),
}

/// What a condition gates when it isn't the whole card.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Bonus {
    Energy,
    Draw,
    /// The card's extra hits.
    Hits,
    /// Block gained this many more times.
    BlockTimes(i32),
    DoubleDamage,
    RetainHand,
}

/// Rules a card sets on costs, growth and the turn.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CardMod {
    /// Costs N less per count ("Costs 1 less Energy for each Attack played this turn").
    CostPer(Count, i32),
    /// The next matching card you play costs 0.
    NextFree(CardFilter),
    /// Matching cards in your hand cost N less until played.
    HandCheaper(CardFilter, i32),
    /// Every card in your hand is free this turn.
    HandFree,
    /// Every card in your hand costs at most N this turn.
    HandCost(i32),
    /// Every card costs N more this turn.
    CostUp(i32),
    /// Its own cost changes by N after each play ("Reduce this card's cost by 1").
    SelfCost(i32),
    /// Its own cost drops to 0 after it's played.
    SelfCostZero,
    /// Its own cost changes by N each time it's drawn.
    CostOnDraw(i32),
    /// Its damage grows by N each play, or each draw, this combat.
    Grow(i32),
    GrowOnDraw(i32),
    /// Every card named so grows by N each play ("ALL Claw cards").
    GrowNamed(String, i32),
    /// Costs 0 while a condition holds.
    FreeIf(Cond),
    /// Costs N less while a condition holds.
    CheaperIf(Cond, i32),
    RetainHand,
    /// No more draws this turn.
    DrawLock,
    /// Only N more cards this turn.
    PlayLimit(i32),
    DoubleEnergy,
    /// Doubles the Block you have.
    DoubleBlock,
    /// Doubles Block gains this turn.
    DoubleBlockGains,
    /// No Block from cards for N turns.
    BlockLock(i32),
    /// Next turn, Attacks deal double damage.
    AttacksDoubleNext,
    /// Every card named so deals double damage from now on ("ALL Hang cards").
    NamedDouble(String),
    /// The named card deals double damage this turn.
    NamedDoubleTurn(String),
    /// Ends your turn.
    EndTurn,
}

/// What a card's text does beyond its numbers, for the simulator.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CardText {
    /// Cards it makes ("Add 3 Shivs into your Hand"); `n` of -1 means X.
    pub makes: Vec<Effect>,
    /// "Add a copy of this card into your Discard Pile."
    pub copy_to: Option<Pile>,
    /// Whether it really draws (the data's card count is also set for
    /// "Add 3 Shivs" and "3 or more cards").
    pub draws: bool,
    /// "Exhaust your Hand" and the like: which cards go, and what it does
    /// for each (its own numbers when `own_per_card`, else `per_card`).
    pub exhausts: Option<ExhaustWhich>,
    pub own_per_card: bool,
    pub per_card: Vec<Effect>,
    /// "Discard your Hand. Add 1 Shiv … for each card discarded."
    pub discards_hand: bool,
    /// "N additional damage for each card in your Exhaust Pile" (a name
    /// counts only those cards: "for each Soul").
    pub per_exhausted: Option<(i32, Option<String>)>,
    /// Works only with at least this many cards in the Exhaust Pile.
    pub exhausted_at_least: Option<i32>,
    /// Works only if a card was exhausted this turn ("If you Exhausted a
    /// card this turn, gain 3 Energy"), or gives this much more Block then.
    pub needs_exhaust_this_turn: bool,
    pub block_if_exhausted: i32,
    /// "When this card is Exhausted, gain 2 Energy."
    pub when_exhausted: Vec<Effect>,
    /// "If this is in your Exhaust Pile, play it" at the start or end of
    /// your turn.
    pub replays_from_exhaust: Option<When>,
    /// "This turn, your next Skill is played an extra time."
    pub next_extra: Option<CardType>,
    /// "Costs 1 less Energy for each card Exhausted this combat."
    pub cheaper_per_exhausted: bool,
    /// "<card> gains Replay N": the card's name and N.
    pub gives_replay: Option<(String, i32)>,
    /// Effects that come later: "Next turn, draw 2 cards" is `(1, [Draw(2)])`,
    /// "Gain 5 Block at the start of the next 2 turns" is `(2, [Block(5)])`:
    /// at the start of each of the next N turns.
    pub later: Vec<(u32, Vec<Effect>)>,
    /// A sentence gains energy now ("Gain 2 Energy"), not a cost or a
    /// threshold ("costs 2 Energy or more") or a kill reward.
    pub gains_energy: bool,
    /// A sentence gains Stars now (not next turn).
    pub gains_stars: bool,
    /// Your own Strength, Dexterity and Focus ("Gain 3 Strength", "Lose 3
    /// Focus this turn"), each with whether it lasts only this turn.
    pub stats: Vec<(Effect, bool)>,
    /// Triggers that last this turn: "Whenever you play an Attack this
    /// turn, gain 3 Block."
    pub this_turn: Vec<(When, Vec<Effect>)>,
    /// Evokes, orb passive triggers, chosen discards and exhausts:
    /// "Evoke your rightmost Orb twice", "Discard 1 card", "Exhaust 1 card".
    pub actions: Vec<Effect>,
    /// "Discard your Hand, then draw that many cards."
    pub draw_per_discard: bool,
    /// HP you lose by playing it ("Lose 3 HP").
    pub lose_hp: i32,
    /// What it does if it's still in your hand at the end of your turn
    /// ("At the end of your turn, if this is in your Hand, take 2 damage").
    pub held_end: Vec<Effect>,
    /// Numbers that scale with the fight or the deck.
    pub scale: Vec<Scale>,
    /// "Can only be played if…"
    pub play_if: Option<Cond>,
    /// The whole effect needs this ("If the enemy is Impaled, Deal 20…").
    pub works_if: Option<Cond>,
    /// Parts that need a condition ("If the enemy is Impaled, gain 1 Energy").
    pub bonus_if: Vec<(Cond, Bonus)>,
    /// It brings back a stuck Arrow ("Dislodge and draw 2 cards").
    pub dislodges: bool,
    /// Rules it sets on costs, growth and the turn.
    pub mods: Vec<CardMod>,
    /// It goes back to your hand: right away, or at the start of next turn.
    pub returns: Option<Pile>,
}

/// A sentence's rule on costs, growth or the turn, if it has one.
fn card_mod(s: &[&str], ws: &[String]) -> Option<CardMod> {
    let has = |p: &[&str]| s.windows(p.len()).any(|w| w == p);
    let starts = |p: &[&str]| s.len() >= p.len() && s[..p.len()] == *p;
    let after = |w: &str| s.iter().position(|x| *x == w).and_then(|i| s.get(i + 1)).and_then(|w| number(w));
    Some(if starts(&["costs"]) && has(&["for", "each"]) && (has(&["less", "energy"]) || has(&["energy", "less"])) {
        let n = s.get(1).and_then(|w| number(w))?;
        let from = s.windows(2).position(|w| w == ["for", "each"])?;
        CardMod::CostPer(count(&s[from..])?, n)
    } else if starts(&["the", "next"]) && has(&["costs", "0"]) {
        let you = s.iter().position(|w| *w == "you")?;
        CardMod::NextFree(filter(&ws[2..you])?)
    } else if has(&["in", "your", "hand", "cost"]) && has(&["less"]) && has(&["until", "played"]) {
        CardMod::HandCheaper(filter(&ws[..1])?, after("cost")?)
    } else if has(&["all", "cards", "in", "your", "hand", "are", "free"]) {
        CardMod::HandFree
    } else if starts(&["reduce", "the", "cost", "of", "all", "cards", "in", "your", "hand", "to"]) {
        CardMod::HandCost(after("to")?)
    } else if starts(&["cards", "cost", "an", "additional"]) {
        CardMod::CostUp(after("additional")?)
    } else if starts(&["reduce", "this", "card's", "cost", "to", "0"]) {
        CardMod::SelfCostZero
    } else if starts(&["reduce", "this", "card's", "cost", "by"]) && !has(&["whenever"]) {
        CardMod::SelfCost(-after("by")?)
    } else if starts(&["increase", "this", "card's", "cost", "by"])
        || has(&["increase", "this", "card's", "cost", "by"])
    {
        CardMod::SelfCost(after("by")?)
    } else if starts(&["whenever", "you", "draw", "this", "card"]) && has(&["reduce", "its", "cost", "by"]) {
        CardMod::CostOnDraw(-after("by")?)
    } else if starts(&["whenever", "you", "draw", "this", "card"]) && has(&["increase", "its", "damage", "by"]) {
        CardMod::GrowOnDraw(after("by")?)
    } else if starts(&["increase", "this", "card's", "damage", "by"]) {
        CardMod::Grow(after("by")?)
    } else if starts(&["increase", "the", "damage", "of", "all"]) {
        CardMod::GrowNamed(s.get(5)?.to_string(), after("by")?)
    } else if starts(&["costs"]) && has(&["less", "energy", "if"]) {
        let at = s.iter().position(|w| *w == "if")?;
        CardMod::CheaperIf(condition(&s[at..])?, s.get(1).and_then(|w| number(w))?)
    } else if has(&["costs", "0", "energy", "if", "osty", "has", "attacked", "this", "turn"]) {
        CardMod::FreeIf(Cond::OstyAttackedThisTurn)
    } else if has(&["retain", "your", "hand", "this", "turn"]) {
        CardMod::RetainHand
    } else if has(&["cannot", "draw", "additional", "cards", "this", "turn"]) {
        CardMod::DrawLock
    } else if starts(&["you", "can", "only", "play"]) {
        CardMod::PlayLimit(after("play")?)
    } else if starts(&["double", "your", "energy"]) {
        CardMod::DoubleEnergy
    } else if starts(&["double", "your", "block", "gain", "this", "turn"]) {
        CardMod::DoubleBlockGains
    } else if starts(&["double", "your", "block"]) {
        CardMod::DoubleBlock
    } else if starts(&["you", "cannot", "gain", "block", "from", "cards", "for"]) {
        CardMod::BlockLock(after("for")?)
    } else if starts(&["next", "turn", "attacks", "deal", "double", "damage"]) {
        CardMod::AttacksDoubleNext
    } else if starts(&["double", "the", "damage", "all"]) {
        CardMod::NamedDouble(s.get(4)?.to_string())
    } else if has(&["deals", "double", "damage", "to", "the", "enemy", "this", "turn"]) {
        let at = s.iter().position(|w| *w == "deals")?;
        CardMod::NamedDoubleTurn(s[..at].join(" "))
    } else if starts(&["end", "your", "turn"]) {
        CardMod::EndTurn
    } else {
        return None;
    })
}

/// The condition after "if" ("the enemy is Impaled", "you lost HP this turn").
fn condition(s: &[&str]) -> Option<Cond> {
    let has = |p: &[&str]| s.windows(p.len()).any(|w| w == p);
    Some(if has(&["enemy", "is", "impaled"]) {
        Cond::EnemyImpaled
    } else if has(&["enemy", "is", "vulnerable"]) {
        Cond::EnemyVulnerable
    } else if has(&["enemy", "has", "poison"]) {
        Cond::EnemyPoisoned
    } else if has(&["lost", "hp", "this", "turn"]) {
        Cond::LostHpThisTurn
    } else if has(&["you", "have", "an", "arrow"]) {
        Cond::HaveArrow
    } else if has(&["no", "cards", "in", "your", "draw", "pile"]) {
        Cond::DrawPileEmpty
    } else if has(&["every", "card", "in", "your", "hand", "is", "an", "attack"]) {
        Cond::HandAllAttacks
    } else if has(&["no", "attacks", "in", "your", "hand"]) {
        Cond::NoAttacksInHand
    } else if has(&["your", "hand", "is", "empty"]) {
        Cond::HandEmpty
    } else if has(&["applied", "doom", "this", "turn"]) {
        Cond::AppliedDoomThisTurn
    } else if has(&["this", "dislodges"]) {
        Cond::Dislodges
    } else if has(&["osty", "is", "alive"]) {
        Cond::OstyAlive
    } else if has(&["you", "have", "a"]) && s.contains(&"melody") {
        let any = |m: &str| s.contains(&m);
        Cond::HaveMelody(any("wah"), any("cho"), any("ki"))
    } else if has(&["this", "kills"]) || s.contains(&"fatal") {
        Cond::Kill
    } else {
        return None;
    })
}

/// What "for each …", "for ALL your cards …" or "equal to …" counts.
fn count(s: &[&str]) -> Option<Count> {
    let has = |p: &[&str]| s.windows(p.len()).any(|w| w == p);
    Some(if let Some(i) = s.iter().position(|w| *w == "containing") {
        Count::DeckNamed(s[i + 1..].iter().filter(|w| !matches!(**w, "or" | "and")).map(|w| w.to_string()).collect())
    } else if has(&["star", "cost"]) {
        Count::DeckStarCost
    } else if has(&["other", "osty", "attacks"]) {
        Count::DeckOtherOsty
    } else if has(&["vulnerable", "on", "the", "enemy"]) {
        Count::EnemyVulnerable
    } else if has(&["enemy's", "doom"]) {
        Count::EnemyDoom
    } else if has(&["poison", "on", "all", "enemies"]) {
        Count::EnemyPoison
    } else if has(&["unique", "debuff"]) {
        Count::EnemyDebuffs
    } else if has(&["attack", "already", "played", "this", "turn"]) {
        Count::AttacksThisTurn
    } else if has(&["skill", "already", "played", "this", "turn"]) {
        Count::SkillsThisTurn
    } else if has(&["skill", "in", "your", "hand"]) {
        Count::SkillsInHand
    } else if has(&["other", "card", "in", "your", "hand"]) {
        Count::OtherCardsInHand
    } else if has(&["channeled", "orb"]) {
        Count::Orbs
    } else if has(&["unique", "orb"]) {
        Count::UniqueOrbs
    } else if has(&["drawn", "this", "combat"]) {
        Count::DrawnThisCombat
    } else if has(&["drawn", "during", "your", "turn"]) {
        Count::DrawnThisTurn
    } else if has(&["ethereal", "card", "played"]) {
        Count::EtherealPlayed
    } else if has(&["star", "gained", "this", "turn"]) {
        Count::StarsGainedThisTurn
    } else if has(&["sadness", "you", "have", "created"]) {
        Count::Made(Some("sadness".into()))
    } else if has(&["card", "you", "created"]) {
        Count::Made(None)
    } else if has(&["energy", "spent", "on", "other", "cards"]) {
        Count::EnergySpentThisTurn
    } else if has(&["cards", "played", "this", "combat"]) {
        Count::PlayedThisCombat
    } else if has(&["cards", "in", "your", "draw", "pile"]) {
        Count::DrawPile
    } else if has(&["cards", "in", "your", "discard", "pile"]) {
        Count::DiscardPile
    } else if has(&["your", "block"]) {
        Count::Block
    } else if has(&["lost", "hp", "this", "combat"]) {
        Count::HpLosses
    } else if has(&["he", "has", "attacked", "this", "turn"]) {
        Count::OstyAttacksThisTurn
    } else if has(&["melody", "you", "have"]) {
        Count::Melodies
    } else if has(&["osty's", "current", "hp"]) || has(&["osty's", "max", "hp"]) || has(&["his", "max", "hp"]) {
        Count::OstyHp
    } else if has(&["attack", "played", "this", "turn"]) {
        Count::AttacksThisTurn
    } else if has(&["skill", "played", "this", "turn"]) {
        Count::SkillsThisTurn
    } else {
        return None;
    })
}

/// A sentence's scaling, if it has one.
fn scale(s: &[&str]) -> Option<Scale> {
    let has = |p: &[&str]| s.windows(p.len()).any(|w| w == p);
    let at = |w: &str| s.iter().position(|x| *x == w);
    let from = s.windows(2).position(|w| w == ["for", "each"] || w == ["for", "all"] || w == ["equal", "to"])?;
    let c = count(&s[from..])?;
    let before = |w: &str| at(w).and_then(|i| i.checked_sub(1)).and_then(|i| number(s[i]));
    let (into, per) = if has(&["hits", "an", "additional", "time"]) {
        (Scaled::Hits, 1)
    } else if has(&["block", "equal", "to", "triple"]) {
        (Scaled::Block, 3)
    } else if has(&["gain", "block", "equal", "to"]) {
        (Scaled::Block, 1)
    } else if s.first() == Some(&"gain") && s.get(2) == Some(&"block") && has(&["for", "each"]) {
        (Scaled::Block, s.get(1).and_then(|w| number(w))?)
    } else if has(&["additional", "damage", "equal", "to"]) {
        (Scaled::DamageAdd, 1)
    } else if s.first() == Some(&"draw") {
        (Scaled::Draw, s.get(1).and_then(|w| number(w)).unwrap_or(1))
    } else if has(&["damage", "equal", "to"]) {
        (Scaled::DamagePer, 1)
    } else if has(&["additional", "damage"]) {
        (Scaled::DamageAdd, before("additional")?)
    } else if has(&["less", "damage"]) {
        (Scaled::DamageAdd, -before("less")?)
    } else if s.first() == Some(&"deal") {
        (Scaled::DamagePer, before("damage")?)
    } else {
        return None;
    };
    Some(Scale { count: c, per, into })
}

/// Which cards an "Exhaust …" sentence takes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExhaustWhich {
    /// The whole hand.
    Hand,
    /// Every non-Attack in the hand.
    HandNonAttacks,
    /// Every Status card, wherever it is.
    Statuses,
    /// Every card with this name in the hand.
    HandNamed(String),
}

/// Reads what a card's text does beyond its numbers (`vars`).
pub fn card_text(text: &str, orbs: &[Orb]) -> CardText {
    let mut c = CardText::default();
    for sentence in sentences(text) {
        let ws = words(&sentence);
        let s: Vec<&str> = ws.iter().map(String::as_str).collect();
        let has = |p: &[&str]| s.windows(p.len()).any(|w| w == p);
        let starts = |p: &[&str]| s.len() >= p.len() && s[..p.len()] == *p;
        if has(&["return", "this", "card", "to", "your", "hand"]) && !starts(&["whenever"]) {
            c.returns = Some(Pile::Hand);
        }
        if has(&["start", "of", "your", "next", "turn", "return", "this", "to", "your", "hand"]) {
            c.returns = Some(Pile::Draw);
            continue;
        }
        // "It's free to play this turn": the random card just made.
        if starts(&["it's", "free"]) || starts(&["they're", "free"]) {
            for e in c.actions.iter_mut().rev() {
                if let Effect::MakeRandom { free, .. } = e {
                    *free = true;
                    break;
                }
            }
            continue;
        }
        if let Some(m) = card_mod(&s, &ws) {
            c.mods.push(m);
            continue;
        }
        // "Next turn, draw 2 cards", "At the start of the next 2 turns,
        // Channel 1 Lightning", "Gain 5 Block at the start of the next 2 turns".
        let next_n = s.windows(4).find(|w| w[..3] == ["of", "the", "next"]).and_then(|w| number(w[3]));
        if starts(&["next", "turn"]) || has(&["start", "of", "your", "next", "turn"]) || next_n.is_some() {
            let found = effects(&ws, orbs);
            if !found.is_empty() {
                c.later.push((next_n.unwrap_or(1).max(1) as u32, found));
            }
            continue;
        }
        if starts(&["at", "the", "end", "of", "your", "turn", "if", "this", "is", "in", "your", "hand"]) {
            c.held_end = effects(&ws[12..], orbs);
            continue;
        }
        if starts(&["whenever"]) && has(&["this", "turn"]) {
            // The same trigger as a power's, for this turn only.
            let rest: Vec<String> = ws
                .iter()
                .enumerate()
                .filter(|(k, w)| {
                    !((*w == "this" && ws.get(k + 1).is_some_and(|n| n == "turn"))
                        || (*w == "turn" && *k > 0 && ws[k - 1] == "this"))
                })
                .map(|(_, w)| w.clone())
                .collect();
            if let (Some(when), at) = timing(&rest) {
                let found = effects(&rest[at.min(rest.len())..], orbs);
                if !found.is_empty() {
                    c.this_turn.push((when, found));
                }
            }
            continue;
        }
        // "Draw 2 cards", "draw a card"; not "into your Draw Pile".
        if s.windows(2).any(|w| w[0] == "draw" && (number(w[1]).is_some() || w[1] == "a")) {
            c.draws = true;
        }
        if let Some(sc) = scale(&s) {
            c.scale.push(sc);
        }
        if starts(&["dislodge"]) || has(&["dislodge", "and"]) {
            c.dislodges = true;
        }
        if has(&["can", "only", "be", "played", "if"]) {
            c.play_if = condition(&s);
        } else if has(&["deals", "double", "damage", "to", "impaled"]) {
            c.bonus_if.push((Cond::EnemyImpaled, Bonus::DoubleDamage));
        } else if starts(&["if"])
            && let Some(cond) = condition(&s)
        {
            let mut parts = Vec::new();
            if s.windows(3).any(|w| w[0] == "gain" && number(w[1]).is_some() && w[2] == "energy") {
                parts.push(Bonus::Energy);
            }
            if s.windows(2).any(|w| w[0] == "draw" && number(w[1]).is_some()) {
                parts.push(Bonus::Draw);
            }
            if has(&["hits", "twice"]) || s.windows(3).any(|w| w[0] == "hits" && w[2] == "times") {
                parts.push(Bonus::Hits);
            }
            if has(&["retain", "your", "hand"]) {
                parts.push(Bonus::RetainHand);
            }
            if has(&["he", "dies"]) {
                c.actions.push(Effect::OstyDies);
            }
            if has(&["gain", "block"]) && has(&["additional"]) {
                let n = s.iter().position(|w| *w == "additional").and_then(|i| s.get(i - 1)).and_then(|w| number(w));
                parts.push(Bonus::BlockTimes(n.unwrap_or(1)));
            }
            let whole = has(&["deal"])
                || has(&["deals"])
                || has(&["apply"])
                || has(&["dislodge"])
                || has(&["block", "equal", "to"]);
            if whole {
                c.works_if = Some(cond);
            } else {
                c.bonus_if.extend(parts.into_iter().map(|b| (cond, b)));
            }
        }
        let trigger = matches!(s.first(), Some(&"whenever") | Some(&"when") | Some(&"every"));
        if !trigger && !starts(&["if"]) {
            for e in effects(&ws, orbs) {
                match e {
                    Effect::Evoke(..)
                    | Effect::OrbPassive(..)
                    | Effect::Discard(_)
                    | Effect::Exhaust { .. }
                    | Effect::OstyDies
                    | Effect::Fetch { .. }
                    | Effect::AutoPlay { .. }
                    | Effect::MakeRandom { .. } => c.actions.push(e),
                    Effect::LoseHp(n) => c.lose_hp += n,
                    _ => {}
                }
            }
        }
        let gain_of = |what: &str| {
            s.windows(3).any(|w| w[0] == "gain" && number(w[1]).is_some() && w[2].trim_end_matches('s') == what)
                || s.windows(4).any(|w| w[..2] == ["gain", "an"] && number(w[3]).is_some() && w[2] == "additional")
                    && has(&[what])
        };
        if !trigger && !starts(&["if", "this", "kills"]) {
            c.gains_energy |= gain_of("energy");
            c.gains_stars |= gain_of("star");
        }
        if !trigger && !starts(&["if"]) && !has(&["for", "each"]) {
            let temp = has(&["this", "turn"]);
            for e in effects(&ws, orbs) {
                if matches!(e, Effect::Strength(_) | Effect::Dexterity(_) | Effect::Focus(_)) {
                    c.stats.push((e, temp));
                }
            }
        }
        if has(&["copy", "of", "this", "card"]) {
            c.copy_to = if has(&["discard", "pile"]) {
                Some(Pile::Discard)
            } else if s.contains(&"hand") {
                Some(Pile::Hand)
            } else {
                Some(Pile::Draw)
            };
            continue;
        }
        if starts(&["exhaust", "your", "hand"]) {
            c.exhausts = Some(ExhaustWhich::Hand);
        } else if starts(&["exhaust", "all", "non-attack", "cards", "in", "your", "hand"]) {
            c.exhausts = Some(ExhaustWhich::HandNonAttacks);
        } else if starts(&["exhaust", "all", "your", "status", "cards"]) {
            c.exhausts = Some(ExhaustWhich::Statuses);
        } else if starts(&["exhaust", "all"]) && has(&["in", "your", "hand"]) {
            let end = s.iter().position(|w| *w == "in").unwrap_or(2);
            c.exhausts = Some(ExhaustWhich::HandNamed(s[2..end].join(" ")));
        }
        if starts(&["discard", "your", "hand"]) {
            c.discards_hand = true;
            c.draw_per_discard = has(&["draw", "that", "many"]);
        }
        if has(&["for", "each", "card", "exhausted"]) && !has(&["this", "combat"]) {
            let at = s.iter().position(|w| *w == "for").unwrap_or(s.len());
            let per = effects(&ws[..at], orbs);
            // A card whose own numbers are per card ("Deal 7 damage for
            // each card Exhausted") uses them; others read the sentence.
            if per.iter().any(|e| matches!(e, Effect::Damage(_) | Effect::Block(_))) {
                c.own_per_card = true;
            } else {
                c.per_card = per;
            }
        }
        if has(&["for", "each", "card", "discarded"]) {
            c.makes.extend(effects(&ws, orbs).into_iter().filter_map(|e| match e {
                Effect::Make { name, pile, .. } => Some(Effect::Make { name, n: -2, pile }),
                _ => None,
            }));
            continue;
        }
        if has(&["additional", "damage", "for", "each"]) && has(&["exhaust", "pile"]) {
            let n = s.iter().position(|w| *w == "additional").and_then(|i| i.checked_sub(1)).and_then(|i| number(s[i]));
            let each = s.iter().position(|w| *w == "each").map_or(s.len(), |i| i + 1);
            let what = s.get(each).filter(|w| **w != "card").map(|w| w.trim_end_matches('s').to_string());
            c.per_exhausted = n.map(|n| (n, what));
            continue;
        }
        if starts(&["if", "you", "have"]) && has(&["or", "more", "cards", "in", "your", "exhaust", "pile"]) {
            c.exhausted_at_least = s.get(3).and_then(|w| number(w));
        }
        if has(&["exhausted", "a", "card", "this", "turn"]) {
            if has(&["another"]) {
                c.block_if_exhausted = s
                    .iter()
                    .position(|w| *w == "another")
                    .and_then(|i| s.get(i + 1))
                    .and_then(|w| number(w))
                    .unwrap_or(0);
            } else {
                c.needs_exhaust_this_turn = true;
            }
        }
        if starts(&["when", "this", "card", "is", "exhausted"]) {
            c.when_exhausted = effects(&ws[5..], orbs);
        }
        if has(&["if", "this", "is", "in", "your", "exhaust", "pile", "play", "it"]) {
            c.replays_from_exhaust =
                if has(&["start", "of", "your", "turn"]) { Some(When::TurnStart) } else { Some(When::TurnEnd) };
        }
        if has(&["is", "played", "an", "extra", "time"]) && starts(&["this", "turn", "your", "next"]) {
            c.next_extra = s.get(4).and_then(|w| card_type(w));
        }
        if starts(&["the", "next"]) && has(&["is", "played", "an", "additional", "time"]) {
            c.next_extra = s.get(2).and_then(|w| card_type(w));
        }
        if has(&["for", "each", "card", "exhausted", "this", "combat"]) && s.contains(&"less") {
            c.cheaper_per_exhausted = true;
        }
        if let Some(i) = s.iter().position(|w| *w == "gains")
            && s.get(i + 1) == Some(&"replay")
        {
            c.gives_replay = Some((s[..i].join(" "), s.get(i + 2).and_then(|w| number(w)).unwrap_or(1)));
        }
        if s.contains(&"add") || s.contains(&"adds") {
            c.makes.extend(effects(&ws, orbs).into_iter().filter(|e| matches!(e, Effect::Make { .. })));
        }
    }
    c
}

/// Sentence-level rules that modify other cards.
fn passive(sentence: &str) -> Option<Passive> {
    let ws = words(sentence);
    let s: Vec<&str> = ws.iter().map(String::as_str).collect();
    let has = |p: &[&str]| s.windows(p.len()).any(|w| w == p);
    let num_before =
        |word: &str| s.iter().position(|w| *w == word).and_then(|i| i.checked_sub(1)).and_then(|i| number(s[i]));
    let pct = s.iter().find_map(|w| w.strip_suffix('%').and_then(|n| n.parse().ok()));
    if has(&["vulnerable", "enemies", "take", "an", "additional"]) {
        return pct.map(Passive::VulnerableBonus);
    }
    if has(&["additional", "damage"]) && !has(&["first"]) {
        let n = num_before("additional")?;
        if has(&["containing", "strike"]) {
            return Some(Passive::ExtraDamage(CardFilter::Named("strike".into()), n));
        }
        if s.first() == Some(&"shivs") {
            return Some(Passive::ExtraDamage(CardFilter::Named("shiv".into()), n));
        }
        if s.first() == Some(&"osty's") {
            return Some(Passive::ExtraOstyDamage(n));
        }
        if has(&["upgraded", "attacks"]) {
            return Some(Passive::ExtraDamage(CardFilter::Upgraded(CardType::Attack), n));
        }
        return None;
    }
    if has(&["weak", "deal"]) && has(&["rather", "than"]) {
        return pct.map(Passive::WeakIs);
    }
    if has(&["vulnerable", "take"]) && has(&["rather", "than"]) {
        return pct.map(Passive::VulnerableIs);
    }
    if has(&["block", "from", "defend", "cards"]) {
        return num_before("additional")
            .or_else(|| num_before("block"))
            .map(|n| Passive::ExtraBlock(CardFilter::Named("defend".into()), n));
    }
    if has(&["cards", "you", "play", "each", "turn", "are", "free"]) {
        return s
            .iter()
            .position(|w| *w == "first")
            .and_then(|i| s.get(i + 1))
            .and_then(|w| number(w))
            .map(Passive::FreeFirst);
    }
    if has(&["whenever", "you", "apply", "poison"]) && has(&["additional"]) {
        let after = s.iter().position(|w| *w == "additional").and_then(|i| s.get(i + 1)).and_then(|w| number(w));
        return after.map(Passive::PoisonBonus);
    }
    if has(&["poison", "is", "triggered"]) {
        return num_before("additional").map(Passive::PoisonExtraTicks);
    }
    if has(&["no", "longer", "discard", "your", "hand"]) {
        return Some(Passive::RetainHand);
    }
    if has(&["energy", "is", "now", "conserved"]) {
        return Some(Passive::ConserveEnergy);
    }
    if s.starts_with(&["skills", "cost", "0"]) {
        return Some(Passive::FreeSkills);
    }
    if has(&["hit", "multiple", "times", "hit", "one", "additional", "time"]) {
        return Some(Passive::ExtraHit);
    }
    if has(&["first", "card", "you", "play", "each", "turn", "is", "played", "an", "extra", "time"]) {
        return Some(Passive::EchoEachTurn);
    }
    if has(&["first", "card", "you", "play", "each", "combat", "is", "played", "an", "extra", "time"]) {
        return Some(Passive::EchoFirstOfCombat);
    }
    if has(&["first", "attack", "each", "turn", "deals"]) {
        return pct.map(Passive::FirstAttackBonus);
    }
    None
}

/// Plain sentences: split on full stops and line breaks, keeping "0.5"
/// style numbers intact. A line ending in a comma runs on ("Whenever a card
/// is Exhausted,⏎draw 1 card.").
fn sentences(text: &str) -> Vec<String> {
    text.replace(",\n", ", ").split(['.', '\n']).map(str::trim).filter(|s| !s.is_empty()).map(String::from).collect()
}

/// Reads a power's or relic's text. `played` says whether sentences without
/// a timing phrase happen when the card is played (powers) or are unread
/// (relics, where they are usually out-of-combat rules).
pub fn read(text: &str, orbs: &[Orb], played: bool) -> Mechanics {
    let mut m = Mechanics::default();
    for sentence in sentences(text) {
        let sentence = sentence.as_str();
        let mut ws = words(sentence);
        let lower = sentence.to_lowercase();
        // "The first time you X each turn, …" is "whenever you X", once a turn.
        let first = ws.len() > 4 && ws[..4] == ["the", "first", "time", "you"];
        let per_turn = !lower.contains("each combat");
        if first {
            ws = std::iter::once("whenever".to_string()).chain(ws[3..].iter().cloned()).collect();
        }
        let counted = matches!(timing(&ws).0, Some(When::Every { .. }));
        let per_star = matches!(timing(&ws).0, Some(When::OnStars { each: true, .. }));
        // Conditions and one-offs aren't guessed at.
        let every_melody = matches!(timing(&ws).0, Some(When::OnAllMelodies));
        let conditional = ws.first().is_some_and(|w| w == "if" || w == "while" || (w == "when" && !every_melody))
            || (lower.contains("the first time") && !first)
            || lower.contains("upon pickup")
            || lower.contains("elite")
            || lower.contains("boss")
            // Scaled or temporary amounts aren't flat effects.
            || (lower.contains("for each") && !per_star)
            || lower.contains("equal to")
            || lower.contains("this turn")
            || (lower.contains("for every") && !counted);
        if !conditional && let Some(p) = passive(sentence) {
            // Corruption's second sentence ("Whenever you play a Skill, Exhaust it") is part of FreeSkills.
            m.passives.push(p);
            continue;
        }
        if ws.first().is_some_and(|w| w == "it's" || w == "they're")
            && lower.contains("free to play")
            && let Some(Effect::MakeRandom { free, .. }) = m.triggers.last_mut().and_then(|(_, es)| es.last_mut())
        {
            *free = true;
            continue;
        }
        if conditional {
            m.unread.push(sentence.to_string());
            continue;
        }
        let (when, at) = match timing(&ws) {
            (None, _) => (trailing_timing(&lower), 0),
            found => found,
        };
        let when = when.map(|w| if first { When::First { when: Box::new(w), per_turn } } else { w });
        let found = effects(&ws[at.min(ws.len())..], orbs);
        // A trigger the reader doesn't know ("Whenever you apply Vulnerable,
        // …", "Every 4 Energy you spend, …") isn't an effect on play.
        let unknown_trigger = when.is_none() && ws.first().is_some_and(|w| matches!(w.as_str(), "whenever" | "every"));
        match (when, found.is_empty()) {
            (Some(When::OnPlay(CardFilter::Type(CardType::Skill))), _)
                if m.passives.contains(&Passive::FreeSkills) && lower.contains("exhaust it") => {}
            (Some(w), false) => m.triggers.push((w, found)),
            (None, false) if played && !unknown_trigger => m.triggers.push((When::Played, found)),
            _ => m.unread.push(sentence.to_string()),
        }
    }
    m
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::OrbEffect;

    fn lightning() -> Vec<Orb> {
        vec![Orb {
            id: OrbId::new("LIGHTNING_ORB"),
            name: "Lightning".into(),
            text: String::new(),
            passive: 3,
            passive_effect: OrbEffect::Damage,
            evoke: 8,
            evoke_effect: OrbEffect::Damage,
            focus: true,
            decay: 0,
        }]
    }

    fn triggers(text: &str, played: bool) -> Vec<(When, Vec<Effect>)> {
        read(text, &lightning(), played).triggers
    }

    #[test]
    fn a_line_ending_in_a_comma_runs_on() {
        assert_eq!(
            triggers("Whenever a card is Exhausted,\ndraw 1 card.", true),
            vec![(When::OnExhaust, vec![Effect::Draw(1)])]
        );
    }

    #[test]
    fn reads_cards_made_and_where_they_go() {
        let make = |name: &str, n, pile| Effect::Make { name: name.into(), n, pile };
        assert_eq!(card_text("Add 3 Shivs into your Hand.", &[]).makes, vec![make("shivs", 3, Pile::Hand)]);
        assert_eq!(
            card_text("Deal 13 damage. Add a Soul into your Draw Pile, Hand, and Discard Pile.", &[]).makes,
            vec![make("soul", 1, Pile::Draw), make("soul", 1, Pile::Hand), make("soul", 1, Pile::Discard)]
        );
        assert_eq!(
            triggers("At the start of your turn, add 1 Shiv into your Hand.", true),
            vec![(When::TurnStart, vec![make("shiv", 1, Pile::Hand)])]
        );
        // Random cards and copies aren't named cards.
        assert!(card_text("Add a random Attack into your Hand.", &[]).makes.is_empty());
        let anger = card_text("Deal 6 damage. Add a copy of this card into your Discard Pile.", &[]);
        assert_eq!((anger.makes.len(), anger.copy_to), (0, Some(Pile::Discard)));
        // "Add 3 Shivs" doesn't draw; "Draw 2 cards" does.
        assert!(!card_text("Add 3 Shivs into your Hand.", &[]).draws);
        assert!(card_text("Draw 2 cards.", &[]).draws);
    }

    #[test]
    fn reads_exhaust_and_replay_phrases() {
        let fiend = card_text("Exhaust your Hand. Deal 7 damage for each card Exhausted.", &[]);
        assert_eq!((fiend.exhausts, fiend.own_per_card), (Some(ExhaustWhich::Hand), true));
        let growth = card_text("Exhaust ALL Sadness in your Hand. Gain 4 Stretch for each card Exhausted.", &[]);
        assert_eq!(growth.exhausts, Some(ExhaustWhich::HandNamed("sadness".into())));
        assert_eq!(growth.per_card, vec![Effect::Stretch(4)]);
        let ashen = card_text("Deal 6 damage. Deals 3 additional damage for each card in your Exhaust Pile.", &[]);
        assert_eq!(ashen.per_exhausted, Some((3, None)));
        let storm = card_text("Deal 9 damage. Deals 4 additional damage for each Soul in your Exhaust Pile.", &[]);
        assert_eq!(storm.per_exhausted, Some((4, Some("soul".into()))));
        assert!(card_text("If you Exhausted a card this turn, gain 3 Energy.", &[]).needs_exhaust_this_turn);
        assert_eq!(
            card_text("Draw 2 cards. When this card is Exhausted, gain 2 Energy.", &[]).when_exhausted,
            vec![Effect::Energy(2)]
        );
        let bomb =
            card_text("Deal 18 damage. At the start of your turn, if this is in your Exhaust Pile, play it.", &[]);
        assert_eq!(bomb.replays_from_exhaust, Some(When::TurnStart));
        assert_eq!(
            card_text("This turn, your next Skill is played an extra time.", &[]).next_extra,
            Some(CardType::Skill)
        );
        assert_eq!(card_text("Sovereign Blade gains Replay 1.", &[]).gives_replay, Some(("sovereign blade".into(), 1)));
    }

    #[test]
    fn reads_turn_and_combat_timings() {
        assert_eq!(
            triggers("At the start of your turn, gain 3 Strength.", true),
            vec![(When::TurnStart, vec![Effect::Strength(3)])]
        );
        assert_eq!(
            triggers("Start each combat with 10 Block.", false),
            vec![(When::CombatStart, vec![Effect::Block(10)])]
        );
        assert_eq!(
            triggers("At the start of each combat, draw 2 additional cards.", false),
            vec![(When::CombatStart, vec![Effect::Draw(2)])]
        );
        assert_eq!(
            triggers("At the start of your 2nd turn, gain 14 Block.", false),
            vec![(When::TurnNumber(2), vec![Effect::Block(14)])]
        );
        assert_eq!(
            triggers("At the start of your first 3 turns, draw 2 additional cards.", false),
            vec![(When::FirstTurns(3), vec![Effect::Draw(2)])]
        );
        assert_eq!(
            triggers("Every 3 turns, gain 1 Energy.", false),
            vec![(When::EveryTurns(3), vec![Effect::Energy(1)])]
        );
        assert_eq!(
            triggers("At the end of your turn, gain 1 Star.", false),
            vec![(When::TurnEnd, vec![Effect::Stars(1)])]
        );
        let m = read("Gain 1 Energy at the start of each turn. You can no longer obtain Potions.", &[], false);
        assert_eq!(m.triggers, vec![(When::TurnStart, vec![Effect::Energy(1)])]);
        assert_eq!(m.unread, vec!["You can no longer obtain Potions"]);
        assert_eq!(
            triggers("Gain an additional 1 Energy at the start of your 3rd turn, and every turn after that.", false),
            vec![(When::TurnsFrom(3), vec![Effect::Energy(1)])]
        );
        assert_eq!(
            triggers("Start each combat with an additional 1 Energy.", false),
            vec![(When::CombatStart, vec![Effect::Energy(1)])]
        );
    }

    #[test]
    fn reads_play_triggers_and_owners() {
        assert_eq!(
            triggers("Whenever you play an Attack, gain 1 Block.", false),
            vec![(When::OnPlay(CardFilter::Type(CardType::Attack)), vec![Effect::Block(1)])]
        );
        assert_eq!(
            triggers("Whenever you play a card that costs 2 Energy or more, gain 4 Block.", false),
            vec![(When::OnPlay(CardFilter::CostAtLeast(2)), vec![Effect::Block(4)])]
        );
        assert_eq!(
            triggers("Every time you play 3 Attacks in a single turn, gain 1 Strength.", false),
            vec![(
                When::EveryPlays { filter: CardFilter::Type(CardType::Attack), n: 3, per_turn: true },
                vec![Effect::Strength(1)]
            )]
        );
        assert_eq!(
            triggers("Every 4 cards you play, draw 1 card.", false),
            vec![(When::EveryPlays { filter: CardFilter::Any, n: 4, per_turn: false }, vec![Effect::Draw(1)])]
        );
        // Enemies gaining Strength is not our gain.
        assert_eq!(
            triggers("At the start of your turn, gain 2 Strength and ALL enemies gain 1 Strength.", false),
            vec![(When::TurnStart, vec![Effect::Strength(2)])]
        );
        assert_eq!(
            triggers("Whenever you play a Power, Channel 1 Lightning.", true)[0].1,
            vec![Effect::Channel(OrbId::new("LIGHTNING_ORB"), 1)]
        );
        assert_eq!(triggers("Gain 2 Strength. Gain 2 Dexterity.", true).len(), 2);
        assert_eq!(triggers("Gain 2 Strength.", false), vec![], "relic sentences without timing aren't played");
    }

    #[test]
    fn reads_passives_and_leaves_conditions() {
        let m = read("Cards containing “Strike” deal 3 additional damage.", &[], false);
        assert_eq!(m.passives, vec![Passive::ExtraDamage(CardFilter::Named("strike".into()), 3)]);
        let m = read("Vulnerable enemies take an additional 25% damage.", &[], true);
        assert_eq!(m.passives, vec![Passive::VulnerableBonus(25)]);
        let m = read("Skills cost 0 1 Energy. Whenever you play a Skill, Exhaust it.", &[], true);
        assert_eq!((m.passives, m.unread.len()), (vec![Passive::FreeSkills], 0));
        let m = read("If you end your turn without Block, gain 6 Block.", &[], false);
        assert!(m.is_empty() && m.unread.len() == 1);
        let m = read("The first time you lose HP each combat, draw 3 cards.", &[], false);
        assert_eq!(
            m.triggers,
            vec![(When::First { when: Box::new(When::OnHpLoss), per_turn: false }, vec![Effect::Draw(3)])]
        );
        let m = read("Whenever you apply Poison, apply an additional 1 Poison.", &[], false);
        assert_eq!(m.passives, vec![Passive::PoisonBonus(1)]);
    }

    #[test]
    fn reads_event_triggers() {
        let t = |text: &str| triggers(text, true);
        assert_eq!(
            t("Whenever you lose HP on your turn, gain 1 Strength."),
            vec![(When::OnHpLoss, vec![Effect::Strength(1)])]
        );
        assert_eq!(
            t("Whenever you apply Vulnerable, draw 1 card."),
            vec![(When::OnApply(Debuff::Vulnerable), vec![Effect::Draw(1)])]
        );
        assert_eq!(
            t("Whenever you apply a debuff to an enemy, they take 9 damage."),
            vec![(When::OnApply(Debuff::Any), vec![Effect::Damage(9)])]
        );
        assert_eq!(
            t("Whenever you create a Status, deal 5 damage to ALL enemies."),
            vec![(When::OnMake(CardFilter::Type(CardType::Status)), vec![Effect::DamageAll(5)])]
        );
        assert_eq!(
            t("Whenever you create a card, gain 1 Strength."),
            vec![(When::OnMake(CardFilter::Any), vec![Effect::Strength(1)])]
        );
        assert_eq!(
            t("Whenever you spend or gain 1 Star, deal 3 damage to ALL enemies."),
            vec![(When::OnStars { gain: true, spend: true, each: false }, vec![Effect::DamageAll(3)])]
        );
        assert_eq!(
            t("Whenever you spend 1 Star, gain 2 Block for each 1 Star spent."),
            vec![(When::OnStars { gain: false, spend: true, each: true }, vec![Effect::Block(2)])]
        );
        assert_eq!(
            t("Whenever you Evoke Lightning, deal 8 damage to each enemy hit."),
            vec![(When::OnEvoke(Some(OrbId::new("LIGHTNING_ORB"))), vec![Effect::Damage(8)])]
        );
        assert_eq!(
            t("Whenever you play a Soul, a random enemy loses 7 HP."),
            vec![(When::OnPlay(CardFilter::Named("soul".into())), vec![Effect::Damage(7)])]
        );
        assert_eq!(
            t("Whenever you play an Ethereal card, gain 4 Block."),
            vec![(When::OnPlay(CardFilter::Keyword("ethereal".into())), vec![Effect::Block(4)])]
        );
        assert_eq!(
            t("Whenever you draw an Ethereal card, draw 1 card."),
            vec![(When::OnDrawOf(CardFilter::Keyword("ethereal".into())), vec![Effect::Draw(1)])]
        );
        assert_eq!(
            t("Every 5 times you Exhaust a card, draw 1 card."),
            vec![(When::Every { what: Counted::Exhausts, n: 5 }, vec![Effect::Draw(1)])]
        );
        assert_eq!(
            t("Every 4 Energy you spend, gain 1 Energy."),
            vec![(When::Every { what: Counted::EnergySpent, n: 4 }, vec![Effect::Energy(1)])]
        );
        assert_eq!(
            t("Sovereign Blade now gains 10 Block."),
            vec![(When::OnPlay(CardFilter::Named("sovereign blade".into())), vec![Effect::Block(10)])]
        );
        assert_eq!(
            t("At the end of your turn, Evoke your leftmost Orb."),
            vec![(When::TurnEnd, vec![Effect::Evoke(OrbPick::Leftmost, 1)])]
        );
        assert_eq!(
            t("At the start of your turn, trigger the passive ability of your rightmost Orb 2 times."),
            vec![(When::TurnStart, vec![Effect::OrbPassive(OrbPick::Rightmost, 2)])]
        );
        let c = card_text("Evoke your rightmost Orb twice.", &lightning());
        assert_eq!(c.actions, vec![Effect::Evoke(OrbPick::Rightmost, 2)]);
        let rage = card_text("Whenever you play an Attack this turn, gain 3 Block.", &[]);
        assert_eq!(rage.this_turn, vec![(When::OnPlay(CardFilter::Type(CardType::Attack)), vec![Effect::Block(3)])]);
        assert_eq!(card_text("Lose 3 HP. Gain 2 Energy.", &[]).lose_hp, 3);
        assert_eq!(
            card_text("At the end of your turn, if this is in your Hand, take 2 damage.", &[]).held_end,
            vec![Effect::LoseHp(2)]
        );
    }
}
