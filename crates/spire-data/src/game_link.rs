//! The companion mod's link file, and the game's run saves it points to.
//!
//! The mod (`mod/SpireDashboardLink`) writes `game-link.json` in
//! `<local app data>/SpireDashboard/` whenever the game's state that matters
//! here changes: whether a run is in progress, where the profile's saves
//! are (the game keeps a separate `modded/` profile while mods are on),
//! when the run was last saved, and the act and its boss.
//!
//! The mod may not manage to name the saves folder (the game's own lookup
//! can change), so the app also finds it: the game keeps its data in the
//! system's app data folder, one folder per account and profile
//! (`SlayTheSpire2/steam/<Steam id>/profile<N>/saves`, or
//! `.../modded/profile<N>/saves` with mods on). [`Link::locate`] fills in
//! whatever the link left out from there.
//!
//! Only read while the player has turned on monitoring. Nothing here writes
//! anywhere.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use serde::Deserialize;

use crate::game_run::ActHint;

/// The link file's contents (schema 2; schema 1 lacks `room`).
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct Link {
    pub mod_version: String,
    pub game_version: Option<String>,
    pub in_progress: bool,
    pub profile_id: Option<i32>,
    pub save_dir: Option<PathBuf>,
    pub run_save: Option<PathBuf>,
    pub history_dir: Option<PathBuf>,
    pub act_index: Option<u8>,
    pub act: Option<String>,
    pub boss: Option<String>,
    pub second_boss: Option<String>,
    /// The room being played (mod 0.2.0 and later).
    pub room: Option<Room>,
}

/// The room the player is in, as the mod reads it from the game. The run
/// save only records a room once it's finished.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct Room {
    /// The game's room class: `CombatRoom`, `EventRoom`, `MapRoom`…
    pub kind: String,
    /// Monster, Elite, Boss… for a fight.
    #[serde(rename = "type")]
    pub room_type: Option<String>,
    /// `ENCOUNTER.X`, when the mod found it.
    pub encounter: Option<String>,
    /// `EVENT.X`, in an event.
    pub event: Option<String>,
    /// The enemies' monster ids (`MONSTER.X`), in a fight.
    pub monsters: Option<Vec<String>>,
}

impl Link {
    /// What the link knows about the act in progress.
    pub fn hint(&self) -> ActHint {
        ActHint {
            index: self.act_index.filter(|_| self.in_progress),
            act: self.act.clone().filter(|_| self.in_progress),
            boss: self.boss.clone().filter(|_| self.in_progress),
            second_boss: self.second_boss.clone().filter(|_| self.in_progress),
        }
    }

    /// The game is keeping this profile under its modded saves.
    pub fn modded_profile(&self) -> bool {
        self.save_dir.as_ref().is_some_and(|d| d.components().any(|c| c.as_os_str() == "modded"))
    }

    /// The run in progress, as the game saved it.
    pub fn read_run_save(&self) -> Option<String> {
        std::fs::read_to_string(self.run_save.as_ref()?).ok()
    }

    /// A finished run from the profile's history: the file named for its
    /// start time if there is one, else the newest.
    pub fn read_finished(&self, start_time: Option<&str>) -> Option<String> {
        let dir = self.history_dir.as_ref()?;
        if let Some(t) = start_time.filter(|t| !t.is_empty() && *t != "0")
            && let Ok(json) = std::fs::read_to_string(dir.join(format!("{t}.run")))
        {
            return Some(json);
        }
        let newest = std::fs::read_dir(dir)
            .ok()?
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == "run"))
            .max_by_key(|p| modified(p))?;
        std::fs::read_to_string(newest).ok()
    }
}

/// Where the companion mod writes its link: the same folder .NET calls
/// `LocalApplicationData` (Windows `%LOCALAPPDATA%`, macOS
/// `~/Library/Application Support`, Linux `~/.local/share`).
pub fn link_path() -> Option<PathBuf> {
    let base = directories::BaseDirs::new()?;
    Some(base.data_local_dir().join("SpireDashboard").join("game-link.json"))
}

/// The game's data folder: Godot keeps it in the system's app data
/// (Windows `%APPDATA%\SlayTheSpire2`, macOS
/// `~/Library/Application Support/SlayTheSpire2`, Linux
/// `~/.local/share/SlayTheSpire2`).
pub fn game_dir() -> Option<PathBuf> {
    Some(directories::BaseDirs::new()?.data_dir().join("SlayTheSpire2"))
}

