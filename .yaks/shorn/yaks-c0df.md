---
id: yaks-c0df
title: 'TUI slice 3a: App-Herd refactor + reload + bottom-line overlay + single-key mutations (S/P/T/X)'
type: task
priority: 3
created: '2026-08-21T00:08:11Z'
updated: '2026-08-21T00:14:37Z'
parent: yaks-86a3
labels:
- rust
- phase2
---

Foundational mutation slice. Give App a Herd handle and a reload() that re-queries all after each mutation (tests keep the herd-less App::new(tasks) read-only constructor). Add a pure bottom-line overlay system (Overlay enum: Pick single-key, Confirm y/N) that render() paints and key handling routes to, faithfully reproducing the Python pick()/confirm() dialogs. Wire: S state picker (h/s/n/x -> transition), P priority (1-5 -> update), T type (t/b/f/i -> update), X delete (confirm -> slaughter to dead, refusing if children). Each ends with reload + a status notification line. Snapshot-test each overlay + a post-mutation reload via TestBackend.

---
▸ 2026-08-21T00:14:33Z
Done. App now holds Option<Herd>; App::with_herd loads the view + reload() re-queries after each mutation and clamps the cursor. Added a pure bottom-line Overlay system (Pick single-key + Confirm y/N) routed before normal keys, reproducing Python pick()/confirm(). Wired S(state h/s/n/x->transition), P(priority 1-5->update), T(type t/b/f/i->update), X(confirm->slaughter to Dead, refuses non-dead children). Each ends with reload + a notification line; same-value picks are no-ops with an "already" message. Design call: X slaughters to Dead (recoverable via revive) rather than Python-TUI hard unlink, matching our git-like recoverability. 51 tests (was 39): overlay-render snapshots + a temp-.yaks live mutation/reload harness. Warning-free.
