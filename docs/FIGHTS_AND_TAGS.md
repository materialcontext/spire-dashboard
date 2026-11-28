# Fights, roles and tags

The model behind fight analysis and deck composition. Built (F1–F6).

## Three layers

1. **Fight shape and rules** (enemy side), from move patterns and monster powers, keeping spikes and quiet turns rather than averaging them away. A shape asks for roles.
2. **Roles** (player side), each measured as a **job** by the deck simulator, not by counting cards.
3. **The fight played out** against the shape (`analysis::survival`), as long as community wins there take: in how many turns, at what HP cost, the role that limits it; and **deck styles** (`analysis::styles`): which build wins the fight most.

## Fight shape (`analysis::shape`)

Per enemy turn, from each monster's pattern and its summon waves:

| Field | Meaning |
|---|---|
| Pressure | Typical damage on a turn that deals damage |
| Spikes | Turns well above pressure: size, turn, chance (random picks keep their distribution) |
| Quiet turns | 0 or low incoming (sleep, stun, buff) |
| Ramp | Growth per cycle (Strength) |
| Clocks | Summons, death blasts, Status pile-up, a ramp that outgrows Block, revives |
| Stages | Summon waves and revive phases |
| Gaps | Where pattern data is incomplete: the move set stands in, order marked unknown |

Thresholds: the playbook's `fights`. Gaps are researched into `data/patterns.json` (game facts, so a catalog layer), each entry citing its sources and whether each was read directly.

## Fight rules (`analysis::rules`)

Read from monster power text: Slippery, Intangible, Skittish, Hardened Shell (many hits); Personal Hive, Slow (few cards or Skills); Enrage and the like. Each changes the damage that lands in the survival ways (`Mods`) and can demand a role (many hits → Chip). Intangible is advice only: community wins at the Intangible bosses take as long as the check predicts without it, so it doesn't slow the predicted kill. A power a move gives holds only in stages that use the move, and a revived phase holds the powers its source gives it.

## Roles and jobs

| Role | Job (simulated) |
|---|---|
| Mitigation | Block on an ordinary turn (low end) |
| Survival | Block on a turn spent defending (high end) |
| Fast Damage | Damage over turns 1–3 |
| Scaling Damage | Last simulated turn's damage over the first |
| AoE | Share of damage hitting every enemy |
| Chip | Attack hits a turn |

Bad-draw damage is a note, not a role. Roles share no name with a card or relic. Composition (each job and role share on the range of A10 decks that beat the boss, warnings only on measured risks) is in `ARCHITECTURE.md`.

## Tags (`analysis::tags`)

Fourteen, each with its number: Damage, Chip Damage, Multi-Hit, Area, Block, Weak, Vulnerable, Draw, Energy, Scaling and Setup on the damage or Block side, Clean-Up. Read from card numbers, the mechanics reader and `tag_rules`; hand-set and personal tags replace the reading. Roles come from tags (`tag_roles`, `role_rules`). A role shortfall points at the tags to look for.

A Block engine (Feel No Pain, Child of the Stars, Afterimage, Plating) is Scaling on the Block side, valued at its Block on a big turn: per firing times the playbook's `tag_lines.big_turn` (5 events) or `big_turn_cards` (9 cards, for a trigger on every card played). At 9 or more it counts for Survival, since its Block climbs past what a fight's spikes ask.

## Deck styles (`analysis::styles`)

| Style | Strong in |
|---|---|
| Guarded | Mitigation, Survival |
| Burst | Fast Damage, Chip |
| Wide | AoE |
| Build-up | Scaling Damage |

Found by clustering A10 decks' jobs, standardised within character and act. Each elite and boss records each style's win rate, turns and HP lost; the clearly best style shows next to the deck's own. The survival check still plays four internal ways (all-out, block what comes, block the spikes, split) for its turns and HP, up to the fight's real length.

## Decisions

| # | Decision |
|---|---|
| D1 | Tags: the ten, plus Chip Damage and Multi-Hit; "Clean-Up" |
| D2 | Roles as above, each a job; bad draws a note |
| D3 | Benchmarks: A10 decks that beat the act's boss, going into it, per character, all characters as fallback (was: decks that cleared the act) |
| D4 | Starter cards count in shares and are shown apart |
| D5 | Aim and Stretch last until first used, across turns |
| D6 | Arrows return after 3 attack hits (each hit counts); Stars spent against gained |
| D7 | Pattern gaps: move set first, then sourced research into `patterns.json` |
| D8 | Play patterns (Race, Wall…) aren't advice: runs can't show them. Deck styles are: measured per fight |
| D11 | Elite and boss demands measured from community decks (roles whose low quarter dies clearly more); thresholds only for hallway and unmeasured fights. Verdict relative to community decks at the fight |
| D9 | Role names share nothing with cards or relics; jobs are internal |
| D10 | Fight rules from monster powers, played into the survival check |
