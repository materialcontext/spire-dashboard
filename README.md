# Spire Dashboard

A desktop companion for **Slay the Spire 2** (beta branch) that helps you learn
and play the *macro* game: what your deck is missing, what the next fight will ask of it,
which `?` events to hope for, and what the act boss demands.

It never reads the game. After each node you tell it what happened: the fight,
the card you took, HP and gold after. Then it re-analyses the run.

## What it shows

- **Run vitals**: character, act, floor, HP, gold, potions, deck size.
- **Deck, relics and potions**, with each card's role on hover. Relics are
  marked with a role too when what they do is plain from their text (Vajra:
  scaling damage), and relics that add potion slots add them when logged.
- **Deck breakdown** by card type (attack, skill, power, curse…) and by role,
  the problems a deck solves: Mitigation, Survival, Fast Damage, Scaling
  Damage, AoE and Chip, against per-act targets. Under each role, how well
  the deck actually does it, simulated: Block on an ordinary and a defending
  turn, damage over the first turns, how damage grows, how much hits every
  enemy, hits a turn. Cards that draw or gain energy are tagged (D and E in
  the deck), and the deck's output on a bad draw is noted. Starter
  Strikes/Defends are shown separately: targets count what you've *added*.
  Under that, the deck **next to community decks that cleared this act**
  (same character, or every character's when there are too few): each
  job, each tag's share and the share of starter cards, with the shortfalls
  first and the tags to look for.
- **Act and boss**: the boss's HP, what it does turn by turn, and what it
  demands, each marked covered, thin or missing for your deck.
- **Enemy turns**: every monster's move order from the game data (fixed
  cycles, random picks with their odds, conditional picks), with the
  Strength it builds and the Vulnerable it puts on you.
- **When runs end** at elites and bosses: how often community runs die
  there and on which turns, named by the move made that turn.
- **Survival check** for elites and bosses: on track, tight or at risk,
  from the deck simulator's damage and block against the enemies' turns.
  See [docs/DEATHS.md](docs/DEATHS.md).
- **Next threats** for Monster, Elite and Boss nodes in the current act. The
  list uses the early (weak) pool, then the normal pool. Hallway fights
  you've already had this act are left out, and so is the elite you just
  fought: community runs show neither comes straight back. Each is checked
  against your deck.
- **`?` event watchlist**: events possible in this act, ranked by how well
  they answer what the run needs (removal for curses, healing when low…).
- **Macro advice** about your run only: role gaps for this act, boss gaps,
  missing area damage and removable curses. There are no general tips.
- **Enemies seen** this run.
- **Analytics** for the current run:
  - HP, max HP and gold by floor.
  - HP lost in each fight next to what other players lose in the same fight.
  - Deck roles growing floor by floor against the act targets.
  - The share of dead cards, and elites fought per act.
- **Deck simulation**, read from the card data. Damage and block per turn
  come with a spread, alongside turnover, the odds of drawing each card by
  turn 3, dead draws, and whether energy fits the deck. Character mechanics are
  simulated: orbs and Focus, Osty, Summon, Stars, Forge, Doom, Poison, Aim,
  Stretch, Vigor and the Fool's Arrows.
  - Powers and relics are read from their text. That covers start- and
    end-of-turn effects, "whenever you play…" triggers, every-N counters,
    Vulnerable, Plating, and extra damage or Block for other cards.
  - The order effects apply in, and its sources, is in
    [docs/SIMULATION.md](docs/SIMULATION.md).
- **Relic catalog**: every relic, filtered by pool, rarity, whether it's in
  your run and whether the simulator uses it. Each relic shows what the
  simulator reads from it and what it leaves out.
- **History**: every run you end or abandon here, filtered by character and
  outcome, with the same charts, plus a floor-by-floor replay of the deck and
  relics. **Edit run…** fixes a filed run: its ascension, result, notes, each
  floor's fight, or a floor logged by mistake; or deletes it.

## Monitored runs (follow the game)

Instead of logging each floor by hand, the dashboard can follow your run in
Slay the Spire 2 through a small companion mod, **Spire Dashboard Link**. The
mod only reads: it changes nothing in the game. It tells the dashboard where
the game saves your run and what the act's boss is, and the dashboard reads
the game's own save after each floor.

1. Get the mod:
   - **Download:** open the latest CI run on GitHub (Actions) and download
     the `SpireDashboardLink` artifact.
   - **Or build it:** with the .NET 8 SDK, run
     `dotnet build mod/SpireDashboardLink/SpireDashboardLink.csproj -c Release`.
