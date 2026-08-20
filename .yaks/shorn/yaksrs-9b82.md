---
id: yaksrs-9b82
title: 'Thin UI over CLI: composable core ops + expose tools for third-party UIs'
type: idea
priority: 2
created: '2026-08-20T19:29:03Z'
updated: '2026-08-20T22:31:13Z'
labels:
- rust
- ui
---

Design goal (from owner, to discuss before TUI work): structure CLI operations so the UI layer stays thin on top of the CLI/library implementation.

- Within pure Rust: the TUI should call the same core operations the CLI dispatches to (a shared 'ops'/service layer returning typed results), not re-implement logic. main.rs command handlers should shrink to arg-parse + render around those ops.
- Externally: explore exposing these operations as tools a third-party UI can call into efficiently (e.g. a stable library API, a --json/stdin protocol, or an MCP-style/IPC surface) so non-Rust UIs get the same capabilities without shelling out per-op.
- Revisit the command/handler split (currently logic lives partly in store + partly in main handlers) so there's a clean, testable core-ops boundary the CLI, TUI, and external callers all share.
Discuss scope/shape before starting Phase 2.

---
▸ 2026-08-20T22:31:13Z
Discussion concluded. Decisions: (1) a print-free core ops facade (Herd) exposing each WHOLE operation as a typed method; CLI and TUI are thin layers over it. (2) Reuse target = editor/IDE plugins (nvim/VSCode/Cursor) that either shell out to yaks or talk to a long-lived LSP-like process over socket/pipe; expose via protocol/specialized CLI LATER — architecture must SUPPORT a persistent process, but defer that plumbing. (3) No composite/atomic ops beyond what the TUI needs; the facade exposes the whole operation for the atomic bits it does need. (4) TUI ships as a `yaks tui` subcommand. Keep core free of clap/rendering so it can later extract to a lib crate.
