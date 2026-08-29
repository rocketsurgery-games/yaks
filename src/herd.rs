//! The core operations facade. `Herd` owns every yaks operation as a typed,
//! print-free method so the CLI, the TUI, and (later) a long-lived process
//! serving editor/IDE plugins all sit thinly on top of the same logic. Each
//! method performs a WHOLE operation (validation + fs mutation) and returns a
//! typed result; nothing here touches argv or stdout.
//!
//! Kept free of `clap` and rendering so it can extract to a `yaks-core` lib
//! crate unchanged.

use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::filter::{self, FilterSpec};
use crate::model::{Status, Task};
use crate::refs;
use crate::rollup;
use crate::store::{self, SchemaStatus};

pub use crate::store::{DepOutcome, MoveOutcome, Reparent};

const NON_DEAD: [Status; 3] = [Status::Hairy, Status::Shaving, Status::Shorn];
const EVERY: [Status; 4] = [Status::Hairy, Status::Shaving, Status::Shorn, Status::Dead];

/// Why opening a herd failed.
pub enum OpenError {
    NoHerd(String),
    SchemaTooNew { found: u32, supported: u32 },
}

/// A handle to one `.yaks/` herd. Cheap to construct; holds no cache yet
/// (leaves room for a future stat-validated index without changing callers).
pub struct Herd {
    root: PathBuf,
    /// Set when the herd's schema predates this build (best-effort read).
    pub schema_warning: Option<String>,
}

/// Fields for a new task (defaults resolved from config inside `create`).
pub struct NewTask {
    pub title: String,
    pub kind: Option<String>,
    pub priority: Option<u8>,
    pub parent: Option<String>,
    pub labels: Vec<String>,
    pub depends_on: Vec<String>,
    pub source: Option<String>,
    pub description: Option<String>,
}

/// A set of edits to apply to a task in one operation.
#[derive(Default)]
pub struct TaskEdit {
    pub title: Option<String>,
    pub kind: Option<String>,
    pub priority: Option<u8>,
    pub description: Option<String>,
    pub add_labels: Vec<String>,
    pub remove_labels: Vec<String>,
    pub source: Option<String>,
    pub note: Option<String>,
}

pub enum CreateOutcome {
    Created(Box<Task>),
    ParentNotFound(String),
}

pub enum UpdateOutcome {
    Updated,
    NoChanges,
    NotFound,
}

/// Result of attaching an artifact to a task.
pub enum AttachOutcome {
    Attached(String),
    NotFound,
}

pub struct Stats {
    pub total: usize,
    pub hairy: usize,
    pub shaving: usize,
    pub shorn: usize,
    pub by_type: Vec<(String, usize)>,
    pub by_priority: Vec<(u8, usize)>,
}

/// A task plus its immediate children, for `show`.
pub struct Show {
    pub task: Task,
    pub children: Vec<Task>,
}

/// What kind of pointer an outgoing reference is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefKind {
    Parent,
    Depends,
    Mention,
}

/// One outgoing reference from a task to another yak.
pub struct RefEntry {
    pub kind: RefKind,
    pub id: String,
    /// False only for a formal parent/dependency pointing at an id that no
    /// longer exists (a dangling reference). Informal mentions are
    /// validation-gated by the resolver, so they are always resolved.
    pub resolved: bool,
    /// 1-based body line for a mention; `None` for formal refs and title mentions.
    pub line: Option<usize>,
}

/// Every outgoing reference a task carries, formal and informal — the
/// integrity-inspection view over the shared resolver.
pub struct TaskRefs {
    pub id: String,
    pub title: String,
    pub entries: Vec<RefEntry>,
}

impl Herd {
    /// Discover the nearest `.yaks/` above `cwd` and apply the schema gate.
    pub fn open(cwd: &Path) -> std::result::Result<Herd, OpenError> {
        let root = store::discover_root(cwd).map_err(|e| OpenError::NoHerd(e.to_string()))?;
        let schema_warning = match store::schema_status(&root) {
            SchemaStatus::Newer(found) => {
                return Err(OpenError::SchemaTooNew {
                    found,
                    supported: store::SCHEMA,
                });
            }
            SchemaStatus::Older(v) => Some(format!(
                "herd schema v{v} predates this yaks (v{}); reading best-effort.",
                store::SCHEMA
            )),
            SchemaStatus::Compatible => None,
        };
        Ok(Herd {
            root,
            schema_warning,
        })
    }

