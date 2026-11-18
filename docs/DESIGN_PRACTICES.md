# Design practices from the native-look research, applied to our looks

A companion to [NATIVE_LOOK.md](NATIVE_LOOK.md). That plan adds System looks
per OS. This report takes the general lessons behind Fluent (Windows 11) and
Apple's Human Interface Guidelines, and applies them to the looks we already
have: Spirebound, Forge, Neon Spire, Grimoire and Terminal, plus the Larger
text option.

**Status: implemented.** Every change below shipped with this report. The
one exception is per-style tracking, which is limited to titles (the
`widgets::title` helper) until the native-look P0 spike settles egui's font
API.

Each section gives:

- **Principle:** what both design systems agree on, with the source.
- **Today:** what our looks do, measured where possible.
- **Apply:** concrete changes. Each one keeps the looks' character and
  respects the performance rules in `PERFORMANCE.md`.

The priority list is at the end.

---

## 1. Hierarchy comes from layering, not from boxes

**Principle.** Fluent builds hierarchy from two layers: a base, and a content
layer of surfaces with a hairline stroke (§4.4 of the plan). In both light and
dark, *darker means less important* and lighter surfaces come forward. Apple
mostly groups with spacing and headings, and rarely draws boxes. Both
avoid outlining everything.

**Today.**
- Every section is an outlined panel, and the panels sit side by side. The
  surface-to-background contrast is 1.08–1.15:1 in every look, which is
  fine.
- **Grimoire, the light look, inverts the rule.** Its surface (228,214,186)
  is *darker* than its background (238,228,206), so panels read as recessed
  rather than raised.

**Apply.**
- **Fewer boxes.** Keep outlines for the few top-level groups: Act, Next
  fight, Deck breakdown. Show sub-sections (Priorities, Fought this run, the
  stat rows) as headings plus spacing. Fewer outlines means fewer shapes, so
  this also helps performance.
- **Grimoire:** make surfaces lighter than the background (raised paper on a
  darker desk). For example, background 228,216,190 and surface 242,234,216.
- **Dark looks:** keep surfaces lighter than the background. They already
  are.

## 2. Use the accent sparingly, and give it a meaning

**Principle.** Both systems keep the accent for meaning: primary actions,
selection, focus and links. Fluent says "used sparingly to highlight
important elements". Apple says to reserve it for elements that truly
benefit from emphasis, and not to color the backgrounds of several controls.

**Today.** The accent does many jobs at once:
- primary buttons, selected tabs and segments
- the gold number, ascension text and the boss callout title and outline
- link underlines and chips
- the Spirebound color band

As a result, it no longer signals "you can act here".

**Apply.**
- **Give the accent one meaning: interactive or selected.** That means the
  primary button, the selected tab or segment, focus, links and the callout
  that needs action.
- **Show data emphasis with weight and size, not color.** Gold, ascension
  level and floor become Body Strong in the text color. Gold can keep a
  *gold* tint, because that's the thing's own color, not the accent.
- **Spirebound's character color** stays as an identity mark (the band and
  the character name). Selection and buttons use the look's accent, so the
  accent never changes meaning between characters. This matches decision
  D2.

## 3. A real type ramp

**Principle.** Both systems define a small, named ramp, with size, line
height and weight per role. Fluent: Caption 12/16, Body 14/20, Body Strong
14/20 semibold, Subtitle 20/28, Title 28/36. Apple: Body 13/16, Headline
13/16 bold, Title 3 15/20, Title 2 17/22. Both set minimums: Fluent 12px
Regular or 14px Semibold, Apple 10pt. Titles are Semibold rather than Bold.
Apple tightens tracking as sizes grow (−6/1000 em at 13pt, −23 at 20pt).

**Today.**
- Each look has three sizes: heading, body, small. Most are 20, 15 and
  12.5; Terminal is 18, 14 and 12.
- Weight comes only from the "strong" family.
- No line heights are defined.
- 41 places use uppercase "eyebrow" labels (DECK, RELICS, NEXT FIGHT).

