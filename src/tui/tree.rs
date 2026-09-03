//! Task tree building + collapse for the list pane.
//!
//! `build` returns a flat, pre-ordered list of rows for one tab. The tab's
//! status set is the *anchor*; the tab also pulls in each anchor's family
//! (ancestors walked up, descendants walked down — any status) so the tree
//! stays rooted. Non-anchor family rows are marked `ghost` (rendered dimmed).
//! Content-filter re-coloring (bright matches vs dim context) is deferred to
//! the filter slice; this is the no-filter path.

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

use crate::filter::{self, FilterSpec};
use crate::model::{Status, Task};
use crate::tui::view::HerdScope;

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

impl<'a> Row<'a> {
    /// A depth-0, non-ghost, childless row (for flat / working-set views).
    pub fn leaf(task: &'a Task) -> Row<'a> {
        Row {
            task,
            depth: 0,
            ghost: false,
            has_children: false,
            collapsed: false,
            hidden: 0,
        }
    }
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

/// Build the pre-ordered tree rows for `tab`'s scope over `all`, honoring the
/// live `spec`. With no content filter, the tab's own tasks are the focus and
/// their family is dimmed context. With a content filter, matches anywhere in
/// that family become the focus and non-matching ancestors are dimmed to root
/// them; everything else is pruned. Mirrors Python `tree.build_tree`.
pub fn build<'a>(all: &'a [Task], spec: &FilterSpec, herd: HerdScope) -> Vec<Row<'a>> {
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

    // Effective status scope: the spec's statuses, else all non-dead statuses
    // (a bare/custom view with no status axis spans the whole herd).
    let eff: Vec<Status> = if spec.statuses.is_empty() {
        vec![Status::Hairy, Status::Shaving, Status::Shorn]
    } else {
        spec.statuses.clone()
    };
    let anchors: HashSet<&str> = all
        .iter()
        .filter(|t| eff.contains(&t.status))
        .map(|t| t.id.as_str())
        .collect();

    let ancestors_of = |start: &str| -> Vec<&'a str> {
        let mut out = Vec::new();
        let mut seen: HashSet<&str> = HashSet::new();
        let mut pid = by_id.get(start).and_then(|t| t.parent.as_deref());
        while let Some(p) = pid {
            if !seen.insert(p) {
                break;
            }
            if let Some(t) = by_id.get(p) {
                out.push(t.id.as_str());
            }
            pid = by_id.get(p).and_then(|t| t.parent.as_deref());
        }
        out
    };

    // universe = anchors + ancestors (up) + all descendants (down), any status.
    // It's the search space for content matches; herd scope then decides how
    // much of it actually renders.
    let mut universe: HashSet<&str> = anchors.clone();
    for &a in &anchors {
        universe.extend(ancestors_of(a));
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

    // Seeds (the bright focus): content matches anywhere in the family when a
    // filter is active, else the anchors themselves.
    let focus: HashSet<&str> = if spec.content_active() {
        let resolved = filter::resolved_ids(all);
        universe
            .iter()
            .copied()
            .filter(|&tid| spec.matches(by_id[tid], &resolved))
            .collect()
    } else {
        anchors.clone()
    };

    // members = seeds + ancestors (always, to root the chain) + herd-scoped
    // descendants. One rule for both the filtered and unfiltered paths, so
    // turning on a filter no longer silently drops descendant context.
    let mut members: HashSet<&str> = focus.clone();
    for &s in &focus {
        members.extend(ancestors_of(s));
    }
    members.extend(herd_descendants(&focus, herd, &children_of, &by_id));

    // roots = members whose parent is not itself a member.
    let mut roots: Vec<&str> = members
        .iter()
        .copied()
        .filter(|id| match by_id[id].parent.as_deref() {
            Some(p) => !members.contains(p),
            None => true,
        })
        .collect();
    if eff == [Status::Shorn] {
        // Recency-first for the (unbounded) shorn view.
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
        flatten(r, 0, &by_id, &children_of, &members, &focus, &mut out);
    }
    out
}

/// The descendant rows a tree view pulls in under its `seeds`, per herd scope.
/// `Lone` yields nothing; `All` yields the full descendant closure; `Remaining`
/// keeps descendants with open work (hairy/shaving) plus the completed nodes
/// that connect them back to a seed, dropping fully-shorn subtrees.
fn herd_descendants<'a>(
    seeds: &HashSet<&'a str>,
    herd: HerdScope,
    children_of: &HashMap<&'a str, Vec<&'a str>>,
    by_id: &HashMap<&'a str, &'a Task>,
) -> HashSet<&'a str> {
    if herd == HerdScope::Lone {
        return HashSet::new();
    }
    // Full descendant closure of the seeds (any status).
    let mut full: HashSet<&str> = HashSet::new();
    let mut stack: Vec<&str> = seeds.iter().copied().collect();
    while let Some(cur) = stack.pop() {
        if let Some(kids) = children_of.get(cur) {
            for &c in kids {
                if !seeds.contains(c) && full.insert(c) {
                    stack.push(c);
                }
            }
        }
    }
    if herd == HerdScope::All {
        return full;
    }
    // Remaining: start from the open descendants, then walk each one up toward
    // its seed, keeping the completed nodes in between as (dim) connectors.
    let is_open = |id: &str| matches!(by_id[id].status, Status::Hairy | Status::Shaving);
    let mut keep: HashSet<&str> = full.iter().copied().filter(|&d| is_open(d)).collect();
    let open: Vec<&str> = keep.iter().copied().collect();
    for od in open {
        let mut pid = by_id[od].parent.as_deref();
        while let Some(p) = pid {
            if seeds.contains(p) || !full.contains(p) || !keep.insert(p) {
                break;
            }
            pid = by_id[p].parent.as_deref();
        }
    }
    keep
}

