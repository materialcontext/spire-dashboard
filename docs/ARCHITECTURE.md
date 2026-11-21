# Architecture

## Crates

```
spire-app ──► spire-data ──► spire-core
    │            (IO)          (pure)
    └──────► spire-platform (OS, native windows)
```

| Crate | Owns | Must not |
|---|---|---|
| `spire-core` | Domain types, run log, all analysis. Depends only on `serde`. | Do IO, network, UI |
| `spire-data` | Bundled data, local files, adapters, fetching | Hold strategy or UI |
| `spire-platform` | OS preferences, appearance, menus, native window pieces | Depend on UI crates |
| `spire-app` | Windows, panels, input | Change state outside `SpireApp::apply` |

`unsafe` is forbidden workspace-wide, except `spire-platform`'s `window` and `menu` modules (`#[allow(unsafe_code)]`, each block with `// SAFETY:`).

## Domain

**Catalog** (`catalog.rs`): game facts in a source-agnostic shape. A `DataPack` is the serialized form; a `Catalog` layers packs **bundled → downloaded → curated → user**, later winning by id. The curated layer (`data/patterns.json`) fixes sourced gaps in monster data, only while the gap remains. Measured layers come whole from one pack: `benchmarks` with their `risks` and `fights`, and `card_lines`, from the pack measured on the most decks (later wins ties), so a small download never replaces a larger bundled sample; `card_lift` from the last pack that has it.

**Playbook** (`playbook.rs`, `data/playbook.json`): strategy as data, editable without a rebuild.

| Key | Holds |
|---|---|
| `tag_rules`, `tag_lines` | Conditions over card fields for effects the tag reader can't map |
| `tag_roles`, `role_rules` | Tags (from a number up) or conditions that give each role |
| `role_max_cost`, `role_exclusions` | Mitigation only at cost ≤ 1; starter Strikes fill no role; ≤ 6 damage isn't Fast Damage without Vulnerable |
| `card_tags`, `card_roles` | Per-card overrides; the player's own are saved apart (`my_tags.json`, `my_roles.json`) |
| `acts` | Weak-fight fallback, big-hit and long-fight thresholds |
| `relic_fits`, `event_fits`, `short` | When a relic or event option is worth `?` rooms |
| `composition` | Method settings for ranges, risks and fight demands (ascension, min build, player cap, confidence, min gap) |
| `simulation`, `fights`, `balance`, `picks`, `report` | Simulator assumptions and analysis thresholds |

There are no role targets: ranges and warnings are measured from community runs.

**Run** (`run.rs`): an event log. `RunEvent`: `Start`, `EnterAct`, `SetBoss`, `SetSecondBoss`, `Node(NodeReport)`, `Adjust`, `AncientHeal`, `End`. `RunState` is `events.fold(default, apply)`; undo pops, saving writes the log, replay is deterministic. `Start` stores the resolved starting kit, so old runs replay after data updates. A `NodeReport` may carry a `FloorRecord` (offers and choices, rest choice, potions used, gold spent) for the run report only; `apply` never reads it.

## Analysis (`spire-core/src/analysis`)

Pure functions over (state, catalog, playbook).

