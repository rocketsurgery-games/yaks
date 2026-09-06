---
id: yaks-eb5f
title: Mechanize remaining validation predicates (commit-names-id, worker-scope-adherence)
type: feature
priority: 3
created: '2026-09-06T22:44:52Z'
updated: '2026-09-06T22:44:52Z'
parent: yaks-10fb
labels:
- agent
---

From a412's validation list; only doctor + doctor --strict (evidence-before-shear) are mechanized so far. Remaining, both git-based (doctor is herd-only, so likely a separate check or CI script): (1) every landed commit names a yak id (grep git log over a range; reuse refs::scan_text from yaks-d4d3 to validate ids); (2) each worker's branch touched only its assigned scope (git diff --stat per branch vs an allowlist). Also: main-src-clean-during-a-run held every run — candidate CI assertion.
