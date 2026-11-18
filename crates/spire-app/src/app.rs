//! App state and the single place where actions change it.

use std::sync::mpsc::{Receiver, channel};

use eframe::egui;
use spire_core::analysis::Dashboard;
use spire_core::analysis::sim::{DeckSim, simulate};
use spire_core::analysis::survival::{self, Outlook};
use spire_core::analysis::timeline::{Timeline, timeline};
use spire_core::run::{Change, Outcome, RunEvent, RunState};
use spire_core::{Catalog, Playbook, Run};
use spire_data::fetch::{self, Source};
use spire_data::store::{RunSaver, Settings, Store};

use crate::actions::Action;
use crate::draft::NodeDraft;
use crate::views;

/// Read-only data every view gets.
pub struct ViewCtx<'a> {
    pub catalog: &'a Catalog,
    pub playbook: &'a Playbook,
    pub state: &'a RunState,
    pub dash: &'a Dashboard,
    pub can_undo: bool,
    /// Bumped whenever `dash` is recomputed; keys layout caches.
    pub generation: u64,
    /// Run analytics, computed only while the Analytics tab is open.
    pub analytics: Option<&'a Analytics>,
    /// The deck simulation and fight outlooks, once built for this state.
    pub combat: Option<&'a Combat>,
    /// The run follows the game (monitored): floors log themselves.
    pub following: bool,
}

/// The live deck's simulation and how it fares against this act's elites
/// and bosses. Built lazily (a few milliseconds) for the tabs that show it.
pub struct Combat {
    /// The dashboard generation it was built for. It can lag the run by a
    /// moment (see `prepare_tab`); keys layout caches.
    pub generation: u64,
    pub sim: DeckSim,
    pub outlooks: Vec<(spire_core::EncounterId, Outlook)>,
}

impl Combat {
    pub fn outlook(&self, id: &spire_core::EncounterId) -> Option<&Outlook> {
        self.outlooks.iter().find(|(e, _)| e == id).map(|(_, o)| o)
    }
}

/// The live run's charts and deck simulation. Costlier than the dashboard
/// (a few milliseconds), so built lazily for the Analytics tab.
pub struct Analytics {
    pub timeline: Timeline,
    pub sim: DeckSim,
}

/// UI-only state that never touches the run.
pub struct UiState {
    pub threat_tab: spire_core::run::NodeKind,
    pub show_data: bool,
    pub new_character: Option<spire_core::CharacterId>,
    /// Unset until picked: every run asks for its ascension.
    pub new_ascension: Option<u32>,
    pub source_url: String,
    pub messages: Vec<String>,
    pub updating: bool,
    /// "Abandon this run?" confirmation is open.
    pub confirm_abandon: bool,
    /// A short-lived notice after a commit, with an Undo button.
    pub toast: Option<Toast>,
    /// Which top-level tab is showing.
    pub tab: crate::views::Tab,
    pub cards: crate::views::cards::CardBrowser,
    pub relics: crate::views::relics::RelicBrowser,
    pub enemies: crate::views::enemies::EnemyBrowser,
    /// The End run dialog is open, with the fight to blame for a death.
    pub end_run_open: bool,
    pub end_encounter: Option<spire_core::EncounterId>,
    pub history: crate::views::history::HistoryBrowser,
    /// Analytics: list every card's draw odds, not just the top ones.
    pub show_all_odds: bool,
    /// Monitored runs: what the companion mod reports.
    pub follow: crate::follow::Status,
}

pub struct Toast {
    pub text: String,
    /// `ctx.input().time` when it appeared (for the fade-in).
    pub since: f64,
    /// `ctx.input().time` after which it disappears.
    pub until: f64,
}

/// Seconds the run must stay unchanged before the deck simulation behind
/// the fight outlooks is rebuilt.
const COMBAT_SETTLE: f64 = 0.15;

/// Seconds a commit notice stays up.
const TOAST_SECONDS: f64 = 6.0;

pub struct SpireApp {
    store: Store,
    saver: RunSaver,
    catalog: Catalog,
    playbook: Playbook,
    settings: Settings,
    run: Run,
    state: RunState,
    dash: Dashboard,
    generation: u64,
    /// Analytics and the generation they were built for.
    analytics: Option<(u64, Analytics)>,
    combat: Option<Combat>,
    /// When the dashboard generation last changed (`ctx.input().time`).
    changed: (u64, f64),
    /// The window had focus last frame (to notice it coming back).
    focused: bool,
    /// The native menu bar (macOS), when installed.
    menu: Option<crate::platform::menu::MenuBar>,
    /// Following the game (monitored runs), while on.
    watcher: Option<crate::monitor::Watcher>,
    draft: NodeDraft,
    ui: UiState,
    update_rx: Option<Receiver<Result<fetch::Fetched, String>>>,
}

