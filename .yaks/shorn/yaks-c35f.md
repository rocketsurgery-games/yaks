---
id: yaks-c35f
title: B (semantic state header) as an optional developer debug facility
type: task
priority: 3
created: '2026-08-22T13:29:05Z'
updated: '2026-08-22T14:58:44Z'
parent: yaks-9b8d
labels:
- tui
- eval
---

Reframe: keep the App-state-derived header / classify as an optional, developer-fillable debugging hook (great for internal-state bugs), explicitly NOT the primary snapshot semantics channel. Per-cell concrete-style encodings are the snapshot focus; B is the oracle we score them against.

---
▸ 2026-08-22T14:58:44Z
Done (simple/literal, no classify): enriched App::state_header into the developer debug facility - now also surfaces rows, blocked=[ids], collapsed count, and filter summary alongside focus/view/cursor/sel/overlay. Validated on the fixture herd (blocked=[fix-0004]). classify / per-cell semantics intentionally deferred; snapshot style output stays literal. Regenerated the headless_session golden snapshot.
