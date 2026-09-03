//! Roll yaks up to the external issues they point at — the read-only half of
//! the yak -> external projection. A yak's `source:` is its external home;
//! a yak with no own `source:` inherits its nearest ancestor's, resolved at
//! query time (never written into descendants). No network. Mirrors
//! yaklib/rollup.py.

use std::collections::{HashMap, HashSet};

use crate::filter::{self, FilterSpec};
use crate::model::Task;

/// Classify a `source:` URL into (tracker, human key).
pub fn tracker_and_key(source: &str) -> (&'static str, Option<String>) {
    let s = source.trim();
    if s.is_empty() {
        return ("none", None);
    }
    if let Some(k) = jira_key(s) {
        return ("jira", Some(k));
    }
    if let Some(k) = linear_key(s) {
        return ("linear", Some(k));
    }
    if let Some(k) = github_key(s) {
        return ("github", Some(k));
    }
    ("other", Some(s.to_string()))
}

// atlassian.net/browse/([A-Z][A-Z0-9_]*-\d+)
fn jira_key(s: &str) -> Option<String> {
    let idx = s.find("atlassian.net/browse/")?;
    let rest = &s[idx + "atlassian.net/browse/".len()..];
    let b = rest.as_bytes();
    let mut i = 0;
    if b.first()?.is_ascii_uppercase() {
        i += 1;
    } else {
        return None;
    }
    while i < b.len() && (b[i].is_ascii_uppercase() || b[i].is_ascii_digit() || b[i] == b'_') {
        i += 1;
    }
    if b.get(i) != Some(&b'-') {
        return None;
    }
    i += 1;
    let ds = i;
    while i < b.len() && b[i].is_ascii_digit() {
        i += 1;
    }
    if i == ds {
        return None;
    }
    Some(rest[..i].to_string())
}

// linear.app/[^/]+/issue/([A-Za-z0-9]+-\d+)
fn linear_key(s: &str) -> Option<String> {
    let li = s.find("linear.app/")?;
    let after = &s[li..];
    let ii = after.find("/issue/")?;
    let rest = &after[ii + "/issue/".len()..];
    let b = rest.as_bytes();
    let mut i = 0;
    while i < b.len() && b[i].is_ascii_alphanumeric() {
        i += 1;
    }
    if i == 0 || b.get(i) != Some(&b'-') {
        return None;
    }
    i += 1;
    let ds = i;
    while i < b.len() && b[i].is_ascii_digit() {
        i += 1;
    }
    if i == ds {
        return None;
    }
    Some(rest[..i].to_ascii_uppercase())
}

// github.com/([^/\s]+)/([^/\s]+)/issues/(\d+)
fn github_key(s: &str) -> Option<String> {
    let gi = s.find("github.com/")?;
    let rest = &s[gi + "github.com/".len()..];
    let seg = |from: usize| -> usize {
        let b = rest.as_bytes();
        let mut i = from;
        while i < b.len() && b[i] != b'/' && !b[i].is_ascii_whitespace() {
            i += 1;
        }
        i
    };
    let o_end = seg(0);
    if o_end == 0 || rest.as_bytes().get(o_end) != Some(&b'/') {
        return None;
    }
    let owner = &rest[..o_end];
    let r_start = o_end + 1;
    let r_end = seg(r_start);
    if r_end == r_start {
        return None;
    }
    let repo = &rest[r_start..r_end];
    let tail = rest[r_end..].strip_prefix("/issues/")?;
    let tb = tail.as_bytes();
    let mut j = 0;
    while j < tb.len() && tb[j].is_ascii_digit() {
        j += 1;
    }
    if j == 0 {
        return None;
    }
    Some(format!("{owner}/{repo}#{}", &tail[..j]))
}

/// Resolve a yak's effective source by walking up the parent chain.
/// Returns (source, inherited_from) — inherited_from is None if the yak
/// carries its own source, else the ancestor id it came from.
fn effective_source<'a>(
    start: &str,
    source_by_id: &HashMap<&str, &'a str>,
    parent_by_id: &HashMap<&str, &str>,
) -> (Option<&'a str>, Option<String>) {
    let mut cur = Some(start);
    let mut seen: HashSet<&str> = HashSet::new();
    while let Some(c) = cur {
        if !seen.insert(c) {
            break;
        }
        if let Some(src) = source_by_id.get(c) {
            let from = if c == start {
                None
            } else {
                Some(c.to_string())
            };
            return (Some(src), from);
        }
        cur = parent_by_id.get(c).copied();
    }
    (None, None)
}

