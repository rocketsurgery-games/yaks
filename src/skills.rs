//! The agent skills, embedded in the binary so the single self-contained
//! `yaks` can install them without cloning the repo (yaks-45b2). The SKILL.md
//! files are baked in at build time via `include_str!`, so the shipped binary
//! always carries the skill that matches its version.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

/// `(name, SKILL.md content)` for each bundled skill. Paths are relative to this
/// source file (`src/skills.rs`), i.e. the repo-root `skills/` directory.
const BUNDLED: &[(&str, &str)] = &[
    ("yak", include_str!("../skills/yak/SKILL.md")),
    (
        "yak-tracker",
        include_str!("../skills/yak-tracker/SKILL.md"),
    ),
];

/// Default install target: `~/.agents/skills`. Overridable so other agents'
/// skills directories (e.g. `~/.claude/skills`) can be targeted.
pub fn default_dir() -> PathBuf {
    for var in ["HOME", "USERPROFILE"] {
        if let Ok(home) = std::env::var(var) {
            if !home.is_empty() {
                return PathBuf::from(home).join(".agents").join("skills");
            }
        }
    }
    PathBuf::from(".agents").join("skills")
}

/// Expand a leading `~/` in a user-supplied path against `$HOME`.
pub fn expand_tilde(p: &str) -> PathBuf {
    if let Some(rest) = p.strip_prefix("~/") {
        if let Ok(home) = std::env::var("HOME") {
            if !home.is_empty() {
                return PathBuf::from(home).join(rest);
            }
        }
    }
    PathBuf::from(p)
}

/// Result of installing one bundled skill.
pub struct Installed {
    pub name: String,
    pub path: PathBuf,
    /// True when an existing SKILL.md was left in place (no `force`).
    pub skipped: bool,
}

/// Write each bundled skill to `<base>/<name>/SKILL.md`. An existing SKILL.md is
/// left untouched unless `force` is set.
pub fn install(base: &Path, force: bool) -> Result<Vec<Installed>> {
    let mut out = Vec::new();
    for (name, content) in BUNDLED {
        let dir = base.join(name);
        let path = dir.join("SKILL.md");
        if path.exists() && !force {
            out.push(Installed {
                name: (*name).to_string(),
                path,
                skipped: true,
            });
            continue;
        }
        std::fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
        std::fs::write(&path, content).with_context(|| format!("writing {}", path.display()))?;
        out.push(Installed {
            name: (*name).to_string(),
            path,
            skipped: false,
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn install_writes_both_then_skips_then_forces() {
        let mut base = std::env::temp_dir();
        base.push(format!("yaks-skills-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);

        let first = install(&base, false).unwrap();
        assert_eq!(first.len(), 2);
        assert!(first.iter().all(|i| !i.skipped));
        assert!(base.join("yak/SKILL.md").is_file());
        assert!(base.join("yak-tracker/SKILL.md").is_file());
        // The embedded content is the real skill (its frontmatter name).
        let yak = std::fs::read_to_string(base.join("yak/SKILL.md")).unwrap();
        assert!(yak.contains("name: yak"));

        // Second run without force leaves the files alone.
        let again = install(&base, false).unwrap();
        assert!(again.iter().all(|i| i.skipped));

        // With force, they are rewritten.
        let forced = install(&base, true).unwrap();
        assert!(forced.iter().all(|i| !i.skipped));

        let _ = std::fs::remove_dir_all(&base);
    }
}