**Apply.**
- **Define a ramp** in the style layer: Caption, Body, Body Strong,
  Subtitle, Title, each with a size and line height. Base it on the Fluent
  ratios, scaled to Inter (body 14 or 15). Larger text scales the whole ramp,
  not single sizes.
- **Replace the uppercase eyebrows with sentence-case Body Strong headings**
  in every look except Terminal, where caps are part of the style. Neither
  system uses all-caps section labels: caps read slower, and translate and
  scale badly.
- **Tighten tracking on titles.** Inter's own guidance matches Apple's: about
  −1 to −2% at 20px and above. Only if egui's font API allows per-size
  letter spacing; see the plan's P0 spike.
- **Keep minimums:** nothing smaller than 12px. Most small text today is
  12.5, which is fine.

## 4. Contrast tiers that match their jobs

**Principle.** Fluent uses three text tiers plus disabled:

| Tier | Alpha | Use |
|---|---|---|
| Primary | 89% | Main text |
| Secondary | 62% | Supporting text |
| Tertiary | 45% | Least important text |
| Disabled | 36% | Disabled |

Contrast floors are WCAG 4.5:1 for small text and 3:1 for control
boundaries; both systems point to WCAG. Contrast themes and Increase Contrast
replace the palette entirely.

**Today.** Measured contrast against each look's background:

| Look | Text | Dim | Faint | Border | Strong border |
|---|---|---|---|---|---|
| Spirebound | 17.4 | 11.1 | 6.9 | 2.1 | 3.7 |
| Forge | 16.8 | 10.3 | 5.3 | 1.7 | 3.3 |
| Neon Spire | 17.5 | 10.7 | 5.3 | 2.4 | 5.3 |
| Grimoire | 13.6 | 7.5 | **4.3** | 2.4 | 4.8 |
| Terminal | 16.4 | 9.6 | 4.8 | 2.2 | 5.3 |

**Apply.**
- **Grimoire's faint text fails AA** at 4.3:1. Darken it to at least 4.6:1,
  e.g. (110,88,66).
- **Input and checkbox outlines use the strong border** (≥3:1, like Fluent's
  `ControlStrongStroke`). Plain panel outlines can stay low-contrast, since
  they're decorative.
- **Add a Disabled tier** (about 40% of text) so disabled controls stop
  borrowing the faint color. Today a disabled button and a hint label look
  the same.
- **High contrast.** The Larger text option already lifts faint to dim. A
  separate **High contrast** option (or following the OS setting) should
  switch to a pure palette: text on background, one link color, one
  highlight pair, no tints, and strong borders everywhere.

## 5. Every interactive state, including keyboard focus

**Principle.** Fluent defines rest, hover, pressed, disabled and focused for
every control, with small, consistent steps:

- hover: about +4% fill
- pressed: less fill and secondary-tier text
- focused: a 2px outer ring plus a 1px inner ring, **shown only for keyboard
  focus**

Apple shows focus with an accent ring, and the accent background for focused
selections.

**Today.**
- Hover changes the fill to `surface_hi`. Pressed looks the same as hover.
- There's no visible keyboard-focus ring. The new arrow-key pickers move a
  highlight, but Tab focus isn't drawn.

**Apply.**
- **Pressed state:** one step lighter than hover in dark looks and one step
  darker in light looks, with the label in the dim color.
- **Keyboard focus ring:** 2px in the text color, 1px inside it in the
  background color, offset by 2px, following the widget's corners. Drawn
  only after Tab or arrow navigation, never after a mouse click. egui knows
  which widget has focus and whether the last input was the keyboard.
- **Selected vs focused:** a selected row gets an accent bar or pill on its
  left edge, like Fluent's 3×16 pill. A focused row gets the ring. The two
  can combine.

## 6. Button hierarchy and honest dialogs

**Principle.** Both systems:

- **one prominent (accent) action per view**, plus standard buttons and
  subtle (frameless) buttons for tertiary actions
- **buttons named by verb**, never OK or Yes for a decision
- **the safe default.** Apple: never make a destructive action the default
  button; always offer Cancel; Esc and ⌘. cancel. Fluent: the safe action
  is required, and Enter triggers only the marked default.
