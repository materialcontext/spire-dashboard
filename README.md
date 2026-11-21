# Spire Dashboard

A desktop companion for **Slay the Spire 2** (beta branch) for the macro game: what your deck lacks, what the next fight asks of it, which `?` rooms are worth taking, what the boss demands.

Log each floor by hand, or let it follow the game through a read-only companion mod.

## Run tab

- **Vitals:** character, act, floor, HP, gold, potions, deck size.
- **Deck, relics, potions:** each card's roles in a strip of columns (six roles, Draw, Energy); hover for why. Relics get roles when their text is plain (Vajra: Scaling Damage); potion-slot relics add slots.
- **Deck composition:** each role's simulated job (Block on ordinary and defending turns, damage turns 1–3, growth, AoE share, hits a turn) on the range of **A10 community decks that beat this act's boss** going in: band = middle half, tick = median, your card count beside it. Damage to Block, deck size and dead draws below. The range moves from act entry to the boss as you play. **No hand-set targets:** a measure turns amber only where community runs show that side losing the boss more often (e.g. too little Block, too many AoE cards in Act 2); win rates on hover. Setup piling up (Aim, Vigor, Stretch) and Arrows or Stars running short are noted.
- **Act and boss:** HP, turn-by-turn moves, what decides the fight (roles whose weak decks die there clearly more in A10 community runs; a gap when your deck is under the line), and the **survival check**: the lead approach (Race, Wall, Windows, Scale) and On track, Typical or At risk against community decks at that fight ([DEATHS.md](docs/DEATHS.md)).
- **Enemy turns:** each monster's move order (cycles, random picks with odds, conditional picks) with its Strength and the Vulnerable it applies; where community runs die and on which move.
- **Next fight:** Monster, Elite and Boss for this act, weak pool first; hallway fights already had, the elite just fought and the fight being played are left out.
- **Fish for relics:** only when a `?` room this act can still give a relic crucial to your deck, by its cards (Lost Wisp with 6+ Powers) or a desperate shortfall of Block or max HP (Orichalcum, Pear). Names the event and option (with cost) or the treasure-room odds, and how often runs meet each.
- **Watch for events:** event options doing what the deck needs: take out a removable curse, raise max HP when short, heal under 30% HP. Each option's effects are measured from community runs (Slippery Bridge's Overcome always removes a card; Wood Carvings' Bird never touches a curse), with how often it delivers and what it costs (red if the HP would kill you). Three places each, the rest on hover.
- **Look for in card rewards:** up to three. *Lines within reach*: cards community decks beat this boss with more often together, where you hold part. Then *cards for what you lack against the boss*: class cards high-skill players win more with, kept only if the simulator improves against the boss. Hover a label for numbers, a card for details.
- **Priorities:** composition warnings, boss demands with no card, removable curses. Run-specific only.

## Other tabs

