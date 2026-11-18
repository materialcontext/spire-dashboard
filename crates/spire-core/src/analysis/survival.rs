//! Is the deck on track to survive a fight, and which way wins it?
//!
//! Plays the fight out on averages: each turn the deck deals the damage the
//! simulator expects when attacking and blocks what it expects when
//! defending; the enemies deal their expected damage for that turn of their
//! pattern. The player acts first, so a fight won on turn N takes N − 1
//! enemy turns. It's played four ways, each a way players win fights:
//!
//! - **Race:** attack every turn and take the hits.
//! - **Wall:** block what each turn brings, attack with the rest.
//! - **Windows:** block the spike turns in full, attack all-out between
//!   them (only for fights with spikes).
//! - **Scale:** split every turn, letting the deck's growth win late (the
//!   best of a quarter, half or three quarters to Block).
//!
//! The lead way is the one that loses least HP, unless the fight has a
//! clock (ramp, summons, blast, statuses, revives) and it takes clearly
//! longer than players usually do: then the fastest way that survives.
//!
//! **Fight rules** change the damage that lands, from what the simulator
//! counts per turn (hits, damage per hit, cards and Skills played):
//! Slippery spends early hits at 1 HP each, Intangible turns a turn's damage
//! into its hits, Skittish's Block soaks the first hit, Hardened Shell caps
//! a turn, Slow adds damage per card played, Personal Hive adds a Dazed per
//! hit, Painful Stabs a Wound per turn with unblocked damage, Enrage
//! Strength per Skill, and Shriek or Plow skip an enemy turn at an HP line.
//!
//! Monsters an on-death power summons, and phases a monster revives into,
//! fight as later stages: once the deck has dealt a stage's HP, the next
//! stage's pattern starts on the following enemy turn.
//!
//! What it leaves out, so it reads as a guide, not a forecast:
//! - Enemies in a stage die together: damage from one that falls early
//!   still counts, and damage past a stage's HP carries into the next.
//! - Debuffs on the player (Weak, Frail) and on enemies are not modelled,
//!   except Vulnerable the pattern puts on the player.
//! - Rules are read from the power's text and amount; Intangible from a
//!   move counts as every other turn, and Slippery isn't regained.
//! - Potions, and cards the simulator can't read.

use crate::analysis::jobs;
use crate::analysis::rules::{self, Kind as RuleKind};
use crate::analysis::shape;
use crate::analysis::sim::{DeckSim, PlayStats};
use crate::analysis::tags::Tag;
use crate::analysis::threats::{Scaling, Wave, lineup_incoming, lineup_statuses, lineup_waves};
use crate::catalog::{Catalog, Encounter};
use crate::playbook::{Playbook, Role};
use crate::run::RunState;

/// Player turns played out before giving up on a kill.
pub const MAX_TURNS: usize = 20;

/// Lost HP below this share of current HP reads as on track.
const SAFE_SHARE: f32 = 0.6;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    /// Expected to win with HP to spare.
    OnTrack,
    /// Expected to win, but with little left.
    Tight,
    /// Expected to lose more HP than the player has, or not to win at all.
    AtRisk,
}

impl Verdict {
    pub fn label(self) -> &'static str {
        match self {
            Verdict::OnTrack => "On track",
            Verdict::Tight => "Tight",
            Verdict::AtRisk => "At risk",
        }
    }
}

/// A way to play a fight out.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Way {
    /// Attack every turn.
    Race,
    /// Block what each turn brings (up to a full turn), attack with the rest.
    Wall,
    /// Block turns bringing at least this much in full; attack otherwise.
    Windows(u32),
    /// A fixed share of every turn goes to Block.
    Scale(f32),
}

impl Way {
    pub fn label(self) -> &'static str {
        match self {
            Way::Race => "Race",
            Way::Wall => "Wall",
            Way::Windows(_) => "Windows",
            Way::Scale(_) => "Scale",
        }
    }

    /// What the way does, in plain words.
    pub fn describe(self) -> String {
        match self {
            Way::Race => "attack every turn and take the hits".into(),
            Way::Wall => "block what each turn brings, attack with the rest".into(),
            Way::Windows(at) => format!("block the turns that bring {at}+ in full, attack all-out between them"),
            Way::Scale(s) => format!("put about {:.0}% of every turn into Block and let the deck grow", s * 100.0),
        }
    }
}

