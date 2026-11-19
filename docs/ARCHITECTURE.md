# Architecture

## Crates

```
spire-app  ──►  spire-data  ──►  spire-core
   (egui)        (IO, adapters)     (pure)
```

Dependencies point one way. `spire-core` depends only on `serde`.

| Crate | Owns | Must not |
|---|---|---|
| `spire-core` | Domain types, the run log, all analysis | Do IO, touch the network, know about egui |
| `spire-data` | Bundled data, local files, Spire Codex adapter, fetching | Contain strategy logic or UI |
| `spire-app` | Windows, panels, input | Change state outside `SpireApp::apply` |

## The three domain ideas

### Catalog: game facts

`catalog.rs`. Cards, characters, relics, potions, enchantments, monsters,
encounters, events and acts, in a source-agnostic shape. A `DataPack` is the
serialized form. A `Catalog` is built by layering packs: **bundled →
downloaded → curated → user**, where later layers override earlier ones by
id. The curated layer (`data/patterns.json`, `spire-data::curated`) holds
sourced fixes for gaps in the monster data, applied only while the gap is
still there.

### Playbook: strategy knowledge, as data

`playbook.rs` plus `data/playbook.json`. It holds:

- Tag rules (`tag_rules`): composable `Condition`s over card fields
  (`{"text_contains": "forge"}`, `{"all": [...]}`…) for effects the tag
  reader can't map by itself, and the chip line (`tag_lines`).
- Tag roles (`tag_roles`): which roles each tag gives, from a number up
  ("Block 10+ is Survival").
- Direct role rules (`role_rules`) for what no tag says (Intangible and
  Buffer are Survival; summons and healing, Mitigation).
- Cost caps per role (`role_max_cost`): Mitigation is for cards costing 1
  or less.
- Per-card tag and role overrides; the player's own overrides of both are
  saved apart (`my_tags.json`, `my_roles.json`).
- Event opportunity tags.
- Per-act targets and thresholds.

None of this is a game fact, so it is kept apart from the catalog. It can be
tuned, or replaced by the user, without a rebuild.

### Run: an event log

`run.rs`. The player reports `RunEvent`s:

- `Start`
- `EnterAct`
- `SetBoss`
- `Node(NodeReport)`
- `Adjust`

`RunState` is `events.fold(default, apply)`. Undo pops the log, saving writes
the log, and replay is deterministic. `Start` carries the resolved starting
kit, so old runs replay even after a data update.

## Analysis

`analysis/` holds pure functions over (state, catalog, playbook):

- `deck`: classify cards into roles (Mitigation, Survival, Fast Damage,
  Scaling Damage, AoE, Chip: the problems a deck solves); `DeckProfile` (by
  type and by role, with starter cards counted separately); `coverage`
  against act targets.
- `jobs`: how well the deck does each role, measured from the simulator's
  spreads (Block on an ordinary and a defending turn, damage over turns
  1–3, growth, area share, hits a turn), plus a bad-draw note.
- `threats`: `ThreatProfile` per encounter (HP, peak turn, buffs, groups),
  the `demands` it places on a deck, `next_threats` for a node type, and the
  most common lineup's turn plans.
- `pattern`: a monster's turns read from its move state machine (cycles,
  weighted random picks, conditional picks), carrying its Strength and the
  player's Vulnerable forward while the order is known. Where the order is
  lost (a dead end, or a pick whose options go different ways), each later
  turn lists the moves the pattern can still reach.
- `shape`: a fight's shape from its lineup's patterns, keeping the spread
  per turn: pressure (the typical turn that deals damage), spikes (certain
  or possible), quiet turns, ramp, clocks (summons, blast, statuses, ramp,
  revives), stages, gaps in the data, and its rules. Thresholds are in the
  playbook's `fights`.
- `rules`: fight rules read from monster powers' text (Slippery, Intangible,
  Hardened Shell, Personal Hive, Slow, Enrage…), each with what it asks of
  the deck (many hits, few big hits, few Skills…).