impl SpireApp {
    /// Loads data, settings and the saved run from `store`.
    pub fn from_store(store: Store) -> Self {
        let loaded = store.load();
        let settings = store.settings();
        let mut messages = loaded.warnings;
        let run = match store.load_run() {
            Some(Ok(run)) => run,
            Some(Err(e)) => {
                messages.push(format!("Could not load the saved run: {e}"));
                Run::default()
            }
            None => Run::default(),
        };
        let state = run.state();
        let dash = Dashboard::compute(&state, &loaded.catalog, &loaded.playbook);
        let source_url = match &settings.source {
            Source::SpireCodex { base_url, .. } => base_url.clone(),
            Source::PackUrl { url } => url.clone(),
        };
        Self {
            draft: NodeDraft::for_next(&state, &loaded.catalog),
            saver: store.run_saver(),
            store,
            catalog: loaded.catalog,
            playbook: loaded.playbook,
            settings,
            run,
            state,
            dash,
            generation: 0,
            analytics: None,
            combat: None,
            changed: (0, 0.0),
            focused: true,
            menu: None,
            watcher: None,
            ui: UiState {
                threat_tab: spire_core::run::NodeKind::Monster,
                show_data: false,
                new_character: None,
                new_ascension: None,
                source_url,
                messages,
                updating: false,
                confirm_abandon: false,
                toast: None,
                tab: crate::views::Tab::Run,
                cards: Default::default(),
                relics: Default::default(),
                enemies: Default::default(),
                end_run_open: false,
                end_encounter: None,
                history: Default::default(),
                show_all_odds: false,
                follow: Default::default(),
            },
            update_rx: None,
        }
    }

    /// Saves the settings, noting a failure.
    fn save_settings(&mut self) {
        if let Err(e) = self.store.save_settings(&self.settings) {
            self.note(format!("Could not save settings: {e}"));
        }
    }

    /// Shows a toast for a few seconds.
    fn toast(&mut self, ctx: &egui::Context, text: String) {
        let now = ctx.input(|i| i.time);
        self.ui.toast = Some(Toast { text, since: now, until: now + TOAST_SECONDS });
        ctx.request_repaint_after(std::time::Duration::from_secs_f64(TOAST_SECONDS));
    }

    fn note(&mut self, msg: impl Into<String>) {
        self.ui.messages.push(msg.into());
        if self.ui.messages.len() > 6 {
            self.ui.messages.remove(0);
        }
    }

    /// Ends the run, files it in history and returns to the start screen.
    fn finish(&mut self, outcome: Outcome) {
        self.run.push(RunEvent::End { outcome, at: Some(spire_data::now_rfc3339()) });
        match self.store.archive_run(&self.run) {
            Ok(_) => self.note("Run saved to history."),
            Err(e) => self.note(format!("Could not save the run to history: {e}")),
        }
        self.run = Run::default();
        self.ui.history.invalidate();
        self.run_changed();
    }

    /// With "Match system" on, shows the light or dark look the system calls
    /// for. A string compare per frame; reconfigures only on a change.
    fn follow_system_theme(&mut self, ctx: &egui::Context) {
        // The OS's colours can change while the app is in the background
        // (accent, contrast theme): read them again when it comes back.
        let focused = ctx.input(|i| i.viewport().focused.unwrap_or(true));
        if focused && !self.focused {
            crate::style::set_appearance(ctx, crate::platform::appearance());
        }
        self.focused = focused;
        // The System look follows light and dark on its own.
        if crate::style::is_system() {
            let pinned = std::env::var_os("SPIRE_THEME").is_some();
            let light = ctx.system_theme().filter(|_| !pinned).map(|t| t == egui::Theme::Light);
            if light.is_some_and(|l| l != crate::style::current().light) {
                crate::style::set_look(ctx, crate::style::SYSTEM_LOOK);
            }
            return;
        }
        if !self.settings.match_system {
            return;
        }
        let light = ctx.system_theme() == Some(egui::Theme::Light);
        let want = if light {
            self.settings.light_look.as_deref().unwrap_or("grimoire")
        } else {
            self.settings.dark_look.as_deref().unwrap_or(crate::style::DEFAULT_LOOK)
        };
        if crate::style::current().name != want {
            crate::style::set_look(ctx, want);
        }
    }