/// What a stage's fight rules do to the deck's damage and the fight.
/// Amounts are the powers' as the data gives them.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Mods {
    /// HP losses that are 1 each (Slippery), spent by the first hits.
    pub slippery: f32,
    /// Share of player turns the enemy is Intangible (each hit does 1).
    pub intangible: f32,
    /// Block it gains on the first hit each turn (Skittish).
    pub skittish: f32,
    /// Most HP it can lose in a turn (Hardened Shell).
    pub cap: Option<f32>,
    /// Extra damage per card played this turn, as a share (Slow).
    pub per_card: f32,
    /// Junk cards per Attack hit on it (Personal Hive).
    pub junk_per_hit: f32,
    /// That junk is Ethereal (Dazed): it leaves once drawn, so it thins
    /// only the draws until then instead of piling up.
    pub junk_fades: bool,
    /// Junk cards per turn its attacks get through (Painful Stabs).
    pub junk_unblocked: f32,
    /// Strength it gains per Skill you play (Enrage).
    pub per_skill: f32,
    /// It's stunned (skips a turn) the first time its HP is at or under this.
    pub stun_at: Option<f32>,
}

impl Mods {
    /// The rules of a stage's monsters. Rules about hits on a monster
    /// count once (the deck hits one at a time); rules about what the
    /// enemies do add up. `fades` says whether a rule's junk is Ethereal.
    pub fn of(rules: &[rules::Rule], fades: impl Fn(&rules::Rule) -> bool) -> Mods {
        let mut m = Mods::default();
        for r in rules {
            let n = r.power.amount.unwrap_or(1).max(1) as f32;
            match r.kind {
                RuleKind::HitsLoseOne => m.slippery = m.slippery.max(n),
                RuleKind::Intangible => m.intangible = 0.5,
                RuleKind::BlockOnFirstHit => m.skittish = m.skittish.max(n),
                RuleKind::HpCapPerTurn => m.cap = Some(m.cap.map_or(n, |c| c.min(n))),
                RuleKind::MorePerCard => m.per_card = m.per_card.max(0.1 * n),
                RuleKind::JunkPerHit => {
                    m.junk_per_hit = m.junk_per_hit.max(n);
                    m.junk_fades = fades(r);
                }
                RuleKind::PunishesUnblocked => m.junk_unblocked += n,
                RuleKind::PunishesSkills => m.per_skill += n,
                RuleKind::StunAtHp => m.stun_at = Some(n),
                _ => {}
            }
        }
        m
    }

    /// The damage that lands from `attack` this turn, with `hits` landed
    /// and `cards` played; spends Slippery from `slippery`.
    fn land(&self, attack: f32, hits: f32, per_hit: f32, cards: f32, slippery: &mut f32) -> f32 {
        if attack <= 0.0 {
            return 0.0;
        }
        // Slow: each card played adds to the attacks after it, about half
        // the turn's cards on average.
        let mut d = attack * (1.0 + self.per_card * cards / 2.0);
        if *slippery > 0.0 {
            let spent = slippery.min(hits);
            d -= spent * (per_hit - 1.0).max(0.0);
            *slippery -= spent;
        }
        d = (d - self.skittish).max(0.0);
        if self.intangible > 0.0 {
            d = (1.0 - self.intangible) * d + self.intangible * hits.min(d);
        }
        self.cap.map_or(d, |c| d.min(c)).max(0.0)
    }
}

/// Enemies that fight together: their expected damage and Status cards
/// per enemy turn from when they arrive, their combined HP and their rules.
#[derive(Clone, Copy, Debug)]
pub struct Stage<'a> {
    pub incoming: &'a [f32],
    /// Unplayable cards they add to the deck on each enemy turn (expected).
    pub statuses: &'a [f32],
    pub hp: u32,
    pub mods: Mods,
}

