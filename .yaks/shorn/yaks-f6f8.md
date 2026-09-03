---
id: yaks-f6f8
title: 'a3a6 PREP: add needs predicate to FilterSpec + matches (coordinator prep commit)'
type: task
priority: 3
created: '2026-09-03T22:28:44Z'
updated: '2026-09-03T22:30:34Z'
parent: yaks-a3a6
labels:
- cli
---

Shared-type change done up front on main (per the coordinating-yaks disjoint-TYPE rule) so the CLI + TUI wiring can then fan out cleanly. Add a needs filter to FilterSpec + filter::matches, and fix every exhaustive construction.

---
▸ 2026-09-03T22:30:34Z [coordinator]
PREP DONE (coordinator commit on main, per the disjoint-TYPE SOP). Added FilterSpec.needs_only + filter::matches (keeps only needs-blocked yaks; composes with any status scope) + JSON round-trip in views_store + fixed all 3 exhaustive constructions (main.rs, tui.rs, views_store.rs) to default false. Test needs_only_keeps_only_blocked_yaks; 220 green. Now the CLI lane (main.rs: --needs -> build_spec) and TUI lane (tui.rs/views_store: inbox view, remove i toggle) can wire the SAME predicate from disjoint files with zero shared-type churn. This IS the pattern the coordinating-yaks skill now prescribes; run it to validate.
