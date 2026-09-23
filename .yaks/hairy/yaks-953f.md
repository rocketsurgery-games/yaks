---
id: yaks-953f
title: Global search context
type: bug
priority: 3
created: '2026-08-23T14:38:59Z'
updated: '2026-09-23T21:40:08Z'
labels:
- ui
- search
needs: human
---

When searching within a list, we get a "filter: ..." header, and it sticks within the current list. But when you change lists, the header persists, but the search context doesn't.

Let's try and unify search handling in such a way that the filter: header and actual search behavior are always in sync. And then ensure that it also propagates to detail views. Once that structure's sound, we can also have the "next" affordance remember the last search and pull it back in, like in vim.

![search-design](artifacts/yaks-953f/search-design.md)

---
▸ 2026-09-23T21:40:08Z [design-scout]
Design proposal (covers yaks-1bbd too)

---
▸ 2026-09-23T21:40:08Z [design-scout]
Root cause found + reproduced headlessly: the persisting 'filter: q' is a toast (App.notification is never cleared anywhere), not the real header; set_view does wipe the search. Decisions: (1) Phase 1 fix: clear toasts on every keypress globally [lean] vs only on view switch? Also drop the redundant 'filter: q' toast on search commit [lean yes]. (2) Should the / query persist across view switches as its own header chip, cleared by Esc [lean yes], or keep today's clear-on-switch? (3) Detail /: update only a shared vim-style search register, not the list filter [lean], and should n past the last match roll into the next matching yak [lean yes]? (4) List n/N with an active query: next/prev row [lean] or reserve n only for 're-apply last search' when the query is empty? Phase 1 can proceed immediately once Q1 is answered; Q2-4 gate phases 2-3.