    /// The effective per-herd config (prefix, defaults, editor mode).
    pub fn config(&self) -> store::Config {
        store::read_config(&self.root)
    }

    /// The herd's `.yaks/` root, e.g. for locating the per-user UI-state cache.
    pub fn root(&self) -> &Path {
        &self.root
    }

    // -- queries ----------------------------------------------------------

    /// Tasks matching `spec`, grouped by status then id (the `list`/`search` view).
    pub fn list(&self, spec: FilterSpec, include_dead: bool) -> Result<Vec<Task>> {
        let tasks = store::load(&self.root, &EVERY)?;
        let mut rows: Vec<Task> = filter::apply(&tasks, &spec, include_dead)
            .into_iter()
            .cloned()
            .collect();
        rows.sort_by(|a, b| {
            rank(a.status)
                .cmp(&rank(b.status))
                .then_with(|| a.id.cmp(&b.id))
        });
        Ok(rows)
    }

    /// Hairy tasks with all dependencies resolved.
    pub fn next(&self, mut spec: FilterSpec) -> Result<Vec<Task>> {
        spec.statuses = vec![Status::Hairy];
        spec.ready_only = true;
        let tasks = store::load(&self.root, &EVERY)?;
        Ok(filter::apply(&tasks, &spec, false)
            .into_iter()
            .cloned()
            .collect())
    }

    /// Hairy tasks with at least one unresolved dependency, each paired with
    /// the list of ids it is waiting on.
    pub fn tangled(&self, mut spec: FilterSpec) -> Result<Vec<(Task, Vec<String>)>> {
        spec.statuses = vec![Status::Hairy];
        spec.tangled_only = true;
        let tasks = store::load(&self.root, &EVERY)?;
        let resolved = filter::resolved_ids(&tasks);
        Ok(filter::apply(&tasks, &spec, false)
            .into_iter()
            .map(|t| {
                let waiting = filter::unresolved_deps(t, &resolved)
                    .into_iter()
                    .map(str::to_string)
                    .collect();
                (t.clone(), waiting)
            })
            .collect())
    }

    pub fn stats(&self) -> Result<Stats> {
        let tasks = store::load(&self.root, &NON_DEAD)?;
        let count = |s: Status| tasks.iter().filter(|t| t.status == s).count();
        let mut by_type = fold_counts(tasks.iter().map(|t| t.kind.clone()));
        by_type.sort();
        let mut by_priority = fold_counts(tasks.iter().map(|t| t.priority));
        by_priority.sort();
        Ok(Stats {
            total: tasks.len(),
            hairy: count(Status::Hairy),
            shaving: count(Status::Shaving),
            shorn: count(Status::Shorn),
            by_type,
            by_priority,
        })
    }

    pub fn show(&self, id: &str) -> Result<Option<Show>> {
        let all = store::load(&self.root, &EVERY)?;
        let Some(task) = all.iter().find(|t| t.id == id).cloned() else {
            return Ok(None);
        };
        let mut children: Vec<Task> = all
            .iter()
            .filter(|c| c.parent.as_deref() == Some(id))
            .cloned()
            .collect();
        children.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(Some(Show { task, children }))
    }

    pub fn rollup(&self, spec: &FilterSpec) -> Result<(Vec<rollup::Group>, usize)> {
        let tasks = store::load(&self.root, &NON_DEAD)?;
        Ok(rollup::build(&tasks, spec))
    }

