# Plan: monitored runs

Status: built with the companion mod (route B), per the decisions below.
It hasn't run against a real game yet: everything is checked against
community `.run` files and simulated link files.

Today the player logs each floor by hand. A **monitored run** would follow
the game and log floors by itself: the room, the fight or event, cards,
relics and potions gained or lost, upgrades, enchantments and transforms,
HP, max HP and gold. The dashboard would update as the run goes.

This plan covers:

- how Slay the Spire 2 can be observed
- what the game already records
- two ways to get it (a companion mod, or the game's own run save)
- how either feeds the existing event log
- the decisions to make, and what to verify on a real install

Facts marked **(verify)** come from community sources or reasoning, not from
a real install. Each needs a look at real files first.

> **This conflicts with a project rule.** `CLAUDE.md` says: "Never read the
> game process or save files; the player reports state." A monitored run
> does one or the other by design. It can only go ahead if that rule
> changes. Decision M1 below.

---

## 1. What the game offers

### Official mod support

Slay the Spire 2 runs on Godot 4.5 with C# (.NET 9). Its own loader picks
up mods from `<game>/mods/`:

- **Manifest:** a `<id>.json` declaring `has_dll`, `has_pck`,
  `dependencies`, `min_game_version` and `affects_gameplay`. It is the only
  file a mod needs.
- **Code:** a `<id>.dll`, plus a Godot `<id>.pck` for assets.
- **Entry point:** a static method marked `[ModInitializer]`.

A mod observes the game in two ways:

- **Semantic hooks** in `MegaCrit.Sts2.Core.Hooks.Hook`. They cover rooms,
  combat, turns, cards, rewards, merchants, potions, powers, healing and
  more. `ModHelper.SubscribeForRunStateHooks` lets a mod receive them
  without owning a card or relic.
- **Harmony patches** on game methods (Harmony ships with the game).

Players enable mods in Settings, behind a consent dialog shown on first
launch. Since v0.107.1, Steam Workshop is the official way to install mods.
`affects_gameplay: false` keeps a mod out of the co-op check that host and
clients run the same gameplay mods.

**With mods on, the game is reported to use a separate "modded" profile**
(saves under `modded/profile1/`), apart from vanilla play **(verify)**.
If so, a player who installs even a read-only companion mod plays in a
different profile while monitoring: different unlocks and a different run
history. That is the biggest cost of the mod route.

### Existing mods that read game state

Several community mods already export live state. None is a dependency we
should take, but they show it's practical:

| Mod | What it does |
|---|---|
| **STS2MCP** (MIT) | HTTP API on `localhost:15526`: full game state, plus actions so AI agents can play. Also profile, compendium and run-history endpoints. |
| **STS2-Cli-Mod** | Game state and 40+ actions over a named pipe, as JSON. |
| **Communication_Mod_STS2** | A port of StS1's CommunicationMod (stdin/stdout JSON). |

All of them can also **control** the game. A monitored run needs only to
read, and should expose nothing that can act. That is a reason to write our
own, very small mod rather than depend on one of these.

### What the game already records

Each finished run is saved as JSON in `saves/history/<timestamp>.run`.
In-progress runs are in `saves/current_run.save`. Both sit under:

| OS | Folder |
|---|---|
| Windows | `%APPDATA%\SlayTheSpire2\steam\<steam id>\profile1\saves` |
| macOS | `~/Library/Application Support/SlayTheSpire2/steam/<steam id>/profile1/saves` |
| Linux | `~/.local/share/SlayTheSpire2/steam/<steam id>/profile1/saves` |

Confirmed on Windows: `C:\Users\<user>\AppData\Roaming\SlayTheSpire2\steam\<steam id>\profile1\saves`,
with mods on. The mod's own lookup of this folder (the game's
`GetProgressPathForProfile`) came back empty there, so the app finds it
itself: every `<store>/<account>/[modded/]profile<N>/saves` under the game's
data folder, choosing the one whose `current_run.save` changed last
(`game_link::Link::locate`). The folder the mod names is only one candidate:
save-path mods such as UnifiedSavePath keep modded saves in the vanilla
`profile<N>`, while the game's own lookup still reports `modded/profile<N>`,
which can hold an old run from before.

Our Spire Codex adapter (`codex_runs.rs`) already parses the `.run` format
for community statistics. Its per-floor record, `map_point_history`, maps
almost one-to-one onto our `NodeReport`. Field counts below come from 800
community runs.

