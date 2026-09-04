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
    assert!(needs.contains(&blocked), "list --needs missing blocked: {needs}");
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
    let pos_id = line.strip_prefix("Created ").unwrap().split(':').next().unwrap().trim();
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
    assert!(std::path::Path::new(path).is_file(), "JSON path is not a file: {path}");

    let _ = std::fs::remove_dir_all(&dir);
}
