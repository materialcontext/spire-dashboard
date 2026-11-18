//! Monitored runs: follow the game through the companion mod's link file.
//!
//! While the player has "Follow the game" on, a background thread checks
//! twice a second whether the link file or the run save it names has
//! changed (two `stat` calls), and only then reads them and sends them over.
//! The UI is woken only when something arrived, so following adds no idle
//! frames. Turning it off stops the thread.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, channel};
use std::time::{Duration, SystemTime};

use eframe::egui;
use spire_data::game_link::{self, Link};

/// Seconds between checks.
const EVERY: Duration = Duration::from_millis(500);

/// What the watcher found.
pub enum Update {
    /// No link file yet: the mod isn't installed, enabled or running.
    NoLink,
    /// The link, and the run it points to (in progress, or the newest
    /// finished one when no run is in progress).
    Run { link: Box<Link>, json: Option<String> },
}

pub struct Watcher {
    stop: Arc<AtomicBool>,
    rx: Receiver<Update>,
}

impl Watcher {
    /// Starts following. `None` if there's no place for the link on this
    /// system (no data directory).
    pub fn start(ctx: egui::Context) -> Option<Watcher> {
        let path = game_link::link_path()?;
        let stop = Arc::new(AtomicBool::new(false));
        let (tx, rx) = channel();
        let flag = stop.clone();
        std::thread::Builder::new()
            .name("follow-game".into())
            .spawn(move || {
                let mut seen: (Option<SystemTime>, Option<SystemTime>) = (None, None);
                let mut first = true;
                let mut run_save: Option<PathBuf> = None;
                while !flag.load(Ordering::Relaxed) {
                    let now = (game_link::modified(&path), run_save.as_deref().and_then(game_link::modified));
                    if first || now != seen {
                        first = false;
                        seen = now;
                        let update = match game_link::read_link(&path) {
                            None => Update::NoLink,
                            Some(link) => {
                                run_save = link.run_save.clone();
                                let json =
                                    if link.in_progress { link.read_run_save() } else { link.read_finished(None) };
                                Update::Run { link: Box::new(link), json }
                            }
                        };
                        if tx.send(update).is_err() {
                            break;
                        }
                        ctx.request_repaint();
                    }
                    std::thread::sleep(EVERY);
                }
            })
            .ok()?;
        Some(Watcher { stop, rx })
    }

    /// Everything that arrived since the last call.
    pub fn poll(&self) -> Vec<Update> {
        self.rx.try_iter().collect()
    }
}

impl Drop for Watcher {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

/// What the Settings page and the log say about following.
#[derive(Default)]
pub struct Status {
    /// The mod's link, once seen.
    pub link: Option<Box<Link>>,
    /// The mod's link hasn't appeared (yet).
    pub no_link: bool,
    /// The game is on a run other than the app's hand-logged one.
    pub conflict: Option<spire_data::game_run::GameRun>,
    /// Runs already filed from the game, so they're not adopted again.
    pub filed: Vec<String>,
    /// Why the game's run isn't being followed, when something stops it.
    pub problem: Option<String>,
    /// Waiting for the next run: the game's latest run ended before we
    /// followed it.
    pub waiting: bool,
}
