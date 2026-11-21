//! Reading encounters as threats, and previewing what the next node can be.

use crate::analysis::deck::{Coverage, DeckProfile, coverage};
use crate::analysis::pattern;
use std::collections::BTreeMap;

use crate::catalog::{Catalog, Encounter, Measure, Monster, RoomType};
use crate::ids::EncounterId;
use crate::playbook::{ActPlan, Playbook, Role};
use crate::run::{NodeKind, RunState};

/// Numbers that describe how an encounter pressures a deck.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ThreatProfile {
    /// Enemies at the start of the fight (largest known lineup).
    pub enemies: u32,
    /// No lineup is known, but the encounter name suggests a group.
    pub group: bool,
    pub hp_min: u32,
    pub hp_max: u32,
    /// Biggest single move across all enemies (damage x hits).
    pub max_hit: u32,
    /// Most damage the enemies can deal in one turn over the turns this
    /// fight usually lasts, following their move patterns.
    pub peak_turn: u32,
    pub buffs: bool,
    pub summons: bool,
    /// Phases a monster revives into, counting the first (0 or 1: none).
    pub phases: u32,
    /// A fight rule that wants many hits or damage spread over turns
    /// (Slippery, Intangible, Skittish, Hardened Shell), as its reason.
    pub chip: Option<String>,
    pub debuffs: bool,
    pub statuses: bool,
}

/// Which ascension values enemies use in this run.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Scaling {
    /// Ascension HP (StS2 A8+).
    pub tough: bool,
    /// Ascension damage (StS2 A9+).
    pub deadly: bool,
}

impl Scaling {
    pub fn for_run(state: &RunState, catalog: &Catalog) -> Scaling {
        Scaling {
            tough: catalog.rules.tough_enemies(state.ascension),
            deadly: catalog.rules.deadly_enemies(state.ascension),
        }
    }
}

/// Turns a fight is read over: as long as it usually lasts (community
/// average, rounded up), else the pattern horizon.
pub fn typical_turns(enc: &Encounter) -> usize {
    enc.stats
        .as_ref()
        .filter(|s| s.avg_turns > 0.0)
        .map_or(pattern::HORIZON, |s| (s.avg_turns.ceil() as usize).clamp(1, pattern::HORIZON))
}

fn lineup_profile(monsters: &[&Monster], scaling: Scaling, vulnerable_pct: u32, turns: usize) -> ThreatProfile {
    let mut t = monsters.iter().fold(ThreatProfile::default(), |mut t, m| {
        // A monster that revives into phases has to be beaten once per phase.
        let (hp_min, hp_max) = m
            .phases
            .iter()
            .map(|p| p.hp_at(scaling.tough))
            .fold(m.hp_at(scaling.tough), |acc, p| (acc.0 + p.0, acc.1 + p.1));
        t.enemies += 1;
        t.hp_min += hp_min;
        t.hp_max += hp_max;
        if !m.phases.is_empty() {
            t.phases = t.phases.max(m.phases.len() as u32 + 1);
        }
        if t.chip.is_none() {
            use crate::analysis::rules::{self, Ask};
            t.chip = rules::of(m)
                .into_iter()
                .find(|r| r.kind.asks().iter().any(|a| matches!(a, Ask::ManyHits | Ask::SpreadDamage)))
                .map(|r| format!("{}: wants many hits", crate::analysis::pattern::power_words(&r.power.power)));
        }
        let biggest = m.moves.iter().map(|mv| mv.total_damage_at(scaling.deadly)).max().unwrap_or(0);
        t.max_hit = t.max_hit.max(biggest);
        t.buffs |= m.moves.iter().any(|mv| mv.intent_has("buff"));
        t.summons |= m.moves.iter().any(|mv| mv.intent_has("summon"));
        t.debuffs |= m.moves.iter().any(|mv| mv.intent_has("debuff"));
        t.statuses |= m.moves.iter().any(|mv| mv.intent_has("status"));
        t
    });
    let plans: Vec<pattern::Plan> =
        monsters.iter().map(|m| pattern::plan(m, scaling.deadly, vulnerable_pct, turns)).collect();
    t.peak_turn = pattern::incoming(&plans, turns).iter().map(|(_, _, most)| *most).max().unwrap_or(0);
    t
}

