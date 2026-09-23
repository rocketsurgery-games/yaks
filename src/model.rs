//! Core task model for yaks.
//!
//! Status is implicit from the
//! directory the file lives in; parentage is a frontmatter field (flat,
//! stable ids).

/// Lifecycle state, encoded by which directory a task file lives in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Hairy,
    Shaving,
    Shorn,
    Dead,
}

impl Status {
    /// Directory name under `.yaks/` for this status.
    pub fn dir(self) -> &'static str {
        match self {
            Status::Hairy => "hairy",
            Status::Shaving => "shaving",
            Status::Shorn => "shorn",
            Status::Dead => "dead",
        }
    }

    /// Compact single-character glyph for listings.
    pub fn glyph(self) -> char {
        match self {
            Status::Hairy => 'H',
            Status::Shaving => 'S',
            Status::Shorn => 'N',
            Status::Dead => 'X',
        }
    }

    /// Status emoji (bison/razor/sheep/skull), matching the TUI list + tab bar.
    pub fn emoji(self) -> &'static str {
        match self {
            Status::Hairy => "\u{1f9ac}",
            Status::Shaving => "\u{1fa92}",
            Status::Shorn => "\u{1f411}",
            Status::Dead => "\u{1f480}",
        }
    }

    /// A dependency counts as "resolved" once it is shorn or dead.
    pub fn is_resolved(self) -> bool {
        matches!(self, Status::Shorn | Status::Dead)
    }
}

/// A single task, parsed from a `.md` file with YAML frontmatter.
///
/// The task's field set. `created`/`updated` are
/// kept as opaque ISO-8601 strings so a read/write round-trip preserves them
/// byte-for-byte (we never reformat timestamps we did not author).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub kind: String,
    pub priority: u8,
    pub status: Status,
    pub created: Option<String>,
    pub updated: Option<String>,
    pub parent: Option<String>,
    pub labels: Vec<String>,
    pub depends_on: Vec<String>,
    pub source: Option<String>,
    /// A soft, external block: while set (e.g. `human`), the yak is not ready in
    /// `next`. Raised by `ask`, cleared by `answer`. Not ownership, not a status.
    pub needs: Option<String>,
    /// An optional rerunnable verification command (the scriptable form of a
    /// yak's evidence contract). `yaks verify <id>` runs it and records the
    /// PASS/FAIL as an attributed note. Never auto-run — invoked explicitly.
    pub verify: Option<String>,
    /// Verbatim frontmatter lines this binary does not model, captured on parse
    /// and re-emitted on write so a round-trip never drops unknown/newer fields.
    /// Keeps `.yaks/` authoritative across versions. Not rendered.
    pub extra: Vec<String>,
    pub body: String,
}

/// Split raw label input into canonical labels — the one parser every entry
/// point (CLI flags, TUI label fields, filters) shares. A label may not contain
/// a comma or whitespace, so each input string is split on both, trimmed,
/// emptied pieces dropped, and duplicates removed (first occurrence wins).
/// `foo, bar`, `foo bar`, and `foo,bar` all yield `[foo, bar]`. Idempotent.
pub fn normalize_labels<I, S>(raw: I) -> Vec<String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut out: Vec<String> = Vec::new();
    for s in raw {
        for piece in s.as_ref().split(|c: char| c == ',' || c.is_whitespace()) {
            if !piece.is_empty() && !out.iter().any(|l| l == piece) {
                out.push(piece.to_string());
            }
        }
    }
    out
}

/// Whether `label` is already canonical: non-empty, no comma, no whitespace.
/// Legacy on-disk labels that fail this (e.g. `ui,docs`) are flagged by
/// `doctor` and re-split the next time the yak's labels are edited.
pub fn is_canonical_label(label: &str) -> bool {
    !label.is_empty() && !label.contains(|c: char| c == ',' || c.is_whitespace())
}

/// Diff an edited label list against a yak's current labels, as the
/// (add, remove) pair a `TaskEdit` carries. `new` must already be normalized.
/// Compares against the *normalized* current labels, so a legacy `ui,docs`
/// label reads as `ui` + `docs`. Returns `None` when nothing changes; when the
/// only difference is legacy spelling, returns `new` as the add set so the
/// edit re-writes the yak's labels canonically (see `Farm::update`).
pub fn label_edit(cur: &[String], new: &[String]) -> Option<(Vec<String>, Vec<String>)> {
    if cur == new {
        return None;
    }
    let cur_n = normalize_labels(cur);
    let add: Vec<String> = new.iter().filter(|l| !cur_n.contains(l)).cloned().collect();
    let remove: Vec<String> = cur_n.iter().filter(|l| !new.contains(l)).cloned().collect();
    if add.is_empty() && remove.is_empty() {
        if cur_n == new {
            // Only legacy spelling differs: re-assert the labels to canonicalize.
            return Some((new.to_vec(), Vec::new()));
        }
        // Same set, different order: not worth a rewrite.
        return None;
    }
    Some((add, remove))
}

#[cfg(test)]
mod label_tests {
    use super::*;

    fn v(xs: &[&str]) -> Vec<String> {
        xs.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn normalize_splits_on_commas_and_whitespace() {
        for raw in [
            "foo, bar",
            "foo bar",
            "foo,bar",
            " foo ,, bar ,",
            "foo\tbar\n",
        ] {
            assert_eq!(normalize_labels([raw]), v(&["foo", "bar"]), "input {raw:?}");
        }
    }

    #[test]
    fn normalize_flattens_many_args_and_dedupes_in_order() {
        assert_eq!(
            normalize_labels(["b,a", "a", "c b", ""]),
            v(&["b", "a", "c"])
        );
        assert!(normalize_labels(["", " , "]).is_empty());
        // Idempotent.
        let once = normalize_labels(["x, y z"]);
        assert_eq!(normalize_labels(&once), once);
    }

    #[test]
    fn canonical_label_rejects_commas_and_spaces() {
        assert!(is_canonical_label("ui"));
        assert!(is_canonical_label("needs-review"));
        assert!(!is_canonical_label("ui,docs"));
        assert!(!is_canonical_label("bug ui"));
        assert!(!is_canonical_label(""));
    }

    #[test]
    fn label_edit_diffs_against_normalized_current() {
        assert_eq!(label_edit(&v(&["a", "b"]), &v(&["a", "b"])), None);
        assert_eq!(label_edit(&v(&["a", "b"]), &v(&["b", "a"])), None);
        assert_eq!(
            label_edit(&v(&["a", "b"]), &v(&["a", "c"])),
            Some((v(&["c"]), v(&["b"])))
        );
        // Legacy `ui,docs` with an unchanged set: re-assert to canonicalize.
        assert_eq!(
            label_edit(&v(&["ui,docs"]), &v(&["ui", "docs"])),
            Some((v(&["ui", "docs"]), vec![]))
        );
        // Legacy label, dropping one half: remove the normalized piece.
        assert_eq!(
            label_edit(&v(&["ui,docs"]), &v(&["ui"])),
            Some((vec![], v(&["docs"])))
        );
    }
}
