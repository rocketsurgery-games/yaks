//! The agent skills, embedded in the binary so the single self-contained
//! `yaks` can install them without cloning the repo (yaks-45b2). The SKILL.md
//! files are baked in at build time via `include_str!`, so the shipped binary
//! always carries the skill that matches its version.
//!
//! Installed copies carry a **provenance stamp** in their frontmatter (see
//! [`stamp`]). Without one, an installed SKILL.md is an anonymous copy: nothing
//! records which yaks wrote it or what it looked like when written, so "the
//! tool moved on" (yaks-bd9a) can't be told apart from "a human edited this"
//! (yaks-d8e9) and the only available answer is to clobber. The stamp turns
//! that into the four decidable states of [`SkillState`].

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

/// This binary's version, stamped into skills it installs.
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

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
#[allow(dead_code)] // enforced by the spec-compliance test over BUNDLED
pub const SPEC_FRONTMATTER_KEYS: &[&str] = &[
    "name",
    "description",
    "license",
    "compatibility",
    "metadata",
    "allowed-tools",
];

/// Environment variable that disables the startup auto-sync (see [`auto_sync`]).
pub const AUTOSYNC_ENV: &str = "YAKS_SKILLS_AUTOSYNC";

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

// -- provenance -----------------------------------------------------------

/// A 64-bit FNV-1a digest, rendered as 16 lowercase hex chars.
///
/// Hand-rolled on purpose: this only has to answer "did these bytes change
/// since we wrote them", which needs no cryptographic strength, and it keeps
/// yaks a self-contained binary with no hashing dependency (the same reason
/// the frontmatter parser is hand-rolled).
fn digest(s: &str) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in s.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x1000_0000_01b3);
    }
    format!("{h:016x}")
}

/// Key names inside the stamped `metadata:` map.
const K_VERSION: &str = "yaks-version";
const K_DIGEST: &str = "yaks-digest";

/// Render the provenance block: a `metadata:` map holding the writing yaks'
/// version and a digest of the content as written.
///
/// It lives under `metadata:` because that is the one spec field reserved for
/// "additional properties not defined by the Agent Skills spec". Values are
/// quoted strings because the spec defines metadata as a map of string keys to
/// *string* values. It is emitted last in the frontmatter so it can be removed
/// again deterministically by [`read_stamp`].
fn stamp_block(version: &str, source_digest: &str) -> String {
    let mut s = String::from("metadata:\n");
    s.push_str("  ");
    s.push_str(K_VERSION);
    s.push_str(": \"");
    s.push_str(version);
    s.push_str("\"\n  ");
    s.push_str(K_DIGEST);
    s.push_str(": \"");
    s.push_str(source_digest);
    s.push('"');
    s
}

/// Stamp `content` (the pristine embedded skill) for installation at `version`.
///
/// The recorded digest is of `content` itself — the text *before* stamping —
/// so verification is a clean round trip: strip the stamp back off an
/// installed file and re-digest what remains.
pub fn stamp(content: &str, version: &str) -> String {
    let block = stamp_block(version, &digest(content));
    let Some(rest) = content.strip_prefix("---\n") else {
        // No frontmatter to extend; never corrupt the file.
        return content.to_string();
    };
    match rest.find("\n---") {
        Some(i) => {
            let (fm, body) = rest.split_at(i + 1); // the newline stays with fm
            format!("---\n{fm}{block}\n{body}")
        }
        None => content.to_string(),
    }
}

/// Provenance recovered from an installed skill.
pub struct Stamp {
    pub version: String,
    pub digest: String,
}

