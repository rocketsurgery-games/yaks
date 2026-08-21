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

## Task tracking

This project uses Yaks. The Yaks skill has the full workflow.

1. Never start coding without a shaving yak. No exceptions.
2. Shear a yak as soon as its work is done. If the project commits its yaks (`.yaks/` is tracked by git), commit the shorn yak alongside the code that completed it; if `.yaks/` is gitignored, keep yak files — and their IDs — out of commits, PRs, and anything external.
3. Check existing yaks before creating new ones.
4. Append progress notes to yak descriptions as you work.
5. When unsure what's next, run `yaks next` — don't freelance.
