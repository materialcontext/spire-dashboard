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

Plays the fight on the simulator's damage and block against the lineup's expected damage per turn, up to the turn by which 9 in 10 community wins there are over: A10 boss wins take 7–10 turns (90% by 12), elite wins 4–7 (90% by 8). Four internal ways (all-out attack, block what comes, block the spikes, split); the one losing least HP gives the turns and HP shown.

**Deck styles, not play patterns.** Runs don't log turns, so how a player played can't be checked; how the deck was built can. Clustering 42,600 A10 elite and boss decks by their jobs (standardised within character and act) gives four styles: Guarded (Mitigation, Survival: bosses won 84%, ~9 turns, 29% of max HP), Burst (Fast Damage, Chip: 75%, ~8 turns, 38%), Wide (AoE: 77%), Build-up (Scaling: 78%). They differ most per fight: Kaiser Crab, Guarded 92% against Burst 67%; The Kin, Wide 84%. Each fight shows its clearly best style (the `risks` test) next to the deck's own.

**Calibration.** On 29,700 A10 elite and boss fights the absolute rule (lose < 60% of HP: On track) called 93% of decks At risk; they died 12.8%. Played on averages, the check is too pessimistic, but it ranks decks: predicted to lose under their HP, 2–3% died; predicted no win, 23%. So the verdict is relative: the deck's score (predicted loss ÷ HP, +1 for no win) against community decks at the same fight.

| Verdict | Score among community decks here |
|---|---|
| On track | Best quarter, when it clearly dies less than the middle half |
| Typical | Middle half, or a quarter that doesn't differ clearly |
| At risk | Worst quarter, when it clearly dies more |

The hover gives that band's death rate. Unmeasured fights (hallway) keep the absolute rule: < 60% of HP lost On track, < 100% Typical, else At risk.

Included: fight rules (`rules` → `Mods`); Status cards thin the deck by `deck / (deck + added)` (`simulation.status_cards`, default 2 per Status move); on-death summons as a later stage (Infested, Stock; Axebot replaces itself one at a time).

Left out: enemies in a stage die together (overflow damage carries on); Weak, Frail and enemy debuffs (except pattern Vulnerable); potions and unread cards; random and conditional picks use expected damage.
