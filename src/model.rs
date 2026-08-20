//! Core task model for the yaks-rs Phase 0 spike.
//!
//! Deliberately mirrors the Python `yaklib.model` shapes closely enough to
//! read the *same* `.yaks/` files (interop is the whole point of Phase 0).
//! Status is implicit from the directory a task file lives in; parentage is a
//! frontmatter field (flat, stable ids).

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
            Status::Shorn => 'C', // Complete
            Status::Dead => 'X',
        }
    }

    /// A dependency counts as "resolved" once it is shorn or dead.
    pub fn is_resolved(self) -> bool {
        matches!(self, Status::Shorn | Status::Dead)
    }
}

/// A single task, parsed from a `.md` file with YAML frontmatter.
#[derive(Debug, Clone)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub kind: String,
    pub priority: u8,
    pub status: Status,
    pub parent: Option<String>,
    pub labels: Vec<String>,
    pub depends_on: Vec<String>,
    pub source: Option<String>,
    pub body: String,
}

impl Task {
    /// One-line summary for `list` / `next`.
    pub fn summary(&self) -> String {
        let mut extra = String::new();
        if !self.labels.is_empty() {
            extra.push_str(&format!(" [{}]", self.labels.join(",")));
        }
        if !self.depends_on.is_empty() {
            extra.push_str(&format!(" (deps: {})", self.depends_on.join(",")));
        }
        format!(
            "[{}] {:<9} p{} {:<8} {}{}",
            self.status.glyph(),
            self.id,
            self.priority,
            self.kind,
            self.title,
            extra
        )
    }
}