    /// From act 2 on, the act's Ancient heals once the boss is known, by the
    /// game's rules (Weary Traveler lowers it). Recorded as its own event, so
    /// Undo can take it back.
    fn ancient_heal(&mut self) {
        // A followed run takes HP from the game itself.
        if self.run.source.is_some() {
            return;
        }
        let Some(percent) = self.state.ancient_heal_due(&self.catalog.rules) else { return };
        let before = self.state.hp;
        self.run.push(RunEvent::AncientHeal { percent });
        self.run_changed();
        let healed = self.state.hp - before;
        self.note(format!("The Ancient healed {healed} HP ({percent}% of missing HP)."));
    }

    /// Re-derives everything from the run log and saves it.
    fn run_changed(&mut self) {
        self.state = self.run.state();
        self.recompute();
        self.draft = NodeDraft::for_next(&self.state, &self.catalog);
        if let Err(e) = self.saver.save(&self.run).or_else(|_| self.store.save_run(&self.run)) {
            self.note(format!("Could not save the run: {e}"));
        }
    }

    fn recompute(&mut self) {
        self.dash = Dashboard::compute(&self.state, &self.catalog, &self.playbook);
        self.generation += 1;
    }

    fn reload_data(&mut self) {
        let loaded = self.store.load();
        self.catalog = loaded.catalog;
        self.playbook = loaded.playbook;
        for w in loaded.warnings {
            self.note(w);
        }
        self.recompute();
    }