- **undo is better than a confirmation**, and alerts are used sparingly.

**Today.**
- Commit floor is the one accent button in the log, which is right.
- **The Abandon dialog makes "Abandon and start a new run" the accent
  primary button**, and "Keep playing" is dim. That's backwards: the
  destructive action looks like the recommended one.
- The End run dialog lists Victory as primary. It's an intentional choice
  rather than a destructive one, so that's acceptable.

**Apply.**
- **Abandon dialog:**
  - "Keep playing" becomes the default: standard style, answers Enter and
    Esc.
  - "Abandon run" gets a destructive style: the bad color as an outline and
    text, not a fill.
  - The buttons follow the platform order. Under System looks, "do it" goes
    left on Windows and Cancel goes leading on macOS. Themed looks follow
    the OS they run on.
- **Header buttons:** End run, New run and Undo are standard buttons;
  Settings becomes subtle. Only one accent action is visible at a time.
- **Keep Undo over confirmation:** the commit toast with Undo already
  follows this. Don't add confirmations for actions Undo can reverse.

## 7. Spacing on a 4px grid

**Principle.** Fluent spaces everything on a 4px grid (8, 12, 16, 24
gutters) and nests rounded shapes concentrically (inner radius = outer
radius minus padding). Apple says to space controls so each one is clearly
separate and easy to hit.

**Today.** Ten different gaps are used: 4, 6, 8, 10, 12, 14, 16, 18, 24 and
28. The most common are 8 (34 uses), 4 (20) and 6 (14).

**Apply.**
- **Spacing tokens:** `xs` 4, `s` 8, `m` 12, `l` 16, `xl` 24. Replace the
  6, 10, 14, 18 and 28 gaps with their nearest token. The rhythm becomes
  visibly more even, and Larger text can scale the tokens instead of single
  gaps.
- **Rounding:** if a look ever adds corners, nest them concentrically.
  Today's square corners are a deliberate, cheap identity (feathering is
  off, and rounded corners would alias). Keep them square in the themed
  looks.

## 8. Hit targets

**Principle.**
- **Apple:** enough space around every control to tell it apart and hit it.
  Its control sizes start at mini 28pt for the touch shape grid. Mac
  controls are smaller, but never cramped.
- **Fluent:** 32px controls.
- **WCAG 2.2:** at least **24×24 px** per target.

**Today.**
- The deck row "+" and "x" buttons and the toast "x" are small-text
  frameless buttons, roughly 14–18px wide.
- The default interact height is 24px (28 with Larger text).

**Apply.**
- **Row actions:** keep the small visuals, but make each hit area at least
  24×24 (allocate the larger rectangle and paint the small glyph centered).
- **Card context menu:** it's the full set of actions, so mention it in the
  deck hint and in tooltips ("Right-click for more").

## 9. Motion: brief, purposeful, optional

**Principle.** Fluent uses short motion: 83ms fades, and 167, 250 or 333ms
moves with fast-in and fast-out curves. Things that appear and disappear the
same way should animate the same way. Apple ties motion to Reduce Motion.
Both use motion only to explain change.

**Today.** No animations at all. That's a performance choice: an idle app
should draw no frames.

**Apply.**
- **Optional micro-motion** that runs only while something changes, with no
  idle cost:
  - the toast fades in and out (83ms)
  - menus and popups fade in (83ms)
  - the selected-tab underline slides (167ms, cubic-bezier(0,0,0,1))
- **Off by default** in Terminal. Off whenever the OS asks for reduced
  motion.
- **Never animate data changes**, such as charts redrawing after a commit.
  Instant is clearer.

## 10. Writing style

**Principle.**
- **Fluent:** sentence case everywhere.
- **Apple:** title case in menus and buttons, sentence case in body text.
- **Both:**
  - short labels that name the result ("End run", not "OK")
  - shortcuts shown in tooltips or menus ("Commit floor (Ctrl+Enter)")
  - an ellipsis on commands that open more UI ("Settings…")

**Today.**
- Labels are mostly sentence case and verb-led, which is good.
- Shortcuts appear in one hint line in the log, but not in tooltips.
- Some panel notes are long sentences.

