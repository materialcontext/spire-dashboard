# Native looks: Windows 11 and macOS

The **System** look (default on both) follows the platform: tokens from Microsoft's WinUI resources and Apple's HIG, values read from the OS at runtime where it reports them (accent, appearance, contrast, text size, motion, transparency). Token values live in `crates/spire-app/src/style/system.rs`, commented with their source names. Untested on real Windows and Mac hardware: checks so far are CI builds for both and Linux previews (`SPIRE_SYSTEM=windows|macos`).

## What's built

| Area | Windows 11 | macOS |
|---|---|---|
| Default look | **System** (D1) | **System** (D1) |
| Colours | Fluent tokens over the OS accent and its shades; contrast themes take every colour from the theme | `NSColor` semantic colours for the current appearance, and the OS accent |
| Light/dark | Follows the OS live; accent, contrast and transparency are read again when the window regains focus | Same |
| Font | Segoe UI Variable from `C:\Windows\Fonts` (Text optical size, 400/600), Segoe UI on Windows 10, Inter if neither | SF Pro from `/System/Library/Fonts/SFNS.ttf` (400/590), Inter if missing |
| Type ramp | Subtitle 20, Body 14, Caption 12 | Title 3 15, Body 13, Subheadline 11 |
| Shapes | 4px controls, 8px overlays and cards; 32px controls | 6px controls, 10px overlays; 24px controls |
| Panels | Fluent cards (translucent fill, hairline) on the content layer | Grouped content: soft fill, no outline (outlined with Increase Contrast) |
| Window | Snap layouts on the maximise button. The app-drawn title bar stays the default (Mica For Everyone made the native one transparent here), with Fluent caption buttons and Mica's solid fallback colour. With **Use the system title bar** on, Mica shows behind a transparent window (`window-vibrancy`) when transparency effects are on and no contrast theme is | Real title bar, transparent over full-size content; traffic lights in place; the sidebar material behind the sidebar |
| Navigation | NavigationView top mode: text items, 16×3 accent pill | Sidebar with the six tabs; the toolbar row holds the vitals and run buttons |
| Settings | A page of the window (top-right navigation item) | A separate Settings window, opened with ⌘, from the app menu |
| Menus | — | Native menu bar (`muda`): app menu, Edit, View (⌘1–6), Run, Window; native context menus |
| Dialogs | Smoke behind (#4D000000), 8px corners, shadow | Lighter dim, 10px corners; Cancel leads, the action trails |
| Lists | Rounded hover backplate, accent pill on the selected row | Rounded selection, no hover |
| Screen readers | AccessKit on; custom rows, tabs and nav items report name and selection | Same |

Built with `unsafe`, in the `spire-platform` crate only (`window.rs`,
`menu.rs`), each block with its safety argument:

- **macOS sidebar material.** An `NSVisualEffectView` (Sidebar material,
  following the window's active state) is placed in the window's frame
  view, *below* egui's content view; a subview of the content view would be
  drawn over egui's layer. The window is transparent, the sidebar is left
  clear, and the content and toolbar are painted with the window
  background. Off with Reduce Transparency.
- **Windows snap layouts** on the app-drawn title bar. A window subclass
  answers `WM_NCHITTEST` with `HTMAXBUTTON` over the app's maximise button,
  so Windows 11 shows the snap layout flyout there. It also maximises or
  restores on click and reports hover and press back for drawing, since
  Windows routes the pointer there instead of to the app.
- **Native context menus** on macOS (the deck card menu), through `muda`.

Not built, and why:

- **Content in the native Windows title bar.** The app-drawn title bar
  already holds the tabs and vitals. With snap layouts it has what the
  native frame offered, without the transparent-title-bar problem that
  Mica For Everyone caused here.
- **Mica behind the app-drawn title bar.** Mica behind an undecorated
  window isn't documented, and if it failed the window would show the
  desktop through it. Mica comes with the system title bar.
- **Liquid Glass**: egui draws its own pixels; the material can't be matched.
- **Per-size tracking and SF's optical size.** Segoe UI Variable gets its
  Text optical size; SF Pro uses its default.

## Decisions

| # | Decision |
|---|---|
| D1 | System is the default look on Windows and macOS; the other looks stay as themes |
| D2 | System looks use the OS accent; the character colour stays an identity mark (and the accent when macOS is set to Multicolor) |
| D3 | Windows title bar: the app-drawn bar is the default, with snap layouts and Fluent caption buttons (Mica For Everyone made the native frame transparent); **Use the system title bar** brings the native frame with Mica |
| D4 | macOS: sidebar for the six tabs; Windows: NavigationView top mode |
| D5 | Approved dependencies: `windows`/`windows-sys`, `objc2` (+ `app-kit`, `foundation`), `muda`, `window-vibrancy`, eframe `accesskit` |
| D6 | Native macOS menu bar and context menus via `muda`; Windows menus egui-drawn with Fluent tokens |

General lessons applied to every look: [DESIGN_PRACTICES.md](DESIGN_PRACTICES.md).

## Still to verify on real machines

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
