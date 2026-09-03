//! Filesystem discovery, frontmatter parsing, and task serialization.
//!
//! The on-disk layout: a `.yaks/` directory with `hairy/ shaving/ shorn/ dead/`
//! subdirs of `<id>.md` files,
//! each with `---`-delimited YAML frontmatter followed by a markdown body.
//!
//! The frontmatter parser is a deliberately small hand-rolled fast path
//! (scalars + simple lists) — the approach yak-3fd4.3 validated in Python. The
//! serializer (see the `write` module) emits the same tiny YAML subset the
//! Python `dump_yaml` produces, so the two tools round-trip each other.

use crate::model::{Status, Task};
use anyhow::{Context, Result};
use chrono::{DateTime, Duration, NaiveDate, NaiveDateTime, TimeZone, Utc};
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
    let mut created = None;
    let mut updated = None;
    let mut parent = None;
    let mut source = None;
    let mut needs = None;
    let mut labels: Vec<String> = Vec::new();
    let mut depends_on: Vec<String> = Vec::new();
    // Frontmatter keys this binary does not model, kept verbatim to re-emit.
    let mut extra: Vec<String> = Vec::new();

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
                // Block-list items under an unknown key: keep verbatim.
                Some("extra") => {
                    extra.push(raw.to_string());
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
            "created" => created = non_empty(unquote(value)),
            "updated" => updated = non_empty(unquote(value)),
            "parent" => parent = non_empty(unquote(value)),
            "source" => source = non_empty(unquote(value)),
            "needs" => needs = non_empty(unquote(value)),
            "labels" => match parse_inline_list(value) {
                Some(list) => labels = list,
                None => pending = Some("labels"),
            },
            "depends_on" => match parse_inline_list(value) {
                Some(list) => depends_on = list,
                None => pending = Some("depends_on"),
            },
            // Unknown key: preserve the raw line. If it's block-style (empty
            // value), its following `- ` items belong to it too (pending=extra).
            _ => {
                extra.push(raw.to_string());
                if value.is_empty() {
                    pending = Some("extra");
                }
            }
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
        created,
        updated,
        parent,
        labels,
        depends_on,
        source,
        needs,
        extra,
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
        // YAML single-quote escaping: '' -> '
        if b[0] == b'\'' {
            return s[1..s.len() - 1].replace("''", "'");
        }
        return s[1..s.len() - 1].to_string();
    }
    s.to_string()
}

fn non_empty(s: String) -> Option<String> {
    if s.is_empty() { None } else { Some(s) }
}

/// Serialization + atomic persistence: frontmatter + markdown body written to
/// a temp file and renamed into place.
///
/// Marked `allow(dead_code)` for now: exercised by the round-trip tests and
/// wired into the `create`/`update` commands next (yaksrs-a2a4).
pub mod write {
    use super::*;

    /// Full file text for a task: frontmatter + body.
    pub fn render(t: &Task) -> String {
        let mut s = String::from("---\n");
        s.push_str(&frontmatter(t));
        s.push_str("---\n");
        let body = t.body.trim();
        if !body.is_empty() {
            s.push('\n');
            s.push_str(body);
            s.push('\n');
        }
        s
    }

    /// Write a task to `root/<status>/<id>.md` atomically (temp + rename).
    pub fn save(root: &Path, t: &Task) -> Result<PathBuf> {
        let dir = root.join(t.status.dir());
        fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
        let path = dir.join(format!("{}.md", t.id));
        atomic_write(&path, &render(t))?;
        Ok(path)
    }

    fn frontmatter(t: &Task) -> String {
        let mut out = String::new();
        out.push_str(&format!("id: {}\n", scalar(&t.id)));
        out.push_str(&format!("title: {}\n", scalar(&t.title)));
        out.push_str(&format!("type: {}\n", scalar(&t.kind)));
        out.push_str(&format!("priority: {}\n", t.priority));
        if let Some(c) = &t.created {
            out.push_str(&format!("created: {}\n", scalar(c)));
        }
        if let Some(u) = &t.updated {
            out.push_str(&format!("updated: {}\n", scalar(u)));
        }
        if let Some(p) = &t.parent {
            out.push_str(&format!("parent: {}\n", scalar(p)));
        }
        if !t.depends_on.is_empty() {
            out.push_str("depends_on:\n");
            for d in &t.depends_on {
                out.push_str(&format!("- {}\n", scalar(d)));
            }
        }
        if !t.labels.is_empty() {
            out.push_str("labels:\n");
            for l in &t.labels {
                out.push_str(&format!("- {}\n", scalar(l)));
            }
        }
        if let Some(s) = &t.source {
            out.push_str(&format!("source: {}\n", scalar(s)));
        }
        if let Some(n) = &t.needs {
            out.push_str(&format!("needs: {}\n", scalar(n)));
        }
        // Re-emit unmodeled frontmatter verbatim, after the known fields.
        for line in &t.extra {
            out.push_str(line);
            out.push('\n');
        }
        out
    }

    /// Emit a scalar plain, or single-quoted (with '' escaping) when a plain
    /// token would be misread by YAML's implicit typing or its syntax.
    fn scalar(s: &str) -> String {
        if needs_quotes(s) {
            format!("'{}'", s.replace('\'', "''"))
        } else {
            s.to_string()
        }
    }