/// Every profile's saves folder under the game's data folder:
/// `<store>/<account>/profile<N>/saves` and `.../modded/profile<N>/saves`.
pub fn save_dirs(game: &Path) -> Vec<PathBuf> {
    let subdirs = |dir: &Path| -> Vec<PathBuf> {
        std::fs::read_dir(dir)
            .map(|rd| rd.filter_map(Result::ok).map(|e| e.path()).filter(|p| p.is_dir()).collect())
            .unwrap_or_default()
    };
    let profiles = |dir: &Path| -> Vec<PathBuf> {
        subdirs(dir)
            .into_iter()
            .filter(|p| profile_number(p).is_some())
            .map(|p| p.join("saves"))
            .filter(|p| p.is_dir())
            .collect()
    };
    let mut out = Vec::new();
    for store in subdirs(game) {
        for account in subdirs(&store) {
            out.extend(profiles(&account));
            out.extend(profiles(&account.join("modded")));
        }
    }
    out
}

/// A finished run in any profile's history, by its start time (the game
/// names finished runs `<start time>.run`).
pub fn find_finished(game: &Path, start_time: i64) -> Option<String> {
    save_dirs(game)
        .iter()
        .find_map(|d| std::fs::read_to_string(d.join("history").join(format!("{start_time}.run"))).ok())
}

/// `profile2` is 2.
fn profile_number(dir: &Path) -> Option<i32> {
    dir.file_name()?.to_str()?.strip_prefix("profile")?.parse().ok()
}

/// The saves folder being played: the one whose run save changed last. With
/// no run save anywhere, the profile the game names, or else the folder
/// that changed last.
pub fn pick_save_dir(dirs: &[PathBuf], profile: Option<i32>) -> Option<PathBuf> {
    let newest = |dirs: &mut dyn Iterator<Item = &PathBuf>, when: &dyn Fn(&Path) -> Option<SystemTime>| {
        dirs.filter_map(|d| Some((when(d)?, d))).max_by_key(|(t, _)| *t).map(|(_, d)| d.clone())
    };
    let run_save = |d: &Path| modified(&d.join(RUN_SAVE));
    let named = |d: &&PathBuf| profile.is_some() && d.parent().and_then(profile_number) == profile;
    newest(&mut dirs.iter(), &run_save)
        .or_else(|| newest(&mut dirs.iter().filter(named), &|d| modified(d)))
        .or_else(|| newest(&mut dirs.iter(), &|d| modified(d)))
}

/// The run in progress, in a profile's saves folder.
pub const RUN_SAVE: &str = "current_run.save";

impl Link {
    /// Settles the saves folder, run save and history: the folder whose run
    /// save changed last, among the game's data folder and the one the mod
    /// named. The mod's is only a candidate: a save-path mod (UnifiedSavePath
    /// keeps modded saves in the vanilla `profile<N>`) makes the game write
    /// somewhere other than the folder its own lookup reports, which may
    /// still hold an old run. With no run save anywhere, the mod's folder,
    /// else the profile it names.
    pub fn locate(&mut self, game: &Path) {
        let named = self.save_dir.clone().filter(|d| d.is_dir());
        let mut dirs = save_dirs(game);
        if let Some(d) = named.as_ref().filter(|d| !dirs.contains(d)) {
            dirs.push(d.clone());
        }
        let newest_run = dirs
            .iter()
            .filter_map(|d| Some((modified(&d.join(RUN_SAVE))?, d)))
            .max_by_key(|(t, _)| *t)
            .map(|(_, d)| d.clone());
        let Some(dir) = newest_run.or(named).or_else(|| pick_save_dir(&dirs, self.profile_id)) else {
            return;
        };
        let run_save = dir.join(RUN_SAVE);
        self.run_save = run_save.is_file().then_some(run_save);
        self.history_dir = Some(dir.join("history"));
        self.save_dir = Some(dir);
    }
}

