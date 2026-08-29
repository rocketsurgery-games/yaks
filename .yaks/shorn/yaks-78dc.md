---
id: yaks-78dc
title: Revisit show/hide/dim rules across yak states
type: task
priority: 2
created: '2026-08-23T02:49:48Z'
updated: '2026-08-28T04:16:07Z'
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

---
▸ 2026-08-28T03:55:07Z
Design settled (see thread). Herd-scope: a per-view, persistent lever for how much of an anchor family a tree view shows.

Vocabulary: anchor = matches the view status/content scope, shown bright (focus). context = shown only as family, dim (ghost). hidden = not shown.

Total rule grid, any yak in a tree view:
1. Anchor -> bright.
2. Ancestor of an anchor -> always shown, dim, to root. Invariant: never show a child without its full parent chain; walk ancestors even through Dead so the chain never breaks.
3. Descendant of an anchor -> governed by herd-scope mode below.
4. Otherwise hidden.

Herd-scope modes:
- lone: no descendants (anchors + rooting ancestors only).
- remaining: descendants not yet shorn/dead, i.e. open work. Proposed global default.
- all: every descendant, any status; shorn shown dim. Equals today behavior.
- auto: inherit the global default; stored as the ABSENCE of an override.
Applies identically with or without an active content filter (fixes the filtered-descendant asymmetry).

Persistence: global default is a constant (remaining) for now, config-exposed later. Per-view overrides live in the per-user XDG cache keyed by view.key, next to collapsed-state. auto writes no entry, so changing the global moves every untouched view.

Affordances:
- Key: h in list focus cycles auto -> lone -> remaining -> all. h is unbound in list focus today (l enters detail; h returns from detail). No-op with a hint on flat views.
- Display: trailing segment on the tab row beside the filter indicator: herd: all / herd: remaining / herd: lone; inheriting shows herd: ~remaining. Hidden on flat views.
- Later: a herd command on the : cmdline once it reaches the list view.

Code notes: logic is in tree::build (src/tui/tree.rs). Name the enum HerdScope (NOT Herd; avoid the facade collision). Flat views keep no herd logic. Tab counts count anchors (non-ghost) so they must stay stable across modes -- keep as an invariant + test.

Dropped the per-view-type magic seeds in favor of one global + explicit per-view overrides.

Filed separately: Dead dead-end bug; filtered-descendant asymmetry folded into this design.

---
▸ 2026-08-28T04:16:06Z
Implemented. HerdScope enum (lone/remaining/all, plus auto=inherit) in view.rs; per-view overrides persisted in the UI-state cache (cache::UiState, keyed by view.key; auto = absent entry). tree::build unified: members = seeds + ancestors (always, walkable through Dead) + herd-scoped descendants. remaining keeps open work (hairy/shaving) plus completed connectors, dropping fully-shorn subtrees; applies in both the filtered and unfiltered paths. h in list focus cycles auto -> lone -> remaining -> all with a notification; right-aligned herd: indicator on the tab row (yields when the tab strip is too wide; ~ marks auto/inherit); h:herd advertised in the footer. Global default = remaining. Tests in tree.rs + cache.rs; bin + cli snapshots regenerated (shorn leaf drops from the default Hairy view; footer gains h:herd). Fixes yaks-3331 as a side effect. Deferred: yaks-fe00 (Dead dead-end).