    fn needs_quotes(s: &str) -> bool {
        if s.is_empty() || resolves_nonstring(s) {
            return true;
        }
        let first = s.chars().next().unwrap();
        let last = s.chars().last().unwrap();
        if first.is_whitespace() || last.is_whitespace() {
            return true;
        }
        if "!&*?|>%@`\"'#,[]{}:".contains(first) {
            return true;
        }
        if s.starts_with("- ") || s == "-" {
            return true;
        }
        s.contains(": ") || s.contains(" #") || s.contains('\n')
    }

    /// Would YAML 1.1 implicit resolution read this as a non-string?
    fn resolves_nonstring(s: &str) -> bool {
        matches!(s, "" | "~" | "null" | "Null" | "NULL")
            || is_bool(s)
            || is_int(s)
            || is_float(s)
            || is_timestamp(s)
    }

    fn is_bool(s: &str) -> bool {
        matches!(
            s,
            "y" | "Y"
                | "yes"
                | "Yes"
                | "YES"
                | "n"
                | "N"
                | "no"
                | "No"
                | "NO"
                | "true"
                | "True"
                | "TRUE"
                | "false"
                | "False"
                | "FALSE"
                | "on"
                | "On"
                | "ON"
                | "off"
                | "Off"
                | "OFF"
        )
    }

    fn is_int(s: &str) -> bool {
        let body = s.strip_prefix(['+', '-']).unwrap_or(s);
        !body.is_empty() && body.bytes().all(|b| b.is_ascii_digit())
    }

    fn is_float(s: &str) -> bool {
        s.bytes().any(|b| b == b'.' || b == b'e' || b == b'E') && s.parse::<f64>().is_ok()
    }

    /// `YYYY-MM-DD` optionally followed by `T`/space and time — the common
    /// YAML timestamp shapes (covers created/updated).
    fn is_timestamp(s: &str) -> bool {
        let b = s.as_bytes();
        b.len() >= 10
            && b[0..4].iter().all(u8::is_ascii_digit)
            && b[4] == b'-'
            && b[5..7].iter().all(u8::is_ascii_digit)
            && b[7] == b'-'
            && b[8..10].iter().all(u8::is_ascii_digit)
            && (b.len() == 10 || matches!(b[10], b'T' | b't' | b' '))
    }

    fn atomic_write(path: &Path, text: &str) -> Result<()> {
        let dir = path.parent().context("path has no parent directory")?;
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .context("path has no file name")?;
        let tmp = dir.join(format!(".{}.{}.tmp", name, std::process::id()));
        fs::write(&tmp, text).with_context(|| format!("writing {}", tmp.display()))?;
        fs::rename(&tmp, path).with_context(|| format!("renaming into {}", path.display()))?;
        Ok(())
    }
}

/// Config read from `.yaks/config.yaml` (only the keys we use).
#[derive(Clone)]
pub struct Config {
    pub prefix: String,
    pub default_type: String,
    pub default_priority: u8,
    /// When true, embedded editors use vim keybindings; otherwise emacs.
    pub vim_mode: bool,
}

/// Read `.yaks/config.yaml`; missing file/keys fall back to the built-in
/// defaults (prefix "yak", type "task", priority 3).
pub fn read_config(root: &Path) -> Config {
    let mut c = Config {
        prefix: "yak".to_string(),
        default_type: "task".to_string(),
        default_priority: 3,
        vim_mode: true,
    };
    if let Ok(text) = fs::read_to_string(root.join("config.yaml")) {
        for line in text.lines() {
            let Some((k, v)) = line.split_once(':') else {
                continue;
            };
            let v = unquote(v.trim());
            match k.trim() {
                "prefix" if !v.is_empty() => c.prefix = v,
                "default_type" if !v.is_empty() => c.default_type = v,
                "default_priority" => {
                    if let Ok(n) = v.parse() {
                        c.default_priority = n;
                    }
                }
                "vim_mode" => c.vim_mode = matches!(v.as_str(), "true" | "True" | "yes" | "1"),
                _ => {}
            }
        }
    }
    c
}

/// Rewrite only the `prefix:` line of `config.yaml` to `new`, leaving every
/// other line untouched (appending a `prefix:` line if none exists, and
/// creating the file if missing). Used by the prefix-rename migration so the
/// config and the on-disk ids agree afterwards.
pub fn set_config_prefix(root: &Path, new: &str) -> Result<()> {
    let path = root.join("config.yaml");
    let existing = fs::read_to_string(&path).unwrap_or_default();
    let mut lines: Vec<String> = Vec::new();
    let mut replaced = false;
    for line in existing.lines() {
        let is_prefix = line
            .split_once(':')
            .map(|(k, _)| k.trim() == "prefix")
            .unwrap_or(false);
        if is_prefix {
            lines.push(format!("prefix: {new}"));
            replaced = true;
        } else {
            lines.push(line.to_string());
        }
    }
    if !replaced {
        lines.push(format!("prefix: {new}"));
    }
    let mut out = lines.join("\n");
    out.push('\n');
    fs::write(&path, out).with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}

