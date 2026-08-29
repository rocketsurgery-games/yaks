---
id: yaks-b315
title: Filtering + next/tangled/search/stats
type: task
priority: 3
created: '2026-08-20T03:26:09Z'
updated: '2026-08-20T18:54:20Z'
parent: yaks-6e21
depends_on:
- yaks-6b8c
labels:
- rust
---

---
▸ 2026-08-20T18:47:43Z
Starting. Porting the shared filter surface + query commands to match Python (yaklib/filter.py FilterSpec + yaklib/deps.py ready/tangled). Plan: a FilterSpec applied to a loaded task set (status/type/priority/label/search/ready/tangled/parent-of, AND across dimensions / OR within), wired into list; plus tangled, search, stats commands. --json deferred to the parity yak (yaks-5ef5).

---
▸ 2026-08-20T18:54:20Z
Done. filter.rs (FilterSpec: status/type/priority/label/search/ready/tangled/parent-of; AND-across, OR-within; descendant scope) + resolved/unresolved/descendant helpers; 5 filter unit tests. main: filter flags flattened into list/next/tangled/search; added tangled/search/stats; fixed Shorn glyph to N; row format matches Python _fmt_task_row. BYTE-IDENTICAL to Python on the real herd for list (plain/filtered), search, list --parent-of, stats, next, tangled. 13 tests total.
