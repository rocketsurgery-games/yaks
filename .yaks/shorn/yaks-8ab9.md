---
id: yaks-8ab9
title: b1cc/1 Scaffolding + shared test_support
type: task
priority: 3
created: '2026-09-11T01:29:48Z'
updated: '2026-09-11T02:01:57Z'
parent: yaks-b1cc
labels:
- ui
verify: cargo test -p yaks
---

Foundation, no logic moves. Introduce src/tui/test_support.rs (cfg(test), pub(crate) helpers: draw/sample/editable/linked + the 'live' harness temp_herd/press), and rewire tui.rs mod tests to use it, so later phases can relocate tests and reuse helpers. Green + snapshots unchanged.

---
▸ 2026-09-11T02:01:57Z [coordinator]
verify: `cargo test -p yaks` -> PASS (exit 0)

---
▸ 2026-09-11T02:01:57Z [coordinator]
Shorn: extracted shared test helpers (task, buffer_to_string, draw, key, enter_key/esc_key/down_key/tab_key, sample, editable, linked, temp_herd, press) into src/tui/test_support.rs (#[cfg(test)], pub(crate)); rewired mod tests + mod live to import them. No tests moved yet, so src/snapshots/ untouched (22, no .snap.new). Foundation for phases 2-8. Evidence: cargo test -p yaks 242+25 PASS.
