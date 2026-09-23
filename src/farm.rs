//! The core operations facade. `Farm` owns every yaks operation as a typed,
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

/// Why opening a farm failed.
pub enum OpenError {
    NoFarm(String),
    SchemaTooNew { found: u32, supported: u32 },
}

/// A handle to one `.yaks/` farm. Cheap to construct; holds no cache yet
/// (leaves room for a future stat-validated index without changing callers).
pub struct Farm {
    root: PathBuf,
    /// Set when the farm's schema predates this build (best-effort read).
    pub schema_warning: Option<String>,
    /// This repo's default herd (id prefix), when discovery followed a `.yaks`
    /// pointer file carrying `prefix:`. Used by `create` below an explicit
    /// `--prefix` and above the farm's config default.
    pointer_prefix: Option<String>,
}

/// Fields for a new task (defaults resolved from config inside `create`).
pub struct NewTask {
    pub title: String,
    /// Which herd (id prefix) the new yak joins. `None` uses the farm's default
    /// prefix, so one farm can hold several herds.
    pub prefix: Option<String>,
    pub kind: Option<String>,
    pub priority: Option<u8>,
    pub parent: Option<String>,
    pub labels: Vec<String>,
    pub depends_on: Vec<String>,
    pub source: Option<String>,
    pub description: Option<String>,
    pub verify: Option<String>,
}

/// A set of edits to apply to a task in one operation.
#[derive(Default, Clone)]
pub struct TaskEdit {
    pub title: Option<String>,
    pub kind: Option<String>,
    pub priority: Option<u8>,
    pub description: Option<String>,
    pub add_labels: Vec<String>,
    pub remove_labels: Vec<String>,
    /// Set the external `source:` URL; an empty string clears it.
    pub source: Option<String>,
    /// A rerunnable verification command to set on the yak (see `Task::verify`).
    pub verify: Option<String>,
    pub note: Option<String>,
    /// Actor to attribute an appended note to (stamped as `[actor]`). Only
    /// meaningful alongside `note`; ownership is never implied.
    pub actor: Option<String>,
}

pub enum CreateOutcome {
    Created(Box<Task>),
    ParentNotFound(String),
    /// An explicit `--prefix` that would mint an id outside the reference
    /// grammar (prefix must be `[a-z0-9]+`).
    InvalidPrefix(String),
}

/// Outcome of merging another farm into this one ([`Farm::merge`]).
pub enum MergeOutcome {
    /// Merge is valid, and (unless `dry_run`) was applied.
    Done(MergePlan),
    /// No `.yaks/` farm was found at the given path (or it resolves to this
    /// same farm).
    NoSource(String),
    /// Ids present in BOTH farms; nothing was written. Reconcile the source's
    /// prefix with `rename-prefix`, then retry.
    Collision(Vec<String>),
}

/// The per-yak plan produced by [`Farm::merge`].
pub struct MergePlan {
    /// Whether the plan was applied (`false` for a dry run).
    pub applied: bool,
    /// The resolved source farm root the yaks came from.
    pub source: PathBuf,
    /// Each merged yak as `(id, status)`, sorted by id.
    pub yaks: Vec<(String, Status)>,
    /// Ids whose `artifacts/<id>/` directory was also copied.
    pub artifacts: Vec<String>,
    /// Incoming herds (id prefixes) newly declared in this farm's `herds:`
    /// config, sorted — so merged yaks' herds join the known-herd set the UI
    /// pickers offer (yaks-095a). For a dry run, the herds that *would* be
    /// declared.
    pub herds: Vec<String>,
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

/// Result of [`Farm::slaughter`] (yaks-05da).
#[derive(Debug, PartialEq, Eq)]
pub enum SlaughterOutcome {
    NotFound,
    /// Already dead, and no live descendants left to take with it.
    AlreadyDead,
    /// Refused: slaughtering would orphan these live (non-dead) descendants.
    /// Pass `family = true` to slaughter them along with the yak.
    HasLiveDescendants(Vec<String>),
    /// Moved to dead: the live descendants (deepest first), then the yak itself
    /// (omitted if it was already dead).
    Slaughtered(Vec<String>),
}

/// Every live (non-dead) descendant of `id` in `tasks`, found transitively via
/// the `parent:` field and ordered deepest-first (children before their
/// parents), so slaughtering in order never leaves a live yak under a dead one
/// mid-sweep. Cycle-safe. Shared by the CLI and the TUI (yaks-05da).
pub fn live_descendants(tasks: &[Task], id: &str) -> Vec<String> {
    let mut seen: Vec<String> = vec![id.to_string()];
    let mut out: Vec<String> = Vec::new();
    let mut frontier: Vec<String> = vec![id.to_string()];
    while !frontier.is_empty() {
        let mut next = Vec::new();
        for p in &frontier {
            let mut kids: Vec<&Task> = tasks
                .iter()
                .filter(|t| t.parent.as_deref() == Some(p.as_str()))
                .filter(|t| !seen.contains(&t.id))
                .collect();
            kids.sort_by(|a, b| a.id.cmp(&b.id));
            for k in kids {
                seen.push(k.id.clone());
                next.push(k.id.clone());
                if k.status != Status::Dead {
                    out.push(k.id.clone());
                }
            }
        }
        frontier = next;
    }
    out.reverse();
    out
}

/// Result of renaming an attachment in `.yaks/artifacts/<id>/`.
#[derive(Debug, PartialEq)]
pub enum RenameAttachmentOutcome {
    /// Renamed to `name`; `rewritten` yak files had links to it updated.
    Renamed { name: String, rewritten: usize },
    /// No yak with that id.
    TaskNotFound,
    /// No such file under the yak's artifacts directory.
    NoSuchAttachment(String),
    /// The requested name sanitizes to nothing usable.
    Invalid(String),
    /// A different attachment already has the target name.
    Collision(String),
    /// The sanitized name equals the current one.
    Unchanged,
}

/// Sanitize a user-supplied attachment filename: keep only the final path
/// component, turn anything but letters, digits, `.`, `_`, `-`, `+` into `-`
/// (so the name is safe on disk and inside a markdown `![](...)` link),
/// collapse `-` runs, and strip leading/trailing `.`/`-`. When the result has
/// no extension, `keep_ext_of`'s extension (if any) is appended, so renaming
/// `paste-….png` to `login` yields `login.png`. `None` when nothing usable is
/// left.
pub fn attachment_name(raw: &str, keep_ext_of: &str) -> Option<String> {
    let base = raw.trim().rsplit(['/', '\\']).next().unwrap_or("");
    let mut out = String::new();
    for c in base.chars() {
        let c = if c.is_alphanumeric() || matches!(c, '.' | '_' | '-' | '+') {
            c
        } else {
            '-'
        };
        if c == '-' && out.ends_with('-') {
            continue;
        }
        out.push(c);
    }
    let out = out.trim_matches(|c| c == '.' || c == '-').to_string();
    if out.is_empty() {
        return None;
    }
    if Path::new(&out).extension().is_none() {
        if let Some(ext) = Path::new(keep_ext_of).extension().and_then(|e| e.to_str()) {
            return Some(format!("{out}.{ext}"));
        }
    }
    Some(out)
}

/// Replace every whole-path occurrence of `old` in `text` with `new`. A match
/// must not be glued to a longer path/name on either side (so renaming
/// `a.png` never touches `a.png.bak` or `xartifacts/...`).
fn replace_path_token(text: &str, old: &str, new: &str) -> String {
    let name_char = |c: char| c.is_alphanumeric() || matches!(c, '.' | '_' | '-' | '+');
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(i) = rest.find(old) {
        let before = rest[..i].chars().last().or_else(|| out.chars().last());
        let after = rest[i + old.len()..].chars().next();
        out.push_str(&rest[..i]);
        let glued = before.is_some_and(|c| c.is_alphanumeric() || c == '_' || c == '-')
            || after.is_some_and(|c| name_char(c) && c != '.')
            || (after == Some('.')
                && rest[i + old.len() + 1..]
                    .chars()
                    .next()
                    .is_some_and(name_char));
        out.push_str(if glued { old } else { new });
        rest = &rest[i + old.len()..];
    }
    out.push_str(rest);
    out
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

/// Outcome of a rename operation ([`Farm::rename_many`]).
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

/// One integrity problem found by [`Farm::doctor`]. Read-only: doctor never
/// mutates the farm, it only reports.
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
    /// A shorn yak with no recorded note — a completion without evidence. Dead
    /// (abandoned) yaks are deliberately exempt: they need no completion
    /// evidence. Only reported in strict mode (the yaks-working
    /// evidence-before-shear rule).
    MissingEvidence,
    /// A shorn yak that carries a `verify:` command whose most recent recorded
    /// run was not a PASS (or was never run). Setting `verify:` is a commitment
    /// that strict mode then enforces at shear. Only reported in strict mode.
    UnverifiedShear,
}

impl IssueKind {
    /// A short, stable machine label (used for `--json` and grouping).
    pub fn code(self) -> &'static str {
        match self {
            IssueKind::DuplicateStatus => "duplicate-status",
            IssueKind::DuplicateId => "duplicate-id",
            IssueKind::DanglingParent => "dangling-parent",
            IssueKind::DanglingDependsOn => "dangling-depends-on",
            IssueKind::MissingEvidence => "missing-evidence",
            IssueKind::UnverifiedShear => "unverified-shear",
        }
    }

