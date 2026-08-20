//! Filesystem discovery + frontmatter parsing for the Phase 0 spike.
//!
//! Reads the same on-disk layout as the Python tool: a `.yaks/` directory with
//! `hairy/ shaving/ shorn/ dead/` subdirs of `<id>.md` files, each with
//! `---`-delimited YAML frontmatter followed by a markdown body.
//!
//! The frontmatter parser here is a deliberately small hand-rolled fast path
//! (scalars + simple lists) — the approach yak-3fd4.3 validated in Python. A
//! serde / serde_yaml_ng fallback for exotic YAML is a later step.

use crate::model::{Status, Task};
use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

/// Walk up from `start` until a directory containing `.yaks/` is found.
pub fn discover_root(start: &Path) -> Result<PathBuf> {
    let mut dir = start;
    loop {
        let candidate = dir.join(".yaks");
        if candidate.is_dir() {
            return Ok(candidate);
        }
        match dir.parent() {
            Some(p) => dir = p,
            None => anyhow::bail!("no .yaks/ directory found at or above {}", start.display()),
        }
    }
}

/// Load every task file in the given statuses, sorted by id.
pub fn load(root: &Path, statuses: &[Status]) -> Result<Vec<Task>> {
    let mut tasks = Vec::new();
    for &status in statuses {
        let dir = root.join(status.dir());
        if !dir.is_dir() {
            continue;
        }
        for entry in fs::read_dir(&dir).with_context(|| format!("reading {}", dir.display()))? {
            let path = entry?.path();
            if path.extension().and_then(|e| e.to_str()) != Some("md") {
                continue;
            }
            let text =
                fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
            if let Some(task) = parse_task(&text, status) {
                tasks.push(task);
            }
        }
    }
    tasks.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(tasks)
}

/// Parse one task file. Returns `None` if it has no usable frontmatter/id.
fn parse_task(text: &str, status: Status) -> Option<Task> {
    let (front, body) = split_frontmatter(text)?;

    let mut id = String::new();
    let mut title = String::new();
    let mut kind = "task".to_string();
    let mut priority: u8 = 3;
    let mut parent = None;
    let mut source = None;
    let mut labels: Vec<String> = Vec::new();
    let mut depends_on: Vec<String> = Vec::new();

    // Which block-list key (if any) subsequent "- item" lines belong to.
    let mut pending: Option<&str> = None;
    for raw in front.lines() {
        let trimmed = raw.trim_start();
        if let Some(item) = trimmed.strip_prefix("- ") {
            match pending {
                Some("labels") => {
                    labels.push(unquote(item.trim()));
                    continue;
                }
                Some("depends_on") => {
                    depends_on.push(unquote(item.trim()));
                    continue;
                }
                _ => {}
            }
        }
        pending = None;

        let Some((key, value)) = raw.split_once(':') else {
            continue;
        };
        let key = key.trim();
        let value = value.trim();
        match key {
            "id" => id = unquote(value),
            "title" => title = unquote(value),
            "type" => kind = unquote(value),
            "priority" => priority = value.parse().unwrap_or(3),
            "parent" => parent = non_empty(unquote(value)),
            "source" => source = non_empty(unquote(value)),
            "labels" => match parse_inline_list(value) {
                Some(list) => labels = list,
                None => pending = Some("labels"),
            },
            "depends_on" => match parse_inline_list(value) {
                Some(list) => depends_on = list,
                None => pending = Some("depends_on"),
            },
            _ => {}
        }
    }

    if id.is_empty() {
        return None;
    }
    Some(Task {
        id,
        title,
        kind,
        priority,
        status,
        parent,
        labels,
        depends_on,
        source,
        body: body.trim().to_string(),
    })
}

/// Split `---\n<frontmatter>\n---\n<body>` into `(frontmatter, body)`.
fn split_frontmatter(text: &str) -> Option<(&str, &str)> {
    let rest = text
        .strip_prefix("---\n")
        .or_else(|| text.strip_prefix("---\r\n"))?;
    let end = rest.find("\n---")?;
    let front = &rest[..end];
    let after = &rest[end + 4..];
    let body = after.trim_start_matches(['\r', '\n']);
    Some((front, body))
}

/// `[a, b, c]` -> `Some(vec)`. Returns `None` for block-style (empty value).
fn parse_inline_list(value: &str) -> Option<Vec<String>> {
    let inner = value.strip_prefix('[')?.strip_suffix(']')?;
    Some(
        inner
            .split(',')
            .map(|s| unquote(s.trim()))
            .filter(|s| !s.is_empty())
            .collect(),
    )
}

/// Strip a single pair of matching surrounding quotes, if present.
fn unquote(s: &str) -> String {
    let s = s.trim();
    let b = s.as_bytes();
    if b.len() >= 2
        && ((b[0] == b'"' && b[b.len() - 1] == b'"') || (b[0] == b'\'' && b[b.len() - 1] == b'\''))
    {
        s[1..s.len() - 1].to_string()
    } else {
        s.to_string()
    }
}

fn non_empty(s: String) -> Option<String> {
    if s.is_empty() { None } else { Some(s) }
}