/// Reads an encounter as numbers. With known lineups, counts every monster
/// instance and spans all variants (HP from the smallest to the largest
/// lineup, the peak turn of any). Without them, falls back to one of each
/// listed type plus the playbook's name heuristic for groups.
pub fn threat_profile(enc: &Encounter, catalog: &Catalog, playbook: &Playbook, scaling: Scaling) -> ThreatProfile {
    let resolve =
        |ids: &'_ [crate::ids::MonsterId]| ids.iter().filter_map(|id| catalog.monsters.get(id)).collect::<Vec<_>>();
    let vulnerable = catalog.rules.vulnerable_pct.unwrap_or(50);
    let turns = typical_turns(enc);
    let mut t = if enc.lineups.is_empty() {
        let mut t = lineup_profile(&resolve(&enc.monsters), scaling, vulnerable, turns);
        t.group = playbook.sounds_like_group(&enc.name);
        t
    } else {
        let profiles: Vec<ThreatProfile> = enc
            .lineups
            .iter()
            .map(|l| {
                lineup_profile(&resolve(&without_replacements(enc, &l.monsters, catalog)), scaling, vulnerable, turns)
            })
            .collect();
        profiles.iter().fold(ThreatProfile { hp_min: u32::MAX, ..Default::default() }, |acc, p| ThreatProfile {
            enemies: acc.enemies.max(p.enemies),
            group: false,
            hp_min: acc.hp_min.min(p.hp_min),
            hp_max: acc.hp_max.max(p.hp_max),
            max_hit: acc.max_hit.max(p.max_hit),
            peak_turn: acc.peak_turn.max(p.peak_turn),
            buffs: acc.buffs || p.buffs,
            summons: acc.summons || p.summons,
            phases: acc.phases.max(p.phases),
            chip: acc.chip.or(p.chip.clone()),
            debuffs: acc.debuffs || p.debuffs,
            statuses: acc.statuses || p.statuses,
        })
    };
    t.summons |= !enc.spawns.is_empty();
    t
}

/// A lineup as it stands on the field: a monster that replaces itself when
/// killed ("a new Axebot is summoned in its place") has its replacements
/// recorded in the fight's monsters too, but they come one at a time, so
/// its copies count as in the lineup where it has the fewest.
fn without_replacements(
    enc: &Encounter,
    monsters: &[crate::ids::MonsterId],
    catalog: &Catalog,
) -> Vec<crate::ids::MonsterId> {
    let replaces_itself = |id: &crate::ids::MonsterId| {
        catalog.monsters.get(id).is_some_and(|m| m.death_summons.is_some()) && enc.spawns.iter().all(|s| s == id)
    };
    let fewest = |id: &crate::ids::MonsterId| {
        enc.lineups.iter().map(|l| l.monsters.iter().filter(|m| *m == id).count()).filter(|n| *n > 0).min().unwrap_or(1)
    };
    let mut out: Vec<crate::ids::MonsterId> = Vec::new();
    for id in monsters {
        let kept = out.iter().filter(|m| *m == id).count();
        if !replaces_itself(id) || kept < fewest(id) {
            out.push(id.clone());
        }
    }
    out
}

/// One monster type in a lineup and what it does each turn.
#[derive(Clone, Debug, PartialEq)]
pub struct MonsterPlan {
    pub monster: crate::ids::MonsterId,
    pub name: String,
    pub count: u32,
    pub plan: pattern::Plan,
    /// It still strikes at the end of the player's turn after it dies.
    pub death_blow: bool,
}

