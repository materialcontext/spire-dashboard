# Fight shapes, winning lines and card tags: plan

Status: **proposal**, waiting on the decisions at the end. Nothing here is
built yet.

## The idea

A fight isn't one number. Each enemy's move pattern gives it a **shape**:
steady pressure, big turns at known points, quiet turns, a ramp, or a clock.
The shape decides which **lines** win it:

- **Wall:** enough Block every turn, as against Terror Eel or Bygone Effigy.
- **Race:** beat it down fast and pay in HP. That's how players usually take
  those two.
- **Windows:** block the big turns and hit hard between them, as against
  Entomancer.
- **Scale:** set up, then win late.
- **A mix:** for the multi-phase Act 3 bosses.

A deck either can or can't play each line. The dashboard should say:

- which lines work for your deck;
- which one players usually take;
- what your deck is short of for the line that's closest to working.

**Tags** are how that shortfall turns into a card pick. They say what each
card does: Block, Damage, Weak and so on.

There are three layers:

1. **Fight shape** (enemy side): from move patterns, keeping spikes and
   quiet turns rather than averaging them away.
2. **Deck capabilities** (player side): from the deck simulator. Steady
   block, block on a big turn, steady damage, burst, and ramp.
3. **Lines:** shape against capabilities, giving per line whether it works,
   how many turns, what it costs in HP, and the one number that limits it.

## What the data shows today

From `audit --patterns`: expected damage per enemy turn, turns 1–12, with
the community's average fight.

| Fight | Expected damage, turns 1–12 | Community | Shape |
|---|---|---|---|
| Phantasmal Gardeners | 15 every turn | 5.8 turns, 19 HP | Steady |
| The Decimillipede | 14 every turn | 4.2 turns, 22 HP | Steady |
| Terror Eel | 16 9 16 9 … | 6.7 turns, 20 HP | Alternating |
| Byrdonis | 17 9 17 9 … | 4.4 turns, 19 HP | Alternating |
| Bygone Effigy | 0 0 23 23 23 … | 5.2 turns, 20 HP | Two free turns, then steady |
| Vantom | 7 12 26 0, repeating | 8.5 turns, 30 HP | Spike every 4th turn, then a quiet one |
| The Insatiable | 0 16 28 0 20 20 30 0 24 24 32 0 | 7.4 turns, 29 HP | Spikes and quiet turns, ramping |
| Mecha Knight | 25 8 0 40 13 0 45 18 0 50 … | 5.8 turns, 22 HP | Rising spikes every 3rd turn, a quiet turn after each |
| Knight Gang | 21 26 33 23 58 33 23 58 … | 4.8 turns, 19 HP | Spike of 58 every 3rd turn |
| Kaiser Crab | 15 22 12 33 14 19 26 16 35 … | 7.6 turns, 41 HP | Ramping, uneven |
| Entomancer | **21 every turn (wrong)** | 5.0 turns, 20 HP | Spikes, but the data lacks the order |
| Lagavulin Matriarch | **0 every turn (wrong)** | 9.5 turns, 26 HP | Sleeps, then cycles; the data lacks the wake condition |
| Test Subject | **HP 0 (wrong)** | 9.8 turns, 39 HP | Revives into new phases (Big Pounce 45) |

What this means for the model:

- **Averages hide the shape.** Entomancer picks between Bees (3×7, so 21)
  and Spear (18), buffed by Pheromone Spit. The data doesn't give the order,
  so we average it flat. Random picks get the same treatment everywhere:
  their expected value, not the spike.
- **Some patterns have gaps.**
  - Entomancer's moves have no transitions.
  - Matriarch's wake condition is empty.
  - Test Subject's phases have no HP.
  - These are gaps in the source data, not unknowns in the game.
- **The survival check** already tries several attack/block splits. They're
  close to Wall, Race and Windows, but unnamed, run on averages, and they
  report only the best one.

## Proposal

### 1. Fight shape (`analysis::shape`, core)

Read from each monster's pattern, and from its waves (summons), per enemy
turn:

- **Pressure:** typical incoming damage on an ordinary turn.
- **Spikes:** turns well above the pressure, with size, turn number, and
  chance when the move is a random pick. For random picks, keep the
  *distribution* (each option and its chance), not only the expectation.
- **Windows:** quiet turns (0 or low incoming: sleep, stun, buff turns).
- **Ramp:** how pressure and spikes grow per cycle (Strength gained).
- **Clock:** what punishes slow play:
  - summons;
  - a blast after death;
  - Status cards piling up;
  - a ramp that outgrows Block.
- **Stages and phases:** waves of summons (already built), and revives.

**Gaps.** Where the pattern data is incomplete, the shape says so and falls
back to the move set. Spikes become "any turn can be its biggest move", and
the order is marked unknown. Decision **D7** is about filling those gaps.

### 2. Deck capabilities (`analysis::capability`, core)

Read from the deck simulator's per-turn distributions, which we already
compute (mean, low, high per turn):

| Capability | Means |
|---|---|
| **Steady block** | Block a normal turn reliably gives (low end) |
| **Big-turn block** | Block when the turn is spent on defense |
| **Steady damage** | Damage per turn on average |
| **Burst** | Damage in one all-in turn, and over two turns, including setup cashed in (Vigor, Aim, Strength) |
| **Ramp** | How output grows from turn 1 to turn 5 |

### 3. Lines (`analysis::lines`, core; replaces the survival splits)

Each line is played out against the shape, phase by phase:

