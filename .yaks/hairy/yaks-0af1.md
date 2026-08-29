---
id: yaks-0af1
title: Rename-yak
type: feature
priority: 3
created: '2026-08-29T17:50:45Z'
updated: '2026-08-29T18:37:49Z'
parent: yaksrs-688d
depends_on:
- yaks-0187
---

Just a special-case of the "rename config prefix" tool; both require chasing and updating references across all the herds.

---
▸ 2026-08-29T18:37:49Z
This is the single-yak entry point to the same rewrite engine as 7a92 (its own desc already says so). Simplest case, so build it FIRST (dep: 0af1 -> 0187 core; 7a92 -> 0af1). Reuse the core validation gate to only rewrite CONFIRMED references, never lookalike prose. Surfaces to chase: file stem (rename the .md), frontmatter id/parent/depends_on, and body text (bare + [[wiki]]). Wants a --dry-run/preview since it edits across many files.
