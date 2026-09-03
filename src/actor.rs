//! Resolving the actor a note or block should be attributed to. Shared by the
//! CLI (`update --as`, `ask`/`answer`) and the TUI (auto-attributed comments) so
//! both surfaces agree on one precedence: an explicit value wins, then
//! `$YAKS_ACTOR` (a harness/coordinator can pin it once per worker), then the git
//! `user.name` (free attribution for interactive human use). `None` writes a
//! bare, unattributed note. Attribution, never ownership.

/// Trim to a non-empty owned string, or `None`.
fn clean(s: &str) -> Option<String> {
    let t = s.trim();
    (!t.is_empty()).then(|| t.to_string())
}

/// Resolve the actor: `--as`/explicit → `$YAKS_ACTOR` → git `user.name`.
/// Best-effort — never fails, returning `None` when nothing is configured.
/// Note this defaults an agent running under the human's git identity to the
/// human's name unless it sets `--as`/`$YAKS_ACTOR`.
pub fn resolve(explicit: Option<&str>) -> Option<String> {
    resolve_with(
        explicit,
        std::env::var("YAKS_ACTOR").ok().as_deref(),
        git_user_name,
    )
}

/// Precedence core, decoupled from the environment and git for testability.
fn resolve_with(
    explicit: Option<&str>,
    env: Option<&str>,
    git: impl FnOnce() -> Option<String>,
) -> Option<String> {
    explicit
        .and_then(clean)
        .or_else(|| env.and_then(clean))
        .or_else(git)
}

/// The git `user.name`, or `None` if git is absent/unconfigured.
fn git_user_name() -> Option<String> {
    let out = std::process::Command::new("git")
        .args(["config", "user.name"])
        .output()
        .ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).into_owned())
        .as_deref()
        .and_then(clean)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_beats_env_beats_git() {
        let git = || Some("git-name".to_string());
        // Explicit wins.
        assert_eq!(
            resolve_with(Some("flag"), Some("env"), git),
            Some("flag".into())
        );
        // Empty/whitespace explicit falls through to env.
        assert_eq!(
            resolve_with(Some("  "), Some("env"), git),
            Some("env".into())
        );
        // No explicit, no env -> git.
        assert_eq!(resolve_with(None, None, git), Some("git-name".into()));
        // Nothing anywhere -> None (bare, unattributed note).
        assert_eq!(resolve_with(None, None, || None), None);
    }
}