    /// A human heading for grouped CLI output.
    pub fn heading(self) -> &'static str {
        match self {
            IssueKind::DuplicateStatus => "Duplicate status (same id in multiple status dirs)",
            IssueKind::DuplicateId => "Duplicate id (same id twice in one status dir)",
            IssueKind::DanglingParent => "Dangling parent",
            IssueKind::DanglingDependsOn => "Dangling depends_on",
            IssueKind::MissingEvidence => "Missing evidence (shorn yak with no recorded note)",
            IssueKind::UnverifiedShear => {
                "Unverified shear (shorn yak whose verify: command did not last PASS)"
            }
        }
    }
}

impl Farm {
    /// Discover the nearest `.yaks/` above `cwd` and apply the schema gate.
    pub fn open(cwd: &Path) -> std::result::Result<Farm, OpenError> {
        let found = store::discover(cwd).map_err(|e| OpenError::NoFarm(e.to_string()))?;
        let root = found.root;
        let schema_warning = match store::schema_status(&root) {
            SchemaStatus::Newer(found) => {
                return Err(OpenError::SchemaTooNew {
                    found,
                    supported: store::SCHEMA,
                });
            }
            SchemaStatus::Older(v) => Some(format!(
                "farm schema v{v} predates this yaks (v{}); reading best-effort.",
                store::SCHEMA
            )),
            SchemaStatus::Compatible => None,
        };
        Ok(Farm {
            root,
            schema_warning,
            pointer_prefix: found.prefix,
        })
    }

    /// The effective per-farm config (prefix, defaults, editor mode).
    pub fn config(&self) -> store::Config {
        store::read_config(&self.root)
    }

    /// The farm's `.yaks/` root, e.g. for locating the per-user UI-state cache.
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