/// What the deck is up against.
#[derive(Clone, Copy, Debug)]
pub struct Foe<'a> {
    /// In order: each stage arrives once the one before has fallen.
    pub stages: &'a [Stage<'a>],
    /// Damage the enemies still deal at the end of the player's turn after
    /// the kill (Waterfall Giant's Steam Eruption), at least this much.
    pub blast: Option<f32>,
    /// The deck's size before any Status cards: those dilute every later
    /// draw, so the deck's damage and block shrink by `deck / (deck + added)`.
    pub deck: f32,
    /// Cards drawn each turn, for how fast Ethereal junk leaves.
    pub hand: f32,
    /// Turns at or over this damage are spikes: the Windows way blocks them.
    pub spike_line: Option<f32>,
    /// The fight punishes slow play (ramp, summons, blast, statuses, revives).
    pub clocked: bool,
    /// How many turns players usually take (community average).
    pub pace: Option<f32>,
    /// Turns over the pace that count as clearly slower.
    pub slower_by: f32,
}

impl Foe<'_> {
    fn hp(&self) -> u32 {
        self.stages.iter().map(|s| s.hp).sum()
    }
}

/// Monsters summoned mid-fight, for the explanation.
#[derive(Clone, Debug, PartialEq)]
pub struct Join {
    pub name: String,
    pub count: u32,
    /// The power that summons them.
    pub power: String,
}

/// One way played out.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Line {
    pub way: Way,
    /// The turn a blow after death lands (the turn after the kill).
    pub blast_turn: Option<u32>,
    /// Unplayable cards the enemies had added by the end.
    pub junk: f32,
    /// Player turn the last enemy falls on.
    pub kill_turn: Option<u32>,
    /// HP lost by then (or by `MAX_TURNS`).
    pub hp_loss: u32,
}

/// What holds the deck back in a fight: the role that would fix it (none
/// for junk, which every role suffers from), why, and the tags to look for.
#[derive(Clone, Debug, PartialEq)]
pub struct Limit {
    pub role: Option<Role>,
    pub text: String,
    pub tags: Vec<Tag>,
}

impl Limit {
    pub fn label(&self) -> &'static str {
        self.role.map_or(Tag::CleanUp.label(), Role::label)
    }
}

