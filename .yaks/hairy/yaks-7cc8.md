---
id: yaks-7cc8
title: 'DECIDE: safety model for filter-driven bulk mutations'
type: task
priority: 2
created: '2026-09-04T17:08:35Z'
updated: '2026-09-04T17:08:35Z'
parent: yaks-2ebe
labels:
- cli
needs: human
---

Explicit id-list bulk mutation (yaks update a b c --add-label X) is safe and is being built now. FILTER-driven mutation (apply to every yak matching a filter — bulk relabel/reparent/slaughter across a matching set) is the powerful-but-destructive-capable part that needs a human risk call before we build it.

---
▸ 2026-09-04T17:08:35Z [coordinator]
Filter-driven bulk mutation is destructive-capable (a wrong filter could relabel or slaughter the whole herd). What safety model do you want for v1?
(a) DRY-RUN BY DEFAULT: a filter mutation prints the affected set and does nothing unless you pass --commit (or --yes); id-list mutation applies directly.
(b) ID-LIST ONLY for now: defer filter-driven mutation entirely until we've lived with the id-list version.
(c) FILTER ALLOWED with a hard cap + confirmation above N matches (e.g. refuse >20 without --force).
My lean: (a) dry-run-default + require --commit, and additionally never allow slaughter via filter without an explicit expected-count match. Which do you want? I'm building the safe id-list half now regardless.
