# Monitored runs

With **Settings → Follow the game** on, the app logs floors itself from the game's run save and the companion mod's link file. Off by default; nothing is read while off.

## Rules (decided)

- Read the game's run saves and the mod's link file only while monitoring is on.
- Never read the game's memory or process; never write to the game's folders.
- The mod is read-only: no Harmony patches, no network port, no commands.
- No live combat state.
- CI builds the mod and attaches the zip; no Workshop.

## Pieces

| Piece | Does |
|---|---|
| `mod/SpireDashboardLink` (C#, mod 0.2.0, link schema 2) | Twice a second on the main thread, by reflection: run in progress, act index, act and its bosses (`RunManager`), the profile's `saves` folder (`SaveManager`, `ProgressSaveManager`), and the room being played (`CurrentRoom`: class, room type, event id, encounter id, live enemy ids). On change, writes `<local app data>/SpireDashboard/game-link.json` atomically. Builds without the game: `mod/Sts2Stub` stands in for `sts2.dll`, Godot from NuGet |
| `spire-data::game_link` | Reads the link; `Link::locate` finds every `<store>/<account>/[modded/]profile<N>/saves` and picks the one whose `current_run.save` changed last (the mod's own path lookup can be empty or stale, e.g. with save-path mods) |
| `spire-data::game_run` | Converts a run save or `.run` into `RunEvent`s, replaying as it goes, ending with one correction to the game's totals. Fills each floor's `FloorRecord` (offers and picks, rest choice, potions used, gold spent) |
| `spire-app::monitor` | Watcher thread: modified times of the link and every profile's run save every 500 ms; reads and sends only on change (a file caught mid-write is re-read up to 4 times first); wakes the UI only then |
| `spire-app::follow` | Pure: `read` the game's run or the reason following stopped; `step`: advance, adopt, ask, wait |
| `SpireApp::sync_game` | Adopts the game's run when the app has none; adds floors (toast per floor); asks before replacing a hand-logged run; locks the floor log; files the run in history when it ends; leaves HP (including Ancient heals) to the game; shows the current fight or event from the link |

Ending a followed run: when the game is on another run but the followed one isn't over, the app first looks for the followed run's `<start time>.run` in every profile's history (another profile's old save can be the newest). After filing, runs that started earlier aren't taken up.

## Game files

| OS | Saves folder |
|---|---|
| Windows | `%APPDATA%\SlayTheSpire2\steam\<steam id>\profile1\saves` (confirmed, mods on) |
| macOS | `~/Library/Application Support/SlayTheSpire2/steam/<steam id>/profile1/saves` |
| Linux | `~/.local/share/SlayTheSpire2/steam/<steam id>/profile1/saves` |

Mods on may move saves under `modded/profile<N>`. Finished runs: `saves/history/<timestamp>.run`; in progress: `saves/current_run.save`. Both are the format the Codex run export uses (`run_file.rs`).

| `.run` per floor | Event log |
|---|---|
| `map_point_type` | `NodeKind` |
| `rooms[].model_id` (`ENCOUNTER.…`, `EVENT.…`) | encounter / event |
| `cards_gained`, `cards_removed`, `upgraded_cards`, `cards_enchanted`, `cards_transformed` | `AddCard`, `RemoveCard`, `UpgradeCard`, `Enchant`, `Transform` |
| `relic_choices` (picked), `relics_removed` | `AddRelic`, `RemoveRelic` |
| `potion_choices` (picked), `potion_used`, `potion_discarded` | `AddPotion`, `RemovePotion` |
| `current_hp`, `max_hp`, `current_gold` | `SetHp`, `SetMaxHp`, `SetGold` |
| `card_choices`, `rest_site_choices`, `gold_spent` | `FloorRecord` |

The `.run` lacks the act's boss before it's fought; the mod supplies it. The run save records a room only once finished, so the room being played comes from the mod.

## Mod platform notes

Slay the Spire 2: Godot 4.5, C# (.NET 9). Mods load from `<game>/mods/` with a manifest (`has_dll`, `has_pck`, `dependencies`, `min_game_version`, `affects_gameplay: false` keeps it out of co-op checks) and a `[ModInitializer]` entry point. Players enable mods behind a consent dialog; Workshop is the official channel since v0.107.1.

## Sources

[Modding tutorial](https://github.com/fresh-milkshake/Modding-Tutorial) (v0.107.1: [loader](https://github.com/fresh-milkshake/Modding-Tutorial/blob/HEAD/docs/02-vanilla-sts2-layout.md), [manifests](https://github.com/fresh-milkshake/Modding-Tutorial/blob/HEAD/docs/03-mod-loading-and-manifests.md), [hooks](https://github.com/fresh-milkshake/Modding-Tutorial/blob/HEAD/docs/05-entrypoints-harmony-and-hooks.md), [saves](https://github.com/fresh-milkshake/Modding-Tutorial/blob/HEAD/docs/15-state-saves-and-randomness.md)) · [STS2FirstMod](https://github.com/jiegec/STS2FirstMod) · [v0.107.1 notes](https://store.steampowered.com/news/app/2868840/view/710026912607505280) · [v0.99.0 notes](https://sts2.untapped.gg/en/patch-notes/v0.99.0?tab=notes) · [run history location](https://steamcommunity.com/app/2868840/discussions/0/802341195824164375/) · [Stat Your Spire](https://github.com/dimmenfredrik/stat-your-spire) · [STS Tracker companion](https://ststracker.app/companion) · [save location](https://primagames.com/tips/slay-the-spire-2-save-file-location-on-pc)
