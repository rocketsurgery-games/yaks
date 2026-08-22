//! Structured, navigable detail-pane model. Ported from Python `yaktui.detail`
//! + `yaklib.links`. `build` turns a task into display lines where the parent,
//! dependencies, children, source, and any task-id references in the body carry
//! link targets; `jumplist` flattens those targets in reading order for the
//! Tab/Enter jump navigation. No regex (hand-rolled id/URL scan).

use std::collections::{HashMap, HashSet};

use crate::model::{Status, Task};

/// Where a link points.
#[derive(Clone, PartialEq, Debug)]
pub enum Target {
    Task(String),
    Url(String),
    /// A markdown image link `![alt](path)` — an attached artifact, opened
    /// externally (path is relative to the herd's `.yaks/` root).
    Artifact(String),
}

/// Styling hint for a detail line.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Kind {
    Field,
    Section,
    Body,
    Empty,
}

/// One display line plus any link spans it carries (col + len in chars).
pub struct DLine {
    pub text: String,
    pub kind: Kind,
    pub links: Vec<(usize, usize, Target)>,
}

/// A single navigable target, located at a line + starting column.
pub struct Jump {
    pub line: usize,
    pub col: usize,
    pub target: Target,
}

fn is_url(s: &str) -> bool {
    s.starts_with("http://") || s.starts_with("https://")
}

/// A char is part of a bare-id/URL token run.
fn is_tok(c: char) -> bool {
    c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '.'
}

/// Rewrite `[[yak-abcd]]` wiki-links to bare `yak-abcd` for uniform handling.
fn strip_brackets(s: &str) -> String {
    s.replace("[[", "").replace("]]", "")
}

/// Scan a body line for known task ids and URLs, returning (col, len, target).
/// Parse a markdown image link `![alt](path)` starting at `start` (`chars[start]`
/// is `!`). Returns `(total_len_in_chars, path)` when well-formed.
fn parse_image_link(chars: &[char], start: usize) -> Option<(usize, String)> {
    if chars.get(start) != Some(&'!') || chars.get(start + 1) != Some(&'[') {
        return None;
    }
    let mut i = start + 2;
    while i < chars.len() && chars[i] != ']' {
        i += 1;
    }
    if chars.get(i + 1) != Some(&'(') {
        return None;
    }
    let path_start = i + 2;
    let mut j = path_start;
    while j < chars.len() && chars[j] != ')' {
        j += 1;
    }
    if j >= chars.len() {
        return None;
    }
    let path: String = chars[path_start..j].iter().collect();
    Some((j + 1 - start, path))
}

fn scan_body_links(line: &str, known: &HashSet<&str>) -> Vec<(usize, usize, Target)> {
    let chars: Vec<char> = line.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '!' {
            if let Some((len, path)) = parse_image_link(&chars, i) {
                out.push((i, len, Target::Artifact(path)));
                i += len;
                continue;
            }
        }
        let rest: String = chars[i..].iter().collect();
        if is_url(&rest) {
            let start = i;
            while i < chars.len() && !chars[i].is_whitespace() {
                i += 1;
            }
            out.push((
                start,
                i - start,
                Target::Url(chars[start..i].iter().collect()),
            ));
            continue;
        }
        if is_tok(chars[i]) {
            let start = i;
            while i < chars.len() && is_tok(chars[i]) {
                i += 1;
            }
            let tok: String = chars[start..i].iter().collect();
            if known.contains(tok.as_str()) {
                out.push((start, i - start, Target::Task(tok)));
            }
            continue;
        }
        i += 1;
    }
    out
}

fn field(label: &str, value: &str) -> DLine {
    DLine {
        text: format!("{label:<13}{value}"),
        kind: Kind::Field,
        links: vec![],
    }
}

fn section(text: &str) -> DLine {
    DLine {
        text: text.into(),
        kind: Kind::Section,
        links: vec![],
    }
}

fn empty() -> DLine {
    DLine {
        text: String::new(),
        kind: Kind::Empty,
        links: vec![],
    }
}

fn status_word(s: Status) -> &'static str {
    match s {
        Status::Hairy => "hairy",
        Status::Shaving => "shaving",
        Status::Shorn => "shorn",
        Status::Dead => "dead",
    }
}

fn opt_date(o: &Option<String>) -> String {
    match o {
        Some(s) => humanize_date(s),
        None => "-".to_string(),
    }
}

