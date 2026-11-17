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
downloaded → user**, where later layers override earlier ones by id.

### Playbook: strategy knowledge, as data

`playbook.rs` plus `data/playbook.json`. It holds:

- Role rules: composable `Condition`s over card fields (`has_damage`,
  `{"applies_power": "Strength"}`, `{"all": [...]}`…).
- Per-card role overrides.
- The area-damage rule.
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

- `deck`: classify cards into roles; `DeckProfile` (by type and by role, with
  starter cards counted separately); `coverage` against act targets.
- `threats`: `ThreatProfile` per encounter (HP, worst turn, buffs, groups),
  the `demands` it places on a deck, and `next_threats` for a node type.
- `events`: run `needs` (removal, heal, upgrade…), and a `watchlist` of
  act events ranked by how well they meet those needs.
- `advice`: act targets, boss gaps and act notes.

`Dashboard::compute` bundles everything a frame needs. The app recomputes it
only when the run or data changes, never per frame.

## App

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
gzipped JSON lines with one community-submitted run each. Every fight records
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

Results land in `Encounter::lineups`, `spawns` and `stats`. The update reads
`run_pages` pages (default 3, about 3,000 runs) from the last 14 days, so
lineups track the current game version. The export's own budget is 120
credits per hour, and one page costs 1 credit. If this step fails, the rest of
the update still applies.

## Extension points

| Want to… | Do this |
|---|---|
| Add a data source | Add a `fetch::Source` variant and an adapter module that returns a `DataPack`. |
| Change how cards are read | Edit `role_rules` / `card_roles` in the playbook; add a `Condition` variant if a new predicate is needed. |
| Add a deck metric | Extend `DeckProfile` and `profile()`; show it in `views/breakdown.rs`. |
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
4. **M4**: run history and post-run review: which roles were missing when HP
   was lost.
5. **M5**: community stats from Codex (`/api/runs/metrics/*`) to weigh card
   picks and event options, cached and updated by the same button.