**Apply.**
- **Tooltips with shortcuts:** add them to Commit, Undo and the tabs
  ("Analytics (Ctrl+2)").
- **Ellipsis:** "Settings…" and "End run…", since both open a window or
  dialog.
- **Case:** follow the OS. Sentence case on Windows and Linux; title case
  for buttons and menus on macOS under the System look. Themed looks stay in
  sentence case.
- **Shorter notes:** trim the boss-callout and empty-state notes to one
  line each.

## 11. Window and focus state

**Principle.** Both platforms show which window is active. Fluent makes
title-bar elements semi-transparent when the window is inactive, and Mica
dims. Apple grays the traffic lights and drops materials. The cue is
small, but people rely on it.

**Today.** The app looks identical whether or not its window has focus.

**Apply.** When the window loses focus, dim the header: tabs, vitals and the
Spirebound band move to the dim tier. Content stays as it is. This is one
check of egui's focus state, so it costs nothing per frame.

## 12. Respect the person's system settings

**Principle.** Both systems follow the user's choices: light or dark mode,
accent, text size, contrast, reduced motion and reduced transparency. Apps
that ignore them feel foreign even when they look good.

**Today.** The look and Larger text are chosen in Settings only.

**Apply.**
- **Light/dark auto-pick:** a "Match system" option that uses Grimoire when
  the OS is in light mode and Spirebound when it's in dark, with the pair
  chosen in Settings.
- **Larger text:** start with it on when the OS text size is above 100%.
- **Reduced motion:** honor the OS setting (§9).

(Detection comes from the plan's P2 phase, which this can share.)

---

## Per-look summary

| Look | Keep | Change |
|---|---|---|
| **Spirebound** | Character band and name color; square corners | Accent only for interactive or selected items (§2); sentence-case headings (§3); focus ring (§5) |
| **Forge** | Ember palette | Border contrast is the lowest (1.7:1), so raise input outlines to the strong border (§4); same accent and heading changes |
| **Neon Spire** | Glow edges | Keep the glow only on the top-level panels, as the one hierarchy cue (§1); accent at 6.1:1 is fine; limit the pink to interactive items |
| **Grimoire** | Parchment and crimson | **Raise surfaces above the background** (§1); **faint text to ≥4.6:1** (§4); light-look pressed and hover states darken rather than lighten |
| **Terminal** | Monospace and caps headings (stylistic) | Focus ring; disabled tier; motion off by default |
| **Larger text** | Scales sizes and lifts faint text | Scale the whole ramp and spacing tokens together (§3, §7); turn on automatically from the OS text size (§12) |

## Priority

| # | Change | Size | Why first |
|---|---|---|---|
| 1 | Abandon dialog: safe default, destructive style (§6) | Small | Prevents an accidental run loss |
| 2 | Grimoire faint-text contrast (§4) | Tiny | Accessibility failure |
| 3 | Keyboard focus ring and pressed state (§5) | Small | The arrow-key work made keyboard use real; focus must be visible |
| 4 | 24px hit areas on row actions (§8) | Small | Misclicks on + and x |
| 5 | Spacing tokens (§7) | Medium | Even rhythm; prepares the token refactor (plan P1) |
| 6 | Type ramp and sentence-case headings (§3) | Medium | Biggest readability gain; shares work with plan P3 |
| 7 | Accent discipline (§2) and fewer boxes (§1) | Medium | Clearer "what can I click"; fewer shapes per frame |
| 8 | Grimoire layering (§1) | Small | Light look reads correctly |
| 9 | Shortcut tooltips, ellipses, inactive-window dimming (§10, §11) | Small | Polish |
| 10 | Match-system and OS text size and motion (§12), micro-motion (§9) | Medium | Builds on plan P2 |

Items 1–4 could ship together in one small PR. Items 5–7 belong with the
token refactor (plan P1), since they change the same code.

Sources: as in [NATIVE_LOOK.md](NATIVE_LOOK.md#sources), plus the WCAG 2.2
target-size (2.5.8) and non-text-contrast (1.4.11) success criteria.
