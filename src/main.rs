//! yaks — a filesystem-native task tracker.
//!
//! This binary is a thin CLI over `herd::Herd` (the print-free core ops
//! facade): every command is parse args -> call one Herd op -> render (text or
//! --json). Tasks live as markdown files under `.yaks/`; status is the folder.

mod actor;
mod clipboard;
mod filter;
mod herd;
mod json;
mod model;
mod refs;
mod rollup;
mod skills;
mod store;
mod tui;

use anyhow::Result;
use clap::{Args, Parser, Subcommand};
use std::env;

use filter::FilterSpec;
use herd::{
    Commits, CreateOutcome, DepOutcome, Herd, LogEntry, MoveOutcome, NewTask, OpenError, RefKind,
    RenameOutcome, RenamePlan, Reparent, Show, Stats, TaskEdit, TaskRefs, UpdateOutcome,
};
use model::{Status, Task};

#[derive(Parser)]
#[command(
    name = "yaks",
    version,
    about = "Filesystem-native task tracker (Rust)"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

/// Shared narrowing flags (AND across fields; OR within a repeated field).
#[derive(Args, Default)]
struct FilterFlags {
    #[arg(long)]
    status: Vec<String>,
    #[arg(long = "type")]
    kind: Vec<String>,
    #[arg(long)]
    priority: Vec<u8>,
    #[arg(long)]
    label: Vec<String>,
    #[arg(long)]
    search: Option<String>,
    #[arg(long)]
    ready: bool,
    #[arg(long)]
    tangled: bool,
    #[arg(long = "parent-of")]
    parent_of: Option<String>,
}

#[derive(Args, Default)]
struct RollupArgs {
    #[command(flatten)]
    filter: FilterFlags,
    /// Print just the external keys (one per line) for pasting into a PR body.
    #[arg(long)]
    keys: bool,
    #[arg(long)]
    json: bool,
}

