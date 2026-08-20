//! Shared task filtering — mirrors the Python `yaklib.filter.FilterSpec` +
//! `yaklib.deps` resolution. Semantics: AND across fields; OR within a
//! multi-value field; an empty field is unconstrained.
//!
//! A dependency counts as "resolved" when its task is shorn OR dead, so
//! slaughtering a blocker unblocks its dependents.

use std::collections::{HashMap, HashSet};

use crate::model::{Status, Task};

const NON_DEAD: [Status; 3] = [Status::Hairy, Status::Shaving, Status::Shorn];

#[derive(Default)]
pub struct FilterSpec {
    /// Empty = unconstrained (caller's default scope applies).
    pub statuses: Vec<Status>,
    pub types: Vec<String>,
    pub priorities: Vec<u8>,
    pub labels: Vec<String>,
    pub search: Option<String>,
    pub ready_only: bool,
    pub tangled_only: bool,
    /// Descendant-of scope (a task id); matches its descendants at any depth.
    pub parent: Option<String>,
}

impl FilterSpec {
    fn matches(&self, t: &Task, resolved: &HashSet<&str>) -> bool {
        if !self.types.is_empty() && !self.types.iter().any(|k| k == &t.kind) {
            return false;
        }
        if !self.priorities.is_empty() && !self.priorities.contains(&t.priority) {
            return false;
        }
        if !self.labels.is_empty() && !t.labels.iter().any(|l| self.labels.contains(l)) {
            return false;
        }
        if let Some(q) = &self.search {
            let q = q.to_lowercase();
            let blob = format!("{} {} {}", t.title, t.body, t.id).to_lowercase();
            if !blob.contains(&q) {
                return false;
            }
        }
        let has_unresolved = t.depends_on.iter().any(|d| !resolved.contains(d.as_str()));
        if self.ready_only && has_unresolved {
            return false;
        }
        if self.tangled_only && !has_unresolved {
            return false;
        }
        true
    }
}

/// Ids that count as "dep satisfied" — shorn + dead.
pub fn resolved_ids(tasks: &[Task]) -> HashSet<&str> {
    tasks
        .iter()
        .filter(|t| t.status.is_resolved())
        .map(|t| t.id.as_str())
        .collect()
}

/// Unresolved deps of a task, in declared order (for `tangled` "waiting on").
pub fn unresolved_deps<'a>(t: &'a Task, resolved: &HashSet<&str>) -> Vec<&'a str> {
    t.depends_on
        .iter()
        .map(String::as_str)
        .filter(|d| !resolved.contains(d))
        .collect()
}

/// All descendants of `root_id` at any depth, following `parent` pointers.
pub fn descendant_ids(tasks: &[Task], root_id: &str, include_dead: bool) -> HashSet<String> {
    let mut children: HashMap<&str, Vec<&str>> = HashMap::new();
    for t in tasks {
        if !include_dead && t.status == Status::Dead {
            continue;
        }
        if let Some(p) = &t.parent {
            children.entry(p.as_str()).or_default().push(t.id.as_str());
        }
    }
    let mut out = HashSet::new();
    let mut stack: Vec<&str> = children.get(root_id).cloned().unwrap_or_default();
    while let Some(id) = stack.pop() {
        if out.insert(id.to_string()) {
            if let Some(kids) = children.get(id) {
                stack.extend(kids.iter().copied());
            }
        }
    }
    out
}

/// Apply `spec` over a fully-loaded task set. `include_dead` only affects the
/// default scope when `spec.statuses` is empty.
pub fn apply<'a>(tasks: &'a [Task], spec: &FilterSpec, include_dead: bool) -> Vec<&'a Task> {
    let resolved = resolved_ids(tasks);
    let scope = spec
        .parent
        .as_ref()
        .map(|p| descendant_ids(tasks, p, spec.statuses.contains(&Status::Dead)));

    tasks
        .iter()
        .filter(|t| {
            if !spec.statuses.is_empty() {
                if !spec.statuses.contains(&t.status) {
                    return false;
                }
            } else if !include_dead && !NON_DEAD.contains(&t.status) {
                return false;
            }
            if let Some(scope) = &scope {
                if !scope.contains(&t.id) {
                    return false;
                }
            }
            spec.matches(t, &resolved)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(id: &str, status: Status, deps: &[&str], parent: Option<&str>) -> Task {
        Task {
            id: id.into(),
            title: format!("title {id}"),
            kind: "task".into(),
            priority: 3,
            status,
            created: None,
            updated: None,
            parent: parent.map(String::from),
            labels: vec![],
            depends_on: deps.iter().map(|s| s.to_string()).collect(),
            source: None,
            body: String::new(),
        }
    }

    fn herd() -> Vec<Task> {
        vec![
            t("a", Status::Shorn, &[], None),
            t("b", Status::Hairy, &["a"], None),       // ready (dep a shorn)
            t("c", Status::Hairy, &["d"], None),       // tangled (dep d hairy)
            t("d", Status::Hairy, &[], None),
            t("k", Status::Hairy, &[], Some("b")),     // child of b
            t("z", Status::Dead, &[], None),
        ]
    }

    fn ids(v: Vec<&Task>) -> Vec<String> {
        let mut o: Vec<String> = v.into_iter().map(|t| t.id.clone()).collect();
        o.sort();
        o
    }

    #[test]
    fn ready_only_is_hairy_with_resolved_deps() {
        let h = herd();
        let spec = FilterSpec { statuses: vec![Status::Hairy], ready_only: true, ..Default::default() };
        assert_eq!(ids(apply(&h, &spec, false)), vec!["b", "d", "k"]);
    }

    #[test]
    fn tangled_only_is_hairy_with_unresolved_deps() {
        let h = herd();
        let spec = FilterSpec { statuses: vec![Status::Hairy], tangled_only: true, ..Default::default() };
        assert_eq!(ids(apply(&h, &spec, false)), vec!["c"]);
    }

    #[test]
    fn default_scope_excludes_dead() {
        let h = herd();
        let got = ids(apply(&h, &FilterSpec::default(), false));
        assert!(!got.contains(&"z".to_string()));
        assert!(got.contains(&"a".to_string()));
    }

    #[test]
    fn parent_of_scopes_to_descendants() {
        let h = herd();
        let spec = FilterSpec { parent: Some("b".into()), ..Default::default() };
        assert_eq!(ids(apply(&h, &spec, false)), vec!["k"]);
    }

    #[test]
    fn search_is_case_insensitive_over_id_title_body() {
        let h = herd();
        let spec = FilterSpec { search: Some("TITLE C".into()), ..Default::default() };
        assert_eq!(ids(apply(&h, &spec, false)), vec!["c"]);
    }
}