    fn start_update(&mut self, ctx: &egui::Context) {
        if self.update_rx.is_some() {
            return;
        }
        let (tx, rx) = channel();
        let source = self.settings.source.clone();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let _ = tx.send(fetch::fetch(&source).map_err(|e| e.to_string()));
            ctx.request_repaint();
        });
        self.update_rx = Some(rx);
        self.ui.updating = true;
        self.note(format!("Updating from {}…", self.settings.source.describe()));
    }

    fn poll_update(&mut self) {
        let Some(rx) = &self.update_rx else { return };
        let Ok(result) = rx.try_recv() else { return };
        self.update_rx = None;
        self.ui.updating = false;
        match result.and_then(|fetched| {
            let summary = fetched.pack.summary();
            self.store.save_downloaded(&fetched.pack).map(|_| (summary, fetched.notes)).map_err(|e| e.to_string())
        }) {
            Ok((summary, notes)) => {
                self.reload_data();
                for n in notes {
                    self.note(n);
                }
                self.note(format!("Data updated: {summary}"));
            }
            Err(e) => self.note(format!("Update failed, keeping current data. {e}")),
        }
    }

    /// Replaces the run (bench and tests); does not save.
    pub fn set_run(&mut self, run: Run) {
        self.run = run;
        self.state = self.run.state();
        self.recompute();
        self.draft = NodeDraft::for_next(&self.state, &self.catalog);
    }

    /// Opens a tab with an item selected (`SPIRE_TAB` / `SPIRE_SELECT`, for
    /// screenshots and docs). Unknown names are ignored.
    pub fn open(&mut self, tab: &str, select: Option<&str>) {
        match tab {
            "cards" => {
                self.ui.tab = crate::views::Tab::Cards;
                self.ui.cards.selected = select.map(Into::into);
            }
            "relics" => {
                self.ui.tab = crate::views::Tab::Relics;
                self.ui.relics.selected = select.map(Into::into);
            }
            "enemies" => {
                self.ui.tab = crate::views::Tab::Enemies;
                self.ui.enemies.selected = select.map(Into::into);
            }
            "analytics" => self.ui.tab = crate::views::Tab::Analytics,
            "history" => {
                self.ui.tab = crate::views::Tab::History;
                if let Some(id) = select {
                    self.ui.history.select(id);
                }
            }
            _ => {}
        }
    }

    /// Hands over the native menu bar, whose commands the app then applies.
    pub fn set_menu(&mut self, menu: Option<crate::platform::menu::MenuBar>) {
        self.menu = menu;
    }

    /// Commands picked from the native menu bar, as actions; also greys out
    /// the items that can't run now.
    fn menu_actions(&mut self, ctx: &egui::Context) -> Vec<Action> {
        use crate::platform::menu::{Command, Enabled};
        let Some(menu) = self.menu.as_mut() else { return Vec::new() };
        let started = self.state.started();
        let open = started && self.state.outcome.is_none();
        menu.sync(Enabled {
            undo: self.run.events.len() > 1,
            new_run: started,
            end_run: open,
            commit: open && !crate::views::log_locked_state(&self.state),
        });
        let mut out = Vec::new();
        for c in menu.poll() {
            match c {
                Command::Settings => out.push(Action::ToggleDataWindow),
                Command::Tab(n) => {
                    if let Some(t) = crate::views::Tab::ALL.get(n) {
                        self.ui.tab = *t;
                    }
                }
                // In a text field, the menu's Undo is the field's own:
                // hand egui the keystroke the menu took.
                Command::Undo if ctx.text_edit_focused() => ctx.input_mut(|i| {
                    i.events.push(egui::Event::Key {
                        key: egui::Key::Z,
                        physical_key: None,
                        pressed: true,
                        repeat: false,
                        modifiers: egui::Modifiers::COMMAND,
                    })
                }),
                Command::Undo => out.push(Action::Undo),
                Command::NewRun if self.state.outcome.is_none() => self.ui.confirm_abandon = true,
                Command::NewRun => out.push(Action::AbandonRun),
                Command::EndRun => {
                    self.ui.end_run_open = true;
                    self.ui.end_encounter = self.state.encounters_seen.last().cloned();
                }
                Command::CommitFloor => {
                    self.ui.tab = crate::views::Tab::Run;
                    out.push(Action::Record(self.draft.to_node(&self.state)));
                }
            }
        }
        out
    }

    pub fn ui_state(&mut self) -> &mut UiState {
        &mut self.ui
    }

    pub fn draft(&mut self) -> &mut NodeDraft {
        &mut self.draft
    }

    pub fn catalog(&self) -> &Catalog {
        &self.catalog
    }

    pub fn run(&self) -> &Run {
        &self.run
    }

    pub fn apply(&mut self, action: Action, ctx: &egui::Context) {
        match action {
            Action::NewRun { character, ascension } => match RunEvent::start(&self.catalog, &character, ascension) {
                Some(start) => {
                    // The next run asks again.
                    self.ui.new_ascension = None;
                    self.run = Run { started_at: Some(spire_data::now_rfc3339()), ..Run::default() };
                    self.run.push(start);
                    self.run_changed();
                }
                None => self.note(format!("Unknown character {character}")),
            },
            Action::AbandonRun => {
                if self.state.started() && !self.run.ended() {
                    self.finish(Outcome::Abandoned);
                } else {
                    self.run = Run::default();
                    self.run_changed();
                }
            }
            Action::EndRun(outcome) => self.finish(outcome),
            Action::EnterAct { act, index, boss } => {
                self.run.push(RunEvent::EnterAct { act, index, boss });
                self.run_changed();
                self.ancient_heal();
            }
            Action::Record(event) => {
                let node = matches!(event, RunEvent::Node(_));
                let boss = matches!(event, RunEvent::SetBoss(_));
                self.run.push(event);
                self.run_changed();
                if boss {
                    self.ancient_heal();
                }
                // A floor that leaves the player at 0 HP ends the run: offer
                // the End run dialog with the fight already filled in.
                if node && self.state.hp <= 0 && self.state.outcome.is_none() {
                    self.ui.end_run_open = true;
                    self.ui.end_encounter = self.state.encounters_seen.last().cloned();
                }
                if node {
                    self.toast(ctx, format!("Floor {} logged", self.state.floor));
                }
            }
            Action::DismissToast => self.ui.toast = None,
            Action::Open(link) => match link {
                crate::actions::Link::Card(id) => {
                    self.ui.tab = crate::views::Tab::Cards;
                    self.ui.cards.selected = Some(id);
                }
                crate::actions::Link::Encounter(id) => {
                    self.ui.tab = crate::views::Tab::Enemies;
                    self.ui.enemies.selected = Some(id);
                }
                crate::actions::Link::Relic(id) => {
                    self.ui.tab = crate::views::Tab::Relics;
                    self.ui.relics.selected = Some(id);
                }
            },
            Action::Undo => {
                if self.run.undo().is_some() {
                    self.run_changed();
                }
            }
            Action::Draft(change) => {
                // Removing a card that was queued as an add just un-queues it.
                if let Change::RemoveCard(card) = &change
                    && let Some(i) = self.draft.changes.iter().position(|c| *c == Change::AddCard(card.clone()))
                {
                    self.draft.changes.remove(i);
                } else {
                    self.draft.changes.push(change);
                }
            }
            Action::UpdateData => self.start_update(ctx),
            Action::RevertData => match self.store.clear_downloaded() {
                Ok(()) => {
                    self.reload_data();
                    self.note("Reverted to the bundled data.");
                }
                Err(e) => self.note(format!("Could not remove downloaded data: {e}")),
            },
            Action::ExportPlaybook => match self.store.export_playbook() {
                Ok(path) => self.note(format!("Playbook at {} (edit, then Reload).", path.display())),
                Err(e) => self.note(format!("Could not write the playbook: {e}")),
            },
            Action::ReloadData => {
                self.reload_data();
                self.note("Reloaded data and playbook from disk.");
            }
            Action::SetSourceUrl(url) => {
                self.settings.source = if url.trim_end_matches('/').ends_with(".json") {
                    Source::PackUrl { url }
                } else {
                    let (run_pages, channel) = match &self.settings.source {
                        Source::SpireCodex { run_pages, channel, .. } => (*run_pages, *channel),
                        Source::PackUrl { .. } => (fetch::DEFAULT_RUN_PAGES, Default::default()),
                    };
                    Source::SpireCodex { base_url: url, run_pages, channel }
                };
                self.save_settings();
            }
            Action::SetLook(name) => {
                crate::style::set_look(ctx, name);
                // While following the system, a pick fills the light or dark
                // slot; System follows both by itself.
                if self.settings.match_system && name != crate::style::SYSTEM_LOOK {
                    let light = crate::style::find_look(name).is_some_and(|l| l.light);
                    let slot = if light { &mut self.settings.light_look } else { &mut self.settings.dark_look };
                    *slot = Some(name.to_string());
                }
                self.settings.look = Some(name.to_string());
                self.save_settings();
            }
            Action::SetPersonalRoles { card, roles } => {
                match roles {
                    Some(r) => self.playbook.personal_roles.insert(card, r),
                    None => self.playbook.personal_roles.remove(&card),
                };
                if let Err(e) = self.store.save_personal_roles(&self.playbook.personal_roles) {
                    self.note(format!("Could not save your roles: {e}"));
                }
                self.recompute();
            }
            Action::SetLargeText(on) => {
                crate::style::set_large(ctx, on);
                self.settings.large_text = on;
                self.save_settings();
            }
            Action::SetMatchSystem(on) => {
                self.settings.match_system = on;
                self.save_settings();
            }
            Action::SetSystemTitleBar(on) => {
                crate::chrome::set_custom(ctx, !on);
                self.settings.system_title_bar = on;
                self.save_settings();
            }
            Action::SetChannel(channel) => {
                if let Source::SpireCodex { channel: c, .. } = &mut self.settings.source {
                    *c = channel;
                }
                self.save_settings();
            }
            Action::ToggleDataWindow => self.ui.show_data = !self.ui.show_data,
            Action::SaveHistory { id, run } => match self.store.update_archived(&id, &run) {
                Ok(()) => {
                    self.ui.history.invalidate();
                    self.note("Run history updated.");
                }
                Err(e) => self.note(format!("Could not save the edited run: {e}")),
            },
            Action::SetFollowGame(on) => {
                self.settings.follow_game = on;
                self.save_settings();
                self.watcher = if on { crate::monitor::Watcher::start(ctx.clone()) } else { None };
                self.ui.follow = Default::default();
                if !on && self.run.source.take().is_some() {
                    // Keep the run, now logged by hand from here on.
                    self.run_changed();
                }
            }
            Action::FollowGameRun => {
                if let Some(game) = self.ui.follow.conflict.take() {
                    if self.state.started() && !self.run.ended() {
                        self.finish(Outcome::Abandoned);
                    }
                    self.adopt(game);
                }
            }
            Action::ContextMenu(entries) => {
                let (menu, actions) = crate::actions::MenuEntry::split(&entries);
                let picked = self.menu.as_mut().and_then(|m| m.context_menu(&menu));
                if let Some(action) = picked.and_then(|i| actions.get(i).cloned()) {
                    self.apply(action, ctx);
                }
            }
            Action::DeleteHistory(id) => match self.store.delete_archived(&id) {
                Ok(()) => {
                    self.ui.history.invalidate();
                    self.note("Run deleted from history.");
                }
                Err(e) => self.note(format!("Could not delete the run: {e}")),
            },
        }
    }
}

