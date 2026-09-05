//! Golden snapshot tests for read commands over a fixed fixture herd
//! (tests/fixtures/herd). Snapshots are yaks's own goldens (semantic).
//! Regenerate with: INSTA_UPDATE=always cargo test.
//! These double as assert_cmd smoke tests (yaksrs-c725).

use assert_cmd::Command;
use std::path::PathBuf;

fn run(args: &[&str]) -> String {
    let herd = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/herd");
    let out = Command::cargo_bin("yaks")
        .unwrap()
        .current_dir(herd)
        .args(args)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "command {args:?} failed: {:?}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap()
}

macro_rules! snap {
    ($name:literal, $args:expr) => {
        insta::assert_snapshot!($name, run($args))
    };
}

/// Drive the headless TUI over the fixture herd with isolated XDG dirs (so
/// persisted views/cache can't make the output machine-dependent).
fn run_headless(args: &[&str], stdin: &str) -> String {
    let herd = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/herd");
    let xdg = std::env::temp_dir().join(format!("yaksrs-headless-{}", std::process::id()));
    let out = Command::cargo_bin("yaks")
        .unwrap()
        .current_dir(herd)
        .env("XDG_CONFIG_HOME", &xdg)
        .env("XDG_CACHE_HOME", &xdg)
        .args(args)
        .write_stdin(stdin)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "headless {args:?} failed: {:?}",
        String::from_utf8_lossy(&out.stderr)
    );
    let _ = std::fs::remove_dir_all(&xdg);
    String::from_utf8(out.stdout).unwrap()
}

#[test]
fn headless_session() {
    // Initial frame + one cursor move; deterministic under isolated XDG.
    let out = run_headless(&["tui", "--headless", "--size", "72x12"], "key j\nquit\n");
    insta::assert_snapshot!("headless_session", out);
}

#[test]
fn headless_state_header_tracks_focus() {
    let out = run_headless(&["tui", "--headless", "--size", "72x12"], "key l\nquit\n");
    // Entering the detail pane flips the state header's focus field.
    assert!(out.contains("focus=list"));
    assert!(out.contains("focus=detail"));
}

#[test]
fn list() {
    snap!("list", &["list"]);
}
#[test]
fn list_all() {
    snap!("list_all", &["list", "--all"]);
}
#[test]
fn refs() {
    // fix-0004 points at a resolved parent (fix-0003) and dependency (fix-0002).
    snap!("refs", &["refs", "fix-0004"]);
}
#[test]
fn list_json() {
    snap!("list_json", &["list", "--json"]);
}
#[test]
fn list_filtered() {
    snap!(
        "list_filtered",
        &[
            "list",
            "--type",
            "feature",
            "--priority",
            "1",
            "--priority",
            "2"
        ]
    );
}
#[test]
fn next() {
    snap!("next", &["next"]);
}
#[test]
fn next_json() {
    snap!("next_json", &["next", "--json"]);
}
#[test]
fn tangled() {
    snap!("tangled", &["tangled"]);
}
#[test]
fn tangled_json() {
    snap!("tangled_json", &["tangled", "--json"]);
}
#[test]
fn search() {
    snap!("search", &["search", "child"]);
}
#[test]
fn stats() {
    snap!("stats", &["stats"]);
}
#[test]
fn stats_json() {
    snap!("stats_json", &["stats", "--json"]);
}
#[test]
fn show() {
    snap!("show", &["show", "fix-0003"]);
}
#[test]
fn show_json() {
    snap!("show_json", &["show", "fix-0003", "--json"]);
}
#[test]
fn parent_of() {
    snap!("parent_of", &["list", "--parent-of", "fix-0003"]);
}
#[test]
fn rollup() {
    snap!("rollup", &["rollup"]);
}
#[test]
fn rollup_json() {
    snap!("rollup_json", &["rollup", "--json"]);
}
#[test]
fn rollup_keys() {
    snap!("rollup_keys", &["rollup", "--keys"]);
}

