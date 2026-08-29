---
id: yaks-7a92
title: Rename config prefix
type: feature
priority: 3
created: '2026-08-29T17:26:50Z'
updated: '2026-08-29T18:42:16Z'
parent: yaksrs-688d
depends_on:
- yaks-0af1
---

It should be practical to reliably rename all the yaks in a repo to reflect a change in the config's prefix.
We'll have to be careful about explicit and implicit cross-yak references, parent/child relationships, implicit references in markdown text, and so forth. But with the right built-in tools, it should be possible to do this reliably, by identifying reference candidates and double-checking them to ensure they reference actual yaks (so we don't inadvertently rename chunks of text that fit the pattern but aren't actual references).

---
▸ 2026-08-29T17:27:56Z
This relates to the informal yak-linking affordances described in yaksrs-688d. We should consider these changes as a whole during implementation.

---
▸ 2026-08-29T18:37:53Z
Bulk case of the 0af1 rewrite engine (dep: 7a92 -> 0af1 -> 0187). Live drift to reconcile: config prefix=yaks, disk=160 yaksrs- / 3 yaks-, and generate_id (store.rs:409) stamps every new yak with the config prefix, so the split widens on each create. Needs the DIRECTION DECISION recorded on parent 688d (migrate yaksrs->yaks vs revert config to yaksrs). Same surfaces as 0af1 (stem, frontmatter id/parent/depends_on, body bare+[[wiki]]), applied across the whole herd, collision-checked against all_ids, with --dry-run. With friends (15f7) it must also chase references in SIBLING herds, not just this one.

---
▸ 2026-08-29T18:42:16Z
DIRECTION SET: target prefix = yaks (migrate the 160 yaksrs- ids to yaks-). This bulk migration IS the acceptance test — build the tool, --dry-run it, then run it for real to reconcile the repo. Config already = yaks, so once migrated, generate_id stops widening the split.