/// "2025-12-31T19:00:00Z" -> "Dec 31, 2025 19:00" (falls back to the raw string).
fn humanize_date(iso: &str) -> String {
    match chrono::DateTime::parse_from_rfc3339(iso) {
        Ok(dt) => dt.format("%b %-d, %Y %H:%M").to_string(),
        Err(_) => iso.to_string(),
    }
}

/// A "glyph id  title" reference line whose id is a link target. `glyph` is a
/// status emoji (display width 2 but a single `char`); link offsets below are
/// char indices, which `render_dline` styles as relative span flow, so the
/// wide emoji doesn't skew the id highlight.
fn ref_line(prefix: String, glyph: &str, id: &str, title: &str, exists: bool) -> DLine {
    let head = format!("{prefix}{glyph} ");
    let col = head.chars().count();
    let text = format!("{head}{id}  {title}");
    let links = if exists {
        vec![(col, id.chars().count(), Target::Task(id.to_string()))]
    } else {
        vec![]
    };
    DLine {
        text,
        kind: Kind::Field,
        links,
    }
}

pub fn build(task: &Task, all: &[Task]) -> Vec<DLine> {
    let by_id: HashMap<&str, &Task> = all.iter().map(|t| (t.id.as_str(), t)).collect();
    let known: HashSet<&str> = by_id.keys().copied().collect();

    let labels = if task.labels.is_empty() {
        "-".to_string()
    } else {
        task.labels.join(", ")
    };
    let mut out = vec![
        section(&format!("Task: {}", task.id)),
        empty(),
        field("Title:", &task.title),
        field("Status:", status_word(task.status)),
        field("Type:", &task.kind),
        field("Priority:", &format!("p{}", task.priority)),
        field("Created:", &opt_date(&task.created)),
        field("Updated:", &opt_date(&task.updated)),
        field("Labels:", &labels),
    ];

    if !task.depends_on.is_empty() {
        out.push(empty());
        out.push(section("Depends on:"));
        for d in &task.depends_on {
            let (glyph, title, exists) = match by_id.get(d.as_str()) {
                Some(t) => (t.status.emoji(), t.title.clone(), true),
                None => (" ", "(missing)".into(), false),
            };
            out.push(ref_line("  ".into(), glyph, d, &title, exists));
        }
    }

    // Blocks: reverse dependencies -- tasks that depend on this one.
    let mut blocks: Vec<&Task> = all
        .iter()
        .filter(|t| t.depends_on.iter().any(|d| d.as_str() == task.id.as_str()))
        .collect();
    blocks.sort_by(|a, b| a.id.cmp(&b.id));
    if !blocks.is_empty() {
        out.push(empty());
        out.push(section("Blocks:"));
        for b in blocks {
            out.push(ref_line(
                "  ".into(),
                b.status.emoji(),
                &b.id,
                &b.title,
                true,
            ));
        }
    }

    if let Some(p) = &task.parent {
        let (glyph, title, exists) = match by_id.get(p.as_str()) {
            Some(t) => (t.status.emoji(), t.title.as_str(), true),
            None => (" ", "(missing)", false),
        };
        out.push(empty());
        out.push(section("Parent:"));
        out.push(ref_line("  ".into(), glyph, p, title, exists));
    }

    let mut kids: Vec<&Task> = all
        .iter()
        .filter(|c| c.parent.as_deref() == Some(task.id.as_str()))
        .collect();
    kids.sort_by(|a, b| a.id.cmp(&b.id));
    if !kids.is_empty() {
        out.push(empty());
        out.push(section("Children:"));
        for c in kids {
            out.push(ref_line(
                "  ".into(),
                c.status.emoji(),
                &c.id,
                &c.title,
                true,
            ));
        }
    }

    if let Some(s) = &task.source {
        let head = format!("{:<13}", "Source:");
        let col = head.chars().count();
        let links = if is_url(s) {
            vec![(col, s.chars().count(), Target::Url(s.clone()))]
        } else {
            vec![]
        };
        out.push(empty());
        out.push(DLine {
            text: format!("{head}{s}"),
            kind: Kind::Field,
            links,
        });
    }

    let body = task.body.trim();
    if !body.is_empty() {
        out.push(DLine {
            text: String::new(),
            kind: Kind::Empty,
            links: vec![],
        });
        for raw in body.lines() {
            let line = strip_brackets(raw);
            let links = scan_body_links(&line, &known);
            out.push(DLine {
                text: line,
                kind: Kind::Body,
                links,
            });
        }
    }
    out
}

#[cfg(test)]
mod parity_tests {
    use super::*;

