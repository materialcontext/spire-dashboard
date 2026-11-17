# Performance

The goal is to stay responsive on minimal hardware: a single slow core with
no GPU, where OpenGL falls back to software rendering (Mesa llvmpipe).

## How to measure

Three tools ship with the app. Run them pinned to one core.

```bash
# Layout (egui UI pass) and paint (tessellation) per frame, headless.
taskset -c 0 cargo run --release -p spire-app --example frame_bench -- --frames 400
# Also: --scenario NAME, --json PATH, --shapes NAME (vertices and shaded
# pixels per shape kind), --components (recompute/save costs), --no-startup.

# End-to-end frames in a real window: layout + tessellation + GL + present.
LIBGL_ALWAYS_SOFTWARE=1 LP_NUM_THREADS=1 SPIRE_BENCH=idle-late:300 \
  taskset -c 0 cargo run --release -p spire-app

# Frames drawn while idle (should stay a small constant).
SPIRE_IDLE_CHECK=8 cargo run --release -p spire-app
```

`SPIRE_STYLE=default` runs the same code with egui's stock style, for A/B
comparisons. Under Xvfb, wrap the window runs in `xvfb-run -a -s "-screen 0
1400x900x24"`. Instruction counts come from `valgrind --tool=callgrind` on
`frame_bench`, differencing two frame counts so that setup cancels out.

### Scenarios (`crates/spire-app/src/bench.rs`)

| Scenario | What changes each frame |
|---|---|
| `start` | Character select, nothing |
| `idle` | Mid act 1, 12 cards: the cost of any repaint |
| `idle-late` | Act 3, 35 cards, 12 relics: the heaviest steady state |
| `narrow` | Late run at 900×700 (single column) |
| `data-window` | Late run with the Data window open |
| `hover-sweep` | Pointer moving across the deck and threat panels |
| `tooltip` | Pointer resting on a card until its tooltip shows |
| `scroll` | Mouse wheel scrolling the dashboard |
| `typing` | A card name typed into search, one key per frame |
| `combo-open` | Encounter dropdown opened by a click |
| `resize` | Window width changing every frame |
| `commit-undo` | Commit a floor, then undo, alternating |

## Results

Hardware: one core of a 2.1 GHz Xeon, Mesa llvmpipe with one thread,
1400×900 window. "Before" is the first build of the dashboard; "after"
includes the redesign and the StS2 beta data (716 cards).

### End to end (render bench, frame time)

| Scenario | Before | After | Speedup |
|---|---:|---:|---:|
| start | 11.54 ms (84 fps) | 2.09 ms (467 fps) | 5.5× |
| idle | 22.18 ms (43 fps) | 6.17 ms (163 fps) | 3.6× |
| idle-late | 23.23 ms (41 fps) | 6.57 ms (147 fps) | 3.5× |
| narrow | 13.05 ms | 3.64 ms | 3.6× |
| data-window | 28.49 ms | 10.09 ms | 2.8× |
| typing | 25.84 ms | 6.82 ms | 3.8× |
| resize | 25.73 ms | 6.33 ms | 4.1× |
| commit-undo | 25.05 ms | 7.47 ms | 3.4× |

### CPU: layout + tessellation (frame_bench, median)

| Scenario | Before | After | Indices before → after |
|---|---:|---:|---:|
| idle | 0.81 ms | 0.32 ms | 46.5k → 14.7k |
| idle-late | 1.44 ms | 0.54 ms | 51.1k → 15.4k |
| hover-sweep | 1.13 ms | 0.59 ms | 51.2k → 15.4k |
| tooltip | 1.18 ms | 0.58 ms | 52.7k → 15.9k |
| scroll | 1.18 ms | 0.55 ms | 55.4k → 16.0k |
| typing | 1.17 ms | 0.61 ms | 51.9k → 15.5k |
| combo-open | 1.14 ms | 0.57 ms | 53.6k → 15.9k |
| resize | 2.18 ms | 0.76 ms | 51.0k → 15.4k |
| commit-undo | 2.08 ms | 1.25 ms | 51.1k → 15.4k |

Other numbers:

| Measure | Before | After |
|---|---:|---:|
| Instructions per late-game frame | 7.82M | 4.15M |
| Shaded pixels per frame | 1.49M (1.18× window) | ~0.23M |
| Binary size | 18.0 MB | 10.3 MB |

Startup:

| Measure | Value |
|---|---:|
| Data load and analysis | 1.9 ms |
| First frame, headless | 17.1 → 9.5 ms |
| Launch to first frame, window | ~55 ms (mostly GL and window setup) |

Peak RSS is about 120 MB, most of it Mesa's LLVM JIT; a real GPU driver
uses much less. Idle frames: a fixed startup burst of 7–8, then none until
input.

## Where the time went, and what fixed it

On software rendering the frame is dominated by **rasterisation**: shaded
pixels and per-triangle setup, then present. Layout matters on slow CPUs but
was never the main cost.

1. **A full-screen fill hidden in every frame.** egui panels fill their area
   on top of the cleared framebuffer, which shades every pixel twice. Panels
   are now transparent and the clear colour is the background: start screen
   11.5 → 2.1 ms, dashboard 22 → 11 ms.
2. **Shadows** on windows and popups are large blurred meshes. Removed: data
   window 27 → 14 ms.
3. **Triangles.** Rounded corners are triangle fans, and anti-alias
   feathering adds a ring of triangles around every shape. The UI is now
   square-cornered with feathering off. Text keeps its anti-aliasing,
   because glyphs are pre-rasterised. Dithering is off too: there are no
   gradients, so it was pure per-pixel shader cost. Dashboard 11 → 7.4 ms.
4. **Outlined panels, not filled.** The redesign keeps that rule: a border
   costs a few hundred pixels, a fill its whole area.
5. **Per-frame recomputation.** The deck panel re-classified every card
   against ~60 role rules each frame, lowercasing the text for every rule.
   That was about 40% of the frame's instructions. Rows are now grouped once
   per run change (`Dashboard::rows`), and rule matching is
   allocation-free (`contains_ci`).
6. **Work for invisible things.**
   - Tooltip text is built only while hovered (`on_hover_ui`).
   - Content scrolled out of view skips layout and reserves its last size
     (`widgets::culled`). It is keyed by content, so a commit only re-lays
     out what changed.
7. **Cheaper widgets.**
   - Chips paint one galley and one rect instead of a `Frame` with a child
     `Ui`.
   - Row actions are frameless until hovered (that alone was ~70 fills per
     frame).
8. **Fewer frames.** Animations are off, so a hover change costs one repaint
   instead of a fade's worth.
9. **Never block on disk.** Run saves go to a coalescing background writer;
   slow storage can't stall the UI.
10. **Build.** Fat LTO, one codegen unit, abort on panic, stripped: 10% fewer
    instructions per frame and a 43% smaller binary.

## Rules for new UI code

- No filled panels or large fills; outline instead.
- No rounded corners on custom shapes; use the `style` palette and
  `widgets` components.
- Build tooltip contents inside `on_hover_ui`, never eagerly.
- Wrap long lists and below-the-fold sections in `widgets::culled`, keyed by
  content.
- Never compute per frame what can be derived when the run changes; put it
  in `Dashboard`.
- Re-run `frame_bench` and the `idle-late` render bench before merging UI
  changes.
