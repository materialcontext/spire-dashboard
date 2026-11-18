# Plan: native looks for Windows 11 and macOS

Status: built, except where noted under "What's built" below. None of it
has been tried on a real Windows or Mac yet (P0); the checks so far are CI
builds for both and screenshots of the System looks previewed on Linux.

## What's built

| Area | Windows 11 | macOS |
|---|---|---|
| Default look | **System** (D1) | **System** (D1) |
| Colours | Fluent tokens (§4.3) over the OS accent and its shades; contrast themes take every colour from the theme | `NSColor` semantic colours for the current appearance, and the OS accent |
| Light/dark | Follows the OS live; accent, contrast and transparency are read again when the window regains focus | Same |
| Font | Segoe UI Variable from `C:\Windows\Fonts` (Text optical size, 400/600), Segoe UI on Windows 10, Inter if neither | SF Pro from `/System/Library/Fonts/SFNS.ttf` (400/590), Inter if missing |
| Type ramp | Subtitle 20, Body 14, Caption 12 | Title 3 15, Body 13, Subheadline 11 |
| Shapes | 4px controls, 8px overlays and cards; 32px controls | 6px controls, 10px overlays; 24px controls |
| Panels | Fluent cards (translucent fill, hairline) on the content layer | Grouped content: soft fill, no outline (outlined with Increase Contrast) |
| Window | The app-drawn title bar stays the default (Mica For Everyone made the native one transparent here), with Fluent caption buttons and Mica's solid fallback colour. With **Use the system title bar** on, Mica shows behind a transparent window (`window-vibrancy`) when transparency effects are on and no contrast theme is | Real title bar, transparent over full-size content; traffic lights in place |
| Navigation | NavigationView top mode: text items, 16×3 accent pill | Sidebar with the six tabs; the toolbar row holds the vitals and run buttons |
| Settings | A page of the window (top-right navigation item) | A separate Settings window, opened with ⌘, from the app menu |
| Menus | — | Native menu bar (`muda`): app menu, Edit, View (⌘1–6), Run, Window |
| Dialogs | Smoke behind (#4D000000), 8px corners, shadow | Lighter dim, 10px corners; Cancel leads, the action trails |
| Lists | Rounded hover backplate, accent pill on the selected row | Rounded selection, no hover |
| Screen readers | AccessKit on; custom rows, tabs and nav items report name and selection | Same |

Not built, and why:

- **macOS vibrancy.** `window-vibrancy` adds its blur view as a subview of
  the content view. egui draws into that view's own layer, which sits under
  every subview, so the blur would cover the UI. Doing it properly means
  placing the blur behind the content view, which needs `unsafe`
  Objective-C, and this workspace forbids `unsafe`. The sidebar uses a
  solid tint instead.
- **Windows title bar with content and snap layouts.** Extending content
  into the native title bar needs a window subclass for `WM_NCHITTEST`
  (`unsafe`). The app-drawn title bar remains.
- **Native context menus.** `muda`'s context menus are `unsafe fn`; context
  menus stay egui-drawn, in the platform's shapes and colours.
- **Liquid Glass**, per §1.
- **Per-size tracking and SF's optical size.** Segoe UI Variable gets its
  Text optical size; SF Pro uses its default.

The goal is a look on each platform that a regular Windows or Mac user can't
tell apart from a first-party app: the same fonts, colors, materials, shapes,
sizes, motion, window behavior, menus, dialogs, shortcuts and accessibility
behavior. The current looks (Spirebound, Forge, Neon Spire, Grimoire,
Terminal) stay as themes. On each OS, a new **System** look becomes the
default.

The plan covers:

- what egui can and can't match
- the platform specifications, with sources
- how each part of this app maps onto each platform
- the architecture and the phases
- decisions to make before starting
- facts that still need checking on real machines

Sources are listed at the end. Facts marked **(verify)** are secondhand or
unconfirmed. Each needs a measurement on a real machine before it's coded.

---

## 1. What "seamless" means for an egui app

egui draws every pixel itself. Nothing is a native control, so a native look
has to come from four things:

1. **The platform's own values, read at runtime:** accent color, light or
   dark mode, high contrast, text scale, reduced motion and transparency,
   and (on macOS) the semantic system colors. Hard-coded copies drift with
   every OS update, so anything the OS will report is read, not copied.
2. **The platform's design tokens**, where the OS doesn't report them: type
   ramp, corner radii, control sizes, spacing and motion curves. These are
   taken from Microsoft's open-source WinUI resource files and Apple's Human
   Interface Guidelines.
3. **Real native window chrome and materials** where possible. On Windows that
   means the DWM frame, Mica and snap layouts. On macOS it means the real
   title bar and traffic lights, `NSVisualEffectView` vibrancy and the real
   menu bar.
4. **Platform conventions:** button order, menu structure, shortcut
   modifiers, where Settings lives, how focus and selection look, and what
   Escape and Return do.

### What egui can and can't match

| Area | Can it match? | How, or why not |
|---|---|---|
| System font | Yes | Load Segoe UI Variable (Windows) or SF Pro (macOS) **from the system at runtime**. Never bundle them: both are licensed only as part of their OS. egui 0.37's text stack (skrifa, harfrust, hinting) knows about variation axes. Weight and optical size need a spike (§7, P0). |
| Text rendering | Close, not identical | egui rasterizes glyphs itself. It has hinting, but no ClearType subpixel rendering on Windows and no CoreText rendering on macOS. Text will look slightly softer than in native apps, mostly on low-DPI Windows screens. This is the one gap we can't fully close. |
| Colors and accent | Yes | Read them at runtime (§4, §5). |
| Window frame and caption buttons | Yes | Windows: the native frame, with content extended into the title bar and snap layouts kept. macOS: a full-size content view, a transparent title bar and the real traffic lights. eframe 0.37 already exposes these, including `Frame::set_traffic_lights_position`. |
| Mica and Acrylic (Windows) | Yes, with care | `DwmSetWindowAttribute(DWMWA_SYSTEMBACKDROP_TYPE)` on Windows 11 build 22621 and later, plus a transparent framebuffer. |
| Vibrancy (macOS) | Yes, with care | An `NSVisualEffectView` behind a transparent egui surface. The `window-vibrancy` crate does this for Tauri. |
| Liquid Glass (macOS 26+) | **No** | Refraction and lensing of the content behind a control can't be faked convincingly in an immediate-mode renderer. Apple says to use Liquid Glass sparingly and only for the functional layer. The plan uses the real window material and system colors, and doesn't imitate glass on controls. |
| macOS menu bar | Yes, through a library | A real `NSMenu` via the `muda` crate. egui can't draw the menu bar. |
| Context menus | Either | Draw them with tokens, or use `muda`'s native context menus. Native is more seamless on macOS (§7, P5 decides). |
| Accessibility | Mostly | eframe's `accesskit` feature exposes the UI to Narrator, NVDA and VoiceOver. Every custom widget needs correct roles and labels. |
| Motion | Yes | egui animation, using the platform curves and durations (§4.9). Turned off when the OS asks for reduced motion. |

---

## 2. Targets

- **Windows 11 22H2 and later** (build 22621+). That's the first build with
  the documented backdrop attribute; earlier builds get a solid background.
  Light, dark and contrast themes. Display scaling from 100% to 200%, and the
  Text size setting.
- **macOS 26 Tahoe and macOS 27.** Tahoe introduced Liquid Glass. macOS 27
  refines it:
  - a Liquid Glass slider in Appearance settings
  - more consistent window corners
  - edge-to-edge sidebars and a unified toolbar
  - stronger active-window cues
  - fewer menu icons

  Light and dark mode, all accent colors including Multicolor, Reduce
  Transparency, Increase Contrast, the tinted Liquid Glass setting, and
  sidebar icon size. macOS 15 and earlier get the same look without the
  26-specific shapes. **(verify on a macOS 15 machine before promising
  support)**
- **Linux** keeps the current looks. GNOME and KDE are out of scope.

---

## 3. How this app maps onto each platform

| Part of Spire Dashboard | Windows 11 | macOS |
|---|---|---|
| Window and title bar | Native frame with Mica. Content extends into a **48px** title bar: tabs, the vitals bar and the run buttons sit in it, with tall system caption buttons. Snap layouts are kept. | Full-size content view with a transparent title bar. Traffic lights stay where the system puts them. The tabs and vitals form a unified toolbar row. |
| Tabs (Run, Analytics, History, Cards, Relics, Enemies) | NavigationView in **Top** mode: 48px pane, a text item per tab, and a 3px accent selection indicator. | A **sidebar** (source list), following the user's sidebar size setting, in the Liquid Glass sidebar layer (D4). The vitals bar and run buttons move to the toolbar. |
| Panels (deck, priorities, threats…) | Content layer on Mica: a `LayerFillColorDefault` surface with `CardStrokeColorDefault` 1px strokes. Rounding per the geometry rules (§4.2). | Plain grouped content on the window background. Section headers in the Headline style. No outlined boxes, since macOS rarely draws them. |
| Floor log (right panel) | A second content-layer surface, or a Fluent "pane". | An inspector (trailing sidebar), with the toggle in the toolbar. |
| Settings | A NavigationView settings item (gear, bottom of left mode or far right of top mode) that opens a full Settings page. | **A separate Settings window**, opened with ⌘, from the app menu. Toolbar panes, a title per pane, minimize and zoom dimmed. |
| End run and Abandon dialogs | ContentDialog: 8px radius, smoke behind it. "Do it" buttons on the **left**, Cancel on the right. | Sheet attached to the window. Default button on the **right**, Cancel on the left. Esc and ⌘. cancel. |
| Toast ("Floor 12 logged · Undo") | InfoBar-style banner at the top of the content area. | In-window banner or overlay, without a material effect. |
| Context menu (deck card) | MenuFlyout look: 8px radius, 32px rows. Or native. | A native `NSMenu` via `muda` (recommended). |
| Shortcuts | Ctrl+Z, Ctrl+Enter, Ctrl+1–6, F1 help. Shown in tooltips, e.g. "Commit floor (Ctrl+Enter)". | ⌘Z, ⌘Return, ⌘1–6, ⌘, Settings, ⌘Q, ⌘H, ⌘W, ⌘M. Shown in menu items. |
| Charts | Series in system palette colors on transparent backgrounds. Grid lines in `DividerStrokeColorDefault`. | Series in `NSColor` system colors. Grid lines in `separatorColor`. |
| Role, card-type and character colors | Kept as data colors, desaturated in a contrast theme. The UI chrome itself uses only tokens. | Same. |

**Spirebound's character accent:** native looks use the **system accent**.
Character color stays as a small identity mark next to the character name.
This is decision D2.

---

## 4. Windows 11 specification

Sources: Microsoft's Windows design docs and the WinUI resource files (see
Sources). "epx" means effective pixels: logical pixels at 100% scale.