/// Split an installed skill into its stamp and the content as originally
/// written (the stamp lines removed).
///
/// Returns `None` when the file carries no recognizable stamp — an *unmanaged*
/// file (hand-written, or produced by another tool), which is never clobbered.
fn read_stamp(installed: &str) -> Option<(Stamp, String)> {
    let mut version: Option<String> = None;
    let mut dig: Option<String> = None;
    let mut kept: Vec<&str> = Vec::new();
    let mut fence = 0usize; // how many `---` delimiters seen
    let mut in_block = false; // inside our metadata: block

    for line in installed.lines() {
        let t = line.trim();
        if t == "---" {
            fence += 1;
            in_block = false;
            kept.push(line);
            continue;
        }
        // Only the frontmatter (between fence 1 and 2) can hold the stamp.
        if fence == 1 {
            if t == "metadata:" {
                in_block = true;
                continue; // drop
            }
            if in_block {
                // Our own keys are dropped; anything else ends the block and is
                // kept, so a skill with its own metadata entries is left alone.
                if let Some(v) = t.strip_prefix(K_VERSION).and_then(|r| r.strip_prefix(':')) {
                    version = Some(unquote(v));
                    continue;
                }
                if let Some(v) = t.strip_prefix(K_DIGEST).and_then(|r| r.strip_prefix(':')) {
                    dig = Some(unquote(v));
                    continue;
                }
                in_block = false;
            }
        }
        kept.push(line);
    }

    let (version, digest) = (version?, dig?);
    let mut body = kept.join("\n");
    // `lines()` drops a trailing newline; restore it so the round trip is exact.
    if installed.ends_with('\n') {
        body.push('\n');
    }
    Some((Stamp { version, digest }, body))
}

fn unquote(s: &str) -> String {
    s.trim().trim_matches('"').to_string()
}

/// Compare dotted numeric versions (`0.0.10` > `0.0.9`). Non-numeric or
/// unparseable components compare as 0, so a weird version never wins.
fn version_gt(a: &str, b: &str) -> bool {
    let parts = |v: &str| -> Vec<u64> {
        v.split(['.', '-', '+'])
            .map(|p| p.parse::<u64>().unwrap_or(0))
            .collect()
    };
    let (x, y) = (parts(a), parts(b));
    for i in 0..x.len().max(y.len()) {
        let (l, r) = (
            x.get(i).copied().unwrap_or(0),
            y.get(i).copied().unwrap_or(0),
        );
        if l != r {
            return l > r;
        }
    }
    false
}

// -- state ----------------------------------------------------------------

/// What an installed skill looks like relative to the binary's embedded copy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkillState {
    /// Nothing installed at that path.
    Absent,
    /// Byte-identical to what this binary would write. Nothing to do.
    Current,
    /// Untouched since we wrote it, and this binary is strictly newer — the
    /// only state that is safe to upgrade automatically.
    Upgradable { from: String },
    /// Untouched since we wrote it, but this binary is *not* newer while the
    /// content differs. Left alone deliberately: auto-writing here is what
    /// makes two co-installed binaries (a release on PATH and a dev build)
    /// ping-pong, each "fixing" the other forever.
    Held { installed: String },
    /// Edited since we wrote it. Never overwritten without an explicit force.
    Modified { installed: String },
    /// Present with no stamp, but byte-identical to what this binary would
    /// write: a pre-stamp install that nobody has touched. Adopting it (writing
    /// the same content, now stamped) destroys nothing, so it needs no force.
    ///
    /// Without this, every skill installed before stamping existed would sit
    /// `Unmanaged` forever and auto-sync could never help it — which is most of
    /// the installed base, and the whole point of the feature.
    Adoptable,
    /// Present, unstamped, and *not* what we would write — hand-written, or
    /// another tool's. Treated like `Modified`: never overwritten without an
    /// explicit force.
    Unmanaged,
    /// The target resolves into a yaks checkout's own `skills/` — typically a
    /// `~/.agents/skills/yaks` symlink pointing at the repo, a common dev
    /// setup. There is nothing to install: the installed skill *is* the source.
    /// Never written, not even with `--force` (yaks-d8e9).
    SourceLinked,
}

