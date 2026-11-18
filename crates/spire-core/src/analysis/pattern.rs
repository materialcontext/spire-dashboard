//! What a monster does turn by turn, read from its move state machine.
//!
//! A fixed cycle reads exactly: Ebb, Eye Lasers, Increasing Intensity,
//! repeat. A random pick lists its options with their chances; a
//! conditional pick lists its options with their conditions. Strength the
//! monster gives itself and Vulnerable it puts on the player carry into
//! later turns while the order is known.

use crate::catalog::{Monster, MonsterMove, PatternState, PowerTarget, StateKind};

/// Turns walked by default: past the length of nearly every fight.
pub const HORIZON: usize = 12;

/// One move a monster may make on a turn.
#[derive(Clone, Debug, PartialEq)]
pub struct MoveOption {
    pub name: String,
    pub intent: String,
    pub hits: u32,
    /// Damage of the whole move, with Strength and the player's Vulnerable.
    pub damage: u32,
    pub block: Option<u32>,
    /// Chance of a random pick (0–1).
    pub chance: Option<f32>,
    /// What decides a conditional pick, in plain words.
    pub condition: Option<String>,
    /// Debuffs and statuses it puts on the player, e.g. "Weak 2".
    pub to_player: Vec<String>,
    /// Strength it gains.
    pub strength: i32,
}

/// One enemy turn.
#[derive(Clone, Debug, PartialEq)]
pub struct Turn {
    pub options: Vec<MoveOption>,
    /// The player is Vulnerable from an earlier move this turn.
    pub vulnerable: bool,
}

impl Turn {
    /// `(least, expected, most)` damage this turn.
    pub fn damage(&self) -> (u32, f32, u32) {
        let min = self.options.iter().map(|o| o.damage).min().unwrap_or(0);
        let max = self.options.iter().map(|o| o.damage).max().unwrap_or(0);
        let weighted: f32 = self.options.iter().map(|o| o.chance.unwrap_or(0.0)).sum();
        let expected = if weighted > 0.0 {
            self.options.iter().map(|o| o.damage as f32 * o.chance.unwrap_or(0.0)).sum::<f32>() / weighted
        } else if self.options.is_empty() {
            0.0
        } else {
            self.options.iter().map(|o| o.damage as f32).sum::<f32>() / self.options.len() as f32
        };
        (min, expected, max)
    }

    pub fn known(&self) -> bool {
        self.options.len() == 1
    }
}

/// A monster's turns, as far as the pattern says.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Plan {
    pub turns: Vec<Turn>,
    /// The fixed order starts over at this turn (1-based)...
    pub repeats_from: Option<usize>,
    /// ...after this many turns: one full pass is `turns[..cycle_end]`.
    pub cycle_end: Option<usize>,
    /// The data doesn't say what comes after a point.
    pub open_ended: bool,
    /// From this turn (1-based) the order is lost: each turn lists the moves
    /// the pattern can still reach (every move after a dead end), so the
    /// biggest one stays visible as a possible spike.
    pub order_unknown_from: Option<usize>,
    /// No pattern at all: every turn is any of the monster's moves.
    pub unknown: bool,
}

impl Plan {
    /// Turns worth showing once: one pass of a cycle, else everything known.
    pub fn shown(&self) -> &[Turn] {
        let lost = self.order_unknown_from.map_or(self.turns.len(), |t| t.saturating_sub(1));
        let end = self.cycle_end.unwrap_or(lost).min(lost).min(self.turns.len());
        if self.unknown { &self.turns[..self.turns.len().min(1)] } else { &self.turns[..end] }
    }

    /// Strength gained over one pass of the cycle.
    pub fn strength_per_cycle(&self) -> i32 {
        match self.cycle_end {
            Some(_) => self.shown().iter().filter(|t| t.known()).map(|t| t.options[0].strength).sum(),
            None => 0,
        }
    }
}

/// Where the walk is: one state, or after a pick whose options all lead
/// to the same state.
struct Walker<'a> {
    monster: &'a Monster,
    deadly: bool,
    vulnerable_pct: u32,
    strength: i32,
    vulnerable_left: i32,
}

