---
id: yaks-0af1
title: Rename-yak
type: feature
priority: 3
created: '2026-08-29T17:50:45Z'
updated: '2026-08-29T19:08:56Z'
parent: yaks-688d
depends_on:
- yaks-0187
---

Just a special-case of the "rename config prefix" tool; both require chasing and updating references across all the herds.

---
▸ 2026-08-29T18:37:49Z
This is the single-yak entry point to the same rewrite engine as 7a92 (its own desc already says so). Simplest case, so build it FIRST (dep: 0af1 -> 0187 core; 7a92 -> 0af1). Reuse the core validation gate to only rewrite CONFIRMED references, never lookalike prose. Surfaces to chase: file stem (rename the .md), frontmatter id/parent/depends_on, and body text (bare + [[wiki]]). Wants a --dry-run/preview since it edits across many files.

---
▸ 2026-08-29T19:08:56Z
SHORN. Built the shared rename engine that both single + prefix rename ride on.

refs::rewrite(text, replace) — whole-ref-token rewrite primitive (preserves [[ ]], punctuation, and longer lookalike tokens); reintroduced refs::has_ref_shape (now consumed by the rename target validator). herd::rename_many(pairs, dry_run) is the engine: validates targets (shape + collision + not-found), then in one pass rewrites every subject file+id and every referrer surface (parent, depends_on, title, body incl [[wiki]]), matched as validated whole tokens so lookalike prose is never touched. Two-phase apply (write all, then remove vacated files) is safe for disjoint-prefix batches. herd::rename() is the single-pair convenience.

CLI: `yaks rename <old> <new> [--dry-run]`. Outcomes: NotFound/Invalid/Collision/NothingToRename/Done(plan).

Tests: 3 refs (rewrite whole-token/brackets, no-op, has_ref_shape) + 4 herd (all surfaces + validation gate, collision, dry-run no-write, multi-pair batch across statuses). 186 lib + 20 CLI, warning-free. Dry-run verified on real data: `rename yaks-688d yaks-688d --dry-run` -> subject + 6 parent referrers.

HANDOFF to yaks-7a92: build pairs from the old-prefix id set and call rename_many; also flip config.yaml prefix.
