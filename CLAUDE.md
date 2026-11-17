# Spire Dashboard

Slay the Spire 2 macro dashboard: Rust workspace, egui UI. Read `docs/ARCHITECTURE.md` before working.

## Commands

- `cargo run -p spire-app`: run the app
- `cargo test --workspace`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo fmt --all --check`
- `cargo run -p spire-data --bin refresh-bundle`: regenerate `data/catalog.json` from Spire Codex
- `cargo run -p spire-data --example audit [-- --cards]`: see how the playbook reads the data

fmt, clippy and test must pass before pushing (CI runs them).

## Rules

- `spire-core` is pure: no IO, network, egui or new dependencies beyond `serde`. Plain functions over immutable data.
- Dependency direction is one-way: `spire-app` → `spire-data` → `spire-core`.
- Game facts go in the catalog (from data sources). Strategy beliefs go in `data/playbook.json`, not in code.
- Run state changes only through `RunEvent`/`Change` and `run::apply`. Never mutate `RunState` elsewhere.
- UI views push `Action`s; only `SpireApp::apply` changes state.
- Never fetch data without an explicit user action. No background or startup updates.
- Never read the game process or save files; the player reports state.
- New data sources are adapters that return a `DataPack`; source-specific types stay in their adapter module.
- Slay the Spire 2 only, beta branch. Never carry over StS1 facts (A20, Act 4, keys, StS1 cards or relics). Game rules come from data (`GameRules`, ascensions) or from community run data, never from memory.
- UI uses the palette and components in `style.rs` and `widgets.rs`; panels are outlined, not filled (frame cost).
- Performance: run `frame_bench` and the render bench (`docs/PERFORMANCE.md`) for UI changes; don't regress the late-game frame.
- Do not use or copy from the `spire-slayer` repository.