/// The link file, if the mod has written one.
pub fn read_link(path: &Path) -> Option<Link> {
    let text = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

/// When a file last changed, for noticing new writes.
pub fn modified(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).and_then(|m| m.modified()).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_mods_link_and_finds_the_saves() {
        let dir = std::env::temp_dir().join(format!("spire-link-{}", std::process::id()));
        let saves = dir.join("steam").join("1").join("modded").join("profile1").join("saves");
        std::fs::create_dir_all(saves.join("history")).unwrap();
        std::fs::write(saves.join("current_run.save"), "{\"run\":1}").unwrap();
        std::fs::write(saves.join("history").join("77.run"), "{\"done\":1}").unwrap();
        let json = serde_json::json!({
            "schema": 1, "mod_version": "0.1.0", "game_version": "v0.107.1", "in_progress": true,
            "profile_id": 1, "save_dir": saves, "run_save": saves.join("current_run.save"),
            "run_save_modified": 5, "history_dir": saves.join("history"),
            "act_index": 1, "act": "ACT.HIVE", "boss": "ENCOUNTER.X_BOSS", "unknown_future_field": true
        });
        let path = dir.join("game-link.json");
        std::fs::write(&path, json.to_string()).unwrap();
        let link = read_link(&path).expect("reads");
        assert!(link.modded_profile());
        assert_eq!(link.hint().boss.as_deref(), Some("ENCOUNTER.X_BOSS"));
        assert_eq!(link.read_run_save().as_deref(), Some("{\"run\":1}"));
        assert_eq!(link.read_finished(Some("77")).as_deref(), Some("{\"done\":1}"));
        assert_eq!(link.read_finished(None).as_deref(), Some("{\"done\":1}"));
        // Out of a run, the act says nothing.
        let idle = Link { in_progress: false, ..link };
        assert_eq!(idle.hint(), ActHint::default());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn follows_the_account_being_played() {
        // Two Steam accounts on one PC; the one being played writes its run
        // save last.
        let game = std::env::temp_dir().join(format!("spire-accounts-{}", std::process::id()));
        let steam = game.join("steam");
        let idle = steam.join("76561197960287930").join("profile1").join("saves");
        let played = steam.join("76561198030202747").join("profile1").join("saves");
        for d in [&idle, &played] {
            std::fs::create_dir_all(d).unwrap();
        }
        std::fs::write(idle.join(RUN_SAVE), "{}").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(20));
        std::fs::write(played.join(RUN_SAVE), "{}").unwrap();
        let mut link = Link { in_progress: true, profile_id: Some(1), ..Link::default() };
        link.locate(&game);
        assert_eq!(link.save_dir, Some(played.clone()));
        assert_eq!(link.run_save, Some(played.join(RUN_SAVE)));
        let _ = std::fs::remove_dir_all(game);
    }

    #[test]
    fn finds_the_saves_the_mod_left_out() {
        let game = std::env::temp_dir().join(format!("spire-game-{}", std::process::id()));
        let account = game.join("steam").join("76561197960287930");
        let plain = account.join("profile1").join("saves");
        let modded = account.join("modded").join("profile1").join("saves");
        let other = account.join("profile2").join("saves");
        for d in [&plain, &modded, &other] {
            std::fs::create_dir_all(d.join("history")).unwrap();
        }
        std::fs::create_dir_all(account.join("not-a-profile").join("saves")).unwrap();
        assert_eq!(save_dirs(&game).len(), 3);
        // No run save anywhere: the profile the game names.
        assert_eq!(pick_save_dir(&save_dirs(&game), Some(2)), Some(other.clone()));
        // A run save: that folder, whatever the profile says.
        std::fs::write(plain.join(RUN_SAVE), "{}").unwrap();
        assert_eq!(pick_save_dir(&save_dirs(&game), Some(2)), Some(plain.clone()));

        let mut link = Link { in_progress: true, profile_id: Some(1), ..Link::default() };
        link.locate(&game);
        assert_eq!(link.save_dir, Some(plain.clone()));
        assert_eq!(link.run_save, Some(plain.join(RUN_SAVE)));
        assert_eq!(link.history_dir, Some(plain.join("history")));
        // The mod names the modded folder, but the game writes the vanilla
        // one (UnifiedSavePath): the newer run save wins.
        let mut named = Link { save_dir: Some(modded.clone()), ..Link::default() };
        named.locate(&game);
        assert_eq!(named.save_dir, Some(plain.clone()));
        // Once the modded folder holds the newer run save, it wins.
        std::thread::sleep(std::time::Duration::from_millis(20));
        std::fs::write(modded.join(RUN_SAVE), "{}").unwrap();
        named.locate(&game);
        assert_eq!(named.save_dir, Some(modded.clone()));
        // No run save anywhere: the folder the mod named.
        std::fs::remove_file(plain.join(RUN_SAVE)).unwrap();
        std::fs::remove_file(modded.join(RUN_SAVE)).unwrap();
        let mut named = Link { save_dir: Some(other.clone()), profile_id: Some(1), ..Link::default() };
        named.locate(&game);
        assert_eq!(named.save_dir, Some(other));
        let _ = std::fs::remove_dir_all(game);
    }

    #[test]
    fn a_finished_run_is_found_in_any_profiles_history() {
        let game = std::env::temp_dir().join(format!("spire-finished-{}", std::process::id()));
        let unmodded = game.join("steam").join("7").join("profile1").join("saves");
        let modded = game.join("steam").join("7").join("modded").join("profile1").join("saves");
        std::fs::create_dir_all(unmodded.join("history")).unwrap();
        std::fs::create_dir_all(modded.join("history")).unwrap();
        std::fs::write(modded.join("history").join("1700.run"), "{\"win\":false}").unwrap();
        assert_eq!(find_finished(&game, 1700).as_deref(), Some("{\"win\":false}"));
        assert_eq!(find_finished(&game, 1699), None);
        std::fs::remove_dir_all(&game).ok();
    }
}