    /// List every yak this task points at — formal (parent, dependencies) and
    /// informal (id mentions in the title/body) — each flagged resolved or
    /// dangling. Both the TUI and this share the same resolver in `refs`, so
    /// what links here is exactly what lights up in the detail pane.
    pub fn refs(&self, id: &str) -> Result<Option<TaskRefs>> {
        let all = store::load(&self.root, &EVERY)?;
        let Some(task) = all.iter().find(|t| t.id == id).cloned() else {
            return Ok(None);
        };
        let ids: std::collections::HashSet<&str> = all.iter().map(|t| t.id.as_str()).collect();
        let known = refs::known_from(&ids);
        let mut entries = Vec::new();
        if let Some(p) = &task.parent {
            entries.push(RefEntry {
                kind: RefKind::Parent,
                id: p.clone(),
                resolved: known(p),
                line: None,
            });
        }
        for d in &task.depends_on {
            entries.push(RefEntry {
                kind: RefKind::Depends,
                id: d.clone(),
                resolved: known(d),
                line: None,
            });
        }
        for m in refs::scan(&refs::strip_wikilinks(&task.title), &known) {
            entries.push(RefEntry {
                kind: RefKind::Mention,
                id: m.id,
                resolved: true,
                line: None,
            });
        }
        for (n, raw) in task.body.lines().enumerate() {
            let line = refs::strip_wikilinks(raw);
            for m in refs::scan(&line, &known) {
                entries.push(RefEntry {
                    kind: RefKind::Mention,
                    id: m.id,
                    resolved: true,
                    line: Some(n + 1),
                });
            }
        }
        Ok(Some(TaskRefs {
            id: task.id,
            title: task.title,
            entries,
        }))
    }

    // -- mutations (each a whole operation) -------------------------------

    pub fn create(&self, new: NewTask) -> Result<CreateOutcome> {
        let cfg = store::read_config(&self.root);
        if let Some(p) = &new.parent {
            if !store::all_ids(&self.root).contains(p) {
                return Ok(CreateOutcome::ParentNotFound(p.clone()));
            }
        }
        let id = store::generate_id(&self.root, &cfg.prefix)?;
        let now = store::now_iso();
        let task = Task {
            id,
            title: new.title,
            kind: new.kind.unwrap_or(cfg.default_type),
            priority: new.priority.unwrap_or(cfg.default_priority),
            status: Status::Hairy,
            created: Some(now.clone()),
            updated: Some(now),
            parent: new.parent,
            labels: new.labels,
            depends_on: new.depends_on,
            source: new.source,
            body: new.description.unwrap_or_default(),
        };
        store::write::save(&self.root, &task)?;
        Ok(CreateOutcome::Created(Box::new(task)))
    }

    pub fn update(&self, id: &str, edit: TaskEdit) -> Result<UpdateOutcome> {
        let Some(mut task) = store::load_task_by_id(&self.root, id)? else {
            return Ok(UpdateOutcome::NotFound);
        };
        let mut changed = false;
        if let Some(t) = edit.title {
            task.title = t;
            changed = true;
        }
        if let Some(k) = edit.kind {
            task.kind = k;
            changed = true;
        }
        if let Some(p) = edit.priority {
            task.priority = p;
            changed = true;
        }
        if let Some(d) = edit.description {
            task.body = d;
            changed = true;
        }
        if !edit.add_labels.is_empty() {
            for l in edit.add_labels {
                if !task.labels.contains(&l) {
                    task.labels.push(l);
                }
            }
            changed = true;
        }
        if !edit.remove_labels.is_empty() {
            task.labels.retain(|l| !edit.remove_labels.contains(l));
            changed = true;
        }
        if let Some(s) = edit.source {
            if !s.is_empty() {
                task.source = Some(s);
                changed = true;
            }
        }
        if let Some(n) = edit.note {
            let ts = store::now_iso();
            task.body = store::append_note(&task.body, &ts, &n);
            changed = true;
        }
        if changed {
            task.updated = Some(store::now_iso());
            store::write::save(&self.root, &task)?;
            Ok(UpdateOutcome::Updated)
        } else {
            Ok(UpdateOutcome::NoChanges)
        }
    }

    /// Write `data` to `.yaks/artifacts/{id}/{name}` and append a markdown image
    /// link to the task body. `name` should be a bare filename. The artifacts
    /// tree lives inside the herd (committed alongside `.yaks/`, like Python).
    pub fn attach(&self, id: &str, name: &str, data: &[u8]) -> Result<AttachOutcome> {
        let Some(mut task) = store::load_task_by_id(&self.root, id)? else {
            return Ok(AttachOutcome::NotFound);
        };
        let dir = self.root.join("artifacts").join(id);
        std::fs::create_dir_all(&dir)?;
        std::fs::write(dir.join(name), data)?;
        let stem = Path::new(name)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(name);
        let link = format!("![{stem}](artifacts/{id}/{name})");
        let mut body = task.body.trim_end().to_string();
        if !body.is_empty() {
            body.push_str("\n\n");
        }
        body.push_str(&link);
        task.body = body;
        task.updated = Some(store::now_iso());
        store::write::save(&self.root, &task)?;
        Ok(AttachOutcome::Attached(name.to_string()))
    }

