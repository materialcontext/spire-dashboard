//! The native macOS menu bar (through `muda`). Elsewhere there is none:
//! Windows apps of this kind keep their commands in the window.
//!
//! Menus follow Apple's order: the app menu (About, Settings…, Services,
//! Hide, Hide Others, Show All, Quit), then Edit, View, Run, Window. Item
//! titles use title case. The menu reports commands; the app applies them.

/// What a menu item asks the app to do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    Settings,
    Undo,
    /// Show the n-th tab (0-based).
    Tab(usize),
    NewRun,
    EndRun,
    CommitFloor,
}

/// Which commands can run right now (the menu greys out the rest).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Enabled {
    pub undo: bool,
    pub new_run: bool,
    pub end_run: bool,
    pub commit: bool,
}

/// One entry of a context menu.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Entry {
    /// A command; picking it returns its index among all items, in order.
    Item(String),
    /// Greyed-out text, e.g. why something isn't available.
    Note(String),
    Submenu(String, Vec<Entry>),
    Separator,
}

/// Whether native context menus are on (macOS, once the menu bar is in).
pub fn native_context_menus() -> bool {
    NATIVE.load(std::sync::atomic::Ordering::Relaxed)
}

static NATIVE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub use imp::MenuBar;

#[cfg(target_os = "macos")]
#[allow(unsafe_code)]
mod imp {
    use std::sync::mpsc::{Receiver, channel};