| Line | Plays | Fits |
|---|---|---|
| **Wall** | Blocks the pressure every turn, attacks with the rest | Steady and alternating fights |
| **Race** | Attacks every turn and takes the hits | Anything you can kill in time |
| **Windows** | Blocks the spike turns in full, goes all in on the quiet ones | Spike cycles: Vantom, Knight Gang, Mecha Knight, Insatiable, Effigy's sleep |
| **Scale** | Sets up early, then wins late | Long fights that start slowly |

- **Each line reports:**
  - whether it works;
  - turns and HP lost;
  - the number that limits it, e.g. "needs 26 Block on turn 3 (you reach it
    in 40% of draws)" or "needs 60 damage over turns 1–2".
- **Spike turns count bad draws.** Each spike turn is scored with the chance
  your deck blocks enough, not just its average.
- **The view shows:**
  - the line that loses least;
  - any line that's clearly faster;
  - the community's pace (turns and HP lost) as a target.

  For example:

  > Entomancer: **Windows** works — block its big turns (Spear 18, Bees
  > 3×7), hit between. 6 turns, about 24 HP. Players finish in 5 turns,
  > losing 20. Limit: Block on big turns — 21 needed, you reach it in 45%
  > of draws. Look for **Block** or **Weak**.

### 4. Tags (card vocabulary)

These are as in the earlier draft: read from card numbers and the mechanics
reader, with playbook rules as a fallback and overrides kept. A tag carries
its number where it has one.

| Tag | Feeds |
|---|---|
| **Damage** (single target) | Steady damage, Burst |
| **Area** | Damage against groups and summons |
| **Block** | Steady and big-turn block |
| **Weak** (incl. lowering enemy Strength) | Big-turn block, as damage avoided |
| **Vulnerable** | Burst |
| **Draw** | Reliability: the low end of every capability rises |
| **Energy** | Reliability, Burst |
| **Scaling** (damage or block side) | Ramp |
| **Setup** (Aim, Stretch, Vigor) | Burst or big-turn block, once cashed in |
| **Cleanup** (exhausts junk) | Reliability against Status fights |

A line's limit maps to the tags that raise it. "Block on big turns" points
to Block, Weak and Setup (Stretch). "Damage over turns 1–2" points to
Damage, Vulnerable, Energy and Setup (Vigor, Aim).

### 5. Composition and benchmarks

- **The deck panel:**
  - each tag's share of the deck;
  - your capabilities, next to decks of the same character that cleared
    that act's boss;
  - the shortfalls first.
- **Built at data refresh**, from community runs. Each deck is rebuilt at
  each act's end from the final deck (`floor_added_to_deck`) and the
  per-floor removals. Then:
  - each deck is tagged and *simulated*;
  - the percentiles of tag shares and capabilities are stored in the catalog;
  - there's a fallback to all characters below 30 decks.
- **Cost.** About 600 decks, at a few ms each, at refresh time only.

### 6. Setup value

Aim, Stretch and Vigor last until first used, across turns (confirmed).

- **Scoring.** The simulator values setup by its consumers in the deck, not
  only those in hand.
- **Deck view.** It flags setup that piles up: many Aim cards, few Shoot
  cards.
- **Capabilities.** Burst and big-turn block include setup cashed in, which
  is where the value of Aim and Stretch shows.

## Milestones

Each ships as its own PR.

| | What | Done when |
|---|---|---|
| **F1** | Fight shape, from patterns with distributions; gaps marked; `audit --patterns` prints shapes | Entomancer shows spikes and unknown order; Effigy shows 2 windows then steady; Matriarch and Test Subject are marked as gaps |
| **F2** | Deck capabilities from the simulator | Starter decks give sensible numbers; tests on known decks |
| **F3** | Lines (Wall, Race, Windows, Scale), phase by phase, with the limiting number; they replace the survival splits and the outlook text | Terror Eel: Wall works for a block-heavy deck, Race for a damage deck. Entomancer reads Windows. |
| **F4** | Tags (core, then UI), replacing roles; limits mapped to tags | Every class card is tagged or listed in `audit --tags` |
| **F5** | Benchmarks from community runs: tag shares and capabilities of decks that cleared each act; community pace per fight | The catalog carries percentiles with sample sizes; the deck panel shows shortfalls |
| **F6** | Setup value: Aim, Stretch and Vigor scored by their consumers | The Fool's Nock test; the pile-up warning |

The order puts the fight model first, because it decides what the deck is
measured against. Tags come once lines need a vocabulary for "what to add".

## Decisions for you

- **D1 Tags.** The ten above?
- **D2 Roles.** Replace the five roles with tags?
  - **Recommended:** yes.
- **D3 Benchmark decks.** Which decks count as having succeeded?
  - **Recommended:** decks that cleared that act's boss, by character,
    falling back to all characters.
- **D4 Starter cards.** Count Strikes and Defends in the shares?
  - **Recommended:** count them, shown apart.
- **D6 Anything else undervalued?** For example, Arrows or Stars.
- **D7 Pattern gaps.** For Entomancer, Matriarch, Test Subject and any
  others, the options are:
  - **(a)** Use the move set (the spike is the biggest move, order unknown).
    No new data needed.
  - **(b)** While following the game, have the mod also record each enemy's
    intent per turn, and learn the real order from your fights. This
    extends monitored runs into combat, which was out of scope (M3).
  - **(c)** Both: (a) now, (b) later.
  - **Recommended:** (c).
- **D8 Choosing a line.** When several lines work, which does the dashboard
  lead with?
  - **Recommended:** the one losing least HP, unless it takes clearly longer
    than the community's pace in a fight with a clock or a ramp. There,
    speed wins.
