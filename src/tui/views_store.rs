//! Persistent, per-user view list + working set — ported from Python
//! `yaktui.views_store`. Durable user intent (view order, pins, renames, custom
//! views, and the starred working set) lives under `$XDG_CONFIG_HOME/yaks/
//! <slug>/` (default `~/.config`), NOT the rebuildable cache. Built-in views
//! are always defined in code; the stored file is an overlay reconciled against
//! the current built-ins on every load. Reads never panic; writes are best-effort.

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use crate::filter::FilterSpec;
use crate::model::Status;

use super::view::{self, SortDir, SortField, View, default_views, short_hash};

const VERSION: u64 = 2;

fn slug(root: &Path) -> String {
    let abs = fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in abs.to_string_lossy().bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{h:016x}")[..12].to_string()
}

fn config_home() -> PathBuf {
    if let Ok(x) = std::env::var("XDG_CONFIG_HOME") {
        if !x.is_empty() {
            return PathBuf::from(x);
        }
    }
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home).join(".config");
    }
    std::env::temp_dir()
}

fn config_dir(root: &Path) -> PathBuf {
    config_home().join("yaks").join(slug(root))
}

pub fn views_path(root: &Path) -> PathBuf {
    config_dir(root).join("views.json")
}

pub fn working_set_path(root: &Path) -> PathBuf {
    config_dir(root).join("working_set.json")
}

// -- (de)serialization -------------------------------------------------------

fn status_from_key(k: &str) -> Option<Status> {
    Some(match k {
        "hairy" => Status::Hairy,
        "shaving" => Status::Shaving,
        "shorn" => Status::Shorn,
        "dead" => Status::Dead,
        _ => return None,
    })
}

fn spec_to_json(s: &FilterSpec) -> Value {
    let mut statuses: Vec<&str> = s.statuses.iter().map(|&st| view::status_key(st)).collect();
    statuses.sort_unstable();
    let mut types = s.types.clone();
    types.sort();
    let mut priorities = s.priorities.clone();
    priorities.sort_unstable();
    json!({
        "statuses": statuses,
        "types": types,
        "priorities": priorities,
        "labels": s.labels,
        "search": s.search,
        "ready_only": s.ready_only,
        "tangled_only": s.tangled_only,
        "needs_only": s.needs_only,
        "parent": s.parent,
    })
}

fn str_vec(v: Option<&Value>) -> Vec<String> {
    v.and_then(|x| x.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|e| e.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

fn spec_from_json(d: Option<&Value>) -> FilterSpec {
    let Some(d) = d else {
        return FilterSpec::default();
    };
    let statuses = str_vec(d.get("statuses"))
        .iter()
        .filter_map(|s| status_from_key(s))
        .collect();
    let priorities = d
        .get("priorities")
        .and_then(|x| x.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|e| e.as_u64().map(|n| n as u8))
                .collect()
        })
        .unwrap_or_default();
    let opt_str = |k: &str| {
        d.get(k)
            .and_then(|x| x.as_str())
            .filter(|s| !s.is_empty())
            .map(String::from)
    };
    FilterSpec {
        statuses,
        types: str_vec(d.get("types")),
        priorities,
        labels: str_vec(d.get("labels")),
        search: opt_str("search"),
        ready_only: d
            .get("ready_only")
            .and_then(|x| x.as_bool())
            .unwrap_or(false),
        tangled_only: d
            .get("tangled_only")
            .and_then(|x| x.as_bool())
            .unwrap_or(false),
        needs_only: d
            .get("needs_only")
            .and_then(|x| x.as_bool())
            .unwrap_or(false),
        parent: opt_str("parent"),
    }
}

fn view_to_json(v: &View) -> Value {
    json!({
        "key": v.key,
        "name": v.name,
        "status": v.status.map(view::status_key),
        "builtin": v.builtin,
        "pinned": v.pinned,
        "spec": spec_to_json(&v.spec),
        "sort_by": v.sort_by.map(|s| s.as_str()),
        "sort_dir": match v.sort_dir { SortDir::Asc => "asc", SortDir::Desc => "desc" },
        "limit": v.limit,
    })
}

