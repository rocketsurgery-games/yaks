---
id: yaks-7d49
title: 'Dogfood: encode toque TUI-verification guidance in this repo'
type: task
priority: 2
created: '2026-09-08T01:32:42Z'
updated: '2026-09-08T01:55:25Z'
parent: yaks-6624
labels:
- meta,eval
---

Fix our own gap first — we are the failing example. Add explicit guidance (AGENTS.md and/or per-repo context) that a TUI change is verified by driving toque to the relevant state and LOOKING at the rendered frame (+ the insta/docshots paths), not by 'tests pass' alone. Then watch whether it actually gets used in subsequent TUI work — this is itself a mini-eval of whether the guidance changes behavior (loops back to a2ca/fe5a).

---
▸ 2026-09-08T01:52:22Z [coordinator]
[accept] Done = AGENTS.md gains a concise 'Verifying changes' note naming THIS repo's levers: a TUI change is verified by driving toque to the relevant state and LOOKING at the rendered frame (plus cargo test docshots / insta snapshots), not 'tests pass' alone; plus the verify-or-ask expectation for changes lacking a lever. Evidence to attach: quote the added lines AND actually exercise the lever once (run the toque/docshots path; paste the command + that it produced a viewable artifact). Scope: AGENTS.md ONLY.

---
▸ 2026-09-08T01:55:22Z [worker-agents]
Added a '## Verifying changes' section to AGENTS.md (after ## Build):

  Verify against the real artifact via the project's lever, not "tests pass" alone. A TUI change is verified by driving `toque` to the relevant state and LOOKING at the rendered frame (and via `cargo test -p yaks docshots -- --ignored`, which renders color SVG frames to `docs/assets/`, plus the `insta` snapshots in `src/tui.rs`). If a change has no lever to see its effect, `yaks ask` rather than declare it done on faith.

Exercised the lever: ran 'cargo test -p yaks docshots -- --ignored' in the worktree — PASSED (test tui::docshots::docshots ... ok; 1 passed). It wrote/refreshed the viewable frames docs/assets/tui-detail.svg, tui-drawer.svg, tui-inbox.svg, tui-list.svg. Deterministic: git status shows those SVGs unchanged, only AGENTS.md modified.
