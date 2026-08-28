---
id: yaksrs-3331
title: Content filter drops descendant context (filtered-descendant asymmetry)
type: bug
priority: 3
created: '2026-08-28T03:55:07Z'
updated: '2026-08-28T04:16:07Z'
labels:
- ui
---

In tree::build, the no-filter path sets members = anchors + ancestors + descendants, but the content-filter path sets members = matches + ancestors only -- descendants of a match are not re-added. So enabling a search/filter quietly changes how much family shows below a row. The yaksrs-78dc herd-scope design unifies this (the descendant policy should apply in both paths); fix here or as part of 78dc.

---
▸ 2026-08-28T04:03:30Z
Folded into yaksrs-78dc: the unified members computation in tree::build (seeds + ancestors + herd-scoped descendants) fixes this asymmetry as a side effect. Shear alongside 78dc.

---
▸ 2026-08-28T04:16:06Z
Fixed as part of yaksrs-78dc: tree::build now adds herd-scoped descendants of matches in the content-filter path too, so enabling a filter no longer silently drops descendant context. Covered by tree::tests::content_filter_includes_descendants_of_matches.