impl Walker<'_> {
    fn option(&self, mv: &MonsterMove, chance: Option<f32>, condition: Option<String>) -> MoveOption {
        let hits = mv.hits.unwrap_or(1).max(1);
        let per_hit = mv.per_hit_at(self.deadly).map_or(0, |d| (d as i32 + self.strength).max(0) as u32);
        let mut damage = per_hit * hits;
        if self.vulnerable_left > 0 && per_hit > 0 {
            damage = damage * (100 + self.vulnerable_pct) / 100;
        }
        let to_player =
            mv.powers
                .iter()
                .filter(|p| p.target == PowerTarget::Player)
                .map(|p| {
                    if p.amount > 0 { format!("{} {}", power_words(&p.power), p.amount) } else { power_words(&p.power) }
                })
                .collect();
        MoveOption {
            name: mv.name.clone(),
            intent: mv.intent.clone(),
            hits,
            damage,
            block: mv.block,
            chance,
            condition,
            to_player,
            strength: mv.self_strength(),
        }
    }

    /// Applies a known move's lasting effects for the turns after it.
    fn after(&mut self, mv: &MonsterMove) {
        self.vulnerable_left = (self.vulnerable_left - 1).max(0);
        self.strength += mv.self_strength();
        for p in mv.powers.iter().filter(|p| p.target == PowerTarget::Player && p.power == "VULNERABLE") {
            self.vulnerable_left = self.vulnerable_left.max(p.amount);
        }
    }
}

/// "CHAINS_OF_BINDING" reads "Chains of binding".
pub fn power_words(id: &str) -> String {
    let lower = id.replace('_', " ").to_lowercase();
    let mut chars = lower.chars();
    chars.next().map_or(String::new(), |f| f.to_uppercase().chain(chars).collect())
}

/// Walks a monster's pattern for up to `horizon` turns.
pub fn plan(monster: &Monster, deadly: bool, vulnerable_pct: u32, horizon: usize) -> Plan {
    walk(monster, None, deadly, vulnerable_pct, horizon)
}

/// The move id the game gives monsters that join mid-fight: they spend
/// their first turn on it (a Stun) before their usual pattern.
pub const SPAWNED_MOVE: &str = "SPAWNED";

/// Like `plan`, for a monster summoned mid-fight: it starts on its
/// `SPAWNED` move when it has one.
pub fn plan_spawned(monster: &Monster, deadly: bool, vulnerable_pct: u32, horizon: usize) -> Plan {
    let start = monster.move_by_id(SPAWNED_MOVE).map(|m| m.id.as_str());
    walk(monster, start, deadly, vulnerable_pct, horizon)
}

/// Like `plan`, starting from a given move (a phase's opening move).
pub fn plan_from(monster: &Monster, start: &str, deadly: bool, vulnerable_pct: u32, horizon: usize) -> Plan {
    walk(monster, Some(start), deadly, vulnerable_pct, horizon)
}