impl SpireApp {
    /// One frame of UI: layout, then apply the actions it produced.
    pub fn frame(&mut self, ui: &mut egui::Ui) {
        self.poll_update();
        crate::style::set_character(ui.ctx(), self.state.character.as_ref().map(|c| c.as_str()));
        self.follow_system_theme(ui.ctx());
        for e in self.saver.take_errors() {
            self.note(format!("Could not save the run: {e}"));
        }
        let ctx = ui.ctx().clone();
        self.follow_game(&ctx);
        let mut actions = self.menu_actions(&ctx);
        actions.extend(self.keys(&ctx));
        if self.ui.toast.as_ref().is_some_and(|t| ctx.input(|i| i.time) > t.until) {
            self.ui.toast = None;
        }
        self.prepare_tab(&ctx);
        {
            let view = ViewCtx {
                catalog: &self.catalog,
                playbook: &self.playbook,
                state: &self.state,
                dash: &self.dash,
                can_undo: self.run.events.len() > 1,
                generation: self.generation,
                analytics: self.analytics.as_ref().filter(|(g, _)| *g == self.generation).map(|(_, a)| a),
                combat: self.combat.as_ref(),
                following: self.following(),
            };
            let (store, settings) = (&self.store, &self.settings);
            let mut page = |ui: &mut egui::Ui, st: &mut UiState, out: &mut Vec<Action>| {
                views::data::contents(ui, &view, store, settings, st, out);
            };
            views::root(ui, &view, &mut self.draft, &mut self.ui, &mut actions, &mut page);
            if self.ui.show_data {
                views::data::window(&ctx, &view, &self.store, &self.settings, &mut self.ui, &mut actions);
            }
        }
        crate::chrome::window_edges(&ctx);
        crate::chrome::focus_ring(&ctx);
        for action in actions {
            self.apply(action, &ctx);
        }
    }
}