### 4.1 Typography (Segoe UI Variable)

| Style | Weight | Size / line height |
|---|---|---|
| Caption | Small, Regular | 12 / 16 |
| Body | Text, Regular | 14 / 20 |
| Body Strong | Text, Semibold | 14 / 20 |
| Body Large | Text, Regular | 18 / 24 |
| Subtitle | Display, Semibold | 20 / 28 |
| Title | Display, Semibold | 28 / 36 |
| Title Large | Display, Semibold | 40 / 52 |
| Display | Display, Semibold | 68 / 92 |

- **Minimum legible size:** 12px Regular or 14px Semibold.
- **Weights:** Regular for text, Semibold for titles.
- **Optical size** is automatic in the font (8–36pt). Letter spacing is 0 at
  every size.
- **The window title** uses Caption.
- **Text size setting:** respect Windows' Settings → Accessibility → Text
  size. It scales text only, not layout, and the title bar grows with it.

### 4.2 Geometry

| Radius | Used for |
|---|---|
| **8px** (`OverlayCornerRadius`) | Windows, dialogs, flyouts, menus, TeachingTips |
| **4px** (`ControlCornerRadius`) | Buttons, CheckBox, ComboBox, TextBox, list rows, ProgressBar, ScrollBar, Slider, ToolTip |
| **0px** | Edges meeting other straight edges; snapped or maximized windows |

