---
id: yaksrs-78dc
title: Revisit show/hide/dim rules across yak states
type: task
priority: 2
created: '2026-08-23T02:49:48Z'
updated: '2026-08-23T02:49:48Z'
labels:
- ui
---

I *think* the rules go something like this (but please double-check before diving in):
- Never show a child yak without its parent chain, regardless of the parents' states.
- If a parent is shaving, and you're viewing shaving yaks, then show only its

---
▸ 2026-08-12T15:41:50Z
Also consider that there may not be one "right" answer to how much detail to show for related parent/child yaks in a different state.
Eg, I might want to show *all* children for currently-shaving yaks, so that I can see both sides of the completion state of its children.
Or I could want to see *only* those parents strictly required to show the parent chain to root.
Or at times it's only important to see the parent chain, and children remaining to be shorn.
IOW, it's probably a view state affordance question.
