//! yaks-rs — a filesystem-native task tracker (Rust port).
//!
//! Reads and writes the SAME `.yaks/` files as the Python `yaks` tool.
//! Commands: list, show, next, tangled, search, stats, create, update, and
//! the status moves (shave/shorn/regrow/slaughter/revive).

mod filter;
mod model;
mod store;

use anyhow::Result;
use clap::{Args, Parser, Subcommand};
use std::env;
use std::path::Path;

use filter::FilterSpec;
use model::{Status, Task};
use store::{DepOutcome, MoveOutcome, Reparent};

#[derive(Parser)]
#[command(name = "yaks", version, about = "Filesystem-native task tracker (Rust)")]
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

#[derive(Subcommand)]
enum Command {
    /// List tasks (non-dead by default; --all also includes dead).
    List {
        #[command(flatten)]
        filter: FilterFlags,
        #[arg(long)]
        all: bool,
    },
    /// Show one task by id.
    Show { id: String },
    /// List hairy tasks whose dependencies are all resolved.
    #[command(visible_alias = "ready")]
    Next {
        #[command(flatten)]
        filter: FilterFlags,
    },
    /// List hairy tasks with at least one unresolved dependency.
    #[command(visible_alias = "blocked")]
    Tangled {
        #[command(flatten)]
        filter: FilterFlags,
    },
    /// Substring search over id/title/description.
    Search {
        query: String,
        #[command(flatten)]
        filter: FilterFlags,
    },
    /// Show task statistics.
    Stats,
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
}

#[derive(Subcommand)]
enum DepAction {
    /// Add DEP_ID as a dependency of ID.
    Add { id: String, dep_id: String },
    /// Remove DEP_ID as a dependency of ID.
    Remove { id: String, dep_id: String },
}

const NON_DEAD: [Status; 3] = [Status::Hairy, Status::Shaving, Status::Shorn];
const EVERY: [Status; 4] = [Status::Hairy, Status::Shaving, Status::Shorn, Status::Dead];

