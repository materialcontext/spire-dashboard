# Fight shapes, winning lines and card tags: plan

Status: **decided**. F1 (shapes and rules), F1b (curated gaps), F2 (roles and jobs), F3 (ways to win), F4 (tags), F5 (benchmarks) and F6 (setup value and resource balance) are built. Nothing here
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

**Roles** are what the player sees, on the deck and on each card. They're
the problems a deck solves:

- Mitigation
- Survival
- Fast Damage
- Scaling Damage
- AoE
- Chip

Each role is backed by a **job**, which the deck simulator measures: how
well the deck actually does that thing. A fight's shape and rules say which
roles it asks for, and lines are made of roles.

**Tags** say what each card does: Block, Damage, Multi-Hit, Weak and so on.
A card's roles come from its tags. A shortfall in a role points at the tags
to look for.

There are three layers:

1. **Fight shape and rules** (enemy side): from move patterns and monster
   powers, keeping spikes and quiet turns rather than averaging them away.
   A shape asks for roles.
2. **Roles** (player side): each measured as a job by the deck simulator,
   not by counting cards.
3. **Lines:** made of roles, played out against the shape. Each line gives
   whether it works, how many turns, what it costs in HP, and the role that
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
- **What was researched** (F1b; the scan in `audit --patterns`). Every
  source below was read directly: the wiki and the Codex pages.

| Monster | Fight | Missing | Filled from |
|---|---|---|---|
| Entomancer | Entomancer | Order of its moves; Pheromone Spit's Strength | Wiki: Beeeees! → Spear! → Pheromone Spit, repeating; Spit gains 1 Strength (2 once Personal Hive is at 3). Codex adds both cases together (+3). |
| Lagavulin Matriarch | Lagavulin Matriarch | When it wakes | Wiki: sleeps 3 turns or until it takes unblocked damage, loses its Plating, then Slash → Disembowel → Slash2 → Soul Siphon. |
| Test Subject | Test Subject | HP, phases, Multi-Claw's hits | Wiki: 100 / 200 / 300 HP (A8+: 111 / 212 / 313); Phase 2's Multi-Claw hits 3 times, plus 1 each use (Codex: 1 hit); each phase loses the last one's powers. |
| Decimillipede segments | The Decimillipede | No pattern | Wiki: each segment cycles Bulk → Writhe → Outgas (Constrict in Codex), starting on different moves. Reattach isn't modelled. |
| Rocket | Kaiser Crab | *(not a gap)* | The dead-end move is never reached; its cycle is complete. |

  Each entry in `data/patterns.json` lists its sources, quotes what they
  say, and marks whether each was read directly. An entry can correct a
  move too (hits, powers) where the data gets it wrong. Where the model
  simplifies, the entry's note says so: Multi-Claw's growing hits, phase
  rules applying to the whole fight, Reattach. 24 normal-fight monsters
  still have gaps.
- **Re-checked at data refresh.** `refresh-bundle` reports when Codex
  starts covering one of them, and the curated entry gives way to Codex.

### 2. Roles and their jobs (`analysis::roles`, core; replace the five roles)

The five roles we have (frontload or scaling damage and defense,
"consistency") count cards by kind. They don't say whether the deck can
*solve* anything.

The new roles are named for the problems fights pose. Each is backed by a
job: what the deck simulator measures, from its per-turn distributions,
draws and energy included.

