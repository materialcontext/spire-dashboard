# Deaths at elites and bosses

Research behind the survival check. Hallway fights excluded.

## Data

Spire Codex run export: 3,163 single-player unmodded runs, builds v0.98–v0.111; 9,320 elite and 4,961 boss fights. Per fight room: `turns_taken`, `damage_taken`, `current_hp`; per run: `killed_by_encounter`, `win`.

- **Death:** the run's last room is a fight matching `killed_by_encounter`, run lost.
- **Death turn:** that room's `turns_taken`.
- **Entering HP:** HP after the previous room.

`codex_runs.rs` reads the same fields at data update. The bundle carries, per encounter: deaths, deaths by turn, survivors' damage (median, 90th percentile), entering HP (survivors, deaths).

## Findings

**1. Deaths cluster on specific turns, on specific moves.**

| Encounter | Deaths / fights | Peak death turns | Move |
|---|---|---|---|
| Vantom | 58 / 356 | 7 (45%), 11 (19%) | Dismember (turn 3 + 4n) |
| Mecha Knight | 29 / 481 | 4, 7 | Heavy Cleave |
| Waterfall Giant | 93 / 395 | 10 (25%), 12 | Pressure Gun (5 + 5n), Stomp |
| Kaiser Crab | 115 / 478 | 4 (32%) | Rocket's Laser |
| Aeonglass | 104 / 310 | 4, 7, 8 | Ebb, then Eye Lasers |
| Terror Eel | 67 / 867 | 3, 5, 7 | Crash (every other turn) |
| Skulking Colony | 58 / 878 | 4 (36%) | Piercing Stabs |
| Infested Prisms | 80 / 937 | 3, 5, 7 | Whirlwind, Jab |
| Knowledge Demon | 94 / 434 | 7 (38%) | conditional after turn 4 |
| The Insatiable | 82 / 464 | 6, 7 | Thrash, Lunging Bite |

**2. Elites kill early (turns 3–5), bosses late (5–12).** Decimillipede's median death turn is 3; Waterfall Giant and Soul Fysh, 10–11.

**3. Entering HP decides a lot.**

| Entering HP | Elite death rate | Boss death rate |
|---|---|---|
| 10–20% | 48% | 48% |
| 30–40% | 19% | 42% |
| 50–60% | 9% | 28% |
| 70–80% | 4% | 17% |
| 90%+ | 2% | 13% |

Elite death rate roughly halves per 20% more HP. Bosses kill 13% of full-HP runs: the deck must kill and block the big turns. Everywhere, losers entered with less HP (Bygone Effigy median: 41% deaths, 67% wins).

**4. Survivors' damage is a yardstick.** Elites: median 35% of entering HP (90th percentile 76%). Bosses: 55% (94%).

## Built from this

| Idea | Status |
|---|---|
| Danger turns, named by move | Built |
| Death rate per fight | Built |
| HP against what survivors lost | Built (survival tooltip) |
| Survival check (below) | Built |
| "You'd win on turn 6, before Dismember on 7" | Both numbers shown; sentence not built |
| Entering-HP risk per fight | Not built: 20–100 deaths per encounter is too few |
| Turn-by-turn HP projection | Not built (data in `survival::play`) |

## Survival check (`analysis::survival`)

Plays the fight on the simulator's damage and block against the lineup's expected damage per turn, four ways (**Race**, **Wall**, **Windows** on spike turns, **Scale**), keeping the way that loses least HP.

| Verdict | Best plan |
|---|---|
| On track | Wins, losing < 60% of current HP |
| Tight | Wins, losing 60–99% |
| At risk | Loses all HP first, or no win within 20 turns |

Included: fight rules (`rules` → `Mods`); Status cards thin the deck by `deck / (deck + added)` (`simulation.status_cards`, default 2 per Status move); on-death summons as a later stage (Infested, Stock; Axebot replaces itself one at a time).

Left out: enemies in a stage die together (overflow damage carries on); Weak, Frail and enemy debuffs (except pattern Vulnerable); potions and unread cards; random and conditional picks use expected damage.
