//! The agent skills, embedded in the binary so the single self-contained
//! `yaks` can install them without cloning the repo (yaks-45b2). The SKILL.md
//! files are baked in at build time via `include_str!`, so the shipped binary
//! always carries the skill that matches its version.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

/// `(name, SKILL.md content)` for each bundled skill. Paths are relative to this
/// source file (`src/skills.rs`), i.e. the repo-root `skills/` directory.
const BUNDLED: &[(&str, &str)] = &[
    ("yaks", include_str!("../skills/yaks/SKILL.md")),
    (
        "yaks-tracker",
        include_str!("../skills/yaks-tracker/SKILL.md"),
    ),
];

/// The complete set of frontmatter keys the Agent Skills specification allows
/// (<https://agentskills.io/specification>). `name` and `description` are
/// required; the rest are optional.
///
/// This matters because the leniency is asymmetric: Claude Code silently
/// *ignores* an unrecognized key, but claude.ai upload, the Skills API, and
/// packaging with `package_skill.py` reject one with a hard error — so a
/// non-spec key is invisible locally and fatal downstream. That is exactly how
/// a bogus `activation:` key rode along in both bundled skills (yaks-e233).
/// Custom data belongs under `metadata:`, which the spec reserves for it.
#[allow(dead_code)] // consumed by the spec-compliance test (and `skills status`)
pub const SPEC_FRONTMATTER_KEYS: &[&str] = &[
    "name",
    "description",
    "license",
    "compatibility",
    "metadata",
    "allowed-tools",
];

/// Default install target: `~/.agents/skills`. Overridable so other agents'
/// skills directories (e.g. `~/.claude/skills`) can be targeted.
///
/// `~/.agents/skills` is the cross-client interoperability convention the
/// Agent Skills client-implementation guide tells agents to scan, alongside
/// their own native directory — so it is the widest-reach default. (The spec
/// itself mandates no install location.) Claude Code reads `~/.claude/skills`
/// and does *not* scan `.agents`, which is why `--dir` stays necessary.
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

    /// Top-level frontmatter keys of a SKILL.md (the `key:` lines between the
    /// opening and closing `---`, ignoring nested/indented lines and comments).
    fn frontmatter_keys(src: &str) -> Vec<String> {
        let mut lines = src.lines();
        assert_eq!(
            lines.next().map(str::trim),
            Some("---"),
            "SKILL.md must open with frontmatter"
        );
        let mut keys = Vec::new();
        for line in lines {
            if line.trim() == "---" {
                break;
            }
            // Only top-level keys: skip indented (nested) lines and comments.
            if line.starts_with(' ') || line.starts_with('\t') || line.trim_start().starts_with('#')
            {
                continue;
            }
            if let Some((k, _)) = line.split_once(':') {
                keys.push(k.trim().to_string());
            }
        }
        keys
    }

    #[test]
    fn bundled_skills_use_only_spec_frontmatter_keys() {
        // Guards yaks-e233: a non-spec key (we shipped `activation:`) is
        // silently ignored by Claude Code but HARD-ERRORS on claude.ai upload /
        // the Skills API / package_skill.py, so it can't be caught by using the
        // skill locally. Put custom data under `metadata:` instead.
        for (name, content) in BUNDLED {
            let keys = frontmatter_keys(content);
            for k in &keys {
                assert!(
                    SPEC_FRONTMATTER_KEYS.contains(&k.as_str()),
                    "skill {name:?} has non-spec frontmatter key {k:?}; \
                     allowed: {SPEC_FRONTMATTER_KEYS:?} (put custom data under `metadata:`)"
                );
            }
            // The two required fields must be present.
            for required in ["name", "description"] {
                assert!(
                    keys.iter().any(|k| k == required),
                    "skill {name:?} is missing required frontmatter key {required:?}"
                );
            }
            // The spec requires `name` to match the skill's directory name.
            let declared = content
                .lines()
                .find_map(|l| l.strip_prefix("name:"))
                .map(str::trim)
                .unwrap_or_default();
            assert_eq!(
                declared, *name,
                "skill {name:?} declares a `name` that doesn't match its directory"
            );
        }
    }

    #[test]
    fn install_writes_both_then_skips_then_forces() {
        let mut base = std::env::temp_dir();
        base.push(format!("yaks-skills-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);

        let first = install(&base, false).unwrap();
        assert_eq!(first.len(), 2);
        assert!(first.iter().all(|i| !i.skipped));
        assert!(base.join("yaks/SKILL.md").is_file());
        assert!(base.join("yaks-tracker/SKILL.md").is_file());
        // The embedded content is the real skill (its frontmatter name).
        let yak = std::fs::read_to_string(base.join("yaks/SKILL.md")).unwrap();
        assert!(yak.contains("name: yaks"));

        // Second run without force leaves the files alone.
        let again = install(&base, false).unwrap();
        assert!(again.iter().all(|i| i.skipped));

        // With force, they are rewritten.
        let forced = install(&base, true).unwrap();
        assert!(forced.iter().all(|i| !i.skipped));

        let _ = std::fs::remove_dir_all(&base);
    }
}
