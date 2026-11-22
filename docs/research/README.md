# Research

Card-by-card and relic-by-relic notes for the simulator overhaul, from the Spire Codex beta data (v0.113.1). They were written before the overhaul and refreshed after it.

- [CARDS.md](CARDS.md): all 716 cards. For each: what it does, what the simulator reads, what it misses, and a gap code. The gap catalogue groups the codes by the change that would close them.
- [RELICS.md](RELICS.md): all 315 relics, plus the 22 enchantments relics apply, on the same scheme.
- [INTERACTIONS.md](INTERACTIONS.md): first- and second-order interactions per class. Engines are written by hand; the index and chains are generated from a resource model of the card text. Each interaction says whether the simulator can see it.

The coverage marks and "Simulator" lines are regenerated from `cargo run -p spire-data --example audit -- --coverage`. The notes describe each gap as it was before the overhaul; a struck-through code is closed, and the gap tables' *Now* column says what is left.