- **Gutters:** spacing is on a 4px grid. Typical gutters are 8, 12, 16 and
  24, and 16 for page margins. **(verify against current Microsoft spacing
  guidance)**
- **Content area corner:** NavigationView's content area has an **8px**
  top-left corner (`NavigationViewContentGridCornerRadius = 8,0,0,0`).
  Top-mode content has 0.

### 4.3 Color tokens

These are WinUI's theme resources (Fluent 2). Values are ARGB. Dark is
WinUI's "Default" dictionary.

| Token | Light | Dark |
|---|---|---|
| TextFillColorPrimary | #E4000000 | #FFFFFF |
| TextFillColorSecondary | #9E000000 | #C5FFFFFF |
| TextFillColorTertiary | #72000000 | #87FFFFFF |
| TextFillColorDisabled | #5C000000 | #5DFFFFFF |
| ControlFillColorDefault | #B3FFFFFF | #0FFFFFFF |
| ControlFillColorSecondary (hover) | #80F9F9F9 | #15FFFFFF |
| ControlFillColorTertiary (pressed) | #4DF9F9F9 | #08FFFFFF |
| ControlFillColorDisabled | #4DF9F9F9 | #0BFFFFFF |
| ControlFillColorInputActive | #FFFFFF | #B31E1E1E |
| SubtleFillColorSecondary (hover, subtle) | #09000000 | #0FFFFFFF |
| SubtleFillColorTertiary (pressed, subtle) | #06000000 | #0AFFFFFF |
| ControlStrokeColorDefault | #0F000000 | #12FFFFFF |
| ControlStrokeColorSecondary | #29000000 | #18FFFFFF |
| ControlStrongStrokeColorDefault | #72000000 | #8BFFFFFF |
| CardStrokeColorDefault | #0F000000 | #19000000 |
| DividerStrokeColorDefault | #0F000000 | #15FFFFFF |
| SurfaceStrokeColorDefault | #66757575 | #66757575 |
| FocusStrokeColorOuter / Inner | #E4000000 / #B3FFFFFF | #FFFFFF / #B3000000 |
| CardBackgroundFillColorDefault | #B3FFFFFF | #0DFFFFFF |
| CardBackgroundFillColorSecondary | #80F6F6F6 | #08FFFFFF |
| LayerFillColorDefault | #80FFFFFF | #4C3A3A3A |
| LayerFillColorAlt | #FFFFFF | #0DFFFFFF |
| SolidBackgroundFillColorBase (Mica fallback) | #F3F3F3 | #202020 |
| SolidBackgroundFillColorSecondary | #EEEEEE | #1C1C1C |
| SolidBackgroundFillColorTertiary | #F9F9F9 | #282828 |
| SmokeFillColorDefault (behind dialogs) | #4D000000 | #4D000000 |
| SystemFillColorSuccess | #0F7B0F | #6CCB5F |
| SystemFillColorCaution | #9D5D00 | #FCE100 |
| SystemFillColorCritical | #C42B1C | #FF99A4 |

**Accent** comes from the OS: `UISettings.GetColorValue` for `Accent` and its
`AccentLight1–3` and `AccentDark1–3` variants. Live updates come from
`ColorValuesChanged`, backed up by `WM_SETTINGCHANGE`, because the event is
reported to be unreliable. Fluent uses the accent:

- accent fills: **AccentLight2** in dark mode, **AccentDark1** in light mode
- accent text: **AccentLight3** in dark, **AccentDark2** in light

**(verify the exact mapping against the WinUI accent resources)**

The semi-transparent fills only look right over Mica or the solid base
color. Composite them in the right order: base, then layer, then control.

### 4.4 Materials and layering

- **Two layers:**
  - **Base layer:** Mica (or Mica Alt with tabs). It also fills the title
    bar.
  - **Content layer:** a `LayerFillColorDefault` surface on top, as one
    standard surface or as separate cards.
- **Mica:**
  - It's opaque and tinted by the wallpaper.
  - It shows the window's active and inactive state on its own.
  - It falls back to `SolidBackgroundFillColorBase` when unavailable: in
    contrast themes, with transparency effects off, in battery saver, or on
    older builds.
- **Acrylic** is only for transient surfaces such as flyouts and menus.
  Smoke is only behind modal dialogs.
- **Elevation:**

  | Surface | Shadow elevation | Stroke |
  |---|---|---|
  | Window | 128 | 1px |
  | Dialog | 128 | 1px |
  | Flyout | 32 | 1px |
  | ToolTip | 16 | 1px |
  | Card | 8 | 1px |

  egui shadows are cheap if they use one blurred shape. Re-bench them
  (§6.4).
