//! Golden snapshot tests for read commands over a fixed fixture farm
//! (tests/fixtures/farm). Snapshots are yaks's own goldens (semantic).
//! Regenerate with: INSTA_UPDATE=always cargo test.
//! These double as assert_cmd smoke tests (yaksrs-c725).

use assert_cmd::Command;
use std::path::PathBuf;

fn run(args: &[&str]) -> String {
    let farm = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/farm");
    let out = Command::cargo_bin("yaks")
        .unwrap()
        .current_dir(farm)
        // Ordinary commands top up the user's skills on startup. A test run
        // must never touch the developer's real ~/.agents/skills (it did:
        // `cargo test` was silently re-stamping them, and through a symlink,
        // the repo's own skills/ source).
        .env("YAKS_SKILLS_AUTOSYNC", "0")
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

/// Drive the headless TUI over the fixture farm with isolated XDG dirs (so
/// persisted views/cache can't make the output machine-dependent).
fn run_headless(args: &[&str], stdin: &str) -> String {
    let farm = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/farm");
    let xdg = std::env::temp_dir().join(format!("yaksrs-headless-{}", std::process::id()));
    let out = Command::cargo_bin("yaks")
        .unwrap()
        .current_dir(farm)
        .env("XDG_CONFIG_HOME", &xdg)
        .env("XDG_CACHE_HOME", &xdg)
        .env("YAKS_SKILLS_AUTOSYNC", "0")
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

/// `list --needs` selects only yaks blocked on a human. Uses a throwaway farm
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
/// emits a parseable id + on-disk path. Throwaway farm built via the CLI so it
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

/// `scan-ids` is the private-mode leak check: text carrying a real farm id is
/// flagged and the command exits NON-ZERO (so a pre-commit hook fails), while
/// text with only id-shaped-but-fake tokens is clean and exits zero. Runs
/// against the shared fixture farm, whose ids are `fix-000N` (yaks-d4d3).
#[test]
fn scan_ids_flags_real_ids_and_is_clean_otherwise() {
    let farm = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/farm");
    let scan = |stdin: &str| -> (bool, String) {
        let out = Command::cargo_bin("yaks")
            .unwrap()
            .current_dir(&farm)
            .arg("scan-ids")
            .write_stdin(stdin)
            .output()
            .unwrap();
        (out.status.success(), String::from_utf8(out.stdout).unwrap())
    };

    // A real farm id (fix-0004) leaks: flagged with line:col, exits non-zero.
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
        .current_dir(&farm)
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
/// Throwaway farm built via the CLI so it never touches the shared fixture.
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
/// Throwaway farm built via the CLI so it never touches the shared fixture.
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

    // (c) No filter flag -> refuse (never operate on the whole farm).
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

/// `yaks slaughter` refuses a parent with live descendants (naming them and
/// pointing at --family), and `--family` slaughters the whole live family
/// (yaks-05da). Throwaway farm built via the CLI so it never touches the
/// shared fixture.
#[test]
fn slaughter_guard_and_family() {
    let dir = std::env::temp_dir().join(format!("yaks-slaughter-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let raw = |args: &[&str]| -> (bool, String, String) {
        let out = Command::cargo_bin("yaks")
            .unwrap()
            .current_dir(&dir)
            .env("YAKS_SKILLS_AUTOSYNC", "0")
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
    let p = created_id(&cli(&["create", "--title", "parent"]));
    let c = created_id(&cli(&["create", "--title", "child", "--parent", &p]));
    let g = created_id(&cli(&["create", "--title", "grandchild", "--parent", &c]));

    // Guard: refused, non-zero, names the descendants and the flag; nothing moves.
    let (ok, stdout, stderr) = raw(&["slaughter", &p]);
    assert!(!ok, "guarded slaughter should exit non-zero: {stdout}");
    assert!(stderr.contains("2 live descendants"), "{stderr}");
    assert!(stderr.contains(&c) && stderr.contains(&g), "{stderr}");
    assert!(
        stderr.contains("--family"),
        "guard must point at --family: {stderr}"
    );
    assert!(cli(&["show", &p]).contains("Hairy"), "guard must not move");

    // --family: grandchild, child, then parent.
    let out = cli(&["slaughter", &p, "--family"]);
    assert_eq!(
        out,
        format!("Slaughtered: {g}\nSlaughtered: {c}\nSlaughtered: {p}\n")
    );
    for id in [&p, &c, &g] {
        assert!(cli(&["show", id]).contains("Dead"), "{id} not dead");
    }

    let _ = std::fs::remove_dir_all(&dir);
}

/// `attach --name` stores under a chosen (sanitized, extension-kept) name, and
/// `rename-attachment` moves the file + rewrites the body link, refusing to
/// clobber another attachment (yaks-fe19).
#[test]
fn attach_name_and_rename_attachment() {
    let dir = std::env::temp_dir().join(format!("yaks-attach-rename-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let cli = |args: &[&str]| {
        Command::cargo_bin("yaks")
            .unwrap()
            .current_dir(&dir)
            .env("YAKS_SKILLS_AUTOSYNC", "0")
            .args(args)
            .output()
            .unwrap()
    };
    let ok = |args: &[&str]| -> String {
        let out = cli(args);
        assert!(
            out.status.success(),
            "command {args:?} failed: {:?}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8(out.stdout).unwrap()
    };
    ok(&["init"]);
    let out = ok(&["create", "shots", "--json"]);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    let id = v["id"].as_str().unwrap().to_string();
    let src = dir.join("paste-20260921-234927.png");
    std::fs::write(&src, b"png").unwrap();
    let src = src.to_str().unwrap();

    ok(&["attach", &id, src]);
    let out = ok(&["attach", &id, src, "--name", "login page"]);
    assert!(out.contains("attached login-page.png"), "{out}");

    let out = ok(&[
        "rename-attachment",
        &id,
        "paste-20260921-234927.png",
        "dashboard",
    ]);
    assert!(out.contains("-> dashboard.png"), "{out}");
    let arts = dir.join(".yaks/artifacts").join(&id);
    assert!(arts.join("dashboard.png").is_file());
    assert!(!arts.join("paste-20260921-234927.png").exists());
    let show = ok(&["show", &id]);
    assert!(
        show.contains(&format!("![dashboard](artifacts/{id}/dashboard.png)")),
        "{show}"
    );
    assert!(
        show.contains(&format!("![login-page](artifacts/{id}/login-page.png)")),
        "{show}"
    );

    // Collision is refused, non-zero, and nothing moves.
    let out = cli(&["rename-attachment", &id, "dashboard.png", "login-page"]);
    assert!(!out.status.success());
    assert!(
        String::from_utf8_lossy(&out.stderr)
            .contains("already has an attachment named login-page.png")
    );
    assert!(arts.join("dashboard.png").is_file());

    let _ = std::fs::remove_dir_all(&dir);
}

/// Labels are normalized at every CLI entry point (yaks-7cb3): commas and
/// whitespace separate labels, empties drop, duplicates collapse in order. A
/// legacy comma label on disk is flagged by `doctor` and re-split on the next
/// label edit. Throwaway farm built via the CLI.
#[test]
fn labels_normalized_on_create_update_bulk_and_doctor() {
    let dir = std::env::temp_dir().join(format!("yaks-labelnorm-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let raw = |args: &[&str]| -> (bool, String) {
        let out = Command::cargo_bin("yaks")
            .unwrap()
            .current_dir(&dir)
            .args(args)
            .output()
            .unwrap();
        (out.status.success(), String::from_utf8(out.stdout).unwrap())
    };
    let cli = |args: &[&str]| -> String {
        let (ok, stdout) = raw(args);
        assert!(ok, "command {args:?} failed: {stdout}");
        stdout
    };
    let labels = |id: &str| -> Vec<String> {
        let v: serde_json::Value = serde_json::from_str(&cli(&["show", id, "--json"])).unwrap();
        v["labels"]
            .as_array()
            .map(|a| a.iter().map(|l| l.as_str().unwrap().to_string()).collect())
            .unwrap_or_default()
    };

    cli(&["init"]);
    let created: serde_json::Value = serde_json::from_str(&cli(&[
        "create", "alpha", "--labels", "foo, bar", "baz,foo", "--json",
    ]))
    .unwrap();
    let a = created["id"].as_str().unwrap().to_string();
    assert_eq!(labels(&a), ["foo", "bar", "baz"]);

    cli(&[
        "update",
        &a,
        "--add-label",
        "x y",
        "--remove-label",
        "foo,baz",
    ]);
    assert_eq!(labels(&a), ["bar", "x", "y"]);

    // Bulk: the dry-run preview shows the normalized labels.
    let dry = cli(&["bulk", "--label", "bar,nope", "--add-label", "p,q"]);
    assert!(dry.contains("add labels [p, q]"), "{dry}");

    // A legacy on-disk comma label: doctor flags it (non-zero exit)...
    let path = dir.join(format!(".yaks/hairy/{a}.md"));
    let text = std::fs::read_to_string(&path).unwrap();
    std::fs::write(&path, text.replacen("- bar\n", "- ui,docs\n", 1)).unwrap();
    let (ok, out) = raw(&["doctor"]);
    assert!(!ok, "doctor should fail on a malformed label: {out}");
    assert!(out.contains("\"ui,docs\""), "{out}");
    // ...and any label edit re-splits it canonically.
    cli(&["update", &a, "--add-label", "ui"]);
    assert_eq!(labels(&a), ["ui", "docs", "x", "y"]);
    assert!(raw(&["doctor"]).0, "doctor clean after re-edit");
    let _ = std::fs::remove_dir_all(&dir);
}
