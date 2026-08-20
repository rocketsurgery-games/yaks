//! Golden snapshot tests for read commands over a fixed fixture herd
//! (tests/fixtures/herd). Snapshots are yaks-rs's own goldens (semantic, not
//! byte-parity with Python). Regenerate with: INSTA_UPDATE=always cargo test.
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
    assert!(out.status.success(), "command {args:?} failed: {:?}", String::from_utf8_lossy(&out.stderr));
    String::from_utf8(out.stdout).unwrap()
}

macro_rules! snap {
    ($name:literal, $args:expr) => {
        insta::assert_snapshot!($name, run($args))
    };
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
fn list_json() {
    snap!("list_json", &["list", "--json"]);
}
#[test]
fn list_filtered() {
    snap!("list_filtered", &["list", "--type", "feature", "--priority", "1", "--priority", "2"]);
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