/// The values [`init`] seeds a new herd's `config.yaml` with. The `Default`
/// impl mirrors the built-in fallbacks in [`read_config`] (prefix "yak",
/// type "task", priority 3, vim keybindings).
pub struct InitConfig {
    pub prefix: String,
    pub default_type: String,
    pub default_priority: u8,
    pub vim_mode: bool,
}

impl Default for InitConfig {
    fn default() -> Self {
        InitConfig {
            prefix: "yak".to_string(),
            default_type: "task".to_string(),
            default_priority: 3,
            vim_mode: true,
        }
    }
}

/// Result of [`init`].
pub enum InitOutcome {
    /// A fresh herd was written at the `.yaks/` path.
    Created,
    /// The `.yaks/` directory already existed; nothing was written.
    AlreadyExists,
}

/// Create a fresh herd at `root` (the `.yaks/` directory itself): the four
/// status subdirectories, a `config.yaml` seeded from `cfg`, and the `schema`
/// marker for this build. Refuses to touch an existing `.yaks/` — the caller
/// gets [`InitOutcome::AlreadyExists`] and nothing is written.
pub fn init(root: &Path, cfg: &InitConfig) -> Result<InitOutcome> {
    if root.exists() {
        return Ok(InitOutcome::AlreadyExists);
    }
    for status in [Status::Hairy, Status::Shaving, Status::Shorn, Status::Dead] {
        let dir = root.join(status.dir());
        fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
    }
    let config = format!(
        "prefix: {}\ndefault_type: {}\ndefault_priority: {}\nvim_mode: {}\n",
        cfg.prefix, cfg.default_type, cfg.default_priority, cfg.vim_mode,
    );
    let config_path = root.join("config.yaml");
    fs::write(&config_path, config)
        .with_context(|| format!("writing {}", config_path.display()))?;
    let schema_path = root.join("schema");
    fs::write(&schema_path, format!("{SCHEMA}\n"))
        .with_context(|| format!("writing {}", schema_path.display()))?;
    Ok(InitOutcome::Created)
}

/// Load every task file
/// Every task id present on disk (all statuses, including dead).
pub fn all_ids(root: &Path) -> std::collections::HashSet<String> {
    let mut ids = std::collections::HashSet::new();
    for st in [Status::Hairy, Status::Shaving, Status::Shorn, Status::Dead] {
        let Ok(rd) = fs::read_dir(root.join(st.dir())) else {
            continue;
        };
        for entry in rd.flatten() {
            let p = entry.path();
            if p.extension().and_then(|e| e.to_str()) == Some("md") {
                if let Some(stem) = p.file_stem().and_then(|s| s.to_str()) {
                    ids.insert(stem.to_string());
                }
            }
        }
    }
    ids
}

/// Generate a collision-free `{prefix}-{4 lowercase hex}` id (Python-compatible).
pub fn generate_id(root: &Path, prefix: &str) -> Result<String> {
    let existing = all_ids(root);
    let mut state = rng_seed();
    for _ in 0..1000 {
        let n = xorshift(&mut state) & 0xffff;
        let id = format!("{prefix}-{n:04x}");
        if !existing.contains(&id) {
            return Ok(id);
        }
    }
    anyhow::bail!("could not generate a unique id after 1000 attempts")
}

fn rng_seed() -> u64 {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    let s = nanos ^ ((std::process::id() as u64) << 32) ^ 0x9e37_79b9_7f4a_7c15;
    if s == 0 { 0x1234_5678_9abc_def0 } else { s }
}

fn xorshift(s: &mut u64) -> u64 {
    let mut x = *s;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *s = x;
    x
}

/// Current UTC time as `YYYY-MM-DDTHH:MM:SSZ` (matches Python `now_iso`).
pub fn now_iso() -> String {
    Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

/// Result of a status move.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveOutcome {
    Moved,
    AlreadyThere,
    NotFound,
}

/// Locate a task's current file by id via exact-path probes (O(1) per status
/// dir).
pub fn find_task_file(root: &Path, id: &str) -> Option<(Status, PathBuf)> {
    for st in [Status::Hairy, Status::Shaving, Status::Shorn, Status::Dead] {
        let p = root.join(st.dir()).join(format!("{id}.md"));
        if p.is_file() {
            return Some((st, p));
        }
    }
    None
}

/// Move a task into `dest`: rename its file into the destination dir, then
/// rewrite it with a bumped `updated` (mirrors Python `move_task`). A no-op
/// when the task is already at `dest`.
pub fn move_task(root: &Path, id: &str, dest: Status) -> Result<MoveOutcome> {
    let Some((status, path)) = find_task_file(root, id) else {
        return Ok(MoveOutcome::NotFound);
    };
    if status == dest {
        return Ok(MoveOutcome::AlreadyThere);
    }
    let text = fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let mut task = parse_task(&text, status)
        .with_context(|| format!("parsing {} before move", path.display()))?;
    let dest_dir = root.join(dest.dir());
    fs::create_dir_all(&dest_dir).with_context(|| format!("creating {}", dest_dir.display()))?;
    let dest_path = dest_dir.join(format!("{id}.md"));
    fs::rename(&path, &dest_path)
        .with_context(|| format!("moving {} -> {}", path.display(), dest_path.display()))?;
    task.status = dest;
    task.updated = Some(now_iso());
    write::save(root, &task)?;
    Ok(MoveOutcome::Moved)
}

