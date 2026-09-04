//! The core operations facade. `Herd` owns every yaks operation as a typed,
//! print-free method so the CLI, the TUI, and (later) a long-lived process
//! serving editor/IDE plugins all sit thinly on top of the same logic. Each
//! method performs a WHOLE operation (validation + fs mutation) and returns a
//! typed result; nothing here touches argv or stdout.
//!
//! Kept free of `clap` and rendering so it can extract to a `yaks-core` lib
//! crate unchanged.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::Utc;

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
    /// Actor to attribute an appended note to (stamped as `[actor]`). Only
    /// meaningful alongside `note`; ownership is never implied.
    pub actor: Option<String>,
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

/// The git commits linked to a yak, recovered from history rather than stored:
/// those that name the id in a commit message, and those that touched the yak's
/// own file as it moved across statuses.
pub struct Commits {
    pub id: String,
    pub path: PathBuf,
    pub by_message: Vec<String>,
    pub by_file: Vec<String>,
}

/// Outcome of a rename operation ([`Herd::rename_many`]).
pub enum RenameOutcome {
    /// The rename was applied, or (when `plan.applied` is false) planned.
    Done(RenamePlan),
    /// No requested old id matched anything (e.g. a prefix with no yaks).
    NothingToRename,
    /// A requested old id does not exist.
    NotFound(String),
    /// A target id already exists (and is not being vacated) or is requested twice.
    Collision(String),
    /// A target id is malformed or identical to its source.
    Invalid(String),
}

/// What a rename changed, or would change when `applied` is false.
pub struct RenamePlan {
    pub applied: bool,
    /// The subject renames, `(old, new)`, sorted.
    pub renames: Vec<(String, String)>,
    /// Every task file touched, sorted by current id.
    pub edits: Vec<RenameEdit>,
}

/// One task file touched by a rename.
pub struct RenameEdit {
    /// The file's current on-disk id.
    pub id: String,
    /// Set when this file is itself a rename subject (its id becomes this).
    pub new_id: Option<String>,
    /// Which reference surfaces changed: any of `parent`, `depends_on`, `title`, `body`.
    pub fields: Vec<&'static str>,
    /// 1-based body line numbers whose mentions were rewritten.
    pub body_lines: Vec<usize>,
}

/// One timestamped note tagged with the yak it belongs to (the `log` view).
pub struct LogEntry {
    pub id: String,
    pub title: String,
    pub ts: String,
    pub actor: Option<String>,
    pub note: String,
}

/// One integrity problem found by [`Herd::doctor`]. Read-only: doctor never
/// mutates the herd, it only reports.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Issue {
    pub kind: IssueKind,
    pub message: String,
    /// The yak id(s) the issue concerns: the subject first, then any id it
    /// points at (e.g. the missing parent for a dangling reference).
    pub ids: Vec<String>,
}

/// The classes of problem `doctor` looks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IssueKind {
    /// One id living in more than one status dir at once — the add/add merge
    /// hazard where a yak was moved on two branches.
    DuplicateStatus,
    /// The same id parsed from two files in a single status dir.
    DuplicateId,
    /// A task whose `parent` names an id no task has.
    DanglingParent,
    /// A task whose `depends_on` names an id no task has.
    DanglingDependsOn,
}