impl SpireApp {
    /// The run follows the game right now.
    fn following(&self) -> bool {
        self.run.source.is_some() && self.watcher.is_some()
    }

    /// Monitored runs: start the watcher when following is on, and take in
    /// what it found.
    fn follow_game(&mut self, ctx: &egui::Context) {
        if self.settings.follow_game && self.watcher.is_none() {
            self.watcher = crate::monitor::Watcher::start(ctx.clone());
        }
        let Some(updates) = self.watcher.as_ref().map(|w| w.poll()) else { return };
        for u in updates {
            self.sync_game(u, ctx);
        }
    }

    fn sync_game(&mut self, update: crate::monitor::Update, ctx: &egui::Context) {
        use crate::follow::Step;
        let (link, json) = match update {
            crate::monitor::Update::NoLink => {
                self.ui.follow.no_link = true;
                self.ui.follow.link = None;
                return;
            }
            crate::monitor::Update::Run { link, json } => (link, json),
        };
        self.ui.follow.no_link = false;
        let read = crate::follow::read(&link, json, &self.catalog);
        self.ui.follow.link = Some(link);
        let game = match read {
            Ok(game) => game,
            Err(problem) => return self.follow_problem(problem),
        };
        self.follow_problem(None);
        match crate::follow::step(game, &self.run, self.state.started(), &self.ui.follow.filed) {
            Step::Nothing => {}
            Step::Advance { events, ended } => {
                if self.run.events != events {
                    let before = self.state.floor;
                    self.run.events = events;
                    self.run_changed();
                    if self.state.floor > before {
                        self.toast(ctx, format!("Floor {} logged from the game", self.state.floor));
                    }
                }
                if ended {
                    self.file_followed();
                }
            }
            Step::Waiting => self.ui.follow.waiting = true,
            Step::Adopt(game) => self.adopt(game),
            Step::Conflict(game) => self.ui.follow.conflict = Some(game),
        }
    }

    /// Notes why following stopped (once per reason), or clears it.
    fn follow_problem(&mut self, problem: Option<String>) {
        if problem != self.ui.follow.problem
            && let Some(p) = &problem
        {
            self.note(p.clone());
        }
        self.ui.follow.problem = problem;
    }

    /// Makes the game's run the app's run.
    fn adopt(&mut self, game: spire_data::game_run::GameRun) {
        self.ui.follow.waiting = false;
        let key = crate::follow::key(&game);
        let started_at = game.start_time.map_or_else(spire_data::now_rfc3339, spire_data::rfc3339);
        self.run = Run { events: game.events, started_at: Some(started_at), source: Some(key), notes: None };
        self.ui.follow.conflict = None;
        self.run_changed();
        self.note(format!("Following the game's run ({} floors so far).", game.floors));
        if !game.unknown.is_empty() {
            self.note(format!(
                "{} things in this run aren't in the game data yet; update data in Settings.",
                game.unknown.len()
            ));
        }
        if game.ended {
            self.file_followed();
        }
    }

    /// The followed run has ended in the game: file it in history.
    fn file_followed(&mut self) {
        self.ui.follow.filed.extend(self.run.source.clone());
        match self.store.archive_run(&self.run) {
            Ok(_) => self.note("The game's run ended; saved to history."),
            Err(e) => self.note(format!("Could not save the run to history: {e}")),
        }
        self.run = Run::default();
        self.ui.history.invalidate();
        self.run_changed();
    }