fn walk(monster: &Monster, start: Option<&str>, deadly: bool, vulnerable_pct: u32, horizon: usize) -> Plan {
    let mut w = Walker { monster, deadly, vulnerable_pct, strength: 0, vulnerable_left: 0 };
    let Some(pattern) = monster.pattern.as_ref().filter(|p| !p.states.is_empty()) else {
        return unknown(&w, horizon);
    };
    let by_id = |id: &str| pattern.states.iter().find(|s| s.id == id);
    let move_state = |move_id: &str| {
        pattern.states.iter().find(|s| s.kind == StateKind::Move && s.move_id.as_deref() == Some(move_id))
    };
    let mut state: Option<&PatternState> = match start.or(pattern.initial.as_deref()) {
        Some(m) => move_state(m),
        None => pattern.states.iter().find(|s| s.kind != StateKind::Move && !s.branches.is_empty()),
    };
    if state.is_none() {
        return unknown(&w, horizon);
    }
    let mut out = Plan::default();
    // Where the order is lost to a pick whose options go different ways.
    let mut diverged: Vec<&str> = Vec::new();
    let mut visited: Vec<(&str, usize)> = Vec::new();
    let mut last_move: Option<&str> = None;
    // Once a pick has happened, the exact order (and so a repeat) is unknown.
    let mut exact = true;
    while out.turns.len() < horizon {
        let Some(s) = state else {
            out.open_ended = true;
            break;
        };
        if exact
            && out.repeats_from.is_none()
            && let Some((_, turn)) = visited.iter().find(|(id, _)| *id == s.id)
        {
            out.repeats_from = Some(turn + 1);
            out.cycle_end = Some(out.turns.len());
        }
        visited.push((s.id.as_str(), out.turns.len()));
        match s.kind {
            StateKind::Move => {
                let Some(mv) = s.move_id.as_deref().and_then(|m| monster.move_by_id(m)) else {
                    out.open_ended = true;
                    break;
                };
                let vulnerable = w.vulnerable_left > 0;
                out.turns.push(Turn { options: vec![w.option(mv, None, None)], vulnerable });
                w.after(mv);
                last_move = Some(mv.id.as_str());
                state = s.next.as_deref().and_then(by_id);
            }
            StateKind::Random | StateKind::Conditional => {
                let random = s.kind == StateKind::Random;
                // An empty pick: the moves whose state returns here.
                let branches: Vec<(String, Option<f32>, Option<String>, bool)> = if s.branches.is_empty() {
                    pattern
                        .states
                        .iter()
                        .filter(|x| x.kind == StateKind::Move && x.next.as_deref() == Some(s.id.as_str()))
                        .filter_map(|x| x.move_id.clone())
                        .map(|m| (m, None, None, false))
                        .collect()
                } else {
                    s.branches.iter().map(|b| (b.move_id.clone(), b.weight, b.condition.clone(), b.no_repeat)).collect()
                };
                let allowed: Vec<_> =
                    branches.iter().filter(|(m, _, _, no_repeat)| !(*no_repeat && last_move == Some(m))).collect();
                let moves: Vec<(&MonsterMove, Option<f32>, Option<String>)> = allowed
                    .iter()
                    .filter_map(|(m, weight, cond, _)| monster.move_by_id(m).map(|mv| (mv, *weight, cond.clone())))
                    .collect();
                if moves.is_empty() {
                    out.open_ended = true;
                    break;
                }
                let total: f32 = moves.iter().map(|(_, wt, _)| wt.unwrap_or(1.0)).sum();
                let options = moves
                    .iter()
                    .map(|(mv, wt, cond)| {
                        let chance = random.then(|| wt.unwrap_or(1.0) / total.max(f32::EPSILON));
                        w.option(mv, chance, cond.clone())
                    })
                    .collect::<Vec<_>>();
                let vulnerable = w.vulnerable_left > 0;
                exact = false;
                if let [only] = moves.as_slice() {
                    w.after(only.0);
                    last_move = Some(only.0.id.as_str());
                } else {
                    w.vulnerable_left = (w.vulnerable_left - 1).max(0);
                    last_move = None;
                }
                out.turns.push(Turn { options, vulnerable });
                // Carry on only if every option leads to the same state.
                let nexts: Vec<Option<&str>> =
                    moves.iter().map(|(mv, _, _)| move_state(&mv.id).and_then(|x| x.next.as_deref())).collect();
                state = match nexts.first() {
                    Some(Some(first)) if nexts.iter().all(|n| *n == Some(*first)) => by_id(first),
                    _ => {
                        diverged = nexts.into_iter().flatten().collect();
                        None
                    }
                };
            }
        }
    }
    if out.open_ended && out.turns.len() < horizon {
        // The move set from here on: what the pattern can still reach from
        // where the order was lost, or every move after a dead end.
        let reach = reachable(pattern, &diverged);
        let moves: Vec<&MonsterMove> = if reach.is_empty() {
            monster.moves.iter().collect()
        } else {
            monster.moves.iter().filter(|m| reach.contains(&m.id.as_str())).collect()
        };
        if !moves.is_empty() {
            out.order_unknown_from = Some(out.turns.len() + 1);
            let options: Vec<MoveOption> = moves.iter().map(|m| w.option(m, None, None)).collect();
            out.turns.resize(horizon, Turn { options, vulnerable: false });
        }
    }
    out
}