/// The most common lineup's monsters, their turn plans for `turns` turns,
/// and their combined HP (midpoint of the range).
pub fn lineup_plans(enc: &Encounter, catalog: &Catalog, scaling: Scaling, turns: usize) -> (Vec<MonsterPlan>, u32) {
    let vulnerable = catalog.rules.vulnerable_pct.unwrap_or(50);
    let counts: Vec<(&crate::ids::MonsterId, u32)> = match enc.lineups.first() {
        Some(l) => l.counts(),
        None => enc.monsters.iter().map(|m| (m, 1)).collect(),
    };
    let mut hp = 0;
    let plans = counts
        .into_iter()
        .filter_map(|(id, count)| {
            let m = catalog.monsters.get(id)?;
            let (lo, hi) = m.hp_at(scaling.tough);
            hp += (lo + hi) / 2 * count;
            Some(MonsterPlan {
                monster: id.clone(),
                name: m.name.clone(),
                count,
                plan: pattern::plan(m, scaling.deadly, vulnerable, turns),
                death_blow: m.death_blow.is_some(),
            })
        })
        .collect();
    (plans, hp)
}

/// Monsters that fight together: the starting lineup, a phase a monster
/// revives into, or the ones a power summons when the wave before dies.
#[derive(Clone, Debug, PartialEq)]
pub struct Wave {
    pub plans: Vec<MonsterPlan>,
    /// Combined HP (midpoint of the range).
    pub hp: u32,
    /// The power that summoned it (`None` for the starting lineup).
    pub summoned_by: Option<String>,
}

/// The most common lineup, then the phases its monsters revive into, then
/// the monsters its on-death powers summon.
///
/// The data says a power summons and gives its amount; it doesn't say how
/// many monsters that is, so the amount is read as the count:
/// - Summoning others (Phrog Parasite's Infested 4): the encounter's
///   `spawns`, `amount` of them, together in one wave.
/// - Summoning itself ("in its place", Axebot's Stock 2, or no `spawns`):
///   `amount` replacements, one wave each, as each holds one less.
///
/// Summoned monsters start on their `SPAWNED` move when they have one.
pub fn lineup_waves(enc: &Encounter, catalog: &Catalog, scaling: Scaling, turns: usize) -> Vec<Wave> {
    let (plans, hp) = lineup_plans(enc, catalog, scaling, turns);
    let vulnerable = catalog.rules.vulnerable_pct.unwrap_or(50);
    let summoned: Vec<(crate::ids::MonsterId, u32, String, bool)> = plans
        .iter()
        .filter_map(|mp| {
            let s = catalog.monsters.get(&mp.monster)?.death_summons.as_ref()?;
            let amount = s.amount.unwrap_or(1).max(1) as u32;
            let others = enc.spawns.iter().find(|m| **m != mp.monster);
            Some(match others {
                Some(other) => (other.clone(), amount * mp.count, s.power.clone(), false),
                None => (mp.monster.clone(), amount, s.power.clone(), true),
            })
        })
        .collect();
    let wave_of = |id: &crate::ids::MonsterId, count: u32, power: &str| -> Option<Wave> {
        let m = catalog.monsters.get(id)?;
        let (lo, hi) = m.hp_at(scaling.tough);
        let plan = pattern::plan_spawned(m, scaling.deadly, vulnerable, turns);
        let mp =
            MonsterPlan { monster: id.clone(), name: m.name.clone(), count, plan, death_blow: m.death_blow.is_some() };
        Some(Wave { plans: vec![mp], hp: (lo + hi) / 2 * count, summoned_by: Some(power.to_string()) })
    };
    // Phases a monster revives into: one more stage each, from the phase's
    // opening move. (A revive that skips a turn isn't counted.)
    let phases: Vec<Wave> = plans
        .iter()
        .filter_map(|mp| catalog.monsters.get(&mp.monster))
        .flat_map(|m| {
            m.phases.iter().map(move |ph| {
                let (lo, hi) = ph.hp_at(scaling.tough);
                let plan = pattern::plan_from(m, &ph.start, scaling.deadly, vulnerable, turns);
                let mp = MonsterPlan { monster: m.id.clone(), name: m.name.clone(), count: 1, plan, death_blow: false };
                Wave { plans: vec![mp], hp: (lo + hi) / 2, summoned_by: None }
            })
        })
        .collect();
    let mut waves = vec![Wave { plans, hp, summoned_by: None }];
    waves.extend(phases);
    for (id, n, power, in_place) in &summoned {
        let (repeats, count) = if *in_place { (*n, 1) } else { (1, *n) };
        waves.extend((0..repeats).filter_map(|_| wave_of(id, count, power)));
    }
    waves
}

