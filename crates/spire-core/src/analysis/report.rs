//! Run report: what a finished (or ongoing) run's history says about how it
//! went, answer first.
//!
//! - **Ending**: how it ended, where, and the HP it went in with.
//! - **Causes**: the likely reasons, ranked: the deck short for the fight
//!   that ended it (the survival outlook with the deck it had then), low HP
//!   going in, fights costing more than the community's, the act's role
//!   targets unmet, gaps against decks that cleared the act, and missed
//!   fixes (an upgrade at a rest site on low HP, gold left unspent,
//!   removable curses kept). Thresholds are the playbook's `report` lines.
//! - **Deciding fight**: the one that ended it, else the one that cost most,
//!   with the deck's outlook there.
//! - **Acts**: a card per act: HP in and out, damage against the community,
//!   elites, deck changes, the act's role targets, gaps against decks that
//!   cleared it, and the outlook against its boss.
//! - **Turning points**: the fights that cost far more than usual, elites,
//!   rest sites (for markers on the HP chart).
//! - **Decisions**: per floor, what was taken and passed over, bought,
//!   removed, upgraded, the rest site's choice, gold spent, potions used.
//!
//! Pure: a few deck simulations per run (one per act and the deciding
//! fight), so compute it once per run shown, not per frame.

use crate::analysis::benchmark::{self, Comparison};
use crate::analysis::deck::run_profile;
use crate::analysis::jobs;
use crate::analysis::sim::simulate;
use crate::analysis::survival::{self, Outlook, Verdict};
use crate::catalog::{Catalog, NodeKind};
use crate::ids::{ActId, CardId, EncounterId, EventId, PotionId, RelicId};
use crate::playbook::{Playbook, Role};
use crate::run::{Change, NodeReport, Outcome, Run, RunEvent, RunState, apply};

/// How the run ended.
#[derive(Clone, Debug, PartialEq)]
pub struct Ending {
    pub outcome: Option<Outcome>,
    pub floor: u32,
    pub act_index: u8,
    /// The last floor's room and fight.
    pub kind: Option<NodeKind>,
    pub encounter: Option<EncounterId>,
    /// HP and max HP going into the last floor.
    pub hp_in: Option<(i32, i32)>,
}

/// A likely reason, short, with its numbers on the side.
#[derive(Clone, Debug, PartialEq)]
pub struct Cause {
    pub text: String,
    pub detail: String,
    /// How much it explains, for ranking.
    pub weight: f32,
}

/// The fight that ended the run (or cost the most), with the deck it had.
#[derive(Clone, Debug, PartialEq)]
pub struct Deciding {
    pub encounter: EncounterId,
    pub floor: u32,
    /// HP and max HP going in.
    pub hp: (i32, i32),
    /// HP it cost (all of it, when it ended the run).
    pub lost: i32,
    pub outlook: Outlook,
    /// The run got past it.
    pub won: bool,
}

/// Damage taken in one room type over an act, next to the community's
/// average for the same fights (only fights the community data knows).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Bleed {
    pub fights: u32,
    pub yours: f32,
    pub typical: f32,
}

impl Bleed {
    pub fn ratio(&self) -> Option<f32> {
        (self.fights > 0 && self.typical > 0.0).then(|| self.yours / self.typical)
    }
}

/// One act at a glance.
#[derive(Clone, Debug, PartialEq)]
pub struct ActCard {
    pub index: u8,
    pub act: Option<ActId>,
    pub hp_in: (i32, i32),
    pub hp_out: (i32, i32),
    /// Hallway, elite and boss fights.
    pub bleed: [Bleed; 3],
    pub elites: u32,
    pub added: u32,
    pub removed: u32,
    pub upgraded: u32,
    /// The act's role targets: the role, cards for it at the act's end, and
    /// the target.
    pub roles: Vec<(Role, u32, u32)>,
    /// Where the deck going into the boss fell short of decks that cleared
    /// the act.
    pub short: Vec<Comparison>,
    /// The boss: the deck's outlook going in, and whether it was beaten.
    pub boss: Option<(EncounterId, Verdict, u32, Option<bool>)>,
}

/// A floor worth a marker on the HP chart.
#[derive(Clone, Debug, PartialEq)]
pub struct Turning {
    pub floor: u32,
    pub what: TurningKind,
}

