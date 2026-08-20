---
id: yaksrs-9b82
title: 'Thin UI over CLI: composable core ops + expose tools for third-party UIs'
type: idea
priority: 2
created: '2026-08-20T19:29:03Z'
updated: '2026-08-20T19:29:03Z'
parent: yaksrs-86a3
labels:
- rust
- ui
---

Design goal (from owner, to discuss before TUI work): structure CLI operations so the UI layer stays thin on top of the CLI/library implementation.

- Within pure Rust: the TUI should call the same core operations the CLI dispatches to (a shared 'ops'/service layer returning typed results), not re-implement logic. main.rs command handlers should shrink to arg-parse + render around those ops.
- Externally: explore exposing these operations as tools a third-party UI can call into efficiently (e.g. a stable library API, a --json/stdin protocol, or an MCP-style/IPC surface) so non-Rust UIs get the same capabilities without shelling out per-op.
- Revisit the command/handler split (currently logic lives partly in store + partly in main handlers) so there's a clean, testable core-ops boundary the CLI, TUI, and external callers all share.
Discuss scope/shape before starting Phase 2.
