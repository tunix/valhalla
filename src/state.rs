use std::{collections::HashMap, path::PathBuf};

use fslock::LockFile;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub id: String,
    pub source: String,
    pub path: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub author: Option<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct State {
    #[serde(default)]
    pub last_refresh_at: u64,
    #[serde(default)]
    pub scheme: Option<String>,
    #[serde(default)]
    pub current_light: Option<String>,
    #[serde(default)]
    pub current_dark: Option<String>,
    #[serde(default)]
    pub history: Vec<HistoryEntry>,
    /// Recently used candidate ids per source, to avoid immediate repeats.
    #[serde(default)]
    pub used: HashMap<String, Vec<String>>,
}

fn state_dir() -> PathBuf {
    let base = std::env::var_os("XDG_STATE_HOME")
        .filter(|p| !p.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| glib::home_dir().join(".local").join("state"));
    base.join("valhalla")
}

#[derive(Debug, Clone)]
pub struct StateStore {
    dir: PathBuf,
}

impl StateStore {
    pub fn new() -> Self {
        let dir = state_dir();
        let _ = std::fs::create_dir_all(&dir);
        Self { dir }
    }

    fn lock_path(&self) -> PathBuf {
        self.dir.join("refresh.lock")
    }

    fn path(&self) -> PathBuf {
        self.dir.join("state.json")
    }

    pub fn read(&self) -> State {
        std::fs::read(self.path())
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default()
    }

    /// Take the cross-process refresh lock. Returns `None` when another
    /// Valhalla process is currently refreshing.
    pub fn try_lock(&self) -> Option<LockedState<'_>> {
        let mut file = LockFile::open(&self.lock_path()).ok()?;
        if !file.try_lock().ok()? {
            return None;
        }
        let state = self.read();
        Some(LockedState {
            store: self,
            _file: file,
            state,
        })
    }
}

pub struct LockedState<'a> {
    store: &'a StateStore,
    _file: LockFile,
    state: State,
}

impl LockedState<'_> {
    pub fn get(&self) -> &State {
        &self.state
    }

    pub fn get_mut(&mut self) -> &mut State {
        &mut self.state
    }

    pub fn save(&self) {
        let tmp = self.store.path().with_extension("json.tmp");
        if let Ok(bytes) = serde_json::to_vec_pretty(&self.state) {
            if std::fs::write(&tmp, bytes).is_ok() {
                let _ = std::fs::rename(&tmp, self.store.path());
            }
        }
    }
}