    /// Builds what the open tab needs and nothing else: analytics for the
    /// live run, or the archived runs read from disk.
    fn prepare_tab(&mut self, ctx: &egui::Context) {
        use crate::views::Tab;
        let fights = matches!(self.ui.tab, Tab::Run | Tab::Enemies | Tab::Analytics);
        let now = ctx.input(|i| i.time);
        if self.changed.0 != self.generation {
            self.changed = (self.generation, now);
        }
        // The deck simulation takes a few milliseconds: rebuild it once the
        // run has settled for a moment, not on the frame of every commit or
        // undo. The previous outlook stays up meanwhile.
        let stale = self.combat.as_ref().is_none_or(|c| c.generation != self.generation);
        let settled = self.combat.is_none() || now - self.changed.1 >= COMBAT_SETTLE;
        if fights && self.state.started() && stale && !settled {
            ctx.request_repaint_after(std::time::Duration::from_secs_f64(COMBAT_SETTLE));
        }
        if fights && self.state.started() && stale && settled {
            let sim = simulate(&self.state, &self.catalog, &self.playbook.simulation);
            let mut outlooks = Vec::new();
            for t in self.dash.threats.iter().flat_map(|(_, l)| l.iter()).chain(self.dash.bosses.iter()) {
                if t.room == spire_core::catalog::RoomType::Monster || outlooks.iter().any(|(e, _)| *e == t.encounter) {
                    continue;
                }
                let Some(enc) = self.catalog.encounters.get(&t.encounter) else { continue };
                if let Some(o) =
                    survival::for_encounter(&sim, enc, &self.catalog, &self.state, &self.playbook.simulation)
                {
                    outlooks.push((t.encounter.clone(), o));
                }
            }
            self.combat = Some(Combat { generation: self.generation, sim, outlooks });
        }
        match self.ui.tab {
            Tab::Analytics if self.analytics.as_ref().is_none_or(|(g, _)| *g != self.generation) => {
                let sim = match &self.combat {
                    Some(c) if c.generation == self.generation => c.sim.clone(),
                    _ => simulate(&self.state, &self.catalog, &self.playbook.simulation),
                };
                let a = Analytics { timeline: timeline(&self.run, &self.catalog, &self.playbook), sim };
                self.analytics = Some((self.generation, a));
            }
            Tab::History if self.ui.history.needs_load() => {
                self.ui.history.load(self.store.archived_runs());
            }
            _ => {}
        }
    }

    /// Keyboard accelerators. Ignored while a text field has focus, so
    /// typing a search never triggers them; everything also works by mouse.
    ///
    /// 1–8 room · Ctrl+Enter commit · Ctrl+Z undo · Ctrl+1–6 tabs · Esc close.
    fn keys(&mut self, ctx: &egui::Context) -> Vec<Action> {
        use egui::{Key, Modifiers};
        let mut out = Vec::new();
        if ctx.text_edit_focused() {
            return out;
        }
        let tabs = crate::views::Tab::ALL;
        let digits = [Key::Num1, Key::Num2, Key::Num3, Key::Num4, Key::Num5, Key::Num6, Key::Num7, Key::Num8];
        ctx.input_mut(|i| {
            for (n, key) in digits.iter().enumerate() {
                if n < tabs.len() && i.consume_key(Modifiers::COMMAND, *key) {
                    self.ui.tab = tabs[n];
                }
            }
            let locked = self.state.act.is_none() || self.state.boss.is_none();
            let following = self.following();
            if self.ui.tab == crate::views::Tab::Run && self.state.started() && !locked && !following {
                for (n, key) in digits.iter().enumerate() {
                    if i.consume_key(Modifiers::NONE, *key) {
                        self.draft.kind = spire_core::run::NodeKind::ALL[n];
                    }
                }
                if i.consume_key(Modifiers::COMMAND, Key::Enter) {
                    out.push(Action::Record(self.draft.to_node(&self.state)));
                }
            }
            if i.consume_key(Modifiers::COMMAND, Key::Z) {
                out.push(Action::Undo);
            }
            if i.consume_key(Modifiers::NONE, Key::Escape) {
                self.ui.show_data = false;
                self.ui.end_run_open = false;
                self.ui.confirm_abandon = false;
            }
        });
        out
    }
}

impl eframe::App for SpireApp {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        // Keep Mica's light or dark in step with the look (no-op unless it
        // changed, and off Windows).
        if crate::style::material() {
            crate::chrome::material(crate::chrome::Window::Frame(frame), !crate::style::current().light);
        }
        self.frame(ui);
    }

    fn clear_color(&self, visuals: &egui::Visuals) -> [f32; 4] {
        crate::style::clear_color(visuals)
    }
}

#[cfg(test)]
mod follow_tests {
    use super::*;
    use crate::monitor::Update;
    use spire_data::game_link::Link;

    const FINISHED: &str = include_str!("../../spire-data/tests/fixtures/finished.run");

