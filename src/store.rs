//! Filesystem discovery, frontmatter parsing, and task serialization.
//!
//! Reads and writes the same on-disk layout as the Python tool: a `.yaks/`
//! directory with `hairy/ shaving/ shorn/ dead/` subdirs of `<id>.md` files,
//! each with `---`-delimited YAML frontmatter followed by a markdown body.
//!
//! The frontmatter parser is a deliberately small hand-rolled fast path
//! (scalars + simple lists) — the approach yak-3fd4.3 validated in Python. The
//! serializer (see the `write` module) emits the same tiny YAML subset the
//! Python `dump_yaml` produces, so the two tools round-trip each other.

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
    let mut created = None;
    let mut updated = None;
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
            "created" => created = non_empty(unquote(value)),
            "updated" => updated = non_empty(unquote(value)),
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
        created,
        updated,
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

/// Serialization + atomic persistence. Mirrors the Python `dump_yaml` /
/// `save_task` output so files round-trip between the two implementations.
///
/// Marked `allow(dead_code)` for now: exercised by the round-trip tests and
/// wired into the `create`/`update` commands next (yaksrs-a2a4).
#[allow(dead_code)]
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
            "y" | "Y" | "yes" | "Yes" | "YES" | "n" | "N" | "no" | "No" | "NO"
                | "true" | "True" | "TRUE" | "false" | "False" | "FALSE"
                | "on" | "On" | "ON" | "off" | "Off" | "OFF"
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
    fn timestamps_quoted_priority_plain_id_plain() {
        let text = write::render(&sample());
        assert!(text.contains("\ncreated: '2026-08-19T00:00:00Z'\n"), "{text}");
        assert!(text.contains("\npriority: 2\n"), "{text}");
        assert!(text.contains("\nid: yaksrs-6b8c\n"), "{text}");
        // Title has ": " so it must be single-quoted.
        assert!(text.contains("\ntitle: 'Task model: full fields"), "{text}");
        // Block list at column 0.
        assert!(text.contains("\ndepends_on:\n- yaksrs-aaaa\n- yaksrs-bbbb\n"), "{text}");
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
