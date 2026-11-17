//! Local files: downloaded data, user overrides, settings and the saved run.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use spire_core::{Catalog, DataPack, Playbook, Run};

use crate::fetch::Source;
use crate::{DataError, bundled_pack, bundled_playbook, parse_pack, parse_playbook};

const DOWNLOADED: &str = "downloaded.json";
const USER_PACK: &str = "user.json";
const PLAYBOOK: &str = "playbook.json";
const SETTINGS: &str = "settings.json";
const RUN: &str = "run.json";

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub source: Source,
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
        let layers: Vec<DataPack> = std::iter::once(bundled_pack())
            .chain(self.optional_pack(DOWNLOADED, &mut warnings))
            .chain(self.optional_pack(USER_PACK, &mut warnings))
            .collect();
        let playbook = match self.read(PLAYBOOK).map(|r| r.and_then(|s| parse_playbook(&s))) {
            Some(Ok(p)) => p,
            Some(Err(e)) => {
                warnings.push(format!("Ignored {PLAYBOOK}, using the default: {e}"));
                bundled_playbook()
            }
            None => bundled_playbook(),
        };
        Loaded { catalog: Catalog::from_layers(layers), playbook, warnings }
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

    pub fn settings(&self) -> Settings {
        self.read(SETTINGS).and_then(Result::ok).and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
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
        assert_eq!(loaded.catalog.layers.len(), 2);
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