    /// The fixture run cut to `floors` floors of act 2, in progress.
    fn in_progress(floors: usize) -> String {
        let mut v: serde_json::Value = serde_json::from_str(FINISHED).unwrap();
        let acts = v["map_point_history"].as_array().unwrap().clone();
        let act2: Vec<_> = acts[1].as_array().unwrap().iter().take(floors).cloned().collect();
        v["map_point_history"] = serde_json::json!([acts[0], act2]);
        for k in ["win", "was_abandoned", "killed_by_encounter"] {
            v.as_object_mut().unwrap().remove(k);
        }
        v.to_string()
    }

    fn app() -> (SpireApp, egui::Context) {
        let dir = std::env::temp_dir().join(format!("spire-follow-{}-{}", std::process::id(), rand_suffix()));
        (SpireApp::from_store(Store::at(dir)), egui::Context::default())
    }

    fn rand_suffix() -> u128 {
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
    }

    fn update(json: String, in_progress: bool) -> Update {
        Update::Run { link: Box::new(Link { in_progress, act_index: Some(1), ..Link::default() }), json: Some(json) }
    }

    #[test]
    fn follows_the_game_from_adoption_to_history() {
        let (mut app, ctx) = app();
        app.sync_game(update(in_progress(3), true), &ctx);
        assert!(app.run.source.is_some(), "adopted the game's run");
        let floor = app.state.floor;
        assert!(floor > 10);

        // The game saves after the next floor: the run moves on.
        app.sync_game(update(in_progress(4), true), &ctx);
        assert_eq!(app.state.floor, floor + 1);
        assert!(app.ui.toast.is_some());

        // The run ends: filed in history, and the app is ready for the next.
        app.sync_game(update(FINISHED.to_string(), false), &ctx);
        assert!(!app.state.started());
        assert_eq!(app.store.archived_runs().len(), 1);
        // Seeing the same finished run again changes nothing.
        app.sync_game(update(FINISHED.to_string(), false), &ctx);
        assert_eq!(app.store.archived_runs().len(), 1);
    }

    #[test]
    fn follows_a_live_run_save() {
        let (mut app, ctx) = app();
        let live = include_str!("../../spire-data/tests/fixtures/in_progress.save");
        let link = Link { in_progress: true, act_index: Some(0), ..Link::default() };
        app.sync_game(Update::Run { link: Box::new(link), json: Some(live.to_string()) }, &ctx);
        assert_eq!(app.ui.follow.problem, None);
        assert!(app.run.source.is_some(), "adopted the game's run");
        assert_eq!((app.state.hp, app.state.gold), (61, 99));
        assert!(!app.run.ended());
    }

    #[test]
    fn says_why_it_cannot_follow() {
        let (mut app, ctx) = app();
        // A save the converter can't read: the reason shows, once.
        app.sync_game(update("\u{1f}\u{8b}binary".into(), true), &ctx);
        let problem = app.ui.follow.problem.clone().expect("a reason");
        assert!(problem.contains("isn't plain JSON"), "{problem}");
        assert!(!app.state.started());
        // No saves folder found.
        let link = Link { in_progress: true, ..Link::default() };
        app.sync_game(Update::Run { link: Box::new(link), json: None }, &ctx);
        assert!(app.ui.follow.problem.as_deref().is_some_and(|p| p.contains("saves folder")));
        // A saves folder, but no run save yet.
        let link = Link { in_progress: true, save_dir: Some(std::env::temp_dir()), ..Link::default() };
        app.sync_game(Update::Run { link: Box::new(link), json: None }, &ctx);
        assert!(app.ui.follow.problem.as_deref().is_some_and(|p| p.contains("hasn't saved")));
        // Once it reads, the reason clears.
        app.sync_game(update(in_progress(3), true), &ctx);
        assert_eq!(app.ui.follow.problem, None);
        assert!(app.run.source.is_some());
    }

    #[test]
    fn a_finished_run_seen_first_is_left_alone() {
        let (mut app, ctx) = app();
        app.sync_game(update(FINISHED.to_string(), false), &ctx);
        assert!(!app.state.started());
        assert!(app.store.archived_runs().is_empty());
        assert!(app.ui.follow.waiting);
    }

    #[test]
    fn a_hand_logged_run_is_not_replaced_without_asking() {
        let (mut app, ctx) = app();
        let character = app.catalog.characters.keys().next().unwrap().clone();
        app.apply(Action::NewRun { character, ascension: 0 }, &ctx);
        let before = app.run.events.len();
        app.sync_game(update(in_progress(2), true), &ctx);
        assert_eq!(app.run.events.len(), before);
        assert!(app.ui.follow.conflict.is_some());
        // Following it files the hand-logged run and takes the game's.
        app.apply(Action::FollowGameRun, &ctx);
        assert!(app.run.source.is_some());
        assert_eq!(app.store.archived_runs().len(), 1);
    }
}