    use muda::accelerator::Accelerator;
    use muda::{AboutMetadata, ContextMenu, IsMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};
    use objc2::rc::Retained;
    use objc2_app_kit::NSView;
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};

    use super::{Command, Enabled, Entry};

    pub struct MenuBar {
        /// Kept alive: the menu bar lives as long as this.
        _menu: Menu,
        items: Vec<(MenuItem, Command)>,
        events: Receiver<MenuEvent>,
        /// Menu-bar events that came in while a context menu was up.
        pending: Vec<MenuEvent>,
        enabled: Option<Enabled>,
        /// The window's view, for context menus.
        view: Option<Retained<NSView>>,
    }

    /// The window's content view, retained.
    fn view_of(window: &impl HasWindowHandle) -> Option<Retained<NSView>> {
        let handle = window.window_handle().ok()?;
        let RawWindowHandle::AppKit(appkit) = handle.as_raw() else { return None };
        // SAFETY: an AppKit handle's `ns_view` is the window's live NSView;
        // retaining it keeps it valid for as long as we hold it.
        unsafe { Retained::retain(appkit.ns_view.as_ptr().cast::<NSView>()) }
    }

    /// Adds `entries` to a context menu, numbering items as it goes.
    fn add(target: &dyn Fn(&dyn IsMenuItem) -> bool, entries: &[Entry], next: &mut usize) {
        for e in entries {
            match e {
                Entry::Item(label) => {
                    let item = MenuItem::with_id(format!("ctx-{next}"), label, true, None);
                    *next += 1;
                    target(&item);
                }
                Entry::Note(label) => {
                    target(&MenuItem::new(label, false, None));
                }
                Entry::Separator => {
                    target(&PredefinedMenuItem::separator());
                }
                Entry::Submenu(label, children) => {
                    let sub = Submenu::new(label, true);
                    add(&|i| sub.append(i).is_ok(), children, next);
                    target(&sub);
                }
            }
        }
    }

    fn item(text: &str, accel: &str) -> MenuItem {
        MenuItem::new(text, true, accel.parse::<Accelerator>().ok())
    }

    impl MenuBar {
        /// Installs the menu bar on the running app. `wake` is called when
        /// a menu item is picked, so the UI can repaint and handle it.
        /// Call once, on the main thread, after the app has launched.
        pub fn install(
            tabs: &[&str],
            window: &impl HasWindowHandle,
            wake: impl Fn() + Send + Sync + 'static,
        ) -> Option<MenuBar> {
            let ns_view = view_of(window);
            let menu = Menu::new();
            let mut items: Vec<(MenuItem, Command)> = Vec::new();

            let about = AboutMetadata {
                name: Some("Spire Dashboard".into()),
                version: Some(env!("CARGO_PKG_VERSION").into()),
                ..Default::default()
            };
            let settings = item("Settings…", "Cmd+Comma");
            let app = Submenu::new("Spire Dashboard", true);
            app.append_items(&[
                &PredefinedMenuItem::about(None, Some(about)),
                &PredefinedMenuItem::separator(),
                &settings,
                &PredefinedMenuItem::separator(),
                &PredefinedMenuItem::services(None),
                &PredefinedMenuItem::separator(),
                &PredefinedMenuItem::hide(None),
                &PredefinedMenuItem::hide_others(None),
                &PredefinedMenuItem::show_all(None),
                &PredefinedMenuItem::separator(),
                &PredefinedMenuItem::quit(None),
            ])
            .ok()?;
            items.push((settings, Command::Settings));

            let undo = item("Undo", "Cmd+KeyZ");
            let edit = Submenu::new("Edit", true);
            edit.append_items(&[
                &undo,
                &PredefinedMenuItem::separator(),
                &PredefinedMenuItem::cut(None),
                &PredefinedMenuItem::copy(None),
                &PredefinedMenuItem::paste(None),
                &PredefinedMenuItem::select_all(None),
            ])
            .ok()?;
            items.push((undo, Command::Undo));

            let view = Submenu::new("View", true);
            for (n, name) in tabs.iter().enumerate() {
                let tab = item(name, &format!("Cmd+Digit{}", n + 1));
                view.append(&tab).ok()?;
                items.push((tab, Command::Tab(n)));
            }
            view.append_items(&[&PredefinedMenuItem::separator(), &PredefinedMenuItem::fullscreen(None)]).ok()?;

            let new_run = item("New Run…", "");
            let end_run = item("End Run…", "");
            let commit = item("Commit Floor", "Cmd+Enter");
            let run = Submenu::new("Run", true);
            run.append_items(&[&commit, &PredefinedMenuItem::separator(), &end_run, &new_run]).ok()?;
            items.extend([(commit, Command::CommitFloor), (end_run, Command::EndRun), (new_run, Command::NewRun)]);

            let window = Submenu::new("Window", true);
            window
                .append_items(&[
                    &PredefinedMenuItem::minimize(None),
                    &PredefinedMenuItem::maximize(None),
                    &PredefinedMenuItem::separator(),
                    &PredefinedMenuItem::bring_all_to_front(None),
                ])
                .ok()?;

            menu.append_items(&[&app, &edit, &view, &run, &window]).ok()?;
            menu.init_for_nsapp();
            window.set_as_windows_menu_for_nsapp();

            let (tx, rx) = channel();
            MenuEvent::set_event_handler(Some(move |e: MenuEvent| {
                let _ = tx.send(e);
                wake();
            }));
            super::NATIVE.store(ns_view.is_some(), std::sync::atomic::Ordering::Relaxed);
            Some(MenuBar { _menu: menu, items, events: rx, pending: Vec::new(), enabled: None, view: ns_view })
        }

        /// Commands picked since the last call.
        pub fn poll(&mut self) -> Vec<Command> {
            let events: Vec<MenuEvent> = self.pending.drain(..).chain(self.events.try_iter()).collect();
            events
                .into_iter()
                .filter_map(|e| self.items.iter().find(|(i, _)| *i.id() == e.id).map(|(_, c)| *c))
                .collect()
        }

        /// Shows a native context menu at the pointer and waits for it to
        /// close. Returns the index of the item picked, counting every
        /// `Entry::Item` in order, submenus included.
        pub fn context_menu(&mut self, entries: &[Entry]) -> Option<usize> {
            let view = self.view.as_ref()?;
            let menu = Menu::new();
            add(&|i| menu.append(i).is_ok(), entries, &mut 0);
            let ptr = Retained::as_ptr(view).cast::<std::ffi::c_void>();
            // SAFETY: `ptr` is the window's NSView, kept alive by the
            // `Retained` in `self.view`; the menu runs modally on the main
            // thread (where the app calls this) and returns when it closes.
            let picked = unsafe { menu.show_context_menu_for_nsview(ptr, None) };
            let mut chosen = None;
            for e in self.events.try_iter() {
                match e.id.0.strip_prefix("ctx-").and_then(|n| n.parse::<usize>().ok()) {
                    Some(n) if picked => chosen = Some(n),
                    Some(_) => {}
                    None => self.pending.push(e),
                }
            }
            chosen
        }

        /// Greys out what can't run now. Cheap when nothing changed.
        pub fn sync(&mut self, enabled: Enabled) {
            if self.enabled == Some(enabled) {
                return;
            }
            self.enabled = Some(enabled);
            for (i, c) in &self.items {
                let on = match c {
                    Command::Undo => enabled.undo,
                    Command::NewRun => enabled.new_run,
                    Command::EndRun => enabled.end_run,
                    Command::CommitFloor => enabled.commit,
                    Command::Settings | Command::Tab(_) => true,
                };
                i.set_enabled(on);
            }
        }
    }
}

#[cfg(not(target_os = "macos"))]
mod imp {
    use raw_window_handle::HasWindowHandle;

    use super::{Command, Enabled, Entry};

    /// No native menu bar off macOS.
    pub struct MenuBar;

    impl MenuBar {
        pub fn install(
            _tabs: &[&str],
            _window: &impl HasWindowHandle,
            _wake: impl Fn() + Send + Sync + 'static,
        ) -> Option<MenuBar> {
            None
        }

        pub fn poll(&mut self) -> Vec<Command> {
            Vec::new()
        }

        pub fn context_menu(&mut self, _entries: &[Entry]) -> Option<usize> {
            None
        }

        pub fn sync(&mut self, _enabled: Enabled) {}
    }
}
