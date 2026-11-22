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
    Forge(i32),
    Stars(i32),
    Summon(i32),
    Channel(OrbId, i32),
    OrbSlots(i32),
    /// Arrows for the Fool.
    Arrows(i32),
    /// Cards made and put in a pile: "add 1 Shiv into your Hand". `name`
    /// is as written (plural or not); the simulator finds the card.
    Make {
        name: String,
        n: i32,
        pile: Pile,
    },
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
            Effect::Forge(n) => format!("Forge {n}"),
            Effect::Stars(n) => signed(*n, "Stars"),
            Effect::Summon(n) => format!("Summon {n}"),
            Effect::Channel(orb, n) => format!("Channel {n} {}", orb.as_str().trim_end_matches("_ORB").to_lowercase()),
            Effect::OrbSlots(n) => signed(*n, "orb slots"),
            Effect::Arrows(n) => signed(*n, "Arrows"),
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
            Effect::Forge(_) => "forge",
            Effect::Stars(_) => "stars",
            Effect::Summon(_) => "summon",
            Effect::Channel(..) => "channel",
            Effect::OrbSlots(_) => "orb_slots",
            Effect::Arrows(_) => "arrows",
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
    matches!(w.trim_end_matches('s'), "shiv" | "strike" | "defend")
        .then(|| CardFilter::Named(w.trim_end_matches('s').into()))
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
            .skip(3)
            .find(|(_, w)| matches!(**w, "gain" | "deal" | "draw" | "channel" | "apply" | "summon" | "forge"))
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
        let theirs = i > 0 && matches!(s[i - 1], "enemies" | "enemy" | "allies" | "players" | "player");
        match s[i] {
            "gain" | "gains" | "with" | "lose" if !theirs => {
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
            "add" | "adds" if !theirs || s[i - 1] == "players" => {
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
    let name = name.join(" ");
    Some((piles.into_iter().map(|pile| Effect::Make { name: name.clone(), n, pile }).collect(), j))
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
        // "Draw 2 cards", "draw a card"; not "into your Draw Pile".
        if s.windows(2).any(|w| w[0] == "draw" && (number(w[1]).is_some() || w[1] == "a")) {
            c.draws = true;
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
    if has(&["vulnerable", "take"]) && has(&["rather", "than"]) {
        return pct.map(Passive::VulnerableIs);
    }
    if has(&["block", "from", "defend", "cards"]) {
        return num_before("additional")
            .or_else(|| num_before("block"))
            .map(|n| Passive::ExtraBlock(CardFilter::Named("defend".into()), n));
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
        let ws = words(sentence);
        let lower = sentence.to_lowercase();
        // Conditions and one-offs aren't guessed at.
        let conditional = ws.first().is_some_and(|w| w == "if" || w == "while" || w == "when")
            || lower.contains("the first time")
            || lower.contains("upon pickup")
            || lower.contains("elite")
            || lower.contains("boss")
            // Scaled or temporary amounts aren't flat effects.
            || lower.contains("for each")
            || lower.contains("equal to")
            || lower.contains("this turn")
            || lower.contains("for every");
        if !conditional && let Some(p) = passive(sentence) {
            // Corruption's second sentence ("Whenever you play a Skill, Exhaust it") is part of FreeSkills.
            m.passives.push(p);
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
        let found = effects(&ws[at.min(ws.len())..], orbs);
        match (when, found.is_empty()) {
            (Some(When::OnPlay(CardFilter::Type(CardType::Skill))), _)
                if m.passives.contains(&Passive::FreeSkills) && lower.contains("exhaust it") => {}
            (Some(w), false) => m.triggers.push((w, found)),
            (None, false) if played => m.triggers.push((When::Played, found)),
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
        assert!(m.is_empty());
    }
}
