---
id: yaks-a3a6
title: Make inbox a filter/view, not a modal toggle (unify with --needs)
type: feature
priority: 3
created: '2026-09-03T21:54:15Z'
updated: '2026-09-03T22:45:51Z'
parent: yaks-594b
labels:
- ui
- cli
---

From Joel: the TUI 'i' inbox_only flag (added in yaks-548b) is extra modal UI state. Better to treat 'awaiting a human' as a FILTER PREDICATE that composes with any view — like the 'recent' view — rather than a mode.

Design: add a 'needs' predicate to FilterSpec (in filter.rs) + filter::matches, then surface it three ways from ONE capability:
- TUI: a saved 'inbox' view (like 'recent'), and/or a filter option usable inside any view; REMOVE the App.inbox_only toggle.
- CLI: the '--needs [who]' filter deferred from yaks-45c7 (list/search/log). 'yaks inbox' becomes sugar for 'list --needs' (with the status-independence from yaks-bc68).

IMPORTANT (learned in the 2nd parallel run): FilterSpec has EXHAUSTIVE literal constructions in tui/ (tui.rs, tui/views_store.rs, tui/view.rs) + main.rs. Adding the field is a SHARED-TYPE change -> do it as a coordinator PREP COMMIT (add the field + fix all constructions + the matches logic up front), THEN any UI/CLI wiring can proceed in parallel without cross-lane collision. This unifies the deferred 45c7 --needs filter and the 548b inbox toggle into one predicate.

---
▸ 2026-09-03T22:45:51Z [coordinator]
THIRD PARALLEL RUN — done, clean; VALIDATED the coordinator-prep-commit pattern for a shared-type change. Prep (yaks-f6f8): I landed FilterSpec.needs_only + matches + JSON round-trip + all constructions on main FIRST. Then two lanes off the prepped main: CLI (yaks-f81a: --needs on FilterFlags -> build_spec, main.rs) + TUI (yaks-11e9: inbox as a filter chip + built-in Inbox view, removed the App.inbox_only modal toggle, kept the badge + ask/answer). Disjoint files (main.rs/tests vs tui.rs/view.rs/views_store.rs/snapshots); ZERO-conflict --no-ff merges; 221 bin + 21 CLI green. The pattern the coordinating-yaks skill prescribes works: a shared type edited once by the coordinator lets the wiring fan out with no cross-lane churn. inbox is now composable (chip + view), not a mode. TUI worker kept Inbox unpinned (design call; pinning is a 1-line flip, noted on yaks-11e9).
