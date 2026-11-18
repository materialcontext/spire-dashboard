//! Reading combat effects from card and relic text.
//!
//! The data gives numbers for a card's own effect (`vars`), but powers and
//! relics describe what they do in prose: "At the start of your turn, gain 3
//! Strength", "Whenever you play an Attack, gain 1 Block". This module reads
//! that prose into [`Trigger`]s and [`Passive`]s the deck simulator can run.
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

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Trigger {
    pub when: Option<When>,
    pub effects: Vec<Effect>,
}

/// What the simulator understood of a text, and what it didn't.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Mechanics {
    pub triggers: Vec<(When, Vec<Effect>)>,
    pub passives: Vec<Passive>,
    /// Sentences left unread, as written.
    pub unread: Vec<String>,
}

impl Mechanics {
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

/// Plain sentences: split on full stops, keeping "0.5" style numbers intact.
fn sentences(text: &str) -> Vec<&str> {
    text.split(['.', '\n']).map(str::trim).filter(|s| !s.is_empty()).collect()
}

/// Reads a power's or relic's text. `played` says whether sentences without
/// a timing phrase happen when the card is played (powers) or are unread
/// (relics, where they are usually out-of-combat rules).
pub fn read(text: &str, orbs: &[Orb], played: bool) -> Mechanics {
    let mut m = Mechanics::default();
    for sentence in sentences(text) {
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
