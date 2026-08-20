//! yaks-rs — a filesystem-native task tracker (Rust port).
//!
//! Reads and writes the SAME `.yaks/` files as the Python `yaks` tool.
//! Commands: list, show, next, create, and the status moves
//! (shave/shorn/regrow/slaughter/revive). More of Phase 1 (update, deps,
//! filtering) is in progress under yaksrs-6e21.

mod model;
mod store;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::collections::HashMap;
use std::env;
use std::path::Path;

use model::{Status, Task};
use store::MoveOutcome;

#[derive(Parser)]
#[command(name = "yaks", version, about = "Filesystem-native task tracker (Rust)")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// List tasks (non-dead by default; --all also includes dead).
    List {
        #[arg(long)]
        all: bool,
    },
    /// Show one task by id.
    Show { id: String },
    /// List hairy tasks whose dependencies are all resolved.
    #[command(visible_alias = "ready")]
    Next,
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
}

// Every non-dead status, matching the Python tool's default `list` view.
const NON_DEAD: [Status; 3] = [Status::Hairy, Status::Shaving, Status::Shorn];
const EVERY: [Status; 4] = [Status::Hairy, Status::Shaving, Status::Shorn, Status::Dead];

fn main() -> Result<()> {
    let cli = Cli::parse();
    let cwd = env::current_dir()?;
    let root = store::discover_root(&cwd)?;

    match cli.command {
        Command::List { all } => {
            let statuses: &[Status] = if all { &EVERY } else { &NON_DEAD };
            let tasks = store::load(&root, statuses)?;
            for t in &tasks {
                println!("{}", t.summary());
            }
            eprintln!("\n{} task(s)", tasks.len());
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
        Command::Next => {
            let all = store::load(&root, &EVERY)?;
            let status_by_id: HashMap<&str, Status> =
                all.iter().map(|t| (t.id.as_str(), t.status)).collect();
            let ready: Vec<&Task> = all
                .iter()
                .filter(|t| t.status == Status::Hairy)
                .filter(|t| {
                    t.depends_on
                        .iter()
                        .all(|d| status_by_id.get(d.as_str()).is_none_or(|s| s.is_resolved()))
                })
                .collect();
            for t in &ready {
                println!("{}", t.summary());
            }
            eprintln!("\n{} ready", ready.len());
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
        Command::Shave { id } => move_cmd(&root, &id, Status::Shaving, "already being shaved", "Shaving")?,
        Command::Shorn { id } => move_cmd(&root, &id, Status::Shorn, "already shorn", "Shorn!")?,
        Command::Regrow { id } => move_cmd(&root, &id, Status::Hairy, "already hairy", "Regrown:")?,
        Command::Slaughter { id } => move_cmd(&root, &id, Status::Dead, "already dead", "Slaughtered:")?,
        Command::Revive { id } => move_cmd(&root, &id, Status::Hairy, "already hairy", "Revived:")?,
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
    }
    Ok(())
}

/// Perform a status move and render Python-compatible messages.
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
