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
                "herd schema v{v} predates this yaks (v{}); reading best-effort. Run the Python yaks once to migrate.",
                store::SCHEMA
            )),
            SchemaStatus::Compatible => None,
        };
        Ok(Herd {
            root,
            schema_warning,
        })
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
