//! Local files: downloaded data, user overrides, settings and the saved run.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use spire_core::analysis::tags::Tag;
use spire_core::playbook::Role;
use spire_core::{Catalog, DataPack, Playbook, Run};

use crate::fetch::Source;
use crate::{DataError, bundled_pack, bundled_playbook, parse_pack, parse_playbook};

const DOWNLOADED: &str = "downloaded.json";
const USER_PACK: &str = "user.json";
const PLAYBOOK: &str = "playbook.json";
const SETTINGS: &str = "settings.json";
const RUN: &str = "run.json";
const RUNS_DIR: &str = "runs";
const MY_ROLES: &str = "my_roles.json";
/// The player's own tag overrides, by tag name.
const MY_TAGS: &str = "my_tags.json";

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub source: Source,
    /// Appearance (a look name from the app's style table).
    pub look: Option<String>,
    /// The "Larger text" readability option.
    pub large_text: bool,
    /// Use the operating system's title bar instead of the app's own.
    pub system_title_bar: bool,
    /// Follow the system's light/dark mode with `light_look` and `dark_look`.
    pub match_system: bool,
    pub light_look: Option<String>,
    pub dark_look: Option<String>,
    /// Monitored runs: follow the game through the companion mod.
    pub follow_game: bool,
}

/// The app's data directory. All reads tolerate missing files.
#[derive(Clone, Debug)]
pub struct Store {
    dir: PathBuf,
}

/// What loading found, including non-fatal problems to show the user.
pub struct Loaded {
    pub catalog: Catalog,
    pub playbook: Playbook,
    pub warnings: Vec<String>,
}

impl Store {
    /// The platform data dir (e.g. `~/.local/share/spire-dashboard`), or
    /// `./spire-dashboard-data` if the platform has none.
    pub fn default_location() -> Store {
        let dir = directories::ProjectDirs::from("", "", "spire-dashboard")
            .map_or_else(|| PathBuf::from("spire-dashboard-data"), |d| d.data_dir().to_path_buf());
        Store { dir }
    }