fn view_from_json(d: &Value) -> Option<View> {
    let name = d.get("name")?.as_str()?.to_string();
    let key = d
        .get("key")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();
    let sort_dir = match d.get("sort_dir").and_then(|x| x.as_str()) {
        Some("asc") => SortDir::Asc,
        _ => SortDir::Desc,
    };
    Some(View {
        name,
        key,
        status: d
            .get("status")
            .and_then(|x| x.as_str())
            .and_then(status_from_key),
        builtin: d.get("builtin").and_then(|x| x.as_bool()).unwrap_or(false),
        pinned: d.get("pinned").and_then(|x| x.as_bool()).unwrap_or(true),
        spec: spec_from_json(d.get("spec")),
        sort_by: d
            .get("sort_by")
            .and_then(|x| x.as_str())
            .and_then(SortField::parse),
        sort_dir,
        limit: d.get("limit").and_then(|x| x.as_u64()).map(|n| n as usize),
    })
}

// -- reconcile / load / save -------------------------------------------------

/// Merge a stored overlay with the code-defined default views: order follows
/// the overlay; built-in structure comes from code (only name/pinned overlaid);
/// omitted built-ins are appended; custom views are rebuilt; unknown built-in
/// keys are dropped. Mirrors Python `reconcile`.
pub fn reconcile(entries: &[Value], defaults: Vec<View>) -> Vec<View> {
    let mut out: Vec<View> = Vec::new();
    let mut seen: Vec<String> = Vec::new();
    for e in entries {
        let Some(key) = e.get("key").and_then(|x| x.as_str()) else {
            continue;
        };
        if key.is_empty() || seen.iter().any(|k| k == key) {
            continue;
        }
        seen.push(key.to_string());
        if let Some(base) = defaults.iter().find(|v| v.key == key) {
            // Built-in: canonical structure; name follows code unless renamed
            // (un-renamed built-ins store a null name); pin from the overlay.
            let mut v = base.clone();
            if let Some(n) = e.get("name").and_then(|x| x.as_str()) {
                v.name = n.to_string();
            }
            v.pinned = e
                .get("pinned")
                .and_then(|x| x.as_bool())
                .unwrap_or(base.pinned);
            out.push(v);
        } else if !e.get("builtin").and_then(|x| x.as_bool()).unwrap_or(false) {
            if let Some(v) = view_from_json(e) {
                out.push(v);
            }
        }
        // else: a built-in key we no longer ship -> drop it.
    }
    for v in defaults {
        if !seen.iter().any(|k| *k == v.key) {
            out.push(v);
        }
    }
    out
}

pub fn load_views(root: &Path) -> Vec<View> {
    let defaults = default_views();
    let Ok(text) = fs::read_to_string(views_path(root)) else {
        return defaults;
    };
    let Ok(data) = serde_json::from_str::<Value>(&text) else {
        return defaults;
    };
    match data.get("views").and_then(|x| x.as_array()) {
        Some(entries) => reconcile(entries, defaults),
        None => defaults,
    }
}

pub fn save_views(root: &Path, views: &[View]) {
    let defaults = default_views();
    let entries: Vec<Value> = views
        .iter()
        .map(|v| {
            let mut d = view_to_json(v);
            // Un-renamed built-in: store a null name so it tracks the code label.
            if v.builtin {
                if let Some(base) = defaults.iter().find(|b| b.key == v.key) {
                    if base.name == v.name {
                        d["name"] = Value::Null;
                    }
                }
            }
            d
        })
        .collect();
    let payload = json!({ "v": VERSION, "views": entries });
    let path = views_path(root);
    if let Some(dir) = path.parent() {
        let _ = fs::create_dir_all(dir);
    }
    let _ = fs::write(path, payload.to_string());
}

// -- pure list mutations (used by the picker) --------------------------------

/// Swap view `i` with its neighbor in `dir` (-1 up / +1 down); returns the new
/// index (unchanged at the ends). (Consumed by the slice 6b-ii view picker.)
#[allow(dead_code)]
pub fn move_view(views: &mut [View], i: usize, dir: i32) -> usize {
    let j = i as i32 + dir;
    if j >= 0 && (j as usize) < views.len() && i < views.len() {
        views.swap(i, j as usize);
        return j as usize;
    }
    i
}

