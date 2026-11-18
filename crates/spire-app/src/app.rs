//! App state and the single place where actions change it.

use std::sync::mpsc::{Receiver, channel};

use eframe::egui;
use spire_core::analysis::Dashboard;
use spire_core::analysis::sim::{DeckSim, simulate};
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
    pub new_ascension: u32,
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
}

pub struct Toast {
    pub text: String,
    /// `ctx.input().time` after which it disappears.
    pub until: f64,
}

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
            ui: UiState {
                threat_tab: spire_core::run::NodeKind::Monster,
                show_data: false,
                new_character: None,
                new_ascension: 0,
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
            },
            update_rx: None,
        }
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
            }
            Action::Record(event) => {
                let node = matches!(event, RunEvent::Node(_));
                self.run.push(event);
                self.run_changed();
                if node {
                    let now = ctx.input(|i| i.time);
                    let text = format!("Floor {} logged", self.state.floor);
                    self.ui.toast = Some(Toast { text, until: now + TOAST_SECONDS });
                    ctx.request_repaint_after(std::time::Duration::from_secs_f64(TOAST_SECONDS));
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
                if let Err(e) = self.store.save_settings(&self.settings) {
                    self.note(format!("Could not save settings: {e}"));
                }
            }
            Action::SetLook(name) => {
                crate::style::set_look(ctx, name);
                self.settings.look = Some(name.to_string());
                if let Err(e) = self.store.save_settings(&self.settings) {
                    self.note(format!("Could not save settings: {e}"));
                }
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
                if let Err(e) = self.store.save_settings(&self.settings) {
                    self.note(format!("Could not save settings: {e}"));
                }
            }
            Action::SetSystemTitleBar(on) => {
                crate::chrome::set_custom(ctx, !on);
                self.settings.system_title_bar = on;
                if let Err(e) = self.store.save_settings(&self.settings) {
                    self.note(format!("Could not save settings: {e}"));
                }
            }
            Action::SetChannel(channel) => {
                if let Source::SpireCodex { channel: c, .. } = &mut self.settings.source {
                    *c = channel;
                }
                if let Err(e) = self.store.save_settings(&self.settings) {
                    self.note(format!("Could not save settings: {e}"));
                }
            }
            Action::ToggleDataWindow => self.ui.show_data = !self.ui.show_data,
        }
    }
}

impl SpireApp {
    /// One frame of UI: layout, then apply the actions it produced.
    pub fn frame(&mut self, ui: &mut egui::Ui) {
        self.poll_update();
        crate::style::set_character(ui.ctx(), self.state.character.as_ref().map(|c| c.as_str()));
        for e in self.saver.take_errors() {
            self.note(format!("Could not save the run: {e}"));
        }
        let ctx = ui.ctx().clone();
        let mut actions = self.keys(&ctx);
        if self.ui.toast.as_ref().is_some_and(|t| ctx.input(|i| i.time) > t.until) {
            self.ui.toast = None;
        }
        self.prepare_tab();
        {
            let view = ViewCtx {
                catalog: &self.catalog,
                playbook: &self.playbook,
                state: &self.state,
                dash: &self.dash,
                can_undo: self.run.events.len() > 1,
                generation: self.generation,
                analytics: self.analytics.as_ref().filter(|(g, _)| *g == self.generation).map(|(_, a)| a),
            };
            views::root(ui, &view, &mut self.draft, &mut self.ui, &mut actions);
            if self.ui.show_data {
                views::data::window(&ctx, &view, &self.store, &self.settings, &mut self.ui, &mut actions);
            }
        }
        crate::chrome::window_edges(&ctx);
        for action in actions {
            self.apply(action, &ctx);
        }
    }
}

impl SpireApp {
    /// Builds what the open tab needs and nothing else: analytics for the
    /// live run, or the archived runs read from disk.
    fn prepare_tab(&mut self) {
        match self.ui.tab {
            crate::views::Tab::Analytics if self.analytics.as_ref().is_none_or(|(g, _)| *g != self.generation) => {
                let a = Analytics {
                    timeline: timeline(&self.run, &self.catalog, &self.playbook),
                    sim: simulate(&self.state, &self.catalog, &self.playbook.simulation),
                };
                self.analytics = Some((self.generation, a));
            }
            crate::views::Tab::History if self.ui.history.needs_load() => {
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
            if self.ui.tab == crate::views::Tab::Run && self.state.started() {
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
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.frame(ui);
    }

    fn clear_color(&self, visuals: &egui::Visuals) -> [f32; 4] {
        crate::style::clear_color(visuals)
    }
}