/// Expected Status moves per enemy turn from a lineup's plans, counting
/// copies: each puts unplayable cards into the player's deck. Past what a
/// plan says, its average turn.
pub fn lineup_statuses(plans: &[MonsterPlan], turns: usize) -> Vec<f32> {
    let status = |t: &pattern::Turn| -> f32 {
        let n = t.options.len().max(1) as f32;
        let weighted: f32 = t.options.iter().map(|o| o.chance.unwrap_or(0.0)).sum();
        t.options
            .iter()
            .filter(|o| o.intent.to_ascii_lowercase().contains("status"))
            .map(|o| if weighted > 0.0 { o.chance.unwrap_or(0.0) / weighted } else { 1.0 / n })
            .sum()
    };
    (0..turns)
        .map(|t| {
            plans
                .iter()
                .map(|mp| {
                    let per = match mp.plan.turns.get(t) {
                        Some(turn) => status(turn),
                        None if mp.plan.turns.is_empty() => 0.0,
                        None => mp.plan.turns.iter().map(status).sum::<f32>() / mp.plan.turns.len() as f32,
                    };
                    per * mp.count as f32
                })
                .sum()
        })
        .collect()
}

/// Expected damage per enemy turn from a lineup's plans, counting copies.
pub fn lineup_incoming(plans: &[MonsterPlan], turns: usize) -> Vec<(u32, f32, u32)> {
    let mut total = vec![(0u32, 0f32, 0u32); turns];
    for mp in plans {
        for (t, (lo, ex, hi)) in pattern::incoming(std::slice::from_ref(&mp.plan), turns).into_iter().enumerate() {
            total[t].0 += lo * mp.count;
            total[t].1 += ex * mp.count as f32;
            total[t].2 += hi * mp.count;
        }
    }
    total
}

#[derive(Clone, Debug, PartialEq)]
pub struct Demand {
    pub role: Role,
    pub reason: String,
    /// Measured from community runs (elites and bosses), with its line.
    pub measured: Option<crate::catalog::FightDemand>,
}

/// What an encounter asks of a deck. An elite or boss measured from
/// community runs asks for the roles that decided it there, nothing else;
/// other fights, by the act's thresholds.
pub fn demands(
    t: &ThreatProfile,
    room: RoomType,
    plan: Option<&ActPlan>,
    fight: Option<&crate::catalog::FightDemands>,
) -> Vec<Demand> {
    if let Some(f) = fight.filter(|_| room != RoomType::Monster) {
        return f
            .demands
            .iter()
            .filter_map(|d| match d.measure {
                Measure::Job(role) | Measure::Share(role) => Some(Demand {
                    role,
                    reason: format!("low decks die here {:.0}%, the rest {:.0}%", d.rate * 100.0, d.typical * 100.0),
                    measured: Some(d.clone()),
                }),
                _ => None,
            })
            .collect();
    }
    let mut out = Vec::new();
    let mut push = |role, reason: String| out.push(Demand { role, reason, measured: None });

    if t.enemies >= 2 || t.group || t.summons {
        let why = match (t.summons, t.enemies) {
            (_, n) if n >= 2 => format!("{n} enemies"),
            (true, _) => "adds join mid-fight".to_string(),
            _ => "several enemies".to_string(),
        };
        push(Role::Aoe, why);
    }
    if let Some(plan) = plan {
        if t.peak_turn >= plan.big_hit {
            push(Role::Survival, format!("up to {} damage in one turn", t.peak_turn));
        }
        let long = match room {
            RoomType::Monster => plan.long_fight_hp.monster,
            RoomType::Elite => plan.long_fight_hp.elite,
            RoomType::Boss => plan.long_fight_hp.boss,
        };
        if t.hp_min >= long {
            push(Role::ScalingDamage, format!("{} total HP", t.hp_min));
            push(Role::Mitigation, "long fight".into());
        }
    }
    if t.buffs {
        push(Role::FastDamage, "buffs itself: end it early".into());
    }
    if let Some(why) = &t.chip {
        push(Role::Chip, why.clone());
    }
    out
}

