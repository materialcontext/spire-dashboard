# Fight shapes, winning lines and card tags: plan

Status: **decided**, except D6. See the decisions at the end. Nothing here
is built yet.

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

**Jobs** replace the five roles. A job is something a deck has to be able to
do in a fight: hold steady pressure, survive a big turn, kill fast, and so
on. Lines are made of jobs, and a fight's shape says which jobs it asks for.

**Tags** are how a shortfall in a job turns into a card pick. They say what
each card does: Block, Damage, Weak and so on.

There are three layers:

1. **Fight shape** (enemy side): from move patterns, keeping spikes and
   quiet turns rather than averaging them away. A shape asks for jobs.
2. **Jobs** (player side): measured by the deck simulator, not by counting
   cards. Each says how well the deck does it.
3. **Lines:** made of jobs, played out against the shape. Each line gives
   whether it works, how many turns, what it costs in HP, and the job that
   limits it.

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
the order is marked unknown (D7).

Then each gap is researched, and the findings ship as a curated data layer.

- **The layer.** It's `data/patterns.json`, read by its own adapter into a
  `DataPack` layer over the Codex one, like every other data source. It's
  game facts, so it goes in the catalog, not the playbook.
- **Sources.** Every entry names its source: a wiki or Codex page, a
  developer note, or a video timestamp. Nothing is taken from memory, and an
  entry without a source isn't added.
- **What's researched today** (the scan in `audit --patterns`):

| Monster | Fight | Missing |
|---|---|---|
| Decimillipede segments (front, middle, back) | The Decimillipede | No pattern at all |
| Entomancer | Entomancer | Order of Bees, Spear and Pheromone Spit |
| Lagavulin Matriarch | Lagavulin Matriarch | When it wakes |
| Test Subject | Test Subject | HP, and its phases |
| Rocket | Kaiser Crab | What follows Precise Smash |

  24 normal-fight monsters have gaps too. They come after the elites and
  bosses.
- **Re-checked at data refresh.** `refresh-bundle` reports when Codex
  starts covering one of them, and the curated entry gives way to Codex.

### 2. Jobs (`analysis::jobs`, core; replace the five roles)

The roles (frontload or scaling damage and defense, "consistency") count
cards by kind. They don't say whether the deck can *solve* anything. Jobs
are what fights ask a deck to do. Each is measured by the deck simulator,
from its per-turn distributions, so it reflects what the deck actually puts
out, draws and energy included.

