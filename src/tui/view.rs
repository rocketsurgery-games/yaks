//! View model for the list pane — ported from Python `yaktui.view`. A View is a
//! named, ordered entry in the tab strip: a saved `FilterSpec` plus optional
//! sort/limit. The three status views behave like the old fixed tabs (their
//! spec scopes to one status); Recent is a flat most-recent list; Starred is
//! the explicit working set. Activating a view loads its spec into the single
//! live filter, so status is just another (removable) filter axis at runtime.

use crate::filter::FilterSpec;
use crate::model::Status;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum SortField {
    Updated,
    Created,
    Priority,
    Title,
    Id,
}

impl SortField {
    pub fn as_str(self) -> &'static str {
        match self {
            SortField::Updated => "updated",
            SortField::Created => "created",
            SortField::Priority => "priority",
            SortField::Title => "title",
            SortField::Id => "id",
        }
    }

    pub fn parse(s: &str) -> Option<SortField> {
        Some(match s {
            "updated" => SortField::Updated,
            "created" => SortField::Created,
            "priority" => SortField::Priority,
            "title" => SortField::Title,
            "id" => SortField::Id,
            _ => return None,
        })
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum SortDir {
    Asc,
    Desc,
}

#[derive(Clone)]
pub struct View {
    pub name: String,
    pub key: String,
    pub status: Option<Status>,
    pub builtin: bool,
    pub pinned: bool,
    pub spec: FilterSpec,
    pub sort_by: Option<SortField>,
    pub sort_dir: SortDir,
    pub limit: Option<usize>,
}

impl View {
    /// Sorted views render flat; unsorted (status/custom-tree) views build the
    /// parent/child tree.
    pub fn is_flat(&self) -> bool {
        self.sort_by.is_some()
    }
}

/// How many rows Recent shows: a working list, not an archive, so it's capped.
pub const RECENT_LIMIT: usize = 50;

fn status_view(name: &str, status: Status) -> View {
    View {
        name: name.into(),
        key: format!("status:{}", status_key(status)),
        status: Some(status),
        builtin: true,
        pinned: true,
        spec: FilterSpec {
            statuses: vec![status],
            ..Default::default()
        },
        sort_by: None,
        sort_dir: SortDir::Desc,
        limit: None,
    }
}

pub fn status_key(s: Status) -> &'static str {
    match s {
        Status::Hairy => "hairy",
        Status::Shaving => "shaving",
        Status::Shorn => "shorn",
        Status::Dead => "dead",
    }
}

pub fn builtin_status_views() -> Vec<View> {
    vec![
        status_view("Hairy", Status::Hairy),
        status_view("Shaving", Status::Shaving),
        status_view("Shorn", Status::Shorn),
    ]
}

pub fn recent_view() -> View {
    View {
        name: "Recent".into(),
        key: "recent".into(),
        status: None,
        builtin: true,
        pinned: true,
        spec: FilterSpec::default(),
        sort_by: Some(SortField::Updated),
        sort_dir: SortDir::Desc,
        limit: Some(RECENT_LIMIT),
    }
}

pub fn working_set_view() -> View {
    View {
        name: "Starred".into(),
        key: "working-set".into(),
        status: None,
        builtin: true,
        pinned: true,
        spec: FilterSpec::default(),
        sort_by: None,
        sort_dir: SortDir::Desc,
        limit: None,
    }
}

pub fn default_views() -> Vec<View> {
    let mut v = builtin_status_views();
    v.push(recent_view());
    v.push(working_set_view());
    v
}

/// A user-created view with a stable generated key. Pinned by default so it
/// lands on the tab bar; not built-in, so the picker can rename or delete it.
pub fn custom_view(
    name: String,
    spec: FilterSpec,
    sort_by: Option<SortField>,
    sort_dir: SortDir,
    limit: Option<usize>,
    key_seed: &str,
) -> View {
    View {
        name,
        key: format!("view:{}", short_hash(key_seed)),
        status: None,
        builtin: false,
        pinned: true,
        spec,
        sort_by,
        sort_dir,
        limit,
    }
}

/// FNV-1a → 8 hex chars, for custom-view keys.
pub fn short_hash(s: &str) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{h:016x}")[..8].to_string()
}