impl SkillState {
    /// True when writing would override something this binary has no business
    /// overwriting silently: a local edit, another tool's file, or an install
    /// from a yaks at least as new as this one (a downgrade).
    pub fn needs_force(&self) -> bool {
        matches!(
            self,
            SkillState::Modified { .. }
                | SkillState::Unmanaged
                | SkillState::Held { .. }
                | SkillState::SourceLinked
        )
    }

    /// True when the blocker is local content we'd destroy, as opposed to a
    /// merely newer install.
    pub fn has_local_edits(&self) -> bool {
        matches!(self, SkillState::Modified { .. } | SkillState::Unmanaged)
    }

    /// Short word for CLI output.
    pub fn word(&self) -> &'static str {
        match self {
            SkillState::Absent => "absent",
            SkillState::Current => "current",
            SkillState::Upgradable { .. } => "stale",
            SkillState::Held { .. } => "held",
            SkillState::Modified { .. } => "modified",
            SkillState::Adoptable => "adoptable",
            SkillState::Unmanaged => "unmanaged",
            SkillState::SourceLinked => "source",
        }
    }
}

/// Classify the installed copy of `content` at `path`.
pub fn inspect(path: &Path, content: &str) -> SkillState {
    // Checked first: writing here would rewrite the source of truth, which is
    // true regardless of what the file currently contains.
    if path.parent().is_some_and(is_source_tree) {
        return SkillState::SourceLinked;
    }
    let Ok(installed) = std::fs::read_to_string(path) else {
        return SkillState::Absent;
    };
    let Some((s, written)) = read_stamp(&installed) else {
        // Unstamped. If it is exactly what we would install anyway, stamping it
        // is lossless; otherwise it is someone else's file.
        return if installed == content {
            SkillState::Adoptable
        } else {
            SkillState::Unmanaged
        };
    };
    if digest(&written) != s.digest {
        return SkillState::Modified {
            installed: s.version,
        };
    }
    if written == content {
        return SkillState::Current;
    }
    if version_gt(version(), &s.version) {
        SkillState::Upgradable { from: s.version }
    } else {
        SkillState::Held {
            installed: s.version,
        }
    }
}

/// One skill's situation in a skills directory.
pub struct Status {
    pub name: String,
    pub path: PathBuf,
    pub state: SkillState,
}

/// Classify every bundled skill in `base`.
pub fn status(base: &Path) -> Vec<Status> {
    BUNDLED
        .iter()
        .map(|(name, content)| {
            let path = base.join(name).join("SKILL.md");
            let state = inspect(&path, content);
            Status {
                name: (*name).to_string(),
                path,
                state,
            }
        })
        .collect()
}

// -- the source-tree guard ------------------------------------------------

/// True when `dir` is a `skills/` directory belonging to a yaks source tree.
fn is_yaks_skills_dir(dir: &Path) -> bool {
    if dir.file_name().and_then(|s| s.to_str()) != Some("skills") {
        return false;
    }
    let Some(root) = dir.parent() else {
        return false;
    };
    let Ok(text) = std::fs::read_to_string(root.join("Cargo.toml")) else {
        return false;
    };
    text.lines()
        .any(|l| l.trim().replace(' ', "") == "name=\"yaks\"")
}

/// True when `path` — **after following symlinks** — lives inside a yaks source
/// tree's `skills/` directory.
///
/// Writing there overwrites the *source of truth* with the binary's baked-in
/// copy: it silently reverts real edits and, in `git status`, looks exactly
/// like an authored change (yaks-d8e9, which happened twice).
///
/// Resolving symlinks is the whole point. A common dev setup symlinks
/// `~/.agents/skills/yaks` at the repo's `skills/yaks`, so a perfectly
/// innocent-looking `yaks skills install` (default dir, no `--dir` at all)
/// lands on the source through the link. Checking only the path we were handed
/// misses that entirely — which is exactly how the original incident happened.
pub fn is_source_tree(path: &Path) -> bool {
    // Resolve as much of the path as exists; a not-yet-created target can't be
    // a symlink to anything, so the literal path is the right fallback.
    let resolved = path.canonicalize().unwrap_or_else(|_| {
        match path.parent().and_then(|p| p.canonicalize().ok()) {
            Some(parent) => match path.file_name() {
                Some(name) => parent.join(name),
                None => path.to_path_buf(),
            },
            None => path.to_path_buf(),
        }
    });
    resolved.ancestors().any(is_yaks_skills_dir)
}