- **Implementation:**
  - `DwmSetWindowAttribute(DWMWA_SYSTEMBACKDROP_TYPE, DWMSBT_MAINWINDOW)`
    gives Mica; `DWMSBT_TABBEDWINDOW` gives Mica Alt.
  - Set `DWMWA_USE_IMMERSIVE_DARK_MODE` (20) to match the theme, and set it
    again whenever the window handle is recreated.
  - Extend the frame into the client area, and clear the framebuffer to
    transparent so the material shows.
  - `DWMWA_WINDOW_CORNER_PREFERENCE` keeps the 8px window rounding.

### 4.5 Title bar

- **Height:** 32px standard. Use **48px** when it holds interactive content
  (tabs, search, a profile), with `PreferredHeightOption = Tall` so the
  caption buttons grow to match.
- **Icon:** 16×16, 16px from the left edge, vertically centered.
- **Title:** in the Caption style, 16px after the icon. It can truncate; the
  icon and caption buttons can't.
- **Background:** Mica, blending into the window. Elements are
  semi-transparent when the window is inactive.
- **Drag area:** every empty or non-interactive area drags the window.
  Right-click opens the system menu, and so does a click on the icon.
- **Caption buttons:** Segoe Fluent Icons glyphs E921 (minimize), E922
  (maximize), E923 (restore) and E8BB (close). Backplates bleed to the edges,
  with rest, hover, pressed and inactive states, and close turns red.
  **Width (verify):** commonly measured at 46px.
- **Snap layouts:** the maximize button must answer `WM_NCHITTEST` with
  `HTMAXBUTTON`, the other buttons `HTMINBUTTON` and `HTCLOSE`, and empty
  areas `HTCAPTION`. That needs a window subclass (`SetWindowSubclass` on the
  window handle). A known winit gotcha: an undecorated winit window keeps a
  1px non-client strip that stops the snap flyout appearing.
- **Relation to today's app-drawn title bar** (PR #9): the native frame is
  better on stock Windows. It brings snap layouts, Mica and system menus.
  Because it uses the real DWM backdrop, it should also work with
  Mica For Everyone, though that needs testing. Keep the app-drawn bar as
  the fallback setting. This is decision D3.

### 4.6 Controls

| Control | Metrics (WinUI resources) |
|---|---|
| Button | 32px tall: padding 11,5,11,6 around Body 14/20, plus a 1px stroke. 4px radius. Rest `ControlFillColorDefault`, hover `…Secondary`, pressed `…Tertiary`. The pressed text goes Secondary. Accent buttons use the accent fill with `TextOnAccentFillColorPrimary`. |
| CheckBox | 20px box with a 12px glyph. The row is 32px tall with 8px padding after the box. |
| RadioButton | 20px; glyph 12 (14 on hover, 10 when pressed). |
| ToggleSwitch | 40×20 track **(verify)**. Minimum width 154 including the header. |
| TextBox, ComboBox | 32px tall, 4px radius. The bottom border turns accent and 2px when focused **(verify)**. Inner button margin 0,4,4,4. |
| Slider | 32px row, 4px track, 18px thumb with a 12px inner dot. Thumb radius 10. |
| ListView row | At least 40px tall, 4px radius backplate. Selection shows a **3×16px accent pill** on the left. |
| NavigationView (left) | 36px items, 40px icon box, items inset 4,2. Selection indicator **3×16**, accent. |
| NavigationView (top) | **48px** pane. Item margins 8/12. The selection indicator is a horizontal pill. |
| MenuFlyout | 8px radius, Acrylic, item padding 11,8,11,9 (32px rows), 4px inset. 1px separators. |
| ContentDialog | 320–548px wide, 184–756px tall, 24px padding. Title in Subtitle style with 12px below it. |
| ScrollBar | 12px. Thin (2px) at rest, widening on hover **(verify)**. Thumb at least 30px. Radius 3. |
| ToolTip | 4px radius, Body or Caption text. **(verify the delay)** |
| Focus visual | 2px outer stroke (`FocusStrokeColorOuter`) and 1px inner stroke, following the control's corner radius plus its margin. Shown for keyboard focus only. |

### 4.7 Dialogs, menus and conventions

- **Dialog buttons:** "do it" buttons (primary, secondary) on the **left**;
  the safe Close or Cancel button on the **right**. One button can be the
  default, shown with the accent style. Esc cancels and Enter accepts the
  default.
- **Titles:** short, describing the decision. Button labels name the action
  ("End run", not "OK").
- **Menus vs context menus:** menus for command buttons; context menus
  (right-click) for secondary commands on content. Rows use sentence case.
- **Shortcuts:** use common accelerators, and put them in tooltips
  ("Copy (Ctrl+C)"). Alt shows access keys. F1 is help.

### 4.8 Contrast themes and accessibility

- **Contrast themes:** in a contrast theme, replace every token with a
  `SystemColor*` pair:
  - Window and WindowText
  - Hotlight for links
  - GrayText for disabled
  - Highlight and HighlightText for selection, hover and pressed
  - ButtonFace and ButtonText

  Drop Mica, shadows and translucency. Read the colors from
  `GetSysColor` or `UISettings`.
- **Text scaling:** respect Windows' Text size setting.
- **Narrator and NVDA:** AccessKit.

### 4.9 Motion

| Purpose | Curve | Duration |
|---|---|---|
| Direct entrance (fast in) | cubic-bezier(0,0,0,1) | 167 / 250 / 333 ms |
| Point to point | cubic-bezier(0.55,0.55,0,1) | 167 / 250 / 333 ms |
| Direct exit (fast out, with fade) | cubic-bezier(0,0,0,1) | 167 ms |
| Gentle exit | cubic-bezier(1,0,1,1) | 167 ms |
| Fade only | linear | 83 ms |

When Windows' animation effects are off (`SPI_GETCLIENTAREAANIMATION`), all
motion turns off.

---

## 5. macOS specification