| Job | The deck can… | Measured as | Asked for by |
|---|---|---|---|
| **Hold** | absorb steady pressure, turn after turn | Block on a normal turn (low end) | Steady and alternating fights (Gardeners, Terror Eel, Effigy once awake) |
| **Brace** | take a big turn | Block when the turn goes all to defense, plus Weak's cut | Spikes (Knight Gang 58, Mecha Knight 40–50, Vantom 26) |
| **Rush** | kill before the fight turns | Damage over the first 3 turns | Clocks: ramps, summons, blasts, status piles; and the Race line |
| **Burst** | punish a window | Damage in one all-in turn, with setup cashed in | Quiet turns (Effigy asleep, Vantom's 0, Mecha Knight's 0s) |
| **Grow** | win a long fight | How output rises from turn 1 to turn 5 | High HP, bosses, ramping enemies |
| **Sweep** | handle a group | Share of damage that hits every enemy | Groups and summons (Phrog's Wrigglers) |
| **Steady** | keep working on bad draws and junk | How narrow the spread is (draw, energy, cleanup) | Status fights; every line, at its limit |

- **Lines are made of jobs:**
  - Wall = Hold.
  - Race = Rush.
  - Windows = Brace + Burst.
  - Scale = Grow, plus enough Hold to get there.
  - Sweep and Steady modify any line.
- **Everywhere roles appear today, jobs take their place:**
  - **The deck panel** shows how well each job is done, next to decks
    that cleared the act.
  - **The act targets** become benchmarks per job.
  - **Each enemy view** shows the jobs the fight asks for, from its shape,
    instead of "role" demands.
  - **Each card** shows the jobs it helps, by the change it makes to the
    deck's jobs when added (simulated). Its tags show what it does.
  - **Personal role overrides** go away; personal tag overrides replace
    them.

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

| Tag | Means |
|---|---|
| **Damage** | Deals damage to one enemy |
| **Area** | Hits every enemy |
| **Block** | Gains Block |
| **Weak** | Cuts what enemies deal (Weak, lowering their Strength) |
| **Vulnerable** | Raises what enemies take |
| **Draw** | Draws cards |
| **Energy** | Gains energy or makes cards cheaper |
| **Scaling** | Grows over the fight (Strength, Dexterity, Focus, Plating, Poison, Doom, powers acting every turn), on the damage or block side |
| **Setup** | Boosts the next card of a kind: Aim (Arrow), Stretch (Block), Vigor (Attack) |
| **Cleanup** | Exhausts or removes junk (Status, Curse) |

A line's limiting job maps to the tags that raise it:

| Job | Raised by tags |
|---|---|
| Hold | Block, Scaling (block side) |
| Brace | Block, Weak, Setup (Stretch) |
| Rush | Damage, Vulnerable, Energy |
| Burst | Damage, Vulnerable, Setup (Vigor, Aim), Energy |
| Grow | Scaling |
| Sweep | Area |
| Steady | Draw, Energy, Cleanup |

### 5. Composition and benchmarks

- **The deck panel:**
  - each tag's share of the deck;
  - your jobs, next to decks of the same character that cleared
    that act's boss;
  - the shortfalls first.
- **Built at data refresh**, from community runs. Each deck is rebuilt at
  each act's end from the final deck (`floor_added_to_deck`) and the
  per-floor removals. Then:
  - each deck is tagged and *simulated*;
  - the percentiles of tag shares and jobs are stored in the catalog;
  - there's a fallback to all characters below 30 decks.
- **Cost.** About 600 decks, at a few ms each, at refresh time only.

### 6. Setup value

Aim, Stretch and Vigor last until first used, across turns (confirmed).

- **Scoring.** The simulator values setup by its consumers in the deck, not
  only those in hand.
- **Deck view.** It flags setup that piles up: many Aim cards, few Shoot
  cards.
- **Jobs.** Burst and Brace include setup cashed in, which is where the
  value of Aim and Stretch shows.

## Milestones

Each ships as its own PR.

| | What | Done when |
|---|---|---|
| **F1** | Fight shape, from patterns with distributions; gaps marked; `audit --patterns` prints shapes | Entomancer shows spikes and unknown order; Effigy shows 2 windows then steady; Matriarch and Test Subject are marked as gaps |
| **F1b** | Research the gaps; `data/patterns.json` with sources, as its own data layer | Entomancer, Matriarch, Test Subject, Decimillipede and Rocket read with sourced patterns |
| **F2** | Jobs from the simulator, replacing roles in the deck panel, act targets and enemy demands | Starter decks give sensible numbers; tests on known decks |
| **F3** | Lines (Wall, Race, Windows, Scale), phase by phase, with the limiting number; they replace the survival splits and the outlook text | Terror Eel: Wall works for a block-heavy deck, Race for a damage deck. Entomancer reads Windows. |
| **F4** | Tags (core, then UI), with personal tag overrides; limiting jobs mapped to tags; each card's job contributions | Every class card is tagged or listed in `audit --tags` |
| **F5** | Benchmarks from community runs: tag shares and jobs of decks that cleared each act; community pace per fight | The catalog carries percentiles with sample sizes; the deck panel shows shortfalls |
| **F6** | Setup value: Aim, Stretch and Vigor scored by their consumers | The Fool's Nock test; the pile-up warning |

The order puts the fight model first, because it decides what the deck is
measured against. Tags come once lines need a vocabulary for "what to add".

## Decisions

- **D1 Tags:** the ten. *Decided.*
- **D2 Roles:** replaced, by jobs rather than tags, so the deck is described
  by how it solves fights. *Decided; jobs are proposed above.*
- **D3 Benchmark decks:** decks that cleared that act's boss, by character,
  falling back to all characters. *Decided.*
- **D4 Starter cards:** counted in tag shares, and shown apart.
  *Decided.*
- **D5 Aim and Stretch:** last until first used, across turns. *Answered.*
- **D6 Arrows and Stars:** *open*; explained in the chat.
- **D7 Pattern gaps:** use the move set for now, then research each gap
  into a sourced data layer (F1b). *Decided.*
- **D8 Choosing a line:** lead with the one losing least HP, unless it takes
  clearly longer than the community's pace in a fight with a clock or a
  ramp. There, speed wins. *Decided.*
