//! Reading encounters as threats, and previewing what the next node can be.

use crate::analysis::deck::{Coverage, DeckProfile, coverage};
use crate::catalog::{Catalog, Encounter, RoomType};
use crate::ids::EncounterId;
use crate::playbook::{ActPlan, Capability, Playbook, Role};
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
    /// Sum of each enemy's biggest move: the worst-case turn.
    pub worst_turn: u32,
    pub buffs: bool,
    pub summons: bool,
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

fn lineup_profile<'a>(monsters: impl Iterator<Item = &'a crate::catalog::Monster>, scaling: Scaling) -> ThreatProfile {
    monsters.fold(ThreatProfile::default(), |mut t, m| {
        let (hp_min, hp_max) = m.hp_at(scaling.tough);
        t.enemies += 1;
        t.hp_min += hp_min;
        t.hp_max += hp_max;
        let biggest = m.moves.iter().map(|mv| mv.total_damage_at(scaling.deadly)).max().unwrap_or(0);
        t.max_hit = t.max_hit.max(biggest);
        t.worst_turn += biggest;
        t.buffs |= m.moves.iter().any(|mv| mv.intent_has("buff"));
        t.summons |= m.moves.iter().any(|mv| mv.intent_has("summon"));
        t.debuffs |= m.moves.iter().any(|mv| mv.intent_has("debuff"));
        t.statuses |= m.moves.iter().any(|mv| mv.intent_has("status"));
        t
    })
}

/// Reads an encounter as numbers. With known lineups, counts every monster
/// instance and spans all variants (HP from the smallest to the largest
/// lineup, the worst turn of any). Without them, falls back to one of each
/// listed type plus the playbook's name heuristic for groups.
pub fn threat_profile(enc: &Encounter, catalog: &Catalog, playbook: &Playbook, scaling: Scaling) -> ThreatProfile {
    let resolve =
        |ids: &'_ [crate::ids::MonsterId]| ids.iter().filter_map(|id| catalog.monsters.get(id)).collect::<Vec<_>>();
    let mut t = if enc.lineups.is_empty() {
        let mut t = lineup_profile(resolve(&enc.monsters).into_iter(), scaling);
        t.group = playbook.sounds_like_group(&enc.name);
        t
    } else {
        let profiles: Vec<ThreatProfile> =
            enc.lineups.iter().map(|l| lineup_profile(resolve(&l.monsters).into_iter(), scaling)).collect();
        profiles.iter().fold(ThreatProfile { hp_min: u32::MAX, ..Default::default() }, |acc, p| ThreatProfile {
            enemies: acc.enemies.max(p.enemies),
            group: false,
            hp_min: acc.hp_min.min(p.hp_min),
            hp_max: acc.hp_max.max(p.hp_max),
            max_hit: acc.max_hit.max(p.max_hit),
            worst_turn: acc.worst_turn.max(p.worst_turn),
            buffs: acc.buffs || p.buffs,
            summons: acc.summons || p.summons,
            debuffs: acc.debuffs || p.debuffs,
            statuses: acc.statuses || p.statuses,
        })
    };
    t.summons |= !enc.spawns.is_empty();
    t
}

#[derive(Clone, Debug, PartialEq)]
pub struct Demand {
    pub capability: Capability,
    pub reason: String,
}

/// What an encounter asks of a deck, given the act's thresholds.
pub fn demands(t: &ThreatProfile, room: RoomType, plan: Option<&ActPlan>) -> Vec<Demand> {
    let mut out = Vec::new();
    let mut push = |capability, reason: String| out.push(Demand { capability, reason });

    if t.enemies >= 2 || t.group || t.summons {
        let why = match (t.summons, t.enemies) {
            (_, n) if n >= 2 => format!("{n} enemies"),
            (true, _) => "adds join mid-fight".to_string(),
            _ => "several enemies".to_string(),
        };
        push(Capability::Aoe, why);
    }
    if let Some(plan) = plan {
        if t.worst_turn >= plan.big_hit {
            push(Capability::Role(Role::FrontloadDefense), format!("up to {} damage in one turn", t.worst_turn));
        }
        let long = match room {
            RoomType::Monster => plan.long_fight_hp.monster,
            RoomType::Elite => plan.long_fight_hp.elite,
            RoomType::Boss => plan.long_fight_hp.boss,
        };
        if t.hp_min >= long {
            push(Capability::Role(Role::ScalingDamage), format!("{} total HP", t.hp_min));
            push(Capability::Role(Role::ScalingDefense), "long fight".into());
        }
    }
    if t.buffs {
        push(Capability::Role(Role::FrontloadDamage), "buffs itself: end it early".into());
    }
    if t.statuses {
        push(Capability::Role(Role::Consistency), "adds status cards".into());
    }
    out
}

#[derive(Clone, Debug, PartialEq)]
pub struct DemandCheck {
    pub demand: Demand,
    pub have: u32,
    pub coverage: Coverage,
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
}

impl ThreatPreview {
    /// The worst coverage among this fight's demands.
    pub fn worst(&self) -> Option<Coverage> {
        self.checks.iter().map(|c| c.coverage).min()
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
    let checks = demands(&profile, enc.room, plan)
        .into_iter()
        .map(|d| DemandCheck {
            have: deck.capability(d.capability),
            coverage: coverage(deck, d.capability, plan),
            demand: d,
        })
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
    }
}

/// Encounters a node of `kind` could hold in the current act, unseen first.
///
/// Hallway fights come from the weak pool for the act's first
/// `weak_fights` fights (per the playbook), then from the normal pool.
/// A boss node shows the known boss, or every boss of the act if unset.
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
        RoomType::Elite => catalog.encounters_in(act, RoomType::Elite).collect(),
        RoomType::Monster => {
            let all: Vec<&Encounter> = catalog.encounters_in(act, RoomType::Monster).collect();
            let phase: Vec<&Encounter> = all.iter().copied().filter(|e| e.weak == weak_phase).collect();
            if phase.is_empty() { all } else { phase }
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