/// `list --needs` selects only yaks blocked on a human. Uses a throwaway herd
/// built via the CLI so it never depends on the shared fixture (yaks-f81a).
#[test]
fn list_needs_filters_to_blocked() {
    let dir = std::env::temp_dir().join(format!("yaks-needs-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let cli = |args: &[&str]| -> String {
        let out = Command::cargo_bin("yaks")
            .unwrap()
            .current_dir(&dir)
            .args(args)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "command {args:?} failed: {:?}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8(out.stdout).unwrap()
    };

    // `Created <id>: <title>` -> id.
    let created_id = |out: &str| -> String {
        out.lines()
            .find_map(|l| l.strip_prefix("Created "))
            .and_then(|rest| rest.split(':').next())
            .unwrap()
            .trim()
            .to_string()
    };

    cli(&["init"]);
    let blocked = created_id(&cli(&["create", "--title", "needs a decision"]));
    let free = created_id(&cli(&["create", "--title", "just work"]));

    // Block one yak on a human.
    cli(&["ask", &blocked, "--note", "which way?"]);

    // Baseline: plain list shows both.
    let all = cli(&["list"]);
    assert!(all.contains(&blocked) && all.contains(&free), "list: {all}");

    // --needs keeps only the blocked one.
    let needs = cli(&["list", "--needs"]);
    assert!(
        needs.contains(&blocked),
        "list --needs missing blocked: {needs}"
    );
    assert!(
        !needs.contains(&free),
        "list --needs leaked unblocked yak: {needs}"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

/// Positional-title create works (not only `--title`), and `create --json`
/// emits a parseable id + on-disk path. Throwaway herd built via the CLI so it
/// never touches the shared fixture (yaks-2120).
#[test]
fn create_positional_title_and_json() {
    let dir = std::env::temp_dir().join(format!("yaks-create-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let cli = |args: &[&str]| -> String {
        let out = Command::cargo_bin("yaks")
            .unwrap()
            .current_dir(&dir)
            .args(args)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "command {args:?} failed: {:?}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8(out.stdout).unwrap()
    };

    cli(&["init"]);

    // Bare positional title works (the stumble yaks-2120 fixes).
    let out = cli(&["create", "positional wins"]);
    let line = out.lines().find(|l| l.starts_with("Created ")).unwrap();
    assert!(line.ends_with(": positional wins"), "unexpected: {out}");
    let pos_id = line
        .strip_prefix("Created ")
        .unwrap()
        .split(':')
        .next()
        .unwrap()
        .trim();
    assert!(!pos_id.is_empty(), "empty id from positional create: {out}");

    // `create --json` emits an object with id + path (+ basic fields).
    let json_out = cli(&["create", "from json", "--json"]);
    let v: serde_json::Value = serde_json::from_str(&json_out).expect("parseable JSON");
    let id = v["id"].as_str().expect("id string");
    assert!(!id.is_empty(), "empty id in JSON: {json_out}");
    assert_eq!(v["title"], "from json");
    let path = v["path"].as_str().expect("path string");
    assert!(
        path.ends_with(&format!("hairy/{id}.md")),
        "path {path} should end with hairy/{id}.md"
    );
    assert!(
        std::path::Path::new(path).is_file(),
        "JSON path is not a file: {path}"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

/// `scan-ids` is the private-mode leak check: text carrying a real herd id is
/// flagged and the command exits NON-ZERO (so a pre-commit hook fails), while
/// text with only id-shaped-but-fake tokens is clean and exits zero. Runs
/// against the shared fixture herd, whose ids are `fix-000N` (yaks-d4d3).
#[test]
fn scan_ids_flags_real_ids_and_is_clean_otherwise() {
    let herd = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/herd");
    let scan = |stdin: &str| -> (bool, String) {
        let out = Command::cargo_bin("yaks")
            .unwrap()
            .current_dir(&herd)
            .arg("scan-ids")
            .write_stdin(stdin)
            .output()
            .unwrap();
        (out.status.success(), String::from_utf8(out.stdout).unwrap())
    };

    // A real herd id (fix-0004) leaks: flagged with line:col, exits non-zero.
    let (ok, stdout) = scan("intro line\nleaked ref fix-0004 in prose\n");
    assert!(
        !ok,
        "a real id must make scan-ids exit non-zero: {stdout:?}"
    );
    assert!(
        stdout.contains("fix-0004"),
        "real id not reported: {stdout:?}"
    );
    assert!(
        stdout.contains("2:"),
        "missing line:col for the hit: {stdout:?}"
    );

    // Only id-shaped but non-existent tokens: clean, exits zero, no output.
    let (ok, stdout) = scan("fix-9999 is not real\nplainword and http://x/y\n");
    assert!(ok, "fake/non-id tokens must exit zero: {stdout:?}");
    assert!(
        stdout.trim().is_empty(),
        "clean text should print nothing: {stdout:?}"
    );

    // --json emits a parseable array carrying the found id.
    let out = Command::cargo_bin("yaks")
        .unwrap()
        .current_dir(&herd)
        .args(["scan-ids", "--json"])
        .write_stdin("see fix-0002 here\n")
        .output()
        .unwrap();
    assert!(
        !out.status.success(),
        "json mode still exits non-zero on a hit"
    );
    let v: serde_json::Value =
        serde_json::from_str(&String::from_utf8(out.stdout).unwrap()).expect("parseable JSON");
    assert_eq!(v[0]["id"], "fix-0002");
}

/// Return true when the `labels:` line of `yaks show` output contains `label`
/// as a whole word.
fn show_labels_line_has(show_out: &str, label: &str) -> bool {
    show_out
        .lines()
        .find(|l| l.starts_with("labels:"))
        .map(|l| l.split_whitespace().any(|w| w == label))
        .unwrap_or(false)
}

/// Bulk `update` applies the same edit to every id in an explicit id-list, and a
/// missing id in the batch is reported + exits non-zero while the good ids still
/// apply. Explicit id-list only; filter-driven selection is deferred (yaks-7cc8).
/// Throwaway herd built via the CLI so it never touches the shared fixture.
#[test]
fn update_bulk_and_partial_failure() {
    let dir = std::env::temp_dir().join(format!("yaks-bulkupdate-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    // Raw runner: (success, stdout, stderr) so we can assert on the failure case.
    let raw = |args: &[&str]| -> (bool, String, String) {
        let out = Command::cargo_bin("yaks")
            .unwrap()
            .current_dir(&dir)
            .args(args)
            .output()
            .unwrap();
        (
            out.status.success(),
            String::from_utf8(out.stdout).unwrap(),
            String::from_utf8(out.stderr).unwrap(),
        )
    };
    let cli = |args: &[&str]| -> String {
        let (ok, stdout, stderr) = raw(args);
        assert!(ok, "command {args:?} failed: {stderr:?}");
        stdout
    };
    let created_id = |out: &str| -> String {
        out.lines()
            .find_map(|l| l.strip_prefix("Created "))
            .and_then(|rest| rest.split(':').next())
            .unwrap()
            .trim()
            .to_string()
    };

    cli(&["init"]);
    let a = created_id(&cli(&["create", "--title", "alpha"]));
    let b = created_id(&cli(&["create", "--title", "beta"]));

    // Bulk update: one edit, both ids get labeled.
    let out = cli(&["update", &a, &b, "--add-label", "x"]);
    assert!(
        out.contains(&format!("Updated {a}")),
        "a not updated: {out}"
    );
    assert!(
        out.contains(&format!("Updated {b}")),
        "b not updated: {out}"
    );
    assert!(
        show_labels_line_has(&cli(&["show", &a]), "x"),
        "a missing label x"
    );
    assert!(
        show_labels_line_has(&cli(&["show", &b]), "x"),
        "b missing label x"
    );

    // Partial failure: a good id followed by a missing id. The good id still
    // applies, the missing id is reported on stderr, and the batch exits
    // non-zero.
    let (ok, stdout, stderr) = raw(&["update", &a, "yaks-nope", "--add-label", "y"]);
    assert!(
        !ok,
        "batch with a missing id should exit non-zero: {stdout}"
    );
    assert!(
        stdout.contains(&format!("Updated {a}")),
        "good id skipped on partial failure: {stdout}"
    );
    assert!(
        stderr.contains("yaks-nope"),
        "missing id not reported: {stderr}"
    );
    assert!(
        show_labels_line_has(&cli(&["show", &a]), "y"),
        "good id not updated after partial failure"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

/// `yaks bulk` is filter-driven and DESTRUCTIVE-CAPABLE, so the safety model
/// (yaks-7cc8) is exercised end to end: dry-run by default changes nothing,
/// --commit applies, and both an unfiltered run and a mutation-less run refuse.
/// Throwaway herd built via the CLI so it never touches the shared fixture.
#[test]
fn bulk_dry_run_commit_and_refusals() {
    let dir = std::env::temp_dir().join(format!("yaks-bulk-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let raw = |args: &[&str]| -> (bool, String, String) {
        let out = Command::cargo_bin("yaks")
            .unwrap()
            .current_dir(&dir)
            .args(args)
            .output()
            .unwrap();
        (
            out.status.success(),
            String::from_utf8(out.stdout).unwrap(),
            String::from_utf8(out.stderr).unwrap(),
        )
    };
    let cli = |args: &[&str]| -> String {
        let (ok, stdout, stderr) = raw(args);
        assert!(ok, "command {args:?} failed: {stderr:?}");
        stdout
    };
    let created_id = |out: &str| -> String {
        out.lines()
            .find_map(|l| l.strip_prefix("Created "))
            .and_then(|rest| rest.split(':').next())
            .unwrap()
            .trim()
            .to_string()
    };
    let show_field = |show_out: &str, field: &str| -> String {
        show_out
            .lines()
            .find(|l| l.starts_with(field))
            .map(|l| l.splitn(2, ':').nth(1).unwrap().trim().to_string())
            .unwrap_or_default()
    };

    cli(&["init"]);
    // Two yaks carry label `pick`; one does not (the control that must never move).
    let a = created_id(&cli(&["create", "--title", "alpha", "--labels", "pick"]));
    let b = created_id(&cli(&["create", "--title", "beta", "--labels", "pick"]));
    let c = created_id(&cli(&["create", "--title", "gamma", "--labels", "other"]));

    // (a) Filtered DRY RUN: lists the matched set and the mutation, changes
    // nothing. Default (no --commit) must never write.
    let (ok, stdout, _) = raw(&["bulk", "--label", "pick", "--add-label", "sprint"]);
    assert!(ok, "dry run should exit 0: {stdout}");
    assert!(
        stdout.contains("would update 2 yaks:"),
        "dry run should preview 2 matches: {stdout}"
    );
    assert!(
        stdout.contains(&a) && stdout.contains(&b),
        "dry run missing ids: {stdout}"
    );
    assert!(
        stdout.contains("sprint"),
        "dry run should describe the mutation: {stdout}"
    );
    // Verify a matched yak is genuinely unchanged after the dry run.
    assert!(
        !show_labels_line_has(&cli(&["show", &a]), "sprint"),
        "dry run must not apply the label"
    );

    // (b) --commit actually applies to the matched set, and only that set.
    let out = cli(&[
        "bulk",
        "--label",
        "pick",
        "--add-label",
        "sprint",
        "--set-priority",
        "1",
        "--commit",
    ]);
    assert!(
        out.contains(&format!("Updated {a}")),
        "a not updated: {out}"
    );
    assert!(
        out.contains(&format!("Updated {b}")),
        "b not updated: {out}"
    );
    assert!(
        show_labels_line_has(&cli(&["show", &a]), "sprint"),
        "a missing sprint"
    );
    assert!(
        show_labels_line_has(&cli(&["show", &b]), "sprint"),
        "b missing sprint"
    );
    assert_eq!(
        show_field(&cli(&["show", &a]), "priority:"),
        "1",
        "a priority not set"
    );
    // The unmatched control yak is untouched.
    assert!(
        !show_labels_line_has(&cli(&["show", &c]), "sprint"),
        "unmatched yak must not be mutated"
    );

    // (c) No filter flag -> refuse (never operate on the whole herd).
    let (ok, _stdout, stderr) = raw(&["bulk", "--add-label", "z"]);
    assert!(!ok, "unfiltered bulk must exit non-zero");
    assert!(
        stderr.contains("filter flag"),
        "unfiltered bulk should explain the refusal: {stderr}"
    );

    // (d) No mutation flag -> refuse.
    let (ok, _stdout, stderr) = raw(&["bulk", "--label", "pick"]);
    assert!(!ok, "bulk with no mutation must exit non-zero");
    assert!(
        stderr.contains("mutation flag"),
        "mutation-less bulk should explain the refusal: {stderr}"
    );

    let _ = std::fs::remove_dir_all(&dir);
}