#[derive(Clone, Debug, PartialEq)]
pub enum TurningKind {
    /// A fight that cost far more than the community's average.
    BigLoss {
        encounter: EncounterId,
        lost: i32,
        typical: f32,
    },
    Elite {
        encounter: Option<EncounterId>,
        lost: i32,
    },
    /// A rest site: the choice and the HP going in.
    Rest {
        choice: String,
        hp: i32,
        max_hp: i32,
    },
}

/// One decision on a floor.
#[derive(Clone, Debug, PartialEq)]
pub enum Decision {
    /// Cards taken (rewards, shop buys, events) and, where the record has
    /// them, the ones offered and passed over.
    Cards {
        taken: Vec<CardId>,
        passed: Vec<CardId>,
    },
    Relics {
        taken: Vec<RelicId>,
        passed: Vec<RelicId>,
    },
    Potions {
        taken: Vec<PotionId>,
        passed: Vec<PotionId>,
    },
    Removed(Vec<CardId>),
    Upgraded(Vec<CardId>),
    Transformed(Vec<(CardId, CardId)>),
    /// A rest site's choice, and the HP going in.
    Rest {
        choice: String,
        hp: i32,
        max_hp: i32,
    },
    /// Gold spent here, and what was left.
    Gold {
        spent: i32,
        left: i32,
    },
    PotionsUsed(Vec<PotionId>),
}

/// A floor's decisions.
#[derive(Clone, Debug, PartialEq)]
pub struct FloorDecisions {
    pub floor: u32,
    pub act_index: u8,
    pub kind: NodeKind,
    pub encounter: Option<EncounterId>,
    pub event: Option<EventId>,
    pub decisions: Vec<Decision>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RunReport {
    pub ending: Ending,
    pub causes: Vec<Cause>,
    pub deciding: Option<Deciding>,
    pub acts: Vec<ActCard>,
    pub turning: Vec<Turning>,
    pub floors: Vec<FloorDecisions>,
}

/// A floor as played: the state going in and coming out.
struct Floor<'a> {
    node: &'a NodeReport,
    before: RunState,
    after: RunState,
}

/// The run's floors, with the state around each, and each act's entry state.
fn replay(run: &Run) -> (Vec<Floor<'_>>, Vec<RunState>) {
    let mut state = RunState::default();
    let mut floors = Vec::new();
    let mut acts = Vec::new();
    for e in &run.events {
        let before = state.clone();
        state = apply(state, e);
        match e {
            RunEvent::Node(node) => floors.push(Floor { node, before, after: state.clone() }),
            RunEvent::EnterAct { .. } => acts.push(state.clone()),
            _ => {}
        }
    }
    (floors, acts)
}

fn typical(catalog: &Catalog, enc: &EncounterId) -> Option<f32> {
    catalog.encounters.get(enc)?.stats.as_ref().map(|s| s.avg_damage_taken)
}

fn outlook(state: &RunState, enc: &EncounterId, catalog: &Catalog, playbook: &Playbook) -> Option<Outlook> {
    let encounter = catalog.encounters.get(enc)?;
    let sim = simulate(state, catalog, &playbook.simulation);
    survival::for_encounter(&sim, encounter, catalog, state, playbook)
}

