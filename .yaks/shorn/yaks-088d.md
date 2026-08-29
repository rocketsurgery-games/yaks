---
id: yaks-088d
title: Need copy affordance for yak ids
type: feature
priority: 3
created: '2026-08-22T20:21:02Z'
updated: '2026-08-22T20:29:02Z'
parent: yaks-0a93
---

---
▸ 2026-08-22T20:29:02Z
Added arboard(3) + src/clipboard.rs (copy_text, best-effort). Bound y in both list and detail panes to copy the selected yak id (copy_selected_id); notifies 'copied {id}' or 'clipboard unavailable'. Added to ? help. Verified via headless (copies on this machine; degrades gracefully where no clipboard). read_png/image support deferred to the artifact-attach work (a49c). 110 unit + 19 CLI green.
