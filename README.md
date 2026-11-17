# Spire Dashboard

A desktop companion for **Slay the Spire 2** (beta branch) that helps you learn
and play the *macro* game: what your deck is missing, what the next fight will ask of it,
which `?` events to hope for, and what the act boss demands.

It never reads the game. After each node you tell it what happened: the fight,
the card you took, HP and gold after. Then it re-analyses the run.

## What it shows

- **Run vitals**: character, act, floor, HP, gold, potions, deck size.
- **Deck, relics and potions**, with each card's role on hover.
- **Deck breakdown** by card type (attack, skill, power, curse…) and by role:
  frontloaded damage, scaling damage, frontloaded defense, scaling defense,
  and consistency/acceleration, against per-act targets. Area damage is
  tracked too. Starter Strikes/Defends are shown separately: targets count
  what you've *added*.
- **Act and boss**: the boss's HP, worst turn, and what it demands, each
  marked covered, thin or missing for your deck.
- **Next threats** for Monster, Elite and Boss nodes in the current act. The
  list uses the early (weak) pool, then the normal pool. Unseen encounters
  come first, and each is checked against your deck.
- **`?` event watchlist**: events possible in this act, ranked by how well
  they answer what the run needs (removal for curses, healing when low…).
- **Macro advice**: role gaps for this act, boss gaps, and act notes.
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
- **History**: every run you end or abandon here, filtered by character and
  outcome, with the same charts, plus a floor-by-floor replay of the deck and
  relics.

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
undoes, Ctrl+1–5 switch tabs and Esc closes dialogs.

The run saves after every change and reloads on start.

## Appearance

**Settings** lets you choose a look, and the choice is saved.

| Look | Style |
|---|---|
| **Spirebound** (default) | High contrast in Inter. The accent follows your character. |
| **Forge** | Charcoal and embers. |
| **Neon Spire** | Synthwave with glowing edges. |
| **Grimoire** | A light theme: parchment, ink and crimson. |
| **Terminal** | A phosphor-green CRT, all monospace. |

**Larger text** is a separate readability switch that works with every look:

- type about 10% bigger
- roomier spacing
- filled chips
- brighter secondary text

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

- which card features mean which role
- per-card role overrides
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