    pub fn at(dir: impl Into<PathBuf>) -> Store {
        Store { dir: dir.into() }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    fn path(&self, name: &str) -> PathBuf {
        self.dir.join(name)
    }

    fn read(&self, name: &str) -> Option<Result<String, DataError>> {
        let path = self.path(name);
        path.exists().then(|| fs::read_to_string(&path).map_err(DataError::from))
    }

    /// Writes atomically (temp file + rename) so a crash never leaves a
    /// half-written file behind.
    fn write(&self, name: &str, contents: &str) -> Result<(), DataError> {
        fs::create_dir_all(&self.dir)?;
        let tmp = self.path(&format!("{name}.tmp"));
        fs::write(&tmp, contents)?;
        fs::rename(&tmp, self.path(name))?;
        Ok(())
    }

    fn write_json<T: Serialize>(&self, name: &str, value: &T) -> Result<(), DataError> {
        let json = serde_json::to_string_pretty(value).map_err(|e| DataError::Parse(e.to_string()))?;
        self.write(name, &json)
    }

    fn optional_pack(&self, name: &str, warnings: &mut Vec<String>) -> Option<DataPack> {
        match self.read(name)?.and_then(|s| parse_pack(&s)) {
            Ok(pack) => Some(pack),
            Err(e) => {
                warnings.push(format!("Ignored {name}: {e}"));
                None
            }
        }
    }

    /// Builds the catalog from all layers and loads the playbook.
    pub fn load(&self) -> Loaded {
        let mut warnings = Vec::new();
        let mut layers: Vec<DataPack> =
            std::iter::once(bundled_pack()).chain(self.optional_pack(DOWNLOADED, &mut warnings)).collect();
        // Sourced fixes for gaps the data above still has.
        layers.extend(crate::curated::pack(&layers));
        layers.extend(self.optional_pack(USER_PACK, &mut warnings));
        let playbook = match self.read(PLAYBOOK).map(|r| r.and_then(|s| parse_playbook(&s))) {
            Some(Ok(p)) => p,
            Some(Err(e)) => {
                warnings.push(format!("Ignored {PLAYBOOK}, using the default: {e}"));
                bundled_playbook()
            }
            None => bundled_playbook(),
        };
        let mut playbook = playbook;
        // Saved by name, so roles from before the role change still load:
        // old names map onto the new roles, and "consistency" is dropped.
        let parse = |s: String| {
            serde_json::from_str::<std::collections::BTreeMap<spire_core::CardId, Vec<String>>>(&s)
                .map_err(|e| DataError::Parse(e.to_string()))
        };
        match self.read(MY_ROLES).map(|r| r.and_then(parse)) {
            Some(Ok(saved)) => {
                playbook.personal_roles = saved
                    .into_iter()
                    .map(|(card, names)| {
                        let roles: Vec<Role> = names.iter().filter_map(|n| Role::from_name(n)).collect();
                        (card, roles)
                    })
                    .filter(|(_, roles)| !roles.is_empty())
                    .collect();
            }
            Some(Err(e)) => warnings.push(format!("Ignored {MY_ROLES}: {e}")),
            None => {}
        }
        // Tags are saved by name too, so a renamed or dropped tag is skipped
        // rather than failing the whole file. An empty list is kept: it
        // means "this card has no tags".
        match self.read(MY_TAGS).map(|r| r.and_then(parse)) {
            Some(Ok(saved)) => {
                playbook.personal_tags = saved
                    .into_iter()
                    .map(|(card, names)| (card, names.iter().filter_map(|n| Tag::from_name(n)).collect()))
                    .collect();
            }
            Some(Err(e)) => warnings.push(format!("Ignored {MY_TAGS}: {e}")),
            None => {}
        }
        Loaded { catalog: Catalog::from_layers(layers), playbook, warnings }
    }

    /// Saves the player's personal role overrides (a separate layer on top
    /// of the shipped playbook).
    pub fn save_personal_roles(
        &self,
        roles: &std::collections::BTreeMap<spire_core::CardId, Vec<spire_core::playbook::Role>>,
    ) -> Result<(), DataError> {
        self.write_json(MY_ROLES, roles)
    }

    /// Saves the player's personal tag overrides, beside their roles.
    pub fn save_personal_tags(
        &self,
        tags: &std::collections::BTreeMap<spire_core::CardId, Vec<Tag>>,
    ) -> Result<(), DataError> {
        self.write_json(MY_TAGS, tags)
    }

    pub fn save_downloaded(&self, pack: &DataPack) -> Result<(), DataError> {
        self.write_json(DOWNLOADED, pack)
    }

    /// Drops downloaded data, falling back to the bundled pack.
    pub fn clear_downloaded(&self) -> Result<(), DataError> {
        match fs::remove_file(self.path(DOWNLOADED)) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e.into()),
            _ => Ok(()),
        }
    }

    /// Writes the default playbook to the data dir for the user to edit.
    pub fn export_playbook(&self) -> Result<PathBuf, DataError> {
        let path = self.path(PLAYBOOK);
        if !path.exists() {
            self.write_json(PLAYBOOK, &bundled_playbook())?;
        }
        Ok(path)
    }

    /// Whether settings were ever saved (false on a first run).
    pub fn has_settings(&self) -> bool {
        self.path(SETTINGS).exists()
    }

    pub fn settings(&self) -> Settings {
        let s: Settings =
            self.read(SETTINGS).and_then(Result::ok).and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
        Settings { source: s.source.upgraded(), ..s }
    }

    pub fn save_settings(&self, settings: &Settings) -> Result<(), DataError> {
        self.write_json(SETTINGS, settings)
    }

    pub fn load_run(&self) -> Option<Result<Run, DataError>> {
        self.read(RUN).map(|r| r.and_then(|s| serde_json::from_str(&s).map_err(|e| DataError::Parse(e.to_string()))))
    }

    pub fn save_run(&self, run: &Run) -> Result<(), DataError> {
        self.write_json(RUN, run)
    }

    /// Saves a finished run into the history folder and returns its id.
    pub fn archive_run(&self, run: &Run) -> Result<String, DataError> {
        let dir = self.dir.join(RUNS_DIR);
        fs::create_dir_all(&dir)?;
        let stamp = run.started_at.clone().unwrap_or_else(crate::now_rfc3339);
        let base: String = stamp.chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '-' }).collect();
        let mut id = base.clone();
        let mut n = 1;
        while dir.join(format!("{id}.json")).exists() {
            n += 1;
            id = format!("{base}-{n}");
        }
        let json = serde_json::to_string_pretty(run).map_err(|e| DataError::Parse(e.to_string()))?;
        let tmp = dir.join(format!("{id}.json.tmp"));
        fs::write(&tmp, json)?;
        fs::rename(&tmp, dir.join(format!("{id}.json")))?;
        Ok(id)
    }

    /// Archived runs, newest first. Unreadable files are skipped.
    /// Replaces an archived run (edited in History). Written to a temporary
    /// file first, so a failed write never leaves half a run.
    pub fn update_archived(&self, id: &str, run: &Run) -> Result<(), DataError> {
        let dir = self.dir.join(RUNS_DIR);
        let path = archived_path(&dir, id)?;
        let json = serde_json::to_string_pretty(run).map_err(|e| DataError::Parse(e.to_string()))?;
        let tmp = dir.join(format!("{id}.json.tmp"));
        fs::write(&tmp, json)?;
        fs::rename(&tmp, path)?;
        Ok(())
    }

    /// Deletes an archived run for good.
    pub fn delete_archived(&self, id: &str) -> Result<(), DataError> {
        let path = archived_path(&self.dir.join(RUNS_DIR), id)?;
        fs::remove_file(path)?;
        Ok(())
    }

    pub fn archived_runs(&self) -> Vec<(String, Run)> {
        let Ok(entries) = fs::read_dir(self.dir.join(RUNS_DIR)) else { return Vec::new() };
        let mut runs: Vec<(String, Run)> = entries
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == "json"))
            .filter_map(|p| {
                let id = p.file_stem()?.to_string_lossy().into_owned();
                let run: Run = serde_json::from_str(&fs::read_to_string(&p).ok()?).ok()?;
                Some((id, run))
            })
            .collect();
        runs.sort_by(|a, b| b.0.cmp(&a.0));
        runs
    }

    /// A writer thread for this store's run file (see [`RunSaver`]).
    pub fn run_saver(&self) -> RunSaver {
        RunSaver::spawn(self.clone())
    }
}

