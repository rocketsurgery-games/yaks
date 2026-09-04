---
id: yaks-39d4
title: 'yaks doctor: herd integrity check (post-merge danglers, parent/child, dup-status)'
type: feature
priority: 3
created: '2026-08-30T22:52:32Z'
updated: '2026-09-04T04:04:08Z'
parent: yaks-3901
labels:
- cli
---

Read-only integrity pass. Detect: a yak present in two status dirs at once (add/add from a branch merge), dangling parent/depends_on refs, parent/child state violations (hairy parent with shorn children), duplicate ids, orphaned artifacts. Especially valuable after merging per-branch herds. Complements the merge driver.
