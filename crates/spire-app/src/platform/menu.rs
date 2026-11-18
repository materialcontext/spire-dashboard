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

pub use imp::MenuBar;

#[cfg(target_os = "macos")]
mod imp {
    use std::sync::mpsc::{Receiver, channel};

    use muda::accelerator::Accelerator;
    use muda::{AboutMetadata, Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};

    use super::{Command, Enabled};

    pub struct MenuBar {
        /// Kept alive: the menu bar lives as long as this.
        _menu: Menu,
        items: Vec<(MenuItem, Command)>,
        events: Receiver<MenuEvent>,
        enabled: Option<Enabled>,
    }

    fn item(text: &str, accel: &str) -> MenuItem {
        MenuItem::new(text, true, accel.parse::<Accelerator>().ok())
    }

    impl MenuBar {
        /// Installs the menu bar on the running app. `wake` is called when
        /// a menu item is picked, so the UI can repaint and handle it.
        /// Call once, on the main thread, after the app has launched.
        pub fn install(tabs: &[&str], wake: impl Fn() + Send + Sync + 'static) -> Option<MenuBar> {
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
            Some(MenuBar { _menu: menu, items, events: rx, enabled: None })
        }

        /// Commands picked since the last call.
        pub fn poll(&self) -> Vec<Command> {
            self.events
                .try_iter()
                .filter_map(|e| self.items.iter().find(|(i, _)| *i.id() == e.id).map(|(_, c)| *c))
                .collect()
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
    use super::{Command, Enabled};

    /// No native menu bar off macOS.
    pub struct MenuBar;

    impl MenuBar {
        pub fn install(_tabs: &[&str], _wake: impl Fn() + Send + Sync + 'static) -> Option<MenuBar> {
            None
        }

        pub fn poll(&self) -> Vec<Command> {
            Vec::new()
        }

        pub fn sync(&mut self, _enabled: Enabled) {}
    }
}