fn flatten<'a>(
    id: &'a str,
    depth: u16,
    by_id: &HashMap<&'a str, &'a Task>,
    children_of: &HashMap<&'a str, Vec<&'a str>>,
    members: &HashSet<&'a str>,
    focus: &HashSet<&str>,
    out: &mut Vec<Row<'a>>,
) {
    let mut kids: Vec<&str> = children_of
        .get(id)
        .map(|v| v.iter().copied().filter(|c| members.contains(c)).collect())
        .unwrap_or_default();
    kids.sort_by(|&a, &b| cmp_child(by_id[a], by_id[b]));
    out.push(Row {
        task: by_id[id],
        depth,
        ghost: !focus.contains(id),
        has_children: !kids.is_empty(),
        collapsed: false,
        hidden: 0,
    });
    for c in kids {
        flatten(c, depth + 1, by_id, children_of, members, focus, out);
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
            needs: None,
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
        let spec = FilterSpec {
            statuses: vec![Status::Hairy],
            ..Default::default()
        };
        let flat = build(&all, &spec, HerdScope::All);
        assert_eq!(flat.len(), 3);
        let collapsed: HashSet<String> = ["a".to_string()].into_iter().collect();
        let vis = apply_collapse(flat, &collapsed);
        assert_eq!(vis.len(), 1);
        assert!(vis[0].collapsed);
        assert_eq!(vis[0].hidden, 2);
    }

    #[test]
    fn content_filter_focuses_matches_and_dims_ancestors() {
        // a > a1 > a2 ; only a2's title matches. a2 is focus (bright), a + a1
        // come along as dimmed ancestors, and unrelated b is pruned.
        let mut a = t("a", Status::Hairy, None);
        a.title = "root alpha".into();
        let mut a1 = t("a1", Status::Hairy, Some("a"));
        a1.title = "mid".into();
        let mut a2 = t("a2", Status::Hairy, Some("a1"));
        a2.title = "needle here".into();
        let b = t("b", Status::Hairy, None);
        let all = vec![a, a1, a2, b];

        let spec = FilterSpec {
            statuses: vec![Status::Hairy],
            search: Some("needle".into()),
            ..Default::default()
        };
        let flat = build(&all, &spec, HerdScope::All);
        let ids: Vec<&str> = flat.iter().map(|r| r.task.id.as_str()).collect();
        assert_eq!(ids, vec!["a", "a1", "a2"]); // b pruned
        let ghost = |id: &str| flat.iter().find(|r| r.task.id == id).unwrap().ghost;
        assert!(ghost("a") && ghost("a1"), "non-matching ancestors dim");
        assert!(!ghost("a2"), "the match is the focus");
    }

    #[test]
    fn herd_scope_governs_descendants() {
        // a(hairy anchor) with a shorn leaf b and a shaving child c. On the
        // Hairy view b and c are non-anchor descendants, governed by the scope.
        let all = vec![
            t("a", Status::Hairy, None),
            t("b", Status::Shorn, Some("a")),
            t("c", Status::Shaving, Some("a")),
        ];
        let spec = FilterSpec {
            statuses: vec![Status::Hairy],
            ..Default::default()
        };
        let ids = |scope| {
            let mut v: Vec<String> = build(&all, &spec, scope)
                .iter()
                .map(|r| r.task.id.clone())
                .collect();
            v.sort();
            v
        };
        assert_eq!(ids(HerdScope::Lone), vec!["a"]);
        assert_eq!(ids(HerdScope::Remaining), vec!["a", "c"]); // shorn leaf b dropped
        assert_eq!(ids(HerdScope::All), vec!["a", "b", "c"]);
    }

    #[test]
    fn remaining_keeps_completed_connectors_to_open_work() {
        // a(hairy) > b(shorn) > c(shaving): b is completed but connects open c,
        // so "remaining" must keep it (never show a child without its chain).
        let all = vec![
            t("a", Status::Hairy, None),
            t("b", Status::Shorn, Some("a")),
            t("c", Status::Shaving, Some("b")),
        ];
        let spec = FilterSpec {
            statuses: vec![Status::Hairy],
            ..Default::default()
        };
        let flat = build(&all, &spec, HerdScope::Remaining);
        let mut ids: Vec<String> = flat.iter().map(|r| r.task.id.clone()).collect();
        ids.sort();
        assert_eq!(ids, vec!["a", "b", "c"]);
        let ghost = |id: &str| flat.iter().find(|r| r.task.id == id).unwrap().ghost;
        assert!(
            ghost("b") && ghost("c"),
            "connector + open descendant are dim"
        );
        assert!(!ghost("a"), "the anchor is bright");
    }

    #[test]
    fn content_filter_includes_descendants_of_matches() {
        // a matches; its open child a1 now comes along as descendant context.
        // (The old filter path dropped descendants of a match — yaksrs-3331.)
        let mut a = t("a", Status::Hairy, None);
        a.title = "needle root".into();
        let a1 = t("a1", Status::Hairy, Some("a"));
        let all = vec![a, a1];
        let spec = FilterSpec {
            statuses: vec![Status::Hairy],
            search: Some("needle".into()),
            ..Default::default()
        };
        let flat = build(&all, &spec, HerdScope::Remaining);
        let ids: Vec<&str> = flat.iter().map(|r| r.task.id.as_str()).collect();
        assert_eq!(ids, vec!["a", "a1"]);
        let ghost = |id: &str| flat.iter().find(|r| r.task.id == id).unwrap().ghost;
        assert!(!ghost("a"), "the match is bright");
        assert!(ghost("a1"), "its descendant is dim context");
    }
}
