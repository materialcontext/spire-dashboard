//! Monitored runs: follow the game through the companion mod's link file.
//!
//! While the player has "Follow the game" on, a background thread checks
//! twice a second whether the link file or any profile's run save has
//! changed (a `stat` each; the profile folders are listed again every few
//! seconds), and only then reads them and sends them over. Watching every
//! profile, not just the one picked, notices when the game starts writing
//! another (another Steam account, or a save-path mod).
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

/// Checks between listings of the profile folders.
const RELIST: u32 = 20;

/// Reads of a half-written file before its error is reported.
const RETRIES: u32 = 4;

/// Whether a run file read is cut short (not whole JSON): the game was
/// writing it.
fn torn(update: &Update) -> bool {
    match update {
        Update::Run { json: Some(j), .. } => !game_link::whole_json(j),
        _ => false,
    }
}

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
        let game = game_link::game_dir();
        let stop = Arc::new(AtomicBool::new(false));
        let (tx, rx) = channel();
        let flag = stop.clone();
        std::thread::Builder::new()
            .name("follow-game".into())
            .spawn(move || {
                let mut seen: (Option<SystemTime>, Vec<Option<SystemTime>>) = (None, Vec::new());
                let mut first = true;
                let mut run_saves: Vec<PathBuf> = Vec::new();
                let mut tick = 0u32;
                let mut retries = 0u32;
                while !flag.load(Ordering::Relaxed) {
                    if tick.is_multiple_of(RELIST) {
                        run_saves = game.as_deref().map_or_else(Vec::new, game_link::save_dirs);
                        run_saves.iter_mut().for_each(|d| d.push(game_link::RUN_SAVE));
                    }
                    tick = tick.wrapping_add(1);
                    let now = (
                        game_link::modified(&path),
                        run_saves.iter().map(|p| game_link::modified(p)).collect::<Vec<_>>(),
                    );
                    if first || now != seen {
                        let update = match game_link::read_link(&path) {
                            None => Update::NoLink,
                            Some(mut link) => {
                                if let Some(game) = &game {
                                    link.locate(game);
                                }
                                let json =
                                    if link.in_progress { link.read_run_save() } else { link.read_finished(None) };
                                Update::Run { link: Box::new(link), json }
                            }
                        };
                        // A file caught mid-write is read again next tick,
                        // a few times, before the error is reported.
                        if torn(&update) && retries < RETRIES {
                            retries += 1;
                            std::thread::sleep(EVERY);
                            continue;
                        }
                        retries = 0;
                        first = false;
                        seen = now;
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
