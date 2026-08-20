//! yaks-rs — a filesystem-native task tracker (Rust port).
//!
//! Reads and writes the SAME `.yaks/` files as the Python `yaks` tool.
//! Read-only commands so far: `list`, `show`, `next`. The write path
//! (create/update/status-moves) is being ported under Phase 1 (yaksrs-6e21).

mod model;
mod store;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::collections::HashMap;
use std::env;

use model::{Status, Task};

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
    Next,
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