#[derive(Clone, Debug, PartialEq)]
pub struct DemandCheck {
    pub demand: Demand,
    /// Cards with the role.
    pub have: u32,
    pub coverage: Coverage,
    /// A measured demand: the deck's value and its character's line.
    pub against: Option<(f32, f32)>,
}

impl DemandCheck {
    /// Its numbers for a hover: "You 9 Block; under 14 Block, 34% die here,
    /// the rest 15%", or the rule's reason and your card count.
    pub fn detail(&self) -> String {
        match (&self.demand.measured, self.against) {
            (Some(d), Some((value, line))) => format!(
                "You {}; under {}, {:.0}% die here, the rest {:.0}%",
                d.measure.format(value),
                d.measure.format(line),
                d.rate * 100.0,
                d.typical * 100.0
            ),
            (Some(d), None) => format!(
                "Under {}, {:.0}% die here, the rest {:.0}%",
                d.measure.format(d.line),
                d.rate * 100.0,
                d.typical * 100.0
            ),
            (None, _) => format!("{}; you have {}", self.demand.reason, self.have),
        }
    }
}

/// A demand against the deck. A measured one is missing when the deck is
/// under its line: by `values` (the simulated deck), else by the role's
/// share of the deck; a job without `values` falls back to having a card.
pub fn check(
    demand: Demand,
    deck: &DeckProfile,
    values: Option<&BTreeMap<Measure, f32>>,
    character: Option<&crate::ids::CharacterId>,
) -> DemandCheck {
    let have = deck.role(demand.role) + deck.starter_role(demand.role);
    let against = demand.measured.as_ref().and_then(|d| {
        let value = match (values.and_then(|v| v.get(&d.measure)), d.measure) {
            (Some(v), _) => *v,
            (None, Measure::Share(_)) => have as f32 / deck.total.max(1) as f32,
            (None, _) => return None,
        };
        Some((value, d.line_for(character)))
    });
    let coverage = match against {
        Some((value, line)) if value < line => Coverage::Missing,
        Some(_) => Coverage::Covered,
        None => coverage(deck, demand.role),
    };
    DemandCheck { have: deck.role(demand.role), coverage, against, demand }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ThreatPreview {
    pub encounter: EncounterId,
    pub name: String,
    pub room: RoomType,
    pub seen: bool,
    pub profile: ThreatProfile,
    pub checks: Vec<DemandCheck>,
    /// Starting lineups as labels with their share, most common first.
    pub lineups: Vec<(String, f32)>,
    /// Names of monsters that can join mid-fight.
    pub spawns: Vec<String>,
    pub stats: Option<crate::catalog::EncounterStats>,
    /// The most common lineup, turn by turn.
    pub plans: Vec<MonsterPlan>,
}

impl ThreatPreview {
    /// The worst coverage among this fight's demands.
    pub fn worst(&self) -> Option<Coverage> {
        self.checks.iter().map(|c| c.coverage).min()
    }

    /// Its demands checked again against the simulated deck's `values`.
    pub fn rechecked(
        &self,
        deck: &DeckProfile,
        values: &BTreeMap<Measure, f32>,
        character: Option<&crate::ids::CharacterId>,
    ) -> ThreatPreview {
        let checks = self.checks.iter().map(|c| check(c.demand.clone(), deck, Some(values), character)).collect();
        ThreatPreview { checks, ..self.clone() }
    }
}

pub fn preview(
    enc: &Encounter,
    catalog: &Catalog,
    playbook: &Playbook,
    act_index: u8,
    scaling: Scaling,
    deck: &DeckProfile,
    seen: bool,
) -> ThreatPreview {
    let plan = playbook.act(act_index);
    let profile = threat_profile(enc, catalog, playbook, scaling);
    let (plans, _) = lineup_plans(enc, catalog, scaling, pattern::HORIZON);
    let checks = demands(&profile, enc.room, plan, catalog.fight(&enc.id))
        .into_iter()
        .map(|d| check(d, deck, None, None))
        .collect();
    ThreatPreview {
        encounter: enc.id.clone(),
        name: enc.name.clone(),
        room: enc.room,
        seen,
        profile,
        checks,
        lineups: enc.lineups.iter().map(|l| (catalog.lineup_label(l), l.share)).collect(),
        spawns: enc.spawns.iter().map(|m| catalog.monster_name(m)).collect(),
        stats: enc.stats.clone(),
        plans,
    }
}

/// Encounters a node of `kind` could hold in the current act, unseen first.
///
/// Hallway fights come from the weak pool for the act's first
/// `weak_fights` fights (per the playbook), then from the normal pool,
/// leaving out those already fought when the rules say hallway fights
/// don't repeat. Elites leave out the one just fought when the rules say
/// an elite can't come twice in a row. A boss node shows the known boss,
/// or every boss of the act if unset.
pub fn next_threats(
    state: &RunState,
    kind: NodeKind,
    catalog: &Catalog,
    playbook: &Playbook,
    deck: &DeckProfile,
) -> Vec<ThreatPreview> {
    let Some(act) = &state.act else { return Vec::new() };
    let plan = playbook.act(state.act_index);
    let room = match kind {
        NodeKind::Unknown => RoomType::Monster,
        other => match other.room() {
            Some(r) => r,
            None => return Vec::new(),
        },
    };

    let _ = plan;
    let weak_phase = state.hallway_fights_in_act < weak_fights(state, catalog, playbook);
    let pool: Vec<&Encounter> = match room {
        RoomType::Boss => {
            let known: Vec<&Encounter> = state.bosses().filter_map(|b| catalog.encounters.get(b)).collect();
            if known.is_empty() { catalog.encounters_in(act, RoomType::Boss).collect() } else { known }
        }
        RoomType::Elite => {
            let all: Vec<&Encounter> = catalog.encounters_in(act, RoomType::Elite).collect();
            // The elite just fought this act can't come straight back.
            let last = state.encounters_seen.iter().rev().find(|e| all.iter().any(|x| x.id == **e));
            let fresh: Vec<&Encounter> = match last {
                Some(last) if catalog.rules.no_elite_twice_in_a_row => {
                    all.iter().copied().filter(|e| e.id != *last).collect()
                }
                _ => all.clone(),
            };
            if fresh.is_empty() { all } else { fresh }
        }
        RoomType::Monster => {
            let all: Vec<&Encounter> = catalog.encounters_in(act, RoomType::Monster).collect();
            let phase: Vec<&Encounter> = all.iter().copied().filter(|e| e.weak == weak_phase).collect();
            let phase = if phase.is_empty() { all } else { phase };
            // Hallway fights don't repeat within an act: drop those fought.
            let fresh: Vec<&Encounter> = if catalog.rules.unique_hallway_fights {
                phase.iter().copied().filter(|e| !state.encounters_seen.contains(&e.id)).collect()
            } else {
                phase.clone()
            };
            if fresh.is_empty() { phase } else { fresh }
        }
    };

    let mut out: Vec<ThreatPreview> = pool
        .into_iter()
        .map(|e| {
            let seen = state.encounters_seen.contains(&e.id);
            preview(e, catalog, playbook, state.act_index, Scaling::for_run(state, catalog), deck, seen)
        })
        .collect();
    out.sort_by(|a, b| a.seen.cmp(&b.seen).then(a.worst().cmp(&b.worst())).then(a.name.cmp(&b.name)));
    out
}

/// Hallway fights drawn from the weak pool this act: measured from community
/// runs when the data has it, else the playbook's value.
pub fn weak_fights(state: &RunState, catalog: &Catalog, playbook: &Playbook) -> u32 {
    state
        .act
        .as_ref()
        .and_then(|a| catalog.acts.get(a))
        .and_then(|a| a.weak_fights)
        .or_else(|| playbook.act(state.act_index).map(|p| p.weak_fights))
        .unwrap_or(0)
}