| `.run` field (per floor) | Our event log |
|---|---|
| `map_point_type`: monster, elite, boss, unknown, rest_site, shop, treasure, ancient | `NodeKind` |
| `rooms[].model_id` (`ENCOUNTER.…`, `EVENT.…`) | `encounter` or `event` |
| `cards_gained` | `Change::AddCard` |
| `cards_removed` | `Change::RemoveCard` |
| `upgraded_cards` | `Change::UpgradeCard` |
| `cards_enchanted` | `Change::Enchant` |
| `cards_transformed` (`original_card` → `final_card`) | `Change::Transform` |
| `relic_choices` (`was_picked`), `bought_relics` | `Change::AddRelic` |
| `relics_removed` | `Change::RemoveRelic` |
| `potion_choices` (`was_picked`), `bought_potions` | `Change::AddPotion` |
| `potion_used`, `potion_discarded` | `Change::RemovePotion` |
| `current_hp`, `max_hp`, `current_gold` | `SetHp`, `SetMaxHp`, `SetGold` |
| `damage_taken`, `turns_taken` | timeline and community comparison |
| `card_choices` (cards offered, which was picked) | not tracked yet; could feed pick analysis |
| `rest_site_choices`, `event_choices`, `ancient_choice` | context for the floor |

The run also records:

- `players[]`: character, `deck`, `relics`, `potions` and
  `max_potion_slot_count`, the full state to check against.
- `acts`, `ascension`, `seed`, `win`, `killed_by_encounter` and
  `was_abandoned`, which cover `Start`, `EnterAct` and `End`.

What the `.run` format **doesn't** carry is the act's boss before it's
fought. The game shows it on the map at the start of an act. Our analysis
wants it then: boss previews and the Ancient heal rule.
`current_run.save` probably carries the map, and so the boss **(verify)**;
a mod can read it directly.

Whether `current_run.save` holds the same `map_point_history` as a finished
`.run` is the key question to verify. A modding guide calls the save format
"JSON-based". A community companion site imports `current_run.save`, and
another tool watches it to sync live, so it is readable and updates during
a run **(verify the contents and when the game writes it)**.

---

## 2. Two routes

### Route A: watch the game's run save (no mod)

When the player turns on monitoring, the app finds the profile's `saves`
folder and watches `current_run.save`. On each change it reads the file,
converts new `map_point_history` entries to `NodeReport`s, and records them.

| For | Against |
|---|---|
| Nothing to install; the player keeps their vanilla profile. | Undocumented format; a game update can change it (the same risk the Codex adapter already carries). |
| Same format we already parse; most of the converter exists. | Updates only when the game saves **(verify how often: per room is likely)**. |
| Read-only; nothing runs inside the game. | No live combat detail (turn, intents); floor-level only. |
| Works the same on Windows, macOS and Linux. | Needs the player's Steam ID folder; a picker when there are several profiles. |

### Route B: a read-only companion mod

A small C# mod (`SpireDashboardLink`, `affects_gameplay: false`):

- **When:** hooks the end of each room, and run start and end.
- **What:** writes a snapshot of the run (the game's own run serialization,
  plus the act's boss) to a file in the dashboard's data folder.
- **How it's written:** atomically (write a temporary file, then rename).
- **What it never does:** listen on a port or accept commands.

The app watches that file the same way as in route A.

| For | Against |
|---|---|
| Exact timing (room end, run start), the act boss up front, and a schema we control. | Players install a mod and, reportedly, play in a separate modded profile **(verify)**. |
| Could later send live combat state for a "this turn" helper. | C# and .NET 9 to maintain; game updates in Early Access can break hooks or Harmony targets. |
| No network surface: a file drop, not a server. | Building against `sts2.dll` needs the game. It can't be in the repo or CI (proprietary). |
| | Distribution: Steam Workshop or a manual copy into `<game>/mods/`. |

**Keeping the mod buildable in CI.** Compile it with no reference to the
game's assemblies, reaching the game by reflection and Harmony patches by
name. Take Harmony from NuGet (`Lib.Harmony`, MIT) for the build only. CI
then builds it, and a version check against `release_info.json` disables
it on unknown game builds.

### Recommendation

**Start with route A.** The data we need is already in a format we parse,
it costs the player nothing to turn on, and it keeps them in their vanilla
profile. Build route B only if route A's timing or missing pieces (the act
boss, live combat) turn out to matter in practice. Both routes feed the same
converter, so A's work carries over.

Using STS2MCP's HTTP API instead is possible for a prototype. But it
exposes control actions to any local process, and it ties us to another
project's schema and release timing.

---

## 3. In the app

### Converter (`spire-data`)