/// Saves the run off the UI thread. Slow storage (SD cards, spinning disks,
/// network homes) can take tens of milliseconds per write; the UI should
/// never wait for it.
///
/// Serialising happens on the caller (microseconds); only the write moves.
/// Pending saves coalesce: if several arrive while one is being written,
/// only the newest is written next. Dropping the saver flushes and joins,
/// so the last state reaches disk on exit.
pub struct RunSaver {
    tx: Option<std::sync::mpsc::Sender<String>>,
    errors: std::sync::mpsc::Receiver<String>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl RunSaver {
    fn spawn(store: Store) -> RunSaver {
        let (tx, rx) = std::sync::mpsc::channel::<String>();
        let (err_tx, errors) = std::sync::mpsc::channel();
        let thread = std::thread::Builder::new()
            .name("run-saver".into())
            .spawn(move || {
                while let Ok(mut json) = rx.recv() {
                    // Skip to the newest pending save.
                    while let Ok(newer) = rx.try_recv() {
                        json = newer;
                    }
                    if let Err(e) = store.write(RUN, &json) {
                        let _ = err_tx.send(e.to_string());
                    }
                }
            })
            .ok();
        RunSaver { tx: Some(tx), errors, thread }
    }

    /// Queues a save. Returns an error only if serialising fails.
    pub fn save(&self, run: &Run) -> Result<(), DataError> {
        let json = serde_json::to_string_pretty(run).map_err(|e| DataError::Parse(e.to_string()))?;
        match (&self.tx, &self.thread) {
            (Some(tx), Some(_)) => {
                let _ = tx.send(json);
                Ok(())
            }
            // No thread (spawn failed): fall back to a blocking write.
            _ => Err(DataError::Io(std::io::Error::other("run saver thread is not running"))),
        }
    }

    /// Errors from finished writes, oldest first.
    pub fn take_errors(&self) -> Vec<String> {
        self.errors.try_iter().collect()
    }
}

impl Drop for RunSaver {
    fn drop(&mut self) {
        drop(self.tx.take());
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

/// The file of an archived run. Ids are file stems the store made; anything
/// that could leave the runs folder is refused.
fn archived_path(dir: &std::path::Path, id: &str) -> Result<std::path::PathBuf, DataError> {
    if id.is_empty() || !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        return Err(DataError::Parse(format!("not a run id: {id}")));
    }
    let path = dir.join(format!("{id}.json"));
    if !path.exists() {
        return Err(DataError::Parse(format!("no saved run {id}")));
    }
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use spire_core::RunEvent;

    fn temp_store(name: &str) -> Store {
        let dir = std::env::temp_dir().join(format!("spire-store-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        Store::at(dir)
    }

    #[test]
    fn user_layer_overrides_bundled() {
        let store = temp_store("layers");
        let bundled = bundled_pack();
        let mut card = bundled.cards[0].clone();
        card.name = "Renamed".into();
        let user = DataPack { format: 1, source: "user".into(), cards: vec![card.clone()], ..Default::default() };
        store.write_json(USER_PACK, &user).unwrap();

        let loaded = store.load();
        assert!(loaded.warnings.is_empty());
        assert_eq!(loaded.catalog.cards[&card.id].name, "Renamed");
        // Bundled, the curated fixes, then the user's.
        let sources: Vec<&str> = loaded.catalog.layers.iter().map(|l| l.source.as_str()).collect();
        assert_eq!(sources, ["spire-codex", "curated", "user"]);
    }

    #[test]
    fn bad_files_warn_instead_of_failing() {
        let store = temp_store("bad");
        store.write(DOWNLOADED, "{not json").unwrap();
        store.write(PLAYBOOK, "[]").unwrap();
        let loaded = store.load();
        assert_eq!(loaded.warnings.len(), 2);
        assert!(!loaded.catalog.cards.is_empty());
    }

    #[test]
    fn saver_writes_latest_run_on_drop() {
        let store = temp_store("saver");
        let cat = Catalog::from_layers([bundled_pack()]);
        let character = cat.characters.keys().next().unwrap().clone();
        let mut run = Run::default();
        run.push(RunEvent::start(&cat, &character, 0).unwrap());
        {
            let saver = store.run_saver();
            for gold in 0..50 {
                run.push(RunEvent::Adjust(vec![spire_core::run::Change::SetGold(gold)]));
                saver.save(&run).unwrap();
            }
        }
        assert_eq!(store.load_run().unwrap().unwrap(), run);
    }

    #[test]
    fn archived_runs_list_newest_first() {
        let store = temp_store("archive");
        let cat = Catalog::from_layers([bundled_pack()]);
        let character = cat.characters.keys().next().unwrap().clone();
        let mut ids = Vec::new();
        for (i, at) in ["2026-01-01T00:00:00Z", "2026-02-01T00:00:00Z"].into_iter().enumerate() {
            let mut run = Run { started_at: Some(at.into()), ..Run::default() };
            run.push(RunEvent::start(&cat, &character, i as u32).unwrap());
            run.push(RunEvent::End { outcome: spire_core::run::Outcome::Abandoned, at: None });
            ids.push(store.archive_run(&run).unwrap());
        }
        let listed = store.archived_runs();
        assert_eq!(listed.len(), 2);
        assert_eq!(listed[0].0, ids[1], "newest first");
        assert_eq!(listed[0].1.summary().ascension, 1);
        // Same start time twice gets a distinct id.
        let again = store.archive_run(&listed[0].1).unwrap();
        assert_ne!(again, ids[1]);

        // Edits replace the file; deletes remove it; odd ids are refused.
        let edited = Run { notes: Some("misclick".into()), ..listed[0].1.clone() };
        store.update_archived(&ids[1], &edited).unwrap();
        let reread = store.archived_runs();
        assert_eq!(reread.iter().find(|(id, _)| *id == ids[1]).unwrap().1.notes.as_deref(), Some("misclick"));
        store.delete_archived(&again).unwrap();
        assert_eq!(store.archived_runs().len(), 2);
        assert!(store.update_archived("../run", &edited).is_err());
        assert!(store.delete_archived("missing").is_err());
    }

    #[test]
    fn personal_roles_saved_before_the_role_change_migrate() {
        let store = temp_store("old-roles");
        store
            .write(
                MY_ROLES,
                r#"{"STRIKE_IRONCLAD": ["frontload_damage", "consistency"], "DEFEND_IRONCLAD": ["consistency"],
                   "BASH": ["scaling_defense"]}"#,
            )
            .unwrap();
        let loaded = store.load();
        assert!(loaded.warnings.is_empty(), "{:?}", loaded.warnings);
        let roles = &loaded.playbook.personal_roles;
        use spire_core::playbook::Role;
        assert_eq!(roles[&spire_core::CardId::new("STRIKE_IRONCLAD")], vec![Role::FastDamage]);
        assert_eq!(roles[&spire_core::CardId::new("BASH")], vec![Role::Mitigation]);
        assert!(!roles.contains_key(&spire_core::CardId::new("DEFEND_IRONCLAD")), "consistency alone: no override");
    }

    #[test]
    fn personal_tags_layer_over_the_reading_and_skip_unknown_names() {
        use spire_core::analysis::tags::{self, TagSource};
        let store = temp_store("tags");
        let cat = Catalog::from_layers([bundled_pack()]);
        // A Defend: starter Strikes fill no role, whatever their tags.
        let strike = cat.cards.values().find(|c| c.name == "Defend").unwrap().clone();
        store.write(MY_TAGS, &format!(r#"{{"{}": ["multi_hit", "no_such_tag"]}}"#, strike.id)).unwrap();
        let loaded = store.load();
        assert!(loaded.warnings.is_empty(), "{:?}", loaded.warnings);
        let (t, source) = tags::of(&strike, false, &loaded.playbook);
        assert_eq!(source, TagSource::Personal);
        assert_eq!(t.iter().map(|t| t.tag).collect::<Vec<_>>(), vec![Tag::MultiHit]);
        assert_eq!(loaded.playbook.roles_of(&strike), vec![Role::Chip], "roles follow your tags");
        let mut saved = loaded.playbook.personal_tags.clone();
        saved.insert(strike.id.clone(), vec![]);
        store.save_personal_tags(&saved).unwrap();
        assert!(tags::tags(&strike, false, &store.load().playbook).is_empty(), "an empty override clears the tags");
    }

    #[test]
    fn personal_roles_layer_over_the_playbook() {
        use spire_core::playbook::{Role, RoleSource};
        let store = temp_store("roles");
        let cat = Catalog::from_layers([bundled_pack()]);
        let strike = cat.cards.values().find(|c| c.name == "Strike").unwrap().clone();
        let mut roles = std::collections::BTreeMap::new();
        roles.insert(strike.id.clone(), vec![Role::Chip]);
        store.save_personal_roles(&roles).unwrap();
        let loaded = store.load();
        assert_eq!(loaded.playbook.roles_of(&strike), vec![Role::Chip]);
        assert!(matches!(loaded.playbook.role_source(&strike), RoleSource::Personal(_)));
        // The shipped playbook file is untouched.
        assert!(bundled_playbook().personal_roles.is_empty());
    }

    #[test]
    fn run_round_trips() {
        let store = temp_store("run");
        let cat = Catalog::from_layers([bundled_pack()]);
        let character = cat.characters.keys().next().unwrap().clone();
        let mut run = Run::default();
        run.push(RunEvent::start(&cat, &character, 0).unwrap());
        store.save_run(&run).unwrap();
        assert_eq!(store.load_run().unwrap().unwrap(), run);
    }
}
