# AGENTS.md — yaks-rs

Rust port of yaks (Phase 0 spike, `yak-94e7`). Design and rationale live in the
parent Python repo's yak herd (research arc `yak-2219`); read those notes before
making architectural decisions.

## Invariants (inherited from the Python design)

- **Files are authoritative.** Status is implicit from the directory a task file
  lives in (`hairy/ shaving/ shorn/ dead/`). Parentage is a frontmatter `parent:`
  field; ids are flat and stable (legacy dots are inert).
- **The index is derived**, per-user, rebuildable, never committed, never a second
  source of truth.
- **Interop:** the Rust and Python tools must read/write the same `.yaks/` layout
  during the transition. Do not change the on-disk format without updating both.

## Layout

- `src/model.rs` — `Status`, `Task`.
- `src/store.rs` — `.yaks/` discovery + frontmatter parsing (hand-rolled fast path).
- `src/main.rs` — clap CLI + command dispatch.

## Build

```sh
cargo build --release
cargo test
```