/// Load a single task by id (whatever status dir it is in).
pub fn load_task_by_id(root: &Path, id: &str) -> Result<Option<Task>> {
    let Some((status, path)) = find_task_file(root, id) else {
        return Ok(None);
    };
    let text = fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    Ok(parse_task(&text, status))
}

/// Append a timestamped note block to a body
/// (`<body>\n\n---\n\u{25b8} <ts>\n<note>`; no leading blank line when empty).
/// An optional actor is stamped onto the marker line as `\u{25b8} <ts> [<actor>]`;
/// an absent or empty actor writes the bare `\u{25b8} <ts>` form, so this is
/// byte-for-byte compatible with pre-actor notes and needs no migration.
pub fn append_note(body: &str, ts: &str, actor: Option<&str>, note: &str) -> String {
    let desc = body.trim_end();
    let sep = if desc.is_empty() { "" } else { "\n\n" };
    format!("{desc}{sep}---\n\u{25b8} {}\n{note}", note_head(ts, actor))
}

/// Format a note marker line's payload: `<ts>` or `<ts> [<actor>]`. The single
/// source of truth for the marker shape, shared by the log writer and the TUI's
/// comment (re)assembly so the two can't drift.
pub fn note_head(ts: &str, actor: Option<&str>) -> String {
    match actor {
        Some(a) if !a.trim().is_empty() => format!("{ts} [{}]", a.trim()),
        _ => ts.to_string(),
    }
}

/// A single timestamped note parsed back out of a task body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteEntry {
    pub ts: String,
    /// The actor that wrote the note, if the marker line carried a `[actor]`
    /// suffix. `None` for bare (pre-actor) notes.
    pub actor: Option<String>,
    pub text: String,
}

/// Split a note marker line's payload into its timestamp and optional trailing
/// `[actor]`. Timestamps never contain `" ["`, so a line ending in `]` with an
/// earlier `" ["` is unambiguously `<ts> [<actor>]`; anything else is a bare ts.
/// Public so the TUI's comment parser reuses the exact same split.
pub fn split_note_head(head: &str) -> (String, Option<String>) {
    if head.ends_with(']') {
        if let Some(idx) = head.rfind(" [") {
            let ts = head[..idx].trim().to_string();
            let actor = head[idx + 2..head.len() - 1].to_string();
            return (ts, Some(actor));
        }
    }
    (head.to_string(), None)
}

/// Parse the timestamped note blocks written by `append_note` out of a body. A
/// block begins at a line `---` immediately followed by a line starting with the
/// note marker, and its text runs to the next such delimiter or the end. Plain
/// description prose is skipped, so this is safe to run over a whole body.
pub fn parse_notes(body: &str) -> Vec<NoteEntry> {
    const MARK: &str = "\u{25b8} ";
    let lines: Vec<&str> = body.lines().collect();
    let is_delim = |i: usize| {
        lines[i].trim_end() == "---" && lines.get(i + 1).is_some_and(|l| l.starts_with(MARK))
    };
    let mut out = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        if is_delim(i) {
            let (ts, actor) = split_note_head(lines[i + 1][MARK.len()..].trim());
            let mut j = i + 2;
            while j < lines.len() && !is_delim(j) {
                j += 1;
            }
            let text = lines[i + 2..j].join("\n").trim().to_string();
            out.push(NoteEntry { ts, actor, text });
            i = j;
        } else {
            i += 1;
        }
    }
    out
}

/// Parse a canonical timestamp (as written by `now_iso`) to a UTC instant.
pub fn parse_ts(s: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|d| d.with_timezone(&Utc))
}

/// Resolve a `--since` spec to an absolute UTC instant, relative to `now`.
/// Accepts a relative duration (an integer then `s`/`m`/`h`/`d`/`w`, e.g. `2h`,
/// `3d`, `1w`), an `YYYY-MM-DD` date (midnight UTC), a naive
/// `YYYY-MM-DDTHH:MM:SS` datetime (UTC), or a full RFC3339 timestamp.
pub fn parse_since(spec: &str, now: DateTime<Utc>) -> Result<DateTime<Utc>> {
    let s = spec.trim();
    if let Some(dt) = parse_relative(s, now) {
        return Ok(dt);
    }
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Ok(dt.with_timezone(&Utc));
    }
    if let Ok(dt) = NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S") {
        return Ok(Utc.from_utc_datetime(&dt));
    }
    if let Ok(d) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        return Ok(Utc.from_utc_datetime(&d.and_hms_opt(0, 0, 0).unwrap()));
    }
    anyhow::bail!(
        "could not parse --since '{spec}'; use a duration like 2h/3d/1w, a date (YYYY-MM-DD), or an RFC3339 timestamp"
    )
}

/// A relative duration spec (`<int><unit>`), resolved against `now`. Returns
/// `None` when `spec` is not a number-then-unit token so callers fall back to
/// absolute parsing.
fn parse_relative(spec: &str, now: DateTime<Utc>) -> Option<DateTime<Utc>> {
    let idx = spec.find(|c: char| c.is_ascii_alphabetic())?;
    if idx == 0 {
        return None;
    }
    let (num, unit) = spec.split_at(idx);
    let n: i64 = num.trim().parse().ok()?;
    let dur = match unit {
        "s" => Duration::seconds(n),
        "m" => Duration::minutes(n),
        "h" => Duration::hours(n),
        "d" => Duration::days(n),
        "w" => Duration::weeks(n),
        _ => return None,
    };
    Some(now - dur)
}