fn main() -> Result<()> {
    let cli = Cli::parse();
    let root = store::discover_root(&env::current_dir()?)?;

    match cli.command {
        Command::List { filter, all } => {
            let tasks = store::load(&root, &EVERY)?;
            let mut rows = filter::apply(&tasks, &build_spec(filter), all);
            rows.sort_by(|a, b| status_rank(a.status).cmp(&status_rank(b.status)).then_with(|| a.id.cmp(&b.id)));
            if rows.is_empty() {
                println!("No tasks found.");
            } else {
                for t in rows {
                    println!("{}", fmt_row(t));
                }
            }
        }
        Command::Show { id } => {
            let all = store::load(&root, &EVERY)?;
            match all.iter().find(|t| t.id == id) {
                Some(t) => print_task(t),
                None => {
                    eprintln!("no such task: {id}");
                    std::process::exit(1);
                }
            }
        }
        Command::Next { filter } => {
            let mut spec = build_spec(filter);
            spec.statuses = vec![Status::Hairy];
            spec.ready_only = true;
            let tasks = store::load(&root, &EVERY)?;
            let rows = filter::apply(&tasks, &spec, false);
            if rows.is_empty() {
                println!("No yaks ready to shave.");
            } else {
                println!("Ready to shave (all dependencies met):");
                for t in rows {
                    println!("{}", fmt_plain_row(t));
                }
            }
        }
        Command::Tangled { filter } => {
            let mut spec = build_spec(filter);
            spec.statuses = vec![Status::Hairy];
            spec.tangled_only = true;
            let tasks = store::load(&root, &EVERY)?;
            let resolved = filter::resolved_ids(&tasks);
            let rows = filter::apply(&tasks, &spec, false);
            if rows.is_empty() {
                println!("No tangled yaks.");
            } else {
                println!("Tangled yaks:");
                for t in rows {
                    let waiting = filter::unresolved_deps(t, &resolved).join(", ");
                    println!("  {}  {}  (waiting on: {})", t.id, t.title, waiting);
                }
            }
        }
        Command::Search { query, filter } => {
            let mut spec = build_spec(filter);
            spec.search = Some(query);
            let tasks = store::load(&root, &EVERY)?;
            let mut rows = filter::apply(&tasks, &spec, false);
            rows.sort_by(|a, b| status_rank(a.status).cmp(&status_rank(b.status)).then_with(|| a.id.cmp(&b.id)));
            if rows.is_empty() {
                println!("No tasks found.");
            } else {
                for t in rows {
                    println!("{}", fmt_row(t));
                }
            }
        }
        Command::Stats => {
            let tasks = store::load(&root, &NON_DEAD)?;
            let count = |s: Status| tasks.iter().filter(|t| t.status == s).count();
            println!(
                "Total: {}  Hairy: {}  Shaving: {}  Shorn: {}",
                tasks.len(),
                count(Status::Hairy),
                count(Status::Shaving),
                count(Status::Shorn),
            );
            let mut by_type: Vec<(String, usize)> = fold_counts(tasks.iter().map(|t| t.kind.clone()));
            if !by_type.is_empty() {
                by_type.sort();
                println!("By type:");
                for (k, v) in by_type {
                    println!("  {k}: {v}");
                }
            }
            let mut by_pri: Vec<(u8, usize)> = fold_counts(tasks.iter().map(|t| t.priority));
            if !by_pri.is_empty() {
                by_pri.sort();
                println!("By priority:");
                for (k, v) in by_pri {
                    println!("  p{k}: {v}");
                }
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
            let cfg = store::read_config(&root);
            if let Some(p) = &parent {
                if !store::all_ids(&root).contains(p) {
                    eprintln!("error: parent task {p} not found");
                    std::process::exit(1);
                }
            }
            let id = store::generate_id(&root, &cfg.prefix)?;
            let now = store::now_iso();
            let task = Task {
                id: id.clone(),
                title: title.clone(),
                kind: kind.unwrap_or(cfg.default_type),
                priority: priority.unwrap_or(cfg.default_priority),
                status: Status::Hairy,
                created: Some(now.clone()),
                updated: Some(now),
                parent,
                labels,
                depends_on,
                source,
                body: description.unwrap_or_default(),
            };
            store::write::save(&root, &task)?;
            println!("Created {id}: {title}");
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
            let Some(mut task) = store::load_task_by_id(&root, &id)? else {
                eprintln!("error: task {id} not found");
                std::process::exit(1);
            };
            let mut changed = false;
            if let Some(t) = title {
                task.title = t;
                changed = true;
            }
            if let Some(k) = kind {
                task.kind = k;
                changed = true;
            }
            if let Some(p) = priority {
                task.priority = p;
                changed = true;
            }
            if let Some(d) = description {
                task.body = d;
                changed = true;
            }
            if !add_label.is_empty() {
                for l in add_label {
                    if !task.labels.contains(&l) {
                        task.labels.push(l);
                    }
                }
                changed = true;
            }
            if !remove_label.is_empty() {
                task.labels.retain(|l| !remove_label.contains(l));
                changed = true;
            }
            if let Some(s) = source {
                if !s.is_empty() {
                    task.source = Some(s);
                    changed = true;
                }
            }
            if let Some(n) = note {
                let ts = store::now_iso();
                task.body = store::append_note(&task.body, &ts, &n);
                changed = true;
            }
            if changed {
                task.updated = Some(store::now_iso());
                store::write::save(&root, &task)?;
                println!("Updated {id}");
            } else {
                println!("No changes specified.");
            }
        }
        Command::Shave { id } => move_cmd(&root, &id, Status::Shaving, "already being shaved", "Shaving")?,
        Command::Shorn { id } => move_cmd(&root, &id, Status::Shorn, "already shorn", "Shorn!")?,
        Command::Regrow { id } => move_cmd(&root, &id, Status::Hairy, "already hairy", "Regrown:")?,
        Command::Slaughter { id } => move_cmd(&root, &id, Status::Dead, "already dead", "Slaughtered:")?,
        Command::Revive { id } => move_cmd(&root, &id, Status::Hairy, "already hairy", "Revived:")?,
        Command::Dep { action } => match action {
            DepAction::Add { id, dep_id } => match store::add_dep(&root, &id, &dep_id)? {
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
            DepAction::Remove { id, dep_id } => match store::remove_dep(&root, &id, &dep_id)? {
                DepOutcome::TaskNotFound => {
                    eprintln!("error: task {id} not found");
                    std::process::exit(1);
                }
                DepOutcome::NotDep => println!("{dep_id} is not a dependency of {id}"),
                DepOutcome::Removed => println!("Removed dependency: {id} -> {dep_id}"),
                _ => {}
            },
        },
        Command::Reparent { id, parent, unparent } => {
            let new_parent = if unparent {
                None
            } else if parent.is_some() {
                parent
            } else {
                eprintln!("error: specify --parent TASK_ID or --unparent");
                std::process::exit(1);
            };
            match store::reparent(&root, &id, new_parent)? {
                Reparent::Error(msg) => {
                    eprintln!("error: {msg}");
                    std::process::exit(1);
                }
                Reparent::Done { new_parent: None } => println!("Promoted {id} to top-level"),
                Reparent::Done { new_parent: Some(p) } => println!("Reparented {id} under {p}"),
            }
        }
    }
    Ok(())
}

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

fn status_rank(s: Status) -> u8 {
    match s {
        Status::Hairy => 0,
        Status::Shaving => 1,
        Status::Shorn => 2,
        Status::Dead => 3,
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

fn fold_counts<K: std::hash::Hash + Eq, I: Iterator<Item = K>>(it: I) -> Vec<(K, usize)> {
    let mut m: std::collections::HashMap<K, usize> = std::collections::HashMap::new();
    for k in it {
        *m.entry(k).or_insert(0) += 1;
    }
    m.into_iter().collect()
}

/// `  [X] id  pN type     title [labels] (deps: ...)` — matches Python _fmt_task_row.
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

/// `  id  pN type     title` — matches Python cmd_next row (no status glyph).
fn fmt_plain_row(t: &Task) -> String {
    format!("  {}  p{} {:8} {}", t.id, t.priority, t.kind, t.title)
}

fn move_cmd(root: &Path, id: &str, dest: Status, already: &str, done: &str) -> Result<()> {
    match store::move_task(root, id, dest)? {
        MoveOutcome::NotFound => {
            eprintln!("error: task {id} not found");
            std::process::exit(1);
        }
        MoveOutcome::AlreadyThere => println!("{id} is {already}"),
        MoveOutcome::Moved => println!("{done} {id}"),
    }
    Ok(())
}

fn print_task(t: &Task) {
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
    if let Some(s) = &t.source {
        println!("source:   {s}");
    }
    if !t.body.is_empty() {
        println!("\n{}", t.body);
    }
}