/// Move ids reachable from the given states, following moves' next states
/// and picks' branches (an empty pick: the moves that return to it).
fn reachable<'a>(pattern: &'a crate::catalog::AttackPattern, from: &[&'a str]) -> Vec<&'a str> {
    let mut seen: Vec<&str> = Vec::new();
    let mut moves: Vec<&str> = Vec::new();
    let mut queue: Vec<&str> = from.to_vec();
    while let Some(id) = queue.pop() {
        if seen.contains(&id) {
            continue;
        }
        seen.push(id);
        let Some(s) = pattern.states.iter().find(|s| s.id == id) else { continue };
        match s.kind {
            StateKind::Move => {
                moves.extend(s.move_id.as_deref());
                queue.extend(s.next.as_deref());
            }
            _ => {
                let picks = s.branches.iter().filter_map(|b| {
                    pattern
                        .states
                        .iter()
                        .find(|x| x.kind == StateKind::Move && x.move_id.as_deref() == Some(b.move_id.as_str()))
                });
                let returning =
                    pattern.states.iter().filter(|x| x.kind == StateKind::Move && x.next.as_deref() == Some(id));
                queue.extend(picks.chain(returning).map(|x| x.id.as_str()));
            }
        }
    }
    moves
}

fn unknown(w: &Walker, horizon: usize) -> Plan {
    let options: Vec<MoveOption> = w.monster.moves.iter().map(|m| w.option(m, None, None)).collect();
    if options.is_empty() {
        return Plan { unknown: true, ..Plan::default() };
    }
    Plan {
        turns: vec![Turn { options, vulnerable: false }; horizon],
        repeats_from: None,
        cycle_end: None,
        open_ended: false,
        order_unknown_from: None,
        unknown: true,
    }
}

/// Expected damage per enemy turn from several monsters together, and the
/// least and most it can be.
pub fn incoming(plans: &[Plan], horizon: usize) -> Vec<(u32, f32, u32)> {
    (0..horizon)
        .map(|t| {
            plans.iter().fold((0, 0.0, 0), |(lo, ex, hi), p| match p.turns.get(t) {
                Some(turn) => {
                    let (a, b, c) = turn.damage();
                    (lo + a, ex + b, hi + c)
                }
                None => {
                    // Past what the data says: assume its average turn.
                    let n = p.turns.len().max(1) as f32;
                    let avg = p.turns.iter().map(|t| t.damage().1).sum::<f32>() / n;
                    let top = p.turns.iter().map(|t| t.damage().2).max().unwrap_or(0);
                    (lo, ex + avg, hi + top)
                }
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{AttackPattern, MovePower, PatternBranch};

    fn mv(id: &str, dmg: Option<u32>, hits: Option<u32>, powers: Vec<MovePower>) -> MonsterMove {
        MonsterMove {
            id: id.into(),
            name: id.into(),
            intent: if dmg.is_some() { "Attack".into() } else { "Buff".into() },
            damage: dmg,
            hits,
            powers,
            ..Default::default()
        }
    }

    fn st(id: &str, m: &str, next: Option<&str>) -> PatternState {
        PatternState {
            id: id.into(),
            kind: StateKind::Move,
            move_id: Some(m.into()),
            next: next.map(Into::into),
            branches: vec![],
        }
    }

    fn monster(moves: Vec<MonsterMove>, pattern: AttackPattern) -> Monster {
        Monster {
            id: "M".into(),
            name: "M".into(),
            hp_min: 10,
            hp_max: None,
            moves,
            innate_powers: vec![],
            hp_min_asc: None,
            hp_max_asc: None,
            pattern: Some(pattern),
            death_blow: None,
            death_summons: None,
            powers: Vec::new(),
            phases: Vec::new(),
        }
    }

    fn strength(n: i32) -> MovePower {
        MovePower { power: "STRENGTH".into(), target: PowerTarget::Monster, amount: n }
    }

    #[test]
    fn cycle_repeats_and_strength_builds() {
        let m = monster(
            vec![mv("HIT", Some(10), None, vec![]), mv("BUFF", None, None, vec![strength(3)])],
            AttackPattern {
                initial: Some("HIT".into()),
                states: vec![st("A", "HIT", Some("B")), st("B", "BUFF", Some("A"))],
                description: String::new(),
            },
        );
        let p = plan(&m, false, 50, 5);
        let dmg: Vec<u32> = p.turns.iter().map(|t| t.damage().2).collect();
        assert_eq!(dmg, vec![10, 0, 13, 0, 16]);
        assert_eq!(p.repeats_from, Some(1));
        assert_eq!(p.cycle_end, Some(2));
        assert!(!p.open_ended);
    }

    #[test]
    fn vulnerable_raises_the_next_hits() {
        let vuln = MovePower { power: "VULNERABLE".into(), target: PowerTarget::Player, amount: 1 };
        let m = monster(
            vec![mv("CURSE", None, None, vec![vuln]), mv("HIT", Some(10), Some(2), vec![])],
            AttackPattern {
                initial: Some("CURSE".into()),
                states: vec![st("A", "CURSE", Some("B")), st("B", "HIT", Some("C")), st("C", "HIT", None)],
                description: String::new(),
            },
        );
        let p = plan(&m, false, 50, 5);
        assert_eq!(p.turns[1].options[0].damage, 30);
        assert!(p.turns[1].vulnerable);
        assert_eq!(p.turns[2].options[0].damage, 20);
        assert!(p.open_ended);
    }

    #[test]
    fn random_picks_list_chances_and_skip_repeats() {
        let pick = PatternState {
            id: "RAND".into(),
            kind: StateKind::Random,
            move_id: None,
            next: None,
            branches: ["A", "B"]
                .iter()
                .map(|m| PatternBranch {
                    move_id: (*m).into(),
                    weight: Some(1.0),
                    no_repeat: true,
                    max_times: None,
                    condition: None,
                })
                .collect(),
        };
        let m = monster(
            vec![mv("A", Some(5), None, vec![]), mv("B", Some(9), None, vec![])],
            AttackPattern {
                initial: Some("A".into()),
                states: vec![st("SA", "A", Some("RAND")), st("SB", "B", Some("RAND")), pick],
                description: String::new(),
            },
        );
        let p = plan(&m, false, 50, 3);
        assert_eq!(p.turns[0].options[0].name, "A");
        // Turn 2 can't repeat A.
        assert_eq!(p.turns[1].options.len(), 1);
        assert_eq!(p.turns[1].options[0].name, "B");
        assert_eq!(p.turns[2].options.len(), 1);
        assert_eq!(p.turns[2].options[0].name, "A");
    }
}