| Module | Does |
|---|---|
| `deck` | Cards → roles (Mitigation, Survival, Fast Damage, Scaling Damage, AoE, Chip); `DeckProfile` with starters apart; `coverage` (any card for a role: unmeasured fights) |
| `tags` | 14 tags with numbers, from card numbers, `mechanics` and `tag_rules`; roles come from tags |
| `requirements` | Cards dead without something the run supplies (Growth without Sadness, Shoot without Arrows) |
| `sim` | Deck simulator: seeded fights vs a passive target, a damage and a block policy sharing shuffles. Order of operations in `docs/SIMULATION.md` |
| `jobs` | Each role's job from the sim (Block ordinary/defending turn, damage turns 1–3, growth, AoE share, hits a turn) |
| `composition` | The deck's `Measure`s (jobs, role shares, damage to Block, size, dead draws) on the range of A10 decks that beat this act's boss, moving from act entry to the boss by floors played. Flags only on a `Risk`. `risks` (data refresh) keeps a side when its boss win-rate gap is ≥ `min_gap`, its player-resampled bootstrap interval stays below zero, and both player halves agree |
| `benchmark` | Weighted quartiles of those decks per character and act; all-character fallback under `min_decks` |
| `balance` | Setup piling up (Aim, Vigor, Stretch) and Arrows or Stars running short |
| `threats` | `ThreatProfile` per encounter; `demands`: an elite or boss in `catalog.fights` asks for its measured roles only (missing when the deck is under its character's line), other fights by the act's thresholds; `next_threats`, turn plans |
| `fight_demands` | Data refresh: per elite and boss, the roles whose bottom quarter (per character, act and kind) dies there clearly more often (the `risks` test); and the survival score's quartiles with each band's death rate |
| `pattern` | A monster's turns from its move state machine; reachable moves once the order is lost |
| `shape` | A fight's shape: pressure, spikes, quiet turns, ramp, clocks, stages, data gaps (`fights` thresholds) |
| `rules` | Fight rules from monster power text (Slippery, Intangible, Hardened Shell…) and what each asks |
| `survival` | Plays a fight out four ways (Race, Wall, Windows, Scale) on the sim's output against expected damage. Lead: least HP lost, unless a clock asks for speed; the way playing to the fight's top measured demand leads when it costs ≤ 10% of HP more. Verdict: the deck's `score` among community decks at the fight (worst quarter At risk, best On track, when clearly different; else Typical), absolute HP rule where unmeasured. `limits` (roles that would help; a measured fight keeps only its measured roles) |
| `floors` | Act length (double-boss extra floor) and which room the next floor must be |
| `healing` | Rest heal (rules percent plus relics) |
| `events` | `lookout`: relics (`relic_fits`) and event options (`event_fits`) a `?` room this act can still give, with odds, how often runs meet each, and costs. Rules share `When` and its test `holds` |
| `lines` | Card lines that beat the boss together (`measure`, data refresh) and card-reward `suggest`ions: lines within reach, then sim-checked fillers |
| `advice` | Removable curses (dashboard); boss gaps against the simulated deck and one line per composition warning (combat stage) |
| `timeline` | The run floor by floor, for charts |
| `report` | History's report: ending, ranked causes, deciding fight, act rows, turning points, per-floor decisions |

`mechanics` (top level) reads power and relic text into triggers and passives, keeping what it can't read for the UI.

**Cost.** `Dashboard::compute` (the Run tab) runs only when the run or data changes. The combat stage (sim, outlooks, demand checks, composition, picks) is rebuilt once the run settles. Analytics build lazily for their tab; History computes a run's timeline and report once on selection.

## App

- **Views** are functions `fn show(ui, &ViewCtx, …, &mut Vec<Action>)`: read immutable data, push `Action`s. `SpireApp::apply` is the only place state changes.
- **Data updates** run on a background thread and report over a channel. Nothing fetches without a button press.
- **Monitored runs**: `monitor.rs` (IO thread: the mod's link file and every profile's run save), `follow.rs` (pure: `read` the game's run, `step` decides advance/adopt/ask/wait), `SpireApp::sync_game` applies. The mod is `mod/SpireDashboardLink` (C#); see `docs/MONITORED_RUN.md`.
- **Style** (`style/`): a `LookDef` per look (palette, type, radii, metrics, panel style, layers, shadows, materials). The System look is built at runtime per platform; `SPIRE_SYSTEM=windows|macos` and `SPIRE_THEME=light|dark` preview it anywhere. `accent` is for interactive or selected items only; `identity` is the character colour. Spacing in `style::space`; motion `FADE` 83 ms, `MOVE` 167 ms. Details in `docs/NATIVE_LOOK.md`.
- **Chrome** (`chrome.rs`): app-drawn title bar, system frame, or macOS unified title bar; Mica on Windows. The System look puts navigation in a top bar and Settings as a page (Windows), or a sidebar, toolbar and Settings window (macOS).
- **Platform** (`spire-platform`): text size, reduced motion, dark mode, accent and contrast colours, transparency, the system font, the macOS menu bar and context menus (`muda`), Mica, the macOS sidebar material, Windows snap layouts (`HTMAXBUTTON`).

## Data format

`DataPack` (format 1): JSON with optional lists (`characters`, `cards`, `relics`, `potions`, `enchantments`, `monsters`, `encounters`, `events`, `acts`, `ascensions`, `orbs`), `rules`, and the measured layers. Shapes are the structs in `catalog.rs`. Unknown enum values deserialize as `Other`; a newer `format` is rejected with a message.

## Game rules

`GameRules` holds the modelled StS2 mechanics: max ascension (10), potion slots and the ascension that removes one (A4), the starting curse (A5), ascension HP (A8) and damage (A9), the double boss (A10, Act 3), Vulnerable %, rest and weary heals, hallway and elite repeat rules, the card Forge makes. The Codex adapter reads them from ascension descriptions and the glossary; a rule whose description disappears is switched off, not guessed. Weak-pool sizes (3/2/2) come from community runs; the playbook's `weak_fights` is a fallback.

## Spire Codex adapter (`spire-data/src/codex.rs`)

- `/api/{acts,characters,cards,relics,potions,enchantments,monsters,encounters,events,ascensions,orbs,powers}?channel=beta`
- `/api/beta/version` (`/api/versions` on stable): version label, optional.
- `/api/runs/metrics/cards?bracket=wr50`: high-skill card lift, optional.
- `/api/glossary`: the card Forge makes, optional.

Beta by default. Wire structs name only used fields, all optional; `null` lists read empty. `strip_markup` removes Godot rich text; class names (`StrikeDefect`) map to ids (`STRIKE_DEFECT`). Event preconditions become `requires` and `act_span`. Keyless use allows 15 calls per minute per endpoint; an update makes one each.

## Community runs (`/api/exports/runs`)

Gzipped JSON lines, one game run file each, read through the shared `run_file.rs`. The cursor runs by **upload** time: a page holds runs played on any build. An update reads `run_pages` pages (default 12) uploaded in the last 14 days. Single-player runs without modifiers only. A failed page stops the read; pages already read are used, and the rest of the update still applies. `refresh-bundle` fails instead.

| Module | Reads |
|---|---|
| `codex_runs.rs` | Exact lineups: anchors (fixed count in ≥ 95% of fights), spawns, variants (≥ 3% share), per-encounter stats. Player-side allies and unknown monsters dropped |
| `benchmarks.rs` | Decks going into each elite and boss (A10, builds ≥ `min_build`, once by `run_hash`, each player capped at `player_cap` runs' worth), simulated once. Boss winners → `benchmarks`; all boss decks → `risks`; elites and bosses → `fights` |
| `codex_events.rs` | Event relics seen given, how often runs meet each event per act, each option's measured `effects` (incl. `uncurse`), `?`-room treasure odds and chest rarities |
| `card_lines.rs` | Card rewards offered per act, for `lines::measure` (every ascension) |

`audit --roles`, `--fights` and `--lookout` rerun these analyses against saved pages.

## Extension points

| Want to… | Do this |
|---|---|
| Add a data source | A `fetch::Source` variant and an adapter returning a `DataPack` |
| Change how cards are read | `tag_rules` / `tag_roles` / `card_tags` (or `role_rules` / `card_roles`); a new `Condition` variant if needed. `audit --tags` lists untagged class cards |
| Add a composition measure | A `Measure` variant, its value in `composition::values`, its range in `Benchmark` |
| Simulate a mechanic | Read its `vars` key in `sim::play_of`, carry it in `sim::Fight`, add a `Resource` to report it |
| Add a run chart | A `FloorPoint` field in `timeline`, a panel in `views/analytics.rs::run_section` |
| Add a threat signal | `ThreatProfile` and `demands()` (hallway and unmeasured fights); thresholds in `ActPlan` |
| Add a run action | A `Change` variant in `apply_change`; UI in `views/log.rs` |
| Add a panel | A file in `views/`, called from `views::root` |