- **Analytics:** HP, max HP, gold by floor; HP lost per fight against other players; roles by floor; dead cards; elites per act; the deck simulation (damage and Block per turn with spread, turnover, draw odds by turn 3, dead draws, energy fit).
- **History:** every finished or abandoned run, by character and outcome. Opens on **How it ended** (ranked causes, the deciding fight) beside a row per act (HP, damage against players', deck changes, warnings, the boss). Charts mark big losses, elites and rest sites; a floor replay; the **Decisions** log (offered vs taken, rest choice, gold, potions; click a floor to replay). **Edit run…** fixes ascension, result, notes, a floor's fight or a stray floor, or deletes the run.
- **Cards, Relics, Enemies:** catalogs with filters; relics show what the simulator reads and leaves out.

The simulator models orbs and Focus, Osty, Summon, Stars, Forge, Doom, Poison, Aim, Stretch, Vigor and the Fool's Arrows; powers and relics are read from their text ([SIMULATION.md](docs/SIMULATION.md)).

## Run it

```bash
cargo run --release -p spire-app
```

Linux needs the usual X11/Wayland runtime libraries (e.g. `libxkbcommon-x11-0`).

## Log by hand

1. Pick character and ascension, then the act and (once seen) its boss. The floor log stays locked until both are set.
2. After each floor: room, encounter or event, HP/max HP/gold, any card, relic or potion; `+`/`x` on deck cards queue upgrades and removals (right-click for more). **Commit**.
3. After a boss, choose the next act. **End run** files it in History.

**Undo** reverts the last step; **Apply as correction** fixes without advancing. Keys: 1–8 room, Ctrl+Enter commit, Ctrl+Z undo, Ctrl+1–6 tabs, Esc closes dialogs; dropdowns follow the arrow keys. The run saves on every change.

Healing follows the game: start at full HP (80% from A2); from Act 2 the Ancient heal applies once the boss is chosen (all missing HP, 80% from A2); at a Rest Site, **Rest +N** fills in 30% of max HP plus relics (Regal Pillow).

## Follow the game

The companion mod **Spire Dashboard Link** only reads: it reports where the game saves your run, the act's boss and the room you're in; the dashboard reads the game's save. Details: [MONITORED_RUN.md](docs/MONITORED_RUN.md).

1. Get the mod: the `SpireDashboardLink` artifact from the latest CI run (GitHub Actions), or `dotnet build mod/SpireDashboardLink/SpireDashboardLink.csproj -c Release` (.NET 8 SDK).
2. Copy `SpireDashboardLink.dll` and `SpireDashboardLink.json` into `mods/SpireDashboardLink/` in the game folder (Windows: `…\steamapps\common\Slay the Spire 2\mods\`; macOS: `SlayTheSpire2.app/Contents/MacOS/mods/`).
3. Enable mods in the game (consent dialog the first time).
4. **Settings → Monitored runs → Follow the game.** It shows when the mod is connected.

While following: floors log themselves and the log is locked; the current fight or event shows above Next fight (mod 0.2.0+); a finished run files to History; a hand-logged run is never replaced without asking. With mods on, the game uses a separate *modded* profile. Nothing is read while following is off.

## Appearance

| Look | Style |
|---|---|
| **System** (default on Windows, macOS) | The OS's own: Windows 11 (Segoe UI, accent, Fluent cards, Mica, top tabs) or macOS (SF Pro, accent, sidebar, toolbar, menu bar). Follows light/dark and contrast themes ([NATIVE_LOOK.md](docs/NATIVE_LOOK.md)) |
| **Spirebound** (default on Linux) | High contrast, Inter; character colour on the header |
| **Forge** | Charcoal and embers |
| **Neon Spire** | Synthwave, glowing edges |
| **Grimoire** | Light: parchment, ink, crimson |
| **Terminal** | Phosphor-green CRT, monospace |

**Larger text** (any look): ~10% bigger type, roomier spacing, brighter secondary text; on by default when the OS text size is above 100%. **Match system light/dark** switches Grimoire/Spirebound with the OS. **Use the system title bar** replaces the app-drawn one (drag to move, double-click to maximise). Shared rules: [DESIGN_PRACTICES.md](docs/DESIGN_PRACTICES.md).

## Game data

A full **StS2 beta** snapshot from [Spire Codex](https://spire-codex.com) ships with the app; it works offline. Ascension follows StS2 (A0–A10), read from the ascension data: A4 one fewer potion slot, A5 Ascender's Bane, A8/A9 enemy ascension HP/damage, A10 two Act 3 bosses.

Community runs on Spire Codex supply exact enemy lineups, per-fight damage and deaths, event effects, card lines, the A10 composition ranges and risks, and what decides each elite and boss.

- **Update:** **Data… → Update data now**. Never automatic. The download layers over the bundle; on failure the current data stays. **Revert to bundled** drops it. The Data window can switch to the stable branch or another source (a base URL is the Codex API; a `.json` URL is a data pack).
- **Fix an entity:** a partial data pack in `user.json` in the data folder overrides by id.
- **Regenerate the bundle:** `cargo run -p spire-data --bin refresh-bundle`.

## Playbook (strategy as data)

`data/playbook.json` holds what the dashboard believes: tag and role rules and per-card overrides, big-hit and long-fight thresholds (hallway and unmeasured fights), `relic_fits`, `event_fits` and `short`, card-reward `picks`, run-report `report` lines, simulator assumptions (`simulation`), and how composition and fight demands are measured (`composition`: ascension, builds, player cap, confidence, smallest gap; the ranges, warnings and demands themselves are measured at each data refresh). **Export playbook** in the Data window, edit, **Reload from disk**.

Inspect it with `cargo run --release -p spire-data --example audit -- --cards` (or `--sim`, `--tags`, `--roles <pages>`; full list in `CLAUDE.md`).

## Development

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Design: [ARCHITECTURE.md](docs/ARCHITECTURE.md). Frame budget and UI rules: [PERFORMANCE.md](docs/PERFORMANCE.md).

Game data from Spire Codex (community-maintained). Not affiliated with Mega Crit.
