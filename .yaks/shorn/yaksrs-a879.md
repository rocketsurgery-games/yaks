---
id: yaksrs-a879
title: rollup (external source projection)
type: task
priority: 3
created: '2026-08-20T03:26:09Z'
updated: '2026-08-20T19:18:57Z'
parent: yaksrs-6e21
depends_on:
- yaksrs-6b8c
labels:
- rust
---

---
▸ 2026-08-20T19:15:21Z
Starting. Porting rollup: group yaks by external source URL with nearest-ancestor inheritance of source; classify tracker + key from URL; --keys lists external keys for PR bodies. Matching yaklib/rollup.py (tracker_and_key, effective_source, build_rollup) + cmd_rollup output. Read-only, no network.

---
▸ 2026-08-20T19:18:57Z
Done. rollup.rs: tracker_and_key (jira/linear/github via hand-rolled scanners, no regex dep) + nearest-ancestor effective_source inheritance + build() grouping (groups sorted by key/source, yaks by id) + unsourced count; 2 unit tests. main: rollup [filters] [--keys] with byte-exact output. Live-verified byte-identical to Python for rollup and rollup --keys on the herd (temporary github source, then restored). 17 tests total.
