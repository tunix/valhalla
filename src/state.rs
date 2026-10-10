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

    /// A store rooted at an explicit directory (tests, sandboxes).
    #[allow(dead_code)] // test-only seam; kept pub for future daemon use
    pub fn at_dir(dir: PathBuf) -> Self {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_store(tag: &str) -> StateStore {
        let dir = std::env::temp_dir().join(format!("valhalla-state-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        StateStore::at_dir(dir)
    }

    fn entry(id: &str) -> HistoryEntry {
        HistoryEntry {
            id: id.into(),
            source: "wallhaven".into(),
            path: format!("/tmp/{id}.jpg"),
            title: Some("Title".into()),
            author: None,
        }
    }

    #[test]
    fn empty_store_reads_default() {
        let store = temp_store("empty");
        let state = store.read();
        assert_eq!(state.last_refresh_at, 0);
        assert!(state.history.is_empty());
        assert!(state.scheme.is_none());
    }

    #[test]
    fn corrupt_file_falls_back_to_default() {
        let store = temp_store("corrupt");
        std::fs::write(store.path(), b"{not json").unwrap();
        assert_eq!(store.read().history.len(), 0);
    }

    #[test]
    fn missing_file_falls_back_to_default() {
        let store = temp_store("missing");
        // No state.json written at all.
        assert!(store.read().current_light.is_none());
    }

    #[test]
    fn roundtrip_preserves_fields() {
        let store = temp_store("roundtrip");
        {
            let mut locked = store.try_lock().expect("lock");
            locked.get_mut().last_refresh_at = 1_234;
            locked.get_mut().scheme = Some("dark".into());
            locked.get_mut().current_dark = Some("/w/dark.jpg".into());
            locked.get_mut().history.push(entry("abc"));
            locked
                .get_mut()
                .used
                .insert("wallhaven".into(), vec!["abc".into()]);
            locked.save();
        }
        let state = store.read();
        assert_eq!(state.last_refresh_at, 1_234);
        assert_eq!(state.scheme.as_deref(), Some("dark"));
        assert_eq!(state.current_dark.as_deref(), Some("/w/dark.jpg"));
        assert_eq!(state.history.len(), 1);
        assert_eq!(state.history[0].id, "abc");
        assert_eq!(state.history[0].source, "wallhaven");
        assert_eq!(state.used.get("wallhaven").map(Vec::len), Some(1));
    }

    #[test]
    fn save_writes_valid_json_no_tmp_leftover() {
        let store = temp_store("notmp");
        {
            let mut locked = store.try_lock().expect("lock");
            locked.get_mut().current_light = Some("/w/light.jpg".into());
            locked.save();
        }
        assert!(store.path().exists());
        let tmp = store.path().with_extension("json.tmp");
        assert!(!tmp.exists(), "tmp file should be renamed away");
    }

    #[test]
    fn second_lock_while_held_returns_none() {
        let store = temp_store("lock2");
        let first = store.try_lock().expect("first lock");
        let second = store.try_lock();
        assert!(second.is_none(), "second concurrent lock must fail");
        drop(first);
    }

    #[test]
    fn used_list_roundtrip() {
        let store = temp_store("used");
        {
            let mut locked = store.try_lock().expect("lock");
            locked
                .get_mut()
                .used
                .entry("unsplash".into())
                .or_default()
                .push("p1".into());
            locked.save();
        }
        let state = store.read();
        assert_eq!(state.used.get("unsplash"), Some(&vec!["p1".to_string()]));
    }
}
