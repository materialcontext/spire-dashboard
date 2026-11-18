# When runs end at elites and bosses

Research notes, and what the dashboard does with them. Regular hallway
fights are left out on purpose.

## Data

The source is the Spire Codex run export (`/api/exports/runs`): community-submitted
StS2 runs, one JSON line each. The research sample had 3,163 single-player,
unmodded runs on builds v0.98 to v0.111. That's 9,320 elite fights and
4,961 boss fights.

Each fight room records:

- `turns_taken`
- `damage_taken`
- the HP after the room (`current_hp`)

Each run records `killed_by_encounter` and `win`. Three things follow from
these fields:

- **A death:** the run's last room is a fight, it matches
  `killed_by_encounter`, and the run was lost.
- **Death turn:** the `turns_taken` of that room.
- **Entering HP:** the HP after the previous room.

The app reads the same fields when you update data (`codex_runs.rs`). The
bundled catalog carries these stats for every encounter, read from recent
runs:

- deaths
- deaths by turn
- survivors' damage: median and 90th percentile
- entering HP: median, survivors and deaths

## What the data says

### 1. Deaths cluster on specific turns, and those turns are specific moves

Death turns aren't spread evenly. They spike on the turns where a boss or
elite makes its big move. For enemies with a fixed cycle, the spike lands on
the same move every time it comes round:

| Encounter | Deaths / fights | Peak death turns | Move on those turns |
|---|---|---|---|
| Vantom | 58 / 356 | 7 (45%), 11 (19%) | Dismember (cycle of 4, turn 3 + 4n) |
| Mecha Knight | 29 / 481 | 4, 7 | Heavy Cleave |
| Waterfall Giant | 93 / 395 | 10 (25%), 12 | Pressure Gun (turn 5 + 5n), Stomp |
| Kaiser Crab | 115 / 478 | 4 (32%) | Rocket's Laser |
| Aeonglass | 104 / 310 | 4, 7, 8 | Ebb, then Eye Lasers |
| Terror Eel | 67 / 867 | 3, 5, 7 | Crash (every other turn) |
| Skulking Colony | 58 / 878 | 4 (36%) | Piercing Stabs |
| Infested Prisms | 80 / 937 | 3, 5, 7 | Whirlwind and Jab |
| Knowledge Demon | 94 / 434 | 7 (38%) | (conditional after turn 4) |
| The Insatiable | 82 / 464 | 6, 7 | Thrash, Lunging Bite |

### 2. Elites kill early; bosses kill late

- **Elites:** most deaths come on turns 3 to 5. Decimillipede's median death
  turn is 3.
- **Bosses:** most deaths come on turns 5 to 12. Waterfall Giant and Soul Fysh
  kill later still, at a median of 10 or 11.

So for an elite, "can I survive turns 3 to 5" is the question. For a boss,
it's "will I still be standing when the big move comes round the second time".

### 3. Entering HP decides a lot

| Entering HP (of max) | Elite death rate | Boss death rate |
|---|---|---|
| 10–20% | 48% | 48% |
| 30–40% | 19% | 42% |
| 50–60% | 9% | 28% |
| 70–80% | 4% | 17% |
| 90%+ | 2% | 13% |

- **Elites:** the death rate roughly halves for every 20% more HP.
- **Bosses:** some runs die even from full HP (13%), because the fight is
  longer than any HP buffer. What matters there is whether the deck can
  kill the boss and block its big turns.
- **In every boss and elite,** players who died came in with clearly less
  HP than those who won. At Bygone Effigy, for example, the median was 41%
  for deaths and 67% for wins.

### 4. Survivors' damage gives a yardstick

- **Elites:** survivors lose about a third of their entering HP (median 35%;
  90th percentile 76%).
- **Bosses:** survivors lose about half (median 55%; 90th percentile 94%).

## Brainstorm: showing whether a run is on track

Ideas, roughly from cheapest to richest:

1. **Danger turns.** Mark the turns where runs end, and name the move made on
   each. *(Built.)*
2. **Death rate.** "15% of runs that reach this fight end here." It's a
   prior, before looking at your deck. *(Built.)*
3. **HP yardstick.** Compare your HP with what survivors lost: "survivors
   lost 27 HP (1 in 10 lost 51+); you have 54." *(Built, in the survival
   check's tooltip.)*
4. **Damage race.** Play the fight out on averages:
   - your deck's damage and block per turn, from the simulator
   - the enemy's expected damage per turn, from its pattern, with its
     Strength ramp and the Vulnerable it puts on you

   Try several ways to split each turn (attack all-out; block a share; block
   what's coming; block only the big turns), keep the one that loses the
   least HP, and compare that with your HP. *(Built: the survival check.)*
5. **Kill before the danger turn.** "You'd win on about turn 6, before
   Dismember comes round again on turn 7." This combines ideas 1 and 4.
   *(Partly: both numbers are shown. The sentence isn't built yet.)*
6. **Entering-HP risk curve.** Turn the table in finding 3 into a per-fight
   "your HP gives about an X% death rate". Per-encounter samples are too
   small for this today (20 to 100 deaths each). It needs more runs, or
   pooling by room type. *(Not built.)*
7. **Turn-by-turn HP projection.** A small chart of your HP across the fight
   under the best plan, with the danger turns marked. *(Not built. The data
   is there in `survival::play`.)*

## The survival check

`analysis::survival` produces On track, Tight or At risk:

| Verdict | Meaning |
|---|---|
| **On track** | The best plan wins, losing under 60% of current HP. |
| **Tight** | It wins, losing 60–99%. |
| **At risk** | It loses all your HP first, or doesn't win within 20 turns. |

What it accounts for beyond the lineup's own damage:

- **Status cards.** Each enemy move with a Status intent adds unplayable
  cards (`status_cards` in the playbook's `simulation`, 2 by default: the data
  marks the move, not the count). The deck's damage and block shrink by
  `deck / (deck + added)` from the next turn on.
- **Summons on death.** A power whose text says the monster summons when it
  dies (Infested, Stock) adds a later stage: the encounter's `spawns`, its
  amount of them (Phrog Parasite: 4 Wrigglers), or that many replacements one
  at a time when it summons itself (Axebot). They start on their `SPAWNED`
  move. The amount read as a count is an inference; the tooltip says so.

What it leaves out, so it reads as a guide, not a forecast:

- **Enemies in a stage die together.** Damage from one that falls early still
  counts, and damage past a stage's HP carries into the next.
- **Debuffs.** Weak and Frail on you aren't modelled, and neither are debuffs
  on enemies. The one exception is Vulnerable from the enemy's pattern.
- **Outside the simulator.** Potions, and cards the simulator can't read.
- **Branches.** Random and conditional picks use their expected damage.

The tooltip shows two comparisons: the all-out race, and what survivors
of that fight actually lost.
