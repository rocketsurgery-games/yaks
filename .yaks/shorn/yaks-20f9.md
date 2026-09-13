---
id: yaks-20f9
title: b1cc/8 Extract runtime + final reconcile (snapshots, slim tui.rs, docs)
type: task
priority: 3
created: '2026-09-11T01:29:48Z'
updated: '2026-09-13T21:13:26Z'
parent: yaks-b1cc
depends_on:
- yaks-e1d1
labels:
- ui
verify: cargo test -p yaks
---

src/tui/runtime.rs: run/event_loop/setup_watcher/setup/restore. Then reconcile: confirm src/snapshots/ fully migrated to per-module snapshots/ dirs; tui.rs reduced to App struct + Focus/Overlay + mod decls (+ core ctors); update AGENTS.md 'Layout' + any docs referencing tui.rs structure; cargo test --workspace + yaks doctor green; record final line-count before/after as evidence.

---
▸ 2026-09-13T21:13:26Z [coordinator]
verify: `cargo test -p yaks` -> PASS (exit 0)

---
▸ 2026-09-13T21:13:26Z [coordinator]
Shorn: (1) extracted runtime (run/event_loop/setup_watcher/setup/restore) -> src/tui/runtime.rs (use super::*; 'pub use runtime::run' keeps tui::run for main.rs). (2) moved the central whole-frame test module -> src/tui/tests.rs (#[cfg(test)] mod tests;) and RELOCATED all 22 insta snapshots src/snapshots/ -> src/tui/snapshots/ (insta co-locates snaps with the test file; still ONE central home next to the central test module -- refines decision (a)'s literal path). (3) updated AGENTS.md Layout to the new module tree. RESULT: tui.rs 2604->348 (7043 original, -95%). CLOSING GATE: cargo test --workspace green (toque 7 + yaks 242 unit + 25 cli + 1 doctest), docshots SVG generation intact with docs/assets byte-IDENTICAL (behavior preserved), yaks doctor all clear.