// -- install --------------------------------------------------------------

/// Result of considering one bundled skill for installation.
pub struct Installed {
    pub name: String,
    pub path: PathBuf,
    /// What the target looked like before we acted.
    pub before: SkillState,
    /// Whether the file was (re)written.
    pub wrote: bool,
}

impl Installed {
    /// True when the file was left alone because it would need `--force`.
    pub fn blocked(&self) -> bool {
        !self.wrote && self.before.needs_force()
    }
}

/// Write `content` to `path` atomically, so parallel `yaks` invocations (the
/// coordinator spawns many) can never observe or leave a half-written file.
fn write_atomic(path: &Path, content: &str) -> Result<()> {
    let tmp = path.with_extension(format!("tmp{}", std::process::id()));
    std::fs::write(&tmp, content).with_context(|| format!("writing {}", tmp.display()))?;
    std::fs::rename(&tmp, path).with_context(|| format!("installing {}", path.display()))?;
    Ok(())
}

/// Install the bundled skills into `base`.
///
/// Files we wrote and that are still untouched are upgraded freely. A
/// `Modified` or `Unmanaged` file is left alone unless `force` is set. Writing
/// into this repo's own `skills/` is refused outright (see [`is_source_tree`]).
pub fn install(base: &Path, force: bool) -> Result<Vec<Installed>> {
    if is_source_tree(base) {
        anyhow::bail!(
            "refusing to install into {} — that is yaks' own skills/ source, \
             and overwriting it would revert the real files to this binary's \
             baked-in copy (yaks-d8e9). Pick a skills directory instead, e.g. \
             `yaks skills install` or `--dir ~/.claude/skills`.",
            base.display()
        );
    }
    let mut out = Vec::new();
    for (name, content) in BUNDLED {
        let dir = base.join(name);
        let path = dir.join("SKILL.md");
        let before = inspect(&path, content);
        let wrote = match &before {
            SkillState::Current => false,
            // Never written, force or not: it resolves onto the source.
            SkillState::SourceLinked => false,
            s if s.needs_force() && !force => false,
            _ => {
                std::fs::create_dir_all(&dir)
                    .with_context(|| format!("creating {}", dir.display()))?;
                write_atomic(&path, &stamp(content, version()))?;
                true
            }
        };
        out.push(Installed {
            name: (*name).to_string(),
            path,
            before,
            wrote,
        });
    }
    Ok(out)
}