Sources: Apple's Human Interface Guidelines (fetched from Apple's HIG data,
change logs up to June 2026), WWDC25 sessions, and community measurements
where Apple publishes no numbers.

### 5.1 Typography (SF Pro, loaded from the system)

| Style | Weight | Size / line height (pt) | Emphasized |
|---|---|---|---|
| Large Title | Regular | 26 / 32 | Bold |
| Title 1 | Regular | 22 / 26 | Bold |
| Title 2 | Regular | 17 / 22 | Bold |
| Title 3 | Regular | 15 / 20 | Semibold |
| Headline | Bold | 13 / 16 | Heavy |
| **Body** | Regular | **13 / 16** | Semibold |
| Callout | Regular | 12 / 15 | Semibold |
| Subheadline | Regular | 11 / 14 | Semibold |
| Footnote | Regular | 10 / 13 | Semibold |
| Caption 1 | Regular | 10 / 13 | Medium |
| Caption 2 | Medium | 10 / 13 | Semibold |

- **Default size** is 13pt; the minimum is 10pt. macOS has no Dynamic Type.
- **Tracking varies by size.** For example:

  | Size | Tracking |
  |---|---|
  | 10pt | +12/1000 em |
  | 12pt | 0 |
  | 13pt | −6/1000 em (−0.08pt) |
  | 15pt | −16/1000 em |
  | 17pt | −26/1000 em |
  | 20pt | −23/1000 em |
  | 22pt | −12/1000 em |

  egui has no per-style tracking. Option: apply it through a per-size
  letter-spacing tweak, if the font API allows it **(spike)**.
- **Text and Display optical sizes** blend between about 17 and 28pt. SF Pro
  is variable, so use its optical-size axis if egui's variation support
  exposes it **(spike)**.
- **Licensing:** load the font from `/System/Library/Fonts/` at runtime.
  Never ship it. **(verify Apple's terms for runtime use by non-AppKit
  renderers)**

### 5.2 Window

- **Anatomy:** a frame (title bar and toolbar) plus the body. The window
  drags by its frame and resizes from its edges.
- **States:** main, key and inactive. Inactive windows lose their material
  effects and their colored traffic lights. A custom-drawn UI must follow
  this: dim accents and drop vibrancy when the window isn't key.
- **Corner radius:**
  - **macOS 26 (verify):** about 16pt for plain title-bar windows and about
    26pt for toolbar windows.
  - **macOS 27:** Apple made window corners more consistent, so measure on
    27.

  The OS clips the window, so the app only needs to match inner shapes
  ("concentric" corners in toolbars).
- **Unified toolbar:** the toolbar sits in the frame, with items unbezeled.
  In macOS 27, apps get a unified top toolbar and sidebars run edge to edge.
- **Implementation:** eframe `ViewportBuilder`:
  - `with_fullsize_content_view(true)`
  - `with_titlebar_shown(false)` for a transparent title bar
  - `with_title_shown(false)`

  Keep the traffic lights, and position them with
  `Frame::set_traffic_lights_position`. Reserve a drag region for the
  toolbar.

### 5.3 Materials

- **Liquid Glass** is the floating functional layer: toolbar, sidebar and
  controls. It comes in regular and clear variants. Don't put it in the
  content layer.
- **Settings that change it:** the macOS 26.1 Clear/Tinted setting, the
  macOS 27 transparency slider, and Reduce Transparency and Increase
  Contrast.
- **Vibrancy:** `NSVisualEffectView` materials (sidebar, window background,
  menu, popover, HUD), blending behind the window or within it.
- **Plan:**
  - Put a sidebar or under-window `NSVisualEffectView` behind the egui
    surface.
  - Draw egui content transparent where the material should show.
  - Don't imitate Liquid Glass on controls.
  - Inactive windows and Reduce Transparency switch to opaque
    `windowBackgroundColor`.

### 5.4 Color

- **Read the semantic colors at runtime** from `NSColor` (objc2-app-kit),
  resolved against the window's `effectiveAppearance`:
  - `windowBackgroundColor`, `controlBackgroundColor`
    `underPageBackgroundColor`
  - `labelColor` and secondary, tertiary and quaternary label colors
  - `separatorColor`, `gridColor`
  - `controlAccentColor`, `keyboardFocusIndicatorColor`
  - `selectedContentBackgroundColor`,
    `unemphasizedSelectedContentBackgroundColor`
  - `selectedTextBackgroundColor`, `linkColor`, `findHighlightColor`
  - the system palette: systemRed, Orange, Yellow, Green, Mint, Teal, Cyan,
    Blue, Indigo, Purple, Pink, Brown and Gray

  Apple's published color tables are images, not values, so read the colors
  rather than hard-coding them.
- **Accent:** the user's accent replaces the app accent unless the setting
  is **Multicolor**. In that case the app's own accent applies; for us,
  Spirebound's character color is a natural app accent (decision D2).
- **Live updates:** watch `effectiveAppearance` and
  `NSSystemColorsDidChangeNotification`.

### 5.5 Controls

- **Sizes:** mini, small, regular and large, plus **extra large** (new in
  macOS 26). Smaller sizes are rounded rectangles; larger ones are capsules.
  AppKit offers a "previous macOS" density for dense layouts.
- **Push buttons:** the default button uses the accent fill and responds to
  Return. Cancel responds to Esc and ⌘. and is never the default. Buttons
  can be tinted.
- **Measurements (verify):**
  - regular push button height, on 26 and 27
  - checkbox, radio, switch, pop-up, text field and segmented control sizes
  - sidebar row heights (small, medium, large, following the user's sidebar
    icon size setting)

  Under Liquid Glass, checkboxes and radios are reported to be slightly
  larger. Measure these from Apple's macOS 26 design resources (Sketch or
  Figma) or from `intrinsicContentSize` in a probe app.
