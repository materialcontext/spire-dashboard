# Spire Dashboard

A desktop companion for **Slay the Spire 2** that helps you learn and play the
*macro* game: what your deck is missing, what the next fight will ask of it,
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

The run saves after every change and reloads on start.

## Game data

The app ships with a full snapshot of the game data (cards, characters, relics,
potions, enchantments, monsters, encounters, events, acts), taken from the
[Spire Codex](https://spire-codex.com) API. It works offline.

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

**Export playbook** in the Data window copies it to your data folder. Edit it,
then press **Reload from disk**.

To see how the current playbook classifies every card and encounter:

```bash
cargo run -p spire-data --example audit -- --cards
```

## Development

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

See `docs/ARCHITECTURE.md` for the design and extension points.

Game data comes from Spire Codex (community-maintained). Not affiliated with
Mega Crit.
