//! yaks-rs — a filesystem-native task tracker (Rust port).
//!
//! Reads and writes the SAME `.yaks/` files as the Python `yaks` tool. This
//! binary is a thin CLI over `herd::Herd` (the print-free core ops facade):
//! every command is parse args -> call one Herd op -> render (text or --json).

mod filter;
mod herd;
mod json;
mod model;
mod rollup;
mod store;
mod tui;

use anyhow::Result;
use clap::{Args, Parser, Subcommand};
use std::env;

use filter::FilterSpec;
use herd::{
    CreateOutcome, DepOutcome, Herd, MoveOutcome, NewTask, OpenError, Reparent, Show, Stats,
    TaskEdit, UpdateOutcome,
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
    },
    /// Start shaving a yak (move to shaving).
    #[command(visible_alias = "work")]
    Shave { id: String },
    /// Mark a yak shorn (move to shorn).
    #[command(visible_alias = "close")]
    Shorn { id: String },
    /// Regrow a shorn yak (move back to hairy).
    #[command(visible_alias = "reopen")]
    Regrow { id: String },
    /// Slaughter a yak (move to dead).
    Slaughter { id: String },
    /// Revive a dead yak (move back to hairy).
    Revive { id: String },
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
    /// Group yaks by the external issue they roll up to.
    Rollup(RollupArgs),
    /// Open the interactive terminal UI.
    Tui,
}

#[derive(Subcommand)]
enum DepAction {
    /// Add DEP_ID as a dependency of ID.
    Add { id: String, dep_id: String },
    /// Remove DEP_ID as a dependency of ID.
    Remove { id: String, dep_id: String },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
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
        } => {
            let edit = TaskEdit {
                title,
                kind,
                priority,
                description,
                add_labels: add_label,
                remove_labels: remove_label,
                source,
                note,
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
        Command::Shave { id } => transition(
            &herd,
            &id,
            Status::Shaving,
            "already being shaved",
            "Shaving",
        )?,
        Command::Shorn { id } => transition(&herd, &id, Status::Shorn, "already shorn", "Shorn!")?,
        Command::Regrow { id } => {
            transition(&herd, &id, Status::Hairy, "already hairy", "Regrown:")?
        }
        Command::Slaughter { id } => {
            transition(&herd, &id, Status::Dead, "already dead", "Slaughtered:")?
        }
        Command::Revive { id } => {
            transition(&herd, &id, Status::Hairy, "already hairy", "Revived:")?
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
        Command::Tui => {
            let tasks = herd.list(FilterSpec::default(), false)?;
            tui::run(tui::App::new(tasks))?;
        }
    }
    Ok(())
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

fn transition(herd: &Herd, id: &str, dest: Status, already: &str, done: &str) -> Result<()> {
    match herd.transition(id, dest)? {
        MoveOutcome::NotFound => {
            eprintln!("error: task {id} not found");
            std::process::exit(1);
        }
        MoveOutcome::AlreadyThere => println!("{id} is {already}"),
        MoveOutcome::Moved => println!("{done} {id}"),
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

/// `  [X] id  pN type     title [labels] (deps: ...)`
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
    format!(
        "  [{}] {}  p{} {:8} {}{}{}",
        t.status.glyph(),
        t.id,
        t.priority,
        t.kind,
        t.title,
        labels,
        deps
    )
}

/// `  id  pN type     title` (no status glyph) — matches Python cmd_next.
fn fmt_plain_row(t: &Task) -> String {
    format!("  {}  p{} {:8} {}", t.id, t.priority, t.kind, t.title)
}
