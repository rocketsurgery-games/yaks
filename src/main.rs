//! yaks-rs — a filesystem-native task tracker (Rust port).
//!
//! Reads and writes the SAME `.yaks/` files as the Python `yaks` tool.
//! All read commands support `--json` (shapes are yaks-rs's own, pinned by
//! golden snapshot tests).

mod filter;
mod json;
mod model;
mod rollup;
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
    match store::schema_status(&root) {
        store::SchemaStatus::Newer(v) => {
            eprintln!(
                "error: this herd uses schema v{v}, newer than this yaks supports (v{}). Upgrade yaks.",
                store::SCHEMA
            );
            std::process::exit(1);
        }
        store::SchemaStatus::Older(v) => eprintln!(
            "warning: herd schema v{v} predates this yaks (v{}); reading best-effort. Run the Python yaks once to migrate.",
            store::SCHEMA
        ),
        store::SchemaStatus::Compatible => {}
    }

    match cli.command {
        Command::List { filter, all, json } => {
            let tasks = store::load(&root, &EVERY)?;
            let mut rows = filter::apply(&tasks, &build_spec(filter), all);
            rows.sort_by(|a, b| status_rank(a.status).cmp(&status_rank(b.status)).then_with(|| a.id.cmp(&b.id)));
            if json {
                json::print(&json::tasks_array(&rows))?;
            } else if rows.is_empty() {
                println!("No tasks found.");
            } else {
                for t in rows {
                    println!("{}", fmt_row(t));
                }
            }
        }
        Command::Show { id, json } => {
            let all = store::load(&root, &EVERY)?;
            let Some(t) = all.iter().find(|t| t.id == id) else {
                eprintln!("no such task: {id}");
                std::process::exit(1);
            };
            let mut children: Vec<&Task> = all.iter().filter(|c| c.parent.as_deref() == Some(id.as_str())).collect();
            children.sort_by(|a, b| a.id.cmp(&b.id));
            if json {
                json::print(&json::show_value(t, &children))?;
            } else {
                print_task(t);
                if !children.is_empty() {
                    println!("\nChildren:");
                    for c in &children {
                        println!("  [{}] {}  {}", c.status.glyph(), c.id, c.title);
                    }
                }
            }
        }
        Command::Next { filter, json } => {
            let mut spec = build_spec(filter);
            spec.statuses = vec![Status::Hairy];
            spec.ready_only = true;
            let tasks = store::load(&root, &EVERY)?;
            let rows = filter::apply(&tasks, &spec, false);
            if json {
                json::print(&json::tasks_array(&rows))?;
            } else if rows.is_empty() {
                println!("No yaks ready to shave.");
            } else {
                println!("Ready to shave (all dependencies met):");
                for t in rows {
                    println!("{}", fmt_plain_row(t));
                }
            }
        }
        Command::Tangled { filter, json } => {
            let mut spec = build_spec(filter);
            spec.statuses = vec![Status::Hairy];
            spec.tangled_only = true;
            let tasks = store::load(&root, &EVERY)?;
            let resolved = filter::resolved_ids(&tasks);
            let rows = filter::apply(&tasks, &spec, false);
            if json {
                let items: Vec<(&Task, Vec<&str>)> =
                    rows.iter().map(|t| (*t, filter::unresolved_deps(t, &resolved))).collect();
                json::print(&json::tangled_array(&items))?;
            } else if rows.is_empty() {
                println!("No tangled yaks.");
            } else {
                println!("Tangled yaks:");
                for t in rows {
                    let waiting = filter::unresolved_deps(t, &resolved).join(", ");
                    println!("  {}  {}  (waiting on: {})", t.id, t.title, waiting);
                }
            }
        }
        Command::Search { query, filter, json } => {
            let mut spec = build_spec(filter);
            spec.search = Some(query);
            let tasks = store::load(&root, &EVERY)?;
            let mut rows = filter::apply(&tasks, &spec, false);
            rows.sort_by(|a, b| status_rank(a.status).cmp(&status_rank(b.status)).then_with(|| a.id.cmp(&b.id)));
            if json {
                json::print(&json::tasks_array(&rows))?;
            } else if rows.is_empty() {
                println!("No tasks found.");
            } else {
                for t in rows {
                    println!("{}", fmt_row(t));
                }
            }
        }
        Command::Stats { json } => {
            let tasks = store::load(&root, &NON_DEAD)?;
            let count = |s: Status| tasks.iter().filter(|t| t.status == s).count();
            let (hairy, shaving, shorn) = (count(Status::Hairy), count(Status::Shaving), count(Status::Shorn));
            let mut by_type: Vec<(String, usize)> = fold_counts(tasks.iter().map(|t| t.kind.clone()));
            by_type.sort();
            let mut by_pri: Vec<(u8, usize)> = fold_counts(tasks.iter().map(|t| t.priority));
            by_pri.sort();
            if json {
                json::print(&json::stats_value(tasks.len(), hairy, shaving, shorn, &by_type, &by_pri))?;
            } else {
                println!("Total: {}  Hairy: {hairy}  Shaving: {shaving}  Shorn: {shorn}", tasks.len());
                if !by_type.is_empty() {
                    println!("By type:");
                    for (k, v) in &by_type {
                        println!("  {k}: {v}");
                    }
                }
                if !by_pri.is_empty() {
                    println!("By priority:");
                    for (k, v) in &by_pri {
                        println!("  p{k}: {v}");
                    }
                }
            }
        }
        Command::Create {
            title, kind, priority, parent, labels, depends_on, source, description,
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
            id, title, kind, priority, description, add_label, remove_label, source, note,
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
        Command::Rollup(args) => {
            let tasks = store::load(&root, &NON_DEAD)?;
            let (groups, unsourced) = rollup::build(&tasks, &build_spec(args.filter));
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
                        let mut row = fmt_row(y.task);
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

fn parse_status(s: &str) -> Option<Status> {
    match s.to_lowercase().as_str() {
        "hairy" => Some(Status::Hairy),
        "shaving" => Some(Status::Shaving),
        "shorn" => Some(Status::Shorn),
        "dead" => Some(Status::Dead),
        _ => None,
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

fn fold_counts<K: std::hash::Hash + Eq, I: Iterator<Item = K>>(it: I) -> Vec<(K, usize)> {
    let mut m: std::collections::HashMap<K, usize> = std::collections::HashMap::new();
    for k in it {
        *m.entry(k).or_insert(0) += 1;
    }
    m.into_iter().collect()
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
    format!("  [{}] {}  p{} {:8} {}{}{}", t.status.glyph(), t.id, t.priority, t.kind, t.title, labels, deps)
}

/// `  id  pN type     title` (no status glyph) — matches Python cmd_next.
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