/// Bring the user-level skills directory up to date on startup, quietly.
///
/// The overwhelmingly common failure is running against stale skills, so this
/// runs on ordinary `yaks` invocations. It is deliberately narrow:
///
/// - only `Absent` and `Upgradable` are written — never `Modified`,
///   `Unmanaged`, or `Held`, so it can neither clobber your edits nor
///   downgrade a newer install;
/// - only the user-level [`default_dir`], never a project-local
///   `.agents/skills` (that would be writing into someone's repo) and never a
///   `--dir` target;
/// - skipped entirely when [`AUTOSYNC_ENV`] is `0`/`false`/`never`, for CI and
///   sandboxes.
///
/// Returns the names it wrote, for a one-line notice; empty means it did
/// nothing, which is the normal case.
pub fn auto_sync() -> Vec<String> {
    match std::env::var(AUTOSYNC_ENV) {
        Ok(v) if matches!(v.trim(), "0" | "false" | "never") => return Vec::new(),
        _ => {}
    }
    let base = default_dir();
    if is_source_tree(&base) {
        return Vec::new();
    }
    let mut done = Vec::new();
    for (name, content) in BUNDLED {
        let dir = base.join(name);
        let path = dir.join("SKILL.md");
        // Never write through a symlink into a yaks checkout (see install).
        if is_source_tree(&dir) {
            continue;
        }
        match inspect(&path, content) {
            SkillState::Absent | SkillState::Upgradable { .. } | SkillState::Adoptable => {
                if std::fs::create_dir_all(&dir).is_ok()
                    && write_atomic(&path, &stamp(content, version())).is_ok()
                {
                    done.push((*name).to_string());
                }
            }
            // Current / Held / Modified / Unmanaged: leave it alone.
            _ => {}
        }
    }
    done
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
            for required in ["name", "description"] {
                assert!(
                    keys.iter().any(|k| k == required),
                    "skill {name:?} is missing required frontmatter key {required:?}"
                );
            }
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
    fn bundled_source_is_never_itself_stamped() {
        // The embedded skills must be PRISTINE: a stamp belongs only to an
        // installed copy. If a stamp ever leaks back into skills/*/SKILL.md,
        // the build embeds it, every install double-stamps, and `inspect`
        // starts comparing stamped content against stamped content. It has
        // happened (a stray install pointed at the repo), so assert it loudly
        // rather than trusting the write-path guard alone.
        for (name, content) in BUNDLED {
            assert!(
                read_stamp(content).is_none(),
                "skills/{name}/SKILL.md carries a provenance stamp; the source \
                 must stay unstamped \u{2014} run `git checkout -- skills/`"
            );
            assert!(
                !content.contains(K_VERSION) && !content.contains(K_DIGEST),
                "skills/{name}/SKILL.md mentions a stamp key; the source must stay pristine"
            );
        }
    }

    #[test]
    fn stamp_round_trips_and_stays_spec_legal() {
        let src = BUNDLED[0].1;
        let out = stamp(src, "1.2.3");
        // The stamp lands in the frontmatter, under the spec's metadata: field.
        assert!(out.contains("metadata:\n  yaks-version: \"1.2.3\""));
        assert_eq!(frontmatter_keys(&out).last().unwrap(), "metadata");
        for k in frontmatter_keys(&out) {
            assert!(SPEC_FRONTMATTER_KEYS.contains(&k.as_str()), "key {k:?}");
        }
        // Stripping it back recovers the pristine source exactly.
        let (s, written) = read_stamp(&out).expect("stamped file parses");
        assert_eq!(s.version, "1.2.3");
        assert_eq!(written, src);
        assert_eq!(s.digest, digest(src));
    }

    #[test]
    fn version_comparison_is_numeric_not_lexical() {
        assert!(version_gt("0.0.10", "0.0.9")); // the lexical trap
        assert!(version_gt("0.1.0", "0.0.99"));
        assert!(!version_gt("0.0.9", "0.0.9"));
        assert!(!version_gt("0.0.9", "0.0.10"));
    }

    fn temp_base(tag: &str) -> PathBuf {
        let mut base = std::env::temp_dir();
        base.push(format!(
            "yaks-skills-{tag}-{}-{:?}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_dir_all(&base);
        base
    }

    #[test]
    fn states_absent_current_modified_and_unmanaged() {
        let base = temp_base("states");
        let (name, content) = BUNDLED[0];
        let path = base.join(name).join("SKILL.md");

        assert_eq!(inspect(&path, content), SkillState::Absent);

        install(&base, false).unwrap();
        assert_eq!(inspect(&path, content), SkillState::Current);

        // A hand edit is detected and protected.
        let edited = std::fs::read_to_string(&path).unwrap() + "\nlocal note\n";
        std::fs::write(&path, &edited).unwrap();
        assert!(matches!(
            inspect(&path, content),
            SkillState::Modified { .. }
        ));
        let res = install(&base, false).unwrap();
        let me = res.iter().find(|i| i.name == name).unwrap();
        assert!(!me.wrote && me.blocked(), "modified file is protected");
        assert_eq!(std::fs::read_to_string(&path).unwrap(), edited);
        // ...until forced.
        let res = install(&base, true).unwrap();
        assert!(res.iter().find(|i| i.name == name).unwrap().wrote);
        assert_eq!(inspect(&path, content), SkillState::Current);

        // An unstamped file is someone else's; also protected.
        std::fs::write(&path, "---\nname: yaks\ndescription: hand rolled\n---\n").unwrap();
        assert_eq!(inspect(&path, content), SkillState::Unmanaged);
        let res = install(&base, false).unwrap();
        assert!(res.iter().find(|i| i.name == name).unwrap().blocked());

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn untouched_prestamp_install_is_adopted_without_force() {
        // The migration path: skills installed before stamping existed carry no
        // stamp. When the content is still exactly ours, adopting it is
        // lossless, so it must not demand --force -- otherwise the whole
        // installed base stays unmanaged and auto-sync never helps it.
        let base = temp_base("adopt");
        let (name, content) = BUNDLED[0];
        let dir = base.join(name);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("SKILL.md");
        // Exactly what an old yaks would have written: our content, no stamp.
        std::fs::write(&path, content).unwrap();
        assert_eq!(inspect(&path, content), SkillState::Adoptable);
        assert!(!inspect(&path, content).needs_force());

        let res = install(&base, false).unwrap();
        assert!(
            res.iter().find(|i| i.name == name).unwrap().wrote,
            "an untouched pre-stamp install adopts with no --force"
        );
        assert_eq!(inspect(&path, content), SkillState::Current);

        // But a DIVERGENT unstamped file is someone else's and stays protected.
        std::fs::write(&path, "---\nname: yaks\ndescription: theirs\n---\n").unwrap();
        assert_eq!(inspect(&path, content), SkillState::Unmanaged);
        assert!(inspect(&path, content).needs_force());

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn older_binary_holds_instead_of_downgrading() {
        // The ping-pong guard (yaks-1d51): a file stamped by a NEWER yaks must
        // not be rewritten by this one, or two co-installed binaries flip-flop.
        let base = temp_base("hold");
        let (name, content) = BUNDLED[0];
        let dir = base.join(name);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("SKILL.md");
        // Stamp altered content at an implausibly high version.
        let future = format!("{content}\n<!-- from the future -->\n");
        std::fs::write(&path, stamp(&future, "99.0.0")).unwrap();

        assert!(matches!(inspect(&path, content), SkillState::Held { .. }));
        let before = std::fs::read_to_string(&path).unwrap();
        // Neither auto-sync nor a plain install may touch it.
        let res = install(&base, false).unwrap();
        assert!(!res.iter().find(|i| i.name == name).unwrap().wrote);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), before);

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn stale_install_is_upgraded_without_force() {
        let base = temp_base("stale");
        let (name, content) = BUNDLED[0];
        let dir = base.join(name);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("SKILL.md");
        // An old, unmodified install of different content.
        let old = format!("{content}\n<!-- old -->\n");
        std::fs::write(&path, stamp(&old, "0.0.0")).unwrap();

        assert!(matches!(
            inspect(&path, content),
            SkillState::Upgradable { .. }
        ));
        let res = install(&base, false).unwrap();
        assert!(
            res.iter().find(|i| i.name == name).unwrap().wrote,
            "a clean stale copy upgrades with no --force"
        );
        assert_eq!(inspect(&path, content), SkillState::Current);

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn refuses_to_install_into_the_yaks_source_tree() {
        // yaks-d8e9: not bypassable by --force, since --force caused it.
        let base = temp_base("srctree").join("skills");
        std::fs::create_dir_all(&base).unwrap();
        std::fs::write(
            base.parent().unwrap().join("Cargo.toml"),
            "[package]\nname = \"yaks\"\nversion = \"0.0.1\"\n",
        )
        .unwrap();
        assert!(is_source_tree(&base));
        assert!(install(&base, false).is_err());
        assert!(
            install(&base, true).is_err(),
            "--force must NOT be the escape hatch here"
        );
        let _ = std::fs::remove_dir_all(base.parent().unwrap());
    }

    #[test]
    fn install_writes_both_then_is_idempotent_then_forces() {
        let base = temp_base("install");

        let first = install(&base, false).unwrap();
        assert_eq!(first.len(), 2);
        assert!(first.iter().all(|i| i.wrote));
        assert!(base.join("yaks/SKILL.md").is_file());
        assert!(base.join("yaks-tracker/SKILL.md").is_file());
        let yak = std::fs::read_to_string(base.join("yaks/SKILL.md")).unwrap();
        assert!(yak.contains("name: yaks"));

        // Second run is a no-op: identical content needs no rewrite.
        let again = install(&base, false).unwrap();
        assert!(
            again
                .iter()
                .all(|i| !i.wrote && i.before == SkillState::Current)
        );

        // Force on an identical file stays a no-op: there is nothing to
        // refresh, and rewriting would only churn mtimes.
        let forced = install(&base, true).unwrap();
        assert!(forced.iter().all(|i| !i.wrote));

        // And status agrees.
        assert!(status(&base).iter().all(|s| s.state == SkillState::Current));

        let _ = std::fs::remove_dir_all(&base);
    }
}

/// The symlink case gets its own module so the guard that actually failed in
/// production is impossible to lose in a refactor of the big test module.
#[cfg(all(test, unix))]
mod symlink_guard_tests {
    use super::*;

    /// The real mechanism behind yaks-d8e9, found the hard way: a dev setup
    /// symlinks `~/.agents/skills/yaks` at the repo's `skills/yaks`, so a plain
    /// install with **no `--dir`** lands on the source through the link.
    /// Guarding only the literal path we were handed misses this entirely.
    #[test]
    fn a_symlink_onto_the_source_tree_is_never_written() {
        let mut tmp = std::env::temp_dir();
        tmp.push(format!("yaks-srclink-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);

        // A fake yaks checkout, with a precious source file.
        let src_dir = tmp.join("repo").join("skills").join("yaks");
        std::fs::create_dir_all(&src_dir).unwrap();
        std::fs::write(
            tmp.join("repo").join("Cargo.toml"),
            "[package]\nname = \"yaks\"\n",
        )
        .unwrap();
        let precious = src_dir.join("SKILL.md");
        std::fs::write(&precious, "PRECIOUS SOURCE").unwrap();

        // An innocent-looking skills dir whose `yaks` entry is a symlink to it.
        let base = tmp.join("home").join(".agents").join("skills");
        std::fs::create_dir_all(&base).unwrap();
        std::os::unix::fs::symlink(&src_dir, base.join("yaks")).unwrap();

        // The base itself is NOT a source tree; only the resolved entry is.
        assert!(!is_source_tree(&base), "base looks innocent");
        assert_eq!(
            inspect(&base.join("yaks").join("SKILL.md"), BUNDLED[0].1),
            SkillState::SourceLinked,
        );

        // Neither a plain install nor a forced one may write through the link.
        for force in [false, true] {
            install(&base, force).unwrap();
            assert_eq!(
                std::fs::read_to_string(&precious).unwrap(),
                "PRECIOUS SOURCE",
                "source overwritten through the symlink (force={force})"
            );
        }
        // Auto-sync must not either (it targets the user dir, so prove the
        // state it keys off is the protective one).
        assert!(
            inspect(&base.join("yaks").join("SKILL.md"), BUNDLED[0].1).needs_force(),
            "source-linked must never be auto-written"
        );

        let _ = std::fs::remove_dir_all(&tmp);
    }
}
