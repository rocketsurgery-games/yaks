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
    /// Verbatim frontmatter lines this binary does not model, captured on parse
    /// and re-emitted on write so a round-trip never drops unknown/newer fields.
    /// Keeps `.yaks/` authoritative across versions. Not rendered.
    pub extra: Vec<String>,
    pub body: String,
}