    fn t(id: &str, parent: Option<&str>, deps: &[&str]) -> Task {
        Task {
            id: id.into(),
            title: format!("title {id}"),
            kind: "task".into(),
            priority: 3,
            status: Status::Hairy,
            created: Some("2025-12-31T19:00:00Z".into()),
            updated: None,
            parent: parent.map(String::from),
            labels: vec![],
            depends_on: deps.iter().map(|s| s.to_string()).collect(),
            source: None,
            body: String::new(),
        }
    }

    #[test]
    fn header_fields_and_reverse_deps() {
        let all = vec![
            t("yak-0001", None, &[]),
            t("yak-0002", None, &["yak-0001"]), // 0002 depends on 0001
        ];
        let lines = build(&all[0], &all);
        let text: Vec<&str> = lines.iter().map(|l| l.text.as_str()).collect();
        assert_eq!(text[0], "Task: yak-0001");
        assert!(text.iter().any(|l| l.starts_with("Status:")));
        assert!(
            text.iter()
                .any(|l| l.starts_with("Created:") && l.contains("Dec 31, 2025 19:00"))
        );
        assert!(text.iter().any(|l| *l == "Blocks:"));
        let jumps = jumplist(&lines);
        assert!(
            jumps
                .iter()
                .any(|j| j.target == Target::Task("yak-0002".into()))
        );
    }

    #[test]
    fn humanize_date_formats_iso() {
        assert_eq!(humanize_date("2025-12-31T19:00:00Z"), "Dec 31, 2025 19:00");
        assert_eq!(humanize_date("not-a-date"), "not-a-date");
    }
}

/// Flatten every link across the lines, in reading order.
pub fn jumplist(lines: &[DLine]) -> Vec<Jump> {
    let mut out = Vec::new();
    for (i, l) in lines.iter().enumerate() {
        let mut links: Vec<&(usize, usize, Target)> = l.links.iter().collect();
        links.sort_by_key(|(c, _, _)| *c);
        for (col, _len, t) in links {
            out.push(Jump {
                line: i,
                col: *col,
                target: t.clone(),
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Status;

    fn task(id: &str, parent: Option<&str>, deps: &[&str], body: &str) -> Task {
        Task {
            id: id.into(),
            title: format!("title {id}"),
            kind: "task".into(),
            priority: 3,
            status: Status::Hairy,
            created: None,
            updated: None,
            parent: parent.map(String::from),
            labels: vec![],
            depends_on: deps.iter().map(|s| s.to_string()).collect(),
            source: None,
            body: body.into(),
        }
    }

    #[test]
    fn links_parent_deps_children_and_body_refs() {
        let all = vec![
            task(
                "yak-0001",
                None,
                &["yak-0002"],
                "see yak-0003 and [[yak-0002]] here",
            ),
            task("yak-0002", None, &[], ""),
            task("yak-0003", Some("yak-0001"), &[], ""), // child of 0001
        ];
        let lines = build(&all[0], &all);
        let jumps = jumplist(&lines);
        let targets: Vec<Target> = jumps.iter().map(|j| j.target.clone()).collect();
        // parent: none; dep yak-0002; child yak-0003; body refs yak-0003, yak-0002.
        assert_eq!(
            targets,
            vec![
                Target::Task("yak-0002".into()), // depends on
                Target::Task("yak-0003".into()), // children
                Target::Task("yak-0003".into()), // body ref
                Target::Task("yak-0002".into()), // body ref (brackets stripped)
            ]
        );
    }

    #[test]
    fn body_link_span_points_at_the_id() {
        let all = vec![task("yak-0001", None, &[], "x yak-0001 y")];
        // self-id still scans in body (resolution is the caller's concern here).
        let lines = build(&all[0], &all);
        let body = lines.last().unwrap();
        assert_eq!(body.text, "x yak-0001 y");
        assert_eq!(body.links.len(), 1);
        let (col, len, _) = &body.links[0];
        let slice: String = body.text.chars().skip(*col).take(*len).collect();
        assert_eq!(slice, "yak-0001");
    }

    #[test]
    fn url_in_body_is_a_link() {
        let all = vec![task(
            "yak-0001",
            None,
            &[],
            "docs at https://example.com/x ok",
        )];
        let lines = build(&all[0], &all);
        let jumps = jumplist(&lines);
        assert_eq!(
            jumps.iter().map(|j| j.target.clone()).collect::<Vec<_>>(),
            vec![Target::Url("https://example.com/x".into())]
        );
    }
}
