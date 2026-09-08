---
id: yaks-eb5f
title: Mechanize remaining validation predicates (commit-names-id, worker-scope-adherence)
type: feature
priority: 3
created: '2026-09-06T22:44:52Z'
updated: '2026-09-08T00:51:01Z'
parent: yaks-10fb
labels:
- agent
---

From a412's validation list; only doctor + doctor --strict (evidence-before-shear) are mechanized so far. Remaining, both git-based (doctor is herd-only, so likely a separate check or CI script): (1) every landed commit names a yak id (grep git log over a range; reuse refs::scan_text from yaks-d4d3 to validate ids); (2) each worker's branch touched only its assigned scope (git diff --stat per branch vs an allowlist). Also: main-src-clean-during-a-run held every run — candidate CI assertion.

---
▸ 2026-09-08T00:51:01Z [coordinator]
pstack cross-ref (from the pstack mining, yaks-a2ca): this yak IS pstack's 'encode-lessons-in-structure' + 'build-the-lever' + 'prove-it-works' applied to yaks — mechanize skill conventions into scripted doctor/CI predicates. pstack's whole thesis ('Verification is all you need'; a good verification lever 100-1000x's a team) is an argument to PRIORITIZE this. The commit-names-id and worker-scope-adherence predicates are the concrete next levers; doctor --strict (evidence-before-shear) is the pattern to extend.