#[cfg(test)]
mod log_parse_tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn parse_notes_splits_timestamped_blocks() {
        let body = append_note("Description.", "2026-01-01T00:00:00Z", None, "first note");
        let body = append_note(&body, "2026-01-02T00:00:00Z", None, "second\nspans lines");
        let notes = parse_notes(&body);
        assert_eq!(notes.len(), 2);
        assert_eq!(notes[0].ts, "2026-01-01T00:00:00Z");
        assert_eq!(notes[0].text, "first note");
        assert_eq!(notes[1].ts, "2026-01-02T00:00:00Z");
        assert_eq!(notes[1].text, "second\nspans lines");
        // Bare notes (no `--as`) carry no actor.
        assert_eq!(notes[0].actor, None);
        assert_eq!(notes[1].actor, None);
    }

    #[test]
    fn append_note_round_trips_actor_and_stays_back_compatible() {
        // An actor is stamped as `[actor]` and parses back out...
        let with = append_note("", "2026-01-01T00:00:00Z", Some("opus@joel"), "did a thing");
        assert!(with.contains("\u{25b8} 2026-01-01T00:00:00Z [opus@joel]"));
        // ...alongside a bare note in the same body, which stays actor-less.
        let body = append_note(&with, "2026-01-02T00:00:00Z", None, "bare");
        let notes = parse_notes(&body);
        assert_eq!(notes[0].actor.as_deref(), Some("opus@joel"));
        assert_eq!(notes[0].ts, "2026-01-01T00:00:00Z");
        assert_eq!(notes[0].text, "did a thing");
        assert_eq!(notes[1].actor, None);
        assert_eq!(notes[1].ts, "2026-01-02T00:00:00Z");
        // An empty/whitespace actor writes the bare form (no empty `[]`).
        let blank = append_note("", "2026-01-03T00:00:00Z", Some("  "), "x");
        assert_eq!(parse_notes(&blank)[0].actor, None);
    }

    #[test]
    fn parse_notes_ignores_prose_without_the_marker() {
        let body = "Just a description.\n\nWith a --- rule but no marker line.";
        assert!(parse_notes(body).is_empty());
    }

    #[test]
    fn parse_since_relative_and_absolute() {
        let now = Utc.with_ymd_and_hms(2026, 1, 10, 12, 0, 0).unwrap();
        assert_eq!(
            parse_since("2h", now).unwrap(),
            Utc.with_ymd_and_hms(2026, 1, 10, 10, 0, 0).unwrap()
        );
        assert_eq!(
            parse_since("3d", now).unwrap(),
            Utc.with_ymd_and_hms(2026, 1, 7, 12, 0, 0).unwrap()
        );
        assert_eq!(
            parse_since("1w", now).unwrap(),
            Utc.with_ymd_and_hms(2026, 1, 3, 12, 0, 0).unwrap()
        );
        assert_eq!(
            parse_since("2026-01-05", now).unwrap(),
            Utc.with_ymd_and_hms(2026, 1, 5, 0, 0, 0).unwrap()
        );
        assert_eq!(
            parse_since("2026-01-05T06:30:00Z", now).unwrap(),
            Utc.with_ymd_and_hms(2026, 1, 5, 6, 30, 0).unwrap()
        );
        assert!(parse_since("banana", now).is_err());
    }
}

/// Outcome of a dependency edit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DepOutcome {
    Added,
    AlreadyDep,
    Removed,
    NotDep,
    TaskNotFound,
    DepNotFound,
}

/// Add `dep` to `id`'s depends_on (dep must exist; no-op if already present).
pub fn add_dep(root: &Path, id: &str, dep: &str) -> Result<DepOutcome> {
    let Some(mut task) = load_task_by_id(root, id)? else {
        return Ok(DepOutcome::TaskNotFound);
    };
    if find_task_file(root, dep).is_none() {
        return Ok(DepOutcome::DepNotFound);
    }
    if task.depends_on.iter().any(|d| d == dep) {
        return Ok(DepOutcome::AlreadyDep);
    }
    task.depends_on.push(dep.to_string());
    task.updated = Some(now_iso());
    write::save(root, &task)?;
    Ok(DepOutcome::Added)
}

/// Remove `dep` from `id`'s depends_on (no existence check on dep, per Python).
pub fn remove_dep(root: &Path, id: &str, dep: &str) -> Result<DepOutcome> {
    let Some(mut task) = load_task_by_id(root, id)? else {
        return Ok(DepOutcome::TaskNotFound);
    };
    if !task.depends_on.iter().any(|d| d == dep) {
        return Ok(DepOutcome::NotDep);
    }
    task.depends_on.retain(|d| d != dep);
    task.updated = Some(now_iso());
    write::save(root, &task)?;
    Ok(DepOutcome::Removed)
}

/// Outcome of a reparent attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reparent {
    Done { new_parent: Option<String> },
    Error(String),
}

