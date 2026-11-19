//! Monitored runs, decided: what the watcher found means for the app's run.
//!
//! [`crate::monitor`] does the IO (a background thread watching the link
//! file and the run saves). This module is pure: [`read`] turns what it
//! found into the game's run, or the reason following stopped, and
//! [`step`] decides what that run does to the app's. `SpireApp::sync_game`
//! applies the step.

use spire_core::Catalog;
use spire_core::run::{Run, RunEvent};
use spire_data::game_link::{self, Link};
use spire_data::game_run::{self, GameRun};

/// What the Settings page and the log say about following.
#[derive(Default)]
pub struct Status {
    /// The mod's link, once seen.
    pub link: Option<Box<Link>>,
    /// The mod's link hasn't appeared (yet).
    pub no_link: bool,
    /// The game is on a run other than the app's hand-logged one.
    pub conflict: Option<GameRun>,
    /// Runs already filed from the game, so they're not adopted again.
    pub filed: Vec<String>,
    /// When the last followed run that ended had started: runs that started
    /// before it are old saves (another profile's), not the next run.
    pub filed_start: Option<i64>,
    /// Why the game's run isn't being followed, when something stops it.
    pub problem: Option<String>,
    /// Waiting for the next run: the game's latest run ended before we
    /// followed it.
    pub waiting: bool,
}

/// The app's name for a game run, kept as the followed run's `source`.
pub fn key(game: &GameRun) -> String {
    format!("game:{}", game.key)
}

/// When the followed run started, from its key (`game:<start>:<seed>`).
pub fn followed_start(run: &Run) -> Option<i64> {
    run.source.as_deref()?.strip_prefix("game:")?.split(':').next()?.parse().ok()
}

/// The game's run from what the watcher found: the run save (or the last
/// finished run) read against the link. `Err(Some(reason))` when following
/// stops; `Err(None)` when there's simply nothing to follow.
pub fn read(link: &Link, json: Option<String>, catalog: &Catalog) -> Result<GameRun, Option<String>> {
    match (json, link.in_progress, &link.run_save) {
        (Some(j), ..) => game_run::read(&j, catalog, &link.hint()).map_err(|why| {
            let what = if link.in_progress { "the game's run save" } else { "the game's last finished run" };
            Some(format!("Can't follow: couldn't read {what}: {why}."))
        }),
        (None, true, None) if link.save_dir.is_none() => {
            let under = game_link::game_dir().map_or(String::new(), |d| format!(" under {}", d.display()));
            Err(Some(format!("Can't follow: couldn't find the game's saves folder{under}.")))
        }
        (None, true, None) => {
            Err(Some("The game hasn't saved this run yet (no current_run.save in its saves folder).".into()))
        }
        (None, true, Some(path)) => Err(Some(format!("Can't follow: couldn't open {}.", path.display()))),
        (None, false, _) => Err(None),
    }
}

/// What the game's run does to the app's.
#[derive(Debug, PartialEq)]
pub enum Step {
    /// Already filed in history: nothing to do.
    Nothing,
    /// The followed run, as the game has it now; file it if it ended.
    Advance { events: Vec<RunEvent>, ended: bool },
    /// A run that finished before following began, or an older run than
    /// the last one followed: wait for the next.
    Waiting,
    /// The game's run becomes the app's.
    Adopt(GameRun),
    /// The game is on another run than the hand-logged one: ask first.
    Conflict(GameRun),
}

/// Decides the step for the game's run against the app's `run`.
/// `started`: the app's run has begun. `after`: when the last followed run
/// that ended had started; older runs aren't taken up.
pub fn step(game: GameRun, run: &Run, started: bool, filed: &[String], after: Option<i64>) -> Step {
    let key = key(&game);
    if filed.contains(&key) {
        Step::Nothing
    } else if run.source.as_deref() == Some(key.as_str()) {
        Step::Advance { ended: game.ended, events: game.events }
    } else if game.ended || after.zip(game.start_time).is_some_and(|(after, start)| start <= after) {
        Step::Waiting
    } else if !started || run.ended() {
        Step::Adopt(game)
    } else {
        Step::Conflict(game)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn game(key: &str, ended: bool) -> GameRun {
        GameRun { key: key.into(), start_time: None, events: Vec::new(), floors: 0, ended, unknown: Vec::new() }
    }

    #[test]
    fn decides_each_case() {
        let fresh = Run::default();
        let followed = Run { source: Some("game:a".into()), ..Run::default() };
        assert!(matches!(step(game("a", false), &fresh, false, &[], None), Step::Adopt(_)));
        assert!(matches!(step(game("a", false), &fresh, true, &[], None), Step::Conflict(_)));
        assert_eq!(step(game("a", true), &fresh, false, &[], None), Step::Waiting);
        assert!(matches!(step(game("a", true), &followed, true, &[], None), Step::Advance { ended: true, .. }));
        assert_eq!(step(game("a", true), &followed, true, &["game:a".into()], None), Step::Nothing);
    }

    #[test]
    fn after_a_run_ends_older_saves_are_not_taken_up() {
        let fresh = Run::default();
        let at = |start: i64| GameRun { start_time: Some(start), ..game("x", false) };
        // Another profile's old save, started before the run just filed.
        assert_eq!(step(at(100), &fresh, false, &[], Some(200)), Step::Waiting);
        // The next run, started after it.
        assert!(matches!(step(at(300), &fresh, false, &[], Some(200)), Step::Adopt(_)));
        assert_eq!(
            followed_start(&Run { source: Some("game:1700000000:SEED".into()), ..Run::default() }),
            Some(1_700_000_000)
        );
    }
}
