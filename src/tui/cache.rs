//! Per-user, per-herd UI-state cache. This is *rebuildable* state (the set of
//! collapsed tree ids, plus per-view herd-scope overrides), so it lives under
//! `$XDG_CACHE_HOME` (default `~/.cache`) at `yaks/<slug>.json`, never in the
//! herd and never committed.
//!
//! The slug is a stable hash of the absolute herd root. A slug change merely
//! resets the cache (collapsed rows re-expand, herd overrides revert to auto),
//! which is harmless for rebuildable state.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use crate::tui::view::HerdScope;

/// The full rebuildable UI state persisted per herd. Fields are independent;
/// both are written together so neither clobbers the other.
#[derive(Default, Clone, PartialEq, Debug)]
pub struct UiState {
    /// Ids of collapsed tree parents.
    pub collapsed: HashSet<String>,
    /// Per-view herd-scope overrides, keyed by `View::key`. A missing entry
    /// means the view inherits [`HerdScope::DEFAULT`] ("auto").
    pub herd: HashMap<String, HerdScope>,
}

/// FNV-1a 64-bit over the absolute root path → 12 hex chars.
fn slug(root: &Path) -> String {
    let abs = fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in abs.to_string_lossy().bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{h:016x}")[..12].to_string()
}

fn cache_home() -> PathBuf {
    if let Ok(x) = std::env::var("XDG_CACHE_HOME") {
        if !x.is_empty() {
            return PathBuf::from(x);
        }
    }
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home).join(".cache");
    }
    std::env::temp_dir()
}

/// Absolute path to this herd's UI-state cache file.
pub fn ui_state_path(root: &Path) -> PathBuf {
    cache_home()
        .join("yaks")
        .join(format!("{}.json", slug(root)))
}

pub fn load(root: &Path) -> UiState {
    load_from(&ui_state_path(root))
}

pub fn save(root: &Path, state: &UiState) {
    save_to(&ui_state_path(root), state);
}

fn load_from(path: &Path) -> UiState {
    let Ok(text) = fs::read_to_string(path) else {
        return UiState::default();
    };
    let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) else {
        return UiState::default();
    };
    let collapsed = v
        .get("collapsed")
        .and_then(|c| c.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|x| x.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    let herd = v
        .get("herd")
        .and_then(|h| h.as_object())
        .map(|obj| {
            obj.iter()
                .filter_map(|(k, val)| {
                    val.as_str()
                        .and_then(HerdScope::parse)
                        .map(|s| (k.clone(), s))
                })
                .collect()
        })
        .unwrap_or_default();
    UiState { collapsed, herd }
}

fn save_to(path: &Path, state: &UiState) {
    if let Some(dir) = path.parent() {
        let _ = fs::create_dir_all(dir);
    }
    let mut ids: Vec<&String> = state.collapsed.iter().collect();
    ids.sort();
    let mut keys: Vec<&String> = state.herd.keys().collect();
    keys.sort();
    let mut herd = serde_json::Map::new();
    for k in keys {
        herd.insert(k.clone(), serde_json::Value::from(state.herd[k].as_str()));
    }
    let payload = serde_json::json!({ "collapsed": ids, "herd": herd });
    let _ = fs::write(path, payload.to_string());
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static SEQ: AtomicU64 = AtomicU64::new(0);

    fn temp_file() -> PathBuf {
        let mut p = std::env::temp_dir();
        let n = SEQ.fetch_add(1, Ordering::Relaxed);
        p.push(format!("yaksrs-cache-{}-{}.json", std::process::id(), n));
        p
    }

    #[test]
    fn round_trip_collapsed() {
        let path = temp_file();
        let mut st = UiState::default();
        st.collapsed.insert("yak-0001".to_string());
        st.collapsed.insert("yak-0002".to_string());
        save_to(&path, &st);
        assert_eq!(load_from(&path), st);
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn round_trip_herd_overrides() {
        let path = temp_file();
        let mut st = UiState::default();
        st.herd.insert("status:hairy".to_string(), HerdScope::All);
        st.herd
            .insert("status:shaving".to_string(), HerdScope::Lone);
        save_to(&path, &st);
        assert_eq!(load_from(&path).herd, st.herd);
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn missing_file_is_empty() {
        let st = load_from(Path::new("/nonexistent/yaksrs/cache.json"));
        assert!(st.collapsed.is_empty() && st.herd.is_empty());
    }

    #[test]
    fn slug_is_stable_and_short() {
        let a = slug(Path::new("/tmp/some/herd"));
        let b = slug(Path::new("/tmp/some/herd"));
        assert_eq!(a, b);
        assert_eq!(a.len(), 12);
    }
}