/// One yak within a rollup group (owned, so groups outlive the load).
pub struct RollupItem {
    pub task: Task,
    pub inherited_from: Option<String>,
}

pub struct Group {
    pub source: String,
    pub tracker: &'static str,
    pub key: Option<String>,
    pub yaks: Vec<RollupItem>,
}

impl Group {
    /// Display head: the human key, or the raw source URL when unclassified.
    pub fn head(&self) -> String {
        self.key.clone().unwrap_or_else(|| self.source.clone())
    }
}

/// Group the filtered yaks by effective source. `tasks` must be the visible
/// (non-dead) set — inheritance maps are built from all of them, not just the
/// filtered subset. Returns (groups sorted by key/source, unsourced count).
pub fn build(tasks: &[Task], spec: &FilterSpec) -> (Vec<Group>, usize) {
    let mut source_by_id: HashMap<&str, &str> = HashMap::new();
    let mut parent_by_id: HashMap<&str, &str> = HashMap::new();
    for t in tasks {
        if let Some(s) = &t.source {
            source_by_id.insert(t.id.as_str(), s.as_str());
        }
        if let Some(p) = &t.parent {
            parent_by_id.insert(t.id.as_str(), p.as_str());
        }
    }

    let mut groups: HashMap<String, Vec<RollupItem>> = HashMap::new();
    let mut unsourced = 0usize;
    for t in filter::apply(tasks, spec, false) {
        let (src, from) = effective_source(&t.id, &source_by_id, &parent_by_id);
        match src {
            None => unsourced += 1,
            Some(src) => groups.entry(src.to_string()).or_default().push(RollupItem {
                task: t.clone(),
                inherited_from: from,
            }),
        }
    }

    let mut out: Vec<Group> = groups
        .into_iter()
        .map(|(source, mut yaks)| {
            yaks.sort_by(|a, b| a.task.id.cmp(&b.task.id));
            let (tracker, key) = tracker_and_key(&source);
            Group {
                source,
                tracker,
                key,
                yaks,
            }
        })
        .collect();
    out.sort_by(|a, b| a.head().cmp(&b.head()));
    (out, unsourced)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Status;

    #[test]
    fn classify_trackers() {
        assert_eq!(
            tracker_and_key("https://x.atlassian.net/browse/SUBTEXT-369"),
            ("jira", Some("SUBTEXT-369".into()))
        );
        assert_eq!(
            tracker_and_key("https://linear.app/team/issue/roc-5/title"),
            ("linear", Some("ROC-5".into()))
        );
        assert_eq!(
            tracker_and_key("https://github.com/o/r/issues/123"),
            ("github", Some("o/r#123".into()))
        );
        assert_eq!(
            tracker_and_key("https://example.com/x"),
            ("other", Some("https://example.com/x".into()))
        );
        assert_eq!(tracker_and_key(""), ("none", None));
    }

    fn t(id: &str, parent: Option<&str>, source: Option<&str>) -> Task {
        Task {
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
            source: source.map(String::from),
            needs: None,
            extra: Vec::new(),
            body: String::new(),
        }
    }

    #[test]
    fn groups_with_ancestor_inheritance_and_unsourced() {
        let tasks = vec![
            t("umbrella", None, Some("https://github.com/o/r/issues/7")),
            t("child", Some("umbrella"), None),
            t("solo", None, Some("https://acme.atlassian.net/browse/AB-1")),
            t("orphan", None, None),
        ];
        let (groups, unsourced) = build(&tasks, &FilterSpec::default());
        assert_eq!(unsourced, 1);
        assert_eq!(groups[0].key.as_deref(), Some("AB-1"));
        assert_eq!(groups[0].tracker, "jira");
        assert_eq!(groups[1].key.as_deref(), Some("o/r#7"));
        let gh = &groups[1];
        assert_eq!(gh.yaks.len(), 2);
        assert_eq!(gh.yaks[0].task.id, "child");
        assert_eq!(gh.yaks[0].inherited_from.as_deref(), Some("umbrella"));
        assert_eq!(gh.yaks[1].task.id, "umbrella");
        assert_eq!(gh.yaks[1].inherited_from, None);
    }
}
