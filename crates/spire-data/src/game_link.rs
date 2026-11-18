//! The companion mod's link file, and the game's run saves it points to.
//!
//! The mod (`mod/SpireDashboardLink`) writes `game-link.json` in
//! `<local app data>/SpireDashboard/` whenever the game's state that matters
//! here changes: whether a run is in progress, where the profile's saves
//! are (the game keeps a separate `modded/` profile while mods are on),
//! when the run was last saved, and the act and its boss.
//!
//! Only read while the player has turned on monitoring. Nothing here writes
//! anywhere.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use serde::Deserialize;

use crate::game_run::ActHint;

/// The link file's contents (schema 1).
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct Link {
    pub schema: u32,
    pub mod_version: String,
    pub game_version: Option<String>,
    pub in_progress: bool,
    pub profile_id: Option<i32>,
    pub save_dir: Option<PathBuf>,
    pub run_save: Option<PathBuf>,
    pub run_save_modified: Option<i64>,
    pub history_dir: Option<PathBuf>,
    pub act_index: Option<u8>,
    pub act: Option<String>,
    pub boss: Option<String>,
    pub second_boss: Option<String>,
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
}
