---
id: yaks-3f19
title: Jumping to another yak (via 'enter' in details) changes the list selection, but doesn't show the detail pane
type: bug
priority: 3
created: '2026-08-22T19:07:20Z'
updated: '2026-08-22T19:17:11Z'
parent: yaks-0a93
---

---
▸ 2026-08-22T19:17:11Z
follow_link now stays in the detail pane on the followed task (was dropping to the list) and resets per-task detail state (scroll/link/find/match). Also fixed the related silent no-op: select_task now expands any collapsed ancestors of the target (new expand_ancestors) so a jumped-to yak hidden under a collapsed parent is revealed and selected. Updated detail_jumplist_follows_to_task; added follow_link_reveals_collapsed_target. 102 unit + 19 CLI green.
