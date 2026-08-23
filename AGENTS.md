# AGENTS.md — yaks

yaks is a filesystem-native task tracker: a single self-contained Rust binary.
Tasks are markdown files with YAML frontmatter under a `.yaks/` directory; a
task's status is implicit in which subdirectory it lives in.

## Invariants

- **Files are authoritative.** Status is implicit from the directory a task file
  lives in (`hairy/ shaving/ shorn/ dead/`). Parentage is a frontmatter `parent:`
  field; ids are flat and stable (any legacy dotted ids are inert — the `parent:`
  field is the only source of hierarchy).
- **Any index is derived.** If a lookup index is added, it must be a per-user,
  rebuildable cache — never committed, never a second source of truth.
- **Don't change the on-disk format lightly.** The `.yaks/` layout is the
  contract; task files are meant to be readable, greppable, and diffable.

## Layout

A Cargo workspace: the root is the `yaks` binary package *and* the workspace
root; reusable pieces live under `crates/`.

- `src/model.rs` — `Status`, `Task`.
- `src/store.rs` — `.yaks/` discovery + frontmatter parsing (hand-rolled fast path).
- `src/main.rs` — clap CLI + command dispatch.
- `src/herd.rs` — the core facade the CLI and TUI both call.
- `src/tui.rs` (+ `src/tui/`) — the interactive TUI; `src/tui/headless.rs` is a
  thin adapter implementing `toque::HeadlessApp` for `App`.
- `crates/toque/` — publishable library: drive any ratatui app headlessly
  (inject keys, capture LLM-/test-legible text snapshots with per-cell style).
  yaks is its first consumer. See `crates/toque/README.md`, and
  `docs/tui-style-eval.md` for the style-encoding research behind it.

## Build

```sh
cargo build --release
cargo test --workspace   # --workspace is required: `cargo test` alone only
                         # tests the root `yaks` package, not crates/toque
```

## Releasing

`.github/workflows/release.yml` builds the 5-platform binaries and publishes the
`@rocketsurgery/yaks` npm packages on a `vX.Y.Z` tag (dry-run on manual
dispatch). See `RELEASING.md`.

## Task tracking

This project uses yaks to track its own work. The yaks skill has the full
workflow.

1. Never start coding without a shaving yak. No exceptions.
2. Shear a yak as soon as its work is done, and commit the shorn yak file
   alongside the code that completed it (`.yaks/` is committed here — team mode).
3. Check existing yaks before creating new ones.
4. Append progress notes to yak descriptions as you work.
5. When unsure what's next, run `yaks next` — don't freelance.