/// What a floor decided, from its changes and record.
fn decisions(f: &Floor) -> Vec<Decision> {
    let mut out = Vec::new();
    let record = f.node.record.as_deref().cloned().unwrap_or_default();
    let changes = &f.node.changes;
    let added: Vec<CardId> =
        changes.iter().filter_map(|c| if let Change::AddCard(d) = c { Some(d.card.clone()) } else { None }).collect();
    let passed: Vec<CardId> = record.cards.iter().filter(|c| !c.taken).map(|c| c.id.clone()).collect();
    if !added.is_empty() || !passed.is_empty() {
        out.push(Decision::Cards { taken: added, passed });
    }
    let relics: Vec<RelicId> =
        changes.iter().filter_map(|c| if let Change::AddRelic(r) = c { Some(r.clone()) } else { None }).collect();
    let relics_passed: Vec<RelicId> = record.relics.iter().filter(|c| !c.taken).map(|c| c.id.clone()).collect();
    if !relics.is_empty() || !relics_passed.is_empty() {
        out.push(Decision::Relics { taken: relics, passed: relics_passed });
    }
    let potions: Vec<PotionId> =
        changes.iter().filter_map(|c| if let Change::AddPotion(p) = c { Some(p.clone()) } else { None }).collect();
    let potions_passed: Vec<PotionId> = record.potions.iter().filter(|c| !c.taken).map(|c| c.id.clone()).collect();
    if !potions.is_empty() || !potions_passed.is_empty() {
        out.push(Decision::Potions { taken: potions, passed: potions_passed });
    }
    let removed: Vec<CardId> = changes
        .iter()
        .filter_map(|c| if let Change::RemoveCard(d) = c { Some(d.card.clone()) } else { None })
        .collect();
    if !removed.is_empty() {
        out.push(Decision::Removed(removed));
    }
    let upgraded: Vec<CardId> = changes
        .iter()
        .filter_map(|c| if let Change::UpgradeCard(d) = c { Some(d.card.clone()) } else { None })
        .collect();
    if !upgraded.is_empty() {
        out.push(Decision::Upgraded(upgraded));
    }
    let transformed: Vec<(CardId, CardId)> = changes
        .iter()
        .filter_map(|c| {
            if let Change::Transform { from, to } = c { Some((from.card.clone(), to.card.clone())) } else { None }
        })
        .collect();
    if !transformed.is_empty() {
        out.push(Decision::Transformed(transformed));
    }
    for choice in &record.rest {
        out.push(Decision::Rest { choice: choice.clone(), hp: f.before.hp, max_hp: f.before.max_hp });
    }
    let spent = if record.gold_spent > 0 {
        record.gold_spent
    } else if f.node.kind == NodeKind::Shop {
        (f.before.gold - f.after.gold).max(0)
    } else {
        0
    };
    if spent > 0 {
        out.push(Decision::Gold { spent, left: f.after.gold });
    }
    if !record.potions_used.is_empty() {
        out.push(Decision::PotionsUsed(record.potions_used.clone()));
    }
    out
}

/// `last`: the run's last floor number and how the run ended there.
fn act_card(
    index: u8,
    entry: &RunState,
    floors: &[&Floor],
    ended: (u32, Option<&Outcome>),
    catalog: &Catalog,
    playbook: &Playbook,
) -> ActCard {
    let last = floors.last().map_or(entry, |f| &f.after);
    let mut bleed = [Bleed::default(); 3];
    for f in floors {
        let slot = match f.node.kind {
            NodeKind::Monster => 0,
            NodeKind::Elite => 1,
            NodeKind::Boss => 2,
            _ => continue,
        };
        let Some(t) = f.node.encounter.as_ref().and_then(|e| typical(catalog, e)) else { continue };
        let b = &mut bleed[slot];
        b.fights += 1;
        b.yours += (f.before.hp - f.after.hp).max(0) as f32;
        b.typical += t;
    }
    let count =
        |want: fn(&Change) -> bool| floors.iter().flat_map(|f| &f.node.changes).filter(|c| want(c)).count() as u32;
    let profile = run_profile(last, catalog, playbook);
    let roles = playbook
        .act(index)
        .map(|p| p.targets.iter().map(|t| (t.role, profile.role(t.role), t.min)).collect())
        .unwrap_or_default();
    // The deck going into the boss (or at the act's end): its outlook there,
    // and how it compares with decks that cleared the act.
    let boss_floor = floors.iter().find(|f| f.node.kind == NodeKind::Boss);
    let at_boss = boss_floor.map_or(last, |f| &f.before);
    let sim = simulate(at_boss, catalog, &playbook.simulation);
    let short = at_boss
        .character
        .as_ref()
        .and_then(|c| benchmark::for_run(catalog, c, index))
        .map(|b| {
            benchmark::compare(b, &run_profile(at_boss, catalog, playbook), &jobs::measure(&sim))
                .into_iter()
                .filter(|c| c.short)
                .collect()
        })
        .unwrap_or_default();
    let boss = boss_floor.and_then(|f| {
        let enc = f.node.encounter.clone()?;
        let o = survival::for_encounter(&sim, catalog.encounters.get(&enc)?, catalog, &f.before, playbook)?;
        // The run's end is logged after its last floor: a boss on that floor
        // was lost if the run died, unknown if it was abandoned.
        let beaten = match ended {
            (floor, Some(Outcome::Died { .. })) if floor == f.after.floor => Some(false),
            (floor, Some(Outcome::Abandoned)) if floor == f.after.floor => None,
            _ => Some(true),
        };
        Some((enc, o.verdict, o.lead.hp_loss, beaten))
    });
    ActCard {
        index,
        act: entry.act.clone(),
        hp_in: (entry.hp, entry.max_hp),
        hp_out: (last.hp, last.max_hp),
        bleed,
        elites: floors.iter().filter(|f| f.node.kind == NodeKind::Elite).count() as u32,
        added: count(|c| matches!(c, Change::AddCard(_))),
        removed: count(|c| matches!(c, Change::RemoveCard(_))),
        upgraded: count(|c| matches!(c, Change::UpgradeCard(_))),
        roles,
        short,
        boss,
    }
}