/// A view may be unpinned unless it is the last pinned one (the tab bar must
/// keep at least one tab). (Consumed by the slice 6b-ii view picker.)
#[allow(dead_code)]
pub fn can_unpin(views: &[View], i: usize) -> bool {
    if !views[i].pinned {
        return true;
    }
    views.iter().filter(|v| v.pinned).count() > 1
}

// -- working set -------------------------------------------------------------

pub fn load_working_set(root: &Path) -> Vec<String> {
    let Ok(text) = fs::read_to_string(working_set_path(root)) else {
        return vec![];
    };
    let Ok(data) = serde_json::from_str::<Value>(&text) else {
        return vec![];
    };
    str_vec(data.get("ids"))
}

pub fn save_working_set(root: &Path, ids: &[String]) {
    let payload = json!({ "v": VERSION, "ids": ids });
    let path = working_set_path(root);
    if let Some(dir) = path.parent() {
        let _ = fs::create_dir_all(dir);
    }
    let _ = fs::write(path, payload.to_string());
}

/// Remove `tid` if present, else append it (new stars go to the bottom).
pub fn toggle_working_set(ids: &[String], tid: &str) -> Vec<String> {
    if ids.iter().any(|i| i == tid) {
        ids.iter().filter(|i| *i != tid).cloned().collect()
    } else {
        let mut v = ids.to_vec();
        v.push(tid.to_string());
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reconcile_appends_missing_builtins_and_keeps_custom() {
        // Overlay mentions only Hairy (unpinned, renamed) + a custom view.
        let entries = vec![
            json!({"key": "status:hairy", "name": "H!", "pinned": false}),
            json!({"key": "view:abcd1234", "name": "Mine", "builtin": false,
                   "spec": {"types": ["bug"]}, "pinned": true}),
        ];
        let out = reconcile(&entries, default_views());
        // Hairy first (overlaid), then the custom, then the remaining built-ins.
        assert_eq!(out[0].key, "status:hairy");
        assert_eq!(out[0].name, "H!");
        assert!(!out[0].pinned);
        assert_eq!(out[1].key, "view:abcd1234");
        assert_eq!(out[1].spec.types, vec!["bug".to_string()]);
        // Every built-in still present.
        for key in ["status:shaving", "status:shorn", "recent", "working-set"] {
            assert!(out.iter().any(|v| v.key == key), "missing {key}");
        }
    }

    #[test]
    fn toggle_working_set_adds_then_removes() {
        let ids = vec!["a".to_string()];
        let with_b = toggle_working_set(&ids, "b");
        assert_eq!(with_b, vec!["a".to_string(), "b".to_string()]);
        let without_a = toggle_working_set(&with_b, "a");
        assert_eq!(without_a, vec!["b".to_string()]);
    }

    #[test]
    fn can_unpin_keeps_last_tab() {
        let mut views = default_views(); // all pinned
        // Unpin all but one; the last pinned can't be unpinned.
        for v in views.iter_mut().skip(1) {
            v.pinned = false;
        }
        assert!(!can_unpin(&views, 0)); // sole pinned
        assert!(can_unpin(&views, 1)); // already unpinned
    }

    #[test]
    fn move_view_swaps_within_bounds() {
        let mut views = default_views();
        let (k0, k1) = (views[0].key.clone(), views[1].key.clone());
        assert_eq!(move_view(&mut views, 0, 1), 1);
        assert_eq!(views[0].key, k1);
        assert_eq!(views[1].key, k0);
        assert_eq!(move_view(&mut views, 0, -1), 0); // at the top, no move
    }
}

/// Stable key seed for a new custom view (name + a disambiguating suffix).
pub fn custom_key_seed(name: &str, existing: &[View]) -> String {
    let mut seed = format!("{name}-{}", existing.len());
    // Ensure uniqueness against existing custom keys.
    while existing
        .iter()
        .any(|v| v.key == format!("view:{}", short_hash(&seed)))
    {
        seed.push('x');
    }
    seed
}