| Role | Job: the deck can… | Measured as | Asked for by |
|---|---|---|---|
| **Mitigation** | absorb steady pressure, turn after turn | Block on a normal turn (low end), plus Weak's cut | Steady and alternating fights (Gardeners, Terror Eel) |
| **Survival** | take a spike turn | Block when the turn goes all to defense, plus Weak's cut | Spikes (Knight Gang 58, Mecha Knight 40–50, Vantom 26) |
| **Fast Damage** | do most of the work before the fight turns | Damage over the first 3 turns, and in the best single turn | Clocks (ramps, summons, blasts, status piles), early free turns (Effigy asleep), quiet turns between spikes |
| **Scaling Damage** | win a long fight | How damage rises from turn 1 to turn 5 | High HP, bosses, ramping enemies |
| **AoE** | handle a group | Share of damage that hits all enemies | Groups and summons (Phrog's Wrigglers) |
| **Chip** | wear down an enemy that has to be hit many times | Hits landed per turn | Slippery (Vantom), Intangible (Soul Fysh, Test Subject), Skittish (Gardeners), Hardened Shell (Skulking Colony) |
| *(no role)* | keep working on bad draws and junk | How high the low end of the spread is (draw, energy, clean-up, resources) | Status fights; every line, at its limit |

The last row is measured and shown as a note ("your deck stalls on 1 hand in
4"), not as a role: it's how well every role holds up, not a problem of its
own.

Lines are made of roles:

| Line | Roles |
|---|---|
| Wall | Mitigation |
| Race | Fast Damage (Chip, when a fight rule caps damage per hit) |
| Windows | Survival on the spike turns, Fast Damage in the quiet ones |
| Scale | Scaling Damage, with enough Mitigation to get there |

AoE modifies any line against a group. Effigy is a Race: its two sleeping
turns are free damage, so Fast Damage decides it.

**Cards get roles from their tags** (section 4), by rules in the playbook.
Thresholds such as how big a Block counts toward Survival are strategy
beliefs, so they live there too.

| Role | From tags |
|---|---|
| Mitigation | Block, Weak, Scaling (block side) |
| Survival | Block (big for its cost), Weak, Setup (Stretch) |
| Fast Damage | Damage, Vulnerable, Setup (Vigor, Aim), Energy |
| Scaling Damage | Scaling (damage side) |
| AoE | Area |
| Chip | Multi-Hit, Chip Damage |

- **The deck panel** shows each role as its measured job, next to decks
  that cleared the act, plus the share of cards holding each role.
- **The act targets** become benchmarks per role.
- **Each enemy view** shows the roles the fight asks for.
- **Each card** shows its roles and its tags.
- **Personal role overrides stay.** Saved overrides migrate:
  - frontload damage becomes Fast Damage;
  - scaling damage becomes Scaling Damage;
  - frontload and scaling defense become Mitigation;
  - consistency is dropped, since it's now a note, not a role.

### 2b. Fight rules: powers that change what works

Many elites and bosses have a power that changes *how* damage or play
works. Then raw numbers aren't enough: Vantom's Slippery turns its next 2 HP
losses into 1 each, so it wants many hits. These rules are read from the
powers each monster starts with or gains, by their text in the data. Each
asks for a deck property the simulator can count per turn: hits, damage per
hit, cards played, Skills played, debuffs applied, and unblocked damage.

| Rule (power) | Fight | Asks for | Measured as |
|---|---|---|---|
| Slippery | Vantom | Many hits (Chip) | Hits per turn |
| Intangible, Nemesis | Soul Fysh, Test Subject | Many hits (Chip), or saving damage for other turns | Hits per turn |
| Skittish | Phantasmal Gardeners | Many hits (Chip): the first one only opens it up | Hits per turn |
| Personal Hive | Entomancer | Few, big hits, and Clean-Up for the Dazed | Damage per hit; Dazed added |
| Hardened Shell | Skulking Colony | Damage spread over turns, not one burst (Chip) | Damage past the cap is wasted |
| Slow | Bygone Effigy | Many cards per turn | Cards played per turn |
| Enrage | Test Subject | Few Skills | Skills played per turn |
| Painful Stabs | Test Subject | No unblocked attack damage | Turns with unblocked hits |
| Artifact | Aeonglass, Mecha Knight | Debuffs applied more than once | Debuff applications |
| Shriek, Plow | Terror Eel, Ceremonial Beast | Enough damage to cross its HP line | Turns to reach the line |
| Plating | Lagavulin Matriarch | Hits that beat its Block | Damage per hit |
| Hex | Knight Gang | Kill Spectral Knight first (all your cards are Ethereal while it lives) | Damage on one target |

- **How rules enter the lines.**
  - **Damage is adjusted.** Slippery and Intangible turn a big hit into 1,
    Hardened Shell caps a turn, and Slow raises damage per card played.
  - **The fight is charged.** Dazed per hit and Wounds per unblocked hit go
    into the junk count, and Enrage's Strength goes into the enemy's damage.
  - **Each rule shows in the enemy view with its measure:** "Slippery: you
    land 3.2 hits a turn; the first 2 each turn do 1."
- **Rules the reader can't measure are still shown**, with their text,
  just not played out (the same policy as unread power text in the
  simulator).

### 3. Lines (built as the ways in `analysis::survival`; they replace the survival splits)

As built (F3): the four ways are `survival::Way`, played in `survival::play`.
Fight rules change each turn's landed damage from the simulator's
`PlayStats` (`survival::Mods`), in this order: Slow's per-card bonus, then
Slippery spending the first hits, Skittish's Block on the first hit,
Intangible (every other turn) and Hardened Shell's cap. Personal Hive's
Dazed is Ethereal, so it thins only the draws until it is drawn; Painful
Stabs adds a Wound on turns that get through; Enrage adds Strength per Skill
to the enemy's turns; Shriek and Plow skip the enemy turn after the HP line.
Rules about hits on a monster count once per stage (the deck hits one at a
time); rules about what the enemies do add up. The lead way follows D8.
Limits (`survival::Limit`) name the role that would help: Survival when a
certain spike tops a defending turn's Block, Mitigation when the pressure
tops an ordinary turn's, Fast Damage when the kill is over the community's
pace by more than 1.5 turns, Chip when a hit rule meets fewer than 3 hits a
turn, AoE when enemies join and little damage hits them all.

Spike turns count bad draws: the simulator keeps each turn's whole spread
of Block (its value at every 5% of fights, `TurnOutput::block_steps`). A
turn a way spends wholly on Block loses the average shortfall over that
spread, not the shortfall of the average, and each certain spike gets the
share of hands that block it in full (`Outlook::spikes`). The Survival
limit names the spike least often blocked, under half the hands (the
playbook's `fights.spike_odds`).


Each line is played out against the shape, phase by phase:

| Line | Plays | Fits |
|---|---|---|
| **Wall** | Blocks the pressure every turn, attacks with the rest | Steady and alternating fights |
| **Race** | Attacks every turn and takes the hits | Anything you can kill in time |
| **Windows** | Blocks the spike turns in full, goes all in on the quiet ones | Spike cycles: Vantom, Knight Gang, Mecha Knight, Insatiable |
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

As built (F4): `analysis::tags`. Scaling and Setup are split by side
(Scaling (damage), Scaling (block), Setup (attack), Setup (block)), so the
roles can follow them; that makes 14 tags. A tag carries the card's number
(per hit for Multi-Hit, with its hits). The playbook holds the judgement
calls: `tag_rules` (channelling Lightning is Damage, Forge is Damage,
exhausting your hand is Clean-Up…), `tag_lines` (Chip Damage: at most 4 a
hit on a card costing 0–1, as the old Chip rule had it) and `tag_roles`.
The player's own tags (`my_tags.json`) replace the reading, and roles
follow them. Limits and fight rules name the tags that answer them
("Look for: Multi-Hit, Chip Damage"); junk piling past 30% of the deck is a
Clean-Up limit. `audit --tags` lists the 62 class cards no tag describes
(copies, card plays, Stars, random cards).

Since F4, a card costing more than 1 energy (or X) isn't Mitigation,
whatever its tags (the playbook's `role_max_cost`); a big Block card can
still be Survival (from 9 Block, so Leap counts). Intangible and Buffer
are Survival only, at any cost. Starter Strikes fill no role at all, and a
card that deals 6 damage or less, or 6 or less per energy, isn't Fast
Damage unless it applies Vulnerable, whose extra damage is presumed to
make up the difference (Bash); the playbook's `role_exclusions`.

One change from the table below: **Energy gives no role.** Draw and Energy
keep every role working on bad draws, the note that isn't a role (D2), so
Energy counts toward neither Fast Damage nor any other role. The fight
thresholds the limits use (`slower_by`, `chip_hits`, `junk_share`) moved
into the playbook's `fights` section.

Tags say what a card does. They're read from card numbers and the mechanics
reader, with playbook rules as a fallback, and overrides are kept. A tag
carries its number where it has one ("Damage 9", "Multi-Hit 3×4",
"Block 8").

| Tag | Means |
|---|---|
| **Damage** | Deals damage to one enemy |
| **Chip Damage** | Small, cheap damage: low damage for a 0–1 cost card (the line is in the playbook) |
| **Multi-Hit** | Hits more than once (each hit counts for Chip, Arrow returns and on-hit rules) |
| **Area** | Hits every enemy |
| **Block** | Gains Block |
| **Weak** | Cuts what enemies deal (Weak, lowering their Strength) |
| **Vulnerable** | Raises what enemies take |
| **Draw** | Draws cards |
| **Energy** | Gains energy or makes cards cheaper |
| **Scaling** | Grows over the fight (Strength, Dexterity, Focus, Plating, Poison, Doom, powers acting every turn), on the damage or block side |
| **Setup** | Boosts the next card of a kind: Aim (Arrow), Stretch (Block), Vigor (Attack) |
| **Clean-Up** | Gets rid of junk cards (exhausts or removes Status and Curse cards) |

A fight rule maps to what answers it, as roles and tags:

| Rule | Answered by |
|---|---|
| Many hits (Slippery, Intangible, Skittish) | Chip: Multi-Hit, Chip Damage |
| Damage capped per turn (Hardened Shell) | Chip, spread over turns |
| Few big hits (Personal Hive, Plating) | Fast Damage from high damage per hit; Clean-Up for the Dazed |
| Junk (Status moves, Painful Stabs) | Clean-Up, Draw, Energy |

### 4b. Resource balance: Arrows and Stars

As built (F6): `analysis::balance`, from the simulator's per-turn flows
(`DeckSim::stores`: made, used, and cards left waiting in hand). A Shoot
card left in hand with no Arrow, or a Star card with too few Stars, waits;
when 1 in 5 of those wants waits (the playbook's `balance.waiting`), the
deck panel says so with the counts: Shoot cards, Arrow slots, attack hits a
turn and the 3-hit return. Stars also warn when they're made and nothing in
the deck spends them.

Some characters have resources that some cards make and others spend. The
deck view checks the balance per turn and counts it in the bad-draws note:

- **Arrows (the Fool).** Each Shoot needs an Arrow. A shot Arrow sticks in
  the enemy until 3 more attack hits land (each hit of a multi-hit attack
  counts), or a card pulls it out. *(3 hits: confirmed by a player.)*
  - **The balance:** Shoot cards played per turn, against Arrow slots plus
    returns. Returns are attack hits per turn ÷ 3, plus pull-out cards.
  - **The warning:** "5 Shoot cards, 2 slots, 2.1 hits a turn: about 1 shot
    in 3 has no Arrow."
  - **Already in the simulator:** each hit counts. This adds the deck-level
    check.
- **Stars (the Regent).** The balance is Stars spent per turn against Stars
  gained. It warns when spenders outrun generators, or when generators have
  nothing to spend on.

### 5. Composition and benchmarks

As built (F5): `analysis::benchmark` and `spire-data/src/benchmarks.rs`.
Decks are rebuilt by replaying each community run's floors onto the
starting deck (the same reader monitored runs use), not from
`floor_added_to_deck`; the deck is taken just before the run enters the
next act, and a won run's last act counts too. Each deck is tagged with the
bundled playbook and simulated, relics included, at every ascension. The
catalog keeps quartiles (25th, median, 75th) of deck size, starter share,
each tag's share and each job, per character and act with 30 or more
decks, and for all characters together. The deck panel lists where the deck
is worse than three in four cleared decks (under the 25th percentile, or
over the 75th for starter cards), worst first, then the rest in one line;
hovering a shortfall names the tags to look for. The bundled numbers come
from the six-page run export saved on 2026-11-17 (3,494 decks, 15 seconds
in a release build); `refresh-bundle` and Update data recompute them. The
Fool had no runs in that export, so it uses every character's decks.

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

As built (F6): the simulator values a point of Aim at the deck's Shoot
cards' mean Aim multiplier, Vigor at 1 when the deck has an attack, and
Stretch at 1 when it has a Block card; 0 when nothing in the deck uses it,
so Nock sits in hand in a deck with no Shoot card. Setup already waiting
counts toward the card that cashes it in. The deck panel warns when setup
piles up: under half of what's made is used, more than two setup cards
share each card that uses them (it waits turns to be cashed in), or nothing
uses it. The lines are the playbook's (`balance`). Fast Damage and Survival
already include setup cashed in, since they're measured by the simulator.

- **Scoring.** The simulator values setup by its consumers in the deck, not
  only those in hand.
- **Deck view.** It flags setup that piles up: many Aim cards, few Shoot
  cards.
- **Roles.** Fast Damage and Survival include setup cashed in, which is where
  the value of Aim and Stretch shows.

## Milestones

Each ships as its own PR.

| | What | Done when |
|---|---|---|
| **F1** | Fight shape, from patterns with distributions; fight rules read from monster powers; gaps marked; `audit --patterns` prints shapes and rules | Entomancer shows spikes and unknown order; Effigy shows 2 windows then steady; Matriarch and Test Subject are marked as gaps |
| **F1b** | Research the gaps; `data/patterns.json` with sources, as its own data layer | Entomancer, Matriarch, Test Subject, Decimillipede and Rocket read with sourced patterns |
| **F2** | The new roles, measured as jobs by the simulator, in the deck panel, act targets and enemy demands; the simulator also counts hits, damage per hit, cards and Skills played, debuffs applied; Arrow and Star balance; old role overrides migrate | Starter decks give sensible numbers; tests on known decks; a Fool deck heavy on Shoot cards warns |
| **F3** | Lines (Wall, Race, Windows, Scale), phase by phase, with fight rules applied and the limiting job or rule; they replace the survival splits and the outlook text | Terror Eel: Wall works for a block-heavy deck, Race for a damage deck. Effigy reads Race. Vantom's Slippery shows the deck's hits per turn. |
| **F4** | Tags (core, then UI), with personal tag overrides; card roles from tags; limiting roles and rules mapped to tags | Every class card is tagged or listed in `audit --tags` |
| **F5** | Benchmarks from community runs: tag shares and jobs of decks that cleared each act; community pace per fight | The catalog carries percentiles with sample sizes; the deck panel shows shortfalls |
| **F6** | Setup value: Aim, Stretch and Vigor scored by their consumers | The Fool's Nock test; the pile-up warning |

The order puts the fight model first, because it decides what the deck is
measured against. Tags come once lines need a vocabulary for "what to add".

## Decisions

- **D1 Tags:** the ten, plus Chip Damage and Multi-Hit. Cleanup is spelled
  Clean-Up. *Decided.*
- **D2 Roles:** kept for the player and on cards, renamed for the problems
  they solve:
  - Mitigation, Survival, Fast Damage, Scaling Damage, AoE;
  - plus Chip, for enemies that must be hit many times.

  Each is measured as a job. Bad draws is a note, not a role. *Decided.*
- **D3 Benchmark decks:** decks that cleared that act's boss, by character,
  falling back to all characters. *Decided.*
- **D4 Starter cards:** counted in tag shares, and shown apart.
  *Decided.*
- **D5 Aim and Stretch:** last until first used, across turns. *Answered.*
- **D6 Arrows and Stars:** resource balance per turn. Arrows count returns
  from attack hits, at 3 hits each, multi-hits counting each hit. Stars count
  spent against gained. *Decided.*
- **D9 Names:** the roles above, which share no name with a card or relic.
  Jobs are internal. *Decided.*
- **D10 Fight rules:** read from monster powers, and played into the lines.
  *Decided.*
- **D7 Pattern gaps:** use the move set for now, then research each gap
  into a sourced data layer (F1b). *Decided.*
- **D8 Choosing a line:** lead with the one losing least HP, unless it takes
  clearly longer than the community's pace in a fight with a clock or a
  ramp. There, speed wins. *Decided.*