/// The run's report.
pub fn report(run: &Run, catalog: &Catalog, playbook: &Playbook) -> RunReport {
    let lines = &playbook.report;
    let (floors, entries) = replay(run);
    let end = run.state();
    let last = floors.last();
    let ending = Ending {
        outcome: end.outcome.clone(),
        floor: end.floor,
        act_index: end.act_index,
        kind: last.map(|f| f.node.kind),
        encounter: last.and_then(|f| f.node.encounter.clone()),
        hp_in: last.map(|f| (f.before.hp, f.before.max_hp)),
    };

    // The fight that ended it, else the one that cost the most.
    let killer = match &end.outcome {
        Some(Outcome::Died { encounter: Some(e) }) => {
            floors.iter().rev().find(|f| f.node.encounter.as_ref() == Some(e))
        }
        _ => None,
    };
    let costliest = || {
        floors
            .iter()
            .filter(|f| f.node.encounter.is_some())
            .max_by_key(|f| f.before.hp - f.after.hp)
            .filter(|f| f.before.hp > f.after.hp)
    };
    let deciding = killer.or_else(costliest).and_then(|f| {
        let enc = f.node.encounter.clone()?;
        let outlook = outlook(&f.before, &enc, catalog, playbook)?;
        let won = killer.is_none_or(|k| !std::ptr::eq(k, f));
        let lost = if won { f.before.hp - f.after.hp } else { f.before.hp };
        Some(Deciding { encounter: enc, floor: f.after.floor, hp: (f.before.hp, f.before.max_hp), lost, outlook, won })
    });

    // Acts: the floors between one act's entry and the next.
    let mut acts = Vec::new();
    for entry in &entries {
        let in_act: Vec<&Floor> =
            floors.iter().filter(|f| f.before.act.is_some() && f.before.act_index == entry.act_index).collect();
        let ended = (last.map_or(0, |f| f.after.floor), end.outcome.as_ref());
        acts.push(act_card(entry.act_index, entry, &in_act, ended, catalog, playbook));
    }

    let mut causes = Vec::new();
    let died = matches!(end.outcome, Some(Outcome::Died { .. }));
    if died {
        if let (Some(d), Some(k)) = (&deciding, killer)
            && d.outlook.verdict != Verdict::OnTrack
        {
            let help: Vec<&str> = d.outlook.limits.iter().filter_map(|l| l.role.map(Role::label)).collect();
            let mut detail = format!(
                "{} way: {} turns",
                d.outlook.lead.way.label(),
                d.outlook.lead.kill_turn.map_or("no win in 20".into(), |t| t.to_string())
            );
            if !help.is_empty() {
                detail.push_str(&format!("; would help: {}", help.join(", ")));
            }
            causes.push(Cause {
                text: format!(
                    "Deck short for {}: about {} HP lost there, with {} HP",
                    catalog.encounter_name(&d.encounter),
                    d.outlook.lead.hp_loss,
                    k.before.hp
                ),
                detail,
                weight: if d.outlook.verdict == Verdict::AtRisk { 3.0 } else { 1.5 },
            });
        }
        if let Some((hp, max)) = ending.hp_in.filter(|(hp, max)| *max > 0 && (*hp as f32 / *max as f32) < lines.low_hp)
        {
            causes.push(Cause {
                text: format!("Went into the last floor at {hp}/{max} HP"),
                detail: format!("Under {:.0}% of max HP.", lines.low_hp * 100.0),
                weight: 2.0 * (1.0 - hp as f32 / max as f32),
            });
        }
    }
    for a in &acts {
        let all = a.bleed.iter().fold(Bleed::default(), |s, b| Bleed {
            fights: s.fights + b.fights,
            yours: s.yours + b.yours,
            typical: s.typical + b.typical,
        });
        if let Some(r) = all.ratio().filter(|r| *r >= lines.bleed && all.yours - all.typical >= lines.bleed_min) {
            causes.push(Cause {
                text: format!("Act {} fights cost {r:.1}× the usual HP", a.index + 1),
                detail: format!(
                    "{:.0} HP over {} fights; the community's average is {:.0}.",
                    all.yours, all.fights, all.typical
                ),
                weight: r - 1.0,
            });
        }
    }
    if let Some(a) = acts.last().filter(|_| died || end.outcome.is_none()) {
        let missing: Vec<String> = a
            .roles
            .iter()
            .filter(|(_, have, min)| have < min)
            .map(|(r, have, min)| format!("{} {have}/{min}", r.label()))
            .collect();
        if !missing.is_empty() {
            causes.push(Cause {
                text: format!("Act {} targets unmet: {}", a.index + 1, missing.join(", ")),
                detail: "Cards added for each role, against the playbook's target for the act.".into(),
                weight: 0.5 * missing.len() as f32,
            });
        }
        if let Some(worst) = a.short.first() {
            let names: Vec<&str> = a.short.iter().take(2).map(|c| c.measure.label()).collect();
            causes.push(Cause {
                text: format!("Behind decks that cleared Act {}: {}", a.index + 1, names.join(", ")),
                detail: "Worse than 3 in 4 community decks that cleared the act, going into its boss.".into(),
                weight: worst.by.min(2.0),
            });
        }
    }
    if died {
        let final_act = end.act_index;
        if let Some((f, hp, max)) = floors.iter().filter(|f| f.before.act_index == final_act).find_map(|f| {
            let smith = f.node.record.as_ref().is_some_and(|r| r.rest.iter().any(|c| c == "SMITH"));
            let frac = f.before.hp as f32 / f.before.max_hp.max(1) as f32;
            (smith && frac < lines.smith_low_hp).then_some((f, f.before.hp, f.before.max_hp))
        }) {
            causes.push(Cause {
                text: format!("Upgraded at a rest site at {hp}/{max} HP (floor {})", f.after.floor),
                detail: "Resting would have healed instead.".into(),
                weight: 0.8,
            });
        }
        if end.gold >= lines.unspent_gold {
            causes.push(Cause {
                text: format!("Ended with {} gold unspent", end.gold),
                detail: "Gold left at the end bought nothing.".into(),
                weight: 0.6,
            });
        }
        let curses = run_profile(&end, catalog, playbook).removable_curses;
        if curses > 0 {
            causes.push(Cause {
                text: format!("{curses} removable curse{} in the deck", if curses == 1 { "" } else { "s" }),
                detail: "A shop or event could have removed it.".into(),
                weight: 0.5,
            });
        }
    }
    causes.sort_by(|a, b| b.weight.total_cmp(&a.weight));
    causes.truncate(lines.causes as usize);

    // Turning points: the costliest fights against the community, elites,
    // rest sites.
    let mut losses: Vec<Turning> = floors
        .iter()
        .filter_map(|f| {
            let enc = f.node.encounter.clone()?;
            let t = typical(catalog, &enc)?;
            let lost = f.before.hp - f.after.hp;
            (lost as f32 - t >= lines.big_loss).then_some(Turning {
                floor: f.after.floor,
                what: TurningKind::BigLoss { encounter: enc, lost, typical: t },
            })
        })
        .collect();
    losses.sort_by(|a, b| {
        let excess = |t: &Turning| match &t.what {
            TurningKind::BigLoss { lost, typical, .. } => *lost as f32 - typical,
            _ => 0.0,
        };
        excess(b).total_cmp(&excess(a))
    });
    losses.truncate(3);
    let mut turning = losses;
    for f in &floors {
        if f.node.kind == NodeKind::Elite && !turning.iter().any(|t| t.floor == f.after.floor) {
            turning.push(Turning {
                floor: f.after.floor,
                what: TurningKind::Elite { encounter: f.node.encounter.clone(), lost: f.before.hp - f.after.hp },
            });
        }
        for choice in f.node.record.iter().flat_map(|r| &r.rest) {
            turning.push(Turning {
                floor: f.after.floor,
                what: TurningKind::Rest { choice: choice.clone(), hp: f.before.hp, max_hp: f.before.max_hp },
            });
        }
    }
    turning.sort_by_key(|t| t.floor);

    let floors_out = floors
        .iter()
        .map(|f| FloorDecisions {
            floor: f.after.floor,
            act_index: f.before.act_index,
            kind: f.node.kind,
            encounter: f.node.encounter.clone(),
            event: f.node.event.clone(),
            decisions: decisions(f),
        })
        .filter(|f| !f.decisions.is_empty())
        .collect();

    RunReport { ending, causes, deciding, acts, turning, floors: floors_out }
}
