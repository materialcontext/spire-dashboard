# Design rules

Lessons from Fluent (Windows 11) and Apple's HIG, applied to every look (Spirebound, Forge, Neon Spire, Grimoire, Terminal, System). System-look specifics: [NATIVE_LOOK.md](NATIVE_LOOK.md). Cost rules: [PERFORMANCE.md](PERFORMANCE.md).

| # | Rule | How we apply it |
|---|---|---|
| 1 | Hierarchy from layering, not boxes | Outline only top-level panels; sub-sections are headings plus space. Surfaces are lighter than the background in every look (Grimoire: raised paper on a darker desk) |
| 2 | Accent means "you can act here" | `accent` only for primary buttons, selection, focus, links, action callouts. Data emphasis by weight. Character colour is `identity` (band, name) |
| 3 | A named type ramp | Caption, Body, Body Strong, Subtitle, Title, each with size and line height; Larger text scales the whole ramp. Sentence-case headings; caps only in Terminal (`caps_headings`). Minimum 12 px |
| 4 | Contrast tiers | Text, dim, faint (≥ 4.5:1 for small text), disabled (`disabled_alpha` 0.4). Input and checkbox outlines use the strong border (≥ 3:1); panel outlines may be decorative |
| 5 | Every state, keyboard focus included | Hover +1 step; pressed one step further, label dimmed; a 2 px focus ring (`chrome::focus_ring`) only after keyboard navigation |
| 6 | Honest buttons and dialogs | One accent action per view; verbs, never OK/Yes; destructive actions outlined in `bad`, never the default; Esc cancels; Undo over confirmation |
| 7 | 4 px spacing grid | `style::space`: `xs` 4, `s` 8, `m` 12, `l` 16, `xl` 24. Square corners in themed looks |
| 8 | 24 × 24 px hit targets (WCAG 2.2) | Small glyphs get full-size hit areas; the right-click menu is mentioned in hints |
| 9 | Brief, optional motion | 83 ms fades, 167 ms moves, only while something changes; off in Terminal and with OS reduced motion; data changes never animate |
| 10 | Writing | Sentence case (System look on macOS: title-case buttons and menus); ellipsis on commands that open UI ("Settings…"); shortcuts in tooltips via `widgets::shortcut` |
| 11 | Show window focus | The header dims when the window loses focus |
| 12 | Follow system settings | Match system light/dark, Larger text from OS text size, reduced motion |

## Per look

| Look | Keep | Watch |
|---|---|---|
| Spirebound | Character band, square corners | Accent only for interactive items |
| Forge | Ember palette | Lowest border contrast: inputs use the strong border |
| Neon Spire | Glow, top-level panels only | Pink only on interactive items |
| Grimoire | Parchment and crimson | Light look: hover and pressed darken; faint ≥ 4.6:1 |
| Terminal | Monospace, caps headings | Motion off |

Sources: as in [NATIVE_LOOK.md](NATIVE_LOOK.md#sources), plus WCAG 2.2 target size (2.5.8) and non-text contrast (1.4.11).