/// Repoint `id` under `new_parent` (None promotes to top-level). Mirrors
/// yaklib.reparent: guards self/missing/descendant/no-op cases.
pub fn reparent(root: &Path, id: &str, new_parent: Option<String>) -> Result<Reparent> {
    let all = load(
        root,
        &[Status::Hairy, Status::Shaving, Status::Shorn, Status::Dead],
    )?;
    let Some(task) = all.iter().find(|t| t.id == id) else {
        return Ok(Reparent::Error(format!("task {id} not found")));
    };
    let old_parent = task.parent.clone();
    if let Some(np) = &new_parent {
        if np == id {
            return Ok(Reparent::Error(
                "cannot reparent a task under itself".into(),
            ));
        }
        if !all.iter().any(|t| &t.id == np) {
            return Ok(Reparent::Error(format!("parent task {np} not found")));
        }
        if crate::filter::descendant_ids(&all, id, true).contains(np.as_str()) {
            return Ok(Reparent::Error(
                "cannot reparent under own descendant".into(),
            ));
        }
        if old_parent.as_deref() == Some(np.as_str()) {
            return Ok(Reparent::Error(format!("{id} is already a child of {np}")));
        }
    } else if old_parent.is_none() {
        return Ok(Reparent::Error(format!("{id} is already a top-level task")));
    }
    let mut t = task.clone();
    t.parent = new_parent.clone();
    t.updated = Some(now_iso());
    write::save(root, &t)?;
    Ok(Reparent::Done { new_parent })
}

/// On-disk schema version this build understands.
pub const SCHEMA: u32 = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchemaStatus {
    Compatible,
    Older(u32),
    Newer(u32),
}