    /// A read-only farm-integrity pass: collect every problem worth a human's
    /// attention without touching a single file. Ordering is deterministic
    /// (by kind, then id) so text and `--json` output are stable and
    /// CI-diffable.
    ///
    /// v1 checks: an id living in two status dirs at once (the add/add merge
    /// hazard), the same id twice inside one status dir, and `parent` /
    /// `depends_on` references that point at no known yak.
    ///
    /// `strict` adds a skill-adherence check: any shorn yak lacking a recorded
    /// note is a completion without evidence. Dead (abandoned) yaks are exempt.
    pub fn doctor(&self, strict: bool) -> Result<Vec<Issue>> {
        let tasks = store::load(&self.root, &EVERY)?;
        let known = store::all_ids(&self.root);
        let mut issues = Vec::new();

        // id -> the statuses it was loaded under (one entry per file on disk).
        // A well-formed farm has exactly one file, hence one status, per id.
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

        // Strict: a shorn yak with no recorded note is a completion without
        // evidence (the yaks-working evidence-before-shear rule). Dead
        // (abandoned) yaks are exempt — abandonment needs no completion note.
        if strict {
            for t in &tasks {
                if t.status == Status::Shorn && store::parse_notes(&t.body).is_empty() {
                    issues.push(Issue {
                        kind: IssueKind::MissingEvidence,
                        message: format!(
                            "{} is {} but has no recorded note (evidence before shear)",
                            t.id,
                            t.status.dir()
                        ),
                        ids: vec![t.id.clone()],
                    });
                }
                // A `verify:` command is a commitment: a shorn yak that carries
                // one must show a recorded PASS as its most recent verify run.
                // (Catches shearing over a failing or never-run verify; it does
                // not catch verify-then-edit-then-shear — that needs a re-run,
                // which is deliberately explicit.)
                if t.status == Status::Shorn && t.verify.is_some() && !last_verify_passed(&t.body) {
                    issues.push(Issue {
                        kind: IssueKind::UnverifiedShear,
                        message: format!(
                            "{} is shorn with a verify: command but its last verify run was \
                             not a PASS (run `yaks verify {}`)",
                            t.id, t.id
                        ),
                        ids: vec![t.id.clone()],
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

    /// Rename one yak, a convenience over [`Farm::rename_many`].
    pub fn rename(&self, old: &str, new: &str, dry_run: bool) -> Result<RenameOutcome> {
        self.rename_many(&[(old.to_string(), new.to_string())], dry_run)
    }

    /// Migrate every yak whose id is `{old}-<tail>` to `{new}-<tail>`, rewriting
    /// all references (via [`Farm::rename_many`]) and, on a real successful run,
    /// flipping the farm's configured `prefix` to `new` so future ids match.
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
    /// them across the whole farm: the file name + `id` of each subject, and the
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
        // Route to a specific herd (id prefix): an explicit `--prefix` wins,
        // else this repo's pointer-file herd, else the farm's config default.
        // A chosen prefix is validated via the reference grammar so we never
        // mint an un-referenceable id.
        let prefix = match new
            .prefix
            .as_deref()
            .or(self.pointer_prefix.as_deref())
            .filter(|p| !p.is_empty())
        {
            Some(p) if refs::has_ref_shape(&format!("{p}-0000")) => p.to_string(),
            Some(p) => return Ok(CreateOutcome::InvalidPrefix(p.to_string())),
            None => cfg.prefix.clone(),
        };
        let id = store::generate_id(&self.root, &prefix)?;
        let now = store::now_iso();
        let task = Task {
            id,
            title: new.title,
            kind: new.kind.unwrap_or_else(|| cfg.default_type_for(&prefix)),
            priority: new
                .priority
                .unwrap_or_else(|| cfg.default_priority_for(&prefix)),
            status: Status::Hairy,
            created: Some(now.clone()),
            updated: Some(now),
            parent: new.parent,
            labels: new.labels,
            depends_on: new.depends_on,
            source: new.source,
            needs: None,
            verify: new.verify,
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
            // An empty string clears the source (like `verify`).
            let new = if s.is_empty() { None } else { Some(s) };
            if new != task.source {
                task.source = new;
                changed = true;
            }
        }
        if let Some(v) = edit.verify {
            task.verify = if v.is_empty() { None } else { Some(v) };
            changed = true;
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
    /// tree lives inside the farm and, in team mode, commits alongside `.yaks/`
    /// (the root `.gitignore` explicitly un-ignores `.yaks/artifacts/`, yaks-52eb)
    /// so an attachment is shareable evidence, not just local scratch.
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

    /// Rename `.yaks/artifacts/{id}/{old}` to a sanitized `new` (see
    /// [`attachment_name`]; the old extension is kept when `new` omits one)
    /// and rewrite every link to it across the farm — `artifacts/{id}/{old}`
    /// path mentions in any yak's body, plus the `![alt]` text of a link whose
    /// alt was the old file stem (what `attach` writes). Refuses to clobber an
    /// existing attachment.
    pub fn rename_attachment(
        &self,
        id: &str,
        old: &str,
        new: &str,
    ) -> Result<RenameAttachmentOutcome> {
        use RenameAttachmentOutcome as R;
        if store::load_task_by_id(&self.root, id)?.is_none() {
            return Ok(R::TaskNotFound);
        }
        let dir = self.root.join("artifacts").join(id);
        let src = dir.join(old);
        if old.contains(['/', '\\']) || old.is_empty() || !src.is_file() {
            return Ok(R::NoSuchAttachment(old.to_string()));
        }
        let Some(name) = attachment_name(new, old) else {
            return Ok(R::Invalid(new.to_string()));
        };
        if name == old {
            return Ok(R::Unchanged);
        }
        // A case-only rename on a case-insensitive filesystem "exists" already.
        if dir.join(&name).exists() && !name.eq_ignore_ascii_case(old) {
            return Ok(R::Collision(name));
        }
        std::fs::rename(&src, dir.join(&name))?;

        let old_path = format!("artifacts/{id}/{old}");
        let new_path = format!("artifacts/{id}/{name}");
        let stem = |n: &str| {
            Path::new(n)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or(n)
                .to_string()
        };
        let old_alt = format!("![{}]({new_path})", stem(old));
        let new_alt = format!("![{}]({new_path})", stem(&name));
        let mut rewritten = 0;
        for mut t in store::load(&self.root, &EVERY)? {
            if !t.body.contains(&old_path) {
                continue;
            }
            let body =
                replace_path_token(&t.body, &old_path, &new_path).replace(&old_alt, &new_alt);
            if body != t.body {
                t.body = body;
                t.updated = Some(store::now_iso());
                store::write::save(&self.root, &t)?;
                rewritten += 1;
            }
        }
        Ok(R::Renamed { name, rewritten })
    }

    pub fn transition(&self, id: &str, dest: Status) -> Result<MoveOutcome> {
        store::move_task(&self.root, id, dest)
    }

    /// Slaughter `id` (move to dead). Refuses with
    /// [`SlaughterOutcome::HasLiveDescendants`] when that would orphan live
    /// descendants, unless `family` is set, in which case the whole family
    /// (every live descendant, deepest first, then `id`) goes (yaks-05da).
    pub fn slaughter(&self, id: &str, family: bool) -> Result<SlaughterOutcome> {
        let all = store::load(&self.root, &EVERY)?;
        let Some(me) = all.iter().find(|t| t.id == id) else {
            return Ok(SlaughterOutcome::NotFound);
        };
        let doomed = live_descendants(&all, id);
        if !doomed.is_empty() && !family {
            return Ok(SlaughterOutcome::HasLiveDescendants(doomed));
        }
        if me.status == Status::Dead && doomed.is_empty() {
            return Ok(SlaughterOutcome::AlreadyDead);
        }
        let mut moved = Vec::new();
        for d in doomed.iter().map(String::as_str).chain(std::iter::once(id)) {
            if store::move_task(&self.root, d, Status::Dead)? == MoveOutcome::Moved {
                moved.push(d.to_string());
            }
        }
        Ok(SlaughterOutcome::Slaughtered(moved))
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

    /// Merge every yak from the farm at `source` into this one, preserving each
    /// yak's status (subdir) and file bytes verbatim and copying any
    /// `artifacts/<id>/` alongside. Non-destructive: the source is left intact
    /// (remove or symlink it yourself once satisfied). Refuses on any id
    /// collision — unique prefixes per herd make these rare; reconcile a clash
    /// with `rename-prefix` in the source first. `dry_run` reports the plan
    /// without writing.
    pub fn merge(&self, source: &Path, dry_run: bool) -> Result<MergeOutcome> {
        let Some(src_root) = resolve_farm_root(source) else {
            return Ok(MergeOutcome::NoSource(source.display().to_string()));
        };
        if let (Ok(a), Ok(b)) = (
            std::fs::canonicalize(&src_root),
            std::fs::canonicalize(&self.root),
        ) {
            if a == b {
                return Ok(MergeOutcome::NoSource(source.display().to_string()));
            }
        }
        // Every source yak, by walking each status dir (file stem = id), kept as
        // raw bytes so frontmatter/body round-trip exactly.
        let mut found: Vec<(String, Status, PathBuf)> = Vec::new();
        for st in [Status::Hairy, Status::Shaving, Status::Shorn, Status::Dead] {
            let Ok(rd) = std::fs::read_dir(src_root.join(st.dir())) else {
                continue;
            };
            for entry in rd.flatten() {
                let p = entry.path();
                if p.extension().and_then(|e| e.to_str()) != Some("md") {
                    continue;
                }
                if let Some(stem) = p.file_stem().and_then(|s| s.to_str()) {
                    found.push((stem.to_string(), st, p));
                }
            }
        }
        // Refuse if any id already lives in the destination (would clobber).
        let dest_ids = store::all_ids(&self.root);
        let mut collisions: Vec<String> = found
            .iter()
            .filter(|(id, _, _)| dest_ids.contains(id))
            .map(|(id, _, _)| id.clone())
            .collect();
        collisions.sort();
        collisions.dedup();
        if !collisions.is_empty() {
            return Ok(MergeOutcome::Collision(collisions));
        }
        found.sort_by(|a, b| a.0.cmp(&b.0));
        let yaks: Vec<(String, Status)> =
            found.iter().map(|(id, st, _)| (id.clone(), *st)).collect();
        let mut artifacts: Vec<String> = found
            .iter()
            .filter(|(id, _, _)| src_root.join("artifacts").join(id).is_dir())
            .map(|(id, _, _)| id.clone())
            .collect();
        artifacts.sort();
        // The incoming yaks' herds (id prefixes) that this farm doesn't already
        // declare. Merging yaks without registering their herd leaves them
        // invisible to the create/TUI herd pickers (yaks-095a).
        let cfg = store::read_config(&self.root);
        let mut incoming: Vec<String> = Vec::new();
        for (id, _, _) in &found {
            if let Some((prefix, _)) = id.split_once('-') {
                if !cfg.herds.contains_key(prefix) && !incoming.iter().any(|h| h == prefix) {
                    incoming.push(prefix.to_string());
                }
            }
        }
        incoming.sort();
        let mut herds = incoming;
        if !dry_run {
            for (_id, st, path) in &found {
                let dest_dir = self.root.join(st.dir());
                std::fs::create_dir_all(&dest_dir)?;
                let name = path.file_name().expect("md file has a name");
                std::fs::copy(path, dest_dir.join(name))
                    .with_context(|| format!("copying {}", path.display()))?;
            }
            for id in &artifacts {
                copy_dir_all(
                    &src_root.join("artifacts").join(id),
                    &self.root.join("artifacts").join(id),
                )?;
            }
            // Declare the incoming herds, so the merged yaks' herds are
            // offered by the pickers. Report what was actually added.
            herds = store::declare_herds(&self.root, &herds)?;
        }
        Ok(MergeOutcome::Done(MergePlan {
            applied: !dry_run,
            source: src_root,
            yaks,
            artifacts,
            herds,
        }))
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

/// True iff the most recent `yaks verify` run recorded in `body` was a PASS.
/// Verify-run notes are the ones `yaks verify` writes: `verify: <cmd> -> PASS…`
/// / `-> FAIL…`. A yak with no such note (verify: set but never run) is false.
fn last_verify_passed(body: &str) -> bool {
    store::parse_notes(body)
        .iter()
        .rev()
        .find(|n| n.text.starts_with("verify: ") && n.text.contains(" -> "))
        .is_some_and(|n| n.text.contains(" -> PASS"))
}

fn issue_rank(k: IssueKind) -> u8 {
    match k {
        IssueKind::DuplicateStatus => 0,
        IssueKind::DuplicateId => 1,
        IssueKind::DanglingParent => 2,
        IssueKind::DanglingDependsOn => 3,
        IssueKind::MissingEvidence => 4,
        IssueKind::UnverifiedShear => 5,
    }
}

fn fold_counts<K: std::hash::Hash + Eq, I: Iterator<Item = K>>(it: I) -> Vec<(K, usize)> {
    let mut m: std::collections::HashMap<K, usize> = std::collections::HashMap::new();
    for k in it {
        *m.entry(k).or_insert(0) += 1;
    }
    m.into_iter().collect()
}

/// Resolve a user-supplied path to a farm root: accept either the `.yaks/`
/// directory itself or a directory that contains one.
fn resolve_farm_root(p: &Path) -> Option<PathBuf> {
    let is_farm = |d: &Path| {
        [Status::Hairy, Status::Shaving, Status::Shorn, Status::Dead]
            .iter()
            .any(|st| d.join(st.dir()).is_dir())
    };
    if is_farm(p) {
        return Some(p.to_path_buf());
    }
    let nested = p.join(".yaks");
    is_farm(&nested).then_some(nested)
}

/// Recursively copy `src` into `dst` (carries a merged yak's `artifacts/<id>/`).
fn copy_dir_all(src: &Path, dst: &Path) -> Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_all(&from, &to)?;
        } else {
            std::fs::copy(&from, &to)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    static SEQ: AtomicU64 = AtomicU64::new(0);

    /// A temp farm (a `.yaks/` under a fresh parent dir) plus an open handle.
    fn temp_farm() -> (PathBuf, Farm) {
        let mut parent = std::env::temp_dir();
        let n = SEQ.fetch_add(1, Ordering::Relaxed);
        parent.push(format!("yaksrs-farm-refs-{}-{}", std::process::id(), n));
        let root = parent.join(".yaks");
        for st in [Status::Hairy, Status::Shaving, Status::Shorn, Status::Dead] {
            std::fs::create_dir_all(root.join(st.dir())).unwrap();
        }
        let farm = match Farm::open(&parent) {
            Ok(h) => h,
            Err(_) => panic!("failed to open temp farm at {parent:?}"),
        };
        (root, farm)
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
            verify: None,
            extra: Vec::new(),
            body: String::new(),
        }
    }

    /// A `needs` block is never invisible: `inbox` surfaces a blocked yak
    /// whatever its status (the shorn-yak silent-block gap), while unblocked
    /// yaks stay out.
    #[test]
    fn inbox_shows_blocked_yaks_regardless_of_status() {
        let (root, farm) = temp_farm();
        let mut blocked_hairy = task("yak-0001", Status::Hairy);
        blocked_hairy.needs = Some("human".into());
        let mut blocked_shorn = task("yak-0002", Status::Shorn);
        blocked_shorn.needs = Some("human".into());
        store::write::save(&root, &blocked_hairy).unwrap();
        store::write::save(&root, &blocked_shorn).unwrap();
        store::write::save(&root, &task("yak-0003", Status::Hairy)).unwrap(); // unblocked

        let mut got: Vec<String> = farm
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
        let (root, farm) = temp_farm();
        store::write::save(&root, &task("yak-0002", Status::Shorn)).unwrap();
        store::write::save(&root, &task("yak-0003", Status::Hairy)).unwrap();
        let mut subject = task("yak-0001", Status::Hairy);
        subject.title = "subject without an id token".into();
        subject.parent = Some("yak-0007".into()); // missing -> dangling
        subject.depends_on = vec!["yak-0002".into(), "yak-9999".into()]; // one dangling
        subject.body = "see yak-0002 and [[yak-0003]] but not yak-9999".into();
        store::write::save(&root, &subject).unwrap();

        let r = farm.refs("yak-0001").unwrap().expect("subject exists");
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
        let (_root, farm) = temp_farm();
        assert!(farm.refs("yak-dead").unwrap().is_none());
    }

    fn new_task(title: &str, prefix: Option<&str>) -> NewTask {
        NewTask {
            title: title.into(),
            prefix: prefix.map(Into::into),
            kind: None,
            priority: None,
            parent: None,
            labels: vec![],
            depends_on: vec![],
            source: None,
            description: None,
            verify: None,
        }
    }

    #[test]
    fn create_routes_to_the_requested_prefix_over_the_config_default() {
        // The farm's config prefix is `yak` (temp_farm default), but an explicit
        // prefix routes the new yak into a different farm within the same root.
        let (_root, farm) = temp_farm();
        match farm
            .create(new_task("cross-farm yak", Some("proj")))
            .unwrap()
        {
            CreateOutcome::Created(t) => {
                assert!(t.id.starts_with("proj-"), "expected proj- id, got {}", t.id)
            }
            _ => panic!("expected Created"),
        }
    }

    #[test]
    fn create_without_a_prefix_uses_the_config_default() {
        let (_root, farm) = temp_farm();
        let default = farm.config().prefix;
        match farm.create(new_task("local yak", None)).unwrap() {
            CreateOutcome::Created(t) => assert!(
                t.id.starts_with(&format!("{default}-")),
                "expected {default}- id, got {}",
                t.id
            ),
            _ => panic!("expected Created"),
        }
    }

    #[test]
    fn create_rejects_a_malformed_prefix() {
        let (_root, farm) = temp_farm();
        // Uppercase/space are outside the reference grammar, so the yak is not
        // minted rather than producing an un-referenceable id.
        assert!(matches!(
            farm.create(new_task("bad", Some("Bad Prefix"))).unwrap(),
            CreateOutcome::InvalidPrefix(_)
        ));
    }

    fn created_id(out: CreateOutcome) -> String {
        match out {
            CreateOutcome::Created(t) => t.id,
            _ => panic!("expected Created"),
        }
    }

    #[test]
    fn merge_copies_a_disjoint_farm_preserving_status() {
        let (dest_root, dest) = temp_farm();
        let (src_root, src) = temp_farm();
        // Source yaks live in a different herd (prefix) so ids can't collide.
        let a = created_id(src.create(new_task("source hairy", Some("proj"))).unwrap());
        let b = created_id(src.create(new_task("source done", Some("proj"))).unwrap());
        src.transition(&b, Status::Shorn).unwrap();

        let plan = match dest.merge(&src_root, false).unwrap() {
            MergeOutcome::Done(p) => p,
            _ => panic!("expected Done"),
        };
        assert!(plan.applied && plan.yaks.len() == 2);
        // Each landed under its original status dir, and both now resolve.
        assert!(dest_root.join("hairy").join(format!("{a}.md")).is_file());
        assert!(dest_root.join("shorn").join(format!("{b}.md")).is_file());
        let ids = store::all_ids(&dest_root);
        assert!(ids.contains(&a) && ids.contains(&b));
        // Non-destructive: the source keeps its files.
        assert!(src_root.join("hairy").join(format!("{a}.md")).is_file());
    }

    #[test]
    fn merge_refuses_on_id_collision_and_writes_nothing() {
        let (dest_root, dest) = temp_farm();
        let (src_root, _src) = temp_farm();
        store::write::save(&dest_root, &task("yak-0001", Status::Hairy)).unwrap();
        store::write::save(&src_root, &task("yak-0001", Status::Shaving)).unwrap();
        store::write::save(&src_root, &task("yak-0002", Status::Hairy)).unwrap();
        match dest.merge(&src_root, false).unwrap() {
            MergeOutcome::Collision(ids) => assert_eq!(ids, vec!["yak-0001".to_string()]),
            _ => panic!("expected Collision"),
        }
        // Refused wholesale: the non-colliding source yak was not copied either.
        assert!(!store::all_ids(&dest_root).contains("yak-0002"));
    }

    #[test]
    fn merge_dry_run_reports_but_writes_nothing() {
        let (dest_root, dest) = temp_farm();
        let (src_root, src) = temp_farm();
        let id = created_id(src.create(new_task("s", Some("proj"))).unwrap());
        match dest.merge(&src_root, true).unwrap() {
            MergeOutcome::Done(p) => assert!(!p.applied && p.yaks.len() == 1),
            _ => panic!("expected Done"),
        }
        assert!(!store::all_ids(&dest_root).contains(&id));
    }

    #[test]
    fn merge_declares_the_incoming_herd_in_config() {
        // Regression (yaks-095a): merging yaks from another herd must also
        // register that herd in the destination's `herds:` config, or the
        // merged yaks' herd is missing from the known-herd set the pickers use.
        let (dest_root, dest) = temp_farm();
        let (src_root, src) = temp_farm();
        let id = created_id(src.create(new_task("s", Some("web"))).unwrap());
        assert!(id.starts_with("web-"));
        let plan = match dest.merge(&src_root, false).unwrap() {
            MergeOutcome::Done(p) => p,
            _ => panic!("expected Done"),
        };
        assert_eq!(plan.herds, vec!["web".to_string()]);
        // The incoming herd is now declared — and so is the destination's own
        // herd, which materializing the block must not drop.
        let known = store::read_config(&dest_root).known_herds();
        assert!(known.contains(&"web".to_string()), "incoming herd declared");
        let own = store::read_config(&dest_root).prefix;
        assert!(
            known.contains(&own),
            "destination's own herd kept: {known:?}"
        );
    }

    #[test]
    fn merge_herd_declaration_is_idempotent_and_dry_run_writes_nothing() {
        let (dest_root, dest) = temp_farm();
        let (src_root, src) = temp_farm();
        created_id(src.create(new_task("s", Some("web"))).unwrap());
        // Dry run: reports the herd it *would* declare, writes no config.
        let before = std::fs::read_to_string(dest_root.join("config.yaml")).unwrap_or_default();
        match dest.merge(&src_root, true).unwrap() {
            MergeOutcome::Done(p) => assert_eq!(p.herds, vec!["web".to_string()]),
            _ => panic!("expected Done"),
        }
        let after = std::fs::read_to_string(dest_root.join("config.yaml")).unwrap_or_default();
        assert_eq!(before, after, "dry run left config.yaml untouched");
        // Apply, then declaring the same herd again adds nothing.
        dest.merge(&src_root, false).unwrap();
        let added = store::declare_herds(&dest_root, &["web".to_string()]).unwrap();
        assert!(added.is_empty(), "already-declared herd is skipped");
    }

    fn temp_dir_named(tag: &str) -> PathBuf {
        let mut p = std::env::temp_dir();
        let n = SEQ.fetch_add(1, Ordering::Relaxed);
        p.push(format!("yaks-{tag}-{}-{}", std::process::id(), n));
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn discover_follows_a_pointer_file_with_prefix() {
        let (farm_root, _f) = temp_farm();
        let repo = temp_dir_named("repo");
        std::fs::write(
            repo.join(".yaks"),
            format!("path: {}\nprefix: web\n", farm_root.display()),
        )
        .unwrap();
        let d = store::discover(&repo).unwrap();
        assert_eq!(d.root, farm_root);
        assert_eq!(d.prefix.as_deref(), Some("web"));
    }

    #[test]
    fn create_routes_via_pointer_prefix_and_explicit_wins() {
        let (farm_root, _f) = temp_farm();
        let repo = temp_dir_named("repo");
        std::fs::write(
            repo.join(".yaks"),
            format!("path: {}\nprefix: web\n", farm_root.display()),
        )
        .unwrap();
        let farm = match Farm::open(&repo) {
            Ok(f) => f,
            Err(_) => panic!("open via pointer failed"),
        };
        // No --prefix: the pointer's herd (web) routes the new yak.
        let a = created_id(farm.create(new_task("via pointer", None)).unwrap());
        assert!(a.starts_with("web-"), "expected web- id, got {a}");
        // An explicit --prefix beats the pointer.
        let b = created_id(farm.create(new_task("explicit", Some("core"))).unwrap());
        assert!(b.starts_with("core-"), "expected core- id, got {b}");
        let ids = store::all_ids(&farm_root);
        assert!(ids.contains(&a) && ids.contains(&b));
    }

    #[test]
    fn create_applies_herd_default_type_from_config() {
        let (root, farm) = temp_farm();
        std::fs::write(
            root.join("config.yaml"),
            "prefix: core\ndefault_type: task\nherds:\n  web:\n    default_type: feature\n",
        )
        .unwrap();
        // Into the web herd: inherits web's default_type (feature).
        match farm.create(new_task("web thing", Some("web"))).unwrap() {
            CreateOutcome::Created(t) => {
                assert!(t.id.starts_with("web-"));
                assert_eq!(t.kind, "feature");
            }
            _ => panic!("expected Created"),
        }
        // Into core (no per-herd override): the farm-global default_type (task).
        match farm.create(new_task("core thing", Some("core"))).unwrap() {
            CreateOutcome::Created(t) => assert_eq!(t.kind, "task"),
            _ => panic!("expected Created"),
        }
    }

    fn done(out: RenameOutcome) -> RenamePlan {
        match out {
            RenameOutcome::Done(p) => p,
            _ => panic!("expected RenameOutcome::Done"),
        }
    }

    #[test]
    fn rename_rewrites_every_reference_surface() {
        let (root, farm) = temp_farm();
        store::write::save(&root, &task("yaksrs-0001", Status::Hairy)).unwrap();
        let mut child = task("yaksrs-0002", Status::Hairy);
        child.parent = Some("yaksrs-0001".into());
        child.depends_on = vec!["yaksrs-0001".into()];
        child.body = "blocked by yaksrs-0001, see [[yaksrs-0001]] and yaksrs-0009".into();
        store::write::save(&root, &child).unwrap();

        let plan = done(farm.rename("yaksrs-0001", "yaks-0001", false).unwrap());
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
        let (root, farm) = temp_farm();
        store::write::save(&root, &task("yaksrs-0001", Status::Hairy)).unwrap();
        store::write::save(&root, &task("yaks-0001", Status::Hairy)).unwrap();
        match farm.rename("yaksrs-0001", "yaks-0001", true).unwrap() {
            RenameOutcome::Collision(id) => assert_eq!(id, "yaks-0001"),
            _ => panic!("expected collision"),
        }
    }

    #[test]
    fn rename_dry_run_leaves_files_untouched() {
        let (root, farm) = temp_farm();
        store::write::save(&root, &task("yaksrs-0001", Status::Hairy)).unwrap();
        let plan = done(farm.rename("yaksrs-0001", "yaks-0001", true).unwrap());
        assert!(!plan.applied);
        assert!(root.join("hairy/yaksrs-0001.md").is_file());
        assert!(!root.join("hairy/yaks-0001.md").exists());
    }

    #[test]
    fn rename_many_migrates_a_prefix_batch_across_statuses() {
        let (root, farm) = temp_farm();
        store::write::save(&root, &task("yaksrs-0001", Status::Hairy)).unwrap();
        let mut b = task("yaksrs-0002", Status::Shorn);
        b.depends_on = vec!["yaksrs-0001".into()];
        store::write::save(&root, &b).unwrap();

        let pairs = vec![
            ("yaksrs-0001".to_string(), "yaks-0001".to_string()),
            ("yaksrs-0002".to_string(), "yaks-0002".to_string()),
        ];
        let plan = done(farm.rename_many(&pairs, false).unwrap());
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
        let (root, farm) = temp_farm();
        store::write::save(&root, &task("yaksrs-0001", Status::Hairy)).unwrap();
        let mut b = task("yaksrs-0002", Status::Hairy);
        b.depends_on = vec!["yaksrs-0001".into()];
        store::write::save(&root, &b).unwrap();
        // A different-prefix id must be left alone (the boundary is `yaksrs-`).
        store::write::save(&root, &task("yaks-abcd", Status::Hairy)).unwrap();

        let plan = done(farm.rename_prefix("yaksrs", "yaks", false).unwrap());
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
        let (root, farm) = temp_farm();
        store::write::save(&root, &task("yaksrs-0001", Status::Hairy)).unwrap();
        let plan = done(farm.rename_prefix("yaksrs", "yaks", true).unwrap());
        assert!(!plan.applied);
        assert!(!root.join("config.yaml").exists());
        assert!(root.join("hairy/yaksrs-0001.md").is_file());
    }

    /// Slaughtering a parent with live descendants is refused (listing them)
    /// unless `family` is set, which takes the whole live family, deepest
    /// first; already-dead descendants are left alone (yaks-05da).
    #[test]
    fn slaughter_guards_and_family_sweeps() {
        let (root, farm) = temp_farm();
        let kid = |id: &str, parent: &str, st: Status| {
            let mut t = task(id, st);
            t.parent = Some(parent.into());
            t
        };
        store::write::save(&root, &task("yak-0001", Status::Hairy)).unwrap();
        store::write::save(&root, &kid("yak-0002", "yak-0001", Status::Shaving)).unwrap();
        store::write::save(&root, &kid("yak-0003", "yak-0002", Status::Shorn)).unwrap();
        store::write::save(&root, &kid("yak-0004", "yak-0001", Status::Dead)).unwrap();
        store::write::save(&root, &task("yak-0005", Status::Hairy)).unwrap();

        assert_eq!(
            farm.slaughter("yak-0001", false).unwrap(),
            SlaughterOutcome::HasLiveDescendants(vec!["yak-0003".into(), "yak-0002".into()])
        );
        assert!(
            root.join("hairy/yak-0001.md").is_file(),
            "guard must not move"
        );

        assert_eq!(
            farm.slaughter("yak-0001", true).unwrap(),
            SlaughterOutcome::Slaughtered(vec![
                "yak-0003".into(),
                "yak-0002".into(),
                "yak-0001".into()
            ])
        );
        for id in ["yak-0001", "yak-0002", "yak-0003", "yak-0004"] {
            assert!(root.join(format!("dead/{id}.md")).is_file(), "{id} dead");
        }
        assert!(
            root.join("hairy/yak-0005.md").is_file(),
            "non-family untouched"
        );

        assert_eq!(
            farm.slaughter("yak-0001", true).unwrap(),
            SlaughterOutcome::AlreadyDead
        );
        assert_eq!(
            farm.slaughter("yak-0005", false).unwrap(),
            SlaughterOutcome::Slaughtered(vec!["yak-0005".into()])
        );
        assert_eq!(
            farm.slaughter("yak-nope", true).unwrap(),
            SlaughterOutcome::NotFound
        );
    }

    /// The multi-id transition path (`yaks shorn a b c`) drives the CLI batch by
    /// calling [`Farm::transition`] once per id. A clean batch moves every id;
    /// a partial-failure batch (one good id + one nonexistent) still moves the
    /// good id and flags a failure, which the CLI turns into a non-zero exit.
    #[test]
    fn transition_batch_moves_valid_ids_and_flags_missing() {
        let (root, farm) = temp_farm();
        store::write::save(&root, &task("yak-0001", Status::Hairy)).unwrap();
        store::write::save(&root, &task("yak-0002", Status::Hairy)).unwrap();

        // All-good batch: both ids move to shorn, nothing flagged.
        let mut any_failed = false;
        for id in ["yak-0001", "yak-0002"] {
            if farm.transition(id, Status::Shorn).unwrap() != MoveOutcome::Moved {
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
            let outcome = farm.transition(id, Status::Shorn).unwrap();
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
        let (root, farm) = temp_farm();
        store::write::save(&root, &task("yak-0002", Status::Shorn)).unwrap();
        let mut subject = task("yak-0001", Status::Hairy);
        subject.parent = Some("yak-0404".into()); // no such yak
        subject.depends_on = vec!["yak-0002".into(), "yak-0405".into()]; // one dangles
        store::write::save(&root, &subject).unwrap();

        let issues = farm.doctor(false).unwrap();
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
        let (root, farm) = temp_farm();
        store::write::save(&root, &task("yak-0001", Status::Hairy)).unwrap();
        store::write::save(&root, &task("yak-0001", Status::Shorn)).unwrap();

        let issues = farm.doctor(false).unwrap();
        assert_eq!(issues.len(), 1, "one duplicated id -> one clash");
        assert_eq!(issues[0].kind, IssueKind::DuplicateStatus);
        assert_eq!(issues[0].ids, vec!["yak-0001"]);
        assert!(
            issues[0].message.contains("hairy") && issues[0].message.contains("shorn"),
            "message names both dirs: {}",
            issues[0].message
        );
    }

    /// A well-formed farm (resolvable parent + dep, one status per id) is clean.
    #[test]
    fn doctor_clean_farm_has_no_issues() {
        let (root, farm) = temp_farm();
        let mut a = task("yak-0001", Status::Hairy);
        a.parent = Some("yak-0002".into());
        a.depends_on = vec!["yak-0002".into()];
        store::write::save(&root, &a).unwrap();
        store::write::save(&root, &task("yak-0002", Status::Shorn)).unwrap();
        assert!(farm.doctor(false).unwrap().is_empty());
    }

    /// Strict mode flags a shorn yak with no recorded note (a completion without
    /// evidence), while a shorn yak carrying a note passes. A note-less DEAD
    /// (abandoned) yak is exempt — abandonment needs no completion evidence.
    /// Plain `doctor` ignores all of them.
    #[test]
    fn doctor_strict_flags_shorn_yak_without_a_note() {
        let (root, farm) = temp_farm();
        // Shorn with an evidence note: clean.
        let mut with_note = task("yak-0001", Status::Shorn);
        with_note.body =
            store::append_note("", "2026-01-02T00:00:00Z", Some("tester"), "did the work");
        store::write::save(&root, &with_note).unwrap();
        // Shorn with no note at all: an evidence-before-shear violation.
        store::write::save(&root, &task("yak-0002", Status::Shorn)).unwrap();
        // Dead with no note at all: exempt — abandonment needs no evidence.
        store::write::save(&root, &task("yak-0003", Status::Dead)).unwrap();

        // Non-strict doctor ignores evidence entirely.
        assert!(
            farm.doctor(false).unwrap().is_empty(),
            "plain doctor must not flag missing evidence"
        );

        // Strict flags only the note-less shorn yak; the note-less dead yak is exempt.
        let issues = farm.doctor(true).unwrap();
        assert_eq!(
            issues.len(),
            1,
            "only the note-less shorn yak is flagged; dead is exempt"
        );
        assert_eq!(issues[0].kind, IssueKind::MissingEvidence);
        assert_eq!(issues[0].ids, vec!["yak-0002"]);
    }

    /// Strict mode: a shorn yak that carries a `verify:` command must show a
    /// PASS as its most recent recorded verify run. A failing, never-run, or
    /// stale-then-failing verify is flagged; a passing (or fail-then-pass) one
    /// is clean; a yak with no `verify:` command is not subject to the check.
    #[test]
    fn doctor_strict_flags_shorn_yak_whose_verify_did_not_pass() {
        let (root, farm) = temp_farm();
        let note = |ts: &str, text: &str| store::append_note("", ts, Some("t"), text);

        // verify: + last run PASS -> clean.
        let mut ok = task("yak-0001", Status::Shorn);
        ok.verify = Some("true".into());
        ok.body = note("2026-01-02T00:00:00Z", "verify: `true` -> PASS (exit 0)");
        store::write::save(&root, &ok).unwrap();

        // verify: + last run FAIL -> flagged.
        let mut bad = task("yak-0002", Status::Shorn);
        bad.verify = Some("false".into());
        bad.body = note("2026-01-02T00:00:00Z", "verify: `false` -> FAIL (exit 1)");
        store::write::save(&root, &bad).unwrap();

        // verify: set but never run (has a note, so MissingEvidence is quiet) -> flagged.
        let mut never = task("yak-0003", Status::Shorn);
        never.verify = Some("true".into());
        never.body = note("2026-01-02T00:00:00Z", "did the work");
        store::write::save(&root, &never).unwrap();

        // No verify: command -> not subject to this check.
        let mut plain = task("yak-0004", Status::Shorn);
        plain.body = note("2026-01-02T00:00:00Z", "did it");
        store::write::save(&root, &plain).unwrap();

        // Multiple runs: FAIL then PASS -> the most recent wins -> clean.
        let mut recovered = task("yak-0005", Status::Shorn);
        recovered.verify = Some("flaky".into());
        let b = note("2026-01-02T00:00:00Z", "verify: `flaky` -> FAIL (exit 1)");
        recovered.body = store::append_note(
            &b,
            "2026-01-03T00:00:00Z",
            Some("t"),
            "verify: `flaky` -> PASS (exit 0)",
        );
        store::write::save(&root, &recovered).unwrap();

        // Plain doctor ignores verification entirely.
        assert!(
            farm.doctor(false)
                .unwrap()
                .iter()
                .all(|i| i.kind != IssueKind::UnverifiedShear)
        );

        let mut flagged: Vec<String> = farm
            .doctor(true)
            .unwrap()
            .into_iter()
            .filter(|i| i.kind == IssueKind::UnverifiedShear)
            .flat_map(|i| i.ids)
            .collect();
        flagged.sort();
        assert_eq!(flagged, vec!["yak-0002", "yak-0003"]);
    }

    #[test]
    fn attachment_name_sanitizes_and_keeps_extension() {
        assert_eq!(
            attachment_name("login", "paste-1.png").as_deref(),
            Some("login.png")
        );
        assert_eq!(
            attachment_name("login.jpg", "paste-1.png").as_deref(),
            Some("login.jpg")
        );
        assert_eq!(
            attachment_name(" ../my shot (v2)", "a.png").as_deref(),
            Some("my-shot-v2.png")
        );
        assert_eq!(attachment_name("../", "a.png"), None);
        assert_eq!(attachment_name("notes", "README").as_deref(), Some("notes"));
    }

    /// Renaming moves the file and rewrites links in the owner's body (alt text
    /// included) and in any other yak that references the artifact path, without
    /// touching lookalike paths.
    #[test]
    fn rename_attachment_moves_file_and_rewrites_links() {
        let (root, farm) = temp_farm();
        store::write::save(&root, &task("yak-0001", Status::Hairy)).unwrap();
        let mut other = task("yak-0002", Status::Shorn);
        other.body =
            "see .yaks/artifacts/yak-0001/paste-1.png and artifacts/yak-0001/paste-1.png.bak"
                .into();
        store::write::save(&root, &other).unwrap();
        let _ = farm.attach("yak-0001", "paste-1.png", b"png").unwrap();

        let out = farm
            .rename_attachment("yak-0001", "paste-1.png", "login screen")
            .unwrap();
        assert_eq!(
            out,
            RenameAttachmentOutcome::Renamed {
                name: "login-screen.png".into(),
                rewritten: 2
            }
        );
        let dir = root.join("artifacts/yak-0001");
        assert!(!dir.join("paste-1.png").exists());
        assert_eq!(std::fs::read(dir.join("login-screen.png")).unwrap(), b"png");
        let owner = store::load_task_by_id(&root, "yak-0001").unwrap().unwrap();
        assert!(
            owner
                .body
                .contains("![login-screen](artifacts/yak-0001/login-screen.png)"),
            "{}",
            owner.body
        );
        let other = store::load_task_by_id(&root, "yak-0002").unwrap().unwrap();
        assert_eq!(
            other.body,
            "see .yaks/artifacts/yak-0001/login-screen.png and artifacts/yak-0001/paste-1.png.bak"
        );
    }

    #[test]
    fn rename_attachment_refuses_collision_and_missing() {
        let (root, farm) = temp_farm();
        store::write::save(&root, &task("yak-0001", Status::Hairy)).unwrap();
        let _ = farm.attach("yak-0001", "a.png", b"a").unwrap();
        let _ = farm.attach("yak-0001", "b.png", b"b").unwrap();
        use RenameAttachmentOutcome as R;
        assert_eq!(
            farm.rename_attachment("yak-0001", "a.png", "b").unwrap(),
            R::Collision("b.png".into())
        );
        assert_eq!(
            std::fs::read(root.join("artifacts/yak-0001/b.png")).unwrap(),
            b"b"
        );
        assert_eq!(
            farm.rename_attachment("yak-0001", "a.png", "a").unwrap(),
            R::Unchanged
        );
        assert_eq!(
            farm.rename_attachment("yak-0001", "nope.png", "c").unwrap(),
            R::NoSuchAttachment("nope.png".into())
        );
        assert_eq!(
            farm.rename_attachment("yak-0001", "a.png", "//").unwrap(),
            R::Invalid("//".into())
        );
        assert_eq!(
            farm.rename_attachment("yak-0009", "a.png", "c").unwrap(),
            R::TaskNotFound
        );
    }
}
