# Performance

Target: responsive on one slow core with no GPU (Mesa llvmpipe software rendering).

## Measure

Pin to one core.

```bash
# Layout + tessellation per frame, headless.
# Flags: --frames N, --scenario NAME, --json PATH, --shapes NAME, --components, --no-startup
taskset -c 0 cargo run --release -p spire-app --example frame_bench -- --frames 400

# End to end in a real window (layout, tessellation, GL, present).
LIBGL_ALWAYS_SOFTWARE=1 LP_NUM_THREADS=1 SPIRE_BENCH=idle-late:300 taskset -c 0 cargo run --release -p spire-app

# Frames drawn while idle (should be a small constant).
SPIRE_IDLE_CHECK=8 cargo run --release -p spire-app
```

- `SPIRE_STYLE=default` runs egui's stock style for A/B.
- Under Xvfb: `xvfb-run -a -s "-screen 0 1400x900x24"`.
- Instruction counts: `valgrind --tool=callgrind` on `frame_bench`, differencing two frame counts.
- Shared machines are noisy: compare A/B runs interleaved, not against old numbers.

**Scenarios** (`crates/spire-app/src/bench.rs`): `start`, `idle` (mid act 1), `idle-late` (act 3, 35 cards, 12 relics: the heaviest steady state), `narrow` (900×700), `data-window`, `hover-sweep`, `tooltip`, `scroll`, `typing`, `combo-open`, `resize`, `commit-undo`, `cards`, `enemies`, `analytics`, `history`.

## Results

One 2.1 GHz Xeon core, llvmpipe with one thread, 1400×900. Before: the first build. After: the redesign with StS2 beta data.

| Render bench | Before | After |
|---|---:|---:|
| start | 11.5 ms | 2.1 ms |
| idle | 22.2 ms | 6.2 ms |
| idle-late | 23.2 ms | 6.6 ms |
| data-window | 28.5 ms | 10.1 ms |
| typing | 25.8 ms | 6.8 ms |
| commit-undo | 25.1 ms | 7.5 ms |

| Other | Before | After |
|---|---:|---:|
| `frame_bench` idle-late (median) | 1.44 ms | 0.54 ms |
| Indices, idle-late | 51.1k | 15.4k |
| Instructions per late-game frame | 7.82M | 4.15M |
| Shaded pixels per frame | 1.49M | ~0.23M |
| Binary | 18.0 MB | 10.3 MB |

Startup: data load and analysis 1.9 ms; first headless frame 9.5 ms; launch to first window frame ~55 ms (mostly GL). Peak RSS ~120 MB, mostly Mesa's JIT. Idle: a startup burst of 7–8 frames, then none until input.

## What made it fast

Software rendering is bound by **rasterisation** (shaded pixels, triangle setup), then present; layout is secondary.

1. **No hidden full-screen fill.** Panels are transparent; the clear colour is the background.
2. **No shadows** (large blurred meshes).
3. **Fewer triangles:** square corners, feathering and dithering off. Glyphs keep anti-aliasing (pre-rasterised).
4. **Outlined panels, not filled:** a border costs hundreds of pixels, a fill its area.
5. **No per-frame recomputation:** deck rows group once per run change (`Dashboard::rows`); rule matching is allocation-free (`contains_ci`).
6. **No work for invisible things:** tooltips build inside `on_hover_ui`; off-screen content skips layout via `widgets::culled`, keyed by content.
7. **Cheap widgets:** chips are one galley and one rect; row actions frameless until hovered.
8. **Few frames:** motion is 83 ms fades, off on looks without motion and when the OS asks for reduced motion.
9. **Never block on disk:** saves go to a coalescing background writer.
10. **Build:** fat LTO, one codegen unit, abort on panic, stripped.

## Rules for UI code

- Outline, don't fill. No rounded custom shapes; use `style` and `widgets`.
- Build tooltip contents inside `on_hover_ui`.
- Wrap long lists and below-the-fold sections in `widgets::culled`, keyed by content.
- Derive on run change (`Dashboard`, the combat stage), never per frame.
- Run `frame_bench` and the `idle-late` render bench before merging UI changes.