- **Focus:** keyboard focus rings use `keyboardFocusIndicatorColor`.
  Focused list rows use the accent background with white text. Unfocused
  selections use the unemphasized gray.
- **Scroll bars** follow the user's "Show scroll bars" setting: overlay
  (hidden until scrolling) or always shown.

### 5.6 Menus, Settings and dialogs

- **Menu bar** (via `muda`):
  - **App menu,** in order: About Spire Dashboard; then Settings… (⌘,);
    Services; Hide (⌘H), Hide Others (⌥⌘H), Show All; Quit (⌘Q).
  - **Other menus:**
    - **Edit:** Undo ⌘Z
    - **View:** the tabs, ⌘1–6
    - **Window:** Minimize ⌘M, Zoom
    - **Help:** a search field is included automatically

  Menu items use title case.
- **Settings:** a separate window opened with ⌘,. It has a non-customizable
  toolbar of panes that always marks the active one, a title that names the
  pane ("Spire Dashboard Settings" if there's only one), minimize and zoom
  dimmed, and it reopens the last pane. No Settings button in the main
  toolbar.
- **Alerts and sheets:**
  - The default button goes on the **trailing** (right) side, Cancel on the
    leading side.
  - The button titles are verbs, with "OK" only for purely informational
    alerts.
  - A destructive action always comes with a Cancel button.
  - Esc and ⌘. cancel.
  - The app icon shows automatically.
  - Use a sheet for document-modal questions such as End run and Abandon.
- **Shortcuts:** don't repurpose the standard ones. Map today's Ctrl
  shortcuts to ⌘; egui's `Modifiers::COMMAND` already does this.

### 5.7 Accessibility

- **VoiceOver:** through AccessKit.
- **Full Keyboard Access:** Tab reaches every control, with focus rings.
- **System settings to follow:** Reduce Motion, Reduce Transparency,
  Increase Contrast and Differentiate Without Color. Read them from
  `NSWorkspace`'s accessibility display properties.

---

## 6. Architecture

### 6.1 Tokens instead of the look table

Today's `LookDef` holds a palette and a few flags. Split it into a token set
that every widget reads:

- **color:** the semantic roles. Text: primary, secondary, tertiary,
  disabled, on-accent. Fills: control (rest, hover, pressed, disabled),
  subtle, layer and card. Strokes: control, card, divider and focus. Plus
  accent and its variants, status (success, caution, critical) and
  selection.
- **type:** the ramp per platform (§4.1 or §5.1), with family, size, line
  height, weight and tracking.
- **shape:** control radius and overlay radius. On macOS, radius depends on
  control size.
- **metrics:** control heights, paddings, list row heights, title bar or
  toolbar height, gutters and the focus ring.
- **motion:** curves and durations, plus a "reduced" flag.
- **material:** none, Mica, Mica Alt or vibrancy, plus the opaque fallback.

The existing looks become token sets too: Spirebound, Forge and the others.
The **System** look is built at runtime from §4 or §5, plus the OS values.

### 6.2 A platform module (`spire-app/src/platform/`)

- **`windows.rs`:** DWM attributes (backdrop, dark mode, corners), the
  window subclass for `WM_NCHITTEST` (snap layouts), the accent and
  `UISettings` colors, contrast-theme colors, text scale, animation
  settings, and change notifications.
- **`macos.rs`:** `NSColor` semantic colors, appearance observers, the
  `NSVisualEffectView` setup, accessibility display settings, and the native
  menu bar and Settings window plumbing.
- **`mod.rs`:** a `SystemAppearance` value (mode, accent, contrast, reduced
  motion and transparency, text scale, fonts) that the style layer turns
  into tokens. Polled when the OS says something changed, never per frame.

### 6.3 Widgets

Add a platform-aware widget layer in `widgets/`: button (standard, accent,
subtle), checkbox, radio, toggle, text field, combo and pop-up, segmented
control, list row with selection, navigation item, card or section,
dialog/sheet, menu, tooltip, scroll bar style and focus ring. Each widget is
one paint function per platform, driven by tokens. Views keep calling the
same helpers they call today, so the views themselves barely change.

### 6.4 Performance

- **Transparent framebuffer.** Mica and vibrancy need one, so the GPU
  composites the window. Measure the late-game frame with it on and off.
  Fall back to an opaque background in battery saver, on software GL, or
  when the user turns transparency off.
- **Filled cards.** Fluent cards are filled (layer fill), which goes against
  our "outlined, not filled" rule from the software-renderer work. Re-bench
  with the render bench. Fills are cheap on a GPU, and the rule mainly
  protects software GL.
- **Hot path.** Token lookups stay cheap: plain struct reads, as the palette
  accessors are now.

### 6.5 Dependencies (each needs your approval)

| Crate | Why |
|---|---|
| `windows-sys` or `windows` | DWM, `UISettings`, window subclass, system colors |
| `objc2`, `objc2-app-kit`, `objc2-foundation` | macOS colors, appearance, vibrancy, accessibility settings. eframe already uses objc2 on macOS, so this is probably already in the dependency tree. |
| `muda` | Native macOS menu bar and native context menus |
| `window-vibrancy` (optional) | Ready-made `NSVisualEffectView` setup; could be replaced by about 50 lines of objc2 |
| eframe `accesskit` feature | Screen readers |

---

## 7. Phases (each phase is its own PR)

| Phase | Work | Done when |
|---|---|---|
| **P0 Spikes** | On real Windows 11 and macOS 26/27 machines: load the system fonts and compare text against native apps in screenshot overlays; try Mica with an extended frame and snap layouts on an eframe window; try a full-size content view with vibrancy and traffic lights; measure the macOS controls marked (verify); try variable-font weight and optical size; check AccessKit with NVDA and VoiceOver. | A go/no-go for each item, with screenshots. Anything that fails gets a documented fallback. |
| **P1 Tokens** | Refactor the style into tokens (§6.1). No visual change. | The current looks render identically, checked by screenshot diffs, and the bench doesn't regress. |
| **P2 System appearance** | The platform module: mode, accent, contrast, text scale, reduced motion and transparency, with live updates. | Changing a setting in the OS updates the app without a restart. |
| **P3 Typography** | System fonts, the platform type ramps and tracking. | Text overlays within a pixel of native at 100% and 200%. |
| **P4 Window and materials** | Windows: native frame, Mica, 48px title bar with content, snap layouts. macOS: full-size content, transparent title bar, vibrancy, inactive-window states. | Passes the window checklist (§8). |
| **P5 Controls** | The widget layer (§6.3), all states including keyboard focus. Native context menus decided here. | Side-by-side with WinUI 3 Gallery and AppKit controls in light and dark: states, sizes and colors match. |
| **P6 Conventions** | Navigation pattern (D4), Settings placement (page or window), dialog/sheet button order, menu bar, shortcut display. | Passes the conventions checklist (§8). |
| **P7 Accessibility** | AccessKit roles and labels, contrast themes, Increase Contrast, keyboard-only use, reduced motion. | Full app use with NVDA and VoiceOver; the contrast theme passes. |
| **P8 Polish** | Motion curves, tooltips, empty states, sentence case (Windows) vs title case (macOS) in labels, icons. Icons are Segoe Fluent Icons on Windows (a system font) and SF Symbols on macOS **(verify that runtime use outside AppKit is allowed)**. | Reviewed on both platforms by someone who uses that OS every day. |

## 8. Acceptance checklists

**Window:**
- Snap layouts appear when hovering maximize.
- Aero Snap and drag-to-edge work.
- Double-clicking the title bar maximizes; right-click shows the system menu.
- The inactive state dims the title bar.
- Corners are right in every state: rounded or square, maximized, full screen.
- Mica follows the wallpaper. On macOS, vibrancy follows the window.
- The traffic lights hover and dim correctly, and the window shadow matches
  native.

**Visual (per platform, light and dark, two accents, contrast mode):**
- Every control state matches the reference app.
- No color, radius or size is off the platform's tokens.
- Text overlays are within 1px.

**Conventions:**
- Dialog button order and the Esc and Return behavior.
- Shortcuts use the platform modifier and appear where the platform shows
  them.
- Settings lives in the platform's place.
- Labels use the platform's capitalization.
- macOS has a full menu bar.

**Accessibility:**
- Screen reader names every control.
- Tab order is logical.
- Focus is always visible from the keyboard.
- Reduced motion, Reduce Transparency, Increase Contrast and text scale are
  all honored.

**Test machines:**
- Windows 11 24H2 and 25H2, at 100%, 150% and 200%, with Mica For Everyone
  both installed and not.
- macOS 26 and 27, on Retina and non-Retina displays, at each sidebar size.

## 9. Decisions (made)

- **D1. Default look:** **System** becomes the default on Windows and macOS.
  The current looks stay as themes.
- **D2. Accent:** System looks use the system accent. The Spirebound
  character color stays as an identity mark, and serves as the app accent
  when macOS is set to Multicolor.
- **D3. Windows title bar:** the native frame with content in it (snap
  layouts, real Mica) is the default. The app-drawn bar from PR #9 stays as
  a setting. Still subject to the P0 spike confirming it works alongside
  Mica For Everyone.
- **D4. macOS navigation:** **sidebar** for the six tabs. Windows uses
  NavigationView Top in the title bar.
- **D5. Dependencies:** approved: `windows`/`windows-sys`, `objc2` and
  `objc2-app-kit`/`objc2-foundation`, `muda`, optionally `window-vibrancy`,
  and eframe's `accesskit` feature.
- **D6. Native menus:** a native macOS menu bar and native context menus
  through `muda`. Windows stays egui-drawn with Fluent tokens.

See also [DESIGN_PRACTICES.md](DESIGN_PRACTICES.md) for the general lessons
of this research, applied to the existing looks.

## 10. Still to verify on real machines

- macOS 26 and 27: control heights, sidebar row heights, window corner radii,
  and the toolbar height with and without a title.
- Windows: caption button width, focus-visual details, the scroll bar's
  rest and hover widths, the TextBox focus underline, and the exact
  accent-variant mapping.
- egui: per-style tracking, and whether the optical-size axis can be set.
- Licensing: runtime use of SF Pro, SF Symbols and Segoe Fluent Icons from
  the system by a non-native renderer.

## Sources

Microsoft:
- [Geometry](https://github.com/MicrosoftDocs/windows-dev-docs/blob/docs/hub/apps/design/signature-experiences/geometry.md)
- [Typography](https://github.com/MicrosoftDocs/windows-dev-docs/blob/docs/hub/apps/design/signature-experiences/typography.md), [Fluent 2 typography](https://fluent2.microsoft.design/typography)
- [Color](https://github.com/MicrosoftDocs/windows-dev-docs/blob/docs/hub/apps/design/signature-experiences/color.md)
- [Layering and elevation](https://github.com/MicrosoftDocs/windows-dev-docs/blob/docs/hub/apps/design/signature-experiences/layering.md)
- [Materials](https://github.com/MicrosoftDocs/windows-dev-docs/blob/docs/hub/apps/design/signature-experiences/materials.md), [Mica](https://github.com/MicrosoftDocs/windows-dev-docs/blob/docs/hub/apps/design/style/mica.md), [Acrylic](https://github.com/MicrosoftDocs/windows-dev-docs/blob/docs/hub/apps/design/style/acrylic.md)
- [Motion](https://github.com/MicrosoftDocs/windows-dev-docs/blob/docs/hub/apps/design/signature-experiences/motion.md)
- [Title bar design](https://github.com/MicrosoftDocs/windows-dev-docs/blob/docs/hub/apps/design/basics/titlebar-design.md), [Title bar customization](https://github.com/MicrosoftDocs/windows-dev-docs/blob/docs/hub/apps/develop/title-bar.md), [Snap layouts](https://learn.microsoft.com/en-us/windows/apps/desktop/modernize/ui/apply-snap-layout-menu)
- [Buttons](https://github.com/MicrosoftDocs/windows-dev-docs/blob/docs/hub/apps/develop/ui/controls/buttons.md), [NavigationView](https://github.com/MicrosoftDocs/windows-dev-docs/blob/docs/hub/apps/develop/ui/controls/navigationview.md), [Dialogs](https://github.com/MicrosoftDocs/windows-dev-docs/blob/docs/hub/apps/develop/ui/controls/dialogs-and-flyouts/dialogs.md), [Menus](https://github.com/MicrosoftDocs/windows-dev-docs/blob/docs/hub/apps/develop/ui/controls/menus-and-context-menus.md)
- [Keyboard accelerators](https://github.com/MicrosoftDocs/windows-dev-docs/blob/docs/hub/apps/develop/input/keyboard-accelerators.md), [Contrast themes](https://github.com/MicrosoftDocs/windows-dev-docs/blob/docs/hub/apps/design/accessibility/high-contrast-themes.md), [Segoe Fluent Icons](https://github.com/MicrosoftDocs/windows-dev-docs/blob/docs/hub/apps/design/iconography/segoe-fluent-icons-font.md)
- [DWMWINDOWATTRIBUTE](https://learn.microsoft.com/en-us/windows/win32/api/dwmapi/ne-dwmapi-dwmwindowattribute), [DWM_SYSTEMBACKDROP_TYPE](https://learn.microsoft.com/en-us/windows/win32/api/dwmapi/ne-dwmapi-dwm_systembackdrop_type)
- WinUI theme resources, `winui2/main` branch: [Common](https://github.com/microsoft/microsoft-ui-xaml/blob/winui2/main/dev/CommonStyles/Common_themeresources_any.xaml), [Button](https://github.com/microsoft/microsoft-ui-xaml/blob/winui2/main/dev/CommonStyles/Button_themeresources.xaml), [NavigationView](https://github.com/microsoft/microsoft-ui-xaml/blob/winui2/main/dev/NavigationView/NavigationView_rs1_themeresources.xaml), and the CheckBox, ToggleSwitch, ListViewItem, MenuFlyout, ScrollBar, ContentDialog, Slider and RadioButton files in the same folder.
- [Segoe UI font family](https://learn.microsoft.com/en-us/typography/font-list/segoe-ui) (licensing), [UISettings.GetColorValue](https://learn.microsoft.com/en-us/uwp/api/windows.ui.viewmanagement.uisettings.getcolorvalue)

Apple (Human Interface Guidelines):
- [Typography](https://developer.apple.com/design/human-interface-guidelines/typography), [Color](https://developer.apple.com/design/human-interface-guidelines/color), [Materials](https://developer.apple.com/design/human-interface-guidelines/materials)
- [Windows](https://developer.apple.com/design/human-interface-guidelines/windows), [Toolbars](https://developer.apple.com/design/human-interface-guidelines/toolbars), [Sidebars](https://developer.apple.com/design/human-interface-guidelines/sidebars)
- [Buttons](https://developer.apple.com/design/human-interface-guidelines/buttons), [Alerts](https://developer.apple.com/design/human-interface-guidelines/alerts), [The menu bar](https://developer.apple.com/design/human-interface-guidelines/the-menu-bar), [Settings](https://developer.apple.com/design/human-interface-guidelines/settings), [Keyboards](https://developer.apple.com/design/human-interface-guidelines/keyboards), [Focus and selection](https://developer.apple.com/design/human-interface-guidelines/focus-and-selection)
- WWDC25: [Build an AppKit app with the new design](https://developer.apple.com/videos/play/wwdc2025/310/), [Build a SwiftUI app with the new design](https://developer.apple.com/videos/play/wwdc2025/323/)
- macOS 27 changes: [9to5Mac](https://9to5mac.com/2026/09/14/macos-27-golden-gate-now-available-here-is-everything-new/), [Macworld](https://www.macworld.com/article/3139330/macos-27-mac-features-siri-apple-intelligence-release-date-compatibility.html), [AppleMagazine](https://applemagazine.com/macos-27-liquid-glass/amp/)
- Window corners on macOS 26 (community measurements): [mac-window-corners](https://github.com/mansarip/mac-window-corners), [Zed discussion](https://github.com/zed-industries/zed/discussions/38233)
- Liquid Glass settings in 26.1: [Eclectic Light](https://eclecticlight.co/2025/11/05/appearance-revisited-get-tahoe-26-1-looking-in-better-shape/)

Rust and egui:
- eframe 0.37 source (`ViewportBuilder` macOS options, `Frame::set_traffic_lights_position`, the `accesskit` feature), [egui issue #3050](https://github.com/emilk/egui/issues/3050) (vibrancy), [window-vibrancy](https://crates.io/crates/window-vibrancy)
- Accent color crates: [mundy](https://docs.rs/mundy/latest/mundy/struct.AccentColor.html), [dynamic_color](https://github.com/H2Sxxa/dynamic_color)
- [winit dark-mode detection issue](https://github.com/rust-windowing/winit/issues/4161)
- Snap layouts with a winit window: [Studio PR #440](https://github.com/Metallic-Club/studio/pull/440)