`spire-data` gets a new adapter, `game_run.rs`. Where existing adapters
return a `DataPack`, this one returns run events.

- **`start(run) -> RunEvent::Start`:** character, ascension, starting deck,
  relics, potion slots and HP, from the first map point.
- **`floor(point, before: &RunState) -> RunEvent::Node`:**
  - room kind
  - encounter or event
  - the change list (above)
  - HP, max HP and gold as `Set…` changes, only when they differ
- **`act(index, act, boss) -> RunEvent::EnterAct`:** from `acts`.
- **`end(run) -> RunEvent::End`:** victory, died (`killed_by_encounter`) or
  abandoned.
- **`reconcile(snapshot, state) -> Option<RunEvent::Adjust>`:** compares
  the full deck, relics, potions, HP and gold with ours. It corrects drift
  (an effect the per-floor record doesn't itemise) as one Adjust.

Ids map through the catalog: `CARD.X` → `CardId("X")`, upgrades through
`current_upgrade_level` **(verify)**. Unknown ids are kept and reported, as
the Codex adapter does. It is pure, so it can be tested on fixture `.run`
files from community exports.

### Watching (`spire-app`)

- **Settings → Monitored run:** off by default.
  - Finds the profile folder and shows which run it sees.
  - Only reads while on; nothing starts on its own (the spirit of the
    "no background fetch" rule).
- **Watching the file:** check its modified time about twice a second
  while monitoring and the window is open. That costs one `stat` per check,
  and avoids a new file-notification dependency.
- **Recording:**
  - New floors become `Action::Record(RunEvent::Node(…))`, through
    `SpireApp::apply` like a manual commit, so undo, saving and history
    work unchanged.
  - A finished run (the save disappears and a new `.run` appears) becomes
    `End` and goes into history, as a victory or a death. When the game
    looks to be on another run while the followed one isn't over, the app
    first looks for the followed run's `<start time>.run` in every
    profile's history: with its save gone, another profile's old save can
    be the newest. After filing, runs that started before the filed one
    aren't taken up; the app waits for the next.
- **The Log panel while monitoring** shows the incoming floors. A **Pause**
  button stops it, and manual logging takes over. If the monitored run and
  the app's run disagree at start (a different seed or character), the app
  asks before replacing anything.
- **Co-op:** a `.run` lists every player. Monitoring follows the local
  player (matched by Steam ID).

The per-floor data adds things the app doesn't record today. It can feed
existing analysis later:

- card offers and picks, for a pick-rate view
- damage per fight, which the timeline already uses

---

## 4. Decisions (made)

- **M1. Allowed.** The app may read the game's run saves and the mod's link
  file while monitoring is on. It never reads the game's memory, never
  writes to the game's folders, and reads nothing while monitoring is off.
- **M2. The companion mod.** Kept as small as possible: it reports where
  the run save is, when it changed, and the act's boss. The game's own save
  stays the record of the run.
- **M3. No live combat** for now.
- **M4. GitHub.** CI builds the mod and attaches the zip; no Workshop.
- **M5. Card offers aren't recorded.**

### As built

- **`mod/SpireDashboardLink`** (C#, about 250 lines):
  - Read-only, with no Harmony patches.
  - Twice a second, on the game's main thread, it reads through reflection:
    - `RunManager.Instance.IsInProgress` and `DebugOnlyGetState()`: the
      current act index, the act, and its boss encounters.
    - `SaveManager.Instance.CurrentProfileId` and
      `ProgressSaveManager.GetProgressPathForProfile`: the profile's
      `saves` folder, which is under `modded/` while mods are on.
  - When anything changed, it writes `game-link.json` in
    `<local app data>/SpireDashboard/`, atomically.
  - It builds without the game: `mod/Sts2Stub` stands in for `sts2.dll`'s
    loader attribute, and Godot comes from NuGet.
- **`spire-data::game_link`** reads the link and the saves it names.
- **`spire-data::game_run`** turns a run save or `.run` into our events.
  It replays as it goes and ends with one correction to the game's own
  totals.
- **`spire-app::monitor`** is the watcher thread. It checks two modified
  times twice a second, reads and sends only on a change, and wakes the UI
  only then.
- **`SpireApp`:**
  - Adopts the game's run when the app has none.
  - Updates the run it follows, with a toast per new floor.
  - Files the run in history when it ends.
  - Asks before replacing a hand-logged run.
  - Locks the floor log while following.
  - Leaves HP, including Ancient heals, to the game.

## Earlier options (for the record)

- **M1. The project rule.** Allow reading the game's save files (route A)
  and/or a companion mod (route B). Today `CLAUDE.md` forbids both.
  - **Suggested wording:** "The player reports state, or the app reads it
    from the game's run save when the player turns on monitoring. Never
    read the game's memory or process, and never write to the game's
    folders."
- **M2. Route.** A first (recommended), B first, or both.
- **M3. Live combat.** Out of scope (floor-level only), or a later
  route B phase.
- **M4. Mod distribution** (route B only): Steam Workshop, GitHub releases,
  or the app copying it into `<game>/mods/` (that writes to the game
  folder, which needs M1 to allow it).
- **M5. Card offers.** Record `card_choices` in our log for pick analysis,
  or ignore them.

## 5. Phases (each its own PR)

| Phase | Work | Done when |
|---|---|---|
| **P0 Verify** | On a real install: read a `current_run.save` mid-run; note when the game writes it; compare it to the finished `.run`; check the boss and map; check whether mods switch to a modded profile. Collect fixture files (no personal IDs). | The **(verify)** items here are answered, with fixtures in `crates/spire-data/tests/fixtures/`. |
| **P1 Converter** | `game_run.rs`: start, floor, act, end, reconcile, with tests on fixtures and community exports. | A whole `.run` replays into a `Run` whose final state matches the file's `players[]`. |
| **P2 Monitoring** | Settings toggle, profile discovery, the polling watcher, recording through actions, pause, end of run. | A real run logs itself floor by floor; undo and history behave as with manual logs. |
| **P3 Polish** | Status in the header ("Monitoring · floor 12"), drift reconciliation, co-op player choice, docs. | A full run with no manual input, on Windows and macOS. |
| **P4 Companion mod** (optional) | Route B, if M2 and M3 call for it. | As in route B above. |

## 6. Effort

- **Route A:** mostly Rust, in code we know. The converter is the bulk
  (about the size of `codex_runs.rs`). The watcher and settings are small.
  Two to three PRs.
- **Route B:** adds a C# project, a release process, Workshop publishing,
  and upkeep with every game update. It's worth it only for live combat
  data, or if route A's save timing proves too coarse.

## Sources

Modding:
- [Slay the Spire 2 Modding Tutorial](https://github.com/fresh-milkshake/Modding-Tutorial) (verified against v0.107.1):
  - [loader surface](https://github.com/fresh-milkshake/Modding-Tutorial/blob/HEAD/docs/02-vanilla-sts2-layout.md)
  - [manifests](https://github.com/fresh-milkshake/Modding-Tutorial/blob/HEAD/docs/03-mod-loading-and-manifests.md)
  - [hooks and Harmony](https://github.com/fresh-milkshake/Modding-Tutorial/blob/HEAD/docs/05-entrypoints-harmony-and-hooks.md)
  - [state and saves](https://github.com/fresh-milkshake/Modding-Tutorial/blob/HEAD/docs/15-state-saves-and-randomness.md)
- [STS2FirstMod](https://github.com/jiegec/STS2FirstMod), a minimal example mod
- Patch notes: [Major Update #2 v0.107.1](https://store.steampowered.com/news/app/2868840/view/710026912607505280) (Steam Workshop); [v0.99.0 beta](https://sts2.untapped.gg/en/patch-notes/v0.99.0?tab=notes) (external manifest, `affects_gameplay`)
- State-export mods: [STS2MCP](https://github.com/Gennadiyev/STS2MCP), [STS2-Cli-Mod](https://github.com/longkerdandy/STS2-Cli-Mod), [STS2-Agent](https://github.com/CharTyr/STS2-Agent); for StS1, [CommunicationMod](https://github.com/ForgottenArbiter/CommunicationMod)
- Modded profile: [Nexus Mods STS2 mod pages](https://www.nexusmods.com/slaythespire2/mods/100), [combat log mod](https://www.nexusmods.com/slaythespire2/mods/992)

Save files:
- [Where is run history saved?](https://steamcommunity.com/app/2868840/discussions/0/802341195824164375/) (Steam discussions)
- [Stat Your Spire](https://github.com/dimmenfredrik/stat-your-spire), which reads `.run` files
- [STS Tracker Run Companion](https://ststracker.app/companion), which imports `current_run.save` and links a desktop app that watches it
- [Slay the Spire 2 save file location](https://primagames.com/tips/slay-the-spire-2-save-file-location-on-pc)
- `.run` field coverage: 800 community runs from the Spire Codex run export (`/api/exports/runs`), already used by `codex_runs.rs`