/// A fight rule that changes the play, in words, with the tags that
/// answer it.
#[derive(Clone, Debug, PartialEq)]
pub struct RuleNote {
    pub text: String,
    pub tags: Vec<Tag>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Outlook {
    pub verdict: Verdict,
    /// The way to lead with.
    pub lead: Line,
    /// Every way played: Race, Wall, Windows (with spikes), Scale.
    pub ways: Vec<Line>,
    pub hp: i32,
    /// Unplayable cards counted per enemy Status move (an assumption; 0
    /// when the enemies have none).
    pub status_cards: u32,
    /// Monsters summoned once others die, in the order they join.
    pub joins: Vec<Join>,
    /// The fight's rules that change the play.
    pub rules: Vec<RuleNote>,
    /// What holds the deck back, most pressing first.
    pub limits: Vec<Limit>,
}

impl Outlook {
    /// A way's result, if it was played.
    pub fn way(&self, label: &str) -> Option<&Line> {
        self.ways.iter().find(|l| l.way.label() == label)
    }
}

/// Plays out one way.
fn play(sim: &DeckSim, foe: Foe, way: Way) -> Line {
    let output = |t: usize| sim.turns[t.min(sim.turns.len() - 1)];
    let share_for = |coming: f32, block: f32| {
        let need = if block > 0.0 { (coming / block).min(1.0) } else { 0.0 };
        match way {
            Way::Race => 0.0,
            Way::Scale(s) => s,
            Way::Wall => need,
            Way::Windows(at) if coming >= at as f32 => need,
            Way::Windows(_) => 0.0,
        }
    };
    let Some(first) = foe.stages.first() else {
        return Line { way, blast_turn: None, junk: 0.0, kill_turn: None, hp_loss: 0 };
    };
    let PlayStats { hits, per_hit, cards, skills, .. } = sim.play;
    let mut dealt = 0.0f32;
    let mut lost = 0.0f32;
    let mut junk = 0.0f32;
    // Ethereal junk still in the deck (it leaves once drawn), and how much
    // was ever added (for the explanation).
    let (mut passing, mut passed) = (0.0f32, 0.0f32);
    // The stage being fought, the enemy turn it arrived on, and the damage
    // that clears it and every stage before.
    let (mut stage, mut since, mut clears) = (0usize, 0usize, first.hp as f32);
    let mut slippery = first.mods.slippery;
    // Strength the stage gained from your Skills (Enrage), and a stun due.
    let (mut enraged, mut stunned, mut stun_used) = (0.0f32, false, false);
    let incoming = |stage: usize, t: usize| foe.stages[stage].incoming.get(t).copied().unwrap_or(0.0);
    let statuses = |stage: usize, t: usize| foe.stages[stage].statuses.get(t).copied().unwrap_or(0.0);
    // Status cards thin out the draws: output scales with the share of the
    // deck that's still playable (a hand-limited deck loses about that much).
    let thinned = |junk: f32| if foe.deck > 0.0 { foe.deck / (foe.deck + junk) } else { 1.0 };
    for t in 0..MAX_TURNS {
        let out = output(t);
        let keep = thinned(junk + passing);
        let mods = foe.stages[stage].mods;
        let (attack, block) = (out.damage.mean * keep, out.block.mean * keep);
        let coming = incoming(stage, t - since) + enraged;
        let share = share_for(coming, block);
        let attacking = 1.0 - share;
        let landed = mods.land(attack * attacking, hits * attacking * keep, per_hit, cards * keep, &mut slippery);
        dealt += landed;
        let added = mods.junk_per_hit * hits * attacking * keep;
        if mods.junk_fades {
            // This turn's draw already happened; the cards drawn next turn
            // take their share of it with them.
            passing += added;
            passed += added;
        } else {
            junk += added;
        }
        if !stun_used && mods.stun_at.is_some_and(|line| clears - dealt <= line) {
            stunned = true;
            stun_used = true;
        }
        while dealt >= clears && stage + 1 < foe.stages.len() {
            stage += 1;
            since = t;
            clears += foe.stages[stage].hp as f32;
            slippery = foe.stages[stage].mods.slippery;
            (enraged, stun_used) = (0.0, false);
        }
        if dealt >= clears {
            // A blow after death: one more hit at the end of the next turn,
            // with that turn's block (there's nothing left to attack, but
            // the way still splits the same way).
            let blast_turn = foe.blast.map(|floor| {
                let blast = floor.max(incoming(stage, t + 1 - since));
                let block = output(t + 1).block.mean * keep;
                lost += (blast - block * share_for(blast, block)).max(0.0);
                t as u32 + 2
            });
            let junk = junk + passed;
            return Line { way, blast_turn, junk, kill_turn: Some(t as u32 + 1), hp_loss: lost.round() as u32 };
        }
        // A stage that arrived this turn takes its first enemy turn now.
        let coming = if stunned { 0.0 } else { incoming(stage, t - since) + enraged };
        stunned = false;
        let through = (coming - block * share_for(coming, block)).max(0.0);
        lost += through;
        if through > 0.0 {
            junk += foe.stages[stage].mods.junk_unblocked;
        }
        junk += statuses(stage, t - since);
        enraged += foe.stages[stage].mods.per_skill * skills;
        let pile = foe.deck + junk + passing;
        if pile > 0.0 {
            passing *= 1.0 - (foe.hand / pile).min(1.0);
        }
    }
    Line { way, blast_turn: None, junk: junk + passed, kill_turn: None, hp_loss: lost.round() as u32 }
}

/// How a fight is likely to go for a deck at `hp`, played every way. `None`
/// when the simulator has nothing to go on (no deck).
pub fn outlook(sim: &DeckSim, foe: Foe, hp: i32) -> Option<Outlook> {
    if sim.turns.is_empty() || foe.hp() == 0 {
        return None;
    }
    let mut ways = vec![play(sim, foe, Way::Race), play(sim, foe, Way::Wall)];
    if let Some(line) = foe.spike_line.filter(|l| *l > 0.0) {
        ways.push(play(sim, foe, Way::Windows(line.round() as u32)));
    }
    let rank = |l: &Line| (l.kill_turn.is_none(), l.hp_loss, l.kill_turn);
    ways.extend([0.25, 0.5, 0.75].into_iter().map(|s| play(sim, foe, Way::Scale(s))).min_by_key(rank));
    let least = *ways.iter().min_by_key(|l| rank(l))?;
    // A clock makes speed matter: past the players' pace, lead with the
    // fastest way that still survives.
    let lead = match (foe.clocked, foe.pace, least.kill_turn) {
        (true, Some(pace), Some(k)) if k as f32 > pace + foe.slower_by => ways
            .iter()
            .filter(|l| l.kill_turn.is_some() && (l.hp_loss as i32) < hp)
            .min_by_key(|l| (l.kill_turn, l.hp_loss))
            .copied()
            .unwrap_or(least),
        _ => least,
    };
    let verdict = match lead.kill_turn {
        None => Verdict::AtRisk,
        Some(_) if lead.hp_loss as f32 >= hp as f32 => Verdict::AtRisk,
        Some(_) if lead.hp_loss as f32 >= hp as f32 * SAFE_SHARE => Verdict::Tight,
        Some(_) => Verdict::OnTrack,
    };
    Some(Outlook { verdict, lead, ways, hp, status_cards: 0, joins: Vec::new(), rules: Vec::new(), limits: Vec::new() })
}

/// A stage's rules, from its monsters' powers.
fn stage_rules(wave: &Wave, catalog: &Catalog) -> Vec<rules::Rule> {
    wave.plans.iter().filter_map(|p| catalog.monsters.get(&p.monster)).flat_map(rules::of).collect()
}

/// The rules that change the play, in words, with what answers them.
fn rule_words(rules: &[rules::Rule], play: &PlayStats) -> Vec<RuleNote> {
    let mut out: Vec<RuleNote> = Vec::new();
    for r in rules {
        let n = r.power.amount.unwrap_or(1);
        let name = crate::analysis::pattern::power_words(&r.power.power);
        let text = match r.kind {
            RuleKind::HitsLoseOne => {
                format!("{name} {n}: its next {n} HP losses are 1 each; you land {:.1} hits a turn", play.hits)
            }
            RuleKind::Intangible => {
                format!("{name}: some turns every hit does 1; you land {:.1} hits a turn", play.hits)
            }
            RuleKind::BlockOnFirstHit => format!("{name} {n}: your first hit each turn gives it {n} Block"),
            RuleKind::HpCapPerTurn => format!("{name} {n}: it loses at most {n} HP a turn"),
            RuleKind::MorePerCard => format!("{name}: more damage per card played; you play {:.1} a turn", play.cards),
            RuleKind::JunkPerHit => format!("{name}: a Dazed per hit; you land {:.1} hits a turn", play.hits),
            RuleKind::PunishesUnblocked => format!("{name}: a Wound each turn its attacks get through"),
            RuleKind::PunishesSkills => {
                format!("{name} {n}: {n} Strength per Skill; you play {:.1} a turn", play.skills)
            }
            RuleKind::StunAtHp => format!("{name}: stunned once at {n} HP"),
            _ => continue,
        };
        if !out.iter().any(|n| n.text == text) {
            let mut tags: Vec<Tag> = r.kind.asks().iter().flat_map(|a| a.tags()).copied().collect();
            tags.dedup();
            out.push(RuleNote { text, tags });
        }
    }
    out
}

/// What holds the deck back, as the roles that would fix it and the tags
/// that give them.
fn limits(
    shape: &shape::Shape,
    jobs: &jobs::Jobs,
    lead: &Line,
    foe: &Foe,
    mods: &Mods,
    play: &PlayStats,
    playbook: &Playbook,
) -> Vec<Limit> {
    let mut out = Vec::new();
    let mut add = |role: Role, text: String| out.push(Limit { role: Some(role), text, tags: playbook.tags_for(role) });
    let biggest = shape.spikes.iter().filter(|s| s.certain).map(|s| s.load.expected).fold(0.0f32, f32::max);
    if biggest > 0.0 && jobs.survival < biggest {
        add(Role::Survival, format!("spikes of {biggest:.0}; a defending turn blocks {:.0}", jobs.survival));
    }
    if shape.pressure > 0.0 && jobs.mitigation < shape.pressure {
        add(
            Role::Mitigation,
            format!("about {:.0} a turn; an ordinary turn blocks {:.0}", shape.pressure, jobs.mitigation),
        );
    }
    if let (Some(pace), Some(k)) = (foe.pace, lead.kill_turn)
        && k as f32 > pace + foe.slower_by
    {
        add(Role::FastDamage, format!("the kill takes about {k} turns; players usually finish in {pace:.0}"));
    }
    let chip = mods.slippery > 0.0 || mods.intangible > 0.0 || mods.skittish > 0.0;
    if chip && play.hits < playbook.fights.chip_hits {
        add(Role::Chip, format!("its rules punish few hits; you land {:.1} a turn", play.hits));
    }
    if shape.stages.len() > 1 && shape.clocks.contains(&shape::Clock::Summons) && jobs.aoe < 0.1 {
        add(Role::Aoe, "enemies join the fight; little of your damage hits them all".into());
    }
    // Junk isn't a role's problem: it thins every draw.
    if foe.deck > 0.0 && lead.junk >= foe.deck * playbook.fights.junk_share {
        out.push(Limit {
            role: None,
            text: format!("about {:.0} junk cards by the end, against a {:.0}-card deck", lead.junk, foe.deck),
            tags: vec![Tag::CleanUp, Tag::Draw],
        });
    }
    out
}

/// The outlook for the run's deck and HP against an encounter's most
/// common lineup, following each monster's move pattern and its rules.
pub fn for_encounter(
    sim: &DeckSim,
    enc: &Encounter,
    catalog: &Catalog,
    state: &RunState,
    playbook: &Playbook,
) -> Option<Outlook> {
    let scaling = Scaling::for_run(state, catalog);
    let waves = lineup_waves(enc, catalog, scaling, MAX_TURNS);
    let per = playbook.simulation.status_cards as f32;
    let series: Vec<(Vec<f32>, Vec<f32>)> = waves
        .iter()
        .map(|w| {
            let incoming = lineup_incoming(&w.plans, MAX_TURNS).into_iter().map(|(_, expected, _)| expected).collect();
            let statuses = lineup_statuses(&w.plans, MAX_TURNS).into_iter().map(|n| n * per).collect();
            (incoming, statuses)
        })
        .collect();
    // Junk a rule adds leaves once drawn when the card it names is Ethereal.
    let fades = |r: &rules::Rule| {
        catalog.cards.values().any(|c| r.power.text.contains(c.name.as_str()) && c.has_keyword("Ethereal", false))
    };
    let wave_rules: Vec<Vec<rules::Rule>> = waves.iter().map(|w| stage_rules(w, catalog)).collect();
    let stages: Vec<Stage> = waves
        .iter()
        .zip(&series)
        .zip(&wave_rules)
        .map(|((w, (incoming, statuses)), r)| Stage { incoming, statuses, hp: w.hp, mods: Mods::of(r, fades) })
        .collect();
    // The blast follows the last kill. Its size grows over the fight and
    // the data gives only its starting value, so it counts as at least
    // that, and at least the turn it would otherwise have had.
    let blast = waves
        .last()
        .into_iter()
        .flat_map(|w| &w.plans)
        .filter_map(|p| catalog.monsters.get(&p.monster)?.death_blow.as_ref())
        .map(|b| b.amount.unwrap_or(0) as f32)
        .reduce(f32::max);
    let shape = shape::of(enc, catalog, scaling, &playbook.fights);
    let foe = Foe {
        stages: &stages,
        blast,
        deck: state.deck.len() as f32,
        hand: playbook.simulation.hand_size as f32,
        spike_line: shape.spikes.iter().any(|s| s.certain).then_some(shape.pressure * playbook.fights.spike),
        clocked: !shape.clocks.is_empty(),
        pace: enc.stats.as_ref().map(|s| s.avg_turns).filter(|p| *p > 0.0),
        slower_by: playbook.fights.slower_by,
    };
    let any = series.iter().any(|(_, s)| s.iter().any(|n| *n > 0.0));
    let joins = waves
        .iter()
        .filter_map(|w| {
            let power = w.summoned_by.clone()?;
            w.plans.first().map(|p| Join { name: p.name.clone(), count: p.count, power })
        })
        .collect();
    let all_rules: Vec<rules::Rule> = wave_rules.into_iter().flatten().collect();
    let mods = Mods::of(&all_rules, fades);
    let jobs = jobs::measure(sim);
    outlook(sim, foe, state.hp).map(|o| {
        let limits = limits(&shape, &jobs, &o.lead, &foe, &mods, &sim.play, playbook);
        Outlook {
            status_cards: if any { playbook.simulation.status_cards } else { 0 },
            joins,
            rules: rule_words(&all_rules, &sim.play),
            limits,
            ..o
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::sim::{Spread, TurnOutput};

    fn sim(attack: f32, block: f32) -> DeckSim {
        let s = |m| Spread { mean: m, low: m, high: m };
        DeckSim { turns: vec![TurnOutput { damage: s(attack), block: s(block) }; 5], ..Default::default() }
    }

    fn stage(incoming: &[f32], hp: u32) -> [Stage<'_>; 1] {
        [Stage { incoming, statuses: &[], hp, mods: Mods::default() }]
    }

    fn foe<'a>(stages: &'a [Stage<'a>], blast: Option<f32>) -> Foe<'a> {
        Foe { stages, blast, deck: 10.0, hand: 5.0, spike_line: None, clocked: false, pace: None, slower_by: 1.5 }
    }

    fn race(o: &Outlook) -> Line {
        *o.way("Race").unwrap()
    }

    #[test]
    fn summoned_monsters_fight_after_the_first_falls() {
        // 20 a turn: the first 40 HP fall on turn 2; the summons arrive
        // stunned on that enemy turn, then hit 15 a turn until their 60 HP
        // fall on turn 5.
        let mut summoned = [15.0f32; 20];
        summoned[0] = 0.0;
        let stages = [
            Stage { incoming: &[5.0; 20], statuses: &[], hp: 40, mods: Mods::default() },
            Stage { incoming: &summoned, statuses: &[], hp: 60, mods: Mods::default() },
        ];
        let o = outlook(&sim(20.0, 0.0), foe(&stages, None), 80).unwrap();
        assert_eq!(race(&o).kill_turn, Some(5));
        assert_eq!(race(&o).hp_loss, 5 + 15 + 15);
    }

    #[test]
    fn status_cards_slow_the_deck_down() {
        // Three unplayable cards a turn into a ten-card deck.
        let statuses = [3.0f32; 20];
        let clean = outlook(&sim(20.0, 0.0), foe(&stage(&[5.0; 20], 120), None), 60).unwrap();
        let junked = [Stage { incoming: &[5.0; 20], statuses: &statuses, hp: 120, mods: Mods::default() }];
        let slowed = outlook(&sim(20.0, 0.0), foe(&junked, None), 60).unwrap();
        assert_eq!(race(&clean).kill_turn, Some(6));
        assert!(race(&slowed).kill_turn.unwrap_or(99) > 8, "{:?}", race(&slowed));
        assert!(race(&slowed).junk > 20.0);
    }

    #[test]
    fn a_blow_after_death_costs_one_more_turn() {
        // Killed on turn 3, it still blasts at the end of turn 4: the fight
        // costs what killing it a turn later would, unless it dies sooner.
        let plain = outlook(&sim(20.0, 0.0), foe(&stage(&[10.0; 20], 50), None), 60).unwrap();
        let blast = outlook(&sim(20.0, 0.0), foe(&stage(&[10.0; 20], 50), Some(25.0)), 60).unwrap();
        assert_eq!(race(&plain).hp_loss, 20);
        assert_eq!(race(&blast).kill_turn, Some(3));
        assert_eq!(race(&blast).blast_turn, Some(4));
        assert_eq!(race(&blast).hp_loss, 45);
    }

    #[test]
    fn a_fast_kill_takes_few_enemy_turns() {
        // 20 a turn into 50 HP: dead on turn 3, after two enemy turns of 10.
        let o = outlook(&sim(20.0, 10.0), foe(&stage(&[10.0; 20], 50), None), 60).unwrap();
        assert_eq!(race(&o).kill_turn, Some(3));
        assert_eq!(race(&o).hp_loss, 20);
        assert_eq!(o.verdict, Verdict::OnTrack);
    }

    #[test]
    fn blocking_helps_against_big_hits() {
        let o = outlook(&sim(10.0, 40.0), foe(&stage(&[20.0; 20], 100), None), 80).unwrap();
        assert!(o.lead.hp_loss < race(&o).hp_loss);
    }

    #[test]
    fn hopeless_fights_are_at_risk() {
        let o = outlook(&sim(1.0, 1.0), foe(&stage(&[30.0; 20], 500), None), 40).unwrap();
        assert_eq!(o.verdict, Verdict::AtRisk);
    }

    fn ruled(incoming: &[f32], hp: u32, mods: Mods) -> [Stage<'_>; 1] {
        [Stage { incoming, statuses: &[], hp, mods }]
    }

    fn hitting(attack: f32, hits: f32) -> DeckSim {
        let mut s = sim(attack, 0.0);
        s.play = PlayStats { hits, per_hit: attack / hits, cards: hits, ..Default::default() };
        s
    }

    #[test]
    fn slippery_spends_the_first_hits() {
        // Slippery 4 against 20 damage a turn: two 10-damage hits lose 18 of
        // it on turn 1 and 18 on turn 2; five 4-damage hits lose 12, once.
        let mods = Mods { slippery: 4.0, ..Mods::default() };
        let few = outlook(&hitting(20.0, 2.0), foe(&ruled(&[0.0; 20], 60, mods), None), 60).unwrap();
        let many = outlook(&hitting(20.0, 5.0), foe(&ruled(&[0.0; 20], 60, mods), None), 60).unwrap();
        assert_eq!(race(&few).kill_turn, Some(5));
        assert_eq!(race(&many).kill_turn, Some(4));
    }

    #[test]
    fn a_cap_per_turn_spreads_the_kill() {
        let mods = Mods { cap: Some(10.0), ..Mods::default() };
        let o = outlook(&sim(40.0, 0.0), foe(&ruled(&[0.0; 20], 50, mods), None), 60).unwrap();
        assert_eq!(race(&o).kill_turn, Some(5));
    }

    #[test]
    fn a_stun_at_its_hp_line_skips_its_next_turn() {
        // 20 a turn into 60 HP: under the 45 line on turn 1, so its first
        // turn deals nothing; it still hits on turn 2.
        let mods = Mods { stun_at: Some(45.0), ..Mods::default() };
        let plain = outlook(&sim(20.0, 0.0), foe(&stage(&[10.0; 20], 60), None), 60).unwrap();
        let stunned = outlook(&sim(20.0, 0.0), foe(&ruled(&[10.0; 20], 60, mods), None), 60).unwrap();
        assert_eq!((race(&plain).hp_loss, race(&stunned).hp_loss), (20, 10));
    }

    #[test]
    fn windows_block_only_the_spikes() {
        // 5 a turn with a 30 every third turn; the deck blocks 30 or deals 20.
        let incoming: Vec<f32> = (0..20).map(|t| if t % 3 == 2 { 30.0 } else { 5.0 }).collect();
        let mut f = foe(&[], None);
        let stages = stage(&incoming, 100);
        f.stages = &stages;
        f.spike_line = Some(20.0);
        let o = outlook(&sim(20.0, 30.0), f, 80).unwrap();
        let windows = o.way("Windows").unwrap();
        assert_eq!(windows.way, Way::Windows(20));
        assert!(windows.hp_loss < race(&o).hp_loss);
        assert!(windows.kill_turn < o.way("Wall").unwrap().kill_turn);
    }

    #[test]
    fn a_clock_leads_with_the_faster_way_when_the_safe_one_is_slow() {
        // Walling loses nothing but kills on turn 7; players take 5.
        let stages = stage(&[6.0; 20], 100);
        let mut f = foe(&stages, None);
        let calm = outlook(&sim(20.0, 30.0), f, 80).unwrap();
        assert_eq!(calm.lead.kill_turn, Some(7));
        assert_eq!(calm.lead.way, Way::Wall);
        f.clocked = true;
        f.pace = Some(5.0);
        let rushed = outlook(&sim(20.0, 30.0), f, 80).unwrap();
        assert_eq!(rushed.lead.way, Way::Race);
        assert_eq!(rushed.lead.kill_turn, Some(5));
    }
}
