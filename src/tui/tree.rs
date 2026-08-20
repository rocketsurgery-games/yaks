//! Task tree building + collapse for the list pane, ported from the Python
//! `yaktui.tree`.
//!
//! `build` returns a flat, pre-ordered list of rows for one tab. The tab's
//! status set is the *anchor*; the tab also pulls in each anchor's family
//! (ancestors walked up, descendants walked down — any status) so the tree
//! stays rooted. Non-anchor family rows are marked `ghost` (rendered dimmed).
//! Content-filter re-coloring (bright matches vs dim context) is deferred to
//! the filter slice; this is the no-filter path.

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

use crate::model::{Status, Task};

pub struct Row<'a> {
    pub task: &'a Task,
    pub depth: u16,
    pub ghost: bool,
    pub has_children: bool,
    /// Set by `apply_collapse` when this row is a collapsed parent.
    pub collapsed: bool,
    /// Number of descendant rows hidden beneath a collapsed parent.
    pub hidden: usize,
}

fn child_rank(s: Status) -> u8 {
    match s {
        Status::Shaving => 0,
        Status::Shorn => 2,
        _ => 1,
    }
}

fn cmp_child(a: &Task, b: &Task) -> Ordering {
    child_rank(a.status)
        .cmp(&child_rank(b.status))
        .then(a.priority.cmp(&b.priority))
        .then(
            a.created
                .as_deref()
                .unwrap_or("")
                .cmp(b.created.as_deref().unwrap_or("")),
        )
        .then(a.id.cmp(&b.id))
}

/// Build the pre-ordered tree rows for `tab`'s scope over `all`.
pub fn build(all: &[Task], tab: Status) -> Vec<Row<'_>> {
    let by_id: HashMap<&str, &Task> = all.iter().map(|t| (t.id.as_str(), t)).collect();

    let mut children_of: HashMap<&str, Vec<&str>> = HashMap::new();
    for t in all {
        if let Some(p) = &t.parent {
            if by_id.contains_key(p.as_str()) {
                children_of
                    .entry(p.as_str())
                    .or_default()
                    .push(t.id.as_str());
            }
        }
    }

    let anchors: HashSet<&str> = all
        .iter()
        .filter(|t| t.status == tab)
        .map(|t| t.id.as_str())
        .collect();

    // universe = anchors + ancestors (up) + descendants (down), any status.
    let mut universe: HashSet<&str> = anchors.clone();
    for &a in &anchors {
        let mut pid = by_id.get(a).and_then(|t| t.parent.as_deref());
        let mut seen: HashSet<&str> = HashSet::new();
        while let Some(p) = pid {
            if !seen.insert(p) {
                break;
            }
            if by_id.contains_key(p) {
                universe.insert(p);
            }
            pid = by_id.get(p).and_then(|t| t.parent.as_deref());
        }
    }
    let mut stack: Vec<&str> = anchors.iter().copied().collect();
    while let Some(cur) = stack.pop() {
        if let Some(kids) = children_of.get(cur) {
            for &c in kids {
                if universe.insert(c) {
                    stack.push(c);
                }
            }
        }
    }

    // roots = members whose parent is not itself a member.
    let mut roots: Vec<&str> = universe
        .iter()
        .copied()
        .filter(|id| match by_id[id].parent.as_deref() {
            Some(p) => !universe.contains(p),
            None => true,
        })
        .collect();
    if tab == Status::Shorn {
        // Recency-first for the (unbounded) shorn tab.
        roots.sort_by(|&a, &b| {
            by_id[b]
                .updated
                .as_deref()
                .unwrap_or("")
                .cmp(by_id[a].updated.as_deref().unwrap_or(""))
                .then(a.cmp(b))
        });
    } else {
        roots.sort_by(|&a, &b| by_id[a].priority.cmp(&by_id[b].priority).then(a.cmp(b)));
    }

    let mut out = Vec::new();
    for r in roots {
        flatten(r, 0, &by_id, &children_of, &universe, &anchors, &mut out);
    }
    out
}

fn flatten<'a>(
    id: &'a str,
    depth: u16,
    by_id: &HashMap<&'a str, &'a Task>,
    children_of: &HashMap<&'a str, Vec<&'a str>>,
    universe: &HashSet<&'a str>,
    anchors: &HashSet<&str>,
    out: &mut Vec<Row<'a>>,
) {
    let mut kids: Vec<&str> = children_of
        .get(id)
        .map(|v| v.iter().copied().filter(|c| universe.contains(c)).collect())
        .unwrap_or_default();
    kids.sort_by(|&a, &b| cmp_child(by_id[a], by_id[b]));
    out.push(Row {
        task: by_id[id],
        depth,
        ghost: !anchors.contains(id),
        has_children: !kids.is_empty(),
        collapsed: false,
        hidden: 0,
    });
    for c in kids {
        flatten(c, depth + 1, by_id, children_of, universe, anchors, out);
    }
}

/// Drop descendants of collapsed ids; annotate collapsed parents with their
/// hidden-row count. Empty `collapsed` returns the input unchanged.
pub fn apply_collapse<'a>(flat: Vec<Row<'a>>, collapsed: &HashSet<String>) -> Vec<Row<'a>> {
    if collapsed.is_empty() {
        return flat;
    }
    let mut counts: HashMap<String, usize> = HashMap::new();
    let mut visible: Vec<Row<'a>> = Vec::new();
    let mut hide_stack: Vec<(String, u16)> = Vec::new();
    for mut row in flat {
        let depth = row.depth;
        let id = row.task.id.clone();
        let is_collapsed = row.has_children && collapsed.contains(id.as_str());
        while hide_stack.last().is_some_and(|(_, d)| depth <= *d) {
            hide_stack.pop();
        }
        if hide_stack.is_empty() {
            row.collapsed = is_collapsed;
            visible.push(row);
        } else {
            for (cid, _) in &hide_stack {
                *counts.entry(cid.clone()).or_insert(0) += 1;
            }
        }
        if is_collapsed {
            hide_stack.push((id, depth));
        }
    }
    for row in &mut visible {
        if row.collapsed {
            row.hidden = counts.get(row.task.id.as_str()).copied().unwrap_or(0);
        }
    }
    visible
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(id: &str, status: Status, parent: Option<&str>) -> Task {
        Task {
            id: id.into(),
            title: id.into(),
            kind: "task".into(),
            priority: 3,
            status,
            created: None,
            updated: None,
            parent: parent.map(String::from),
            labels: vec![],
            depends_on: vec![],
            source: None,
            body: String::new(),
        }
    }

    #[test]
    fn collapse_hides_descendants_and_counts() {
        let all = vec![
            t("a", Status::Hairy, None),
            t("a1", Status::Hairy, Some("a")),
            t("a2", Status::Hairy, Some("a")),
        ];
        let flat = build(&all, Status::Hairy);
        assert_eq!(flat.len(), 3);
        let collapsed: HashSet<String> = ["a".to_string()].into_iter().collect();
        let vis = apply_collapse(flat, &collapsed);
        assert_eq!(vis.len(), 1);
        assert!(vis[0].collapsed);
        assert_eq!(vis[0].hidden, 2);
    }
}