    pub fn transition(&self, id: &str, dest: Status) -> Result<MoveOutcome> {
        store::move_task(&self.root, id, dest)
    }

    pub fn dep_add(&self, id: &str, dep: &str) -> Result<DepOutcome> {
        store::add_dep(&self.root, id, dep)
    }

    pub fn dep_remove(&self, id: &str, dep: &str) -> Result<DepOutcome> {
        store::remove_dep(&self.root, id, dep)
    }

    pub fn reparent(&self, id: &str, new_parent: Option<String>) -> Result<Reparent> {
        store::reparent(&self.root, id, new_parent)
    }
}

fn rank(s: Status) -> u8 {
    match s {
        Status::Hairy => 0,
        Status::Shaving => 1,
        Status::Shorn => 2,
        Status::Dead => 3,
    }
}

fn fold_counts<K: std::hash::Hash + Eq, I: Iterator<Item = K>>(it: I) -> Vec<(K, usize)> {
    let mut m: std::collections::HashMap<K, usize> = std::collections::HashMap::new();
    for k in it {
        *m.entry(k).or_insert(0) += 1;
    }
    m.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    static SEQ: AtomicU64 = AtomicU64::new(0);

    /// A temp herd (a `.yaks/` under a fresh parent dir) plus an open handle.
    fn temp_herd() -> (PathBuf, Herd) {
        let mut parent = std::env::temp_dir();
        let n = SEQ.fetch_add(1, Ordering::Relaxed);
        parent.push(format!("yaksrs-herd-refs-{}-{}", std::process::id(), n));
        let root = parent.join(".yaks");
        for st in [Status::Hairy, Status::Shaving, Status::Shorn, Status::Dead] {
            std::fs::create_dir_all(root.join(st.dir())).unwrap();
        }
        let herd = match Herd::open(&parent) {
            Ok(h) => h,
            Err(_) => panic!("failed to open temp herd at {parent:?}"),
        };
        (root, herd)
    }

    fn task(id: &str, status: Status) -> Task {
        Task {
            id: id.into(),
            title: format!("title {id}"),
            kind: "task".into(),
            priority: 3,
            status,
            created: Some("2026-01-01T00:00:00Z".into()),
            updated: Some("2026-01-01T00:00:00Z".into()),
            parent: None,
            labels: vec![],
            depends_on: vec![],
            source: None,
            body: String::new(),
        }
    }

    /// `refs` reports formal parent/deps (flagging danglers) and validated body
    /// mentions, and never invents a mention for an id-shaped token that is not
    /// a real yak.
    #[test]
    fn refs_flags_danglers_and_validated_mentions() {
        let (root, herd) = temp_herd();
        store::write::save(&root, &task("yak-0002", Status::Shorn)).unwrap();
        store::write::save(&root, &task("yak-0003", Status::Hairy)).unwrap();
        let mut subject = task("yak-0001", Status::Hairy);
        subject.title = "subject without an id token".into();
        subject.parent = Some("yak-0007".into()); // missing -> dangling
        subject.depends_on = vec!["yak-0002".into(), "yak-9999".into()]; // one dangling
        subject.body = "see yak-0002 and [[yak-0003]] but not yak-9999".into();
        store::write::save(&root, &subject).unwrap();

        let r = herd.refs("yak-0001").unwrap().expect("subject exists");
        let got: Vec<(RefKind, &str, bool, Option<usize>)> = r
            .entries
            .iter()
            .map(|e| (e.kind, e.id.as_str(), e.resolved, e.line))
            .collect();
        assert_eq!(
            got,
            vec![
                (RefKind::Parent, "yak-0007", false, None),
                (RefKind::Depends, "yak-0002", true, None),
                (RefKind::Depends, "yak-9999", false, None),
                (RefKind::Mention, "yak-0002", true, Some(1)),
                (RefKind::Mention, "yak-0003", true, Some(1)),
            ],
            "yak-9999 in the body must not become a mention (validation-gated)"
        );
    }

    #[test]
    fn refs_none_for_missing_task() {
        let (_root, herd) = temp_herd();
        assert!(herd.refs("yak-dead").unwrap().is_none());
    }
}
