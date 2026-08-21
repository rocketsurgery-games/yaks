//! Per-user, per-herd UI-state cache. This is *rebuildable* state (currently
//! the set of collapsed tree ids), so it lives under `$XDG_CACHE_HOME`
//! (default `~/.cache`) at `yaks/<slug>.json`, never in the herd and never
//! committed. Mirrors the Python TUI's `~/.cache/yaks/<slug>.json`.
//!
//! The slug is a stable hash of the absolute herd root. It need not match the
//! Python tool's sha1 — this cache belongs to the Rust binary alone, and a
//! slug change merely resets the cache (collapsed rows re-expand), which is
//! harmless for rebuildable state.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

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

pub fn load_collapsed(root: &Path) -> HashSet<String> {
    load_from(&ui_state_path(root))
}

pub fn save_collapsed(root: &Path, collapsed: &HashSet<String>) {
    save_to(&ui_state_path(root), collapsed);
}

fn load_from(path: &Path) -> HashSet<String> {
    let Ok(text) = fs::read_to_string(path) else {
        return HashSet::new();
    };
    match serde_json::from_str::<serde_json::Value>(&text) {
        Ok(v) => v
            .get("collapsed")
            .and_then(|c| c.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|x| x.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default(),
        Err(_) => HashSet::new(),
    }
}

fn save_to(path: &Path, collapsed: &HashSet<String>) {
    if let Some(dir) = path.parent() {
        let _ = fs::create_dir_all(dir);
    }
    let mut ids: Vec<&String> = collapsed.iter().collect();
    ids.sort();
    let payload = serde_json::json!({ "collapsed": ids });
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
        let mut set = HashSet::new();
        set.insert("yak-0001".to_string());
        set.insert("yak-0002".to_string());
        save_to(&path, &set);
        assert_eq!(load_from(&path), set);
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn missing_file_is_empty() {
        assert!(load_from(Path::new("/nonexistent/yaksrs/cache.json")).is_empty());
    }

    #[test]
    fn slug_is_stable_and_short() {
        let a = slug(Path::new("/tmp/some/herd"));
        let b = slug(Path::new("/tmp/some/herd"));
        assert_eq!(a, b);
        assert_eq!(a.len(), 12);
    }
}
