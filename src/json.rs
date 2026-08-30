//! JSON rendering for `--json`. Output shapes are stable, pinned by golden
//! snapshot
//! tests. serde_json's `preserve_order` keeps key order deterministic.
//!
//! Pure rendering: takes typed results from `herd` and returns `Value`s.

use serde_json::{Map, Value, json};

use crate::herd::Stats;
use crate::model::{Status, Task};
use crate::rollup::Group;

fn status_str(s: Status) -> &'static str {
    match s {
        Status::Hairy => "hairy",
        Status::Shaving => "shaving",
        Status::Shorn => "shorn",
        Status::Dead => "dead",
    }
}

/// Canonical JSON object for a task.
pub fn task_value(t: &Task) -> Value {
    let mut m = Map::new();
    m.insert("status".into(), json!(status_str(t.status)));
    m.insert("id".into(), json!(t.id));
    m.insert("title".into(), json!(t.title));
    m.insert("type".into(), json!(t.kind));
    m.insert("priority".into(), json!(t.priority));
    if let Some(c) = &t.created {
        m.insert("created".into(), json!(c));
    }
    if let Some(u) = &t.updated {
        m.insert("updated".into(), json!(u));
    }
    if let Some(p) = &t.parent {
        m.insert("parent".into(), json!(p));
    }
    if !t.depends_on.is_empty() {
        m.insert("depends_on".into(), json!(t.depends_on));
    }
    if !t.labels.is_empty() {
        m.insert("labels".into(), json!(t.labels));
    }
    if let Some(s) = &t.source {
        m.insert("source".into(), json!(s));
    }
    let body = t.body.trim();
    if !body.is_empty() {
        m.insert("description".into(), json!(body));
    }
    Value::Object(m)
}

pub fn tasks_array(tasks: &[Task]) -> Value {
    Value::Array(tasks.iter().map(task_value).collect())
}

pub fn log_array(entries: &[crate::herd::LogEntry]) -> Value {
    Value::Array(
        entries
            .iter()
            .map(|e| json!({"timestamp": e.ts, "id": e.id, "title": e.title, "note": e.note}))
            .collect(),
    )
}

pub fn tangled_array(items: &[(Task, Vec<String>)]) -> Value {
    Value::Array(
        items
            .iter()
            .map(|(t, un)| {
                let mut v = task_value(t);
                if let Value::Object(m) = &mut v {
                    m.insert("unresolved_deps".into(), json!(un));
                }
                v
            })
            .collect(),
    )
}

pub fn show_value(t: &Task, children: &[Task]) -> Value {
    let mut v = task_value(t);
    if let Value::Object(m) = &mut v {
        if !children.is_empty() {
            let kids: Vec<Value> = children
                .iter()
                .map(|c| json!({"id": c.id, "status": status_str(c.status), "title": c.title}))
                .collect();
            m.insert("children".into(), Value::Array(kids));
        }
    }
    v
}

pub fn stats_value(s: &Stats) -> Value {
    let mut bt = Map::new();
    for (k, v) in &s.by_type {
        bt.insert(k.clone(), json!(v));
    }
    let mut bp = Map::new();
    for (k, v) in &s.by_priority {
        bp.insert(k.to_string(), json!(v));
    }
    json!({
        "total": s.total,
        "hairy": s.hairy,
        "shaving": s.shaving,
        "shorn": s.shorn,
        "by_type": Value::Object(bt),
        "by_priority": Value::Object(bp),
    })
}

pub fn rollup_value(groups: &[Group]) -> Value {
    Value::Array(
        groups
            .iter()
            .map(|g| {
                let yaks: Vec<Value> = g
                    .yaks
                    .iter()
                    .map(|y| {
                        json!({
                            "id": y.task.id,
                            "status": status_str(y.task.status),
                            "title": y.task.title,
                            "inherited": y.inherited_from.is_some(),
                            "inherited_from": y.inherited_from,
                        })
                    })
                    .collect();
                json!({"source": g.source, "tracker": g.tracker, "key": g.key, "yaks": yaks})
            })
            .collect(),
    )
}

pub fn print(v: &Value) -> anyhow::Result<()> {
    println!("{}", serde_json::to_string_pretty(v)?);
    Ok(())
}
