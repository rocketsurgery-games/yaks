---
id: yaks-7d49
title: 'Dogfood: encode toque TUI-verification guidance in this repo'
type: task
priority: 2
created: '2026-09-08T01:32:42Z'
updated: '2026-09-08T01:52:22Z'
parent: yaks-6624
labels:
- meta,eval
---

Fix our own gap first — we are the failing example. Add explicit guidance (AGENTS.md and/or per-repo context) that a TUI change is verified by driving toque to the relevant state and LOOKING at the rendered frame (+ the insta/docshots paths), not by 'tests pass' alone. Then watch whether it actually gets used in subsequent TUI work — this is itself a mini-eval of whether the guidance changes behavior (loops back to a2ca/fe5a).

---
▸ 2026-09-08T01:52:22Z [coordinator]
[accept] Done = AGENTS.md gains a concise 'Verifying changes' note naming THIS repo's levers: a TUI change is verified by driving toque to the relevant state and LOOKING at the rendered frame (plus cargo test docshots / insta snapshots), not 'tests pass' alone; plus the verify-or-ask expectation for changes lacking a lever. Evidence to attach: quote the added lines AND actually exercise the lever once (run the toque/docshots path; paste the command + that it produced a viewable artifact). Scope: AGENTS.md ONLY.