impl IssueKind {
    /// A short, stable machine label (used for `--json` and grouping).
    pub fn code(self) -> &'static str {
        match self {
            IssueKind::DuplicateStatus => "duplicate-status",
            IssueKind::DuplicateId => "duplicate-id",
            IssueKind::DanglingParent => "dangling-parent",
            IssueKind::DanglingDependsOn => "dangling-depends-on",
        }
    }

    /// A human heading for grouped CLI output.
    pub fn heading(self) -> &'static str {
        match self {
            IssueKind::DuplicateStatus => "Duplicate status (same id in multiple status dirs)",
            IssueKind::DuplicateId => "Duplicate id (same id twice in one status dir)",
            IssueKind::DanglingParent => "Dangling parent",
            IssueKind::DanglingDependsOn => "Dangling depends_on",
        }
    }
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

    /// Set (or clear) a yak's `needs` block, optionally appending an attributed
    /// note in the same write. Backs `ask` (set `needs=human` + question) and
    /// `answer` (clear `needs` + reply). Clearing an already-clear block, or
    /// setting an already-identical one, still records the note if given.
    /// Set (or clear) a yak's `needs` block, optionally appending an attributed
    /// note. Returns the yak's status (so callers can warn about blocking
    /// finished work), or `None` if the id is unknown.
    pub fn set_needs(
        &self,
        id: &str,
        needs: Option<String>,
        actor: Option<&str>,
        note: Option<&str>,
    ) -> Result<Option<Status>> {
        let Some(mut task) = store::load_task_by_id(&self.root, id)? else {
            return Ok(None);
        };
        let status = task.status;
        task.needs = needs;
        if let Some(n) = note {
            let ts = store::now_iso();
            task.body = store::append_note(&task.body, &ts, actor, n);
        }
        task.updated = Some(store::now_iso());
        store::write::save(&self.root, &task)?;
        Ok(Some(status))
    }

    /// Every yak carrying a `needs` block, regardless of status. The invariant is
    /// that a set block is never invisible: an `ask` on a shorn/dead yak must
    /// still surface here (that silent-block gap is exactly why this ignores
    /// status). Other filter flags (priority/label/search) still apply.
    pub fn inbox(&self, mut spec: FilterSpec) -> Result<Vec<Task>> {
        spec.statuses = vec![Status::Hairy, Status::Shaving, Status::Shorn, Status::Dead];
        let tasks = store::load(&self.root, &EVERY)?;
        Ok(filter::apply(&tasks, &spec, true)
            .into_iter()
            .filter(|t| t.needs.is_some())
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

    /// Timestamped notes across the filtered set, oldest first. `since`, when
    /// set, keeps only notes at or after that instant (see `store::parse_since`).
    /// Notes live in each task body; state transitions are not timestamped, so
    /// this is a note log, not a full audit trail.
    pub fn log(
        &self,
        spec: FilterSpec,
        since: Option<&str>,
        by: Option<&str>,
    ) -> Result<Vec<LogEntry>> {
        let cutoff = match since {
            Some(s) => Some(store::parse_since(s, Utc::now())?),
            None => None,
        };
        let include_dead = spec.statuses.contains(&Status::Dead);
        let tasks = store::load(&self.root, &EVERY)?;
        let mut out = Vec::new();
        for t in filter::apply(&tasks, &spec, include_dead) {
            for note in store::parse_notes(&t.body) {
                if let Some(cut) = cutoff {
                    if let Some(ts) = store::parse_ts(&note.ts) {
                        if ts < cut {
                            continue;
                        }
                    }
                }
                if let Some(who) = by {
                    if note.actor.as_deref() != Some(who) {
                        continue;
                    }
                }
                out.push(LogEntry {
                    id: t.id.clone(),
                    title: t.title.clone(),
                    ts: note.ts,
                    actor: note.actor,
                    note: note.text,
                });
            }
        }
        out.sort_by(|a, b| a.ts.cmp(&b.ts).then_with(|| a.id.cmp(&b.id)));
        Ok(out)
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

    /// A read-only herd-integrity pass: collect every problem worth a human's
    /// attention without touching a single file. Ordering is deterministic
    /// (by kind, then id) so text and `--json` output are stable and
    /// CI-diffable.
    ///
    /// v1 checks: an id living in two status dirs at once (the add/add merge
    /// hazard), the same id twice inside one status dir, and `parent` /
    /// `depends_on` references that point at no known yak.
    pub fn doctor(&self) -> Result<Vec<Issue>> {
        let tasks = store::load(&self.root, &EVERY)?;
        let known = store::all_ids(&self.root);
        let mut issues = Vec::new();

        // id -> the statuses it was loaded under (one entry per file on disk).
        // A well-formed herd has exactly one file, hence one status, per id.
        let mut by_id: std::collections::BTreeMap<&str, Vec<Status>> =
            std::collections::BTreeMap::new();
        for t in &tasks {
            by_id.entry(t.id.as_str()).or_default().push(t.status);
        }
        for (id, statuses) in &by_id {
            let mut distinct = statuses.clone();
            distinct.sort_by_key(|s| rank(*s));
            distinct.dedup();
            if distinct.len() > 1 {
                let dirs: Vec<&str> = distinct.iter().map(|s| s.dir()).collect();
                issues.push(Issue {
                    kind: IssueKind::DuplicateStatus,
                    message: format!(
                        "{id} is in {} status dirs at once: {}",
                        dirs.len(),
                        dirs.join(", ")
                    ),
                    ids: vec![(*id).to_string()],
                });
            } else if statuses.len() > 1 {
                issues.push(Issue {
                    kind: IssueKind::DuplicateId,
                    message: format!(
                        "{id} appears {} times in {}/",
                        statuses.len(),
                        distinct[0].dir()
                    ),
                    ids: vec![(*id).to_string()],
                });
            }
        }

        // Dangling references: a `parent` or `depends_on` naming no known id.
        for t in &tasks {
            if let Some(p) = &t.parent {
                if !known.contains(p) {
                    issues.push(Issue {
                        kind: IssueKind::DanglingParent,
                        message: format!("{} has parent {} but no such yak exists", t.id, p),
                        ids: vec![t.id.clone(), p.clone()],
                    });
                }
            }
            for d in &t.depends_on {
                if !known.contains(d) {
                    issues.push(Issue {
                        kind: IssueKind::DanglingDependsOn,
                        message: format!("{} depends_on {} but no such yak exists", t.id, d),
                        ids: vec![t.id.clone(), d.clone()],
                    });
                }
            }
        }

        // Deterministic order: group by kind, then by the ids involved. Dedup
        // drops any identical issue a duplicated-on-disk task could produce.
        issues.sort_by(|a, b| {
            issue_rank(a.kind)
                .cmp(&issue_rank(b.kind))
                .then_with(|| a.ids.first().cmp(&b.ids.first()))
                .then_with(|| a.ids.get(1).cmp(&b.ids.get(1)))
        });
        issues.dedup();
        Ok(issues)
    }

    /// Recover the git commits linked to `id` without any stored hash: commits
    /// whose message names the id, and commits that touched the yak's own file
    /// (followed across its status moves). `None` if the id is not a task.
    pub fn commits(&self, id: &str) -> Result<Option<Commits>> {
        let Some((_, path)) = store::find_task_file(&self.root, id) else {
            return Ok(None);
        };
        let by_message = git_log(&self.root, &["--oneline", &format!("--grep={id}")])?;
        let by_file = git_log(
            &self.root,
            &["--oneline", "--follow", "--", &path.to_string_lossy()],
        )?;
        Ok(Some(Commits {
            id: id.to_string(),
            path,
            by_message,
            by_file,
        }))
    }

    /// Rename one yak, a convenience over [`Herd::rename_many`].
    pub fn rename(&self, old: &str, new: &str, dry_run: bool) -> Result<RenameOutcome> {
        self.rename_many(&[(old.to_string(), new.to_string())], dry_run)
    }

    /// Migrate every yak whose id is `{old}-<tail>` to `{new}-<tail>`, rewriting
    /// all references (via [`Herd::rename_many`]) and, on a real successful run,
    /// flipping the herd's configured `prefix` to `new` so future ids match.
    /// The `{old}-` boundary means `rename_prefix("yaks", ..)` never touches a
    /// `yaksrs-` id.
    pub fn rename_prefix(&self, old: &str, new: &str, dry_run: bool) -> Result<RenameOutcome> {
        let all = store::load(&self.root, &EVERY)?;
        let marker = format!("{old}-");
        let mut pairs = Vec::new();
        for t in &all {
            if let Some(tail) = t.id.strip_prefix(&marker) {
                pairs.push((t.id.clone(), format!("{new}-{tail}")));
            }
        }
        let outcome = self.rename_many(&pairs, dry_run)?;
        if !dry_run {
            if let RenameOutcome::Done(_) = &outcome {
                store::set_config_prefix(&self.root, new)?;
            }
        }
        Ok(outcome)
    }

    /// Rename one or more yaks in a single pass, rewriting every reference to
    /// them across the whole herd: the file name + `id` of each subject, and the
    /// `parent`, `depends_on`, title, and body mentions of every task that
    /// points at one. References are matched as whole tokens validated against
    /// real ids (via `refs`), so lookalike prose is never touched. With
    /// `dry_run`, nothing is written and the returned plan describes what would
    /// change. This is the shared engine behind `yaks rename` (one pair) and the
    /// bulk prefix migration (many pairs).
    pub fn rename_many(&self, pairs: &[(String, String)], dry_run: bool) -> Result<RenameOutcome> {
        let all = store::load(&self.root, &EVERY)?;
        let existing: std::collections::HashSet<&str> = all.iter().map(|t| t.id.as_str()).collect();

        // Validate the requested pairs and build the old -> new map.
        let mut map: std::collections::HashMap<String, String> = std::collections::HashMap::new();
        let mut targets: std::collections::HashSet<String> = std::collections::HashSet::new();
        for (old, new) in pairs {
            if !existing.contains(old.as_str()) {
                return Ok(RenameOutcome::NotFound(old.clone()));
            }
            if new == old || !refs::has_ref_shape(new) {
                return Ok(RenameOutcome::Invalid(new.clone()));
            }
            if !targets.insert(new.clone()) || map.insert(old.clone(), new.clone()).is_some() {
                return Ok(RenameOutcome::Collision(new.clone()));
            }
        }
        if map.is_empty() {
            return Ok(RenameOutcome::NothingToRename);
        }
        // A target that already exists and isn't itself being vacated would
        // clobber a live yak.
        for new in &targets {
            if existing.contains(new.as_str()) && !map.contains_key(new) {
                return Ok(RenameOutcome::Collision(new.clone()));
            }
        }

        let lookup = |t: &str| map.get(t).cloned();
        let mut edits = Vec::new();
        let mut writes = Vec::new();
        for task in &all {
            let mut edit = RenameEdit {
                id: task.id.clone(),
                new_id: map.get(&task.id).cloned(),
                fields: Vec::new(),
                body_lines: Vec::new(),
            };
            let mut next = task.clone();
            if let Some(nid) = &edit.new_id {
                next.id = nid.clone();
            }
            if let Some(p) = &task.parent {
                if let Some(np) = map.get(p) {
                    next.parent = Some(np.clone());
                    edit.fields.push("parent");
                }
            }
            if task.depends_on.iter().any(|d| map.contains_key(d)) {
                next.depends_on = task
                    .depends_on
                    .iter()
                    .map(|d| map.get(d).cloned().unwrap_or_else(|| d.clone()))
                    .collect();
                edit.fields.push("depends_on");
            }
            let (title, tchanged) = refs::rewrite(&task.title, &lookup);
            if tchanged {
                next.title = title;
                edit.fields.push("title");
            }
            let mut body_changed = false;
            let mut lines: Vec<String> = Vec::new();
            for (n, line) in task.body.lines().enumerate() {
                let (nl, ch) = refs::rewrite(line, &lookup);
                if ch {
                    body_changed = true;
                    edit.body_lines.push(n + 1);
                }
                lines.push(nl);
            }
            if body_changed {
                next.body = lines.join("\n");
                edit.fields.push("body");
            }
            if edit.new_id.is_some() || !edit.fields.is_empty() {
                writes.push((task.status, task.id.clone(), next));
                edits.push(edit);
            }
        }

        if !dry_run {
            for (_st, _old, next) in &writes {
                store::write::save(&self.root, next)?;
            }
            // Remove each vacated subject file, unless its id is reused as a
            // rename target (a swap/chain, which stays put under its new writer).
            for (st, old, _next) in &writes {
                if map.contains_key(old) && !targets.contains(old) {
                    let p = self.root.join(st.dir()).join(format!("{old}.md"));
                    let _ = std::fs::remove_file(p);
                }
            }
        }

        edits.sort_by(|a, b| a.id.cmp(&b.id));
        let mut renames: Vec<(String, String)> =
            map.iter().map(|(o, n)| (o.clone(), n.clone())).collect();
        renames.sort();
        Ok(RenameOutcome::Done(RenamePlan {
            applied: !dry_run,
            renames,
            edits,
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
            needs: None,
            extra: Vec::new(),
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
            task.body = store::append_note(&task.body, &ts, edit.actor.as_deref(), &n);
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

fn git_log(root: &Path, args: &[&str]) -> Result<Vec<String>> {
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(root)
        .arg("log")
        .args(args)
        .output()
        .context("running git log")?;
    if !out.status.success() {
        anyhow::bail!(
            "git log failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(str::to_string)
        .collect())
}

fn rank(s: Status) -> u8 {
    match s {
        Status::Hairy => 0,
        Status::Shaving => 1,
        Status::Shorn => 2,
        Status::Dead => 3,
    }
}

fn issue_rank(k: IssueKind) -> u8 {
    match k {
        IssueKind::DuplicateStatus => 0,
        IssueKind::DuplicateId => 1,
        IssueKind::DanglingParent => 2,
        IssueKind::DanglingDependsOn => 3,
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
            needs: None,
            extra: Vec::new(),
            body: String::new(),
        }
    }

    /// A `needs` block is never invisible: `inbox` surfaces a blocked yak
    /// whatever its status (the shorn-yak silent-block gap), while unblocked
    /// yaks stay out.
    #[test]
    fn inbox_shows_blocked_yaks_regardless_of_status() {
        let (root, herd) = temp_herd();
        let mut blocked_hairy = task("yak-0001", Status::Hairy);
        blocked_hairy.needs = Some("human".into());
        let mut blocked_shorn = task("yak-0002", Status::Shorn);
        blocked_shorn.needs = Some("human".into());
        store::write::save(&root, &blocked_hairy).unwrap();
        store::write::save(&root, &blocked_shorn).unwrap();
        store::write::save(&root, &task("yak-0003", Status::Hairy)).unwrap(); // unblocked

        let mut got: Vec<String> = herd
            .inbox(FilterSpec::default())
            .unwrap()
            .into_iter()
            .map(|t| t.id)
            .collect();
        got.sort();
        assert_eq!(got, vec!["yak-0001", "yak-0002"]); // shorn block included, unblocked excluded
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

    fn done(out: RenameOutcome) -> RenamePlan {
        match out {
            RenameOutcome::Done(p) => p,
            _ => panic!("expected RenameOutcome::Done"),
        }
    }

    #[test]
    fn rename_rewrites_every_reference_surface() {
        let (root, herd) = temp_herd();
        store::write::save(&root, &task("yaksrs-0001", Status::Hairy)).unwrap();
        let mut child = task("yaksrs-0002", Status::Hairy);
        child.parent = Some("yaksrs-0001".into());
        child.depends_on = vec!["yaksrs-0001".into()];
        child.body = "blocked by yaksrs-0001, see [[yaksrs-0001]] and yaksrs-0009".into();
        store::write::save(&root, &child).unwrap();

        let plan = done(herd.rename("yaksrs-0001", "yaks-0001", false).unwrap());
        assert!(plan.applied);
        assert_eq!(
            plan.renames,
            vec![("yaksrs-0001".to_string(), "yaks-0001".to_string())]
        );

        // Subject file moved.
        assert!(root.join("hairy/yaks-0001.md").is_file());
        assert!(!root.join("hairy/yaksrs-0001.md").exists());

        // Every referring surface updated; the non-id lookalike is left alone.
        let c = store::load_task_by_id(&root, "yaksrs-0002")
            .unwrap()
            .unwrap();
        assert_eq!(c.parent.as_deref(), Some("yaks-0001"));
        assert_eq!(c.depends_on, vec!["yaks-0001".to_string()]);
        assert!(c.body.contains("blocked by yaks-0001"));
        assert!(c.body.contains("[[yaks-0001]]"));
        assert!(
            c.body.contains("yaksrs-0009"),
            "a token that is not a real id must not be rewritten"
        );
    }

    #[test]
    fn rename_rejects_collision_with_existing_id() {
        let (root, herd) = temp_herd();
        store::write::save(&root, &task("yaksrs-0001", Status::Hairy)).unwrap();
        store::write::save(&root, &task("yaks-0001", Status::Hairy)).unwrap();
        match herd.rename("yaksrs-0001", "yaks-0001", true).unwrap() {
            RenameOutcome::Collision(id) => assert_eq!(id, "yaks-0001"),
            _ => panic!("expected collision"),
        }
    }

    #[test]
    fn rename_dry_run_leaves_files_untouched() {
        let (root, herd) = temp_herd();
        store::write::save(&root, &task("yaksrs-0001", Status::Hairy)).unwrap();
        let plan = done(herd.rename("yaksrs-0001", "yaks-0001", true).unwrap());
        assert!(!plan.applied);
        assert!(root.join("hairy/yaksrs-0001.md").is_file());
        assert!(!root.join("hairy/yaks-0001.md").exists());
    }

    #[test]
    fn rename_many_migrates_a_prefix_batch_across_statuses() {
        let (root, herd) = temp_herd();
        store::write::save(&root, &task("yaksrs-0001", Status::Hairy)).unwrap();
        let mut b = task("yaksrs-0002", Status::Shorn);
        b.depends_on = vec!["yaksrs-0001".into()];
        store::write::save(&root, &b).unwrap();

        let pairs = vec![
            ("yaksrs-0001".to_string(), "yaks-0001".to_string()),
            ("yaksrs-0002".to_string(), "yaks-0002".to_string()),
        ];
        let plan = done(herd.rename_many(&pairs, false).unwrap());
        assert_eq!(plan.renames.len(), 2);
        assert!(root.join("hairy/yaks-0001.md").is_file());
        assert!(root.join("shorn/yaks-0002.md").is_file());
        assert!(!root.join("hairy/yaksrs-0001.md").exists());
        assert!(!root.join("shorn/yaksrs-0002.md").exists());
        let b2 = store::load_task_by_id(&root, "yaks-0002").unwrap().unwrap();
        assert_eq!(b2.depends_on, vec!["yaks-0001".to_string()]);
    }

    #[test]
    fn rename_prefix_migrates_matching_ids_and_flips_config() {
        let (root, herd) = temp_herd();
        store::write::save(&root, &task("yaksrs-0001", Status::Hairy)).unwrap();
        let mut b = task("yaksrs-0002", Status::Hairy);
        b.depends_on = vec!["yaksrs-0001".into()];
        store::write::save(&root, &b).unwrap();
        // A different-prefix id must be left alone (the boundary is `yaksrs-`).
        store::write::save(&root, &task("yaks-abcd", Status::Hairy)).unwrap();

        let plan = done(herd.rename_prefix("yaksrs", "yaks", false).unwrap());
        let news: Vec<&str> = plan.renames.iter().map(|(_, n)| n.as_str()).collect();
        assert_eq!(news, vec!["yaks-0001", "yaks-0002"]);
        assert!(root.join("hairy/yaks-0001.md").is_file());
        assert!(root.join("hairy/yaks-abcd.md").is_file()); // untouched
        assert!(!root.join("hairy/yaksrs-0001.md").exists());
        let b2 = store::load_task_by_id(&root, "yaks-0002").unwrap().unwrap();
        assert_eq!(b2.depends_on, vec!["yaks-0001".to_string()]);
        assert_eq!(store::read_config(&root).prefix, "yaks");
    }

    #[test]
    fn rename_prefix_dry_run_does_not_flip_config_or_move_files() {
        let (root, herd) = temp_herd();
        store::write::save(&root, &task("yaksrs-0001", Status::Hairy)).unwrap();
        let plan = done(herd.rename_prefix("yaksrs", "yaks", true).unwrap());
        assert!(!plan.applied);
        assert!(!root.join("config.yaml").exists());
        assert!(root.join("hairy/yaksrs-0001.md").is_file());
    }

    /// The multi-id transition path (`yaks shorn a b c`) drives the CLI batch by
    /// calling [`Herd::transition`] once per id. A clean batch moves every id;
    /// a partial-failure batch (one good id + one nonexistent) still moves the
    /// good id and flags a failure, which the CLI turns into a non-zero exit.
    #[test]
    fn transition_batch_moves_valid_ids_and_flags_missing() {
        let (root, herd) = temp_herd();
        store::write::save(&root, &task("yak-0001", Status::Hairy)).unwrap();
        store::write::save(&root, &task("yak-0002", Status::Hairy)).unwrap();

        // All-good batch: both ids move to shorn, nothing flagged.
        let mut any_failed = false;
        for id in ["yak-0001", "yak-0002"] {
            if herd.transition(id, Status::Shorn).unwrap() != MoveOutcome::Moved {
                any_failed = true;
            }
        }
        assert!(!any_failed, "an all-valid batch must not flag failure");
        assert!(root.join("shorn/yak-0001.md").is_file());
        assert!(root.join("shorn/yak-0002.md").is_file());

        // Partial failure: good id moves, missing id reports NotFound. The batch
        // must not abort on the first error, so the good id still moves.
        store::write::save(&root, &task("yak-0003", Status::Hairy)).unwrap();
        let mut outcomes = Vec::new();
        let mut any_failed = false;
        for id in ["yak-0003", "yak-nope"] {
            let outcome = herd.transition(id, Status::Shorn).unwrap();
            if outcome != MoveOutcome::Moved {
                any_failed = true;
            }
            outcomes.push(outcome);
        }
        assert_eq!(outcomes, vec![MoveOutcome::Moved, MoveOutcome::NotFound]);
        assert!(
            any_failed,
            "a missing id must flag the batch (non-zero exit)"
        );
        assert!(
            root.join("shorn/yak-0003.md").is_file(),
            "the valid id must move despite a sibling failure"
        );
    }

    /// `doctor` flags a task whose formal `parent` and `depends_on` name ids no
    /// yak has, and stays silent about references that do resolve.
    #[test]
    fn doctor_flags_dangling_parent_and_depends_on() {
        let (root, herd) = temp_herd();
        store::write::save(&root, &task("yak-0002", Status::Shorn)).unwrap();
        let mut subject = task("yak-0001", Status::Hairy);
        subject.parent = Some("yak-0404".into()); // no such yak
        subject.depends_on = vec!["yak-0002".into(), "yak-0405".into()]; // one dangles
        store::write::save(&root, &subject).unwrap();

        let issues = herd.doctor().unwrap();
        let got: Vec<(IssueKind, Vec<&str>)> = issues
            .iter()
            .map(|i| (i.kind, i.ids.iter().map(String::as_str).collect()))
            .collect();
        assert_eq!(
            got,
            vec![
                (IssueKind::DanglingParent, vec!["yak-0001", "yak-0404"]),
                (IssueKind::DanglingDependsOn, vec!["yak-0001", "yak-0405"]),
            ],
            "resolvable refs (yak-0002) must not be flagged"
        );
    }

    /// The headline check: the same id written into two status dirs (the add/add
    /// merge hazard) is flagged once as a duplicate-status clash.
    #[test]
    fn doctor_flags_same_id_in_two_status_dirs() {
        let (root, herd) = temp_herd();
        store::write::save(&root, &task("yak-0001", Status::Hairy)).unwrap();
        store::write::save(&root, &task("yak-0001", Status::Shorn)).unwrap();

        let issues = herd.doctor().unwrap();
        assert_eq!(issues.len(), 1, "one duplicated id -> one clash");
        assert_eq!(issues[0].kind, IssueKind::DuplicateStatus);
        assert_eq!(issues[0].ids, vec!["yak-0001"]);
        assert!(
            issues[0].message.contains("hairy") && issues[0].message.contains("shorn"),
            "message names both dirs: {}",
            issues[0].message
        );
    }

    /// A well-formed herd (resolvable parent + dep, one status per id) is clean.
    #[test]
    fn doctor_clean_herd_has_no_issues() {
        let (root, herd) = temp_herd();
        let mut a = task("yak-0001", Status::Hairy);
        a.parent = Some("yak-0002".into());
        a.depends_on = vec!["yak-0002".into()];
        store::write::save(&root, &a).unwrap();
        store::write::save(&root, &task("yak-0002", Status::Shorn)).unwrap();
        assert!(herd.doctor().unwrap().is_empty());
    }
}