/// Compare the herd's `.yaks/schema` marker against `SCHEMA`. A missing or
/// unparseable marker is treated as compatible (don't block hand-made herds).
pub fn schema_status(root: &Path) -> SchemaStatus {
    let Ok(raw) = fs::read_to_string(root.join("schema")) else {
        return SchemaStatus::Compatible;
    };
    let Ok(v) = raw.trim().parse::<u32>() else {
        return SchemaStatus::Compatible;
    };
    if v > SCHEMA {
        SchemaStatus::Newer(v)
    } else if v < SCHEMA {
        SchemaStatus::Older(v)
    } else {
        SchemaStatus::Compatible
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Status;

    fn sample() -> Task {
        Task {
            id: "yaksrs-6b8c".into(),
            title: "Task model: full fields + serialize (round-trip)".into(),
            kind: "task".into(),
            priority: 2,
            status: Status::Shaving,
            created: Some("2026-08-19T00:00:00Z".into()),
            updated: Some("2026-08-19T01:00:00Z".into()),
            parent: Some("yaksrs-6e21".into()),
            labels: vec!["rust".into()],
            depends_on: vec!["yaksrs-aaaa".into(), "yaksrs-bbbb".into()],
            source: None,
            needs: None,
            extra: Vec::new(),
            body: "First line.\n\n---\n\u{25b8} 2026-08-19T02:00:00Z\nA note with an apostrophe: don't panic.".into(),
        }
    }

    #[test]
    fn round_trip_preserves_all_fields() {
        let t = sample();
        let text = write::render(&t);
        let parsed = parse_task(&text, Status::Shaving).expect("should parse");
        assert_eq!(parsed, t);
    }

    #[test]
    fn unknown_frontmatter_survives_a_round_trip() {
        // A herd written by a newer/other tool: fields this binary doesn't model,
        // both scalar and block-style, plus a real note. None may be dropped.
        let text = "---\n\
            id: yaksrs-9f0a\n\
            title: forward compat\n\
            type: task\n\
            priority: 3\n\
            assignee: alice\n\
            reviewers:\n\
            - bob\n\
            - carol\n\
            ---\n\
            Body.\n";
        let parsed = parse_task(text, Status::Hairy).expect("should parse");
        // Unknown keys are captured verbatim, not lost to the `_ =>` arm.
        assert_eq!(
            parsed.extra,
            vec!["assignee: alice", "reviewers:", "- bob", "- carol"]
        );
        // ...and a re-render keeps them (after the known fields) and re-parses equal.
        let rendered = write::render(&parsed);
        assert!(rendered.contains("assignee: alice"), "{rendered}");
        assert!(
            rendered.contains("reviewers:\n- bob\n- carol\n"),
            "{rendered}"
        );
        assert_eq!(parse_task(&rendered, Status::Hairy).unwrap(), parsed);
    }

    #[test]
    fn timestamps_quoted_priority_plain_id_plain() {
        let text = write::render(&sample());
        assert!(
            text.contains("\ncreated: '2026-08-19T00:00:00Z'\n"),
            "{text}"
        );
        assert!(text.contains("\npriority: 2\n"), "{text}");
        assert!(text.contains("\nid: yaksrs-6b8c\n"), "{text}");
        // Title has ": " so it must be single-quoted.
        assert!(text.contains("\ntitle: 'Task model: full fields"), "{text}");
        // Block list at column 0.
        assert!(
            text.contains("\ndepends_on:\n- yaksrs-aaaa\n- yaksrs-bbbb\n"),
            "{text}"
        );
    }

    #[test]
    fn quoting_escapes_apostrophes_and_round_trips() {
        // ": " forces single-quoting; the inner apostrophe must be doubled ('').
        let mut t = sample();
        t.title = "it's: a tricky one".into();
        let text = write::render(&t);
        assert!(text.contains("title: 'it''s: a tricky one'"), "{text}");
        let parsed = parse_task(&text, Status::Shaving).unwrap();
        assert_eq!(parsed.title, "it's: a tricky one");
    }

    #[test]
    fn plain_when_safe() {
        // Punctuation that does NOT require quoting stays plain and round-trips.
        let mut t = sample();
        t.title = "don't \"stop\" now".into();
        let text = write::render(&t);
        assert!(text.contains("title: don't \"stop\" now\n"), "{text}");
        let parsed = parse_task(&text, Status::Shaving).unwrap();
        assert_eq!(parsed.title, "don't \"stop\" now");
    }
}

#[cfg(test)]
mod move_tests {
    use super::*;
    use crate::model::Status;
    use std::sync::atomic::{AtomicU64, Ordering};

    static SEQ: AtomicU64 = AtomicU64::new(0);

    fn temp_root() -> PathBuf {
        let mut p = std::env::temp_dir();
        let n = SEQ.fetch_add(1, Ordering::Relaxed);
        p.push(format!("yaksrs-test-{}-{}", std::process::id(), n));
        for st in [Status::Hairy, Status::Shaving, Status::Shorn, Status::Dead] {
            fs::create_dir_all(p.join(st.dir())).unwrap();
        }
        p
    }

    fn task(id: &str, status: Status) -> Task {
        Task {
            id: id.into(),
            title: "move me".into(),
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

    #[test]
    fn move_relocates_file_and_bumps_updated() {
        let root = temp_root();
        write::save(&root, &task("yaksrs-mv01", Status::Hairy)).unwrap();
        assert!(root.join("hairy/yaksrs-mv01.md").is_file());

        assert_eq!(
            move_task(&root, "yaksrs-mv01", Status::Shaving).unwrap(),
            MoveOutcome::Moved
        );
        assert!(!root.join("hairy/yaksrs-mv01.md").exists());
        assert!(root.join("shaving/yaksrs-mv01.md").is_file());

        let text = fs::read_to_string(root.join("shaving/yaksrs-mv01.md")).unwrap();
        let moved = parse_task(&text, Status::Shaving).unwrap();
        assert_ne!(moved.updated.as_deref(), Some("2026-01-01T00:00:00Z"));

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn move_is_noop_when_already_there_and_reports_missing() {
        let root = temp_root();
        write::save(&root, &task("yaksrs-mv02", Status::Shorn)).unwrap();
        assert_eq!(
            move_task(&root, "yaksrs-mv02", Status::Shorn).unwrap(),
            MoveOutcome::AlreadyThere
        );
        assert_eq!(
            move_task(&root, "does-not-exist", Status::Hairy).unwrap(),
            MoveOutcome::NotFound
        );
        let _ = fs::remove_dir_all(&root);
    }
}

#[cfg(test)]
mod note_tests {
    use super::append_note;

    #[test]
    fn append_note_on_empty_body_has_no_leading_blank() {
        assert_eq!(
            append_note("", "2026-01-01T00:00:00Z", None, "hi"),
            "---\n\u{25b8} 2026-01-01T00:00:00Z\nhi"
        );
    }

    #[test]
    fn append_note_separates_from_existing_body() {
        let out = append_note("Existing.\n", "2026-01-01T00:00:00Z", None, "second");
        assert_eq!(
            out,
            "Existing.\n\n---\n\u{25b8} 2026-01-01T00:00:00Z\nsecond"
        );
    }
}

#[cfg(test)]
mod graph_tests {
    use super::*;
    use crate::model::Status;
    use std::sync::atomic::{AtomicU64, Ordering};

    static SEQ: AtomicU64 = AtomicU64::new(0);

    fn temp_root() -> PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!(
            "yaksrs-graph-{}-{}",
            std::process::id(),
            SEQ.fetch_add(1, Ordering::Relaxed)
        ));
        for st in [Status::Hairy, Status::Shaving, Status::Shorn, Status::Dead] {
            fs::create_dir_all(p.join(st.dir())).unwrap();
        }
        p
    }

    fn mk(root: &Path, id: &str, parent: Option<&str>) {
        let t = Task {
            id: id.into(),
            title: id.into(),
            kind: "task".into(),
            priority: 3,
            status: Status::Hairy,
            created: None,
            updated: None,
            parent: parent.map(String::from),
            labels: vec![],
            depends_on: vec![],
            source: None,
            needs: None,
            extra: Vec::new(),
            body: String::new(),
        };
        write::save(root, &t).unwrap();
    }

    #[test]
    fn dep_add_remove_lifecycle() {
        let root = temp_root();
        mk(&root, "yaksrs-a", None);
        mk(&root, "yaksrs-b", None);
        assert_eq!(
            add_dep(&root, "yaksrs-a", "yaksrs-b").unwrap(),
            DepOutcome::Added
        );
        assert_eq!(
            add_dep(&root, "yaksrs-a", "yaksrs-b").unwrap(),
            DepOutcome::AlreadyDep
        );
        assert_eq!(
            add_dep(&root, "yaksrs-a", "ghost").unwrap(),
            DepOutcome::DepNotFound
        );
        assert_eq!(
            add_dep(&root, "ghost", "yaksrs-b").unwrap(),
            DepOutcome::TaskNotFound
        );
        assert_eq!(
            load_task_by_id(&root, "yaksrs-a")
                .unwrap()
                .unwrap()
                .depends_on,
            vec!["yaksrs-b".to_string()]
        );
        assert_eq!(
            remove_dep(&root, "yaksrs-a", "yaksrs-b").unwrap(),
            DepOutcome::Removed
        );
        assert_eq!(
            remove_dep(&root, "yaksrs-a", "yaksrs-b").unwrap(),
            DepOutcome::NotDep
        );
        assert!(
            load_task_by_id(&root, "yaksrs-a")
                .unwrap()
                .unwrap()
                .depends_on
                .is_empty()
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn reparent_rules() {
        let root = temp_root();
        mk(&root, "yaksrs-p", None);
        mk(&root, "yaksrs-c", None);
        mk(&root, "yaksrs-g", Some("yaksrs-c")); // g is a child of c
        assert!(matches!(
            reparent(&root, "yaksrs-c", Some("yaksrs-c".into())).unwrap(),
            Reparent::Error(_)
        ));
        assert!(matches!(
            reparent(&root, "yaksrs-c", Some("ghost".into())).unwrap(),
            Reparent::Error(_)
        ));
        assert!(matches!(
            reparent(&root, "yaksrs-c", Some("yaksrs-g".into())).unwrap(),
            Reparent::Error(_)
        ));
        assert_eq!(
            reparent(&root, "yaksrs-c", Some("yaksrs-p".into())).unwrap(),
            Reparent::Done {
                new_parent: Some("yaksrs-p".into())
            }
        );
        assert_eq!(
            load_task_by_id(&root, "yaksrs-c")
                .unwrap()
                .unwrap()
                .parent
                .as_deref(),
            Some("yaksrs-p")
        );
        assert!(matches!(
            reparent(&root, "yaksrs-c", Some("yaksrs-p".into())).unwrap(),
            Reparent::Error(_)
        ));
        assert_eq!(
            reparent(&root, "yaksrs-c", None).unwrap(),
            Reparent::Done { new_parent: None }
        );
        assert!(matches!(
            reparent(&root, "yaksrs-c", None).unwrap(),
            Reparent::Error(_)
        ));
        let _ = fs::remove_dir_all(&root);
    }
}

#[cfg(test)]
mod schema_tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(0);

    fn temp_root(schema: Option<&str>) -> PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!(
            "yaksrs-schema-{}-{}",
            std::process::id(),
            SEQ.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&p).unwrap();
        if let Some(v) = schema {
            fs::write(p.join("schema"), v).unwrap();
        }
        p
    }

    #[test]
    fn schema_gate() {
        assert_eq!(
            schema_status(&temp_root(Some("3"))),
            SchemaStatus::Compatible
        );
        assert_eq!(
            schema_status(&temp_root(Some("4\n"))),
            SchemaStatus::Newer(4)
        );
        assert_eq!(schema_status(&temp_root(Some("2"))), SchemaStatus::Older(2));
        assert_eq!(schema_status(&temp_root(None)), SchemaStatus::Compatible);
        assert_eq!(
            schema_status(&temp_root(Some("garbage"))),
            SchemaStatus::Compatible
        );
    }
}

#[cfg(test)]
mod init_tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static SEQ: AtomicU64 = AtomicU64::new(0);

    fn temp_dir() -> PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!(
            "yaksrs-init-{}-{}",
            std::process::id(),
            SEQ.fetch_add(1, Ordering::Relaxed)
        ));
        p
    }

    #[test]
    fn init_creates_dirs_config_and_schema() {
        let base = temp_dir();
        let root = base.join(".yaks");
        let cfg = InitConfig {
            prefix: "acme".into(),
            default_type: "chore".into(),
            default_priority: 1,
            vim_mode: false,
        };
        assert!(matches!(init(&root, &cfg).unwrap(), InitOutcome::Created));

        for sub in ["hairy", "shaving", "shorn", "dead"] {
            assert!(root.join(sub).is_dir(), "missing {sub}/");
        }
        assert_eq!(fs::read_to_string(root.join("schema")).unwrap(), "3\n");

        // The seeded config round-trips through the reader.
        let read = read_config(&root);
        assert_eq!(read.prefix, "acme");
        assert_eq!(read.default_type, "chore");
        assert_eq!(read.default_priority, 1);
        assert!(!read.vim_mode);

        // A fresh herd is discoverable and empty.
        assert_eq!(discover_root(&base).unwrap(), root);
        assert!(load(&root, &[Status::Hairy]).unwrap().is_empty());

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn init_refuses_to_clobber_existing_herd() {
        let base = temp_dir();
        let root = base.join(".yaks");
        assert!(matches!(
            init(&root, &InitConfig::default()).unwrap(),
            InitOutcome::Created
        ));
        // Drop a marker file, then re-init: it must be left untouched.
        fs::write(root.join("hairy/keep.md"), "x").unwrap();
        assert!(matches!(
            init(&root, &InitConfig::default()).unwrap(),
            InitOutcome::AlreadyExists
        ));
        assert_eq!(fs::read_to_string(root.join("hairy/keep.md")).unwrap(), "x");

        let _ = fs::remove_dir_all(&base);
    }
}