- `balance`: setup piling up (Aim, Vigor, Stretch made faster than used, or
  never used) and the Fool's Arrows or the Regent's Stars running short, from
  the simulator's per-turn flows; the lines are the playbook's `balance`.
- `survival`: whether a deck is on track against a fight, and which way wins
  it. It plays the fight out on the simulator's average damage and block
  against the lineup's expected damage per turn four ways (Race, Wall,
  Windows on the spike turns, Scale), with the fight rules (`Mods`, from
  `rules` and the simulator's hits, cards and Skills) changing the damage
  that lands. Status cards the enemies add thin the deck's output, and
  monsters an on-death power summons (`threats::lineup_waves`) fight as a
  later stage. The lead way loses least HP unless a clocked fight runs past
  the community's pace; `limits` name the roles that would help.
- `events`: run `needs` (removal, heal, upgrade…), and a `watchlist` of
  act events ranked by how well they meet those needs.
- `advice`: act targets, boss gaps, area damage and removable curses (run-specific only).
- `healing`: the rest heal (rules percent plus "heal an additional N" relics); the Ancient heal and starting HP are `GameRules` methods.
- `tags`: what each card does, in 14 tags (Damage, Chip Damage,
  Multi-Hit, Area, Block, Weak, Vulnerable, Draw, Energy, Scaling and Setup
  on the damage or block side, Clean-Up), each with its number. Read from
  the card's numbers, the mechanics reader and the playbook's `tag_rules`;
  hand-set and personal tags replace the reading. Roles come from tags
  (`Playbook::roles_from`).
- `timeline`: a run floor by floor (HP, gold, roles, dead cards, damage per
  fight against the community average, elites per act), folded from the event
  log, so archived runs work the same as the live one.
- `sim`: the deck simulator. It plays seeded fights against a passive target
  with two policies, one leaning to damage and one to block, that share
  shuffles. The per-card data it reads:
  - `vars` and `upgrade` deltas
  - star costs and orbs channelled
  - the character's energy, orb slots and arrow slots

  It carries Strength, Dexterity, Focus, orbs, Poison, Plating, Vulnerable,
  Aim, Stretch, Vigor, Stars and Arrows between turns. Assumptions the data
  doesn't state (hand limit, Arrow return, orb slot cap) are in the
  playbook's `simulation` block. The results are deterministic. The order of
  operations, with sources, is in `docs/SIMULATION.md`.
- `mechanics` (top level in core): reads power and relic text into timed
  triggers ("at the start of your turn", "whenever you play an Attack",
  "every 3 turns") and passives (extra damage, Vulnerable, free Skills,
  echoes). Sentences it can't read are kept, so the UI can show what isn't
  simulated.

`Dashboard::compute` bundles everything the Run tab needs. The app recomputes
it only when the run or data changes, never per frame. Analytics (`timeline` +
`sim`, a few ms) are built lazily, only while the Analytics tab is open; History
reads archived runs from disk when its tab opens.

## App

The `spire-platform` crate is what the OS says and the native pieces the
app asks of it. It depends on no UI crate, so it can be checked on its
own for each target. It is the one crate allowed `unsafe`, and only in
`window::mac`, `window::win` and the context-menu call in `menu`, the
native plumbing with no safe API. The rest of the workspace forbids it.
The crate provides:

- **Preferences:** text size, reduced motion.
- **Appearance:** dark mode, the accent and its shades, contrast-theme
  colours, transparency, and macOS's semantic colours. It is read at
  startup and again when the window regains focus.
- **The system font file.**
- **The macOS menu bar** and native context menus (`menu.rs`, through
  `muda`).
- **Window pieces** (`window.rs`):
  - Mica on Windows.
  - The macOS sidebar material (an `NSVisualEffectView` placed in the
    window's frame view below egui's view).
  - Snap layouts for the app-drawn Windows title bar: a window subclass
    reports the app's maximise button as `HTMAXBUTTON`.

Light and dark mode also come live from eframe.

The style layer (`style/`) has these parts:

- **Looks as tokens:** a `LookDef` per look holds the palette and typeface,
  and also:
  - corner radii and control metrics
  - the panel style (outlined, Fluent card, macOS grouped)
  - the content layer and control fills
  - shadows, and whether a window material may show
- **The System look** (`style/system.rs`): built at runtime for the
  platform (`Native::Windows` or `Native::Mac`) from its design tokens and
  the OS's appearance. `SPIRE_SYSTEM=windows|macos` previews either look on
  any OS, and `SPIRE_THEME=light|dark` pins its mode. The look in use is a
  thread-local cell. Built System looks are cached and reused.
- **Palette accessors.** `accent` is for interactive or selected items only;
  `identity` is the character colour.
- **Spacing tokens:** `style::space`.
- **Motion constants:** `FADE` 83 ms and `MOVE` 167 ms.
- **Per-look flags:** `caps_headings`, `motion`.

Monitored runs are split three ways:

- `monitor.rs` does the IO. A background thread watches the companion
  mod's link file and every profile's run save, and sends them on when
  they change.
- `follow.rs` decides, as pure functions. `read` turns what arrived into
  the game's run (`spire-data::game_run`) or the reason following stopped;
  `step` says what that does to the app's run: advance, adopt, ask
  (conflict), wait, or nothing.
- `SpireApp::sync_game` applies the step.

The mod itself is in `mod/SpireDashboardLink` (C#); see
`docs/MONITORED_RUN.md`.

`chrome.rs` sets up the window frame. There are three kinds:

- the app-drawn title bar
- the system frame
- macOS's unified title bar with the System look

It also puts Mica behind a transparent window on Windows. The System look
also decides where navigation and Settings go:

- **Windows:** a top navigation bar, and Settings as a page.
- **macOS:** a sidebar and toolbar, and a Settings window.

Views are functions: `fn show(ui, &ViewCtx, …, &mut Vec<Action>)`. They read
immutable data and push `Action`s. `SpireApp::apply` is the only place state
changes. Data updates run on a background thread and report back over a
channel. Nothing fetches without a button press.

## Data format

`DataPack` (format 1) is JSON with optional lists:

```json
{ "format": 1, "source": "…", "game_version": "v0.111.0", "fetched_at": "…",
  "characters": [], "cards": [], "relics": [], "potions": [], "enchantments": [],
  "monsters": [], "encounters": [], "events": [], "acts": [] }
```

The field shapes are the structs in `spire-core/src/catalog.rs`. Unknown enum
values (e.g. a new card type) deserialize as `Other` instead of failing. A pack
with a newer `format` is rejected with a clear message.

## Game rules (StS2)

`GameRules` in the catalog holds the mechanics the dashboard models:

- the maximum ascension (10)
- base potion slots
- the ascensions that remove a slot (A4) and add the starting curse (A5)
- the ascensions where enemies use their ascension HP (A8) and damage (A9)
- the double-boss act (A10, Act 3)

The Codex adapter derives these from the ascension descriptions. A rule
whose description disappears is switched off rather than guessed.
`RunEvent::start` applies the rules once and stores the result, and threat
previews read `Scaling::for_run`.

Weak-pool sizes per act (3/2/2) come from community runs (the most common
count among acts that reached their boss). The playbook's `weak_fights` is
only a fallback.

## Spire Codex adapter

`spire-data/src/codex.rs` maps the API responses into a `DataPack`:

- `/api/{acts,characters,cards,relics,potions,enchantments,monsters,encounters,events,ascensions}?channel=beta`
- `/api/beta/version` (or `/api/versions` on stable), for the game version label

The dashboard follows the **beta** branch by default (`Channel::Beta`, which
sends `?channel=beta`). Wire structs name only the fields used, every field is
optional, and `null` lists read as empty.

Notes:

- Codex text has Godot rich-text markup. `strip_markup` reduces it to plain
  text.
- Starting decks and relics use class names (`StrikeDefect`), which are mapped
  to ids (`STRIKE_DEFECT`).
- Encounters list each distinct monster type once, not the count. Exact
  lineups come from the run export instead (below). The playbook's
  `multi_enemy_names` is only a fallback for encounters with no run data.
- Keyless use allows 15 requests per minute per endpoint. One update makes one
  call per endpoint.

## Community runs: exact lineups

`spire-data/src/codex_runs.rs` reads `/api/exports/runs`, which returns
gzipped JSON lines with one community-submitted run each. Those are the
game's own run files, so it shares `run_file.rs` with `game_run.rs`: one
description of the format, for finished runs and the live save alike. Every fight records
its `monster_ids`, duplicates included.

The aggregation:

1. Keep single-player runs without custom modifiers.
2. Drop player-side allies. These are monsters that aren't listed for an
   encounter but appear across 5 or more encounters (Necrobinder's Osty,
   pets, relic summons). Drop fights with monsters missing from the catalog
   (mods).
3. Per encounter (at least 20 fights), find the **anchors**: monster types
   at one fixed count in 95% or more of fights.
4. If the anchors alone also occur as a whole fight (5% or more), the
   encounter has one starting lineup and the extras are **spawns** that join
   mid-fight. This also requires a summoner or an extra type that isn't an
   anchor.
5. Otherwise the encounter draws from a random pool. Each lineup with a 3% or
   higher share becomes a **variant**.
6. Average damage taken and turns per encounter go into `stats`.

`spire-data/src/benchmarks.rs` reads the same pages a second way: each run
replays through `game_run::read`, and the deck at the end of every act it
cleared becomes a sample. `analysis::benchmark::measure` tags and simulates
them into `Catalog::benchmarks` (quartiles per character and act, plus all
characters), and `benchmark::compare` lines the run's deck up against them
for the deck panel.

Results land in `Encounter::lineups`, `spawns` and `stats`. The update reads
`run_pages` pages (default 3, about 3,000 runs) from the last 14 days, so
lineups track the current game version. The export's own budget is 120
credits per hour, and one page costs 1 credit. If this step fails, the rest of
the update still applies.

## Extension points

| Want to… | Do this |
|---|---|
| Add a data source | Add a `fetch::Source` variant and an adapter module that returns a `DataPack`. |
| Change how cards are read | Edit `tag_rules` / `tag_roles` / `card_tags` in the playbook (`role_rules` / `card_roles` for roles no tag gives); add a `Condition` variant if a new predicate is needed. `audit --tags` lists the class cards left untagged. |
| Add a deck metric | Extend `DeckProfile` and `profile()`; show it in `views/breakdown.rs`. |
| Simulate a new card mechanic | Read its `vars` key in `sim::play_of`, carry it in `Fight`, and add a `Resource` if it should be reported. |
| Add a run chart | Add a field to `FloorPoint` in `timeline`, then a panel in `views/analytics.rs::run_section` (History gets it too). |
| Add a threat signal | Extend `ThreatProfile` and `demands()`; thresholds go in `ActPlan`. |
| Add a run action | Add a `Change` variant and handle it in `apply_change`; add UI in `views/log.rs`. |
| Add a panel | New file in `views/`, call it from `views::root`. |

## Roadmap

Each milestone is its own PR.

1. **M1 (this)**: catalog, playbook, event-sourced run, analysis, egui
   dashboard, manual Codex update.
2. **M2**: enchant and transform flows in the log panel; potion-slot changes;
   shop node helper (removal cost, gold planning).
3. **M3**: route planning: enter the visible map and get elite/rest/`?` counts
   against the deck's readiness.
4. **M4** (done): run history, floor-by-floor replay, run analytics and the
   deck simulator.
5. **M5**: community stats from Codex (`/api/runs/metrics/*`) to weigh card
   picks and event options, cached and updated by the same button.