2. Copy `SpireDashboardLink.dll` and `SpireDashboardLink.json` into a
   `SpireDashboardLink` folder in the game's `mods` folder:
   - **Windows:** `…\steamapps\common\Slay the Spire 2\mods\`
   - **macOS:** `SlayTheSpire2.app/Contents/MacOS/mods/` (Show Package
     Contents)
3. Start the game and enable mods in its settings. A consent dialog appears
   the first time.
4. In the dashboard, open **Settings → Monitored runs** and turn on **Follow
   the game**. It shows when the mod is connected.

While following:

- **Logging:** floors log themselves, and the floor log is locked.
- **When the run ends:** it's filed in History.
- **If you're already logging a different run by hand:** the dashboard asks
  before switching.
- **Modded profile:** with mods on, the game plays in its separate *modded*
  profile, with its own unlocks and run history.

Turn following off to log by hand again. Nothing is read while it's off.

## Run it

```bash
cargo run --release -p spire-app
```

Linux needs the usual X11/Wayland runtime libraries (e.g. `libxkbcommon-x11-0`).

## Use it

1. Pick a character and ascension, then the act (and its boss once you see it).
2. After each node, in the right panel: choose the node type and the encounter
   or event, set HP/max HP/gold, add any card, relic or potion, and use `+`/`x`
   in the deck panel to queue upgrades and removals. Then press **Commit**.
3. After a boss, choose the next act.
4. **Undo** reverts the last step. **Apply as correction** fixes a mistake
   without advancing the floor.
5. **End run** files the run in History.

Keys are optional shortcuts: 1–8 pick the room, Ctrl+Enter commits, Ctrl+Z
undoes, Ctrl+1–6 switch tabs and Esc closes dialogs.

The floor log stays locked until the act and its boss are chosen.

Healing follows the game's rules:
- Runs start at full HP, or 80% of max from A2 (Weary Traveler).
- From Act 2, the Ancient heal is applied once the boss is chosen: all missing HP, or 80% of it from A2.
- At a Rest Site, **Rest +N** beside HP fills in the heal: 30% of max HP, plus relics like Regal Pillow.

Dropdowns and search results in the log follow the arrow keys, and Enter picks.

The run saves after every change and reloads on start.

The header doubles as the title bar:
- Drag it to move the window, and double-click it to maximise.
- Drag any edge or corner to resize.

The app draws this title bar itself, so desktop tools that restyle native title
bars (Mica For Everyone, blur themes) can't hide it. **Settings → Use the
system title bar** brings back the operating system's own.

## Appearance

**Settings** lets you choose a look, and the choice is saved.

| Look | Style |
|---|---|
| **System** (default on Windows and macOS) | Looks like the operating system. Windows 11 gets Segoe UI, your accent colour, Fluent cards and Mica, with tabs across the top and Settings as a page. macOS gets SF Pro, your accent colour, a sidebar, a toolbar, the menu bar and a Settings window. Follows light and dark mode, and Windows contrast themes. See `docs/NATIVE_LOOK.md`. |
| **Spirebound** (default on Linux) | High contrast in Inter. Your character's colour marks the header; gold marks what you can act on. |
| **Forge** | Charcoal and embers. |
| **Neon Spire** | Synthwave with glowing edges. |
| **Grimoire** | A light theme: parchment, ink and crimson. |
| **Terminal** | A phosphor-green CRT, all monospace. |

**Larger text** is a separate readability switch that works with every look:

- type about 10% bigger
- roomier spacing
- filled chips
- brighter secondary text

On a first run it starts on if Windows' text size is above 100%.

**Match system light/dark** follows the operating system: Grimoire in light
mode, Spirebound in dark. Picking a look while it's on sets the look for that
mode. The System look follows light and dark on its own.

Every look shares the same rules, from the Windows and macOS design guidance
(see `docs/DESIGN_PRACTICES.md`):

- **The accent** marks only what you can act on or what's selected.
- **Headings** are sentence case, except in Terminal.
- **Spacing** sits on a 4px grid.
- **Controls** have 3:1 outlines and 24px click targets.
- **Keyboard focus** shows a ring after Tab or the arrow keys.
- **Motion** is brief (83–167 ms fades and slides) and off in Terminal or when
  the system asks for reduced motion.
- **An inactive window** dims its header.

## Game data

The app ships with a full snapshot of the **StS2 beta branch** (cards,
characters including The Fool, relics, potions, enchantments, monsters,
encounters, events, acts and ascensions), taken from the
[Spire Codex](https://spire-codex.com) API. It works offline. The Data window
can switch to the stable branch.

Ascension follows StS2 (A0–A10), with the mechanical effects modelled:

| Level | Effect |
|---|---|
| A4 | One fewer potion slot |
| A5 | Start with Ascender's Bane |
| A8 | Threats use enemies' ascension HP |
| A9 | Threats use enemies' ascension damage |
| A10 | Two Act 3 bosses |

These are read from the ascension data itself.

Exact enemy lineups ("3× Corpse Slug", "3 of 5 Ruby Raiders", "Fogmog +
Eye With Teeth mid-fight") and average damage taken per fight are aggregated
from recent community-submitted runs on Spire Codex.

To refresh it, open **Data…** and press **Update data now**. Nothing updates
automatically. The download is saved locally and layered over the bundled
snapshot. If anything fails, the current data is kept. **Revert to bundled**
drops the download.

The source is configurable in the same window. A base URL means the Spire Codex
API; a URL ending in `.json` is read as a data pack in this app's own format
(see `docs/ARCHITECTURE.md`).

To fix or add an entity by hand, put a partial data pack in `user.json` in the
data folder. It overrides by id.

To regenerate the bundled snapshot:

```bash
cargo run -p spire-data --bin refresh-bundle
```

## Tuning the strategy (playbook)

Everything the dashboard *believes* lives in `data/playbook.json`, not in code:

- which card features mean which tag, and which tags give which role
- per-card tag and role overrides
- per-act role targets
- thresholds for "big hit" and "long fight"
- the words that mark event options as removal, upgrade, relic and so on
- what the deck simulator assumes where the game data is silent (`simulation`):
  cards drawn per turn, hand limit, turns and fights simulated, and hits that
  bring back a shot Arrow

**Export playbook** in the Data window copies it to your data folder. Edit it,
then press **Reload from disk**.

To see how the current playbook classifies every card and encounter:

```bash
cargo run -p spire-data --example audit -- --cards
```

To run the deck simulator on every starter deck:

```bash
cargo run --release -p spire-data --example audit -- --sim
```

## Development

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

See `docs/ARCHITECTURE.md` for the design and extension points, and
`docs/PERFORMANCE.md` for benchmarks and the rules that keep frames cheap
on weak hardware.

Game data comes from Spire Codex (community-maintained). Not affiliated with
Mega Crit.