#[derive(Subcommand)]
enum Command {
    /// List tasks (non-dead by default; --all also includes dead).
    List {
        #[command(flatten)]
        filter: FilterFlags,
        #[arg(long)]
        all: bool,
        #[arg(long)]
        json: bool,
    },
    /// Show one task by id.
    Show {
        id: String,
        #[arg(long)]
        json: bool,
    },
    /// List the yaks a task points at (parent, deps, and id mentions in its
    /// text), flagging any formal reference that dangles.
    Refs { id: String },
    /// Show the git commits linked to a yak: those naming its id and those that
    /// touched its file across status moves.
    Commits { id: String },
    /// List hairy tasks whose dependencies are all resolved.
    #[command(visible_alias = "ready")]
    Next {
        #[command(flatten)]
        filter: FilterFlags,
        #[arg(long)]
        json: bool,
    },
    /// List hairy tasks with at least one unresolved dependency.
    #[command(visible_alias = "blocked")]
    Tangled {
        #[command(flatten)]
        filter: FilterFlags,
        #[arg(long)]
        json: bool,
    },
    /// Timestamped notes across a filtered set, oldest first (an activity log).
    Log {
        #[command(flatten)]
        filter: FilterFlags,
        /// Only notes at or after this point: a duration (2h, 3d, 1w), a date
        /// (YYYY-MM-DD), or an RFC3339 timestamp. Omit for the full log.
        #[arg(long)]
        since: Option<String>,
        /// Only notes attributed to this actor (matches the `[actor]` stamp).
        #[arg(long)]
        by: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Substring search over id/title/description.
    Search {
        query: String,
        #[command(flatten)]
        filter: FilterFlags,
        #[arg(long)]
        json: bool,
    },
    /// Show task statistics.
    Stats {
        #[arg(long)]
        json: bool,
    },
    /// Create a new task (in hairy).
    Create {
        #[arg(long)]
        title: String,
        #[arg(long = "type")]
        kind: Option<String>,
        #[arg(long)]
        priority: Option<u8>,
        #[arg(long)]
        parent: Option<String>,
        #[arg(long, num_args = 1..)]
        labels: Vec<String>,
        #[arg(long = "depends-on", num_args = 1..)]
        depends_on: Vec<String>,
        #[arg(long)]
        source: Option<String>,
        #[arg(long)]
        description: Option<String>,
    },
    /// Update fields, labels, or append a timestamped note.
    Update {
        id: String,
        #[arg(long)]
        title: Option<String>,
        #[arg(long = "type")]
        kind: Option<String>,
        #[arg(long)]
        priority: Option<u8>,
        #[arg(long)]
        description: Option<String>,
        #[arg(long = "add-label", num_args = 1..)]
        add_label: Vec<String>,
        #[arg(long = "remove-label", num_args = 1..)]
        remove_label: Vec<String>,
        #[arg(long)]
        source: Option<String>,
        #[arg(long)]
        note: Option<String>,
        /// Attribute the note to this actor (stamped as `[actor]`). Defaults to
        /// $YAKS_ACTOR, then the git user; never implies ownership.
        #[arg(long = "as")]
        as_actor: Option<String>,
    },
    /// Block a yak on a human decision and record the question. Sets the `needs`
    /// field so the yak drops out of `next`; clear it with `answer`.
    Ask {
        id: String,
        /// The question for the human (recorded as an attributed note).
        #[arg(long)]
        note: Option<String>,
        /// What the yak is waiting on. Defaults to `human`.
        #[arg(long, default_value = "human")]
        needs: String,
        #[arg(long = "as")]
        as_actor: Option<String>,
    },
    /// Clear a yak's `needs` block (answer it) and record the reply. The
    /// human-reserved counterpart to `ask`; returns the yak to `next`.
    Answer {
        id: String,
        /// The reply/decision (recorded as an attributed note).
        #[arg(long)]
        note: Option<String>,
        #[arg(long = "as")]
        as_actor: Option<String>,
    },
    /// List yaks awaiting a human (the `needs` inbox).
    Inbox {
        #[command(flatten)]
        filter: FilterFlags,
        #[arg(long)]
        json: bool,
    },
    /// Start shaving one or more yaks (move to shaving).
    #[command(visible_alias = "work")]
    Shave {
        #[arg(required = true, num_args = 1..)]
        ids: Vec<String>,
    },
    /// Mark one or more yaks shorn (move to shorn).
    #[command(visible_alias = "close")]
    Shorn {
        #[arg(required = true, num_args = 1..)]
        ids: Vec<String>,
    },
    /// Regrow one or more shorn yaks (move back to hairy).
    #[command(visible_alias = "reopen")]
    Regrow {
        #[arg(required = true, num_args = 1..)]
        ids: Vec<String>,
    },
    /// Slaughter one or more yaks (move to dead).
    Slaughter {
        #[arg(required = true, num_args = 1..)]
        ids: Vec<String>,
    },
    /// Revive one or more dead yaks (move back to hairy).
    Revive {
        #[arg(required = true, num_args = 1..)]
        ids: Vec<String>,
    },
    /// Add or remove a dependency.
    Dep {
        #[command(subcommand)]
        action: DepAction,
    },
    /// Move a task under a new parent (--parent) or to top-level (--unparent).
    Reparent {
        id: String,
        #[arg(long)]
        parent: Option<String>,
        #[arg(long)]
        unparent: bool,
    },
    /// Rename a yak, updating its file + id and every reference to it
    /// (parent, depends_on, and body/title mentions) across the herd.
    Rename {
        old: String,
        new: String,
        /// Preview the change without writing anything.
        #[arg(long)]
        dry_run: bool,
    },
    /// Migrate every yak from one id prefix to another (e.g. yaksrs -> yaks),
    /// rewriting all references and updating the herd's configured prefix.
    RenamePrefix {
        old: String,
        new: String,
        /// Preview the change without writing anything.
        #[arg(long)]
        dry_run: bool,
    },
    /// Create a new .yaks/ herd in the current directory (hairy/ shaving/
    /// shorn/ dead/ + config.yaml + schema). Works without an existing herd.
    Init {
        /// Id prefix for new yaks (default: yak).
        #[arg(long)]
        prefix: Option<String>,
        /// Default task type for new yaks (default: task).
        #[arg(long = "type")]
        kind: Option<String>,
        /// Default priority for new yaks (default: 3).
        #[arg(long)]
        priority: Option<u8>,
        /// Use emacs keybindings in embedded editors instead of vim.
        #[arg(long)]
        emacs: bool,
    },
    /// Install the bundled agent skills (yak, yak-tracker) into a skills
    /// directory. Works anywhere — no herd required.
    Skills {
        #[command(subcommand)]
        action: SkillsAction,
    },
    /// Group yaks by the external issue they roll up to.
    Rollup(RollupArgs),
    /// Open the interactive terminal UI.
    Tui {
        /// Drive the TUI headlessly: read actions on stdin, emit text snapshots
        /// on stdout (for agents / scripted tests). No real terminal is used.
        #[arg(long)]
        headless: bool,
        /// Terminal size for headless mode, e.g. "100x30" (default 80x24).
        #[arg(long)]
        size: Option<String>,
        /// In headless mode, also emit style information after the char grid.
        #[arg(long)]
        style: bool,
        /// Headless style encoding: parallel | interleaved | spans. Implies
        /// --style; defaults to parallel when only --style is given.
        #[arg(long)]
        style_encoding: Option<String>,
        /// Headless: after the first frame, emit only changed body lines.
        #[arg(long)]
        diff: bool,
    },
}

#[derive(Subcommand)]
enum DepAction {
    /// Add DEP_ID as a dependency of ID.
    Add { id: String, dep_id: String },
    /// Remove DEP_ID as a dependency of ID.
    Remove { id: String, dep_id: String },
}

#[derive(Subcommand)]
enum SkillsAction {
    /// Write the bundled SKILL.md files into a skills directory
    /// (default: ~/.agents/skills). Works without a herd.
    Install {
        /// Target skills directory (e.g. ~/.claude/skills). Defaults to ~/.agents/skills.
        #[arg(long)]
        dir: Option<String>,
        /// Overwrite existing SKILL.md files.
        #[arg(long)]
        force: bool,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // `init` creates a herd where none exists, and `skills` installs the skill
    // that teaches an agent how to use yaks (likely before any herd exists) —
    // both must run without opening a herd, so handle them first.
    if let Command::Init {
        prefix,
        kind,
        priority,
        emacs,
    } = &cli.command
    {
        return run_init(prefix.clone(), kind.clone(), *priority, *emacs);
    }
    if let Command::Skills { action } = &cli.command {
        return run_skills(action);
    }

    let herd = match Herd::open(&env::current_dir()?) {
        Ok(h) => h,
        Err(OpenError::SchemaTooNew { found, supported }) => {
            eprintln!(
                "error: this herd uses schema v{found}, newer than this yaks supports (v{supported}). Upgrade yaks."
            );
            std::process::exit(1);
        }
        Err(OpenError::NoHerd(m)) => {
            eprintln!("error: {m}");
            std::process::exit(1);
        }
    };
    if let Some(w) = &herd.schema_warning {
        eprintln!("warning: {w}");
    }

    match cli.command {
        Command::List { filter, all, json } => {
            let rows = herd.list(build_spec(filter), all)?;
            render_rows(&rows, json, "No tasks found.")?;
        }
        Command::Search {
            query,
            filter,
            json,
        } => {
            let mut spec = build_spec(filter);
            spec.search = Some(query);
            let rows = herd.list(spec, false)?;
            render_rows(&rows, json, "No tasks found.")?;
        }
        Command::Log {
            filter,
            since,
            by,
            json,
        } => {
            let entries = herd.log(build_spec(filter), since.as_deref(), by.as_deref())?;
            render_log(&entries, json)?;
        }
        Command::Show { id, json } => match herd.show(&id)? {
            None => {
                eprintln!("no such task: {id}");
                std::process::exit(1);
            }
            Some(s) => {
                if json {
                    json::print(&json::show_value(&s.task, &s.children))?;
                } else {
                    render_show(&s);
                }
            }
        },
        Command::Rename { old, new, dry_run } => report_rename(herd.rename(&old, &new, dry_run)?),
        Command::RenamePrefix { old, new, dry_run } => {
            report_rename(herd.rename_prefix(&old, &new, dry_run)?)
        }
        Command::Init { .. } => unreachable!("init is handled before opening a herd"),
        Command::Skills { .. } => unreachable!("skills is handled before opening a herd"),
        Command::Refs { id } => match herd.refs(&id)? {
            None => {
                eprintln!("no such task: {id}");
                std::process::exit(1);
            }
            Some(r) => render_refs(&r),
        },
        Command::Commits { id } => match herd.commits(&id)? {
            None => {
                eprintln!("no such task: {id}");
                std::process::exit(1);
            }
            Some(c) => render_commits(&c),
        },
        Command::Next { filter, json } => {
            let rows = herd.next(build_spec(filter))?;
            if json {
                json::print(&json::tasks_array(&rows))?;
            } else if rows.is_empty() {
                println!("No yaks ready to shave.");
            } else {
                println!("Ready to shave (all dependencies met):");
                for t in &rows {
                    println!("{}", fmt_plain_row(t));
                }
            }
        }
        Command::Tangled { filter, json } => {
            let rows = herd.tangled(build_spec(filter))?;
            if json {
                json::print(&json::tangled_array(&rows))?;
            } else if rows.is_empty() {
                println!("No tangled yaks.");
            } else {
                println!("Tangled yaks:");
                for (t, waiting) in &rows {
                    println!(
                        "  {}  {}  (waiting on: {})",
                        t.id,
                        t.title,
                        waiting.join(", ")
                    );
                }
            }
        }
        Command::Stats { json } => {
            let s = herd.stats()?;
            if json {
                json::print(&json::stats_value(&s))?;
            } else {
                render_stats(&s);
            }
        }
        Command::Create {
            title,
            kind,
            priority,
            parent,
            labels,
            depends_on,
            source,
            description,
        } => {
            let new = NewTask {
                title,
                kind,
                priority,
                parent,
                labels,
                depends_on,
                source,
                description,
            };
            match herd.create(new)? {
                CreateOutcome::ParentNotFound(p) => {
                    eprintln!("error: parent task {p} not found");
                    std::process::exit(1);
                }
                CreateOutcome::Created(t) => println!("Created {}: {}", t.id, t.title),
            }
        }
        Command::Update {
            id,
            title,
            kind,
            priority,
            description,
            add_label,
            remove_label,
            source,
            note,
            as_actor,
        } => {
            let actor = note
                .as_ref()
                .and_then(|_| actor::resolve(as_actor.as_deref()));
            let edit = TaskEdit {
                title,
                kind,
                priority,
                description,
                add_labels: add_label,
                remove_labels: remove_label,
                source,
                note,
                actor,
            };
            match herd.update(&id, edit)? {
                UpdateOutcome::NotFound => {
                    eprintln!("error: task {id} not found");
                    std::process::exit(1);
                }
                UpdateOutcome::Updated => println!("Updated {id}"),
                UpdateOutcome::NoChanges => println!("No changes specified."),
            }
        }
        Command::Ask {
            id,
            note,
            needs,
            as_actor,
        } => {
            let actor = actor::resolve(as_actor.as_deref());
            match herd.set_needs(&id, Some(needs.clone()), actor.as_deref(), note.as_deref())? {
                None => {
                    eprintln!("error: task {id} not found");
                    std::process::exit(1);
                }
                Some(st) if matches!(st, Status::Shorn | Status::Dead) => {
                    eprintln!(
                        "warning: {id} is {st:?} — blocking finished work; did you mean a hairy yak? (it will still show in `inbox`)"
                    );
                    println!("Asked {id}: needs {needs}");
                }
                Some(_) => {
                    println!("Asked {id}: needs {needs} (dropped from next until answered)")
                }
            }
        }
        Command::Answer { id, note, as_actor } => {
            let actor = actor::resolve(as_actor.as_deref());
            match herd.set_needs(&id, None, actor.as_deref(), note.as_deref())? {
                None => {
                    eprintln!("error: task {id} not found");
                    std::process::exit(1);
                }
                Some(_) => println!("Answered {id}: needs cleared (back in next)"),
            }
        }
        Command::Inbox { filter, json } => {
            let rows = herd.inbox(build_spec(filter))?;
            render_rows(&rows, json, "Inbox empty: nothing awaiting a human.")?;
        }
        Command::Shave { ids } => transition_many(
            &herd,
            &ids,
            Status::Shaving,
            "already being shaved",
            "Shaving",
        )?,
        Command::Shorn { ids } => {
            transition_many(&herd, &ids, Status::Shorn, "already shorn", "Shorn!")?
        }
        Command::Regrow { ids } => {
            transition_many(&herd, &ids, Status::Hairy, "already hairy", "Regrown:")?
        }
        Command::Slaughter { ids } => {
            transition_many(&herd, &ids, Status::Dead, "already dead", "Slaughtered:")?
        }
        Command::Revive { ids } => {
            transition_many(&herd, &ids, Status::Hairy, "already hairy", "Revived:")?
        }
        Command::Dep { action } => match action {
            DepAction::Add { id, dep_id } => match herd.dep_add(&id, &dep_id)? {
                DepOutcome::TaskNotFound => {
                    eprintln!("error: task {id} not found");
                    std::process::exit(1);
                }
                DepOutcome::DepNotFound => {
                    eprintln!("error: dependency task {dep_id} not found");
                    std::process::exit(1);
                }
                DepOutcome::AlreadyDep => println!("{dep_id} is already a dependency of {id}"),
                DepOutcome::Added => println!("Added dependency: {id} -> {dep_id}"),
                _ => {}
            },
            DepAction::Remove { id, dep_id } => match herd.dep_remove(&id, &dep_id)? {
                DepOutcome::TaskNotFound => {
                    eprintln!("error: task {id} not found");
                    std::process::exit(1);
                }
                DepOutcome::NotDep => println!("{dep_id} is not a dependency of {id}"),
                DepOutcome::Removed => println!("Removed dependency: {id} -> {dep_id}"),
                _ => {}
            },
        },
        Command::Reparent {
            id,
            parent,
            unparent,
        } => {
            let new_parent = if unparent {
                None
            } else if parent.is_some() {
                parent
            } else {
                eprintln!("error: specify --parent TASK_ID or --unparent");
                std::process::exit(1);
            };
            match herd.reparent(&id, new_parent)? {
                Reparent::Error(msg) => {
                    eprintln!("error: {msg}");
                    std::process::exit(1);
                }
                Reparent::Done { new_parent: None } => println!("Promoted {id} to top-level"),
                Reparent::Done {
                    new_parent: Some(p),
                } => println!("Reparented {id} under {p}"),
            }
        }
        Command::Rollup(args) => {
            let (groups, unsourced) = herd.rollup(&build_spec(args.filter))?;
            if args.keys {
                let mut seen = std::collections::HashSet::new();
                let keys: Vec<String> = groups
                    .iter()
                    .map(|g| g.head())
                    .filter(|k| seen.insert(k.clone()))
                    .collect();
                if args.json {
                    json::print(&serde_json::json!(keys))?;
                } else {
                    for k in keys {
                        println!("{k}");
                    }
                }
            } else if args.json {
                json::print(&json::rollup_value(&groups))?;
            } else if groups.is_empty() {
                println!("No yaks with an external source.");
            } else {
                for g in &groups {
                    println!("{}  ({})  {}", g.head(), g.tracker, g.source);
                    for y in &g.yaks {
                        let mut row = fmt_row(&y.task);
                        if let Some(from) = &y.inherited_from {
                            row.push_str(&format!("  (via {from})"));
                        }
                        println!("{row}");
                    }
                    println!();
                }
                if unsourced > 0 {
                    let noun = if unsourced == 1 { "yak" } else { "yaks" };
                    println!("{unsourced} {noun} in scope with no external source (omitted).");
                }
            }
        }
        Command::Tui {
            headless,
            size,
            style,
            style_encoding,
            diff,
        } => {
            let app = tui::App::with_herd(herd)?;
            if headless {
                let (w, h) = parse_size(size.as_deref());
                let encoding = match style_encoding.as_deref() {
                    Some(name) => match toque::StyleEncoding::parse(name) {
                        Some(e) => Some(e),
                        None => anyhow::bail!(
                            "unknown --style-encoding '{name}' (expected parallel|interleaved|spans)"
                        ),
                    },
                    None if style => Some(toque::StyleEncoding::Parallel),
                    None => None,
                };
                toque::run(
                    app,
                    toque::DriverOpts {
                        width: w,
                        height: h,
                        style: encoding,
                        diff,
                    },
                )?;
            } else {
                tui::run(app)?;
            }
        }
    }
    Ok(())
}

/// Parse a "WxH" size string; falls back to 80x24 on absence or bad input.
fn parse_size(s: Option<&str>) -> (u16, u16) {
    let default = (80, 24);
    let Some(s) = s else { return default };
    let Some((w, h)) = s.split_once(['x', 'X']) else {
        return default;
    };
    match (w.trim().parse(), h.trim().parse()) {
        (Ok(w), Ok(h)) if w > 0 && h > 0 => (w, h),
        _ => default,
    }
}

// -- CLI arg mapping ------------------------------------------------------

fn build_spec(f: FilterFlags) -> FilterSpec {
    FilterSpec {
        statuses: f.status.iter().filter_map(|s| parse_status(s)).collect(),
        types: f.kind,
        priorities: f.priority,
        labels: f.label,
        search: f.search,
        ready_only: f.ready,
        tangled_only: f.tangled,
        parent: f.parent_of,
    }
}

fn parse_status(s: &str) -> Option<Status> {
    match s.to_lowercase().as_str() {
        "hairy" => Some(Status::Hairy),
        "shaving" => Some(Status::Shaving),
        "shorn" => Some(Status::Shorn),
        "dead" => Some(Status::Dead),
        _ => None,
    }
}

// -- rendering ------------------------------------------------------------

/// Transition a single yak, printing a per-id result line. Returns `true` on
/// success (moved or already there) and `false` on failure (not found), so a
/// batch caller can process every id and set the exit code once at the end.
fn transition(herd: &Herd, id: &str, dest: Status, already: &str, done: &str) -> Result<bool> {
    match herd.transition(id, dest)? {
        MoveOutcome::NotFound => {
            eprintln!("error: task {id} not found");
            Ok(false)
        }
        MoveOutcome::AlreadyThere => {
            println!("{id} is {already}");
            Ok(true)
        }
        MoveOutcome::Moved => {
            println!("{done} {id}");
            Ok(true)
        }
    }
}

/// Transition every id in `ids`, one at a time. All ids are processed even if
/// one fails (no abort-on-first-error); if any id was not found, the process
/// exits non-zero after the whole batch is handled.
fn transition_many(
    herd: &Herd,
    ids: &[String],
    dest: Status,
    already: &str,
    done: &str,
) -> Result<()> {
    let mut any_failed = false;
    for id in ids {
        if !transition(herd, id, dest, already, done)? {
            any_failed = true;
        }
    }
    if any_failed {
        std::process::exit(1);
    }
    Ok(())
}

fn render_rows(rows: &[Task], json: bool, empty_msg: &str) -> Result<()> {
    if json {
        json::print(&json::tasks_array(rows))?;
    } else if rows.is_empty() {
        println!("{empty_msg}");
    } else {
        for t in rows {
            println!("{}", fmt_row(t));
        }
    }
    Ok(())
}

fn render_log(entries: &[LogEntry], json: bool) -> Result<()> {
    if json {
        json::print(&json::log_array(entries))?;
    } else if entries.is_empty() {
        println!("No notes found.");
    } else {
        for e in entries {
            let who = e
                .actor
                .as_deref()
                .map(|a| format!("  [{a}]"))
                .unwrap_or_default();
            println!("\u{25b8} {}  {}  {}{}", e.ts, e.id, e.title, who);
            for line in e.note.lines() {
                println!("    {line}");
            }
        }
    }
    Ok(())
}

fn render_stats(s: &Stats) {
    println!(
        "Total: {}  Hairy: {}  Shaving: {}  Shorn: {}",
        s.total, s.hairy, s.shaving, s.shorn
    );
    if !s.by_type.is_empty() {
        println!("By type:");
        for (k, v) in &s.by_type {
            println!("  {k}: {v}");
        }
    }
    if !s.by_priority.is_empty() {
        println!("By priority:");
        for (k, v) in &s.by_priority {
            println!("  p{k}: {v}");
        }
    }
}

fn report_rename(out: RenameOutcome) {
    match out {
        RenameOutcome::NotFound(id) => {
            eprintln!("error: no such task: {id}");
            std::process::exit(1);
        }
        RenameOutcome::Invalid(id) => {
            eprintln!("error: invalid target id: {id}");
            std::process::exit(1);
        }
        RenameOutcome::Collision(id) => {
            eprintln!("error: target id already exists: {id}");
            std::process::exit(1);
        }
        RenameOutcome::NothingToRename => println!("Nothing to rename."),
        RenameOutcome::Done(plan) => render_rename(&plan),
    }
}

fn run_init(
    prefix: Option<String>,
    kind: Option<String>,
    priority: Option<u8>,
    emacs: bool,
) -> Result<()> {
    let mut cfg = store::InitConfig::default();
    if let Some(p) = prefix {
        cfg.prefix = p;
    }
    if let Some(k) = kind {
        cfg.default_type = k;
    }
    if let Some(p) = priority {
        cfg.default_priority = p;
    }
    if emacs {
        cfg.vim_mode = false;
    }

    let root = env::current_dir()?.join(".yaks");
    match store::init(&root, &cfg)? {
        store::InitOutcome::AlreadyExists => {
            eprintln!(
                "error: {} already exists — leaving it untouched.",
                root.display()
            );
            std::process::exit(1);
        }
        store::InitOutcome::Created => {
            println!("Initialized empty yaks herd in {}", root.display());
            println!(
                "  prefix {}  ·  default type {}  ·  default priority {}",
                cfg.prefix, cfg.default_type, cfg.default_priority
            );
            println!("Create your first yak with: yaks create --title \"…\"");
            Ok(())
        }
    }
}

fn run_skills(action: &SkillsAction) -> Result<()> {
    match action {
        SkillsAction::Install { dir, force } => {
            let base = match dir {
                Some(d) => skills::expand_tilde(d),
                None => skills::default_dir(),
            };
            let installed = skills::install(&base, *force)?;
            for i in &installed {
                if i.skipped {
                    println!(
                        "skip {}: {} exists (use --force to overwrite)",
                        i.name,
                        i.path.display()
                    );
                } else {
                    println!("installed {} -> {}", i.name, i.path.display());
                }
            }
            println!(
                "\nThe skill activates when a .yaks/ directory is present. For another agent, re-run with --dir pointing at its skills directory (e.g. --dir ~/.claude/skills)."
            );
            Ok(())
        }
    }
}

fn render_rename(plan: &RenamePlan) {
    let head = if plan.applied {
        "Renamed"
    } else {
        "Dry run \u{2014} would rename"
    };
    println!("{head}:");
    for (old, new) in &plan.renames {
        println!("  {old} -> {new}");
    }
    println!("Reference edits: {} file(s)", plan.edits.len());
    for e in &plan.edits {
        let mut parts: Vec<String> = Vec::new();
        if e.new_id.is_some() {
            parts.push("id".to_string());
        }
        for f in &e.fields {
            if *f != "body" {
                parts.push((*f).to_string());
            }
        }
        if !e.body_lines.is_empty() {
            let lines: Vec<String> = e.body_lines.iter().map(|n| format!("L{n}")).collect();
            parts.push(format!("body:{}", lines.join(",")));
        }
        println!("  {:<14} {}", e.id, parts.join(", "));
    }
}

fn render_refs(r: &TaskRefs) {
    println!("{}  {}", r.id, r.title);
    if r.entries.is_empty() {
        println!("  (no references)");
        return;
    }
    for e in &r.entries {
        let kind = match e.kind {
            RefKind::Parent => "parent",
            RefKind::Depends => "depends",
            RefKind::Mention => "mention",
        };
        let status = if e.resolved { "ok" } else { "DANGLING" };
        let loc = e.line.map(|n| format!("  body:L{n}")).unwrap_or_default();
        println!("  {kind:<8} {:<14} {status}{loc}", e.id);
    }
}

fn render_commits(c: &Commits) {
    println!("Commits mentioning {}:", c.id);
    if c.by_message.is_empty() {
        println!("  (none)");
    } else {
        for l in &c.by_message {
            println!("  {l}");
        }
    }
    println!("\nCommits touching {}:", c.path.display());
    if c.by_file.is_empty() {
        println!("  (none)");
    } else {
        for l in &c.by_file {
            println!("  {l}");
        }
    }
}

fn render_show(s: &Show) {
    let t = &s.task;
    println!("id:       {}", t.id);
    println!("title:    {}", t.title);
    println!("status:   {:?}", t.status);
    println!("type:     {}", t.kind);
    println!("priority: {}", t.priority);
    if let Some(c) = &t.created {
        println!("created:  {c}");
    }
    if let Some(u) = &t.updated {
        println!("updated:  {u}");
    }
    if let Some(p) = &t.parent {
        println!("parent:   {p}");
    }
    if !t.labels.is_empty() {
        println!("labels:   {}", t.labels.join(", "));
    }
    if !t.depends_on.is_empty() {
        println!("depends:  {}", t.depends_on.join(", "));
    }
    if let Some(src) = &t.source {
        println!("source:   {src}");
    }
    if let Some(n) = &t.needs {
        println!("needs:    {n}");
    }
    if !t.body.is_empty() {
        println!("\n{}", t.body);
    }
    if !s.children.is_empty() {
        println!("\nChildren:");
        for c in &s.children {
            println!("  [{}] {}  {}", c.status.glyph(), c.id, c.title);
        }
    }
}

/// `  [X] id  pN type     title [labels] (deps: ...) ⚠ needs:<who>`
fn fmt_row(t: &Task) -> String {
    let labels = if t.labels.is_empty() {
        String::new()
    } else {
        format!(" [{}]", t.labels.join(","))
    };
    let deps = if t.depends_on.is_empty() {
        String::new()
    } else {
        format!(" (deps: {})", t.depends_on.join(","))
    };
    // Make a needs-blocked yak visually distinct from a ready one in list/next.
    let needs = match &t.needs {
        Some(who) => format!(" \u{26a0} needs:{who}"),
        None => String::new(),
    };
    format!(
        "  [{}] {}  p{} {:8} {}{}{}{}",
        t.status.glyph(),
        t.id,
        t.priority,
        t.kind,
        t.title,
        labels,
        deps,
        needs
    )
}

/// `  id  pN type     title` (no status glyph) — matches Python cmd_next.
fn fmt_plain_row(t: &Task) -> String {
    format!("  {}  p{} {:8} {}", t.id, t.priority, t.kind, t.title)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn task() -> Task {
        Task {
            id: "yak-0001".into(),
            title: "a title".into(),
            kind: "feature".into(),
            priority: 3,
            status: Status::Hairy,
            created: None,
            updated: None,
            parent: None,
            labels: vec![],
            depends_on: vec![],
            source: None,
            needs: None,
            extra: vec![],
            body: String::new(),
        }
    }

    #[test]
    fn fmt_row_marks_needs_blocked() {
        let mut t = task();
        t.needs = Some("human".into());
        let row = fmt_row(&t);
        // A blocked yak carries a visible, greppable marker naming who it needs.
        assert!(row.contains("\u{26a0} needs:human"), "row was: {row:?}");
    }

    #[test]
    fn fmt_row_ready_has_no_needs_marker() {
        // A ready yak (needs == None) must stay distinguishable: no marker at all.
        let row = fmt_row(&task());
        assert!(!row.contains("needs:"), "row was: {row:?}");
        assert!(!row.contains('\u{26a0}'), "row was: {row:?}");
    }
}
