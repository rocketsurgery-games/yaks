---
id: yaks-a3a6
title: Make inbox a filter/view, not a modal toggle (unify with --needs)
type: feature
priority: 3
created: '2026-09-03T21:54:15Z'
updated: '2026-09-03T21:54:15Z'
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
